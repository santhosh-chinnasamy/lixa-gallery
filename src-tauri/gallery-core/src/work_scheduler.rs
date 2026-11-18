//! Bounded priority queue for image work.
//!
//! Interactive jobs always run before background. Same [`WorkKey`] coalesces
//! onto one execution; a later Interactive submit upgrades a queued Background
//! job.
//!
//! [`begin_generation`] cancels **queued** work from a previous folder and bumps
//! the generation. In-flight jobs keep their `running` slots until `work.await`
//! finishes — admission never exceeds [`ImageWorkScheduler::max_workers`]. On
//! success, every waiter receives `Ok` so in-flight protocol requests can still
//! serve bytes. Callers gate UI emits with [`ImageWorkScheduler::is_current`].

use crate::models::{GalleryError, PhotoMetadata, Result};
use crate::work::{
    recommended_background_slots, recommended_pool_size, WorkKey, WorkPriority,
};
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Notify, oneshot};

type JobFuture = Pin<Box<dyn Future<Output = Result<PhotoMetadata>> + Send>>;
type Reply = oneshot::Sender<std::result::Result<PhotoMetadata, String>>;

struct Job {
    key: WorkKey,
    priority: WorkPriority,
    work: JobFuture,
}

struct State {
    interactive: VecDeque<Job>,
    background: VecDeque<Job>,
    /// Keys queued or running → current priority.
    active: HashMap<WorkKey, WorkPriority>,
    waiters: HashMap<WorkKey, Vec<Reply>>,
    running: usize,
    running_background: usize,
    workers_started: usize,
}

struct Inner {
    generation: AtomicU64,
    state: Mutex<State>,
    notify: Notify,
    max_workers: usize,
    max_background: usize,
}

/// Shared scheduler for protocol, viewer, and folder indexing.
pub struct ImageWorkScheduler {
    inner: Arc<Inner>,
}

impl Default for ImageWorkScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageWorkScheduler {
    pub fn new() -> Self {
        let max_workers = recommended_pool_size();
        Self::with_limits(max_workers, recommended_background_slots(max_workers))
    }

    pub fn with_limits(max_workers: usize, max_background: usize) -> Self {
        let max_workers = max_workers.max(1);
        let max_background = max_background.clamp(1, max_workers);
        Self {
            inner: Arc::new(Inner {
                generation: AtomicU64::new(0),
                state: Mutex::new(State {
                    interactive: VecDeque::new(),
                    background: VecDeque::new(),
                    active: HashMap::new(),
                    waiters: HashMap::new(),
                    running: 0,
                    running_background: 0,
                    workers_started: 0,
                }),
                notify: Notify::new(),
                max_workers,
                max_background,
            }),
        }
    }

    pub fn max_workers(&self) -> usize {
        self.inner.max_workers
    }

    /// How many worker tasks have been spawned (for tests / diagnostics).
    pub async fn workers_started(&self) -> usize {
        self.inner.state.lock().await.workers_started
    }

    /// Bump the generation and cancel **queued** work from a previous folder.
    /// In-flight jobs keep their `running` slots (and stay coalesce targets)
    /// until they finish — never spawn past [`Self::max_workers`].
    /// Callers gate UI emits with [`Self::is_current`].
    pub async fn begin_generation(&self) -> u64 {
        let mut state = self.inner.state.lock().await;
        let next = self.inner.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let mut cancelled_keys: Vec<WorkKey> =
            state.interactive.drain(..).map(|job| job.key).collect();
        cancelled_keys.extend(state.background.drain(..).map(|job| job.key));
        // Queued only — in-flight keys were already popped and stay in `active`
        // until the worker finishes, so later submits can still coalesce.
        for key in cancelled_keys {
            state.active.remove(&key);
            if let Some(waiters) = state.waiters.remove(&key) {
                for waiter in waiters {
                    let _ = waiter.send(Err("cancelled".to_string()));
                }
            }
        }
        drop(state);
        self.inner.notify.notify_waiters();
        next
    }

    pub fn current_generation(&self) -> u64 {
        self.inner.generation.load(Ordering::SeqCst)
    }

    pub fn is_current(&self, generation: u64) -> bool {
        self.current_generation() == generation
    }

    /// True while interactive work is queued or running (background alone is idle).
    pub async fn has_interactive_work(&self) -> bool {
        let state = self.inner.state.lock().await;
        interactive_busy(&state)
    }

    /// Wait until interactive work has been quiet for `quiet_for`, or `max_wait` elapses.
    /// Used so full-folder background indexing yields to on-screen protocol work.
    pub async fn wait_until_interactive_idle(&self, quiet_for: Duration, max_wait: Duration) {
        let deadline = Instant::now() + max_wait;
        let mut quiet_since: Option<Instant> = None;
        loop {
            if Instant::now() >= deadline {
                return;
            }
            if self.has_interactive_work().await {
                quiet_since = None;
            } else {
                let since = *quiet_since.get_or_insert_with(Instant::now);
                if since.elapsed() >= quiet_for {
                    return;
                }
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    /// Run `work` at `priority`, coalescing on `key`.
    ///
    /// If another caller already submitted the same key, `work` is not polled
    /// and this future waits for that result. Successful decodes fan out `Ok`
    /// to every waiter (including ones that attached under an older generation)
    /// so URI-protocol callers are not stuck with a cached 404. Returns
    /// cancelled when this submit's generation is already stale before enqueue,
    /// or when a queued job is drained by [`Self::begin_generation`].
    pub async fn run<F, Fut>(
        &self,
        priority: WorkPriority,
        key: WorkKey,
        generation: u64,
        work: F,
    ) -> Result<PhotoMetadata>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<PhotoMetadata>> + Send + 'static,
    {
        if !self.is_current(generation) {
            return Err(cancelled());
        }

        let (tx, rx) = oneshot::channel();
        {
            let mut state = self.inner.state.lock().await;
            if state.active.contains_key(&key) {
                if priority == WorkPriority::Interactive {
                    if let Some(existing) = state.active.get_mut(&key) {
                        if *existing == WorkPriority::Background {
                            *existing = WorkPriority::Interactive;
                            if let Some(idx) =
                                state.background.iter().position(|job| job.key == key)
                            {
                                let mut job = state.background.remove(idx).expect("index");
                                job.priority = WorkPriority::Interactive;
                                state.interactive.push_front(job);
                            }
                        }
                    }
                }
                state.waiters.entry(key).or_default().push(tx);
            } else {
                state.active.insert(key.clone(), priority);
                state.waiters.entry(key.clone()).or_default().push(tx);
                let job = Job {
                    key,
                    priority,
                    work: Box::pin(work()),
                };
                match priority {
                    WorkPriority::Interactive => state.interactive.push_back(job),
                    WorkPriority::Background => state.background.push_back(job),
                }
                self.ensure_workers(&mut state);
            }
        }
        self.inner.notify.notify_one();

        match rx.await {
            Ok(Ok(photo)) => Ok(photo),
            Ok(Err(message)) if message == "cancelled" => Err(cancelled()),
            Ok(Err(message)) => Err(GalleryError::Unknown(message)),
            Err(_) => Err(cancelled()),
        }
    }

    fn ensure_workers(&self, state: &mut State) {
        while state.workers_started < self.inner.max_workers {
            state.workers_started += 1;
            let inner = self.inner.clone();
            tokio::spawn(async move {
                worker_loop(inner).await;
            });
        }
    }
}

fn interactive_busy(state: &State) -> bool {
    !state.interactive.is_empty() || state.running > state.running_background
}

fn cancelled() -> GalleryError {
    GalleryError::Unknown("cancelled".to_string())
}

async fn worker_loop(inner: Arc<Inner>) {
    loop {
        let job = {
            let mut state = inner.state.lock().await;
            loop {
                let can_background = state.running_background < inner.max_background
                    && state.interactive.is_empty();
                let next = if !state.interactive.is_empty()
                    && state.running < inner.max_workers
                {
                    state.interactive.pop_front()
                } else if can_background
                    && !state.background.is_empty()
                    && state.running < inner.max_workers
                {
                    state.background.pop_front()
                } else {
                    None
                };

                if let Some(job) = next {
                    state.running += 1;
                    if job.priority == WorkPriority::Background {
                        state.running_background += 1;
                    }
                    break job;
                }

                drop(state);
                inner.notify.notified().await;
                state = inner.state.lock().await;
            }
        };

        let is_background = job.priority == WorkPriority::Background;
        let key = job.key.clone();

        // Always run once dequeued. Folder switches must not discard durable
        // decode work; callers gate UI emits via is_current.
        let outcome = match job.work.await {
            Ok(photo) => Ok(photo),
            Err(err) => Err(err.to_string()),
        };

        {
            let mut state = inner.state.lock().await;
            state.running = state.running.saturating_sub(1);
            if is_background {
                state.running_background = state.running_background.saturating_sub(1);
            }
            state.active.remove(&key);
            if let Some(waiters) = state.waiters.remove(&key) {
                for waiter in waiters {
                    let _ = waiter.send(outcome.clone());
                }
            }
        }
        inner.notify.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FileMetadata;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::time::Duration;

    fn photo(path: &str) -> PhotoMetadata {
        PhotoMetadata {
            metadata: FileMetadata {
                name: path.to_string(),
                modified: 1,
                created: 1,
                size: 1,
            },
            thumbnail_path: "t".into(),
            path: path.to_string(),
            sidecar_path: None,
            preview_path: None,
            exif: None,
            is_raw: false,
        }
    }

    #[tokio::test]
    async fn interactive_runs_before_queued_background() {
        let sched = ImageWorkScheduler::with_limits(1, 1);
        let gen = sched.begin_generation().await;
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));

        let (release_tx, release_rx) = oneshot::channel::<()>();
        let release_rx = Arc::new(Mutex::new(Some(release_rx)));

        let gate = release_rx.clone();
        let block_handle = tokio::spawn({
            let sched = sched.clone();
            let order = order.clone();
            async move {
                sched
                    .run(
                        WorkPriority::Background,
                        WorkKey::materialize("/block", 1, 1),
                        gen,
                        move || {
                            let gate = gate;
                            let order = order;
                            async move {
                                order.lock().unwrap().push("block-start");
                                if let Some(rx) = gate.lock().await.take() {
                                    let _ = rx.await;
                                }
                                order.lock().unwrap().push("block-end");
                                Ok(photo("/block"))
                            }
                        },
                    )
                    .await
            }
        });

        tokio::time::sleep(Duration::from_millis(20)).await;

        let bg_handle = tokio::spawn({
            let sched = sched.clone();
            let order = order.clone();
            async move {
                sched
                    .run(
                        WorkPriority::Background,
                        WorkKey::materialize("/bg", 1, 1),
                        gen,
                        move || {
                            let order = order;
                            async move {
                                order.lock().unwrap().push("bg");
                                Ok(photo("/bg"))
                            }
                        },
                    )
                    .await
            }
        });

        tokio::time::sleep(Duration::from_millis(20)).await;

        let int_handle = tokio::spawn({
            let sched = sched.clone();
            let order = order.clone();
            async move {
                sched
                    .run(
                        WorkPriority::Interactive,
                        WorkKey::materialize("/int", 1, 1),
                        gen,
                        move || {
                            let order = order;
                            async move {
                                order.lock().unwrap().push("int");
                                Ok(photo("/int"))
                            }
                        },
                    )
                    .await
            }
        });

        tokio::time::sleep(Duration::from_millis(20)).await;
        let _ = release_tx.send(());

        let _ = block_handle.await.unwrap();
        let _ = int_handle.await.unwrap();
        let _ = bg_handle.await.unwrap();

        let seen = order.lock().unwrap().clone();
        let int_pos = seen.iter().position(|s| *s == "int").unwrap();
        let bg_pos = seen.iter().position(|s| *s == "bg").unwrap();
        assert!(
            int_pos < bg_pos,
            "interactive should run before background, got {seen:?}"
        );
    }

    #[tokio::test]
    async fn coalesce_runs_work_once() {
        let sched = ImageWorkScheduler::with_limits(2, 1);
        let gen = sched.begin_generation().await;
        let runs = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..5 {
            let sched = sched.clone();
            let runs = runs.clone();
            handles.push(tokio::spawn(async move {
                sched
                    .run(
                        WorkPriority::Interactive,
                        WorkKey::materialize("/same", 5, 9),
                        gen,
                        move || {
                            let runs = runs;
                            async move {
                                runs.fetch_add(1, AtomicOrdering::SeqCst);
                                tokio::time::sleep(Duration::from_millis(30)).await;
                                Ok(photo("/same"))
                            }
                        },
                    )
                    .await
            }));
        }

        for handle in handles {
            let result = handle.await.unwrap().unwrap();
            assert_eq!(result.path, "/same");
        }
        assert_eq!(runs.load(AtomicOrdering::SeqCst), 1);
    }

    #[tokio::test]
    async fn stale_generation_cancels() {
        let sched = ImageWorkScheduler::with_limits(2, 1);
        let old = sched.begin_generation().await;
        let _new = sched.begin_generation().await;

        let err = sched
            .run(
                WorkPriority::Background,
                WorkKey::materialize("/x", 1, 1),
                old,
                || async { Ok(photo("/x")) },
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("cancelled"));
    }

    #[tokio::test]
    async fn background_upgrade_to_interactive_coalesces() {
        let sched = ImageWorkScheduler::with_limits(1, 1);
        let gen = sched.begin_generation().await;
        let runs = Arc::new(AtomicUsize::new(0));

        let (release_tx, release_rx) = oneshot::channel::<()>();
        let release_rx = Arc::new(Mutex::new(Some(release_rx)));

        let gate = release_rx.clone();
        let block = tokio::spawn({
            let sched = sched.clone();
            async move {
                sched
                    .run(
                        WorkPriority::Interactive,
                        WorkKey::materialize("/hold", 1, 1),
                        gen,
                        move || {
                            let gate = gate;
                            async move {
                                if let Some(rx) = gate.lock().await.take() {
                                    let _ = rx.await;
                                }
                                Ok(photo("/hold"))
                            }
                        },
                    )
                    .await
            }
        });

        tokio::time::sleep(Duration::from_millis(15)).await;

        let runs_bg = runs.clone();
        let bg = tokio::spawn({
            let sched = sched.clone();
            async move {
                sched
                    .run(
                        WorkPriority::Background,
                        WorkKey::materialize("/up", 1, 1),
                        gen,
                        move || {
                            let runs = runs_bg;
                            async move {
                                runs.fetch_add(1, AtomicOrdering::SeqCst);
                                Ok(photo("/up"))
                            }
                        },
                    )
                    .await
            }
        });

        tokio::time::sleep(Duration::from_millis(15)).await;

        let runs_int = runs.clone();
        let int = tokio::spawn({
            let sched = sched.clone();
            async move {
                sched
                    .run(
                        WorkPriority::Interactive,
                        WorkKey::materialize("/up", 1, 1),
                        gen,
                        move || {
                            let runs = runs_int;
                            async move {
                                runs.fetch_add(1, AtomicOrdering::SeqCst);
                                Ok(photo("/up"))
                            }
                        },
                    )
                    .await
            }
        });

        let _ = release_tx.send(());
        let _ = block.await.unwrap();
        assert_eq!(bg.await.unwrap().unwrap().path, "/up");
        assert_eq!(int.await.unwrap().unwrap().path, "/up");
        assert_eq!(runs.load(AtomicOrdering::SeqCst), 1);
    }

    #[tokio::test]
    async fn in_flight_survives_generation_bump_and_serves_new_waiter() {
        let sched = ImageWorkScheduler::with_limits(1, 1);
        let old_gen = sched.begin_generation().await;
        let (release_tx, release_rx) = oneshot::channel::<()>();
        let gate = Arc::new(Mutex::new(Some(release_rx)));
        let runs = Arc::new(AtomicUsize::new(0));

        let old = tokio::spawn({
            let sched = sched.clone();
            let gate = gate.clone();
            let runs = runs.clone();
            async move {
                sched
                    .run(
                        WorkPriority::Background,
                        WorkKey::materialize("/a.jpg", 1, 1),
                        old_gen,
                        move || {
                            let gate = gate;
                            let runs = runs;
                            async move {
                                runs.fetch_add(1, AtomicOrdering::SeqCst);
                                if let Some(rx) = gate.lock().await.take() {
                                    let _ = rx.await;
                                }
                                Ok(photo("/a.jpg"))
                            }
                        },
                    )
                    .await
            }
        });

        tokio::time::sleep(Duration::from_millis(30)).await;
        let new_gen = sched.begin_generation().await;
        assert_ne!(old_gen, new_gen);

        let new_handle = tokio::spawn({
            let sched = sched.clone();
            async move {
                sched
                    .run(
                        WorkPriority::Interactive,
                        WorkKey::materialize("/a.jpg", 1, 1),
                        new_gen,
                        move || async move {
                            panic!("coalesced waiter must not run a second closure");
                        },
                    )
                    .await
            }
        });

        tokio::time::sleep(Duration::from_millis(20)).await;
        let _ = release_tx.send(());

        let old_result = old.await.unwrap();
        let new_result = new_handle.await.unwrap();
        assert!(
            old_result.is_ok(),
            "in-flight starter must still receive Ok so protocol can serve bytes, got {old_result:?}"
        );
        assert!(
            new_result.is_ok(),
            "new session waiter should receive Ok, got {new_result:?}"
        );
        assert_eq!(runs.load(AtomicOrdering::SeqCst), 1);
        assert!(sched.is_current(new_gen));
        assert!(!sched.is_current(old_gen));
    }

    /// Protocol-shaped regression: a folder switch must not turn an in-flight
    /// successful decode into a cancelled error for the waiter that started it.
    #[tokio::test]
    async fn successful_in_flight_decode_is_not_cancelled_after_generation_bump() {
        let sched = ImageWorkScheduler::with_limits(1, 1);
        let old_gen = sched.begin_generation().await;
        let (release_tx, release_rx) = oneshot::channel::<()>();
        let gate = Arc::new(Mutex::new(Some(release_rx)));

        let started = tokio::spawn({
            let sched = sched.clone();
            let gate = gate.clone();
            async move {
                sched
                    .run(
                        WorkPriority::Interactive,
                        WorkKey::materialize("/thumb.jpg", 1, 1),
                        old_gen,
                        move || {
                            let gate = gate;
                            async move {
                                if let Some(rx) = gate.lock().await.take() {
                                    let _ = rx.await;
                                }
                                Ok(photo("/thumb.jpg"))
                            }
                        },
                    )
                    .await
            }
        });

        tokio::time::sleep(Duration::from_millis(20)).await;
        let _new_gen = sched.begin_generation().await;
        let _ = release_tx.send(());

        let result = started.await.unwrap();
        assert!(
            result.is_ok(),
            "protocol waiter would 404 if this were Cancelled: {result:?}"
        );
    }

    struct ConcurrentGuard {
        concurrent: Arc<AtomicUsize>,
    }

    impl Drop for ConcurrentGuard {
        fn drop(&mut self) {
            self.concurrent.fetch_sub(1, AtomicOrdering::SeqCst);
        }
    }

    fn enter_body(concurrent: &Arc<AtomicUsize>, peak: &Arc<AtomicUsize>) -> ConcurrentGuard {
        let now = concurrent.fetch_add(1, AtomicOrdering::SeqCst) + 1;
        peak.fetch_max(now, AtomicOrdering::SeqCst);
        ConcurrentGuard {
            concurrent: concurrent.clone(),
        }
    }

    /// Hard-bound admission: with max_workers=1 a stale in-flight job may finish
    /// before new Interactive starts, and concurrent bodies never exceed the cap.
    #[tokio::test]
    async fn generation_bump_keeps_hard_worker_bound() {
        let sched = ImageWorkScheduler::with_limits(1, 1);
        let old_gen = sched.begin_generation().await;
        let (release_tx, release_rx) = oneshot::channel::<()>();
        let gate = Arc::new(Mutex::new(Some(release_rx)));
        let concurrent = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));

        let stale = tokio::spawn({
            let sched = sched.clone();
            let gate = gate.clone();
            let concurrent = concurrent.clone();
            let peak = peak.clone();
            async move {
                sched
                    .run(
                        WorkPriority::Background,
                        WorkKey::materialize("/old.jpg", 1, 1),
                        old_gen,
                        move || {
                            let gate = gate;
                            let concurrent = concurrent;
                            let peak = peak;
                            async move {
                                let _guard = enter_body(&concurrent, &peak);
                                if let Some(rx) = gate.lock().await.take() {
                                    let _ = rx.await;
                                }
                                Ok(photo("/old.jpg"))
                            }
                        },
                    )
                    .await
            }
        });

        tokio::time::sleep(Duration::from_millis(30)).await;
        let new_gen = sched.begin_generation().await;

        let interactive = tokio::spawn({
            let sched = sched.clone();
            let concurrent = concurrent.clone();
            let peak = peak.clone();
            async move {
                sched
                    .run(
                        WorkPriority::Interactive,
                        WorkKey::materialize("/new.jpg", 1, 1),
                        new_gen,
                        move || {
                            let concurrent = concurrent;
                            let peak = peak;
                            async move {
                                let _guard = enter_body(&concurrent, &peak);
                                Ok(photo("/new.jpg"))
                            }
                        },
                    )
                    .await
            }
        });

        // Stale may still hold the only slot; Interactive is allowed to wait.
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            peak.load(AtomicOrdering::SeqCst) <= 1,
            "concurrent bodies exceeded max_workers=1"
        );

        let _ = release_tx.send(());
        let _ = stale.await.unwrap();
        assert!(interactive.await.unwrap().is_ok());
        assert!(
            peak.load(AtomicOrdering::SeqCst) <= 1,
            "peak concurrent bodies exceeded max_workers=1: {}",
            peak.load(AtomicOrdering::SeqCst)
        );
        assert!(
            sched.workers_started().await <= 1,
            "workers_started exceeded max_workers"
        );
    }

    /// Rapid folder switches must not oversubscribe decode past max_workers while
    /// durable in-flight jobs are still blocked in their bodies.
    #[tokio::test]
    async fn rapid_generation_bumps_peak_concurrent_bodies_bounded() {
        let max_workers = 2;
        let sched = ImageWorkScheduler::with_limits(max_workers, 1);
        let gen = sched.begin_generation().await;
        let concurrent = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let mut release_txs = Vec::new();
        let mut stale_handles = Vec::new();

        for i in 0..max_workers {
            let (release_tx, release_rx) = oneshot::channel::<()>();
            let gate = Arc::new(Mutex::new(Some(release_rx)));
            release_txs.push(release_tx);
            let path = format!("/stale-{i}.jpg");
            let sched = sched.clone();
            let concurrent = concurrent.clone();
            let peak = peak.clone();
            stale_handles.push(tokio::spawn(async move {
                sched
                    .run(
                        // Interactive so we fill the whole pool (background alone
                        // is capped by max_background).
                        WorkPriority::Interactive,
                        WorkKey::materialize(path.clone(), 1, 1),
                        gen,
                        move || {
                            let gate = gate;
                            let concurrent = concurrent;
                            let peak = peak;
                            async move {
                                let now = concurrent.fetch_add(1, AtomicOrdering::SeqCst) + 1;
                                peak.fetch_max(now, AtomicOrdering::SeqCst);
                                if let Some(rx) = gate.lock().await.take() {
                                    let _ = rx.await;
                                }
                                concurrent.fetch_sub(1, AtomicOrdering::SeqCst);
                                Ok(photo(&path))
                            }
                        },
                    )
                    .await
            }));
        }

        let fill_deadline = Instant::now() + Duration::from_millis(500);
        while concurrent.load(AtomicOrdering::SeqCst) < max_workers {
            assert!(
                Instant::now() < fill_deadline,
                "timed out waiting for {max_workers} in-flight bodies, have {}",
                concurrent.load(AtomicOrdering::SeqCst)
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        let mut interactive_handles = Vec::new();
        for bump in 0..8 {
            let new_gen = sched.begin_generation().await;
            let path = format!("/new-{bump}.jpg");
            let sched = sched.clone();
            let concurrent = concurrent.clone();
            let peak = peak.clone();
            interactive_handles.push(tokio::spawn(async move {
                sched
                    .run(
                        WorkPriority::Interactive,
                        WorkKey::materialize(path.clone(), 1, 1),
                        new_gen,
                        move || {
                            let concurrent = concurrent;
                            let peak = peak;
                            async move {
                                let now = concurrent.fetch_add(1, AtomicOrdering::SeqCst) + 1;
                                peak.fetch_max(now, AtomicOrdering::SeqCst);
                                tokio::time::sleep(Duration::from_millis(5)).await;
                                concurrent.fetch_sub(1, AtomicOrdering::SeqCst);
                                Ok(photo(&path))
                            }
                        },
                    )
                    .await
            }));
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        tokio::time::sleep(Duration::from_millis(30)).await;
        let observed_peak = peak.load(AtomicOrdering::SeqCst);
        assert!(
            observed_peak <= max_workers,
            "peak concurrent job bodies {observed_peak} exceeded max_workers={max_workers}"
        );

        for tx in release_txs {
            let _ = tx.send(());
        }
        for handle in stale_handles {
            let _ = handle.await.unwrap();
        }
        for handle in interactive_handles {
            let _ = handle.await.unwrap();
        }
        assert!(
            peak.load(AtomicOrdering::SeqCst) <= max_workers,
            "final peak {} exceeded max_workers={max_workers}",
            peak.load(AtomicOrdering::SeqCst)
        );
    }

    /// Detaching slots and spawning overflow workers on each bump was the hang:
    /// workers_started climbed past the cap while stale decode still ran.
    #[tokio::test]
    async fn generation_bumps_do_not_spawn_past_max_workers() {
        let max_workers = 3;
        let sched = ImageWorkScheduler::with_limits(max_workers, 1);
        let gen = sched.begin_generation().await;
        let mut release_txs = Vec::new();
        let mut handles = Vec::new();

        for i in 0..max_workers {
            let (release_tx, release_rx) = oneshot::channel::<()>();
            let gate = Arc::new(Mutex::new(Some(release_rx)));
            release_txs.push(release_tx);
            let path = format!("/hold-{i}.jpg");
            let sched = sched.clone();
            handles.push(tokio::spawn(async move {
                sched
                    .run(
                        WorkPriority::Interactive,
                        WorkKey::materialize(path.clone(), 1, 1),
                        gen,
                        move || {
                            let gate = gate;
                            async move {
                                if let Some(rx) = gate.lock().await.take() {
                                    let _ = rx.await;
                                }
                                Ok(photo(&path))
                            }
                        },
                    )
                    .await
            }));
        }

        tokio::time::sleep(Duration::from_millis(40)).await;
        assert_eq!(sched.workers_started().await, max_workers);

        for _ in 0..10 {
            let _ = sched.begin_generation().await;
        }

        let started = sched.workers_started().await;
        assert!(
            started <= max_workers,
            "workers_started={started} exceeded max_workers={max_workers} after generation bumps"
        );

        for tx in release_txs {
            let _ = tx.send(());
        }
        for handle in handles {
            let _ = handle.await.unwrap();
        }
    }
}

impl Clone for ImageWorkScheduler {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

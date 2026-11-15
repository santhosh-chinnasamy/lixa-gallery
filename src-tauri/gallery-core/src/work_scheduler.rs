//! Bounded priority queue for image work.
//!
//! Interactive jobs always run before background. Same [`WorkKey`] coalesces
//! onto one execution; a later Interactive submit upgrades a queued Background
//! job. [`begin_generation`] cancels stale folder work.

use crate::models::{GalleryError, PhotoMetadata, Result};
use crate::work::{
    recommended_background_slots, recommended_pool_size, WorkKey, WorkPriority,
};
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, Notify, oneshot};

type JobFuture = Pin<Box<dyn Future<Output = Result<PhotoMetadata>> + Send>>;
type Reply = oneshot::Sender<std::result::Result<PhotoMetadata, String>>;

struct Job {
    key: WorkKey,
    priority: WorkPriority,
    generation: u64,
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

    /// Bump the generation so in-flight/queued work from a previous folder
    /// resolves as cancelled and does not emit to the UI. Queued jobs are
    /// drained before this returns; in-flight jobs cancel when they finish.
    pub async fn begin_generation(&self) -> u64 {
        let mut state = self.inner.state.lock().await;
        let next = self.inner.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let mut cancelled_keys: Vec<WorkKey> =
            state.interactive.drain(..).map(|job| job.key).collect();
        cancelled_keys.extend(state.background.drain(..).map(|job| job.key));
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

    /// Run `work` at `priority`, coalescing on `key`.
    ///
    /// If another caller already submitted the same key, `work` is not polled
    /// and this future waits for that result. Returns [`GalleryError::Unknown`]
    /// with `"cancelled"` when the generation is stale.
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
                    generation,
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
        let generation = job.generation;

        let outcome = if inner.generation.load(Ordering::SeqCst) != generation {
            Err("cancelled".to_string())
        } else {
            match job.work.await {
                Ok(photo) => {
                    if inner.generation.load(Ordering::SeqCst) != generation {
                        Err("cancelled".to_string())
                    } else {
                        Ok(photo)
                    }
                }
                Err(err) => Err(err.to_string()),
            }
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

        // Hold the single worker with a gate.
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

        // Occupy the worker.
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
}

impl Clone for ImageWorkScheduler {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

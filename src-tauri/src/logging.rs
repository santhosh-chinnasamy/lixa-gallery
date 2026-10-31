use chrono::Local;
use log::{Level, LevelFilter, Log, Metadata, Record, SetLoggerError};
use std::collections::VecDeque;
use std::fs::{create_dir_all, read_dir, remove_file, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, TryLockError};

/// Maximum total memory in bytes the log ring buffer can consume (2 MB default).
pub const DEFAULT_MAX_BUFFER_BYTES: usize = 2 * 1024 * 1024;

/// Maximum number of log entries kept in the buffer.
pub const DEFAULT_MAX_ENTRIES: usize = 2000;

/// Maximum byte length for a single log message before truncation.
pub const MAX_MESSAGE_BYTES: usize = 2048;

/// On-demand dump filename (not used for crash reports).
pub const SESSION_LOG_FILENAME: &str = "Lixa Gallery.log";

/// Delete legacy/rotated log files larger than this at startup (5 MB).
pub const MAX_ON_DISK_LOG_BYTES: u64 = 5 * 1024 * 1024;

/// Keep this many timestamped crash dumps.
pub const MAX_CRASH_LOGS: usize = 5;

const APP_IDENTIFIER: &str = "com.lixa-gallery.app";

pub struct LogRingBuffer {
    entries: VecDeque<String>,
    current_bytes: usize,
    max_bytes: usize,
    max_entries: usize,
}

impl LogRingBuffer {
    pub fn new(max_bytes: usize, max_entries: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_entries.min(1024)),
            current_bytes: 0,
            max_bytes,
            max_entries,
        }
    }

    pub fn push(&mut self, line: String) {
        // Account for the trailing newline written when flushing to disk.
        let needed = line.len().saturating_add(1);

        while !self.entries.is_empty()
            && (self.current_bytes + needed > self.max_bytes
                || self.entries.len() >= self.max_entries)
        {
            if let Some(old) = self.entries.pop_front() {
                self.current_bytes = self
                    .current_bytes
                    .saturating_sub(old.len().saturating_add(1));
            }
        }

        self.current_bytes = self.current_bytes.saturating_add(needed);
        self.entries.push_back(line);
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.entries.iter().cloned().collect()
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn byte_size(&self) -> usize {
        self.current_bytes
    }
}

fn write_log_file(path: &Path, entries: &[String], extra: Option<&str>) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;

    for entry in entries {
        file.write_all(entry.as_bytes())?;
        file.write_all(b"\n")?;
    }
    if let Some(extra) = extra {
        file.write_all(extra.as_bytes())?;
    }
    file.flush()?;
    Ok(())
}

fn truncate_message(mut msg: String) -> String {
    if msg.len() <= MAX_MESSAGE_BYTES {
        return msg;
    }

    let mut cut = MAX_MESSAGE_BYTES;
    while cut > 0 && !msg.is_char_boundary(cut) {
        cut -= 1;
    }
    msg.truncate(cut);
    msg.push_str("... [TRUNCATED]");
    msg
}

fn snapshot_buffer(buffer: &Arc<Mutex<LogRingBuffer>>) -> Vec<String> {
    match buffer.try_lock() {
        Ok(guard) => guard.snapshot(),
        Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner().snapshot(),
        Err(TryLockError::WouldBlock) => Vec::new(),
    }
}

pub struct FlightRecorderLogger {
    ring_buffer: Arc<Mutex<LogRingBuffer>>,
    stdout_enabled: bool,
}

impl FlightRecorderLogger {
    pub fn new(ring_buffer: Arc<Mutex<LogRingBuffer>>, stdout_enabled: bool) -> Self {
        Self {
            ring_buffer,
            stdout_enabled,
        }
    }
}

impl Log for FlightRecorderLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // Suppress verbose internal sqlx query logs above info
        if metadata.target().starts_with("sqlx::query") && metadata.level() > Level::Info {
            return false;
        }
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let now = Local::now().format("[%Y-%m-%d][%H:%M:%S%.3f]").to_string();
        let msg = truncate_message(record.args().to_string());
        let formatted = format!("{}[{}][{}] {}", now, record.level(), record.target(), msg);

        if self.stdout_enabled {
            println!("{formatted}");
        }

        match self.ring_buffer.lock() {
            Ok(mut buffer) => buffer.push(formatted),
            Err(poisoned) => poisoned.into_inner().push(formatted),
        }
    }

    fn flush(&self) {}
}

static RECORDER: OnceLock<Arc<Mutex<LogRingBuffer>>> = OnceLock::new();
static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn init_flight_recorder() -> Result<Arc<Mutex<LogRingBuffer>>, SetLoggerError> {
    let stdout_enabled = cfg!(debug_assertions) || std::env::var("LIXA_LOG_STDOUT").is_ok();
    let ring_buffer = Arc::new(Mutex::new(LogRingBuffer::new(
        DEFAULT_MAX_BUFFER_BYTES,
        DEFAULT_MAX_ENTRIES,
    )));

    let logger = FlightRecorderLogger::new(ring_buffer.clone(), stdout_enabled);
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(LevelFilter::Info);

    let _ = RECORDER.set(ring_buffer.clone());
    Ok(ring_buffer)
}

pub fn set_log_dir(path: PathBuf) {
    cleanup_oversized_logs(&path, MAX_ON_DISK_LOG_BYTES);
    let _ = LOG_DIR.set(path);
}

pub fn get_log_dir() -> Option<&'static PathBuf> {
    LOG_DIR.get()
}

pub fn get_ring_buffer() -> Option<Arc<Mutex<LogRingBuffer>>> {
    RECORDER.get().cloned()
}

/// Platform log dir matching Tauri's `app_log_dir` conventions.
pub fn default_log_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|d| d.join("Library/Logs").join(APP_IDENTIFIER))
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Linux: ~/.local/share/<id>/logs; Windows: LocalAppData/<id>/logs
        dirs::data_local_dir().map(|d| d.join(APP_IDENTIFIER).join("logs"))
    }
}

/// Remove oversized `*.log` files left by the previous unbounded logger.
pub fn cleanup_oversized_logs(dir: &Path, max_bytes: u64) {
    let Ok(entries) = read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let is_log = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("log"));
        if !is_log {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if meta.len() > max_bytes {
                let _ = remove_file(&path);
            }
        }
    }
}

fn prune_crash_logs(dir: &Path, keep: usize) {
    let Ok(entries) = read_dir(dir) else {
        return;
    };

    let mut crashes: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            if !(name.starts_with("crash-") && name.ends_with(".log")) {
                return None;
            }
            let modified = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            Some((modified, path))
        })
        .collect();

    crashes.sort_by_key(|a| a.0);
    let excess = crashes.len().saturating_sub(keep);
    for (_, path) in crashes.into_iter().take(excess) {
        let _ = remove_file(path);
    }
}

/// Flush the in-memory buffer to the on-demand session log (overwrites that file only).
pub fn flush_session_log(path: &Path) -> std::io::Result<()> {
    let entries = get_ring_buffer()
        .map(|buffer| snapshot_buffer(&buffer))
        .unwrap_or_default();
    write_log_file(path, &entries, None)
}

/// Write ring-buffer + panic info to a dedicated crash file; never touches the session log.
pub fn dump_crash(crash_info: &str) {
    let dir = get_log_dir().cloned().or_else(default_log_dir);
    let Some(dir) = dir else {
        return;
    };

    let entries = get_ring_buffer()
        .map(|buffer| snapshot_buffer(&buffer))
        .unwrap_or_default();

    write_crash_dump(&dir, &entries, crash_info);
}

fn write_crash_dump(dir: &Path, entries: &[String], crash_info: &str) {
    let stamp = Local::now().format("%Y%m%d-%H%M%S-%3f");
    let log_file = dir.join(format!("crash-{stamp}.log"));
    let _ = write_log_file(&log_file, entries, Some(crash_info));
    prune_crash_logs(dir, MAX_CRASH_LOGS);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn test_ring_buffer_eviction_by_count() {
        let mut buf = LogRingBuffer::new(10_000, 3);
        buf.push("line 1".to_string());
        buf.push("line 2".to_string());
        buf.push("line 3".to_string());
        assert_eq!(buf.entry_count(), 3);

        buf.push("line 4".to_string());
        assert_eq!(buf.entry_count(), 3);
        assert_eq!(buf.entries[0], "line 2");
        assert_eq!(buf.entries[2], "line 4");
    }

    #[test]
    fn test_ring_buffer_eviction_by_bytes() {
        // max 25 bytes including trailing newlines counted in current_bytes.
        let mut buf = LogRingBuffer::new(25, 100);
        buf.push("123456789".to_string()); // 10 bytes
        buf.push("abcdefghi".to_string()); // 10 bytes
        assert_eq!(buf.entry_count(), 2);
        assert!(buf.byte_size() <= 25);

        buf.push("ABCDEFGHI".to_string());
        assert_eq!(buf.entry_count(), 2);
        assert!(buf.byte_size() <= 25);
        assert_eq!(buf.entries[0], "abcdefghi");
        assert_eq!(buf.entries[1], "ABCDEFGHI");
    }

    #[test]
    fn test_flush_session_log_writes_snapshot() {
        let dir = tempdir().unwrap();
        let log_file = dir.path().join("test.log");

        let mut buf = LogRingBuffer::new(10_000, 10);
        buf.push("entry alpha".to_string());
        buf.push("entry beta".to_string());

        write_log_file(&log_file, &buf.snapshot(), None).unwrap();
        let content = fs::read_to_string(&log_file).unwrap();
        assert_eq!(content, "entry alpha\nentry beta\n");
    }

    #[test]
    fn test_truncate_message_utf8_safe() {
        // Tamil அ is 3 bytes; force a cut that would land mid-character at 2048.
        let msg = "அ".repeat(800);
        assert!(msg.len() > MAX_MESSAGE_BYTES);
        assert!(!msg.is_char_boundary(MAX_MESSAGE_BYTES));

        let truncated = truncate_message(msg);
        assert!(truncated.ends_with("... [TRUNCATED]"));
        assert!(truncated.is_char_boundary(truncated.len()));
        // Must not panic when used as a normal String.
        let _ = truncated.to_string();
    }

    #[test]
    fn test_cleanup_oversized_logs() {
        let dir = tempdir().unwrap();
        let small = dir.path().join("small.log");
        let large = dir.path().join("large.log");
        fs::write(&small, vec![b'x'; 10]).unwrap();
        fs::write(&large, vec![b'y'; 100]).unwrap();

        cleanup_oversized_logs(dir.path(), 50);

        assert!(small.exists());
        assert!(!large.exists());
    }

    #[test]
    fn test_prune_crash_logs_keeps_newest() {
        let dir = tempdir().unwrap();
        for i in 0..7 {
            let path = dir.path().join(format!("crash-2026010{i}-000000-000.log"));
            fs::write(&path, format!("crash {i}")).unwrap();
            // Ensure distinct mtimes for stable ordering.
            thread::sleep(Duration::from_millis(5));
        }
        // Non-crash log should be ignored.
        fs::write(dir.path().join("Lixa Gallery.log"), "session").unwrap();

        prune_crash_logs(dir.path(), 5);

        let mut remaining: Vec<_> = read_dir(dir.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        remaining.sort();
        assert!(remaining.iter().any(|n| n == "Lixa Gallery.log"));
        let crashes: Vec<_> = remaining
            .into_iter()
            .filter(|n| n.starts_with("crash-"))
            .collect();
        assert_eq!(crashes.len(), 5);
    }

    #[test]
    fn test_write_crash_dump_uses_separate_file() {
        let dir = tempdir().unwrap();
        let session = dir.path().join(SESSION_LOG_FILENAME);
        fs::write(&session, "previous session").unwrap();

        write_crash_dump(dir.path(), &["action 1".to_string()], "CRASH DETAIL");

        // Session log must remain untouched.
        assert_eq!(fs::read_to_string(&session).unwrap(), "previous session");

        let crashes: Vec<_> = read_dir(dir.path())
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("crash-"))
            })
            .collect();
        assert_eq!(crashes.len(), 1);
        let content = fs::read_to_string(&crashes[0]).unwrap();
        assert!(content.contains("action 1"));
        assert!(content.contains("CRASH DETAIL"));
    }

    #[test]
    fn test_snapshot_recovers_poisoned_lock() {
        let buffer = Arc::new(Mutex::new(LogRingBuffer::new(10_000, 10)));
        buffer.lock().unwrap().push("before poison".to_string());

        // Poison the mutex intentionally.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = buffer.lock().unwrap();
            panic!("poison");
        }));

        let snap = snapshot_buffer(&buffer);
        assert_eq!(snap, vec!["before poison".to_string()]);
    }
}

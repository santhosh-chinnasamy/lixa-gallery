//! Priority image work for thumbnails, RAW previews, and EXIF.
//!
//! Interactive work (visible grid / viewer) always preempts background
//! folder indexing. Jobs coalesce on identity so the same file is not
//! decoded twice.

use std::hash::{Hash, Hasher};

/// Prefer interactive slots. Ord so Interactive < Background for heap/queues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorkPriority {
    Interactive = 0,
    Background = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkKind {
    /// Thumbnail (and RAW preview when applicable) plus EXIF.
    Materialize,
    /// EXIF only — used when the thumb is already current.
    ExifOnly,
    /// Decoded viewer preview (formats the webview cannot paint natively).
    Preview,
}

/// Coalesce key. Path alone is not enough after an in-place edit.
#[derive(Debug, Clone, Eq)]
pub struct WorkKey {
    pub path: String,
    pub mtime: u64,
    pub size: u64,
    pub kind: WorkKind,
}

impl PartialEq for WorkKey {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
            && self.mtime == other.mtime
            && self.size == other.size
            && self.kind == other.kind
    }
}

impl Hash for WorkKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
        self.mtime.hash(state);
        self.size.hash(state);
        self.kind.hash(state);
    }
}

impl WorkKey {
    pub fn materialize(path: impl Into<String>, mtime: u64, size: u64) -> Self {
        Self {
            path: path.into(),
            mtime,
            size,
            kind: WorkKind::Materialize,
        }
    }

    pub fn exif_only(path: impl Into<String>, mtime: u64, size: u64) -> Self {
        Self {
            path: path.into(),
            mtime,
            size,
            kind: WorkKind::ExifOnly,
        }
    }

    pub fn preview(path: impl Into<String>, mtime: u64, size: u64) -> Self {
        Self {
            path: path.into(),
            mtime,
            size,
            kind: WorkKind::Preview,
        }
    }
}

/// Leave headroom for the OS and webview. Never use every core.
pub fn recommended_pool_size() -> usize {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    // Visible-first + cheaper decode leave headroom for a slightly larger pool.
    (cores / 2).clamp(2, 6)
}

/// At most one background materialize at a time.
///
/// Each job already burns a Rayon/decode core. Running several background
/// jobs in parallel saturated the machine (measured ~270% CPU) and starved
/// the webview while scrolling a large Lazy folder. Interactive work still
/// uses the rest of the scheduler pool.
pub fn recommended_background_slots(_pool_size: usize) -> usize {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_sorts_before_background() {
        assert!(WorkPriority::Interactive < WorkPriority::Background);
    }

    #[test]
    fn pool_size_is_capped_and_non_zero() {
        let n = recommended_pool_size();
        assert!((2..=6).contains(&n));
        assert_eq!(recommended_background_slots(n), 1);
    }

    #[test]
    fn work_key_includes_identity() {
        let a = WorkKey::materialize("/a.jpg", 1, 10);
        let b = WorkKey::materialize("/a.jpg", 2, 10);
        assert_ne!(a, b);
    }
}

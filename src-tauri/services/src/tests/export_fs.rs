use super::fakes::io_error;
use async_trait::async_trait;
use gallery_core::fs::{collision_key, FileSystem};
use gallery_core::models::{FileMetadata, Result};
use std::ffi::OsString;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

struct StoredFile {
    path: PathBuf,
    content: Vec<u8>,
}

/// Filesystem double whose `copy` and `rename` replace an existing destination,
/// matching `tokio::fs`. Exclusive operations refuse that replacement.
/// `case_insensitive` models the default macOS and Windows volumes.
///
/// Rename and copy failures are injectable so a cross-device move can be
/// simulated without a second drive.
pub struct ExportFs {
    files: Mutex<Vec<StoredFile>>,
    case_insensitive: bool,
    fail_every_rename: Mutex<Option<ErrorKind>>,
    fail_rename_paths: Mutex<Vec<(PathBuf, ErrorKind)>>,
    /// `copy_exclusive` writes this partial body, leaves it in place, and
    /// returns the error. The export loop has to delete it.
    fail_copy: Mutex<Option<(ErrorKind, Vec<u8>)>>,
    /// `copy_exclusive` writes only this many leading bytes and returns `Ok`.
    short_copy: Mutex<Option<usize>>,
    fail_sync: Mutex<bool>,
    fail_remove_paths: Mutex<Vec<PathBuf>>,
    removed: Mutex<Vec<PathBuf>>,
    copy_count: AtomicU32,
    rename_count: AtomicU32,
}

impl ExportFs {
    pub fn new(case_insensitive: bool) -> Self {
        Self {
            files: Mutex::new(Vec::new()),
            case_insensitive,
            fail_every_rename: Mutex::new(None),
            fail_rename_paths: Mutex::new(Vec::new()),
            fail_copy: Mutex::new(None),
            short_copy: Mutex::new(None),
            fail_sync: Mutex::new(false),
            fail_remove_paths: Mutex::new(Vec::new()),
            removed: Mutex::new(Vec::new()),
            copy_count: AtomicU32::new(0),
            rename_count: AtomicU32::new(0),
        }
    }

    pub fn fail_every_rename(&self, kind: ErrorKind) {
        *self.fail_every_rename.lock().unwrap() = Some(kind);
    }

    pub fn fail_rename_of(&self, path: &str, kind: ErrorKind) {
        self.fail_rename_paths
            .lock()
            .unwrap()
            .push((PathBuf::from(path), kind));
    }

    pub fn fail_copy_after_writing(&self, partial: &[u8], kind: ErrorKind) {
        *self.fail_copy.lock().unwrap() = Some((kind, partial.to_vec()));
    }

    pub fn short_copy(&self, bytes: usize) {
        *self.short_copy.lock().unwrap() = Some(bytes);
    }

    pub fn fail_sync(&self) {
        *self.fail_sync.lock().unwrap() = true;
    }

    pub fn fail_remove_of(&self, path: &str) {
        self.fail_remove_paths
            .lock()
            .unwrap()
            .push(PathBuf::from(path));
    }

    pub fn removed_paths(&self) -> Vec<PathBuf> {
        self.removed.lock().unwrap().clone()
    }

    pub fn copy_count(&self) -> u32 {
        self.copy_count.load(Ordering::SeqCst)
    }

    pub fn rename_count(&self) -> u32 {
        self.rename_count.load(Ordering::SeqCst)
    }

    fn rename_failure_for(&self, from: &Path) -> Option<ErrorKind> {
        let specific = self.fail_rename_paths.lock().unwrap();
        if let Some((_, kind)) = specific
            .iter()
            .find(|(path, _)| self.equivalent(path, from))
        {
            return Some(*kind);
        }
        *self.fail_every_rename.lock().unwrap()
    }

    pub fn insert(&self, path: &str, content: &[u8]) {
        let path = PathBuf::from(path);
        let mut files = self.files.lock().unwrap();
        if let Some(existing) = self.find_mut(&mut files, &path) {
            existing.content = content.to_vec();
            return;
        }
        files.push(StoredFile {
            path,
            content: content.to_vec(),
        });
    }

    pub fn read(&self, path: &str) -> Option<Vec<u8>> {
        let files = self.files.lock().unwrap();
        let path = PathBuf::from(path);
        self.find(&files, &path).map(|file| file.content.clone())
    }

    pub fn names_under(&self, dir: &str) -> Vec<String> {
        let files = self.files.lock().unwrap();
        let dir = PathBuf::from(dir);
        let mut names: Vec<String> = files
            .iter()
            .filter(|file| self.parents_equal(file.path.parent(), Some(dir.as_path())))
            .filter_map(|file| {
                file.path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .collect();
        names.sort();
        names
    }

    fn equivalent(&self, left: &Path, right: &Path) -> bool {
        if self.case_insensitive {
            collision_key(left.as_os_str()) == collision_key(right.as_os_str())
        } else {
            left == right
        }
    }

    fn parents_equal(&self, left: Option<&Path>, right: Option<&Path>) -> bool {
        match (left, right) {
            (Some(left), Some(right)) => self.equivalent(left, right),
            (None, None) => true,
            _ => false,
        }
    }

    fn find<'a>(&self, files: &'a [StoredFile], path: &Path) -> Option<&'a StoredFile> {
        files.iter().find(|file| self.equivalent(&file.path, path))
    }

    fn find_mut<'a>(&self, files: &'a mut [StoredFile], path: &Path) -> Option<&'a mut StoredFile> {
        files
            .iter_mut()
            .find(|file| self.equivalent(&file.path, path))
    }

    fn position(&self, files: &[StoredFile], path: &Path) -> Option<usize> {
        files
            .iter()
            .position(|file| self.equivalent(&file.path, path))
    }
}

#[async_trait]
impl FileSystem for ExportFs {
    async fn get_file_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let files = self.files.lock().unwrap();
        let file = self
            .find(&files, path)
            .ok_or_else(|| io_error(ErrorKind::NotFound, path))?;
        Ok(FileMetadata {
            name: file
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            modified: 0,
            created: 0,
            size: file.content.len() as u64,
        })
    }

    async fn list_images_in_dir(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let files = self.files.lock().unwrap();
        Ok(files
            .iter()
            .filter(|file| self.parents_equal(file.path.parent(), Some(dir)))
            .filter(|file| file.path.extension().is_some())
            .map(|file| file.path.clone())
            .collect())
    }

    async fn list_subfolders(&self, _dir: &Path) -> Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<u64> {
        let mut files = self.files.lock().unwrap();
        let content = self
            .find(&files, from)
            .ok_or_else(|| io_error(ErrorKind::NotFound, from))?
            .content
            .clone();
        let len = content.len() as u64;
        if let Some(existing) = self.find_mut(&mut files, to) {
            existing.content = content;
        } else {
            files.push(StoredFile {
                path: to.to_path_buf(),
                content,
            });
        }
        Ok(len)
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        let mut files = self.files.lock().unwrap();
        let index = self
            .position(&files, from)
            .ok_or_else(|| io_error(ErrorKind::NotFound, from))?;
        let content = files.remove(index).content;
        if let Some(existing) = self.find_mut(&mut files, to) {
            existing.content = content;
        } else {
            files.push(StoredFile {
                path: to.to_path_buf(),
                content,
            });
        }
        Ok(())
    }

    async fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        let files = self.files.lock().unwrap();
        self.find(&files, path)
            .map(|file| file.path.clone())
            .ok_or_else(|| io_error(ErrorKind::NotFound, path))
    }

    async fn create_dir_all(&self, _path: &Path) -> Result<()> {
        Ok(())
    }

    async fn list_dir_names(&self, dir: &Path) -> Result<Vec<OsString>> {
        let files = self.files.lock().unwrap();
        Ok(files
            .iter()
            .filter(|file| self.parents_equal(file.path.parent(), Some(dir)))
            .filter_map(|file| file.path.file_name().map(|name| name.to_os_string()))
            .collect())
    }

    async fn copy_exclusive(&self, from: &Path, to: &Path) -> Result<u64> {
        self.copy_count.fetch_add(1, Ordering::SeqCst);
        // One-shot: the next file in the same export uses a normal copy.
        let fail_copy = self.fail_copy.lock().unwrap().take();
        let short_copy = self.short_copy.lock().unwrap().take();
        let mut files = self.files.lock().unwrap();
        if self.position(&files, to).is_some() {
            return Err(io_error(ErrorKind::AlreadyExists, to));
        }
        let content = self
            .find(&files, from)
            .ok_or_else(|| io_error(ErrorKind::NotFound, from))?
            .content
            .clone();
        if let Some((kind, partial)) = fail_copy {
            files.push(StoredFile {
                path: to.to_path_buf(),
                content: partial,
            });
            return Err(io_error(kind, to));
        }
        let content = match short_copy {
            Some(bytes) => content[..bytes.min(content.len())].to_vec(),
            None => content,
        };
        let len = content.len() as u64;
        files.push(StoredFile {
            path: to.to_path_buf(),
            content,
        });
        Ok(len)
    }

    async fn rename_exclusive(&self, from: &Path, to: &Path) -> Result<()> {
        self.rename_count.fetch_add(1, Ordering::SeqCst);
        if let Some(kind) = self.rename_failure_for(from) {
            return Err(io_error(kind, from));
        }
        let mut files = self.files.lock().unwrap();
        if self.position(&files, to).is_some() {
            return Err(io_error(ErrorKind::AlreadyExists, to));
        }
        let index = self
            .position(&files, from)
            .ok_or_else(|| io_error(ErrorKind::NotFound, from))?;
        let content = files.remove(index).content;
        files.push(StoredFile {
            path: to.to_path_buf(),
            content,
        });
        Ok(())
    }

    async fn file_len(&self, path: &Path) -> Result<u64> {
        let files = self.files.lock().unwrap();
        self.find(&files, path)
            .map(|file| file.content.len() as u64)
            .ok_or_else(|| io_error(ErrorKind::NotFound, path))
    }

    async fn sync_file(&self, path: &Path) -> Result<()> {
        if *self.fail_sync.lock().unwrap() {
            return Err(sync_failure(path));
        }
        let files = self.files.lock().unwrap();
        if self.find(&files, path).is_none() {
            return Err(io_error(ErrorKind::NotFound, path));
        }
        Ok(())
    }

    async fn remove_file(&self, path: &Path) -> Result<()> {
        self.removed.lock().unwrap().push(path.to_path_buf());
        let blocked = self
            .fail_remove_paths
            .lock()
            .unwrap()
            .iter()
            .any(|blocked| self.equivalent(blocked, path));
        if blocked {
            return Err(io_error(ErrorKind::PermissionDenied, path));
        }
        let mut files = self.files.lock().unwrap();
        if let Some(index) = self.position(&files, path) {
            files.remove(index);
        }
        Ok(())
    }
}

fn sync_failure(path: &Path) -> gallery_core::models::GalleryError {
    gallery_core::models::GalleryError::Io(std::io::Error::other(format!(
        "fsync failed for {}",
        path.display()
    )))
}

use async_trait::async_trait;
use gallery_core::fs::FileSystem;
use gallery_core::models::{FileMetadata, GalleryError, Result};
use std::{
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::fs;

fn system_time_secs(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// Modified time in seconds, or the Unix epoch when the filesystem has none.
fn resolve_modified_secs(modified: io::Result<SystemTime>) -> u64 {
    modified.map(system_time_secs).unwrap_or(0)
}

/// Birth time when the filesystem records one, otherwise `modified_secs`.
fn resolve_created_secs(created: io::Result<SystemTime>, modified_secs: u64) -> u64 {
    match created {
        Ok(time) => system_time_secs(time),
        Err(_) => modified_secs,
    }
}

fn log_creation_time_fallback(path: &Path, error: &io::Error) {
    static LOGGED: AtomicBool = AtomicBool::new(false);
    if LOGGED.swap(true, Ordering::Relaxed) {
        log::debug!(
            "Creation time unavailable for {}: {}. Using modified time.",
            path.display(),
            error
        );
    } else {
        log::warn!(
            "Creation time unavailable for {}: {}. Using modified time. Further files are logged at debug level.",
            path.display(),
            error
        );
    }
}

pub struct LocalFileSystem;

#[async_trait]
impl FileSystem for LocalFileSystem {
    async fn get_file_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let metadata = fs::metadata(path).await?;
        let size = metadata.len();

        let modified_result = metadata.modified();
        if let Err(error) = &modified_result {
            log::warn!(
                "Modification time unavailable for {}: {}. Using the Unix epoch.",
                path.display(),
                error
            );
        }
        let modified = resolve_modified_secs(modified_result);

        let created_result = metadata.created();
        if let Err(error) = &created_result {
            log_creation_time_fallback(path, error);
        }
        let created = resolve_created_secs(created_result, modified);

        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                GalleryError::InvalidPath(format!("Could not get file name for {}", path.display()))
            })?
            .to_string();

        Ok(FileMetadata {
            name,
            size,
            modified,
            created,
        })
    }

    async fn list_images_in_dir(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut entries = fs::read_dir(dir).await?;
        let mut images = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if matches!(
                    ext.to_lowercase().as_str(),
                    "jpg"
                        | "jpeg"
                        | "png"
                        | "webp"
                        | "bmp"
                        | "gif"
                        | "tiff"
                        | "tif"
                        | "avif"
                        | "heic"
                        | "heif"
                        | "svg"
                ) {
                    images.push(path);
                }
            }
        }
        Ok(images)
    }

    async fn list_subfolders(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut entries = fs::read_dir(dir).await?;
        let mut folders = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            }
        }
        Ok(folders)
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<u64> {
        fs::copy(from, to).await.map_err(|e| GalleryError::Io(e))
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        fs::rename(from, to).await.map_err(|e| GalleryError::Io(e))
    }

    async fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        fs::canonicalize(path)
            .await
            .map_err(|e| GalleryError::Io(e))
    }

    async fn create_dir_all(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path)
            .await
            .map_err(|e| GalleryError::Io(e))
    }
}

pub async fn get_file_metadata<P: AsRef<Path>>(path: P) -> Result<FileMetadata> {
    LocalFileSystem.get_file_metadata(path.as_ref()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_list_images_in_dir() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.jpg");
        File::create(&file_path).unwrap();

        let other_path = dir.path().join("test.txt");
        File::create(&other_path).unwrap();

        let fs = LocalFileSystem;
        let images = fs.list_images_in_dir(dir.path()).await.unwrap();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].file_name().unwrap(), "test.jpg");
    }

    #[tokio::test]
    async fn test_get_file_metadata() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("meta.png");
        File::create(&file_path).unwrap();

        let fs = LocalFileSystem;
        let metadata = fs.get_file_metadata(&file_path).await.unwrap();

        assert_eq!(metadata.name, "meta.png");
        assert!(metadata.size == 0);
    }

    #[test]
    fn created_secs_use_modified_then_epoch() {
        let missing = || {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "creation time is not available for the filesystem",
            ))
        };

        assert_eq!(
            resolve_created_secs(missing(), 1_700_000_000),
            1_700_000_000
        );
        assert_eq!(resolve_modified_secs(missing()), 0);
        assert_eq!(
            resolve_created_secs(missing(), resolve_modified_secs(missing())),
            0
        );

        let created = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(42);
        assert_eq!(resolve_created_secs(Ok(created), 7), 42);
    }

    /// Filesystems without a birth time used to fail `created()` and drop the file.
    #[tokio::test]
    async fn creation_time_falls_back_to_modified_time() {
        let path = Path::new("/proc/self/stat");
        let Ok(std_meta) = std::fs::metadata(path) else {
            return;
        };
        if std_meta.created().is_ok() {
            return;
        }

        let modified = std_meta
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or(0);

        let metadata = LocalFileSystem
            .get_file_metadata(path)
            .await
            .expect("a readable file must not be dropped when birth time is missing");

        assert_eq!(metadata.name, "stat");
        assert_eq!(metadata.modified, modified);
        assert_eq!(metadata.created, modified);
    }
}

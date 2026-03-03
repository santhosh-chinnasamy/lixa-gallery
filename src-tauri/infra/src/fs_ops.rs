use gallery_core::fs::FileSystem;
use gallery_core::models::FileMetadata;
use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

pub struct LocalFileSystem;

impl FileSystem for LocalFileSystem {
    fn get_file_metadata(&self, path: &Path) -> std::io::Result<FileMetadata> {
        let metadata = fs::metadata(path)?;
        let size = metadata.len();
        let modified = metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let created = metadata
            .created()?
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Could not get file name for {}", path.display()),
                )
            })?
            .to_string();

        Ok(FileMetadata {
            name,
            size,
            modified,
            created,
        })
    }

    fn list_images_in_dir(&self, dir: &Path) -> std::io::Result<Vec<PathBuf>> {
        let entries = fs::read_dir(dir)?;
        let images = entries
            .filter_map(|e| e.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if path.is_dir() {
                    return None;
                }
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if matches!(
                        ext.to_lowercase().as_str(),
                        "jpg" | "jpeg" | "png" | "webp" | "bmp" | "gif"
                    ) {
                        return Some(path);
                    }
                }
                None
            })
            .collect();
        Ok(images)
    }

    fn copy(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        fs::copy(from, to)
    }

    fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf> {
        fs::canonicalize(path)
    }
}

// Deprecated: migrate usages to LocalFileSystem
pub fn get_file_metadata<P: AsRef<Path>>(path: P) -> std::io::Result<FileMetadata> {
    LocalFileSystem.get_file_metadata(path.as_ref())
}

pub fn list_images_in_dir(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    LocalFileSystem.list_images_in_dir(dir)
}

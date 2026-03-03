use crate::domain::models::FileMetadata;
use std::path::{Path, PathBuf};

pub trait FileSystem: Send + Sync {
    fn get_file_metadata(&self, path: &Path) -> std::io::Result<FileMetadata>;
    fn list_images_in_dir(&self, dir: &Path) -> std::io::Result<Vec<PathBuf>>;
    fn copy(&self, from: &Path, to: &Path) -> std::io::Result<u64>;
    fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf>;
}

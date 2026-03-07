use crate::models::{FileMetadata, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

#[async_trait]
pub trait FileSystem: Send + Sync {
    async fn get_file_metadata(&self, path: &Path) -> Result<FileMetadata>;
    async fn list_images_in_dir(&self, dir: &Path) -> Result<Vec<PathBuf>>;
    async fn list_subfolders(&self, dir: &Path) -> Result<Vec<PathBuf>>;
    async fn copy(&self, from: &Path, to: &Path) -> Result<u64>;
    async fn canonicalize(&self, path: &Path) -> Result<PathBuf>;
}

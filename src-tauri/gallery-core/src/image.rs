use crate::models::{PhotoMetadata, Result};
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[async_trait]
pub trait ImageProcessor: Send + Sync {
    async fn convert_image(&self, file_path: &Path, thumbnail_dir: &str) -> Result<PhotoMetadata>;
    fn get_semaphore(&self) -> Option<Arc<Semaphore>>;
}

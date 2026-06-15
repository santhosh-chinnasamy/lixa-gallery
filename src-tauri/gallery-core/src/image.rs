use crate::models::{PhotoMetadata, Result};
use async_trait::async_trait;
use std::path::Path;

#[async_trait]
pub trait ImageProcessor: Send + Sync {
    async fn convert_image(&self, file_path: &Path, thumbnail_dir: &str) -> Result<PhotoMetadata>;
}

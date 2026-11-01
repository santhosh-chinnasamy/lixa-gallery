use crate::models::{ExifInfo, PhotoMetadata, Result};
use async_trait::async_trait;
use std::path::Path;

#[async_trait]
pub trait ImageProcessor: Send + Sync {
    async fn convert_image(&self, file_path: &Path, thumbnail_dir: &str) -> Result<PhotoMetadata>;

    /// Read shooting settings without generating a thumbnail.
    /// Lazy scans use this so the info panel and date sort have capture time
    /// before background thumbnail work finishes.
    async fn read_exif(&self, file_path: &Path) -> Result<Option<ExifInfo>>;
}

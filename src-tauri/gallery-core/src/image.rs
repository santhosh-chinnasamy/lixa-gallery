use crate::models::{ExifInfo, PhotoMetadata, Result};
use crate::work::StaleCheck;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

#[async_trait]
pub trait ImageProcessor: Send + Sync {
    async fn convert_image(&self, file_path: &Path, thumbnail_dir: &str) -> Result<PhotoMetadata>;

    /// `convert_image` with a folder-session handle.
    ///
    /// Implementations may run long stages at a lower OS priority once the
    /// folder that asked for the work has been replaced. The default keeps
    /// processors that do no scheduling work unchanged.
    async fn convert_image_tracked(
        &self,
        file_path: &Path,
        thumbnail_dir: &str,
        _stale: StaleCheck,
    ) -> Result<PhotoMetadata> {
        self.convert_image(file_path, thumbnail_dir).await
    }

    /// Read shooting settings without generating a thumbnail.
    /// Lazy scans use this so the info panel and date sort have capture time
    /// before background thumbnail work finishes.
    async fn read_exif(&self, file_path: &Path) -> Result<Option<ExifInfo>>;

    /// Decode `source` into a WebP the webview can paint (viewer `kind=preview`).
    async fn ensure_decoded_preview(&self, source: &Path, cache_dir: &Path) -> Result<PathBuf>;
}

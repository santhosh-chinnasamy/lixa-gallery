use crate::domain::models::PhotoMetadata;
use std::path::Path;

pub trait ImageProcessor: Send + Sync {
    fn convert_image(&self, file_path: &Path, thumbnail_dir: &str)
        -> anyhow::Result<PhotoMetadata>;
}

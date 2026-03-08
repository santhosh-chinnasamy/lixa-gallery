use crate::fs_ops;
use async_trait::async_trait;
use gallery_core::image::ImageProcessor as ImageProcessorTrait;
use gallery_core::models::{GalleryError, PhotoMetadata, Result};
use std::path::{Path, PathBuf};

pub struct ImageProcessor;

#[async_trait]
impl ImageProcessorTrait for ImageProcessor {
    async fn convert_image(&self, file_path: &Path, thumbnail_dir: &str) -> Result<PhotoMetadata> {
        let path = file_path.to_path_buf();
        let thumb_dir = thumbnail_dir.to_string();

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Handle vector/web-native formats that don't need raster thumbnails
        if ext == "svg" || ext == "webp" || ext == "gif" {
            let metadata = fs_ops::get_file_metadata(file_path).await?;
            return Ok(PhotoMetadata {
                metadata,
                path: file_path.to_string_lossy().into_owned(),
                thumbnail_path: file_path.to_string_lossy().into_owned(),
            });
        }

        let output_path_result = tokio::task::spawn_blocking(move || {
            let img = match image::open(&path) {
                Ok(img) => img,
                Err(e) => {
                    log::warn!(
                        "Failed to open image {}: {}. Falling back to original.",
                        path.display(),
                        e
                    );
                    return Ok::<Option<PathBuf>, GalleryError>(None);
                }
            };

            let (width, height) = (img.width(), img.height());
            let max_size = 512;
            let (thumb_width, thumb_height) = if width > height {
                let ratio = max_size as f32 / width as f32;
                (max_size, (height as f32 * ratio) as u32)
            } else {
                let ratio = max_size as f32 / height as f32;
                ((width as f32 * ratio) as u32, max_size)
            };

            let thumbnail = img.resize(
                thumb_width,
                thumb_height,
                image::imageops::FilterType::CatmullRom,
            );

            let file_stem = path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
                GalleryError::InvalidPath(format!("Could not get file stem for {}", path.display()))
            })?;

            let output_filename = format!("{}.webp", file_stem);
            let output_path = PathBuf::from(thumb_dir).join(&output_filename);

            thumbnail
                .save(&output_path)
                .map_err(|e| GalleryError::Image(e.to_string()))?;

            Ok::<Option<PathBuf>, GalleryError>(Some(output_path))
        })
        .await
        .map_err(|e| GalleryError::Unknown(e.to_string()))?;

        let thumbnail_path = match output_path_result {
            Ok(Some(p)) => p.to_string_lossy().into_owned(),
            _ => file_path.to_string_lossy().into_owned(), // Fallback to original
        };

        let metadata = fs_ops::get_file_metadata(file_path).await?;

        Ok(PhotoMetadata {
            metadata,
            path: file_path.to_string_lossy().into_owned(),
            thumbnail_path,
        })
    }
}

use crate::exif_info;
use crate::fs_ops;
use crate::raw_preview;
use async_trait::async_trait;
use gallery_core::formats::{extension, is_raw_path};
use gallery_core::image::ImageProcessor as ImageProcessorTrait;
use gallery_core::models::{ExifInfo, GalleryError, PhotoMetadata, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct RayonImageProcessor {
    pool: Arc<rayon::ThreadPool>,
}

impl RayonImageProcessor {
    pub fn new() -> Self {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .thread_name(|i| format!("rayon-img-{}", i))
            .build()
            .expect("Failed to create Rayon thread pool");
        Self {
            pool: Arc::new(pool),
        }
    }
}

struct Rendered {
    thumbnail_path: String,
    preview_path: Option<String>,
    exif: ExifInfo,
}

#[async_trait]
impl ImageProcessorTrait for RayonImageProcessor {
    async fn convert_image(&self, file_path: &Path, thumbnail_dir: &str) -> Result<PhotoMetadata> {
        let path = file_path.to_path_buf();
        let thumb_dir = thumbnail_dir.to_string();
        let pool = self.pool.clone();

        let rendered =
            tokio::task::spawn_blocking(move || pool.install(|| render_photo(&path, &thumb_dir)))
                .await
                .map_err(|err| GalleryError::Unknown(err.to_string()))??;

        let metadata = fs_ops::get_file_metadata(file_path).await?;
        Ok(PhotoMetadata {
            metadata,
            path: file_path.to_string_lossy().into_owned(),
            thumbnail_path: rendered.thumbnail_path,
            sidecar_path: None,
            preview_path: rendered.preview_path,
            exif: Some(rendered.exif),
            is_raw: is_raw_path(file_path),
        })
    }

    async fn read_exif(&self, file_path: &Path) -> Result<Option<ExifInfo>> {
        let path = file_path.to_path_buf();
        let info = tokio::task::spawn_blocking(move || exif_info::from_path(&path))
            .await
            .map_err(|err| GalleryError::Unknown(err.to_string()))?;
        Ok(Some(info))
    }
}

fn render_photo(path: &Path, thumb_dir: &str) -> Result<Rendered> {
    // RAW is handled on its own so the JPEG/TIFF path below stays the flat
    // `{stem}.webp` write. Orientation for ordinary files is owned elsewhere.
    if is_raw_path(path) {
        let raw = raw_preview::render_raw(path, thumb_dir)?;
        return Ok(Rendered {
            thumbnail_path: raw.thumbnail_path,
            preview_path: raw.preview_path,
            exif: raw.exif,
        });
    }

    let exif = exif_info::from_path(path);
    let ext = extension(path);
    if matches!(ext.as_str(), "svg" | "webp" | "gif") {
        return Ok(Rendered {
            thumbnail_path: path.to_string_lossy().into_owned(),
            preview_path: None,
            exif,
        });
    }

    let thumbnail_path = match write_raster_thumbnail(path, thumb_dir) {
        Ok(Some(output)) => output.to_string_lossy().into_owned(),
        _ => path.to_string_lossy().into_owned(),
    };
    Ok(Rendered {
        thumbnail_path,
        preview_path: None,
        exif,
    })
}

fn write_raster_thumbnail(path: &Path, thumb_dir: &str) -> Result<Option<PathBuf>> {
    let img = match image::open(path) {
        Ok(img) => img,
        Err(err) => {
            log::warn!(
                "Failed to open image {}: {}. Falling back to original.",
                path.display(),
                err
            );
            return Ok(None);
        }
    };

    let thumbnail = img.thumbnail(512, 512);
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| {
            GalleryError::InvalidPath(format!("Could not get file stem for {}", path.display()))
        })?;
    let output = PathBuf::from(thumb_dir).join(format!("{stem}.webp"));
    thumbnail
        .save(&output)
        .map_err(|err| GalleryError::Image(err.to_string()))?;
    Ok(Some(output))
}

use crate::fs_ops;
use crate::thumbnail_cache::{self, ThumbnailIdentity};
use async_trait::async_trait;
use gallery_core::image::ImageProcessor as ImageProcessorTrait;
use gallery_core::models::{GalleryError, PhotoMetadata, Result};
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

#[async_trait]
impl ImageProcessorTrait for RayonImageProcessor {
    async fn convert_image(&self, file_path: &Path, thumbnail_dir: &str) -> Result<PhotoMetadata> {
        let ext = file_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        let metadata = fs_ops::get_file_metadata(file_path).await?;
        let source_path = file_path.to_string_lossy().into_owned();

        // Vector and web-native formats are served directly. They never occupy
        // a shared `{stem}.webp` slot in the thumbnail cache.
        if ext == "svg" || ext == "webp" || ext == "gif" {
            return Ok(PhotoMetadata {
                metadata,
                path: source_path.clone(),
                thumbnail_path: source_path,
            });
        }

        let identity = ThumbnailIdentity {
            mtime_secs: metadata.modified,
            size: metadata.size,
        };
        let output_path =
            thumbnail_cache::thumbnail_disk_path(Path::new(thumbnail_dir), file_path, identity);
        if output_path.is_file() {
            return Ok(PhotoMetadata {
                metadata,
                path: source_path,
                thumbnail_path: output_path.to_string_lossy().into_owned(),
            });
        }

        // Decode off the async executor. The output path is already unique to
        // this source path and identity, so two folders cannot overwrite it.
        let path = file_path.to_path_buf();
        let pool = self.pool.clone();
        let output_path_result = tokio::task::spawn_blocking(move || {
            pool.install(|| {
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

                let thumbnail = img.thumbnail(512, 512);
                write_webp_atomic(&thumbnail, &output_path)?;
                if let Err(err) = thumbnail_cache::remove_other_identity_files(&output_path) {
                    log::warn!(
                        "Could not remove stale thumbnails beside {}: {err}",
                        output_path.display()
                    );
                }
                Ok::<Option<PathBuf>, GalleryError>(Some(output_path))
            })
        })
        .await
        .map_err(|e| GalleryError::Unknown(e.to_string()))?;

        let thumbnail_path = match output_path_result {
            Ok(Some(path)) => path.to_string_lossy().into_owned(),
            Ok(None) => source_path.clone(),
            Err(err) => {
                log::warn!(
                    "Failed to write thumbnail for {}: {err}. Serving the original.",
                    file_path.display()
                );
                source_path.clone()
            }
        };

        Ok(PhotoMetadata {
            metadata,
            path: source_path,
            thumbnail_path,
        })
    }
}

fn write_webp_atomic(image: &image::DynamicImage, dest: &Path) -> Result<()> {
    let parent = dest.parent().ok_or_else(|| {
        GalleryError::InvalidPath(format!("thumbnail path has no parent: {}", dest.display()))
    })?;
    std::fs::create_dir_all(parent)?;
    let file_name = dest
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("thumb.webp");
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let tmp = parent.join(format!(".{file_name}.{}.{unique}.tmp", std::process::id()));
    if let Err(err) = image.save_with_format(&tmp, image::ImageFormat::WebP) {
        let _ = std::fs::remove_file(&tmp);
        return Err(GalleryError::Image(err.to_string()));
    }
    if let Err(err) = rename_overwrite(&tmp, dest) {
        let _ = std::fs::remove_file(&tmp);
        return Err(GalleryError::Io(err));
    }
    Ok(())
}

fn rename_overwrite(from: &Path, to: &Path) -> std::io::Result<()> {
    if let Err(err) = std::fs::rename(from, to) {
        if to.exists() {
            std::fs::remove_file(to)?;
            std::fs::rename(from, to)?;
        } else {
            return Err(err);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::time::{Duration, SystemTime};

    fn write_solid(path: &Path, color: [u8; 3], side: u32) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut image = image::RgbImage::new(side, side);
        for pixel in image.pixels_mut() {
            *pixel = image::Rgb(color);
        }
        let format = match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref()
        {
            Some("png") => image::ImageFormat::Png,
            _ => image::ImageFormat::Jpeg,
        };
        image.save_with_format(path, format).unwrap();
    }

    fn closer_to(path: &Path, color: [u8; 3]) -> bool {
        let image = image::open(path).unwrap().into_rgb8();
        let pixel = image.get_pixel(image.width() / 2, image.height() / 2);
        let distance = |sample: [u8; 3]| {
            let red = pixel[0] as i32 - sample[0] as i32;
            let green = pixel[1] as i32 - sample[1] as i32;
            let blue = pixel[2] as i32 - sample[2] as i32;
            red * red + green * green + blue * blue
        };
        distance(color)
            < distance(if color[0] > color[2] {
                [0, 0, 255]
            } else {
                [255, 0, 0]
            })
    }

    fn webp_files(dir: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&current) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().and_then(|ext| ext.to_str()) == Some("webp") {
                    found.push(path);
                }
            }
        }
        found
    }

    #[tokio::test]
    async fn same_filename_in_different_folders_do_not_share_a_thumbnail() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("cache");
        let red = root.path().join("A").join("IMG_0001.JPG");
        let blue = root.path().join("B").join("IMG_0001.JPG");
        write_solid(&red, [255, 0, 0], 8);
        write_solid(&blue, [0, 0, 255], 8);

        let processor = RayonImageProcessor::new();
        let cache_dir = cache.to_str().unwrap();
        let left = processor.convert_image(&red, cache_dir).await.unwrap();
        let right = processor.convert_image(&blue, cache_dir).await.unwrap();

        assert_ne!(left.thumbnail_path, right.thumbnail_path);
        assert_ne!(
            std::fs::read(&left.thumbnail_path).unwrap(),
            std::fs::read(&right.thumbnail_path).unwrap()
        );
        assert!(closer_to(Path::new(&left.thumbnail_path), [255, 0, 0]));
        assert!(closer_to(Path::new(&right.thumbnail_path), [0, 0, 255]));
        assert_eq!(webp_files(&cache).len(), 2);
    }

    #[tokio::test]
    async fn same_stem_with_different_extensions_do_not_share_a_thumbnail() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("cache");
        let jpeg = root.path().join("card").join("a.jpg");
        let png = root.path().join("card").join("a.png");
        write_solid(&jpeg, [255, 0, 0], 8);
        write_solid(&png, [0, 0, 255], 8);

        let processor = RayonImageProcessor::new();
        let cache_dir = cache.to_str().unwrap();
        let left = processor.convert_image(&jpeg, cache_dir).await.unwrap();
        let right = processor.convert_image(&png, cache_dir).await.unwrap();

        assert_ne!(left.thumbnail_path, right.thumbnail_path);
        assert!(closer_to(Path::new(&left.thumbnail_path), [255, 0, 0]));
        assert!(closer_to(Path::new(&right.thumbnail_path), [0, 0, 255]));
    }

    #[tokio::test]
    async fn replacing_a_file_regenerates_its_thumbnail_and_drops_the_old_one() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("cache");
        let source = root.path().join("shoot").join("IMG_0001.JPG");
        write_solid(&source, [255, 0, 0], 8);

        let processor = RayonImageProcessor::new();
        let cache_dir = cache.to_str().unwrap();
        let first = processor.convert_image(&source, cache_dir).await.unwrap();
        let first_bytes = std::fs::read(&first.thumbnail_path).unwrap();
        assert!(closer_to(Path::new(&first.thumbnail_path), [255, 0, 0]));

        write_solid(&source, [0, 0, 255], 24);
        let mut file = File::options().write(true).open(&source).unwrap();
        file.set_modified(SystemTime::now() + Duration::from_secs(180))
            .unwrap();
        file.flush().unwrap();
        drop(file);

        let before = ThumbnailIdentity::read(&source).unwrap();
        let first_identity = ThumbnailIdentity {
            mtime_secs: first.metadata.modified,
            size: first.metadata.size,
        };
        assert_ne!(
            (first_identity.mtime_secs, first_identity.size),
            (before.mtime_secs, before.size)
        );

        let second = processor.convert_image(&source, cache_dir).await.unwrap();
        assert_ne!(first.thumbnail_path, second.thumbnail_path);
        assert!(!Path::new(&first.thumbnail_path).exists());
        assert_ne!(first_bytes, std::fs::read(&second.thumbnail_path).unwrap());
        assert!(closer_to(Path::new(&second.thumbnail_path), [0, 0, 255]));
        assert_eq!(webp_files(&cache).len(), 1);

        let third = processor.convert_image(&source, cache_dir).await.unwrap();
        assert_eq!(second.thumbnail_path, third.thumbnail_path);
        assert_eq!(webp_files(&cache).len(), 1);
    }
}

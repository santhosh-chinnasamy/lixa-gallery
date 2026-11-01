use crate::fs_ops;
use crate::thumbnail_cache::{self, ThumbnailIdentity};
use async_trait::async_trait;
use gallery_core::image::ImageProcessor as ImageProcessorTrait;
use gallery_core::models::{GalleryError, PhotoMetadata, Result};
use image::{GenericImageView, ImageDecoder};
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
                let img = match open_oriented(&path) {
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

/// Longest edge of a viewer preview. Smaller frames are not enlarged.
const PREVIEW_MAX_EDGE: u32 = 2048;

/// Decode `source` and write a WebP the webview can paint.
///
/// Used for formats the platform webview cannot render, such as TIFF on
/// Windows and Linux. The file is cached next to grid thumbnails and replaced
/// when the source mtime or size changes.
pub async fn ensure_decoded_preview(source: &Path, cache_dir: &Path) -> Result<PathBuf> {
    let identity = ThumbnailIdentity::read(source)?;
    let output = thumbnail_cache::preview_disk_path(cache_dir, source, identity);
    if output.is_file() {
        return Ok(output);
    }

    let source = source.to_path_buf();
    let output_for_task = output.clone();
    tokio::task::spawn_blocking(move || {
        let image = open_oriented(&source).map_err(GalleryError::Image)?;
        let preview = fit_within(image, PREVIEW_MAX_EDGE);
        write_webp_atomic(&preview, &output_for_task)?;
        if let Err(err) = thumbnail_cache::remove_other_identity_files(&output_for_task) {
            log::warn!(
                "Could not remove stale previews beside {}: {err}",
                output_for_task.display()
            );
        }
        Ok(output_for_task)
    })
    .await
    .map_err(|err| GalleryError::Unknown(err.to_string()))?
}

/// `image::open` leaves pixels in sensor order. Cameras store the display
/// rotation in EXIF Orientation (and TIFF stores the same tag). Apply it
/// before any resize so a portrait frame is not baked in sideways. The WebP
/// we write has no EXIF of its own.
fn open_oriented(path: &Path) -> std::result::Result<image::DynamicImage, String> {
    let reader = image::ImageReader::open(path).map_err(|err| err.to_string())?;
    let mut decoder = reader
        .with_guessed_format()
        .map_err(|err| err.to_string())?
        .into_decoder()
        .map_err(|err| err.to_string())?;
    let orientation = decoder.orientation().map_err(|err| err.to_string())?;
    let mut image = image::DynamicImage::from_decoder(decoder).map_err(|err| err.to_string())?;
    image.apply_orientation(orientation);
    Ok(image)
}

fn fit_within(image: image::DynamicImage, max_edge: u32) -> image::DynamicImage {
    let (width, height) = image.dimensions();
    if width <= max_edge && height <= max_edge {
        return image;
    }
    image.thumbnail(max_edge, max_edge)
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
    use image::{ImageEncoder, Pixel};
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

    fn quadrants(width: u32, height: u32) -> image::RgbImage {
        let mut image = image::RgbImage::new(width, height);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let left = x < width / 2;
            let top = y < height / 2;
            *pixel = image::Rgb(match (left, top) {
                (true, true) => [255, 0, 0],
                (false, true) => [0, 255, 0],
                (true, false) => [0, 0, 255],
                (false, false) => [255, 255, 255],
            });
        }
        image
    }

    /// The eight EXIF orientation values, in the same order `image` applies them.
    fn orient_like_exif(image: &image::DynamicImage, orientation: u8) -> image::DynamicImage {
        match orientation {
            1 => image.clone(),
            2 => image.fliph(),
            3 => image.rotate180(),
            4 => image.flipv(),
            5 => image.rotate90().fliph(),
            6 => image.rotate90(),
            7 => image.rotate270().fliph(),
            8 => image.rotate270(),
            _ => panic!("orientation {orientation} is not an EXIF value"),
        }
    }

    fn exif_app1(orientation: u8) -> Vec<u8> {
        let mut tiff = Vec::new();
        tiff.extend_from_slice(b"II");
        tiff.extend_from_slice(&42u16.to_le_bytes());
        tiff.extend_from_slice(&8u32.to_le_bytes());
        tiff.extend_from_slice(&1u16.to_le_bytes());
        tiff.extend_from_slice(&0x0112u16.to_le_bytes());
        tiff.extend_from_slice(&3u16.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());
        tiff.extend_from_slice(&(orientation as u16).to_le_bytes());
        tiff.extend_from_slice(&0u16.to_le_bytes());
        tiff.extend_from_slice(&0u32.to_le_bytes());

        let mut segment = Vec::new();
        segment.extend_from_slice(&[0xFF, 0xE1]);
        let length = (2 + 6 + tiff.len()) as u16;
        segment.extend_from_slice(&length.to_be_bytes());
        segment.extend_from_slice(b"Exif\0\0");
        segment.extend_from_slice(&tiff);
        segment
    }

    fn write_oriented_jpeg(path: &Path, image: &image::RgbImage, orientation: u8) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut encoded = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 100);
        encoder
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                image::ExtendedColorType::Rgb8,
            )
            .unwrap();
        assert_eq!(&encoded[..2], &[0xFF, 0xD8]);
        let mut jpeg = Vec::with_capacity(encoded.len() + 40);
        jpeg.extend_from_slice(&[0xFF, 0xD8]);
        jpeg.extend_from_slice(&exif_app1(orientation));
        jpeg.extend_from_slice(&encoded[2..]);
        std::fs::write(path, jpeg).unwrap();
    }

    fn fitted_thumbnail_size(width: u32, height: u32) -> (u32, u32) {
        let ratio = (512.0 / f64::from(width)).min(512.0 / f64::from(height));
        let fitted_width = (f64::from(width) * ratio).round().max(1.0) as u32;
        let fitted_height = (f64::from(height) * ratio).round().max(1.0) as u32;
        (fitted_width, fitted_height)
    }

    fn assert_pixel_is(pixel: image::Rgb<u8>, expected: [u8; 3]) {
        let distance = |sample: [u8; 3]| {
            let red = i32::from(pixel[0]) - i32::from(sample[0]);
            let green = i32::from(pixel[1]) - i32::from(sample[1]);
            let blue = i32::from(pixel[2]) - i32::from(sample[2]);
            red * red + green * green + blue * blue
        };
        for other in [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255]] {
            if other == expected {
                continue;
            }
            assert!(
                distance(expected) < distance(other),
                "pixel {pixel:?} is closer to {other:?} than {expected:?}"
            );
        }
    }

    #[test]
    fn exif_orientation_changes_the_decoded_pixel_layout() {
        let root = tempfile::tempdir().unwrap();
        let source = quadrants(64, 32);
        for orientation in 1..=8u8 {
            let path = root.path().join(format!("orient-{orientation}.jpg"));
            write_oriented_jpeg(&path, &source, orientation);

            // image::open keeps sensor order. A thumbnail that used it directly
            // would stay 64x32 for every orientation tag.
            let sensor = image::open(&path).unwrap();
            assert_eq!(sensor.dimensions(), (64, 32));

            let expected = orient_like_exif(&sensor, orientation);
            let actual = open_oriented(&path).unwrap();
            assert_eq!(actual.dimensions(), expected.dimensions());
            assert_eq!(actual.to_rgb8().into_raw(), expected.to_rgb8().into_raw());
        }
    }

    #[tokio::test]
    async fn each_exif_orientation_is_applied_before_the_thumbnail_is_resized() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("cache");
        let source = quadrants(64, 32);
        let visual = image::DynamicImage::ImageRgb8(source.clone());
        let processor = RayonImageProcessor::new();
        let cache_dir = cache.to_str().unwrap();

        for orientation in 1..=8u8 {
            let path = root.path().join(format!("grid-{orientation}.jpg"));
            write_oriented_jpeg(&path, &source, orientation);
            let oriented = orient_like_exif(&visual, orientation);
            let (expected_width, expected_height) =
                fitted_thumbnail_size(oriented.width(), oriented.height());

            let metadata = processor.convert_image(&path, cache_dir).await.unwrap();
            let thumb = image::open(&metadata.thumbnail_path).unwrap().to_rgb8();
            assert_eq!(
                (thumb.width(), thumb.height()),
                (expected_width, expected_height),
                "orientation {orientation}"
            );
            if matches!(orientation, 5..=8) {
                assert!(
                    thumb.height() > thumb.width(),
                    "orientation {orientation} must swap the landscape sensor frame"
                );
            }

            for (x_num, y_num) in [(1u32, 1u32), (3, 1), (1, 3), (3, 3)] {
                let x = thumb.width() * x_num / 4;
                let y = thumb.height() * y_num / 4;
                let source_x = x * oriented.width() / thumb.width();
                let source_y = y * oriented.height() / thumb.height();
                let expected = oriented.get_pixel(source_x, source_y).to_rgb();
                let actual = *thumb.get_pixel(x, y);
                assert_pixel_is(actual, [expected[0], expected[1], expected[2]]);
            }
        }
    }

    #[test]
    fn preview_keeps_small_frames_and_caps_the_long_edge() {
        let small = fit_within(image::DynamicImage::new_rgb8(20, 10), PREVIEW_MAX_EDGE);
        assert_eq!(small.dimensions(), (20, 10));
        let wide = fit_within(image::DynamicImage::new_rgb8(4000, 1000), PREVIEW_MAX_EDGE);
        assert_eq!(wide.dimensions(), (PREVIEW_MAX_EDGE, 512));
    }

    #[tokio::test]
    async fn tiff_preview_is_a_decoded_webp_and_not_the_grid_thumbnail() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("cache");
        let tiff = root.path().join("scan.tiff");
        let mut image = image::RgbImage::new(20, 10);
        for pixel in image.pixels_mut() {
            *pixel = image::Rgb([220, 20, 20]);
        }
        image
            .save_with_format(&tiff, image::ImageFormat::Tiff)
            .unwrap();

        let preview = ensure_decoded_preview(&tiff, &cache).await.unwrap();
        let bytes = std::fs::read(&preview).unwrap();
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WEBP");

        let decoded = image::open(&preview).unwrap();
        assert_eq!(
            decoded.dimensions(),
            (20, 10),
            "a viewer preview must not be the upscaled 512px grid thumbnail"
        );
        assert!(closer_to(&preview, [220, 20, 20]));
        assert!(preview.starts_with(cache.join(gallery_core::thumbnail_cache::CACHE_DIR_NAME)));

        let identity = ThumbnailIdentity::read(&tiff).unwrap();
        let thumb = thumbnail_cache::thumbnail_disk_path(&cache, &tiff, identity);
        assert_ne!(preview, thumb);

        let again = ensure_decoded_preview(&tiff, &cache).await.unwrap();
        assert_eq!(preview, again);
        assert_eq!(webp_files(&cache).len(), 1);
    }
}

//! Embedded JPEG previews for camera RAW files.
//!
//! This does not demosaic. It finds the largest displayable JPEG already
//! stored in the file (the preview culling tools use) and bakes that JPEG's
//! EXIF orientation into the pixels. A file with no such preview is skipped
//! without failing the scan.

use crate::exif_info;
use gallery_core::models::{ExifInfo, GalleryError, Result};
use image::metadata::Orientation;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub struct EmbeddedJpeg {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub struct RawRender {
    pub thumbnail_path: String,
    pub preview_path: Option<String>,
    pub exif: ExifInfo,
}

struct OutputPaths {
    thumbnail: PathBuf,
    preview: PathBuf,
    missing: PathBuf,
}

/// `{stem}.webp` matches the flat thumbnail names already written for JPEG.
/// `{stem}.preview.jpg` is only the viewer file. No cache-version directory:
/// that layout belongs to the open orientation work.
fn output_paths(source: &Path, thumb_dir: &str) -> Result<OutputPaths> {
    let stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| {
            GalleryError::InvalidPath(format!("Could not get file stem for {}", source.display()))
        })?;
    let dir = PathBuf::from(thumb_dir);
    Ok(OutputPaths {
        thumbnail: dir.join(format!("{stem}.webp")),
        preview: dir.join(format!("{stem}.preview.jpg")),
        missing: dir.join(format!("{stem}.no-preview")),
    })
}

pub fn render_raw(source: &Path, thumb_dir: &str) -> Result<RawRender> {
    let paths = output_paths(source, thumb_dir)?;
    std::fs::create_dir_all(thumb_dir).map_err(GalleryError::Io)?;

    if fresh(&paths.missing, source) {
        // The first pass already recorded that this file has no preview.
        // Returning here avoids reading the RAW again for every grid cell.
        return Ok(RawRender {
            thumbnail_path: String::new(),
            preview_path: None,
            exif: ExifInfo::default(),
        });
    }

    if fresh(&paths.preview, source) && fresh(&paths.thumbnail, source) {
        return Ok(RawRender {
            thumbnail_path: paths.thumbnail.to_string_lossy().into_owned(),
            preview_path: Some(paths.preview.to_string_lossy().into_owned()),
            exif: exif_for_cached(source, &paths.preview),
        });
    }

    let bytes = std::fs::read(source).map_err(GalleryError::Io)?;
    let mut parsed = exif_info::parse_bytes(&bytes);
    let Some(jpeg) = extract_largest_jpeg(&bytes) else {
        log::warn!(
            "No embedded JPEG preview in {}. It will show as a failed thumbnail.",
            source.display()
        );
        let _ = File::create(&paths.missing);
        let _ = std::fs::remove_file(&paths.preview);
        let _ = std::fs::remove_file(&paths.thumbnail);
        return Ok(RawRender {
            thumbnail_path: String::new(),
            preview_path: None,
            exif: parsed.info,
        });
    };

    if !parsed.has_shooting_settings() {
        parsed.fill_gaps_from(&exif_info::parse_bytes(&jpeg.bytes));
    }
    let container_orientation = parsed.orientation_value;

    let (image, preview_bytes) =
        orient_embedded_jpeg(&jpeg.bytes, container_orientation).map_err(GalleryError::Image)?;

    write_file(&paths.preview, &preview_bytes)?;
    let thumbnail = image.thumbnail(512, 512);
    thumbnail
        .save(&paths.thumbnail)
        .map_err(|err| GalleryError::Image(err.to_string()))?;
    let _ = std::fs::remove_file(&paths.missing);

    Ok(RawRender {
        thumbnail_path: paths.thumbnail.to_string_lossy().into_owned(),
        preview_path: Some(paths.preview.to_string_lossy().into_owned()),
        exif: parsed.info,
    })
}

fn exif_for_cached(source: &Path, preview: &Path) -> ExifInfo {
    let mut parsed = exif_info::parse_path(source);
    if !parsed.has_shooting_settings() {
        if let Ok(bytes) = std::fs::read(preview) {
            parsed.fill_gaps_from(&exif_info::parse_bytes(&bytes));
        }
    }
    parsed.info
}

fn fresh(cached: &Path, source: &Path) -> bool {
    let Ok(cached_meta) = cached.metadata() else {
        return false;
    };
    if !cached_meta.is_file() {
        return false;
    }
    let ext = cached
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    if cached_meta.len() == 0 && (ext == "jpg" || ext == "webp") {
        return false;
    }
    match (mtime(cached), mtime(source)) {
        (Some(cached_time), Some(source_time)) => cached_time >= source_time,
        _ => true,
    }
}

fn mtime(path: &Path) -> Option<SystemTime> {
    path.metadata().ok().and_then(|meta| meta.modified().ok())
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = File::create(path).map_err(GalleryError::Io)?;
    file.write_all(bytes).map_err(GalleryError::Io)?;
    Ok(())
}

/// Largest baseline or progressive JPEG embedded in `data`.
/// Lossless JPEG (the sensor bitstream in some RAW files) is ignored.
pub fn extract_largest_jpeg(data: &[u8]) -> Option<EmbeddedJpeg> {
    let mut best: Option<EmbeddedJpeg> = None;
    let mut index = 0;
    while index + 3 < data.len() {
        if data[index] == 0xFF && data[index + 1] == 0xD8 && data[index + 2] == 0xFF {
            if let Some(jpeg) = parse_jpeg(&data[index..]) {
                let area = jpeg.width as u64 * jpeg.height as u64;
                let replace = match &best {
                    None => true,
                    Some(current) => {
                        let current_area = current.width as u64 * current.height as u64;
                        area > current_area
                            || (area == current_area && jpeg.bytes.len() > current.bytes.len())
                    }
                };
                index += jpeg.bytes.len().max(1);
                if replace {
                    best = Some(jpeg);
                }
                continue;
            }
        }
        index += 1;
    }
    best
}

fn parse_jpeg(data: &[u8]) -> Option<EmbeddedJpeg> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }
    let mut pos = 2;
    let mut width = 0u32;
    let mut height = 0u32;
    let mut displayable = false;

    loop {
        if pos >= data.len() || data[pos] != 0xFF {
            return None;
        }
        pos += 1;
        while pos < data.len() && data[pos] == 0xFF {
            pos += 1;
        }
        if pos >= data.len() {
            return None;
        }
        let marker = data[pos];
        pos += 1;

        if marker == 0xD9 {
            break;
        }
        if marker == 0x00 || (0xD0..=0xD7).contains(&marker) {
            continue;
        }
        if pos + 2 > data.len() {
            return None;
        }
        let len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
        if len < 2 || pos + len > data.len() {
            return None;
        }

        match marker {
            0xC0..=0xC2 => {
                if len >= 7 && data[pos + 2] == 8 {
                    height = u16::from_be_bytes([data[pos + 3], data[pos + 4]]) as u32;
                    width = u16::from_be_bytes([data[pos + 5], data[pos + 6]]) as u32;
                    displayable = width > 0 && height > 0;
                }
            }
            // Lossless, differential, and arithmetic JPEG. Not a preview.
            0xC3 | 0xC5 | 0xC6 | 0xC7 | 0xC9 | 0xCA | 0xCB | 0xCD | 0xCE | 0xCF => {
                return None;
            }
            0xDA => {
                pos += len;
                return finish_scan(data, pos, width, height, displayable);
            }
            _ => {}
        }
        pos += len;
    }

    if !displayable {
        return None;
    }
    Some(EmbeddedJpeg {
        bytes: data[..pos].to_vec(),
        width,
        height,
    })
}

fn finish_scan(
    data: &[u8],
    mut pos: usize,
    width: u32,
    height: u32,
    displayable: bool,
) -> Option<EmbeddedJpeg> {
    while pos + 1 < data.len() {
        if data[pos] != 0xFF {
            pos += 1;
            continue;
        }
        let mut marker_pos = pos + 1;
        while marker_pos < data.len() && data[marker_pos] == 0xFF {
            marker_pos += 1;
        }
        if marker_pos >= data.len() {
            return None;
        }
        let marker = data[marker_pos];
        if marker == 0x00 || (0xD0..=0xD7).contains(&marker) {
            pos = marker_pos + 1;
            continue;
        }
        if marker == 0xD9 {
            if !displayable {
                return None;
            }
            let end = marker_pos + 1;
            return Some(EmbeddedJpeg {
                bytes: data[..end].to_vec(),
                width,
                height,
            });
        }
        return None;
    }
    None
}

/// Apply the preview's own orientation. If that JPEG has no orientation tag,
/// use the RAW container's tag. Orientation 1 is left untouched so the stored
/// preview stays the camera's JPEG bytes.
pub fn orient_embedded_jpeg(
    jpeg: &[u8],
    container_orientation: Option<u16>,
) -> std::result::Result<(image::DynamicImage, Vec<u8>), String> {
    let jpeg_orientation = exif_info::orientation_from_bytes(jpeg);
    let value = jpeg_orientation.or(container_orientation).unwrap_or(1);
    let image = image::load_from_memory_with_format(jpeg, image::ImageFormat::Jpeg)
        .map_err(|err| err.to_string())?;
    if value <= 1 {
        return Ok((image, jpeg.to_vec()));
    }
    let Some(orientation) = Orientation::from_exif(value as u8) else {
        return Ok((image, jpeg.to_vec()));
    };
    if orientation == Orientation::NoTransforms {
        return Ok((image, jpeg.to_vec()));
    }
    let mut oriented = image;
    oriented.apply_orientation(orientation);
    let encoded = encode_jpeg(&oriented)?;
    Ok((oriented, encoded))
}

fn encode_jpeg(image: &image::DynamicImage) -> std::result::Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 90);
    image
        .write_with_encoder(encoder)
        .map_err(|err| err.to_string())?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{jpeg_with_shooting_exif, solid_jpeg};
    use image::GenericImageView;
    use tempfile::tempdir;

    #[test]
    fn picks_the_larger_preview_and_ignores_a_file_with_none() {
        let small = solid_jpeg(2, 2, [10, 10, 10]);
        let large = solid_jpeg(8, 4, [20, 30, 40]);
        let mut blob = b"RAWHEADER".to_vec();
        blob.extend_from_slice(&small);
        blob.extend_from_slice(&[0, 1, 2, 3, 0xFF, 0xD9]);
        blob.extend_from_slice(&large);
        let found = extract_largest_jpeg(&blob).unwrap();
        assert_eq!((found.width, found.height), (8, 4));

        assert!(extract_largest_jpeg(b"no jpeg here, just text").is_none());
    }

    #[test]
    fn lossless_jpeg_is_not_a_preview() {
        // SOI, SOF3 (lossless), EOI. Not displayable.
        let lossless = [
            0xFF, 0xD8, 0xFF, 0xC3, 0x00, 0x0B, 0x08, 0x00, 0x02, 0x00, 0x02, 0x01, 0x01, 0x11,
            0x00, 0xFF, 0xD9,
        ];
        assert!(extract_largest_jpeg(&lossless).is_none());
    }

    #[test]
    fn orientation_6_turns_the_preview_upright() {
        let jpeg = jpeg_with_shooting_exif(4, 2, 6);
        let (image, stored) = orient_embedded_jpeg(&jpeg, None).unwrap();
        assert_eq!(image.dimensions(), (2, 4));
        assert_ne!(stored, jpeg);

        let upright = jpeg_with_shooting_exif(4, 2, 1);
        let (image, stored) = orient_embedded_jpeg(&upright, Some(6)).unwrap();
        // The preview names orientation 1, so the container tag is not applied twice.
        assert_eq!(image.dimensions(), (4, 2));
        assert_eq!(stored, upright);
    }

    #[test]
    fn container_orientation_applies_when_the_preview_has_none() {
        let jpeg = solid_jpeg(4, 2, [200, 10, 10]);
        let (image, _) = orient_embedded_jpeg(&jpeg, Some(6)).unwrap();
        assert_eq!(image.dimensions(), (2, 4));
    }

    #[test]
    fn generated_dng_renders_a_thumbnail_and_a_viewer_jpeg() {
        let dir = tempdir().unwrap();
        let jpeg = jpeg_with_shooting_exif(4, 2, 6);
        let mut dng = b"DNG-FIXTURE\0".to_vec();
        dng.extend_from_slice(&[0u8; 24]);
        dng.extend_from_slice(&jpeg);
        let path = dir.path().join("IMG_0001.dng");
        std::fs::write(&path, &dng).unwrap();

        let rendered = render_raw(&path, dir.path().to_str().unwrap()).unwrap();
        let preview_path = rendered.preview_path.as_ref().unwrap();
        assert!(preview_path.ends_with(".preview.jpg"));
        assert!(rendered.thumbnail_path.ends_with(".webp"));
        let preview = image::open(preview_path).unwrap();
        assert_eq!(preview.dimensions(), (2, 4));
        let (thumb_w, thumb_h) = image::open(&rendered.thumbnail_path).unwrap().dimensions();
        assert!(
            thumb_h > thumb_w,
            "oriented thumbnail should be portrait, got {thumb_w}x{thumb_h}"
        );
        assert_eq!(rendered.exif.camera.as_deref(), Some("Canon EOS R5"));
        assert_eq!(rendered.exif.shutter.as_deref(), Some("1/200"));

        let again = render_raw(&path, dir.path().to_str().unwrap()).unwrap();
        assert_eq!(again.preview_path, rendered.preview_path);
    }

    #[test]
    fn missing_preview_does_not_error_and_is_remembered() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.cr3");
        std::fs::write(&path, b"not a camera file").unwrap();
        let rendered = render_raw(&path, dir.path().to_str().unwrap()).unwrap();
        assert!(rendered.preview_path.is_none());
        assert!(rendered.thumbnail_path.is_empty());

        std::fs::write(&path, b"still not a camera file, but rewritten").unwrap();
        // The source is newer than the marker, so this must look again and still not panic.
        let rendered = render_raw(&path, dir.path().to_str().unwrap()).unwrap();
        assert!(rendered.preview_path.is_none());
    }
}

use crate::domain::models::PhotoMetadata;
use std::path::{Path, PathBuf};

pub fn convert_image(file_path: &Path, thumbnail_dir: &str) -> anyhow::Result<PhotoMetadata> {
    let img = image::open(file_path)?;

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

    let file_stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Could not get file stem for {}", file_path.display()))?;

    let output_filename = format!("{}.webp", file_stem);
    let output_path = PathBuf::from(thumbnail_dir).join(&output_filename);

    thumbnail.save(&output_path)?;

    let metadata = super::fs_ops::get_file_metadata(file_path)?;

    Ok(PhotoMetadata {
        metadata,
        path: file_path.to_string_lossy().into_owned(),
        thumbnail_path: output_path.to_string_lossy().into_owned(),
    })
}

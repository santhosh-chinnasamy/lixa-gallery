use anyhow::Result;
use log::{error, info};
use rayon::prelude::*;
use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileMetadata {
    pub name: String,
    pub modified: u64,
    pub created: u64,
    pub size: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhotoMetadata {
    pub metadata: FileMetadata,
    pub thumbnail_path: String,
    pub path: String,
}

pub async fn convert_images(
    path: &str,
    thumbnail_path: String,
) -> Result<Vec<PhotoMetadata>, String> {
    let path = PathBuf::from(path);
    // Handle the result from read_dir instead of unwrapping
    let entries: Vec<_> = match fs::read_dir(path) {
        Ok(dir) => dir.filter_map(|e| e.ok()).collect(),
        Err(e) => return Err(format!("Failed to read directory: {}", e)),
    };

    let results: Vec<PhotoMetadata> = entries
        .par_iter()
        .filter_map(|entry| {
            let file_path = entry.path();
            if file_path.is_dir() {
                return None;
            }

            if let Some(ext) = file_path.extension().and_then(|s| s.to_str()) {
                if matches!(
                    ext.to_lowercase().as_str(),
                    "jpg" | "jpeg" | "png" | "webp" | "bmp" | "gif"
                ) {
                    match convert_image(&file_path, &thumbnail_path) {
                        Ok(photo_metadata) => Some(photo_metadata),
                        Err(e) => {
                            error!("Failed to convert image {}: {}", file_path.display(), e);
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    Ok(results)
}

fn convert_image(file_path: &Path, thumbnail_path: &str) -> anyhow::Result<PhotoMetadata> {
    // Log which file is being processed to easily find the problematic one.
    info!("Attempting to convert: {}", file_path.display());

    let img = image::open(file_path)?; // Use '?' to propagate errors, not unwrap()
    
    // Get original dimensions
    let (width, height) = (img.width(), img.height());
    
    // Calculate thumbnail dimensions while preserving aspect ratio
    let max_size = 512;
    let (thumb_width, thumb_height) = if width > height {
        let ratio = max_size as f32 / width as f32;
        (max_size, (height as f32 * ratio) as u32)
    } else {
        let ratio = max_size as f32 / height as f32;
        ((width as f32 * ratio) as u32, max_size)
    };
    
    // Resize while preserving orientation and aspect ratio
    let thumbnail = img.resize(thumb_width, thumb_height, image::imageops::FilterType::Lanczos3);

    let file_stem = &file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Could not get file stem for {}", file_path.display()))?;

    let output_filename = format!("{}.webp", file_stem);
    let output_path = PathBuf::from(thumbnail_path).join(&output_filename);

    thumbnail.save(&output_path)?;

    let result = get_file_metadata(file_path).map_err(|e| {
        anyhow::anyhow!("Failed to get metadata for {}: {}", file_path.display(), e)
    })?;
    Ok(PhotoMetadata {
        metadata: result,
        path: file_path.to_string_lossy().into_owned(),
        thumbnail_path: output_path.to_string_lossy().into_owned(),
    })
}

fn get_file_metadata<P: AsRef<Path>>(path: P) -> std::io::Result<FileMetadata> {
    let metadata = fs::metadata(&path)?;

    let size = metadata.len();

    let modified = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let created = metadata
        .created()?
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let name = path
        .as_ref()
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Could not get file name for {}", path.as_ref().display()),
            )
        })?
        .to_string();

    Ok(FileMetadata {
        name,
        size,
        modified,
        created,
    })
}

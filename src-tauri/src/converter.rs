use image::imageops::FilterType;
use image::ImageFormat;

use log::{error, info};
use rayon::prelude::*;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn convert_images(path: &str, thumbnail_path: String) -> Result<Vec<String>, String> {
    let path = PathBuf::from(path);
    // Handle the result from read_dir instead of unwrapping
    let entries: Vec<_> = match fs::read_dir(path) {
        Ok(dir) => dir.filter_map(|e| e.ok()).collect(),
        Err(e) => return Err(format!("Failed to read directory: {}", e)),
    };

    let results: Vec<String> = entries
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
                    // Handle the Result from convert_image
                    match convert_image(&file_path, &thumbnail_path) {
                        Ok(output_path) => {
                            info!(
                                "Successfully converted: {} -> {}",
                                file_path.display(),
                                &output_path
                            );
                            return Some(output_path);
                        }
                        Err(e) => {
                            error!("Failed to convert image {}: {}", file_path.display(), e);
                            return None;
                        }
                    }
                }
            }
            None
        })
        .collect();

    Ok(results)
}

fn convert_image(file_path: &Path, thumbnail_path: &str) -> anyhow::Result<String> {
    // Log which file is being processed to easily find the problematic one.
    info!("Attempting to convert: {}", file_path.display());

    let img = image::open(file_path)?; // Use '?' to propagate errors, not unwrap()
    let thumbnail = img.thumbnail(512, 512);

    let file_stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Could not get file stem for {}", file_path.display()))?;

    let output_filename = format!("{}.webp", file_stem);
    let output_path = PathBuf::from(thumbnail_path).join(&output_filename);

    thumbnail.save(&output_path)?;

    Ok(output_path.to_string_lossy().into_owned())
}

/* fn convert_image(file_path: &PathBuf, thumbnail_path: &str) -> String {
    // This function would handle the actual conversion of images to webp format
    let image_path = Path::new(file_path);
    let width = 512;
    let height = 512;

    let img = image::open(image_path).unwrap();
    // if image_path contains "thumbnail", skip processing
    if image_path.to_string_lossy().contains(".thumbnail") {
        return image_path.display().to_string();
    }

    // only resize if the image is larger than the target size
    if img.width() <= width && img.height() <= height {
        fs::copy(image_path, thumbnail_path).unwrap();
        return thumbnail_path.to_string();
    }

    let thumbnail = img.resize(width, height, FilterType::CatmullRom);

    // save as webp format
    let thumbnail_path = PathBuf::from(thumbnail_path)
        .join(file_path.file_stem().unwrap())
        .with_extension("webp");
    let _ = thumbnail.save_with_format(&thumbnail_path, ImageFormat::WebP);
    return thumbnail_path.display().to_string();
}
 */

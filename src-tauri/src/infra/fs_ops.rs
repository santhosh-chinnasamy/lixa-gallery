use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use crate::domain::models::FileMetadata;

pub fn get_file_metadata<P: AsRef<Path>>(path: P) -> std::io::Result<FileMetadata> {
    let metadata = fs::metadata(&path)?;
    let size = metadata.len();
    let modified = metadata.modified()?.duration_since(UNIX_EPOCH).unwrap().as_secs();
    let created = metadata.created()?.duration_since(UNIX_EPOCH).unwrap().as_secs();

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

pub fn list_images_in_dir(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let entries = fs::read_dir(dir)?;
    let images = entries
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                return None;
            }
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if matches!(
                    ext.to_lowercase().as_str(),
                    "jpg" | "jpeg" | "png" | "webp" | "bmp" | "gif"
                ) {
                    return Some(path);
                }
            }
            None
        })
        .collect();
    Ok(images)
}

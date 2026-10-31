use async_trait::async_trait;
use gallery_core::fs::FileSystem;
use gallery_core::models::{FileMetadata, GalleryError, Result};
use std::{
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use tokio::fs;

pub struct LocalFileSystem;

#[async_trait]
impl FileSystem for LocalFileSystem {
    async fn get_file_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let metadata = fs::metadata(path).await?;
        let size = metadata.len();

        let modified = metadata
            .modified()
            .map_err(|e| GalleryError::Io(e))?
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let created = metadata
            .created()
            .map_err(|e| GalleryError::Io(e))?
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                GalleryError::InvalidPath(format!("Could not get file name for {}", path.display()))
            })?
            .to_string();

        Ok(FileMetadata {
            name,
            size,
            modified,
            created,
        })
    }

    async fn list_images_in_dir(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut entries = fs::read_dir(dir).await?;
        let mut images = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                if file_name.starts_with('.') {
                    continue;
                }
            }
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if matches!(
                    ext.to_lowercase().as_str(),
                    "jpg"
                        | "jpeg"
                        | "png"
                        | "webp"
                        | "bmp"
                        | "gif"
                        | "tiff"
                        | "tif"
                        | "avif"
                        | "heic"
                        | "heif"
                        | "svg"
                ) {
                    images.push(path);
                }
            }
        }
        Ok(images)
    }

    async fn list_subfolders(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut entries = fs::read_dir(dir).await?;
        let mut folders = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                if let Some(folder_name) = path.file_name().and_then(|s| s.to_str()) {
                    if folder_name.starts_with('.') {
                        continue;
                    }
                }
                folders.push(path);
            }
        }
        Ok(folders)
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<u64> {
        fs::copy(from, to).await.map_err(|e| GalleryError::Io(e))
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        fs::rename(from, to).await.map_err(|e| GalleryError::Io(e))
    }

    async fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        fs::canonicalize(path)
            .await
            .map_err(|e| GalleryError::Io(e))
    }

    async fn create_dir_all(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path)
            .await
            .map_err(|e| GalleryError::Io(e))
    }
}

pub async fn get_file_metadata<P: AsRef<Path>>(path: P) -> Result<FileMetadata> {
    LocalFileSystem.get_file_metadata(path.as_ref()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_list_images_in_dir() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.jpg");
        File::create(&file_path).unwrap();

        let other_path = dir.path().join("test.txt");
        File::create(&other_path).unwrap();

        // Hidden files and macOS AppleDouble metadata files should be ignored
        let hidden_image = dir.path().join(".hidden.jpg");
        File::create(&hidden_image).unwrap();

        let macos_double = dir.path().join("._IMG_0068.JPG");
        File::create(&macos_double).unwrap();

        let fs = LocalFileSystem;
        let images = fs.list_images_in_dir(dir.path()).await.unwrap();

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].file_name().unwrap(), "test.jpg");
    }

    #[tokio::test]
    async fn test_list_subfolders_ignores_hidden() {
        let dir = tempdir().unwrap();
        let visible_dir = dir.path().join("albums");
        std::fs::create_dir(&visible_dir).unwrap();

        let hidden_dir = dir.path().join(".git");
        std::fs::create_dir(&hidden_dir).unwrap();

        let fs = LocalFileSystem;
        let folders = fs.list_subfolders(dir.path()).await.unwrap();

        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].file_name().unwrap(), "albums");
    }

    #[tokio::test]
    async fn test_get_file_metadata() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("meta.png");
        File::create(&file_path).unwrap();

        let fs = LocalFileSystem;
        let metadata = fs.get_file_metadata(&file_path).await.unwrap();

        assert_eq!(metadata.name, "meta.png");
        assert!(metadata.size == 0);
    }
}

use async_trait::async_trait;
use gallery_core::events::EventHub;
use gallery_core::fs::FileSystem;
use gallery_core::image::ImageProcessor;
use gallery_core::models::{Favourite, FileMetadata, GalleryError, PhotoMetadata, Result};
use gallery_core::repos::{CachedPhotoRecord, FavouriteRepository, PhotoRepository};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct FakeFileSystem {
    pub files: Mutex<Vec<(PathBuf, FileMetadata)>>,
}

#[async_trait]
impl FileSystem for FakeFileSystem {
    async fn get_file_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let files = self.files.lock().unwrap();
        files
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, m)| m.clone())
            .ok_or_else(|| {
                GalleryError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not found",
                ))
            })
    }

    async fn list_images_in_dir(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let files = self.files.lock().unwrap();
        Ok(files
            .iter()
            .filter(|(p, _)| p.parent().map(|parent| parent == dir).unwrap_or(false))
            .filter(|(p, _)| p.extension().is_some())
            .map(|(p, _)| p.clone())
            .collect())
    }

    async fn list_subfolders(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let files = self.files.lock().unwrap();
        Ok(files
            .iter()
            .filter(|(p, _)| p.parent().map(|parent| parent == dir).unwrap_or(false))
            .filter(|(p, _)| p.extension().is_none())
            .map(|(p, _)| p.clone())
            .collect())
    }

    async fn copy(&self, _from: &Path, _to: &Path) -> Result<u64> {
        Ok(0)
    }

    async fn rename(&self, _from: &Path, _to: &Path) -> Result<()> {
        Ok(())
    }

    async fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        Ok(path.to_path_buf())
    }
}

pub struct FakeImageProcessor;
#[async_trait]
impl ImageProcessor for FakeImageProcessor {
    async fn convert_image(&self, path: &Path, _thumb_dir: &str) -> Result<PhotoMetadata> {
        Ok(PhotoMetadata {
            metadata: FileMetadata {
                name: path.file_name().unwrap().to_string_lossy().to_string(),
                modified: 0,
                created: 0,
                size: 0,
            },
            thumbnail_path: "thumb".to_string(),
            path: path.to_string_lossy().to_string(),
        })
    }
}

pub struct FakePhotoRepository {
    pub photos: Mutex<Vec<PhotoMetadata>>,
}

#[async_trait]
impl PhotoRepository for FakePhotoRepository {
    async fn get_cached_photos_for_path(&self, _prefix: &str) -> Result<Vec<CachedPhotoRecord>> {
        let photos = self.photos.lock().unwrap();
        Ok(photos
            .iter()
            .map(|p| CachedPhotoRecord {
                path: p.path.clone(),
                thumbnail_path: p.thumbnail_path.clone(),
                mtime: p.metadata.modified as i64,
                size: p.metadata.size as i64,
            })
            .collect())
    }

    async fn batch_insert_photos(&self, photos: &[PhotoMetadata]) -> Result<()> {
        let mut existing = self.photos.lock().unwrap();
        existing.extend(photos.to_vec());
        Ok(())
    }
}

pub struct FakeEventHub;
impl EventHub for FakeEventHub {
    fn emit_progress(&self, _event: &str, _progress: u32) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct FakeFavouriteRepository {
    pub favourites: Mutex<Vec<Favourite>>,
}

#[async_trait]
impl FavouriteRepository for FakeFavouriteRepository {
    async fn add_favourite(&self, path: String) -> Result<()> {
        self.favourites.lock().unwrap().push(Favourite { path });
        Ok(())
    }
    async fn get_favourites(&self) -> Result<Vec<Favourite>> {
        Ok(self.favourites.lock().unwrap().clone())
    }
    async fn remove_favourite(&self, path: String) -> Result<()> {
        self.favourites.lock().unwrap().retain(|f| f.path != path);
        Ok(())
    }
    async fn clear_favourites(&self) -> Result<()> {
        self.favourites.lock().unwrap().clear();
        Ok(())
    }
}

use futures::future::BoxFuture;
use gallery_core::events::EventHub;
use gallery_core::fs::FileSystem;
use gallery_core::image::ImageProcessor;
use gallery_core::models::{Favourite, FileMetadata, PhotoMetadata};
use gallery_core::repos::{FavouriteRepository, PhotoRepository};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct FakeFileSystem {
    pub files: Mutex<Vec<(PathBuf, FileMetadata)>>,
}

impl FileSystem for FakeFileSystem {
    fn get_file_metadata(&self, path: &Path) -> std::io::Result<FileMetadata> {
        let files = self.files.lock().unwrap();
        files
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, m)| m.clone())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "not found"))
    }

    fn list_images_in_dir(&self, dir: &Path) -> std::io::Result<Vec<PathBuf>> {
        let files = self.files.lock().unwrap();
        Ok(files
            .iter()
            .filter(|(p, _)| p.parent().map(|parent| parent == dir).unwrap_or(false))
            .map(|(p, _)| p.clone())
            .collect())
    }

    fn copy(&self, _from: &Path, _to: &Path) -> std::io::Result<u64> {
        Ok(0)
    }
    fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf> {
        Ok(path.to_path_buf())
    }
}

pub struct FakeImageProcessor;
impl ImageProcessor for FakeImageProcessor {
    fn convert_image(&self, path: &Path, _thumb_dir: &str) -> anyhow::Result<PhotoMetadata> {
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

impl PhotoRepository for FakePhotoRepository {
    fn get_cached_photos_for_path<'a>(
        &'a self,
        _prefix: &'a str,
    ) -> BoxFuture<'a, anyhow::Result<Vec<(String, String, i64, i64)>>> {
        Box::pin(async move {
            let photos = self.photos.lock().unwrap();
            Ok(photos
                .iter()
                .map(|p| {
                    (
                        p.path.clone(),
                        p.thumbnail_path.clone(),
                        p.metadata.modified as i64,
                        p.metadata.size as i64,
                    )
                })
                .collect())
        })
    }

    fn batch_insert_photos<'a>(
        &'a self,
        photos: &'a [PhotoMetadata],
    ) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            let mut existing = self.photos.lock().unwrap();
            existing.extend(photos.to_vec());
            Ok(())
        })
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

impl FavouriteRepository for FakeFavouriteRepository {
    fn add_favourite(&self, path: String) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async move {
            self.favourites.lock().unwrap().push(Favourite { path });
            Ok(())
        })
    }
    fn get_favourites(&self) -> BoxFuture<'_, anyhow::Result<Vec<Favourite>>> {
        Box::pin(async move { Ok(self.favourites.lock().unwrap().clone()) })
    }
    fn remove_favourite(&self, path: String) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async move {
            self.favourites.lock().unwrap().retain(|f| f.path != path);
            Ok(())
        })
    }
    fn clear_favourites(&self) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async move {
            self.favourites.lock().unwrap().clear();
            Ok(())
        })
    }
}

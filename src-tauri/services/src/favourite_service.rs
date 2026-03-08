use gallery_core::events::EventHub;
use gallery_core::fs::FileSystem;
use gallery_core::models::{Favourite, GalleryError, Result};
use gallery_core::repos::FavouriteRepository;
use std::{path::PathBuf, sync::Arc};

pub struct FavouriteService {
    favourite_repo: Arc<dyn FavouriteRepository>,
    fs: Arc<dyn FileSystem>,
}

impl FavouriteService {
    pub fn new(favourite_repo: Arc<dyn FavouriteRepository>, fs: Arc<dyn FileSystem>) -> Self {
        Self { favourite_repo, fs }
    }

    pub async fn export_favourites(
        &self,
        events: &dyn EventHub,
        destination: &str,
        mode: &str,
    ) -> Result<()> {
        let favourites = self.favourite_repo.get_favourites().await?;

        let mut counter = 0;
        for fav in favourites {
            let file_path = fav.path.clone();
            let file = PathBuf::from(&file_path);
            let name = file
                .file_name()
                .ok_or_else(|| GalleryError::InvalidPath(file_path.clone()))?;

            let destination_path = PathBuf::from(destination).join(name);

            let canonical_src = self.fs.canonicalize(&file).await?;
            let canonical_dst = self
                .fs
                .canonicalize(&destination_path)
                .await
                .unwrap_or(destination_path.clone());

            if canonical_src == canonical_dst {
                continue;
            }

            if mode == "move" {
                self.fs.rename(&file, &destination_path).await?;
                // Remove from DB since the file was physically moved
                self.remove_favourite(fav.path).await?;
            } else {
                self.fs.copy(&file, &destination_path).await?;
            }

            counter += 1;
            events
                .emit_progress("export-progress", counter)
                .map_err(|e| GalleryError::Unknown(e.to_string()))?;
        }

        Ok(())
    }

    pub async fn add_favourite(&self, path: String) -> Result<()> {
        self.favourite_repo.add_favourite(path).await
    }

    pub async fn get_favourites(&self) -> Result<Vec<Favourite>> {
        self.favourite_repo.get_favourites().await
    }

    pub async fn remove_favourite(&self, path: String) -> Result<()> {
        self.favourite_repo.remove_favourite(path).await
    }

    pub async fn clear_favourites(&self) -> Result<()> {
        self.favourite_repo.clear_favourites().await
    }
}

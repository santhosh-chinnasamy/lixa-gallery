use gallery_core::events::EventHub;
use gallery_core::fs::FileSystem;
use gallery_core::models::Favourite;
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
    ) -> anyhow::Result<()> {
        let favourites = self.favourite_repo.get_favourites().await?;
        let files = favourites.into_iter().map(|f| f.path).collect::<Vec<_>>();

        let mut counter = 0;
        for file_path in files {
            let file = PathBuf::from(&file_path);
            let name = file
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid path: {}", file_path))?;

            let destination_path = PathBuf::from(destination).join(name);

            let canonical_src = self.fs.canonicalize(&file)?;
            let canonical_dst = self
                .fs
                .canonicalize(&destination_path)
                .unwrap_or(destination_path.clone());

            if canonical_src == canonical_dst {
                continue;
            }

            self.fs.copy(&file, &destination_path)?;

            counter += 1;
            events.emit_progress("export-progress", counter)?;
        }

        Ok(())
    }

    pub async fn add_favourite(&self, path: String) -> anyhow::Result<()> {
        self.favourite_repo.add_favourite(path).await
    }

    pub async fn get_favourites(&self) -> anyhow::Result<Vec<Favourite>> {
        self.favourite_repo.get_favourites().await
    }

    pub async fn remove_favourite(&self, path: String) -> anyhow::Result<()> {
        self.favourite_repo.remove_favourite(path).await
    }

    pub async fn clear_favourites(&self) -> anyhow::Result<()> {
        self.favourite_repo.clear_favourites().await
    }
}

use crate::domain::models::Favourite;
use crate::domain::repos::FavouriteRepository;
use std::{fs, path::PathBuf, sync::Arc};
use tauri::{AppHandle, Emitter};

pub struct FavouriteService {
    favourite_repo: Arc<dyn FavouriteRepository>,
}

impl FavouriteService {
    pub fn new(favourite_repo: Arc<dyn FavouriteRepository>) -> Self {
        Self { favourite_repo }
    }

    pub async fn export_favourites(&self, app: AppHandle, destination: &str) -> anyhow::Result<()> {
        let favourites = self.favourite_repo.get_favourites().await?;
        let files = favourites.into_iter().map(|f| f.path).collect::<Vec<_>>();

        let mut counter = 0;
        for file_path in files {
            let file = PathBuf::from(&file_path);
            let name = file
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid path: {}", file_path))?;

            let destination_path = PathBuf::from(destination).join(name);

            let canonical_src = fs::canonicalize(&file_path)?;
            let canonical_dst =
                fs::canonicalize(&destination_path).unwrap_or(destination_path.clone());

            if canonical_src == canonical_dst {
                continue;
            }

            fs::copy(&file_path, &destination_path)?;

            counter += 1;
            app.emit("export-progress", counter)?;
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

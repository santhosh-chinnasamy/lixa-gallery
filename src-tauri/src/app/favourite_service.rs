use crate::domain::models::Favourite;
use crate::infra::db;
use sqlx::SqlitePool;
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Emitter};

pub struct FavouriteService {
    pool: SqlitePool,
}

impl FavouriteService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn export_favourites(&self, app: AppHandle, destination: &str) -> anyhow::Result<()> {
        let favourites = db::get_favourites(&self.pool).await?;
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
        db::add_favourite(&self.pool, path).await
    }

    pub async fn get_favourites(&self) -> anyhow::Result<Vec<Favourite>> {
        db::get_favourites(&self.pool).await
    }

    pub async fn remove_favourite(&self, path: String) -> anyhow::Result<()> {
        db::remove_favourite(&self.pool, path).await
    }

    pub async fn clear_favourites(&self) -> anyhow::Result<()> {
        db::clear_favourites(&self.pool).await
    }
}

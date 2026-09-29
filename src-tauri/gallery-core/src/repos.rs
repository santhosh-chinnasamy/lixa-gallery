use crate::models::{Favourite, PhotoMetadata, Result, Workspace};
use async_trait::async_trait;

pub struct CachedPhotoRecord {
    pub path: String,
    pub thumbnail_path: String,
    pub mtime: i64,
    pub size: i64,
}

#[async_trait]
pub trait PhotoRepository: Send + Sync {
    async fn get_cached_photos_for_path(&self, prefix: &str) -> Result<Vec<CachedPhotoRecord>>;
    async fn get_photos_by_paths(&self, paths: &[String]) -> Result<Vec<PhotoMetadata>>;
    async fn batch_insert_photos(&self, photos: &[PhotoMetadata]) -> Result<()>;
}

#[async_trait]
pub trait FavouriteRepository: Send + Sync {
    async fn add_favourite(&self, path: String) -> Result<()>;
    async fn get_favourites(&self) -> Result<Vec<Favourite>>;
    async fn get_favourites_by_prefix(&self, prefix: &str) -> Result<Vec<Favourite>>;
    async fn remove_favourite(&self, path: String) -> Result<()>;
    async fn clear_favourites(&self) -> Result<()>;
    async fn clear_favourites_by_prefix(&self, prefix: &str) -> Result<()>;
}

#[async_trait]
pub trait WorkspaceRepository: Send + Sync {
    async fn upsert_workspace(&self, workspace: &Workspace) -> Result<()>;
    async fn get_workspaces(&self) -> Result<Vec<Workspace>>;
    async fn get_workspace_by_path(&self, root_path: &str) -> Result<Option<Workspace>>;
    async fn remove_workspace(&self, id: &str) -> Result<()>;
}

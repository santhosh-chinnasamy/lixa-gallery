use crate::models::{Favourite, PhotoMetadata, Result};
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
    async fn batch_insert_photos(&self, photos: &[PhotoMetadata]) -> Result<()>;
}

#[async_trait]
pub trait FavouriteRepository: Send + Sync {
    async fn add_favourite(&self, path: String) -> Result<()>;
    async fn get_favourites(&self) -> Result<Vec<Favourite>>;
    async fn remove_favourite(&self, path: String) -> Result<()>;
    async fn clear_favourites(&self) -> Result<()>;
}

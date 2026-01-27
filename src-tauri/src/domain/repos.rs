use crate::domain::models::{Favourite, PhotoMetadata};
use futures::future::BoxFuture;

pub trait PhotoRepository: Send + Sync {
    fn get_cached_photos_for_path<'a>(
        &'a self,
        prefix: &'a str,
    ) -> BoxFuture<'a, anyhow::Result<Vec<(String, String, i64, i64)>>>;
    fn batch_insert_photos<'a>(
        &'a self,
        photos: &'a [PhotoMetadata],
    ) -> BoxFuture<'a, anyhow::Result<()>>;
}

pub trait FavouriteRepository: Send + Sync {
    fn add_favourite(&self, path: String) -> BoxFuture<'_, anyhow::Result<()>>;
    fn get_favourites(&self) -> BoxFuture<'_, anyhow::Result<Vec<Favourite>>>;
    fn remove_favourite(&self, path: String) -> BoxFuture<'_, anyhow::Result<()>>;
    fn clear_favourites(&self) -> BoxFuture<'_, anyhow::Result<()>>;
}

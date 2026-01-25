use crate::app::{favourite_service::FavouriteService, gallery_service::GalleryService};

pub struct AppState {
    pub gallery: GalleryService,
    pub favourite: FavouriteService,
    pub thumbnail_path: String,
}

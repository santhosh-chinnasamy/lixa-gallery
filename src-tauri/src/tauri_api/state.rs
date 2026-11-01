use services::{favourite_service::FavouriteService, gallery_service::GalleryService};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub struct AppState {
    pub gallery: GalleryService,
    pub favourite: FavouriteService,
    pub thumbnail_path: String,
    /// Set by `cancel_export` and read between files. `begin_export` clears it
    /// for the next run.
    pub export_cancel: Arc<AtomicBool>,
}

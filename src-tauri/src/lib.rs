use tauri::Manager;

pub mod app;
pub mod domain;
pub mod infra;
pub mod tauri_api;

use crate::infra::db;
use crate::tauri_api::{commands, state::AppState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::scan_folder,
            commands::export_favourites,
            commands::add_favourite,
            commands::remove_favourite,
            commands::get_favourites,
            commands::clear_favourites
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            let app_data_dir = app_handle
                .path()
                .app_data_dir()
                .expect("failed to get data_dir");
            let pkg_name = env!("CARGO_PKG_NAME");

            let mut thumbnail_path = app_data_dir.clone();
            thumbnail_path.push(".thumbnails");
            if !thumbnail_path.exists() {
                std::fs::create_dir_all(&thumbnail_path)
                    .expect("failed to create thumbnail directory");
            }

            tauri::async_runtime::block_on(async move {
                let pool = db::setup_db(app_data_dir, pkg_name).await;

                let photo_repo = std::sync::Arc::new(db::SqlitePhotoRepository::new(pool.clone()));
                let favourite_repo = std::sync::Arc::new(db::SqliteFavouriteRepository::new(pool));
                let image_processor = std::sync::Arc::new(infra::image::ImageProcessor);

                let gallery =
                    app::gallery_service::GalleryService::new(photo_repo, image_processor);
                let favourite = app::favourite_service::FavouriteService::new(favourite_repo);

                app_handle.manage(AppState {
                    gallery,
                    favourite,
                    thumbnail_path: thumbnail_path.to_string_lossy().to_string(),
                });
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

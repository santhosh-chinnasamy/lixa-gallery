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

            tauri::async_runtime::block_on(async move {
                let pool = db::setup_db(app_data_dir, pkg_name).await;

                let gallery = app::gallery_service::GalleryService::new(pool.clone());
                let favourite = app::favourite_service::FavouriteService::new(pool);

                app_handle.manage(AppState { gallery, favourite });
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

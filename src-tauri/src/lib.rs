use tauri::Manager;

pub mod tauri_api;

use crate::tauri_api::{commands, state::AppState};
use infra::db;
use percent_encoding::percent_decode_str;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol("lixa-thumbnail", move |ctx, request, responder| {
            let app_handle = ctx.app_handle().clone();
            tauri::async_runtime::spawn(async move {
                let uri = request.uri().to_string();
                let prefix = "lixa-thumbnail://localhost/";
                if !uri.starts_with(prefix) {
                    responder.respond(
                        tauri::http::Response::builder()
                            .status(400)
                            .body(Vec::new())
                            .unwrap(),
                    );
                    return;
                }

                let encoded_path = &uri[prefix.len()..];
                let path_str = percent_decode_str(encoded_path).decode_utf8_lossy().into_owned();
                let original_path = std::path::PathBuf::from(path_str);

                let state = app_handle.state::<AppState>();
                let thumb_dir = &state.thumbnail_path;

                // Simple check if it already exists to avoid redundant generation
                // The thumbnail filename logic must match infra::image
                let file_stem = original_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
                let thumb_full_path = std::path::Path::new(thumb_dir).join(format!("{}.webp", file_stem));

                if thumb_full_path.exists() {
                    if let Ok(data) = std::fs::read(&thumb_full_path) {
                        responder.respond(
                            tauri::http::Response::builder()
                                .status(200)
                                .header("Content-Type", "image/webp")
                                .body(data)
                                .unwrap(),
                        );
                        return;
                    }
                }

                // Not found or failed to read, generate on the fly
                match state.gallery.get_or_create_thumbnail(&original_path, thumb_dir).await {
                    Ok(generated_path) => {
                        if let Ok(data) = std::fs::read(&generated_path) {
                            responder.respond(
                                tauri::http::Response::builder()
                                    .status(200)
                                    .header("Content-Type", "image/webp")
                                    .body(data)
                                    .unwrap(),
                            );
                        } else {
                            responder.respond(
                                tauri::http::Response::builder()
                                    .status(500)
                                    .body(Vec::new())
                                    .unwrap(),
                            );
                        }
                    }
                    Err(_) => {
                        responder.respond(
                            tauri::http::Response::builder()
                                .status(404)
                                .body(Vec::new())
                                .unwrap(),
                        );
                    }
                }
            });
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan_folder,
            commands::export_favourites,
            commands::add_favourite,
            commands::remove_favourite,
            commands::get_favourites,
            commands::clear_favourites,
            commands::get_folder_tree
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

                let fs = std::sync::Arc::new(infra::fs_ops::LocalFileSystem);
                let photo_repo = std::sync::Arc::new(db::SqlitePhotoRepository::new(pool.clone()));
                let favourite_repo = std::sync::Arc::new(db::SqliteFavouriteRepository::new(pool));
                let image_processor = std::sync::Arc::new(infra::image::ImageProcessor);
                let benchmark_logger = std::sync::Arc::new(infra::benchmark::JsonlBenchmarkLogger::new(app_data_dir));

                let gallery = services::gallery_service::GalleryService::new(
                    photo_repo,
                    image_processor,
                    fs.clone(),
                    benchmark_logger,
                    tokio::runtime::Handle::current(),
                );
                let favourite =
                    services::favourite_service::FavouriteService::new(favourite_repo, fs);

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

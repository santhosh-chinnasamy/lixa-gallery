use crate::tauri_api::{commands, state::AppState};
use infra::db;
use percent_encoding::percent_decode_str;
use tauri::Manager;

pub mod tauri_api;

fn webp_response(status: u16, body: Vec<u8>) -> tauri::http::Response<Vec<u8>> {
    let mut response = tauri::http::Response::builder().status(status);
    if status == 200 {
        response = response
            .header("Content-Type", "image/webp")
            .header("Cache-Control", "no-cache");
    }
    response.body(body).expect("webp response")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload".to_string()
        };

        let location = panic_info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "Unknown location".to_string());

        let backtrace = std::backtrace::Backtrace::force_capture();
        log::error!(
            target: "crash",
            "APPLICATION PANIC / CRASH at {location}:\nPayload: {payload}\nBacktrace:\n{backtrace}"
        );
        default_hook(panic_info);
    }));

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: None,
                    }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                ])
                .max_file_size(5 * 1024 * 1024)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(5))
                .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
                .level(log::LevelFilter::Info)
                .filter(|metadata| {
                    if metadata.target().starts_with("sqlx::query")
                        && metadata.level() > log::Level::Info
                    {
                        return false;
                    }
                    true
                })
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol(
            "lixa-thumbnail",
            move |ctx, request, responder| {
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

                    let request =
                        infra::thumbnail_cache::parse_thumbnail_request(&uri[prefix.len()..]);
                    let path_str = percent_decode_str(request.path)
                        .decode_utf8_lossy()
                        .into_owned();
                    let original_path = std::path::PathBuf::from(&path_str);

                    let state = app_handle.state::<AppState>();
                    let thumb_dir = state.thumbnail_path.clone();

                    // TIFF on Windows and Linux cannot be painted by the webview.
                    // The viewer asks for a decoded WebP instead of the original.
                    if request.preview {
                        match infra::image::ensure_decoded_preview(
                            &original_path,
                            std::path::Path::new(&thumb_dir),
                        )
                        .await
                        {
                            Ok(preview_path) => match std::fs::read(&preview_path) {
                                Ok(data) => responder.respond(webp_response(200, data)),
                                Err(_) => responder.respond(webp_response(500, Vec::new())),
                            },
                            Err(err) => {
                                log::warn!(
                                    "Could not build a preview for {}: {err}",
                                    original_path.display()
                                );
                                responder.respond(webp_response(404, Vec::new()));
                            }
                        }
                        return;
                    }

                    // Same path as infra::image. mtime and size are part of the
                    // key, so an edited file does not reuse the previous bytes.
                    let Ok(identity) =
                        infra::thumbnail_cache::ThumbnailIdentity::read(&original_path)
                    else {
                        responder.respond(webp_response(404, Vec::new()));
                        return;
                    };
                    let thumb_full_path = infra::thumbnail_cache::thumbnail_disk_path(
                        std::path::Path::new(&thumb_dir),
                        &original_path,
                        identity,
                    );

                    if thumb_full_path.is_file() {
                        if let Ok(data) = std::fs::read(&thumb_full_path) {
                            responder.respond(webp_response(200, data));
                            return;
                        }
                    }

                    // Not found or failed to read, generate on the fly
                    match state
                        .gallery
                        .get_or_create_thumbnail(&original_path, &thumb_dir)
                        .await
                    {
                        Ok(generated_path) => {
                            if let Ok(data) = std::fs::read(&generated_path) {
                                responder.respond(webp_response(200, data));
                            } else {
                                responder.respond(webp_response(500, Vec::new()));
                            }
                        }
                        Err(_) => {
                            responder.respond(webp_response(404, Vec::new()));
                        }
                    }
                });
            },
        )
        .invoke_handler(tauri::generate_handler![
            commands::scan_folder,
            commands::export_favourites,
            commands::add_favourite,
            commands::remove_favourite,
            commands::get_favourites,
            commands::get_favourite_photos,
            commands::get_favourite_folder_groups,
            commands::clear_favourites,
            commands::clear_favourites_by_prefix,
            commands::save_workspace,
            commands::get_workspaces,
            commands::get_folder_tree,
            commands::open_logs_dir,
            commands::get_log_dir
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
            match infra::thumbnail_cache::purge_obsolete_thumbnails(&thumbnail_path) {
                Ok(stats) => {
                    if stats.removed_entries > 0 {
                        log::info!(
                            "Removed {} obsolete thumbnail cache entries ({} bytes)",
                            stats.removed_entries,
                            stats.removed_bytes
                        );
                    }
                    if stats.errors > 0 {
                        log::warn!(
                            "Thumbnail cache migration left {} entries in place",
                            stats.errors
                        );
                    }
                }
                Err(err) => {
                    log::warn!(
                        "Could not purge obsolete thumbnails in {}: {err}",
                        thumbnail_path.display()
                    );
                }
            }

            tauri::async_runtime::block_on(async move {
                let pool = db::setup_db(app_data_dir.clone(), pkg_name).await;

                let fs = std::sync::Arc::new(infra::fs_ops::LocalFileSystem);
                let photo_repo = std::sync::Arc::new(db::SqlitePhotoRepository::new(pool.clone()));
                let favourite_repo =
                    std::sync::Arc::new(db::SqliteFavouriteRepository::new(pool.clone()));
                let workspace_repo = std::sync::Arc::new(db::SqliteWorkspaceRepository::new(pool));
                let image_processor = std::sync::Arc::new(infra::image::RayonImageProcessor::new());
                let benchmark_logger =
                    std::sync::Arc::new(infra::benchmark::JsonlBenchmarkLogger::new(app_data_dir));

                let gallery = services::gallery_service::GalleryService::new(
                    photo_repo.clone(),
                    image_processor,
                    fs.clone(),
                    benchmark_logger,
                    tokio::runtime::Handle::current(),
                );
                let favourite = services::favourite_service::FavouriteService::new(
                    favourite_repo,
                    photo_repo,
                    workspace_repo,
                    fs,
                );

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

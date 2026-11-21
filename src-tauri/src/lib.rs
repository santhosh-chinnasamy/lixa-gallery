use crate::tauri_api::{commands, state::AppState};
use infra::db;
use percent_encoding::percent_decode_str;
use tauri::Manager;

pub mod logging;
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

/// Tail after the custom-protocol origin. Linux/macOS use `scheme://localhost/`;
/// Windows/Android use `http(s)://scheme.localhost/`.
fn thumbnail_uri_tail(uri: &str) -> Option<&str> {
    const PREFIXES: &[&str] = &[
        "lixa-thumbnail://localhost/",
        "http://lixa-thumbnail.localhost/",
        "https://lixa-thumbnail.localhost/",
    ];
    for prefix in PREFIXES {
        if let Some(rest) = uri.strip_prefix(prefix) {
            return Some(rest);
        }
    }
    None
}

/// Percent-decode the protocol path and restore a leading `/` on Unix when
/// WebKit already decoded `%2F` and dropped it from the URI path form.
fn decode_source_path(encoded: &str) -> String {
    let mut path = percent_decode_str(encoded).decode_utf8_lossy().into_owned();
    #[cfg(unix)]
    {
        if !path.is_empty() && !path.starts_with('/') {
            path.insert(0, '/');
        }
    }
    path
}

fn is_webp_bytes(data: &[u8]) -> bool {
    data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP"
}

/// Read `path` only when the bytes are a WebP. Avoids labeling JPEG fallbacks
/// as `image/webp`.
fn read_webp_file(path: &std::path::Path) -> Option<Vec<u8>> {
    let data = std::fs::read(path).ok()?;
    is_webp_bytes(&data).then_some(data)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize in-memory Flight Recorder logger (capped at 2 MB RAM)
    if let Err(e) = logging::init_flight_recorder() {
        eprintln!("Warning: Failed to initialize flight recorder logger: {e}");
    }

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
        let crash_msg = format!(
            "\n[CRASH] APPLICATION PANIC at {location}:\nPayload: {payload}\nBacktrace:\n{backtrace}\n"
        );
        eprintln!("{crash_msg}");
        logging::dump_crash(&crash_msg);
        default_hook(panic_info);
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().skip_logger().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .register_asynchronous_uri_scheme_protocol(
            "lixa-thumbnail",
            move |ctx, request, responder| {
                let app_handle = ctx.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    let uri = request.uri().to_string();
                    let Some(tail) = thumbnail_uri_tail(&uri) else {
                        responder.respond(
                            tauri::http::Response::builder()
                                .status(400)
                                .body(Vec::new())
                                .unwrap(),
                        );
                        return;
                    };

                    let request = infra::thumbnail_cache::parse_thumbnail_request(tail);
                    let original_path =
                        std::path::PathBuf::from(decode_source_path(request.path));

                    let state = app_handle.state::<AppState>();
                    let thumb_dir = state.thumbnail_path.clone();

                    // TIFF on Windows and Linux cannot be painted by the webview.
                    // The viewer asks for a decoded WebP instead of the original.
                    // Route through the bounded scheduler so folder switches cannot
                    // oversubscribe decode past max_workers.
                    if request.preview {
                        match state
                            .gallery
                            .get_or_create_preview(&original_path, &thumb_dir)
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

                    if let Some(data) = read_webp_file(&thumb_full_path) {
                        responder.respond(webp_response(200, data));
                        return;
                    }

                    // Not found or failed to read, generate on the fly
                    match state
                        .gallery
                        .get_or_create_thumbnail(&original_path, &thumb_dir)
                        .await
                    {
                        Ok(generated_path) => {
                            match read_webp_file(std::path::Path::new(&generated_path)) {
                                Some(data) => responder.respond(webp_response(200, data)),
                                // Never mislabel a JPEG/original as image/webp.
                                None => responder.respond(webp_response(404, Vec::new())),
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
            commands::ensure_raw_preview,
            commands::export_favourites,
            commands::begin_export,
            commands::cancel_export,
            commands::add_favourite,
            commands::remove_favourite,
            commands::get_favourites,
            commands::get_favourite_photos,
            commands::get_favourite_folder_groups,
            commands::clear_favourites,
            commands::clear_favourites_by_prefix,
            commands::set_cull_flag,
            commands::get_cull_flags,
            commands::save_workspace,
            commands::get_workspaces,
            commands::get_folder_tree,
            commands::open_logs_dir,
            commands::get_log_dir,
            commands::ensure_thumbnail_path,
            commands::get_thumbnail_bytes,
            commands::read_image_bytes
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            if let Ok(log_dir) = app_handle.path().app_log_dir() {
                logging::set_log_dir(log_dir);
            }
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
                let marks = std::sync::Arc::new(db::SqliteFavouriteRepository::new(pool.clone()));
                let workspace_repo = std::sync::Arc::new(db::SqliteWorkspaceRepository::new(pool));
                let image_processor = std::sync::Arc::new(infra::image::RayonImageProcessor::new());
                let benchmark_logger =
                    std::sync::Arc::new(infra::benchmark::JsonlBenchmarkLogger::new(app_data_dir));
                let events: std::sync::Arc<dyn gallery_core::events::EventHub> =
                    std::sync::Arc::new(crate::tauri_api::events::TauriEventHub::new(
                        app_handle.clone(),
                    ));

                let gallery = services::gallery_service::GalleryService::new(
                    photo_repo.clone(),
                    image_processor,
                    fs.clone(),
                    benchmark_logger,
                    tokio::runtime::Handle::current(),
                    events,
                );
                let favourite = services::favourite_service::FavouriteService::new(
                    marks.clone(),
                    marks,
                    photo_repo,
                    workspace_repo,
                    fs,
                );

                app_handle.manage(AppState {
                    gallery,
                    favourite,
                    thumbnail_path: thumbnail_path.to_string_lossy().to_string(),
                    export_cancel: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                });
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod protocol_tests {
    use super::{decode_source_path, is_webp_bytes, thumbnail_uri_tail};

    #[test]
    fn uri_tail_strips_custom_and_http_origins() {
        assert_eq!(
            thumbnail_uri_tail(
                "lixa-thumbnail://localhost/thumb?path=%2Fhome%2Fa.jpg&v=1_2"
            ),
            Some("thumb?path=%2Fhome%2Fa.jpg&v=1_2")
        );
        assert_eq!(
            thumbnail_uri_tail(
                "http://lixa-thumbnail.localhost/thumb?path=%2Ftmp%2Fb.jpg&v=1_2"
            ),
            Some("thumb?path=%2Ftmp%2Fb.jpg&v=1_2")
        );
        assert_eq!(thumbnail_uri_tail("https://evil.example/x"), None);
    }

    #[test]
    fn decode_restores_unix_root_for_legacy_webkit_tail() {
        let decoded = decode_source_path("home/santhosh/Pictures/a.JPG");
        assert_eq!(decoded, "/home/santhosh/Pictures/a.JPG");
    }

    #[test]
    fn decode_path_query_value_keeps_absolute_path() {
        let decoded = decode_source_path(
            "%2Fhome%2Fsanthosh%2FPictures%2Fmarriage%2F100MSDCF%2FDSC04630.JPG",
        );
        assert_eq!(
            decoded,
            "/home/santhosh/Pictures/marriage/100MSDCF/DSC04630.JPG"
        );
    }

    #[test]
    fn webp_magic_rejects_jpeg() {
        assert!(!is_webp_bytes(&[0xff, 0xd8, 0xff, 0xe0]));
        let mut webp = b"RIFF....WEBP".to_vec();
        webp[4..8].copy_from_slice(&[0, 0, 0, 0]);
        assert!(is_webp_bytes(&webp));
    }
}

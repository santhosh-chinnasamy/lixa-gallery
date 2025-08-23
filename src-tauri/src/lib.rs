use futures::TryStreamExt;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
    SqlitePool,
};
use std::{
    fs::{self},
    path::PathBuf,
    str::FromStr,
};
use tauri::{App, AppHandle, Emitter, Manager as _};

mod converter;
mod state;
use crate::{converter::PhotoMetadata, state::Favourite};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
async fn scan_folder(
    app: AppHandle,
    db: tauri::State<'_, state::AppState>,
    path: &str,
) -> Result<Vec<PhotoMetadata>, String> {
    let mut thumbnail_path = app.path().app_data_dir().expect("failed to get data_dir");
    thumbnail_path.push(".thumbnails");
    if !thumbnail_path.exists() {
        fs::create_dir_all(&thumbnail_path)
            .map_err(|e| format!("Failed to create thumbnail directory: {}", e))?;
    }

    let folder_path = PathBuf::from(path);
    let result: Vec<PhotoMetadata> = converter::convert_images(
        &folder_path.display().to_string(),
        thumbnail_path.display().to_string(),
        db,
    )
    .await?;
    return Ok(result);
}

#[tauri::command]
async fn export_favourites(
    app: AppHandle,
    db: tauri::State<'_, state::AppState>,
    destination: &str,
) -> Result<(), String> {
    let favourites = get_favourites(db).await?;
    let files = favourites
        .into_iter()
        .map(|favorite| favorite.path)
        .collect::<Vec<String>>();

    let mut counter = 0;

    for file_path in files {
        let file = PathBuf::from(&file_path);
        let name = file
            .file_name()
            .ok_or_else(|| format!("Invalid path (no file name): {}", file_path))?;
        let name_str = name
            .to_str()
            .ok_or_else(|| format!("Non-UTF8 file name: {}", file_path))?;

        let destination_path = PathBuf::from(destination).join(name_str);

        let canonical_src = fs::canonicalize(&file_path)
            .map_err(|e| format!("Failed to canonicalize source path: {}", e))?;
        let canonical_dst = fs::canonicalize(&destination_path).unwrap_or(destination_path.clone());

        if canonical_src == canonical_dst {
            log::info!(
                "Skipping copy of {} as it already exists in the same folder",
                file_path
            );
            continue;
        }

        fs::copy(&file_path, &destination_path)
            .map_err(|e| format!("Error copying file: {}", e))?;

        counter += 1;
        app.emit("export-progress", counter)
            .map_err(|e| format!("Failed to emit event: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
async fn add_favourite(db: tauri::State<'_, state::AppState>, path: String) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO favourites (path) VALUES (?1)
             ON CONFLICT(path) DO NOTHING",
    )
    .bind(path)
    .execute(&db.db)
    .await
    .map_err(|e| format!("Error adding favourite: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn get_favourites(db: tauri::State<'_, state::AppState>) -> Result<Vec<Favourite>, String> {
    let favourites: Vec<Favourite> = sqlx::query_as::<_, Favourite>("SELECT path FROM favourites")
        .fetch(&db.db)
        .try_collect()
        .await
        .map_err(|e| format!("Failed to get favourites {}", e))?;
    Ok(favourites)
}

#[tauri::command]
async fn remove_favourite(
    db: tauri::State<'_, state::AppState>,
    path: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM favourites WHERE path = ?1")
        .bind(path)
        .execute(&db.db)
        .await
        .map_err(|e| format!("Error removing favourite: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn clear_favourites(db: tauri::State<'_, state::AppState>) -> Result<(), String> {
    sqlx::query("DELETE FROM favourites")
        .execute(&db.db)
        .await
        .map_err(|e| format!("Error removing favourites: {}", e))?;
    Ok(())
}

async fn setup_db(app: &App) -> SqlitePool {
    let mut dir = app.path().app_data_dir().expect("failed to get data_dir");
    std::fs::create_dir_all(&dir).expect("error creating app data dir");

    let mut db_path = PathBuf::from(&dir);
    db_path.push(format!("{}.db", env!("CARGO_PKG_NAME")));

    let opts: SqliteConnectOptions = SqliteConnectOptions::from_str(db_path.to_str().unwrap())
        .unwrap()
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(4) // small pool is best
        .acquire_timeout(std::time::Duration::from_secs(10))
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                // Apply to every pooled connection
                sqlx::query("PRAGMA busy_timeout = 5000;")
                    .execute(&mut *conn)
                    .await?;
                sqlx::query("PRAGMA temp_store = MEMORY;")
                    .execute(&mut *conn)
                    .await?;
                sqlx::query("PRAGMA cache_size = -40000;")
                    .execute(&mut *conn)
                    .await?; // ~40MB
                sqlx::query("PRAGMA wal_autocheckpoint = 1000;")
                    .execute(&mut *conn)
                    .await?;
                // Optional (platform‑dependent help):
                sqlx::query("PRAGMA mmap_size = 268435456;")
                    .execute(&mut *conn)
                    .await?; // 256MB
                Ok::<_, sqlx::Error>(())
            })
        })
        .connect_with(opts)
        .await
        .expect("failed to connect sqlite");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    let _ = sqlx::query("PRAGMA optimize;").execute(&pool).await;

    log::info!("DB path: {}", db_path.display());
    pool
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Webview,
                ))
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            scan_folder,
            export_favourites,
            add_favourite,
            remove_favourite,
            get_favourites,
            clear_favourites
        ])
        .setup(|app| {
            tauri::async_runtime::block_on(async move {
                let db = setup_db(&app).await;
                app.manage(state::AppState { db });
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

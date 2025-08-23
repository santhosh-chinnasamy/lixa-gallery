use futures::TryStreamExt;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePoolOptions, Sqlite};
use std::{
    fs::{self},
    path::PathBuf,
};
use tauri::{App, AppHandle, Emitter, Manager as _};

mod converter;
mod state;
use crate::{converter::PhotoMetadata, state::Favourite};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
async fn scan_folder(
    app: AppHandle,
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
    println!("path: {}", path);
    sqlx::query("INSERT INTO favourites (path) VALUES (?1)")
        .bind(path)
        .execute(&db.db)
        .await
        .map_err(|e| format!("Error adding favourite: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn get_favourites(db: tauri::State<'_, state::AppState>) -> Result<Vec<Favourite>, String> {
    let favourites: Vec<Favourite> =
        sqlx::query_as::<_, Favourite>("SELECT DISTINCT(path) FROM favourites")
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

async fn setup_db(app: &App) -> state::Db {
    let mut path = app.path().app_data_dir().expect("failed to get data_dir");

    match std::fs::create_dir_all(path.clone()) {
        Ok(_) => {}
        Err(err) => {
            panic!("error creating directory {}", err);
        }
    };

    path.push(env!("CARGO_PKG_NAME").to_string() + ".db");

    Sqlite::create_database(
        format!(
            "sqlite:///{}",
            path.to_str().expect("path should be something")
        )
        .as_str(),
    )
    .await
    .expect("failed to create database");

    let db = SqlitePoolOptions::new()
        .connect(
            format!(
                "sqlite:///{}",
                path.to_str().expect("path should be something")
            )
            .as_str(),
        )
        .await
        .unwrap();

    log::info!("DB path: {:?}", path);
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("Error running DB migrations");

    let _ = sqlx::query("PRAGMA journal_mode=WAL;").execute(&db).await;

    db
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

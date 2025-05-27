use futures::TryStreamExt;
use std::{
    fs::{self},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use sqlx::{migrate::MigrateDatabase, prelude::FromRow, sqlite::SqlitePoolOptions, Pool, Sqlite};
use tauri::{App, AppHandle, Emitter, Manager as _};

type Db = Pool<Sqlite>;

struct AppState {
    db: Db,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct Favourite {
    path: String,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn scan_folder(path: &str) -> Result<Vec<String>, String> {
    let mut result = Vec::new();

    let path = PathBuf::from(path);
    match fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let file_path = entry.path();
                        // Filter for image extensions only
                        if let Some(ext) = file_path.extension().and_then(|s| s.to_str()) {
                            let ext = ext.to_lowercase();
                            if ["jpg", "jpeg", "png", "webp", "bmp", "gif"].contains(&ext.as_str())
                            {
                                result.push(file_path.display().to_string());
                            }
                        }
                    }
                    Err(e) => eprintln!("Error reading entry: {}", e),
                }
            }
            Ok(result)
        }
        Err(e) => Err(format!("Error reading folder: {}", e)),
    }
}

#[tauri::command]
async fn export_favourites(
    app: AppHandle,
    db: tauri::State<'_, AppState>,
    destination: &str,
) -> Result<Vec<String>, String> {
    let favourites = get_favourites(db).await?;
    let files = favourites
        .into_iter()
        .map(|favorite| favorite.path)
        .collect::<Vec<String>>();

    let mut counter = 0;
    files
        .into_iter()
        .map(|file_path| {
            let file = PathBuf::from(&file_path);
            let name = file
                .file_name()
                .ok_or_else(|| format!("Invalid path (no file name): {}", file_path))?;
            let name_str = name
                .to_str()
                .ok_or_else(|| format!("Non-UTF8 file name: {}", file_path))?;

            let destination_path = PathBuf::from(destination).join(name_str);
            fs::copy(&file_path, &destination_path)
                .map_err(|e| format!("Error copying file: {}", e))?;

            counter += 1;
            // emit event to update the progress bar
            app.emit("export-progress", counter)
                .expect("failed to emit progress event");
            Ok(name_str.to_string())
        })
        .collect()
}

#[tauri::command]
async fn add_favourite(db: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    println!("path: {}", path);
    sqlx::query("INSERT INTO favourites (path) VALUES (?1)")
        .bind(path)
        .execute(&db.db)
        .await
        .map_err(|e| format!("Error adding favourite: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn get_favourites(db: tauri::State<'_, AppState>) -> Result<Vec<Favourite>, String> {
    let favourites: Vec<Favourite> =
        sqlx::query_as::<_, Favourite>("SELECT DISTINCT(path) FROM favourites")
            .fetch(&db.db)
            .try_collect()
            .await
            .map_err(|e| format!("Failed to get favourites {}", e))?;
    Ok(favourites)
}

#[tauri::command]
async fn remove_favourite(db: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    sqlx::query("DELETE FROM favourites WHERE path = ?1")
        .bind(path)
        .execute(&db.db)
        .await
        .map_err(|e| format!("Error removing favourite: {}", e))?;
    Ok(())
}

async fn setup_db(app: &App) -> Db {
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
        ])
        .setup(|app| {
            tauri::async_runtime::block_on(async move {
                let db = setup_db(&app).await;
                app.manage(AppState { db });
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

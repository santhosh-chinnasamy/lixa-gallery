use crate::domain::models::{Favourite, PhotoMetadata};
use crate::tauri_api::state::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn scan_folder(
    state: State<'_, AppState>,
    path: &str,
) -> Result<Vec<PhotoMetadata>, String> {
    state
        .gallery
        .scan_folder(path, &state.thumbnail_path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_favourites(
    app: AppHandle,
    state: State<'_, AppState>,
    destination: &str,
) -> Result<(), String> {
    state
        .favourite
        .export_favourites(app, destination)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_favourite(state: State<'_, AppState>, path: String) -> Result<(), String> {
    state
        .favourite
        .add_favourite(path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_favourites(state: State<'_, AppState>) -> Result<Vec<Favourite>, String> {
    state
        .favourite
        .get_favourites()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_favourite(state: State<'_, AppState>, path: String) -> Result<(), String> {
    state
        .favourite
        .remove_favourite(path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_favourites(state: State<'_, AppState>) -> Result<(), String> {
    state
        .favourite
        .clear_favourites()
        .await
        .map_err(|e| e.to_string())
}

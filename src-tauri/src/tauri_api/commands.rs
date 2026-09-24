use crate::tauri_api::events::TauriEventHub;
use crate::tauri_api::state::AppState;
use gallery_core::models::{Favourite, PhotoMetadata};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn scan_folder(
    state: State<'_, AppState>,
    path: &str,
    mode: gallery_core::models::LoadingMode,
) -> Result<Vec<PhotoMetadata>, String> {
    state
        .gallery
        .scan_folder(path, &state.thumbnail_path, mode)
        .await
        .map_err(|e| e.to_string())
}


#[tauri::command]
pub async fn export_favourites(
    app: AppHandle,
    state: State<'_, AppState>,
    destination: &str,
    mode: &str,
) -> Result<(), String> {
    let events = TauriEventHub::new(app);
    state
        .favourite
        .export_favourites(&events, destination, mode)
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

#[tauri::command]
pub async fn get_folder_tree(
    state: State<'_, AppState>,
    path: &str,
) -> Result<gallery_core::models::FolderNode, String> {
    state
        .gallery
        .get_folder_tree(path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_logs_dir(app_handle: AppHandle) -> Result<String, String> {
    use tauri::Manager;
    let log_dir = app_handle.path().app_log_dir().map_err(|e| e.to_string())?;
    let log_file = log_dir.join("Lixa Gallery.log");
    let target = if log_file.exists() { log_file } else { log_dir };
    tauri_plugin_opener::reveal_item_in_dir(&target).map_err(|e| e.to_string())?;
    Ok(target.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_log_dir(app_handle: AppHandle) -> Result<String, String> {
    use tauri::Manager;
    let log_dir = app_handle.path().app_log_dir().map_err(|e| e.to_string())?;
    Ok(log_dir.to_string_lossy().to_string())
}

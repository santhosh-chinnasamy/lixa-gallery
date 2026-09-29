use crate::tauri_api::events::TauriEventHub;
use crate::tauri_api::state::AppState;
use gallery_core::models::{
    ExportOptions, Favourite, FavouriteFolderGroup, FolderNode, LoadingMode, PhotoMetadata,
    Workspace,
};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn scan_folder(
    state: State<'_, AppState>,
    path: &str,
    mode: LoadingMode,
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
    destination: String,
    mode: String,
    paths: Option<Vec<String>>,
    preserve_folder_structure: Option<bool>,
) -> Result<(), String> {
    let events = TauriEventHub::new(app);
    let options = ExportOptions {
        destination,
        mode,
        paths,
        preserve_folder_structure: preserve_folder_structure.unwrap_or(false),
    };
    state
        .favourite
        .export_favourites(&events, options)
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
pub async fn get_favourite_photos(
    state: State<'_, AppState>,
    folder_scope: Option<String>,
) -> Result<Vec<PhotoMetadata>, String> {
    state
        .favourite
        .get_favourite_photos(folder_scope)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_favourite_folder_groups(
    state: State<'_, AppState>,
) -> Result<Vec<FavouriteFolderGroup>, String> {
    state
        .favourite
        .get_favourite_folder_groups()
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
pub async fn clear_favourites_by_prefix(
    state: State<'_, AppState>,
    prefix: String,
) -> Result<(), String> {
    state
        .favourite
        .clear_favourites_by_prefix(&prefix)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_workspace(
    state: State<'_, AppState>,
    path: String,
    name: Option<String>,
) -> Result<(), String> {
    let folder_name = name.unwrap_or_else(|| {
        std::path::Path::new(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&path)
            .to_string()
    });
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let ws = Workspace {
        id: format!("ws-{}", nanos),
        name: folder_name,
        root_path: path,
        created_at: now,
        last_opened_at: now,
    };
    state
        .favourite
        .upsert_workspace(ws)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_workspaces(state: State<'_, AppState>) -> Result<Vec<Workspace>, String> {
    state
        .favourite
        .get_workspaces()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_folder_tree(state: State<'_, AppState>, path: &str) -> Result<FolderNode, String> {
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

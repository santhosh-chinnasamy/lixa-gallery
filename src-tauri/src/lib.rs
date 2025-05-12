use std::{
    fs::{self},
    path::PathBuf,
};

use tauri::Manager;

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
fn export_favourites(destination: &str, files: Vec<String>) -> Result<Vec<String>, String> {
    println!("destination: {}", destination);
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
            // TODO: add file to destination
            let destination_path = PathBuf::from(destination).join(name_str);
            fs::copy(&file_path, &destination_path).map_err(|e| format!("Error copying file: {}", e))?;
            Ok(name_str.to_string())
        })
        .collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let main_window = app.get_webview_window("main").unwrap();
            main_window.open_devtools();
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![scan_folder, export_favourites])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

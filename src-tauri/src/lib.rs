use std::{fs, path::PathBuf};

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![scan_folder])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

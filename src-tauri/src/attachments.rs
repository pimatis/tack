use base64::Engine as _;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub(crate) fn attachments_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = data_dir.join("attachments");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub(crate) fn base64_encode(input: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(input)
}

// strip data url prefix (e.g. "data:image/png;base64,") and whitespace, then decode
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.split(',').next_back().unwrap_or(input);
    let input = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect::<Vec<u8>>();
    base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_attachment(app: AppHandle, id: String, file_data: String) -> Result<String, String> {
    let dir = attachments_dir(&app)?;
    let bytes = base64_decode(&file_data)?;
    let path = dir.join(&id);
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

// read binary file from disk and return as base64 data url
#[tauri::command]
pub fn read_attachment(app: AppHandle, id: String, mime_type: String) -> Result<String, String> {
    let dir = attachments_dir(&app)?;
    let path = dir.join(&id);
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(format!(
        "data:{};base64,{}",
        mime_type,
        base64_encode(&bytes)
    ))
}

// delete file from disk
#[tauri::command]
pub fn delete_attachment(app: AppHandle, id: String) -> Result<(), String> {
    let dir = attachments_dir(&app)?;
    let path = dir.join(&id);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// copy attachment file to user-selected path
#[tauri::command]
pub fn download_attachment(app: AppHandle, id: String, dest_path: String) -> Result<(), String> {
    let dir = attachments_dir(&app)?;
    let src = dir.join(&id);
    if !src.exists() {
        return Err("attachment file not found".to_string());
    }
    std::fs::copy(&src, &dest_path).map_err(|e| e.to_string())?;
    Ok(())
}

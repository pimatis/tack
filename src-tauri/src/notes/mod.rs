use serde::Serialize;

#[derive(Serialize)]
pub struct NoteInfo {
    pub name: String,
    pub path: String,
    pub modified: u64,
}

// list .md files in the notes folder, most recently modified first
#[tauri::command]
pub fn list_notes(dir: String) -> Result<Vec<NoteInfo>, String> {
    let mut notes = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        let is_md = path.extension().map(|e| e == "md").unwrap_or(false);
        if !path.is_file() || !is_md {
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        notes.push(NoteInfo {
            name,
            path: path.to_string_lossy().into_owned(),
            modified,
        });
    }
    notes.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(notes)
}

#[derive(Serialize)]
pub struct NoteFull {
    pub path: String,
    pub name: String,
    pub content: String,
}

// read every .md note with its content; used to build the search index
#[tauri::command]
pub fn read_notes(dir: String) -> Result<Vec<NoteFull>, String> {
    let mut notes = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        let is_md = path.extension().map(|e| e == "md").unwrap_or(false);
        if !path.is_file() || !is_md {
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        notes.push(NoteFull {
            path: path.to_string_lossy().into_owned(),
            name,
            content,
        });
    }
    Ok(notes)
}

// rename a note file; creates the target folder (e.g. archive) when needed
#[tauri::command]
pub fn rename_note(old_path: String, new_path: String) -> Result<(), String> {
    if let Some(parent) = std::path::Path::new(&new_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&old_path, &new_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_note(path: String) -> Result<(), String> {
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}

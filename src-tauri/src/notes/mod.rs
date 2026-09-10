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

// reject unsafe folder/file names coming from the ui
fn sanitize_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        return Err("Invalid name".into());
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains(':') {
        return Err("Name cannot contain path separators".into());
    }
    Ok(trimmed.to_string())
}

// every subfolder of the notes folder, recursively, as relative paths;
// archive and trash stay hidden from the tree
fn walk_folders(base: &std::path::Path, dir: &std::path::Path, out: &mut Vec<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let rel = match path.strip_prefix(base) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if rel == "archive" || rel == "trash" {
            continue;
        }
        out.push(rel.clone());
        walk_folders(base, &path, out);
    }
}

#[tauri::command]
pub fn list_note_folders(dir: String) -> Result<Vec<String>, String> {
    let base = std::path::Path::new(&dir);
    let mut out = Vec::new();
    walk_folders(base, base, &mut out);
    out.sort();
    Ok(out)
}

// every .md note under dir and its subfolders; archive and trash stay hidden
fn walk_notes(base: &std::path::Path, dir: &std::path::Path, out: &mut Vec<NoteInfo>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let rel = match path.strip_prefix(base) {
                Ok(r) => r.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            if rel == "archive" || rel == "trash" {
                continue;
            }
            walk_notes(base, &path, out);
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
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
            out.push(NoteInfo {
                name,
                path: path.to_string_lossy().into_owned(),
                modified,
            });
        }
    }
}

#[tauri::command]
pub fn list_notes_deep(dir: String) -> Result<Vec<NoteInfo>, String> {
    let base = std::path::Path::new(&dir);
    let mut out = Vec::new();
    walk_notes(base, base, &mut out);
    Ok(out)
}

// recursive read for the search index; includes subfolder notes
#[tauri::command]
pub fn read_notes_deep(dir: String) -> Result<Vec<NoteFull>, String> {
    let mut infos = Vec::new();
    walk_notes(std::path::Path::new(&dir), std::path::Path::new(&dir), &mut infos);
    Ok(infos
        .into_iter()
        .map(|info| {
            let content = std::fs::read_to_string(&info.path).unwrap_or_default();
            NoteFull {
                path: info.path,
                name: info.name,
                content,
            }
        })
        .collect())
}

// create a notes subfolder; the name may be a relative path like "a/b",
// so each segment is sanitized on its own (blocks traversal and separators)
#[tauri::command]
pub fn create_folder(dir: String, name: String) -> Result<(), String> {
    let segments = name
        .split('/')
        .map(sanitize_name)
        .collect::<Result<Vec<_>, _>>()?;
    std::fs::create_dir_all(std::path::Path::new(&dir).join(segments.join("/")))
        .map_err(|e| e.to_string())
}

// only empty folders can be deleted; notes must be moved out first
#[tauri::command]
pub fn delete_folder(path: String) -> Result<(), String> {
    std::fs::remove_dir(&path).map_err(|e| e.to_string())
}

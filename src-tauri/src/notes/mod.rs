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
// system folders (archive, trash, .tack) stay hidden from the tree
fn is_hidden_rel(rel: &str) -> bool {
    rel == "archive" || rel == "trash" || rel == ".tack" || rel.starts_with(".tack/")
}

// legacy archive/trash folders lived at the notes root; move them under the
// hidden .tack system folder so they stay out of the user's notes
fn migrate_system_dirs(base: &std::path::Path) {
    let tack = base.join(".tack");
    for name in ["archive", "trash"] {
        let legacy = base.join(name);
        if !legacy.is_dir() {
            continue;
        }
        let target = tack.join(name);
        if !target.exists() {
            let _ = std::fs::create_dir_all(&tack);
            // fast path: nothing to merge, just move the whole folder
            if std::fs::rename(&legacy, &target).is_ok() {
                continue;
            }
        }
        // target exists: merge file by file, never overwrite
        let _ = std::fs::create_dir_all(&target);
        if let Ok(entries) = std::fs::read_dir(&legacy) {
            for entry in entries.flatten() {
                let dest = target.join(entry.file_name());
                if !dest.exists() {
                    let _ = std::fs::rename(entry.path(), dest);
                }
            }
        }
        let _ = std::fs::remove_dir(&legacy);
    }
}

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
        if is_hidden_rel(&rel) {
            continue;
        }
        out.push(rel.clone());
        walk_folders(base, &path, out);
    }
}

#[tauri::command]
pub fn list_note_folders(dir: String) -> Result<Vec<String>, String> {
    let base = std::path::Path::new(&dir);
    migrate_system_dirs(base);
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
            if is_hidden_rel(&rel) {
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
    migrate_system_dirs(base);
    let mut out = Vec::new();
    walk_notes(base, base, &mut out);
    Ok(out)
}

// recursive read for the search index; includes subfolder notes
#[tauri::command]
pub fn read_notes_deep(dir: String) -> Result<Vec<NoteFull>, String> {
    let base = std::path::Path::new(&dir);
    migrate_system_dirs(base);
    let mut infos = Vec::new();
    walk_notes(base, base, &mut infos);
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

#[derive(Serialize)]
pub struct NoteDetails {
    pub kind: String, // "note" | "folder"
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub created: u64,
    pub modified: u64,
    pub word_count: u64,
    pub note_count: u64, // folder only, recursive
    pub folder_count: u64,
}

fn epoch_millis(t: std::io::Result<std::time::SystemTime>) -> u64 {
    t.ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// stats for the "get info" dialog: file or folder metadata
#[tauri::command]
pub fn note_info(path: String) -> Result<NoteDetails, String> {
    let p = std::path::Path::new(&path);
    let meta = std::fs::metadata(p).map_err(|e| e.to_string())?;
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (kind, size, note_count, folder_count, word_count);
    if meta.is_dir() {
        let mut total = 0u64;
        let mut notes = 0u64;
        let mut dirs = 0u64;
        let mut words = 0u64;
        let mut stack = vec![p.to_path_buf()];
        while let Some(d) = stack.pop() {
            let entries = match std::fs::read_dir(&d) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let ep = entry.path();
                if ep.is_dir() {
                    dirs += 1;
                    stack.push(ep);
                } else if ep.extension().map(|e| e == "md").unwrap_or(false) {
                    notes += 1;
                    if let Ok(content) = std::fs::read_to_string(&ep) {
                        words += content.split_whitespace().count() as u64;
                    }
                    total += entry.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
        }
        kind = "folder".to_string();
        size = total;
        note_count = notes;
        folder_count = dirs;
        word_count = words;
    } else {
        let words = std::fs::read_to_string(p)
            .map(|c| c.split_whitespace().count() as u64)
            .unwrap_or(0);
        kind = "note".to_string();
        size = meta.len();
        note_count = 0;
        folder_count = 0;
        word_count = words;
    }
    Ok(NoteDetails {
        kind,
        name,
        path,
        size_bytes: size,
        created: epoch_millis(meta.created()),
        modified: epoch_millis(meta.modified()),
        word_count,
        note_count,
        folder_count,
    })
}

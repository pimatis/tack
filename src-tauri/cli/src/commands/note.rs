use rusqlite::{params, Connection};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::db::*;

// notes live as plain .md files inside the user's notes folder; the gui
// watches this folder, so every cli change shows up in the ui live

struct NoteFile {
    name: String,   // file name with .md
    rel: String,    // path relative to the notes root ("test/a.md", root is "")
    path: PathBuf,
    modified: u64,
    size: u64,
}

pub fn resolve_dir(cli_dir: Option<&PathBuf>, conn: &Connection) -> Result<PathBuf> {
    if let Some(dir) = cli_dir {
        let dir = dir.clone();
        if !dir.is_dir() {
            return Err(format!("notes folder does not exist: {}", dir.display()));
        }
        return Ok(dir);
    }
    let stored = get_setting(conn, "notesFolder")
        .ok_or("No notes folder configured - pick one in the app or pass --notes-dir")?;
    let dir = PathBuf::from(stored);
    if !dir.is_dir() {
        return Err(format!("notes folder does not exist: {}", dir.display()));
    }
    Ok(dir)
}

// same rules as the gui's sanitizeName: no separators, no .md suffix
fn sanitize_name(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    let no_ext = trimmed
        .strip_suffix(".md")
        .or_else(|| trimmed.strip_suffix(".MD"))
        .unwrap_or(trimmed);
    let name: String = no_ext
        .trim()
        .chars()
        .map(|c| if c == '/' || c == '\\' || c == ':' { '-' } else { c })
        .collect();
    let name = name.trim().to_string();
    if name.is_empty() || name == "." || name == ".." {
        return Err("Invalid name".to_string());
    }
    Ok(name)
}

// replicate javascript encodeURIComponent so mention links written by the
// gui (](note:ENCODED) stay byte-compatible with cli rewrites
fn encode_uri_component(s: &str) -> String {
    let mut out = String::new();
    for byte in s.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*'
            | b'\'' | b'(' | b')' => out.push(*byte as char),
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

fn epoch_ms(meta: &std::fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn format_ms(ms: u64) -> String {
    chrono::DateTime::from_timestamp((ms / 1000) as i64, 0)
        .map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

// recursive walk of every .md file; hidden dirs (.tack) are skipped
fn walk_notes(root: &Path) -> Result<Vec<NoteFile>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries =
            std::fs::read_dir(&dir).map_err(|e| format!("Failed to read folder: {}", e))?;
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let hidden = p
                    .file_name()
                    .map(|n| n.to_string_lossy().starts_with('.'))
                    .unwrap_or(false);
                if !hidden {
                    stack.push(p);
                }
            } else if p.extension().map(|e| e == "md").unwrap_or(false) {
                let meta = entry.metadata().ok();
                let rel = p
                    .strip_prefix(root)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push(NoteFile {
                    name: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                    rel: rel.clone(),
                    path: p.clone(),
                    modified: meta.as_ref().map(epoch_ms).unwrap_or(0),
                    size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                });
            }
        }
    }
    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(out)
}

// resolve a user-supplied note reference: rel path first, then a unique
// file-name match anywhere in the tree
fn resolve_note(root: &Path, input: &str) -> Result<NoteFile> {
    let notes = walk_notes(root)?;
    let by_rel = notes.iter().find(|n| n.rel == input);
    if let Some(n) = by_rel {
        return Ok(NoteFile {
            name: n.name.clone(),
            rel: n.rel.clone(),
            path: n.path.clone(),
            modified: n.modified,
            size: n.size,
        });
    }
    let with_ext = if input.ends_with(".md") {
        input.to_string()
    } else {
        format!("{}.md", input)
    };
    let matches: Vec<&NoteFile> = notes.iter().filter(|n| n.name == with_ext).collect();
    match matches.len() {
        1 => {
            let n = matches[0];
            Ok(NoteFile {
                name: n.name.clone(),
                rel: n.rel.clone(),
                path: n.path.clone(),
                modified: n.modified,
                size: n.size,
            })
        }
        0 => Err(format!("Note not found: {}", input)),
        _ => {
            let paths = matches.iter().map(|m| m.rel.as_str()).collect::<Vec<_>>().join(", ");
            Err(format!("Ambiguous note name, use a folder path: {}", paths))
        }
    }
}

// after a rename/move every gui mention link ](note:OLD) must be rewritten
fn rewrite_mentions(root: &Path, remap: &[(String, String)]) -> Result<()> {
    for note in walk_notes(root)? {
        let mut content = match std::fs::read_to_string(&note.path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut changed = false;
        for (old, new) in remap {
            let token = format!("](note:{})", encode_uri_component(old));
            if content.contains(&token) {
                content = content.replace(&token, &format!("](note:{})", encode_uri_component(new)));
                changed = true;
            }
        }
        if changed {
            std::fs::write(&note.path, content)
                .map_err(|e| format!("Failed to update links: {}", e))?;
        }
    }
    Ok(())
}

fn archive_dir(root: &Path) -> PathBuf {
    root.join(".tack").join("archive")
}

fn trash_dir(root: &Path) -> PathBuf {
    root.join(".tack").join("trash")
}

fn first_free_name(dir: &Path, stem: &str) -> String {
    let mut name = format!("{}.md", stem);
    for i in 2.. {
        if !dir.join(&name).exists() {
            return name;
        }
        name = format!("{} {}.md", stem, i);
    }
    name
}

// ---- note actions ----

pub fn list(root: &Path, json: bool, folder: Option<&str>) -> Result<()> {
    let mut notes = walk_notes(root)?;
    if let Some(rel) = folder {
        let prefix = rel.trim_matches('/').to_string();
        notes.retain(|n| {
            n.rel.starts_with(&format!("{}/", prefix))
                && !n.rel[prefix.len() + 1..].contains('/')
        });
    }
    if json {
        let items: Vec<serde_json::Value> = notes
            .iter()
            .map(|n| {
                json!({
                    "name": n.name,
                    "folder": n.rel.rsplit_once('/').map(|(f, _)| f.to_string()).unwrap_or_default(),
                    "path": n.rel,
                    "size_bytes": n.size,
                    "modified": n.modified,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json!({ "notes": items })).map_err(|e| e.to_string())?);
        return Ok(());
    }
    let rows: Vec<Vec<String>> = notes
        .iter()
        .map(|n| {
            vec![
                n.name.clone(),
                n.rel.rsplit_once('/').map(|(f, _)| f.to_string()).unwrap_or_else(|| "-".to_string()),
                format_size(n.size),
                format_ms(n.modified),
            ]
        })
        .collect();
    print_table(&["Name", "Folder", "Size", "Modified"], &rows);
    Ok(())
}

pub fn create(root: &Path, json: bool, title: &str, folder: Option<&str>, content: Option<&str>) -> Result<()> {
    let stem = sanitize_name(title)?;
    let target_dir = match folder {
        Some(rel) => {
            let dir = root.join(rel.trim_matches('/'));
            std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create folder: {}", e))?;
            dir
        }
        None => root.to_path_buf(),
    };
    let name = first_free_name(&target_dir, &stem);
    let path = target_dir.join(&name);
    std::fs::write(&path, content.unwrap_or("")).map_err(|e| format!("Failed to create note: {}", e))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true, "action": "note_created",
                "name": name,
                "path": path.to_string_lossy(),
            })).map_err(|e| e.to_string())?
        );
    } else {
        println!("Created note: {}", path.display());
    }
    Ok(())
}

pub fn show(root: &Path, json: bool, input: &str) -> Result<()> {
    let note = resolve_note(root, input)?;
    let content = std::fs::read_to_string(&note.path).map_err(|e| format!("Failed to read note: {}", e))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "name": note.name, "path": note.rel, "content": content })).map_err(|e| e.to_string())?
        );
    } else {
        println!("{}", content);
    }
    Ok(())
}

pub fn update(root: &Path, json: bool, input: &str, content: Option<&str>, stdin: bool) -> Result<()> {
    let note = resolve_note(root, input)?;
    let next = if stdin {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).map_err(|e| format!("Failed to read stdin: {}", e))?;
        buf
    } else {
        content.ok_or("Nothing to write: pass --content or --stdin")?.to_string()
    };
    std::fs::write(&note.path, next).map_err(|e| format!("Failed to write note: {}", e))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({ "success": true, "action": "note_updated", "path": note.rel })).map_err(|e| e.to_string())?);
    } else {
        println!("Updated note: {}", note.path.display());
    }
    Ok(())
}

pub fn rename(root: &Path, json: bool, input: &str, title: &str) -> Result<()> {
    let note = resolve_note(root, input)?;
    let stem = sanitize_name(title)?;
    let parent = note.path.parent().unwrap_or(root);
    let target = parent.join(first_free_name(parent, &stem));
    std::fs::rename(&note.path, &target).map_err(|e| format!("Failed to rename note: {}", e))?;
    let old_abs = note.path.to_string_lossy().into_owned();
    let new_abs = target.to_string_lossy().into_owned();
    rewrite_mentions(root, &[(old_abs, new_abs)])?;
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({ "success": true, "action": "note_renamed", "path": target.to_string_lossy() })).map_err(|e| e.to_string())?);
    } else {
        println!("Renamed note: {}", target.display());
    }
    Ok(())
}

pub fn move_note(root: &Path, json: bool, input: &str, folder: Option<&str>) -> Result<()> {
    let note = resolve_note(root, input)?;
    let target_dir = match folder {
        Some(rel) if !rel.trim_matches('/').is_empty() => root.join(rel.trim_matches('/')),
        _ => root.to_path_buf(),
    };
    if !target_dir.is_dir() {
        return Err(format!("Folder does not exist: {}", target_dir.display()));
    }
    let target = target_dir.join(&note.name);
    if target.exists() {
        return Err(format!("A note with this name already exists: {}", target.display()));
    }
    std::fs::rename(&note.path, &target).map_err(|e| format!("Failed to move note: {}", e))?;
    let old_abs = note.path.to_string_lossy().into_owned();
    let new_abs = target.to_string_lossy().into_owned();
    rewrite_mentions(root, &[(old_abs, new_abs)])?;
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({ "success": true, "action": "note_moved", "path": target.to_string_lossy() })).map_err(|e| e.to_string())?);
    } else {
        println!("Moved note: {}", target.display());
    }
    Ok(())
}

pub fn archive(root: &Path, json: bool, input: &str) -> Result<()> {
    let note = resolve_note(root, input)?;
    std::fs::create_dir_all(archive_dir(root)).map_err(|e| format!("Failed to create archive: {}", e))?;
    let target = archive_dir(root).join(&note.name);
    if target.exists() {
        return Err("An archived note with this name already exists".to_string());
    }
    std::fs::rename(&note.path, &target).map_err(|e| format!("Failed to archive note: {}", e))?;
    let old_abs = note.path.to_string_lossy().into_owned();
    let new_abs = target.to_string_lossy().into_owned();
    rewrite_mentions(root, &[(old_abs, new_abs)])?;
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({ "success": true, "action": "note_archived", "path": target.to_string_lossy() })).map_err(|e| e.to_string())?);
    } else {
        println!("Archived note: {}", target.display());
    }
    Ok(())
}

pub fn archived_list(root: &Path, json: bool) -> Result<()> {
    let dir = archive_dir(root);
    if !dir.is_dir() {
        println!("(empty)");
        return Ok(());
    }
    let notes = walk_notes(&dir)?;
    if json {
        let items: Vec<serde_json::Value> = notes
            .iter()
            .map(|n| json!({ "name": n.name, "size_bytes": n.size, "modified": n.modified }))
            .collect();
        println!("{}", serde_json::to_string_pretty(&json!({ "archived": items })).map_err(|e| e.to_string())?);
        return Ok(());
    }
    let rows: Vec<Vec<String>> = notes
        .iter()
        .map(|n| vec![n.name.clone(), format_size(n.size), format_ms(n.modified)])
        .collect();
    print_table(&["Name", "Size", "Modified"], &rows);
    Ok(())
}

pub fn unarchive(root: &Path, json: bool, input: &str) -> Result<()> {
    let dir = archive_dir(root);
    let name = if input.ends_with(".md") { input.to_string() } else { format!("{}.md", input) };
    let source = dir.join(&name);
    if !source.exists() {
        return Err(format!("Archived note not found: {}", input));
    }
    let target = root.join(&name);
    if target.exists() {
        return Err("A note with this name already exists".to_string());
    }
    std::fs::rename(&source, &target).map_err(|e| format!("Failed to restore note: {}", e))?;
    let old_abs = source.to_string_lossy().into_owned();
    let new_abs = target.to_string_lossy().into_owned();
    rewrite_mentions(root, &[(old_abs, new_abs)])?;
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({ "success": true, "action": "note_unarchived", "path": target.to_string_lossy() })).map_err(|e| e.to_string())?);
    } else {
        println!("Restored note: {}", target.display());
    }
    Ok(())
}

pub fn delete(root: &Path, json: bool, input: &str) -> Result<()> {
    let note = resolve_note(root, input)?;
    std::fs::create_dir_all(trash_dir(root)).map_err(|e| format!("Failed to create trash: {}", e))?;
    let target = trash_dir(root).join(&note.name);
    if target.exists() {
        return Err("A trashed note with this name already exists".to_string());
    }
    std::fs::rename(&note.path, &target).map_err(|e| format!("Failed to delete note: {}", e))?;
    let old_abs = note.path.to_string_lossy().into_owned();
    let new_abs = target.to_string_lossy().into_owned();
    rewrite_mentions(root, &[(old_abs, new_abs)])?;
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({ "success": true, "action": "note_deleted" })).map_err(|e| e.to_string())?);
    } else {
        println!("Moved to trash: {}", note.name);
    }
    Ok(())
}

pub fn trash_list(root: &Path, json: bool) -> Result<()> {
    let dir = trash_dir(root);
    if !dir.is_dir() {
        println!("(empty)");
        return Ok(());
    }
    let notes = walk_notes(&dir)?;
    if json {
        let items: Vec<serde_json::Value> = notes
            .iter()
            .map(|n| json!({ "name": n.name, "size_bytes": n.size, "modified": n.modified }))
            .collect();
        println!("{}", serde_json::to_string_pretty(&json!({ "trashed": items })).map_err(|e| e.to_string())?);
        return Ok(());
    }
    let rows: Vec<Vec<String>> = notes
        .iter()
        .map(|n| vec![n.name.clone(), format_size(n.size), format_ms(n.modified)])
        .collect();
    print_table(&["Name", "Size", "Modified"], &rows);
    Ok(())
}

pub fn restore(root: &Path, json: bool, name: &str) -> Result<()> {
    let file_name = if name.ends_with(".md") { name.to_string() } else { format!("{}.md", name) };
    let source = trash_dir(root).join(&file_name);
    if !source.exists() {
        return Err(format!("Trashed note not found: {}", name));
    }
    // never overwrite an existing note, same as the gui
    let stem = file_name.trim_end_matches(".md").to_string();
    let target_name = first_free_name(root, &stem);
    let target = root.join(&target_name);
    std::fs::rename(&source, &target).map_err(|e| format!("Failed to restore note: {}", e))?;
    let old_abs = source.to_string_lossy().into_owned();
    let new_abs = target.to_string_lossy().into_owned();
    rewrite_mentions(root, &[(old_abs, new_abs)])?;
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({ "success": true, "action": "note_restored", "path": target.to_string_lossy() })).map_err(|e| e.to_string())?);
    } else {
        println!("Restored note: {}", target.display());
    }
    Ok(())
}

pub fn purge(root: &Path, json: bool, name: Option<&str>, all: bool) -> Result<()> {
    let dir = trash_dir(root);
    if all {
        let notes = walk_notes(&dir)?;
        for note in notes {
            std::fs::remove_file(&note.path).map_err(|e| format!("Failed to purge: {}", e))?;
        }
        if json {
            println!("{}", serde_json::to_string_pretty(&json!({ "success": true, "action": "trash_emptied" })).map_err(|e| e.to_string())?);
        } else {
            println!("Trash emptied");
        }
        return Ok(());
    }
    let Some(name) = name else {
        return Err("Specify a note name or --all".to_string());
    };
    let file_name = if name.ends_with(".md") { name.to_string() } else { format!("{}.md", name) };
    let source = dir.join(&file_name);
    if !source.exists() {
        return Err(format!("Trashed note not found: {}", name));
    }
    std::fs::remove_file(&source).map_err(|e| format!("Failed to purge: {}", e))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({ "success": true, "action": "note_purged" })).map_err(|e| e.to_string())?);
    } else {
        println!("Permanently deleted: {}", file_name);
    }
    Ok(())
}

// pins are mirrored into the settings db under notesPinned:<folder>;
// the gui reads the same key so both sides stay in sync
const PIN_KEY: &str = "notesPinned";

fn pin_key(root: &Path, conn: &Connection) -> String {
    get_setting(conn, "notesFolder")
        .map(|folder| format!("{}:{}", PIN_KEY, folder))
        .unwrap_or_else(|| format!("{}:{}", PIN_KEY, root.to_string_lossy()))
}

pub fn pin(conn: &Connection, json: bool, root: &Path, input: &str, unpin: bool) -> Result<()> {
    let note = resolve_note(root, input)?;
    let key = pin_key(root, conn);
    let current: Vec<String> = get_setting(conn, &key)
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default();
    let mut next = current.clone();
    if unpin {
        next.retain(|n| n != &note.name);
    } else if !next.contains(&note.name) {
        next.push(note.name.clone());
    }
    let value = serde_json::to_string(&next).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
        params![key, value],
    )
    .map_err(|e| format!("Failed to save pin: {}", e))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({ "success": true, "action": if unpin { "note_unpinned" } else { "note_pinned" }, "name": note.name })).map_err(|e| e.to_string())?);
    } else if unpin {
        println!("Unpinned: {}", note.name);
    } else {
        println!("Pinned: {}", note.name);
    }
    Ok(())
}

pub fn info(root: &Path, json: bool, input: &str) -> Result<()> {
    let note = resolve_note(root, input)?;
    let meta = std::fs::metadata(&note.path).map_err(|e| format!("Failed to stat note: {}", e))?;
    let content = std::fs::read_to_string(&note.path).unwrap_or_default();
    let words = content.split_whitespace().count();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "name": note.name,
                "path": note.path.to_string_lossy(),
                "size_bytes": note.size,
                "created": epoch_ms(&meta),
                "modified": note.modified,
                "word_count": words,
            })).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    let created = epoch_ms(&meta);
    let rows = vec![
        vec!["Name".to_string(), note.name.clone()],
        vec!["Size".to_string(), format_size(note.size)],
        vec!["Created".to_string(), format_ms(created)],
        vec!["Modified".to_string(), format_ms(note.modified)],
        vec!["Words".to_string(), words.to_string()],
        vec!["Location".to_string(), note.rel.clone()],
    ];
    print_table(&["Field", "Value"], &rows);
    Ok(())
}

pub fn search(root: &Path, json: bool, query: &str, folder: Option<&str>) -> Result<()> {
    let mut notes = walk_notes(root)?;
    if let Some(rel) = folder {
        let prefix = rel.trim_matches('/').to_string();
        notes.retain(|n| n.rel.starts_with(&format!("{}/", prefix)));
    }
    let query_lower = query.to_lowercase();
    let mut rows = Vec::new();
    let mut items = Vec::new();
    for note in notes {
        let content = std::fs::read_to_string(&note.path).unwrap_or_default();
        let count = content.to_lowercase().matches(&query_lower).count();
        if count > 0 {
            rows.push(vec![note.name.clone(), note.rel.clone(), count.to_string()]);
            items.push(json!({ "name": note.name, "path": note.rel, "matches": count }));
        }
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({ "query": query, "results": items })).map_err(|e| e.to_string())?);
        return Ok(());
    }
    print_table(&["Name", "Path", "Matches"], &rows);
    Ok(())
}

pub fn today(root: &Path, json: bool) -> Result<()> {
    let name = format!("{}.md", chrono::Local::now().format("%Y-%m-%d"));
    let path = root.join(&name);
    if !path.exists() {
        std::fs::write(&path, "").map_err(|e| format!("Failed to create note: {}", e))?;
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({ "path": path.to_string_lossy() })).map_err(|e| e.to_string())?);
    } else {
        println!("{}", path.display());
    }
    Ok(())
}

// ---- folder actions ----

fn list_folders(root: &Path) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries =
            std::fs::read_dir(&dir).map_err(|e| format!("Failed to read folder: {}", e))?;
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            let hidden = p
                .file_name()
                .map(|n| n.to_string_lossy().starts_with('.'))
                .unwrap_or(false);
            if hidden {
                continue;
            }
            let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().replace('\\', "/");
            out.push(rel.clone());
            stack.push(p);
        }
    }
    out.sort();
    Ok(out)
}

pub fn folder_list(root: &Path, json: bool) -> Result<()> {
    let folders = list_folders(root)?;
    let notes = walk_notes(root)?;
    if json {
        let items: Vec<serde_json::Value> = folders
            .iter()
            .map(|rel| {
                let count = notes.iter().filter(|n| n.rel.starts_with(&format!("{}/", rel))).count();
                json!({ "folder": rel, "notes": count })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json!({ "folders": items })).map_err(|e| e.to_string())?);
        return Ok(());
    }
    let rows: Vec<Vec<String>> = folders
        .iter()
        .map(|rel| {
            let count = notes.iter().filter(|n| n.rel.starts_with(&format!("{}/", rel))).count();
            vec![rel.clone(), count.to_string()]
        })
        .collect();
    print_table(&["Folder", "Notes"], &rows);
    Ok(())
}

pub fn folder_create(root: &Path, json: bool, name: &str, parent: Option<&str>) -> Result<()> {
    let clean = sanitize_name(name)?;
    let target = match parent {
        Some(rel) if !rel.trim_matches('/').is_empty() => root.join(rel.trim_matches('/')).join(&clean),
        _ => root.join(&clean),
    };
    if target.exists() {
        return Err(format!("Folder already exists: {}", target.display()));
    }
    std::fs::create_dir_all(&target).map_err(|e| format!("Failed to create folder: {}", e))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({ "success": true, "action": "folder_created", "path": target.to_string_lossy() })).map_err(|e| e.to_string())?);
    } else {
        println!("Created folder: {}", target.display());
    }
    Ok(())
}

pub fn folder_rename(root: &Path, json: bool, rel: &str, name: &str) -> Result<()> {
    let clean = sanitize_name(name)?;
    let rel = rel.trim_matches('/').to_string();
    let source = root.join(&rel);
    if !source.is_dir() {
        return Err(format!("Folder not found: {}", rel));
    }
    let parent = PathBuf::from(rel.rsplit_once('/').map(|(p, _)| p).unwrap_or(""));
    let target = if rel.contains('/') { root.join(parent).join(&clean) } else { root.join(&clean) };
    if target.exists() {
        return Err(format!("Folder already exists: {}", target.display()));
    }
    // collect the remap before the move: after renaming, walk_notes on the
    // old path would return nothing
    let remap: Vec<(String, String)> = walk_notes(&source)?
        .iter()
        .map(|n| {
            let new_path = target.join(n.path.strip_prefix(&source).unwrap_or(&n.path));
            (n.path.to_string_lossy().into_owned(), new_path.to_string_lossy().into_owned())
        })
        .collect();
    std::fs::rename(&source, &target).map_err(|e| format!("Failed to rename folder: {}", e))?;
    // every note under the old folder changed path: rewrite mention links
    rewrite_mentions(root, &remap)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({ "success": true, "action": "folder_renamed", "path": target.to_string_lossy() })).map_err(|e| e.to_string())?);
    } else {
        println!("Renamed folder: {}", target.display());
    }
    Ok(())
}

pub fn folder_delete(root: &Path, json: bool, rel: &str) -> Result<()> {
    let rel = rel.trim_matches('/');
    let target = root.join(rel);
    if !target.is_dir() {
        return Err(format!("Folder not found: {}", rel));
    }
    // remove_dir only succeeds on empty folders, same rule as the gui
    std::fs::remove_dir(&target).map_err(|_| {
        "Folder is not empty - move or delete its notes first".to_string()
    })?;
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({ "success": true, "action": "folder_deleted" })).map_err(|e| e.to_string())?);
    } else {
        println!("Deleted folder: {}", rel);
    }
    Ok(())
}

use rusqlite::{params, Connection};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::db::*;

// notes live as plain .md files inside the user's notes folder; the gui
// watches this folder, so every cli change shows up in the ui live

struct NoteFile {
    name: String, // file name with .md
    rel: String,  // path relative to the notes root ("test/a.md", root is "")
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
        .map(|c| {
            if c == '/' || c == '\\' || c == ':' {
                '-'
            } else {
                c
            }
        })
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
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => out.push(*byte as char),
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
        .map(|dt| {
            dt.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
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
                    name: p
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
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
            let paths = matches
                .iter()
                .map(|m| m.rel.as_str())
                .collect::<Vec<_>>()
                .join(", ");
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
                content =
                    content.replace(&token, &format!("](note:{})", encode_uri_component(new)));
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

pub fn list(root: &Path, json: bool, folder: Option<&str>, tag: Option<&str>) -> Result<()> {
    let mut notes = walk_notes(root)?;
    if let Some(rel) = folder {
        let prefix = rel.trim_matches('/').to_string();
        notes.retain(|n| {
            n.rel.starts_with(&format!("{}/", prefix)) && !n.rel[prefix.len() + 1..].contains('/')
        });
    }
    if let Some(tag) = tag {
        notes.retain(|n| note_tags(&n.path).iter().any(|t| t == tag));
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
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "notes": items })).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    let rows: Vec<Vec<String>> = notes
        .iter()
        .map(|n| {
            vec![
                n.name.clone(),
                n.rel
                    .rsplit_once('/')
                    .map(|(f, _)| f.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                format_size(n.size),
                format_ms(n.modified),
            ]
        })
        .collect();
    print_table(&["Name", "Folder", "Size", "Modified"], &rows);
    Ok(())
}

pub fn create(
    root: &Path,
    json: bool,
    title: &str,
    folder: Option<&str>,
    content: Option<&str>,
) -> Result<()> {
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
    std::fs::write(&path, content.unwrap_or(""))
        .map_err(|e| format!("Failed to create note: {}", e))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true, "action": "note_created",
                "name": name,
                "path": path.to_string_lossy(),
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Created note: {}", path.display());
    }
    Ok(())
}

// quick capture: an Untitled note in the root with optional content, same
// naming the app's quick capture uses
pub fn quick(root: &Path, json: bool, folder: Option<&str>, content: Option<&str>) -> Result<()> {
    let target_dir = match folder {
        Some(rel) => {
            let dir = root.join(rel.trim_matches('/'));
            std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create folder: {}", e))?;
            dir
        }
        None => root.to_path_buf(),
    };
    let name = first_free_name(&target_dir, "Untitled");
    let path = target_dir.join(&name);
    std::fs::write(&path, content.unwrap_or(""))
        .map_err(|e| format!("Failed to create note: {}", e))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true, "action": "note_created",
                "name": name,
                "path": path.to_string_lossy(),
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Created note: {}", path.display());
    }
    Ok(())
}

pub fn show(root: &Path, json: bool, input: &str) -> Result<()> {
    let note = resolve_note(root, input)?;
    let content =
        std::fs::read_to_string(&note.path).map_err(|e| format!("Failed to read note: {}", e))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({ "name": note.name, "path": note.rel, "content": content })
            )
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("{}", content);
    }
    Ok(())
}

pub fn update(
    root: &Path,
    json: bool,
    input: &str,
    content: Option<&str>,
    stdin: bool,
) -> Result<()> {
    let note = resolve_note(root, input)?;
    let next = if stdin {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("Failed to read stdin: {}", e))?;
        buf
    } else {
        content
            .ok_or("Nothing to write: pass --content or --stdin")?
            .to_string()
    };
    // snapshot the overwritten content, same as the gui's save flow
    snapshot_history(
        root,
        &note.rel,
        &std::fs::read_to_string(&note.path).unwrap_or_default(),
    );
    std::fs::write(&note.path, next).map_err(|e| format!("Failed to write note: {}", e))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({ "success": true, "action": "note_updated", "path": note.rel })
            )
            .map_err(|e| e.to_string())?
        );
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
        return Err(format!(
            "A note with this name already exists: {}",
            target.display()
        ));
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
    std::fs::create_dir_all(archive_dir(root))
        .map_err(|e| format!("Failed to create archive: {}", e))?;
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
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "archived": items }))
                .map_err(|e| e.to_string())?
        );
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
    let name = if input.ends_with(".md") {
        input.to_string()
    } else {
        format!("{}.md", input)
    };
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
    std::fs::create_dir_all(trash_dir(root))
        .map_err(|e| format!("Failed to create trash: {}", e))?;
    let target = trash_dir(root).join(&note.name);
    if target.exists() {
        return Err("A trashed note with this name already exists".to_string());
    }
    std::fs::rename(&note.path, &target).map_err(|e| format!("Failed to delete note: {}", e))?;
    let old_abs = note.path.to_string_lossy().into_owned();
    let new_abs = target.to_string_lossy().into_owned();
    rewrite_mentions(root, &[(old_abs, new_abs)])?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "success": true, "action": "note_deleted" }))
                .map_err(|e| e.to_string())?
        );
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
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "trashed": items })).map_err(|e| e.to_string())?
        );
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
    let file_name = if name.ends_with(".md") {
        name.to_string()
    } else {
        format!("{}.md", name)
    };
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
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &json!({ "success": true, "action": "trash_emptied" })
                )
                .map_err(|e| e.to_string())?
            );
        } else {
            println!("Trash emptied");
        }
        return Ok(());
    }
    let Some(name) = name else {
        return Err("Specify a note name or --all".to_string());
    };
    let file_name = if name.ends_with(".md") {
        name.to_string()
    } else {
        format!("{}.md", name)
    };
    let source = dir.join(&file_name);
    if !source.exists() {
        return Err(format!("Trashed note not found: {}", name));
    }
    std::fs::remove_file(&source).map_err(|e| format!("Failed to purge: {}", e))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "success": true, "action": "note_purged" }))
                .map_err(|e| e.to_string())?
        );
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

// markdown image links in a note, resolved against the vault root; the local
// files (pasted screenshots live in .tack/assets) are what `note info` lists,
// mirroring how task show reports attachments
fn note_images(root: &Path, content: &str) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    let mut cursor = 0usize;
    while let Some(rel) = content[cursor..].find("![") {
        let label_start = cursor + rel + 2;
        let Some(label_rel) = content[label_start..].find("](") else {
            break;
        };
        let label_end = label_start + label_rel;
        let alt = content[label_start..label_end].trim().to_string();
        let url_start = label_end + 2;
        let Some(url_rel) = content[url_start..].find(')') else {
            break;
        };
        let url_end = url_start + url_rel;
        // an optional "title" follows the path: keep only the first token
        let raw = content[url_start..url_end].trim();
        let target = raw
            .trim_matches(|c| c == '"' || c == '\'')
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string();
        cursor = url_end + 1;
        if target.is_empty()
            || target.starts_with("http://")
            || target.starts_with("https://")
            || target.starts_with("data:")
        {
            continue;
        }
        let abs = if target.starts_with('/') {
            PathBuf::from(&target)
        } else {
            root.join(&target)
        };
        out.push((alt, abs));
    }
    out
}

pub fn info(root: &Path, json: bool, input: &str) -> Result<()> {
    let note = resolve_note(root, input)?;
    let meta = std::fs::metadata(&note.path).map_err(|e| format!("Failed to stat note: {}", e))?;
    let content = std::fs::read_to_string(&note.path).unwrap_or_default();
    let words = content.split_whitespace().count();
    let tags = note_tags(&note.path);
    let images = note_images(root, &content);
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
                "tags": tags,
                "images": images.iter().map(|(alt, p)| json!({
                    "alt": alt,
                    "path": p.to_string_lossy(),
                    "size_bytes": std::fs::metadata(p).map(|m| m.len()).unwrap_or(0),
                    "exists": p.is_file(),
                })).collect::<Vec<_>>(),
            }))
            .map_err(|e| e.to_string())?
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
        vec![
            "Tags".to_string(),
            if tags.is_empty() {
                "-".to_string()
            } else {
                tags.join(", ")
            },
        ],
        vec!["Images".to_string(), images.len().to_string()],
        vec!["Location".to_string(), note.rel.clone()],
    ];
    print_table(&["Field", "Value"], &rows);
    if !images.is_empty() {
        println!();
        println!("Images:");
        for (alt, path) in &images {
            let label = if alt.is_empty() { "-" } else { alt.as_str() };
            if path.is_file() {
                let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                println!("  {} ({}) -> {}", label, format_size(size), path.display());
            } else {
                println!("  {} (missing) -> {}", label, path.display());
            }
        }
    }
    Ok(())
}

// notes that contain the note's title as plain text but do not link to it
pub fn unlinked(root: &Path, json: bool, input: &str) -> Result<()> {
    let note = resolve_note(root, input)?;
    let title = note.name.trim_end_matches(".md").to_lowercase();
    if title.is_empty() {
        return Err("Note has an empty title".to_string());
    }
    let linked_abs = format!(
        "(note:{}",
        encode_uri_component(&note.path.to_string_lossy())
    );
    let linked_rel = format!("(note:{}", encode_uri_component(&note.rel));
    let wiki = format!("[[{}", title);
    let mut out: Vec<String> = Vec::new();
    for n in walk_notes(root)? {
        if n.path == note.path {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&n.path) else {
            continue;
        };
        let lower = content.to_lowercase();
        if lower.contains(&title)
            && !content.contains(&linked_abs)
            && !content.contains(&linked_rel)
            && !content.contains(&wiki)
        {
            out.push(n.rel);
        }
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "note": note.rel, "unlinked": out }))
                .map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if out.is_empty() {
        println!("No unlinked mentions of {}", note.rel);
        return Ok(());
    }
    let rows: Vec<Vec<String>> = out.iter().map(|r| vec![r.clone()]).collect();
    print_table(&["Note"], &rows);
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
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "query": query, "results": items }))
                .map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    print_table(&["Name", "Path", "Matches"], &rows);
    Ok(())
}

pub fn today(root: &Path, json: bool, offset: Option<i64>) -> Result<()> {
    let date = chrono::Local::now().date_naive() + chrono::Duration::days(offset.unwrap_or(0));
    let name = format!("{}.md", date.format("%Y-%m-%d"));
    let path = root.join(&name);
    if !path.exists() {
        std::fs::write(&path, "").map_err(|e| format!("Failed to create note: {}", e))?;
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "path": path.to_string_lossy() }))
                .map_err(|e| e.to_string())?
        );
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
            let rel = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
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
                let count = notes
                    .iter()
                    .filter(|n| n.rel.starts_with(&format!("{}/", rel)))
                    .count();
                json!({ "folder": rel, "notes": count })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "folders": items })).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    let rows: Vec<Vec<String>> = folders
        .iter()
        .map(|rel| {
            let count = notes
                .iter()
                .filter(|n| n.rel.starts_with(&format!("{}/", rel)))
                .count();
            vec![rel.clone(), count.to_string()]
        })
        .collect();
    print_table(&["Folder", "Notes"], &rows);
    Ok(())
}

pub fn folder_create(root: &Path, json: bool, name: &str, parent: Option<&str>) -> Result<()> {
    let clean = sanitize_name(name)?;
    let target = match parent {
        Some(rel) if !rel.trim_matches('/').is_empty() => {
            root.join(rel.trim_matches('/')).join(&clean)
        }
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
    let target = if rel.contains('/') {
        root.join(parent).join(&clean)
    } else {
        root.join(&clean)
    };
    if target.exists() {
        return Err(format!("Folder already exists: {}", target.display()));
    }
    // collect the remap before the move: after renaming, walk_notes on the
    // old path would return nothing
    let remap: Vec<(String, String)> = walk_notes(&source)?
        .iter()
        .map(|n| {
            let new_path = target.join(n.path.strip_prefix(&source).unwrap_or(&n.path));
            (
                n.path.to_string_lossy().into_owned(),
                new_path.to_string_lossy().into_owned(),
            )
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
    std::fs::remove_dir(&target)
        .map_err(|_| "Folder is not empty - move or delete its notes first".to_string())?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "success": true, "action": "folder_deleted" }))
                .map_err(|e| e.to_string())?
        );
    } else {
        println!("Deleted folder: {}", rel);
    }
    Ok(())
}

// ---- tags (frontmatter) ----

// split a leading `---` frontmatter block from the body; matches the gui's
// minimal yaml parser (tags are the only key we rewrite)
fn split_frontmatter(text: &str) -> (Option<&str>, &str) {
    let rest = match text.strip_prefix("---\n") {
        Some(r) => r,
        None => return (None, text),
    };
    match rest.find("\n---") {
        Some(idx) => {
            let after = &rest[idx + 4..];
            let after = after
                .strip_prefix("\r\n")
                .or_else(|| after.strip_prefix('\n'))
                .unwrap_or(after);
            (Some(&rest[..idx]), after)
        }
        None => (None, text),
    }
}

// frontmatter + inline #tags, same mix as the gui's noteTags
pub fn note_tags(path: &Path) -> Vec<String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    let (fm, body) = split_frontmatter(&text);
    let mut tags = Vec::new();
    if let Some(fm) = fm {
        let mut in_tags = false;
        for line in fm.lines() {
            if line.starts_with("tags:") {
                in_tags = true;
                let inline = line.trim_start_matches("tags:").trim();
                if inline.starts_with('[') && inline.ends_with(']') {
                    for item in inline[1..inline.len() - 1].split(',') {
                        let t = item.trim().trim_matches('"').trim_matches('\'');
                        if !t.is_empty() && !tags.iter().any(|x| x == t) {
                            tags.push(t.to_string());
                        }
                    }
                } else if !inline.is_empty() {
                    for item in inline.split(',') {
                        let t = item.trim().trim_matches('"').trim_matches('\'');
                        if !t.is_empty() && !tags.iter().any(|x| x == t) {
                            tags.push(t.to_string());
                        }
                    }
                }
                continue;
            }
            if in_tags && line.trim_start().starts_with("- ") {
                let t = line.trim().trim_start_matches("- ").trim();
                if !t.is_empty() && !tags.iter().any(|x| x == t) {
                    tags.push(t.to_string());
                }
                continue;
            }
            in_tags = false;
        }
    }
    for line in body.lines() {
        if line.trim_start().starts_with('#') {
            continue; // headings are not tags
        }
        for part in line.split_whitespace() {
            let t = part.strip_prefix('#').unwrap_or(part);
            if t != part && !t.is_empty() && !tags.iter().any(|x| x == t) {
                tags.push(t.to_string());
            }
        }
    }
    tags
}

// rewrite the frontmatter keeping every other key, replacing only tags
fn set_tags(text: &str, tags: &[String]) -> String {
    let (fm, body) = split_frontmatter(text);
    match fm {
        None => {
            if tags.is_empty() {
                return text.to_string();
            }
            let mut out = String::from("---\ntags:\n");
            for t in tags {
                out.push_str(&format!("  - {}\n", t));
            }
            out.push_str("---\n\n");
            out.push_str(body);
            out
        }
        Some(fm) => {
            let mut kept: Vec<&str> = Vec::new();
            let mut skip_list = false;
            for line in fm.lines() {
                if line.starts_with("tags:") {
                    skip_list = true;
                    continue;
                }
                if skip_list && line.trim_start().starts_with("- ") {
                    continue;
                }
                skip_list = false;
                kept.push(line);
            }
            if kept.is_empty() && tags.is_empty() {
                return body.to_string();
            }
            let mut out = String::from("---\n");
            for l in &kept {
                out.push_str(l);
                out.push('\n');
            }
            if !tags.is_empty() {
                out.push_str("tags:\n");
                for t in tags {
                    out.push_str(&format!("  - {}\n", t));
                }
            }
            out.push_str("---\n\n");
            out.push_str(body);
            out
        }
    }
}

pub fn tag_add(root: &Path, json: bool, input: &str, tag: &str) -> Result<()> {
    let note = resolve_note(root, input)?;
    let text =
        std::fs::read_to_string(&note.path).map_err(|e| format!("Failed to read note: {}", e))?;
    let mut tags = note_tags(&note.path);
    if !tags.iter().any(|t| t == tag) {
        tags.push(tag.to_string());
    }
    std::fs::write(&note.path, set_tags(&text, &tags))
        .map_err(|e| format!("Failed to write note: {}", e))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({ "success": true, "action": "tag_added", "path": note.rel, "tags": tags })
            )
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Added tag '{}' to {}", tag, note.rel);
    }
    Ok(())
}

pub fn tag_remove(root: &Path, json: bool, input: &str, tag: &str) -> Result<()> {
    let note = resolve_note(root, input)?;
    let text =
        std::fs::read_to_string(&note.path).map_err(|e| format!("Failed to read note: {}", e))?;
    let tags: Vec<String> = note_tags(&note.path)
        .into_iter()
        .filter(|t| t != tag)
        .collect();
    std::fs::write(&note.path, set_tags(&text, &tags))
        .map_err(|e| format!("Failed to write note: {}", e))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({ "success": true, "action": "tag_removed", "path": note.rel, "tags": tags })
            )
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Removed tag '{}' from {}", tag, note.rel);
    }
    Ok(())
}

// all tags across the vault with per-tag note counts
pub fn tag_list(root: &Path, json: bool) -> Result<()> {
    let notes = walk_notes(root)?;
    let mut counts: Vec<(String, usize)> = Vec::new();
    for n in &notes {
        for t in note_tags(&n.path) {
            match counts.iter_mut().find(|(name, _)| *name == t) {
                Some((_, c)) => *c += 1,
                None => counts.push((t, 1)),
            }
        }
    }
    counts.sort();
    if json {
        let items: Vec<serde_json::Value> = counts
            .iter()
            .map(|(t, c)| json!({ "tag": t, "notes": c }))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "tags": items })).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    let rows: Vec<Vec<String>> = counts
        .iter()
        .map(|(t, c)| vec![t.clone(), c.to_string()])
        .collect();
    print_table(&["Tag", "Notes"], &rows);
    Ok(())
}

// ---- version history (.tack/history/<rel with / as %>/<ts>.md) ----

fn history_root(root: &Path) -> PathBuf {
    root.join(".tack").join("history")
}

// write a timestamped snapshot and keep the newest 30, matching the gui
fn snapshot_history(root: &Path, rel: &str, content: &str) {
    if content.is_empty() {
        return;
    }
    let dir = history_root(root).join(history_key(rel));
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let now = chrono::Utc::now().timestamp_millis();
    if std::fs::write(dir.join(format!("{}.md", now)), content).is_err() {
        return;
    }
    let mut versions: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "md").unwrap_or(false))
            .collect(),
        Err(_) => return,
    };
    versions.sort();
    while versions.len() > 30 {
        let oldest = versions.remove(0);
        let _ = std::fs::remove_file(oldest);
    }
}

fn history_key(rel: &str) -> String {
    rel.replace('\\', "%").replace('/', "%")
}

// list the saved snapshots of a note, newest first
pub fn history(root: &Path, json: bool, input: &str) -> Result<()> {
    let note = resolve_note(root, input)?;
    let dir = history_root(root).join(history_key(&note.rel));
    let mut out: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "md").unwrap_or(false) {
                out.push(
                    p.file_stem()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                );
            }
        }
    }
    out.sort();
    out.reverse();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "note": note.rel, "versions": out }))
                .map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if out.is_empty() {
        println!("No saved versions for {}", note.rel);
        return Ok(());
    }
    let rows: Vec<Vec<String>> = out.iter().map(|ts| vec![ts.clone()]).collect();
    print_table(&["Version (unix ms)"], &rows);
    Ok(())
}

// restore a snapshot by timestamp; the current content is snapshotted first,
// matching the gui's restore flow
pub fn restore_version(root: &Path, json: bool, input: &str, at: &str) -> Result<()> {
    let note = resolve_note(root, input)?;
    let snapshot = history_root(root)
        .join(history_key(&note.rel))
        .join(format!("{}.md", at));
    if !snapshot.exists() {
        return Err(format!("Version not found: {}", at));
    }
    let current =
        std::fs::read_to_string(&note.path).map_err(|e| format!("Failed to read note: {}", e))?;
    let restored = std::fs::read_to_string(&snapshot)
        .map_err(|e| format!("Failed to read snapshot: {}", e))?;
    if current != restored {
        // keep the overwritten content as its own snapshot
        let key = history_key(&note.rel);
        let dir = history_root(root).join(&key);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create history dir: {}", e))?;
        let now = chrono::Utc::now().timestamp_millis();
        std::fs::write(dir.join(format!("{}.md", now)), &current)
            .map_err(|e| format!("Failed to write snapshot: {}", e))?;
    }
    std::fs::write(&note.path, restored).map_err(|e| format!("Failed to write note: {}", e))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({ "success": true, "action": "version_restored", "path": note.rel })
            )
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Restored {} to version {}", note.rel, at);
    }
    Ok(())
}

// ---- templates (.tack/templates) ----

fn templates_dir(root: &Path) -> PathBuf {
    root.join(".tack").join("templates")
}

pub fn template_list(root: &Path, json: bool) -> Result<()> {
    let dir = templates_dir(root);
    let mut out: Vec<NoteFile> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "md").unwrap_or(false) {
                let meta = entry.metadata().ok();
                out.push(NoteFile {
                    name: p
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    rel: p
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    path: p.clone(),
                    modified: meta.as_ref().map(epoch_ms).unwrap_or(0),
                    size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    if json {
        let items: Vec<serde_json::Value> = out
            .iter()
            .map(|t| json!({ "name": t.name, "size_bytes": t.size }))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "templates": items }))
                .map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if out.is_empty() {
        println!("No templates yet");
        return Ok(());
    }
    let rows: Vec<Vec<String>> = out
        .iter()
        .map(|t| vec![t.name.clone(), format_size(t.size)])
        .collect();
    print_table(&["Template", "Size"], &rows);
    Ok(())
}

// create a template; {{title}} {{date}} {{time}} {{yesterday}} {{tomorrow}}
// are filled in when a note is created from it
pub fn template_create(root: &Path, json: bool, title: &str, content: Option<&str>) -> Result<()> {
    let stem = sanitize_name(title)?;
    let name = first_free_name(&templates_dir(root), &stem);
    let dir = templates_dir(root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create templates dir: {}", e))?;
    let path = dir.join(&name);
    std::fs::write(&path, content.unwrap_or(""))
        .map_err(|e| format!("Failed to create template: {}", e))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({ "success": true, "action": "template_created", "name": name })
            )
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Created template: {}", path.display());
    }
    Ok(())
}

// create a note from a template, filling the variables like the gui
pub fn create_from_template(
    root: &Path,
    json: bool,
    title: &str,
    template: &str,
    folder: Option<&str>,
) -> Result<()> {
    let tpl_name = if template.ends_with(".md") {
        template.to_string()
    } else {
        format!("{}.md", template)
    };
    let tpl_path = templates_dir(root).join(&tpl_name);
    let template_body = std::fs::read_to_string(&tpl_path)
        .map_err(|_| format!("Template not found: {}", tpl_name))?;

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
    let title_stem = name.trim_end_matches(".md").to_string();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let shift = |days: i64| -> String {
        (chrono::Local::now().date_naive() + chrono::Duration::days(days))
            .format("%Y-%m-%d")
            .to_string()
    };
    let body = template_body
        .replace("{{title}}", &title_stem)
        .replace("{{date}}", &today)
        .replace(
            "{{time}}",
            &chrono::Local::now().format("%H:%M").to_string(),
        )
        .replace("{{yesterday}}", &shift(-1))
        .replace("{{tomorrow}}", &shift(1));
    std::fs::write(&path, body).map_err(|e| format!("Failed to create note: {}", e))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "success": true, "action": "note_created", "name": name, "path": path.to_string_lossy() }))
                .map_err(|e| e.to_string())?
        );
    } else {
        println!("Created note: {}", path.display());
    }
    Ok(())
}

// ---- export ----

pub fn export(root: &Path, json: bool, input: &str, format: &str, out: Option<&str>) -> Result<()> {
    let note = resolve_note(root, input)?;
    let content =
        std::fs::read_to_string(&note.path).map_err(|e| format!("Failed to read note: {}", e))?;
    let target = match out {
        Some(p) => PathBuf::from(p),
        None => {
            let stem = note.name.trim_end_matches(".md");
            match format {
                "html" => note.path.with_file_name(format!("{}.html", stem)),
                _ => note.path.with_file_name(format!("{}.md", stem)),
            }
        }
    };
    match format {
        "html" => {
            let title = note.name.trim_end_matches(".md").to_string();
            let html = format!(
                "<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n<style>\nbody {{ font-family: -apple-system, 'Segoe UI', sans-serif; max-width: 720px; margin: 2rem auto; padding: 0 1rem; color: #1f2328; line-height: 1.6; }}\npre {{ background: #f6f8fa; padding: 0.75rem; border-radius: 6px; overflow-x: auto; }}\ncode {{ background: #f6f8fa; padding: 0.1rem 0.3rem; border-radius: 4px; }}\nblockquote {{ border-left: 3px solid #d0d7de; margin: 0.5rem 0; padding: 0.25rem 0.9rem; color: #59636e; }}\ntable {{ border-collapse: collapse; }}\nth, td {{ border: 1px solid #d0d7de; padding: 0.35rem 0.7rem; }}\nimg {{ max-width: 100%; }}\n</style>\n</head>\n<body>\n<h1>{}</h1>\n<pre>{}</pre>\n</body>\n</html>\n",
                title,
                title,
                html_escape(&content)
            );
            std::fs::write(&target, html).map_err(|e| format!("Failed to write export: {}", e))?;
        }
        _ => {
            std::fs::copy(&note.path, &target)
                .map_err(|e| format!("Failed to write export: {}", e))?;
        }
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "success": true, "action": "note_exported", "path": target.to_string_lossy() }))
                .map_err(|e| e.to_string())?
        );
    } else {
        println!("Exported: {}", target.display());
    }
    Ok(())
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// ---- attachments ----

// copy a file into the vault's .tack/assets and append an image reference
pub fn attach(root: &Path, json: bool, input: &str, file: &str) -> Result<()> {
    let note = resolve_note(root, input)?;
    let source = PathBuf::from(file);
    if !source.is_file() {
        return Err(format!("File not found: {}", file));
    }
    let file_name = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let assets = root.join(".tack").join("assets");
    std::fs::create_dir_all(&assets).map_err(|e| format!("Failed to create assets dir: {}", e))?;
    let dot = file_name.rfind('.').unwrap_or(file_name.len());
    let (stem, ext) = file_name.split_at(dot);
    let mut rel = format!(".tack/assets/{}", file_name);
    let mut candidate = assets.join(&file_name);
    let mut i = 2;
    while candidate.exists() {
        rel = format!(".tack/assets/{}-{}{}", stem, i, ext);
        candidate = root.join(&rel);
        i += 1;
    }
    std::fs::copy(&source, &candidate).map_err(|e| format!("Failed to copy file: {}", e))?;
    let text =
        std::fs::read_to_string(&note.path).map_err(|e| format!("Failed to read note: {}", e))?;
    let next = format!("{}\n![{}]({})\n", text.trim_end(), file_name, rel);
    std::fs::write(&note.path, next).map_err(|e| format!("Failed to write note: {}", e))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({ "success": true, "action": "note_attached", "asset": rel })
            )
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Attached: {}", rel);
    }
    Ok(())
}

// ---- backlinks ----

// notes that link to the given note via @mentions or [[wiki]] links
pub fn backlinks(root: &Path, json: bool, input: &str) -> Result<()> {
    let note = resolve_note(root, input)?;
    let abs = note.path.to_string_lossy().to_string();
    let title = note.name.trim_end_matches(".md");
    let mut out: Vec<String> = Vec::new();
    for n in walk_notes(root)? {
        if n.path == note.path {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&n.path) else {
            continue;
        };
        let mentions = content.contains(&format!("(note:{}", encode_uri_component(&abs)))
            || content.contains(&format!("(note:{}", encode_uri_component(&note.rel)))
            || content.contains(&format!("[[{}]]", title))
            || content.contains(&format!("[[{}|", title));
        if mentions {
            out.push(n.rel);
        }
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "note": note.rel, "backlinks": out }))
                .map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if out.is_empty() {
        println!("No backlinks to {}", note.rel);
        return Ok(());
    }
    let rows: Vec<Vec<String>> = out.iter().map(|r| vec![r.clone()]).collect();
    print_table(&["Note"], &rows);
    Ok(())
}

// ---- note → task ----

// turn a note into a task: title becomes the task title, body the description
pub fn to_task(conn: &Connection, root: &Path, json: bool, input: &str) -> Result<()> {
    let note = resolve_note(root, input)?;
    let content =
        std::fs::read_to_string(&note.path).map_err(|e| format!("Failed to read note: {}", e))?;
    let title = note.name.trim_end_matches(".md").to_string();
    let id = crate::commands::task::create_quiet(conn, &title, &content)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "success": true, "action": "task_created", "task_id": id, "note": note.rel }))
                .map_err(|e| e.to_string())?
        );
    } else {
        println!("Created task {} from {}", id, note.rel);
    }
    Ok(())
}

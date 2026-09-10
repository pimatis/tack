//! notes endpoints for the live web client; every path is validated to stay
//! inside the notes folder configured in the app settings

use super::http::{json_response, percent_decode};
use super::Ctx;
use crate::notes::{
    create_folder, delete_folder, list_note_folders, list_notes, note_info, read_notes_deep,
    rename_note, save_note_with_history, write_binary_file, NoteFull,
};
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::json;
use std::io::Read;
use std::path::{Path, PathBuf};
use tiny_http::{Header, Request, Response, StatusCode};

type BytesResponse = Response<std::io::Cursor<Vec<u8>>>;

fn notes_root(ctx: &Ctx) -> Result<PathBuf, String> {
    let conn = Connection::open(&ctx.db_path).map_err(|e| e.to_string())?;
    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'notesFolder'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let root = stored.ok_or_else(|| "No notes folder configured".to_string())?;
    PathBuf::from(&root)
        .canonicalize()
        .map_err(|e| format!("Notes folder unavailable: {}", e))
}

// canonicalize targets (parent for missing files) so symlinks and .. can't
// escape the notes folder
fn validate(root: &Path, target: &Path) -> Result<PathBuf, String> {
    if target.exists() {
        let canonical = target.canonicalize().map_err(|e| e.to_string())?;
        if canonical.starts_with(root) {
            return Ok(canonical);
        }
        return Err("Path outside notes folder".into());
    }
    let parent = target
        .parent()
        .ok_or("Path outside notes folder")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !parent.starts_with(root) {
        return Err("Path outside notes folder".into());
    }
    let name = target.file_name().ok_or("Path outside notes folder")?;
    Ok(parent.join(name))
}

// validate every supplied path against the notes root
fn validate_all(ctx: &Ctx, raw_paths: &[Option<&String>]) -> Result<Vec<PathBuf>, String> {
    let root = notes_root(ctx)?;
    raw_paths
        .iter()
        .map(|p| {
            let raw = p.ok_or("Missing path")?;
            validate(&root, Path::new(raw))
        })
        .collect()
}

fn ok<T: serde::Serialize>(value: T) -> BytesResponse {
    json_response(StatusCode(200), value)
}

fn err(message: String) -> BytesResponse {
    json_response(StatusCode(500), json!({ "error": message }))
}

fn query_param(query: &str, key: &str) -> Option<String> {
    let prefix = format!("{}=", key);
    query.split('&').find_map(|pair| {
        pair.strip_prefix(&prefix)
            .map(|v| percent_decode(&v.replace('+', " ")))
    })
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct NotesPost {
    #[serde(default)]
    dir: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    to: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    keep: Option<u32>,
    #[serde(default)]
    bytes: Option<Vec<u8>>,
}

pub(super) fn serve_deep(ctx: &Ctx) -> BytesResponse {
    match notes_root(ctx).and_then(|root| read_notes_deep(root.to_string_lossy().into_owned())) {
        Ok(notes) => ok(notes),
        Err(e) => err(e),
    }
}

pub(super) fn serve_root(ctx: &Ctx) -> BytesResponse {
    match notes_root(ctx) {
        Ok(root) => ok(json!({ "root": root.to_string_lossy() })),
        Err(e) => err(e),
    }
}

pub(super) fn serve_folders(ctx: &Ctx, query: &str) -> BytesResponse {
    let Some(dir) = query_param(query, "dir") else {
        return json_response(StatusCode(400), json!({ "error": "Missing dir" }));
    };
    match validate_all(ctx, &[Some(&dir)])
        .and_then(|mut p| list_note_folders(p.remove(0).to_string_lossy().into_owned()))
    {
        Ok(folders) => ok(folders),
        Err(e) => err(e),
    }
}

pub(super) fn serve_list(ctx: &Ctx, query: &str) -> BytesResponse {
    let Some(dir) = query_param(query, "dir") else {
        return json_response(StatusCode(400), json!({ "error": "Missing dir" }));
    };
    match validate_all(ctx, &[Some(&dir)])
        .and_then(|mut p| list_notes(p.remove(0).to_string_lossy().into_owned()))
    {
        Ok(notes) => ok(notes),
        Err(e) => err(e),
    }
}

pub(super) fn serve_file(ctx: &Ctx, query: &str) -> BytesResponse {
    let Some(path) = query_param(query, "path") else {
        return json_response(StatusCode(400), json!({ "error": "Missing path" }));
    };
    let read = || -> Result<NoteFull, String> {
        let p = validate_all(ctx, &[Some(&path)])?.remove(0);
        let meta = std::fs::metadata(&p).map_err(|e| e.to_string())?;
        Ok(NoteFull {
            path: p.to_string_lossy().into_owned(),
            name: p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            content: std::fs::read_to_string(&p).map_err(|e| e.to_string())?,
            modified: meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            size: meta.len(),
        })
    };
    match read() {
        Ok(note) => ok(note),
        Err(e) => err(e),
    }
}

pub(super) fn serve_info(ctx: &Ctx, query: &str) -> BytesResponse {
    let Some(path) = query_param(query, "path") else {
        return json_response(StatusCode(400), json!({ "error": "Missing path" }));
    };
    match validate_all(ctx, &[Some(&path)])
        .and_then(|mut p| note_info(p.remove(0).to_string_lossy().into_owned()))
    {
        Ok(details) => ok(details),
        Err(e) => err(e),
    }
}

pub(super) fn serve_asset(ctx: &Ctx, query: &str) -> BytesResponse {
    let Some(path) = query_param(query, "path") else {
        return json_response(StatusCode(400), json!({ "error": "Missing path" }));
    };
    let p = match validate_all(ctx, &[Some(&path)]) {
        Ok(mut p) => p.remove(0),
        Err(e) => return err(e),
    };
    match std::fs::read(&p) {
        Ok(data) => {
            let headers = vec![
                Header::from_bytes(b"Content-Type", mime_for(&p).as_bytes()).unwrap(),
                Header::from_bytes(b"Cache-Control", b"no-cache").unwrap(),
                Header::from_bytes(b"Connection", b"close").unwrap(),
            ];
            let len = data.len();
            Response::new(
                StatusCode(200),
                headers,
                std::io::Cursor::new(data),
                Some(len),
                None,
            )
        }
        Err(_) => json_response(StatusCode(404), json!({ "error": "Asset not found" })),
    }
}

fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("pdf") => "application/pdf",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("mp4") => "video/mp4",
        Some("mov") => "video/quicktime",
        _ => "application/octet-stream",
    }
}

pub(super) fn post_notes(request: &mut Request, path: &str, ctx: &Ctx) -> BytesResponse {
    let mut body = Vec::new();
    if let Err(e) = request
        .as_reader()
        .take(64 * 1024 * 1024)
        .read_to_end(&mut body)
    {
        return json_response(
            StatusCode(400),
            json!({ "error": format!("Failed to read request: {}", e) }),
        );
    }
    let payload: NotesPost = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            return json_response(
                StatusCode(400),
                json!({ "error": format!("Invalid request body: {}", e) }),
            )
        }
    };

    let result: Result<serde_json::Value, String> = match path {
        "/api/notes/write" => validated(&ctx, &[payload.path.as_ref()], |p| {
            std::fs::write(&p[0], payload.content.unwrap_or_default())
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        }),
        "/api/notes/save" => validated(&ctx, &[payload.path.as_ref()], move |mut p| {
            let target = p.remove(0);
            let content = payload.content.clone().unwrap_or_default();
            // the history root always lives under the notes root; never trust
            // the client-supplied path
            let root = notes_root(ctx)?;
            let history_root = root.join(".tack/history").to_string_lossy().into_owned();
            let key = payload.key.clone().unwrap_or_else(|| {
                target
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
            save_note_with_history(
                target.to_string_lossy().into_owned(),
                content,
                history_root,
                key,
                payload.keep.unwrap_or(50),
            )?;
            Ok(json!({ "ok": true }))
        }),
        "/api/notes/rename" => validated(
            &ctx,
            &[payload.from.as_ref(), payload.to.as_ref()],
            |paths| {
                rename_note(
                    paths[0].to_string_lossy().into_owned(),
                    paths[1].to_string_lossy().into_owned(),
                )?;
                Ok(json!({ "ok": true }))
            },
        ),
        "/api/notes/delete" => validated(&ctx, &[payload.path.as_ref()], |p| {
            std::fs::remove_file(&p[0]).map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        }),
        "/api/notes/folder" => validated(&ctx, &[payload.dir.as_ref()], |mut dirs| {
            let name = payload.name.clone().ok_or("Missing name")?;
            create_folder(dirs.remove(0).to_string_lossy().into_owned(), name)?;
            Ok(json!({ "ok": true }))
        }),
        "/api/notes/folder-delete" => validated(&ctx, &[payload.path.as_ref()], |mut p| {
            delete_folder(p.remove(0).to_string_lossy().into_owned())?;
            Ok(json!({ "ok": true }))
        }),
        "/api/notes/binary" => validated(&ctx, &[payload.path.as_ref()], |mut p| {
            let bytes = payload.bytes.clone().ok_or("Missing bytes")?;
            write_binary_file(p.remove(0).to_string_lossy().into_owned(), bytes)?;
            Ok(json!({ "ok": true }))
        }),
        _ => return json_response(StatusCode(404), json!({ "error": "Not found" })),
    };

    match result {
        Ok(value) => ok(value),
        Err(e) => err(e),
    }
}

fn validated(
    ctx: &Ctx,
    raw_paths: &[Option<&String>],
    f: impl FnOnce(Vec<PathBuf>) -> Result<serde_json::Value, String>,
) -> Result<serde_json::Value, String> {
    f(validate_all(ctx, raw_paths)?)
}

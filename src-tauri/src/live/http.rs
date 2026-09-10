use super::attachments::{delete_attachment, put_attachment, serve_attachment};
use super::auth::{authorized, handle_auth, load_auth};
use super::backups::{create_backup_http, delete_backup_http, restore_backup_http, serve_backups};
use super::events::{poll_events, stream_events};
use super::notes;
use super::query::run_query;
use super::Ctx;
use serde::Serialize;
use serde_json::json;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tiny_http::{Header, Method, Request, Response, StatusCode};

pub(super) fn json_response<T: Serialize>(
    status: StatusCode,
    body: T,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let data = serde_json::to_vec(&body).unwrap_or_default();
    let headers = vec![
        Header::from_bytes(b"Content-Type", b"application/json; charset=utf-8").unwrap(),
        Header::from_bytes(b"Cache-Control", b"no-cache").unwrap(),
        Header::from_bytes(b"Connection", b"close").unwrap(),
    ];
    let len = data.len();
    Response::new(status, headers, std::io::Cursor::new(data), Some(len), None)
}

pub(super) fn handle_request(request: Request, ctx: &Ctx) {
    let raw_url = request.url().to_string();
    let (path, query) = raw_url.split_once('?').unwrap_or((raw_url.as_str(), ""));
    let method = request.method().clone();

    // only api routes consult the shared-password config; static files carry
    // no user data and stay open so the login screen can load
    let auth = if path.starts_with("/api/") {
        load_auth(&ctx.db_path)
    } else {
        None
    };

    // login endpoint: the only api route reachable without a session token
    if method == Method::Post && path == "/api/auth" {
        let mut request = request;
        let response = handle_auth(&mut request, ctx);
        let _ = request.respond(response);
        return;
    }

    // api gate: when a shared password is set, every data endpoint (sse
    // included) requires the session token
    if let Some(a) = &auth {
        if !authorized(&request, query, a) {
            let _ = request.respond(json_response(
                StatusCode(401),
                json!({ "error": "Unauthorized" }),
            ));
            return;
        }
    }

    // sse bypasses respond(): tiny_http only flushes a response body once it
    // finishes, so write frames straight to the connection writer instead
    if method == Method::Get && path == "/api/events/stream" {
        let closed = Arc::new(AtomicBool::new(false));
        ctx.sse.lock().unwrap().push(closed.clone());
        let mut writer = request.into_writer();
        stream_events(&mut writer, ctx, &closed);
        ctx.sse.lock().unwrap().retain(|f| !Arc::ptr_eq(f, &closed));
        return;
    }

    let mut request = request;
    let response = match (method, path) {
        (Method::Post, "/api/select") => run_query(&mut request, ctx, true),
        (Method::Post, "/api/execute") => run_query(&mut request, ctx, false),
        (Method::Get, "/api/events") => poll_events(ctx),
        (Method::Get, p) if p.starts_with("/api/attachment/") => serve_attachment(p, query, ctx),
        (Method::Put, p) if p.starts_with("/api/attachment/") => {
            put_attachment(&mut request, p, ctx)
        }
        (Method::Delete, p) if p.starts_with("/api/attachment/") => delete_attachment(p, ctx),
        (Method::Get, "/api/backups") => serve_backups(ctx),
        (Method::Get, "/api/notes/deep") => notes::serve_deep(ctx),
        (Method::Get, "/api/notes/root") => notes::serve_root(ctx),
        (Method::Get, "/api/notes/folders") => notes::serve_folders(ctx, query),
        (Method::Get, "/api/notes/list") => notes::serve_list(ctx, query),
        (Method::Get, "/api/notes/file") => notes::serve_file(ctx, query),
        (Method::Get, "/api/notes/info") => notes::serve_info(ctx, query),
        (Method::Get, "/api/notes/asset") => notes::serve_asset(ctx, query),
        (Method::Post, p) if p.starts_with("/api/notes/") => {
            notes::post_notes(&mut request, p, ctx)
        }
        (Method::Post, "/api/backups") => create_backup_http(&mut request, ctx),
        (Method::Post, p) if p.starts_with("/api/backups/") && p.ends_with("/restore") => {
            restore_backup_http(p, ctx)
        }
        (Method::Delete, p) if p.starts_with("/api/backups/") => delete_backup_http(p, ctx),
        (Method::Get, _) | (Method::Head, _) => serve_static(path, ctx),
        _ => json_response(StatusCode(404), json!({ "error": "Not found" })),
    };
    let _ = request.respond(response);
}

fn serve_static(path: &str, ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    let decoded = percent_decode(path.trim_start_matches('/'));
    if decoded.split('/').any(|seg| seg == ".." || seg == ".") {
        return json_response(StatusCode(400), json!({ "error": "Invalid path" }));
    }
    let mut file = if decoded.is_empty() {
        ctx.frontend.join("index.html")
    } else {
        ctx.frontend.join(&decoded)
    };
    if file.is_dir() {
        file = file.join("index.html");
    }
    if !file.exists() {
        // spa fallback: let the client router render the route
        file = ctx.frontend.join("index.html");
    }
    match std::fs::read(&file) {
        Ok(data) => file_response(&file, data, path),
        Err(_) => json_response(StatusCode(404), json!({ "error": "Not found" })),
    }
}

fn file_response(
    path: &Path,
    data: Vec<u8>,
    request_path: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let cache = if request_path.starts_with("/_app/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    let headers = vec![
        Header::from_bytes(b"Content-Type", mime_for(path).as_bytes()).unwrap(),
        Header::from_bytes(b"Cache-Control", cache.as_bytes()).unwrap(),
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

fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") | Some("webmanifest") | Some("map") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("txt") => "text/plain; charset=utf-8",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

pub(super) fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

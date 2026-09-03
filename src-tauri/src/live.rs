use crate::{app_db_path, attachments_dir, backup, base64_encode};
use rusqlite::{params_from_iter, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Manager;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

pub type Result<T> = std::result::Result<T, String>;

// change signaling for browser clients: a generation counter + condvar.
// the live server long-polls on it, so every http response completes
// (tiny_http only flushes its writer once a response body finishes); the
// sse stream waits on the same condvar but writes frames straight to the
// connection writer (request.into_writer), flushing after each one
#[derive(Default)]
pub struct LiveHub {
    state: Mutex<LiveHubState>,
    cv: std::sync::Condvar,
}

#[derive(Default)]
struct LiveHubState {
    generation: u64,
}

impl LiveHub {
    pub fn notify(&self) {
        let mut state = self.state.lock().unwrap();
        state.generation = state.generation.wrapping_add(1);
        self.cv.notify_all();
    }

    pub fn generation(&self) -> u64 {
        self.state.lock().unwrap().generation
    }

    // true when a change happened since `last`; waits up to `timeout` for one
    pub fn changed_since(&self, last: u64, timeout: Duration) -> bool {
        let state = self.state.lock().unwrap();
        if state.generation != last {
            return true;
        }
        let (guard, _) = self.cv.wait_timeout(state, timeout).unwrap();
        guard.generation != last
    }
}

#[derive(Clone, Serialize)]
pub struct LiveStatus {
    pub port: u16,
    pub url: String,
}

pub struct LiveServer {
    pub port: u16,
    server: Arc<Server>,
    // accept loop handle, joined on drop so the listener socket is
    // guaranteed closed (port freed) once the server is dropped
    accept: Option<std::thread::JoinHandle<()>>,
    // close flags for in-flight sse streams; set on drop so live_stop ends
    // them instead of leaving them attached to the app-lifetime hub
    sse: Arc<Mutex<Vec<Arc<AtomicBool>>>>,
}

impl Drop for LiveServer {
    fn drop(&mut self) {
        // unblock the accept loop and wait for it to exit; the last Arc
        // reference then drops the Server, which closes the listener
        self.server.unblock();
        if let Some(handle) = self.accept.take() {
            let _ = handle.join();
        }
        for flag in self.sse.lock().unwrap().iter() {
            flag.store(true, Ordering::Relaxed);
        }
    }
}

pub struct LiveState {
    pub server: Mutex<Option<LiveServer>>,
    pub hub: Arc<LiveHub>,
}

#[derive(Clone)]
struct Ctx {
    db_path: PathBuf,
    attachments: PathBuf,
    frontend: PathBuf,
    hub: Arc<LiveHub>,
    // how long /api/events holds a quiet poll before answering
    events_timeout: Duration,
    // close flags for in-flight /api/events/stream connections
    sse: Arc<Mutex<Vec<Arc<AtomicBool>>>>,
}

#[tauri::command]
pub fn live_start(app: tauri::AppHandle, port: u16) -> Result<LiveStatus> {
    start(&app, port)
}

#[tauri::command]
pub fn live_stop(app: tauri::AppHandle) -> Result<()> {
    stop(&app)
}

#[tauri::command]
pub fn live_status(app: tauri::AppHandle) -> Option<LiveStatus> {
    status(&app)
}

// tiny_http binds with a plain TcpListener::bind, which leaves the port
// unbindable for ~2*MSL when the app exits with browser connections open
// (TIME_WAIT sockets on the live port). bind with SO_REUSEADDR so a quick
// reopen can rebind the same port at once
fn bind_listener(port: u16) -> std::io::Result<std::net::TcpListener> {
    let socket = socket2::Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::STREAM,
        Some(socket2::Protocol::TCP),
    )?;
    socket.set_reuse_address(true)?;
    socket.bind(&std::net::SocketAddr::from(([0, 0, 0, 0], port)).into())?;
    socket.listen(128)?;
    Ok(socket.into())
}

fn url(port: u16) -> String {
    // prefer the local network address so phones on the same wifi can join
    lan_ip()
        .map(|ip| format!("http://{}:{}", ip, port))
        .unwrap_or_else(|| format!("http://127.0.0.1:{}", port))
}

// best-effort local network address; udp connect never sends packets
fn lan_ip() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback()).then_some(ip.to_string())
}

fn start(app: &tauri::AppHandle, port: u16) -> Result<LiveStatus> {
    let state = app.state::<LiveState>();
    let mut guard = state.server.lock().unwrap();
    if let Some(existing) = guard.as_ref() {
        if existing.port == port {
            return Ok(LiveStatus { port, url: url(port) });
        }
        // port changed while running: fully shut the old server down first,
        // so its listener is closed before we try to bind again
        drop(guard.take());
    }

    let sse = Arc::new(Mutex::new(Vec::new()));
    let ctx = Ctx {
        db_path: app_db_path(app)?,
        attachments: attachments_dir(app)?,
        frontend: frontend_dir(app).ok_or_else(|| {
            "frontend build not found - run `bun run build` and try again".to_string()
        })?,
        hub: state.hub.clone(),
        events_timeout: Duration::from_secs(20),
        sse: sse.clone(),
    };

    let server = Arc::new(
        // 0.0.0.0 so phones and other devices on the local network can connect
        Server::from_listener(
            bind_listener(port)
                .map_err(|e| format!("could not start live server on port {}: {}", port, e))?,
            None,
        )
        .map_err(|e| format!("could not start live server on port {}: {}", port, e))?,
    );

    let accept = server.clone();
    let handle = std::thread::spawn(move || {
        for request in accept.incoming_requests() {
            let ctx = ctx.clone();
            std::thread::spawn(move || handle_request(request, &ctx));
        }
    });

    *guard = Some(LiveServer {
        port,
        server,
        accept: Some(handle),
        sse,
    });
    Ok(LiveStatus { port, url: url(port) })
}

fn stop(app: &tauri::AppHandle) -> Result<()> {
    let state = app.state::<LiveState>();
    let mut guard = state.server.lock().unwrap();
    // LiveServer's Drop impl unblocks the accept loop and joins it, so the
    // listener socket is fully closed and the port freed before we return
    drop(guard.take());
    Ok(())
}

fn status(app: &tauri::AppHandle) -> Option<LiveStatus> {
    let state = app.state::<LiveState>();
    let guard = state.server.lock().unwrap();
    guard
        .as_ref()
        .map(|live| LiveStatus { port: live.port, url: url(live.port) })
}

fn handle_request(request: Request, ctx: &Ctx) {
    let raw_url = request.url().to_string();
    let (path, query) = raw_url.split_once('?').unwrap_or((raw_url.as_str(), ""));
    let method = request.method().clone();

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
        (Method::Post, "/api/backups") => create_backup_http(&mut request, ctx),
        (Method::Post, p) if p.starts_with("/api/backups/") && p.ends_with("/restore") => {
            restore_backup_http(p, ctx)
        }
        (Method::Delete, p) if p.starts_with("/api/backups/") => delete_backup_http(p, ctx),
        (Method::Get, _) | (Method::Head, _) => serve_static(path, ctx),
        _ => json_response(StatusCode(404), json!({ "error": "not found" })),
    };
    let _ = request.respond(response);
}

#[derive(Deserialize)]
struct QueryPayload {
    sql: String,
    #[serde(default)]
    params: Vec<Value>,
}

fn run_query(request: &mut Request, ctx: &Ctx, is_select: bool) -> Response<std::io::Cursor<Vec<u8>>> {
    match execute_query(request, ctx, is_select) {
        Ok(body) => json_response(StatusCode(200), body),
        Err(e) => json_response(StatusCode(400), json!({ "error": e })),
    }
}

fn execute_query(request: &mut Request, ctx: &Ctx, is_select: bool) -> Result<Value> {
    let mut body = String::new();
    request
        .as_reader()
        .take(1024 * 1024)
        .read_to_string(&mut body)
        .map_err(|e| format!("failed to read request body: {}", e))?;
    let payload: QueryPayload =
        serde_json::from_str(&body).map_err(|e| format!("invalid request body: {}", e))?;

    // translate sqlx-style $n placeholders to rusqlite positional ones
    let sql = translate_placeholders(&payload.sql);
    let values: Vec<rusqlite::types::Value> = payload.params.iter().map(to_sqlite_value).collect();

    let conn = open_conn(&ctx.db_path)?;
    if is_select {
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let names: Vec<String> = stmt.column_names().iter().map(|n| n.to_string()).collect();
        let mut rows = stmt
            .query(params_from_iter(values.iter()))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let mut obj = serde_json::Map::new();
            for (i, name) in names.iter().enumerate() {
                let v: rusqlite::types::Value = row.get(i).map_err(|e| e.to_string())?;
                obj.insert(name.clone(), sqlite_to_json(v));
            }
            out.push(Value::Object(obj));
        }
        Ok(json!({ "rows": out }))
    } else {
        let affected = conn
            .execute(&sql, params_from_iter(values.iter()))
            .map_err(|e| e.to_string())?;
        Ok(json!({ "rowsAffected": affected }))
    }
}

// rewrite $n (sqlx style, used by the frontend) to ?n (rusqlite numbered),
// so a placeholder referenced twice binds the same value once
fn translate_placeholders(sql: &str) -> String {
    let chars: Vec<char> = sql.chars().collect();
    let mut out = String::with_capacity(sql.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '$' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 {
                out.push('?');
                out.extend(chars[i + 1..j].iter());
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn to_sqlite_value(v: &Value) -> rusqlite::types::Value {
    match v {
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(b) => rusqlite::types::Value::Integer(*b as i64),
        Value::Number(n) => n
            .as_i64()
            .map(rusqlite::types::Value::Integer)
            .unwrap_or_else(|| rusqlite::types::Value::Real(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        _ => rusqlite::types::Value::Null,
    }
}

fn sqlite_to_json(v: rusqlite::types::Value) -> Value {
    match v {
        rusqlite::types::Value::Null => Value::Null,
        rusqlite::types::Value::Integer(i) => json!(i),
        rusqlite::types::Value::Real(r) => json!(r),
        rusqlite::types::Value::Text(t) => json!(t),
        rusqlite::types::Value::Blob(b) => json!(base64_encode(&b)),
    }
}

fn open_conn(path: &Path) -> Result<Connection> {
    let conn =
        Connection::open(path).map_err(|e| format!("failed to open database: {}", e))?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    Ok(conn)
}

fn json_response<T: Serialize>(status: StatusCode, body: T) -> Response<std::io::Cursor<Vec<u8>>> {
    let data = serde_json::to_vec(&body).unwrap_or_default();
    let headers = vec![
        Header::from_bytes(b"Content-Type", b"application/json; charset=utf-8").unwrap(),
        Header::from_bytes(b"Cache-Control", b"no-cache").unwrap(),
        Header::from_bytes(b"Connection", b"close").unwrap(),
    ];
    let len = data.len();
    Response::new(status, headers, std::io::Cursor::new(data), Some(len), None)
}

fn serve_static(path: &str, ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    let decoded = percent_decode(path.trim_start_matches('/'));
    if decoded.split('/').any(|seg| seg == ".." || seg == ".") {
        return json_response(StatusCode(400), json!({ "error": "invalid path" }));
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
        Err(_) => json_response(StatusCode(404), json!({ "error": "not found" })),
    }
}

fn file_response(path: &Path, data: Vec<u8>, request_path: &str) -> Response<std::io::Cursor<Vec<u8>>> {
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
    Response::new(StatusCode(200), headers, std::io::Cursor::new(data), Some(len), None)
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

fn percent_decode(s: &str) -> String {
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

fn attachment_id(path: &str) -> Option<&str> {
    let id = path.strip_prefix("/api/attachment/")?;
    let valid = !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
    valid.then_some(id)
}

fn serve_attachment(path: &str, query: &str, ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(id) = attachment_id(path) else {
        return json_response(StatusCode(400), json!({ "error": "invalid attachment id" }));
    };
    let mime = query
        .split("mime=")
        .nth(1)
        .map(|v| v.split('&').next().unwrap_or("").to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());
    match std::fs::read(ctx.attachments.join(id)) {
        Ok(data) => {
            let headers = vec![
                Header::from_bytes(b"Content-Type", mime.as_bytes()).unwrap(),
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
        Err(_) => json_response(StatusCode(404), json!({ "error": "attachment not found" })),
    }
}

fn put_attachment(request: &mut Request, path: &str, ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(id) = attachment_id(path) else {
        return json_response(StatusCode(400), json!({ "error": "invalid attachment id" }));
    };
    let mut data = Vec::new();
    if let Err(e) = request.as_reader().take(16 * 1024 * 1024).read_to_end(&mut data) {
        return json_response(StatusCode(400), json!({ "error": format!("failed to read upload: {}", e) }));
    }
    match std::fs::write(ctx.attachments.join(id), &data) {
        Ok(()) => json_response(StatusCode(200), json!({ "ok": true })),
        Err(e) => json_response(
            StatusCode(500),
            json!({ "error": format!("failed to save attachment: {}", e) }),
        ),
    }
}

fn delete_attachment(path: &str, ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(id) = attachment_id(path) else {
        return json_response(StatusCode(400), json!({ "error": "invalid attachment id" }));
    };
    match std::fs::remove_file(ctx.attachments.join(id)) {
        Ok(()) => json_response(StatusCode(200), json!({ "ok": true })),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            json_response(StatusCode(200), json!({ "ok": true }))
        }
        Err(e) => json_response(
            StatusCode(500),
            json!({ "error": format!("failed to delete attachment: {}", e) }),
        ),
    }
}

fn poll_events(ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    // long-poll: hold the request until a db change lands (or a quiet timeout),
    // then answer with a complete response the browser can immediately re-poll
    let last = ctx.hub.generation();
    let changed = ctx.hub.changed_since(last, ctx.events_timeout);
    json_response(StatusCode(200), json!({ "changed": changed }))
}

// sse stream for agents: keep the connection open and push a db-changed event
// whenever the hub generation advances. the hello event carries the current
// generation, so a client that sees a jump larger than one knows it missed
// events and should re-query. the heartbeat doubles as a dead-client check:
// the next write fails on a dropped connection and ends the stream
fn stream_events(writer: &mut dyn Write, ctx: &Ctx, closed: &AtomicBool) {
    let _ = write!(
        writer,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n"
    );
    let mut last = ctx.hub.generation();
    let _ = write!(
        writer,
        "event: connected\r\ndata: {{\"generation\":{last}}}\r\n\r\n"
    );
    let _ = writer.flush();
    while !closed.load(Ordering::Relaxed) {
        let changed = ctx.hub.changed_since(last, Duration::from_secs(5));
        if changed {
            last = ctx.hub.generation();
            let _ = write!(
                writer,
                "event: db-changed\r\ndata: {{\"generation\":{last}}}\r\n\r\n"
            );
        } else {
            let _ = write!(writer, ": ping\r\n\r\n");
        }
        if writer.flush().is_err() {
            return;
        }
    }
}

// "/api/backups/backup-20240101-000000.000[/restore]" -> "backup-20240101-000000.000"
fn backup_name_from_path(path: &str, with_restore_suffix: bool) -> Option<String> {
    let rest = path.strip_prefix("/api/backups/")?;
    let name = if with_restore_suffix {
        rest.strip_suffix("/restore")?
    } else {
        rest
    };
    (!name.is_empty()).then(|| percent_decode(name))
}

fn serve_backups(ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    match backup::list_backups(&ctx.db_path) {
        Ok(list) => json_response(StatusCode(200), json!({ "backups": list })),
        Err(e) => json_response(StatusCode(500), json!({ "error": e })),
    }
}

#[derive(Deserialize)]
struct CreateBackupPayload {
    #[serde(default = "default_keep")]
    keep: usize,
}

fn default_keep() -> usize {
    7
}

fn create_backup_http(request: &mut Request, ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    let mut body = String::new();
    let _ = request.as_reader().take(1024).read_to_string(&mut body);
    let keep = serde_json::from_str::<CreateBackupPayload>(&body)
        .map(|p| p.keep)
        .unwrap_or_else(|_| default_keep());
    match backup::create_backup(&ctx.db_path, keep) {
        Ok(name) => json_response(StatusCode(200), json!({ "name": name })),
        Err(e) => json_response(StatusCode(500), json!({ "error": e })),
    }
}

fn restore_backup_http(path: &str, ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(name) = backup_name_from_path(path, true) else {
        return json_response(StatusCode(400), json!({ "error": "invalid backup name" }));
    };
    match backup::restore_backup(&ctx.db_path, &name) {
        Ok(()) => json_response(StatusCode(200), json!({ "ok": true })),
        Err(e) => json_response(StatusCode(500), json!({ "error": e })),
    }
}

fn delete_backup_http(path: &str, ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(name) = backup_name_from_path(path, false) else {
        return json_response(StatusCode(400), json!({ "error": "invalid backup name" }));
    };
    match backup::delete_backup(&ctx.db_path, &name) {
        Ok(()) => json_response(StatusCode(200), json!({ "ok": true })),
        Err(e) => json_response(StatusCode(500), json!({ "error": e })),
    }
}

fn frontend_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    // packaged app: build dir copied into resources
    if let Ok(resource) = app.path().resource_dir() {
        let bundled = resource.join("build");
        if bundled.join("index.html").exists() {
            return Some(bundled);
        }
    }
    // dev: repo build dir
    let local = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../build");
    (local.join("index.html").exists()).then_some(local)
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::{RecursiveMode, Watcher};
    use tiny_http::TestRequest;

    // the file watcher sees checkpointing writes (per-request connections,
    // like the live server and the cli)
    #[test]
    fn watcher_sees_per_request_writes() {
        let dir = std::env::temp_dir().join(format!(
            "tack-watch-test-{}-watcher",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("tack.db");

        // warm up: create the db with wal + existing -wal/-shm files, exactly
        // like the real app before the watcher starts; keep this connection
        // open so the -wal file is never checkpointed away (like the gui pool)
        let keeper = Connection::open(&db_path).unwrap();
        keeper.pragma_update(None, "journal_mode", "WAL").unwrap();
        keeper.execute_batch("CREATE TABLE t (id TEXT PRIMARY KEY)").unwrap();
        keeper.execute("INSERT INTO t (id) VALUES ('seed')", []).unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |res| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })
        .unwrap();
        watcher.watch(&dir, RecursiveMode::NonRecursive).unwrap();
        // let the watcher settle before recording events
        std::thread::sleep(std::time::Duration::from_millis(1000));
        while rx.try_iter().next().is_some() {}

        // short-lived connection (live server / cli style) - with the keeper
        // still open this only appends to the existing -wal
        {
            let conn2 = Connection::open(&db_path).unwrap();
            conn2
                .execute("INSERT INTO t (id) VALUES ('per-request')", [])
                .unwrap();
        }
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let events: Vec<String> = rx.try_iter().map(|e| format!("{:?}", e)).collect();

        println!("after per-request: {:#?}", events);
        assert!(
            events.iter().any(|e| e.contains("tack.db")),
            "watcher missed the per-request write: {events:?}"
        );
        drop(keeper);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // the events endpoint must long-poll: complete responses that fire
    // quickly when a change lands and only after a quiet timeout otherwise
    #[test]
    fn events_endpoint_long_polls() {
        let dir = std::env::temp_dir().join(format!("tack-sse-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("tack.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch("CREATE TABLE t (id TEXT PRIMARY KEY)").unwrap();
        drop(conn);

        let ctx = Ctx {
            db_path,
            attachments: dir.join("attachments"),
            frontend: dir.clone(),
            hub: Arc::new(LiveHub::default()),
            events_timeout: Duration::from_secs(2),
            sse: Arc::new(Mutex::new(Vec::new())),
        };

        let server = Arc::new(Server::http(("127.0.0.1", 0)).unwrap());
        let port = server.server_addr().to_ip().unwrap().port();
        let accept = server.clone();
        let accept_ctx = ctx.clone();
        std::thread::spawn(move || {
            for request in accept.incoming_requests() {
                let ctx = accept_ctx.clone();
                std::thread::spawn(move || handle_request(request, &ctx));
            }
        });

        fn get_events(port: u16, read_timeout: std::time::Duration) -> String {
            let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
            stream.set_read_timeout(Some(read_timeout)).unwrap();
            use std::io::{BufReader, Write};
            write!(
                stream,
                "GET /api/events HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            stream.flush().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut body = String::new();
            let _ = reader.read_to_string(&mut body);
            body
        }

        // a poll with no change must still complete (quiet timeout)
        let t0 = std::time::Instant::now();
        let quiet = get_events(port, std::time::Duration::from_secs(5));
        assert!(
            quiet.contains("200") && quiet.contains("changed") && !quiet.contains("\"changed\":true"),
            "quiet poll should answer changed:false, got: {quiet:?}"
        );
        assert!(
            t0.elapsed() >= std::time::Duration::from_millis(1500),
            "quiet poll returned too early"
        );

        // a change landing mid-poll must wake it immediately
        let t0 = std::time::Instant::now();
        let handle = std::thread::spawn(move || get_events(port, std::time::Duration::from_secs(5)));
        std::thread::sleep(std::time::Duration::from_millis(400));
        ctx.hub.notify();
        let changed = handle.join().unwrap();
        assert!(
            changed.contains("\"changed\":true"),
            "poll with a mid-flight change should answer changed:true, got: {changed:?}"
        );
        assert!(
            t0.elapsed() < std::time::Duration::from_secs(3),
            "changed poll took too long"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    // the sse stream must answer with event-stream headers, push a hello
    // event with the current generation, then a db-changed event as soon as
    // the hub notifies
    #[test]
    fn events_stream_sse() {
        let dir = std::env::temp_dir().join(format!("tack-sse-stream-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("tack.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch("CREATE TABLE t (id TEXT PRIMARY KEY)").unwrap();
        drop(conn);

        let ctx = Ctx {
            db_path,
            attachments: dir.join("attachments"),
            frontend: dir.clone(),
            hub: Arc::new(LiveHub::default()),
            events_timeout: Duration::from_secs(2),
            sse: Arc::new(Mutex::new(Vec::new())),
        };

        let server = Arc::new(Server::http(("127.0.0.1", 0)).unwrap());
        let port = server.server_addr().to_ip().unwrap().port();
        let accept = server.clone();
        let accept_ctx = ctx.clone();
        std::thread::spawn(move || {
            for request in accept.incoming_requests() {
                let ctx = accept_ctx.clone();
                std::thread::spawn(move || handle_request(request, &ctx));
            }
        });

        use std::io::{BufRead, BufReader, Write};
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        write!(
            stream,
            "GET /api/events/stream HTTP/1.1\r\nHost: localhost\r\nAccept: text/event-stream\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        stream.flush().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());

        // consume response headers up to the blank line
        let mut line = String::new();
        loop {
            line.clear();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" || line == "\n" {
                break;
            }
        }

        // hello event carries the current generation (0)
        let mut frame = String::new();
        for _ in 0..3 {
            line.clear();
            reader.read_line(&mut line).unwrap();
            frame.push_str(&line);
        }
        assert!(
            frame.contains("event: connected") && frame.contains("\"generation\":0"),
            "expected connected event with generation 0, got: {frame:?}"
        );

        // a change landing mid-stream must fire a db-changed event quickly
        let t0 = std::time::Instant::now();
        ctx.hub.notify();
        let mut frame = String::new();
        for _ in 0..3 {
            line.clear();
            reader.read_line(&mut line).unwrap();
            frame.push_str(&line);
        }
        assert!(
            frame.contains("event: db-changed") && frame.contains("\"generation\":1"),
            "expected db-changed event with generation 1, got: {frame:?}"
        );
        assert!(
            t0.elapsed() < std::time::Duration::from_secs(2),
            "db-changed event arrived late"
        );

        drop(reader);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // sqlx-style $n placeholders must become rusqlite numbered ones
    #[test]
    fn translates_placeholders() {
        assert_eq!(
            translate_placeholders("SELECT * FROM tasks WHERE id = $1"),
            "SELECT * FROM tasks WHERE id = ?1"
        );
        assert_eq!(
            translate_placeholders("UPDATE tasks SET a = $1, b = $2 WHERE id = $1"),
            "UPDATE tasks SET a = ?1, b = ?2 WHERE id = ?1"
        );
        // the settings upsert reuses $2 - it must bind to the same value
        assert_eq!(
            translate_placeholders(
                "INSERT INTO settings (key, value) VALUES ($1, $2) ON CONFLICT(key) DO UPDATE SET value = $2"
            ),
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2"
        );
        assert_eq!(translate_placeholders("SELECT 1"), "SELECT 1");
        assert_eq!(translate_placeholders("SELECT $"), "SELECT $");
        assert_eq!(translate_placeholders("SELECT $x"), "SELECT $x");
    }

    #[test]
    fn decodes_percent_encoding() {
        assert_eq!(percent_decode("_app/immutable/a%20b.js"), "_app/immutable/a b.js");
        assert_eq!(percent_decode("favicon.png"), "favicon.png");
        assert_eq!(percent_decode("%2e%2e/etc"), "../etc");
    }

    // the http query + attachment paths against a real sqlite file
    #[test]
    fn api_roundtrip() {
        let dir = std::env::temp_dir().join(format!("tack-live-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("attachments")).unwrap();
        let db_path = dir.join("test.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE t (id TEXT PRIMARY KEY, n INTEGER);
             INSERT INTO t VALUES ('a', 1), ('b', 2);
             CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )
        .unwrap();
        drop(conn);

        let ctx = Ctx {
            db_path,
            attachments: dir.join("attachments"),
            frontend: dir.clone(),
            hub: Arc::new(LiveHub::default()),
            events_timeout: Duration::from_secs(2),
            sse: Arc::new(Mutex::new(Vec::new())),
        };

        // select with sqlx-style placeholder + numeric param
        let request: Request = TestRequest::new()
            .with_method(Method::Post)
            .with_path("/api/select")
            .with_body(r#"{"sql":"SELECT id, n FROM t WHERE n = $1","params":[2]}"#)
            .into();
        let mut request = request;
        let result = execute_query(&mut request, &ctx, true).unwrap();
        assert_eq!(result["rows"][0]["id"], "b");
        assert_eq!(result["rows"][0]["n"], 2);

        // execute returns rows affected
        let request: Request = TestRequest::new()
            .with_method(Method::Post)
            .with_path("/api/execute")
            .with_body(r#"{"sql":"INSERT INTO t (id, n) VALUES ($1, $2)","params":["c", 3]}"#)
            .into();
        let mut request = request;
        let result = execute_query(&mut request, &ctx, false).unwrap();
        assert_eq!(result["rowsAffected"], 1);

        // the settings upsert reuses $2 - it must not fail on param count
        let request: Request = TestRequest::new()
            .with_method(Method::Post)
            .with_path("/api/execute")
            .with_body(
                r#"{"sql":"INSERT INTO settings (key, value) VALUES ($1, $2) ON CONFLICT(key) DO UPDATE SET value = $2","params":["theme","light"]}"#,
            )
            .into();
        let mut request = request;
        let result = execute_query(&mut request, &ctx, false).unwrap();
        assert_eq!(result["rowsAffected"], 1);
        let request: Request = TestRequest::new()
            .with_method(Method::Post)
            .with_path("/api/execute")
            .with_body(
                r#"{"sql":"INSERT INTO settings (key, value) VALUES ($1, $2) ON CONFLICT(key) DO UPDATE SET value = $2","params":["theme","dark"]}"#,
            )
            .into();
        let mut request = request;
        let result = execute_query(&mut request, &ctx, false).unwrap();
        assert_eq!(result["rowsAffected"], 1);

        // attachment upload + read + delete
        let request: Request = TestRequest::new()
            .with_method(Method::Put)
            .with_path("/api/attachment/abc-123")
            .with_body("hello bytes")
            .into();
        let mut request = request;
        let response = put_attachment(&mut request, "/api/attachment/abc-123", &ctx);
        assert_eq!(response.status_code(), StatusCode(200));
        assert_eq!(std::fs::read(ctx.attachments.join("abc-123")).unwrap(), b"hello bytes");

        let response = serve_attachment("/api/attachment/abc-123", "mime=text/plain", &ctx);
        assert_eq!(response.status_code(), StatusCode(200));

        let response = delete_attachment("/api/attachment/abc-123", &ctx);
        assert_eq!(response.status_code(), StatusCode(200));
        assert!(!ctx.attachments.join("abc-123").exists());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    // dropping the server must unblock the accept loop, join it, and close
    // the listener socket, so the port is freed and can be rebound at once
    #[test]
    fn dropped_server_closes_listener() {
        let server = Arc::new(Server::http(("127.0.0.1", 0)).unwrap());
        let port = server.server_addr().to_ip().unwrap().port();
        let accept = server.clone();
        let handle = std::thread::spawn(move || {
            for _ in accept.incoming_requests() {}
        });
        let live = LiveServer {
            port,
            server,
            accept: Some(handle),
            sse: Arc::new(Mutex::new(Vec::new())),
        };

        // while alive, the port accepts connections
        assert!(std::net::TcpStream::connect(("127.0.0.1", port)).is_ok());

        drop(live);

        // after drop the port must reject new connections and be rebindable
        assert!(std::net::TcpStream::connect(("127.0.0.1", port)).is_err());
        assert!(Server::http(("127.0.0.1", port)).is_ok());
    }
}

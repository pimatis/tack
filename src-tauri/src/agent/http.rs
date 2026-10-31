use super::{act, screenshot, tasks, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::io::Read;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tiny_http::{Header, Method, Request, Response, StatusCode};

pub(super) fn json_response<T: Serialize>(
    status: StatusCode,
    body: T,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let data = serde_json::to_vec(&body).unwrap_or_default();
    let headers = vec![
        Header::from_bytes(b"Content-Type", b"application/json; charset=utf-8").unwrap(),
        Header::from_bytes(b"Connection", b"close").unwrap(),
    ];
    let len = data.len();
    Response::new(status, headers, std::io::Cursor::new(data), Some(len), None)
}

pub(super) fn err(status: u16, msg: impl Into<String>) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(
        StatusCode(status),
        json!({ "ok": false, "error": msg.into() }),
    )
}

pub(super) fn read_body(request: &mut Request) -> Result<Value> {
    let mut body = String::new();
    request
        .as_reader()
        .take(64 * 1024)
        .read_to_string(&mut body)
        .map_err(|e| format!("could not read request body: {}", e))?;
    if body.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(&body).map_err(|e| format!("invalid json body: {}", e))
}

pub(super) fn handle_request(request: Request, app: &tauri::AppHandle) {
    let raw_url = request.url().to_string();
    let (path, query) = raw_url.split_once('?').unwrap_or((raw_url.as_str(), ""));
    let method = request.method().clone();
    let mut request = request;

    let response = match (method, path) {
        (Method::Get, "/api/agent/ping") => json_response(
            StatusCode(200),
            json!({ "ok": true, "name": "tack-agent", "version": env!("CARGO_PKG_VERSION") }),
        ),
        (Method::Get, "/api/agent/help") => json_response(StatusCode(200), help()),
        (Method::Get, "/api/agent/snapshot") => snapshot_response(app, query),
        (Method::Get, "/api/agent/screenshot") => screenshot_response(app),
        (Method::Post, "/api/agent/action") => match read_body(&mut request) {
            Ok(body) => action_response(app, &body),
            Err(e) => err(400, e),
        },
        (Method::Post, "/api/agent/confirm") => match read_body(&mut request) {
            Ok(body) => confirm_response(app, &body),
            Err(e) => err(400, e),
        },
        // structured data api: task crud straight to sqlite, so agents do not
        // have to depend on what the ui happens to look like
        (Method::Get, "/api/agent/tasks") => tasks::list(app, query),
        (Method::Post, "/api/agent/tasks") => tasks::create(&mut request, app),
        (Method::Get, "/api/agent/projects") => tasks::projects(app),
        (Method::Get, "/api/agent/labels") => tasks::labels(app),
        (Method::Post, p) if p.starts_with("/api/agent/tasks/") && p.ends_with("/restore") => {
            tasks::restore(app, p)
        }
        (Method::Get, p) if p.starts_with("/api/agent/tasks/") => tasks::get(app, p),
        (Method::Patch, p) if p.starts_with("/api/agent/tasks/") => {
            tasks::update(&mut request, app, p)
        }
        (Method::Delete, p) if p.starts_with("/api/agent/tasks/") => tasks::delete(app, p, query),
        _ => err(404, "not found - try /api/agent/help"),
    };
    let _ = request.respond(response);
}

fn snapshot_response(app: &tauri::AppHandle, query: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    match act(app, "snapshot", &json!({})) {
        Ok(value) => {
            let snapshot = value.get("snapshot").cloned().unwrap_or(value);
            if query.contains("format=text") {
                return text_response(format_snapshot_text(&snapshot));
            }
            json_response(StatusCode(200), snapshot)
        }
        Err(e) => err(500, e),
    }
}

// one line per element; the shape coding agents parse cheapest
fn format_snapshot_text(snapshot: &Value) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "url: {}\ntitle: {}\n",
        snapshot["url"].as_str().unwrap_or(""),
        snapshot["title"].as_str().unwrap_or("")
    ));
    if let Some(elements) = snapshot["elements"].as_array() {
        for el in elements {
            let mut flags = String::new();
            if el["danger"].as_bool().unwrap_or(false) {
                flags.push_str(" [danger]");
            }
            if el["disabled"].as_bool().unwrap_or(false) {
                flags.push_str(" [disabled]");
            }
            if el["offscreen"].as_bool().unwrap_or(false) {
                flags.push_str(" [offscreen]");
            }
            out.push_str(&format!(
                "[{}] {} \"{}\" @ {},{}{}\n",
                el["ref"].as_u64().unwrap_or(0),
                el["role"].as_str().unwrap_or(""),
                el["label"].as_str().unwrap_or(""),
                el["x"].as_i64().unwrap_or(0),
                el["y"].as_i64().unwrap_or(0),
                flags
            ));
        }
    }
    out
}

fn text_response(text: String) -> Response<std::io::Cursor<Vec<u8>>> {
    let data = text.into_bytes();
    let headers = vec![
        Header::from_bytes(b"Content-Type", b"text/plain; charset=utf-8").unwrap(),
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

fn screenshot_response(app: &tauri::AppHandle) -> Response<std::io::Cursor<Vec<u8>>> {
    // swap the ghost cursor to a camera and flash the frame, so the user
    // sees the capture happen; best-effort, a missed marker is not an error
    let _ = act(app, "notify", &json!({ "kind": "screenshot" }));
    match screenshot::capture(app) {
        Ok(png) => {
            let headers = vec![
                Header::from_bytes(b"Content-Type", b"image/png").unwrap(),
                Header::from_bytes(b"Connection", b"close").unwrap(),
            ];
            let len = png.len();
            Response::new(
                StatusCode(200),
                headers,
                std::io::Cursor::new(png),
                Some(len),
                None,
            )
        }
        Err(e) => err(500, e),
    }
}

// the user-approval gate: actions the dom marks dangerous come back with
// danger:true; we ask the user in a native dialog and only then replay the
// action. denied or unanswered requests return ok:false so the agent stops
fn action_response(app: &tauri::AppHandle, body: &Value) -> Response<std::io::Cursor<Vec<u8>>> {
    let name = body["action"].as_str().unwrap_or("").to_string();
    if name.is_empty() {
        return err(400, "missing \"action\"");
    }
    let confirmed = body["confirmed"].as_bool().unwrap_or(false);
    let mut params = body.get("params").cloned().unwrap_or_else(|| json!({}));
    if confirmed {
        params["confirmed"] = json!(true);
    }

    let result = match act(app, &name, &params) {
        Ok(v) => v,
        Err(e) => return err(500, e),
    };
    let danger = result["danger"].as_bool().unwrap_or(false);
    if !danger || confirmed {
        return json_response(StatusCode(200), result);
    }
    let summary = result["summary"]
        .as_str()
        .unwrap_or("a destructive action")
        .to_string();
    match ask_user(app, "Agent wants to act", &format!("{}\n\nAllow?", summary)) {
        true => match act(app, &name, &{
            let mut p = params;
            p["confirmed"] = json!(true);
            p
        }) {
            Ok(v) => {
                let mut v = v;
                v["confirmedBy"] = json!("user");
                json_response(StatusCode(200), v)
            }
            Err(e) => err(500, e),
        },
        false => json_response(
            StatusCode(200),
            json!({ "ok": false, "denied": true, "summary": summary }),
        ),
    }
}

fn confirm_response(app: &tauri::AppHandle, body: &Value) -> Response<std::io::Cursor<Vec<u8>>> {
    let title = body["title"]
        .as_str()
        .unwrap_or("Agent requests approval")
        .to_string();
    let detail = body["body"].as_str().unwrap_or("").to_string();
    let approved = ask_user(app, &title, &detail);
    json_response(StatusCode(200), json!({ "ok": true, "approved": approved }))
}

// native modal attached to the app window; rfd handles the main-thread hop
// internally, so it is safe to block this request thread until the user
// decides. true when "Allow" is pressed
pub(super) fn ask_user(app: &tauri::AppHandle, title: &str, body: &str) -> bool {
    app.dialog()
        .message(body)
        .title(title)
        .kind(tauri_plugin_dialog::MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Allow".to_string(),
            "Deny".to_string(),
        ))
        .blocking_show()
}

fn help() -> Value {
    json!({
        "ok": true,
        "server": "tack agent api - local ui control for ai agents (loopback only)",
        "endpoints": {
            "GET /api/agent/ping": "health check",
            "GET /api/agent/snapshot": "interactable elements with ref ids and danger flags (?format=text for compact lines)",
            "GET /api/agent/screenshot": "png of the app window",
            "POST /api/agent/action": "{\"action\":name,\"params\":{...}} - see actions",
            "POST /api/agent/confirm": "{\"title\":...,\"body\":...} -> {approved} - asks the user in a native dialog; call before any irreversible action",
            "GET /api/agent/tasks": "list tasks (?status=, ?project=prefix-or-id, ?trash=1)",
            "POST /api/agent/tasks": "create task {title, description?, status?, priority?, project?, due_date?, end_date?, estimate_minutes?, parent?}",
            "GET /api/agent/tasks/{id}": "one task with labels and subtasks; id is a uuid or display number (TSK-4, #4)",
            "PATCH /api/agent/tasks/{id}": "update fields; null or \"\" clears nullable fields",
            "DELETE /api/agent/tasks/{id}": "move to trash; ?permanent=1 deletes forever (asks the user first)",
            "POST /api/agent/tasks/{id}/restore": "restore a trashed task",
            "GET /api/agent/projects": "projects with open task counts",
            "GET /api/agent/labels": "all labels",
            "GET /api/agent/help": "this document"
        },
        "actions": {
            "click": "{ref} or {x,y} - press the element",
            "dblclick": "{ref} or {x,y}",
            "rightclick": "{ref} or {x,y} - context menu",
            "hover": "{ref} or {x,y}",
            "type": "{ref,text} - focus and set input value",
            "key": "{keys:\"meta+k\"} - mac shortcuts use meta",
            "scroll": "{ref|x,y,dy[,dx]} - scroll the container under the target (default: page)",
            "drag": "{fromRef,fromX,fromY,toRef,toX,toY} - pointer + html5 drag sequence",
            "goto": "{path} - navigate the app router",
            "wait": "{ms<=5000}",
            "snapshot": "same as GET snapshot",
            "eval": "{code} - arbitrary js in the app; power tool, results are serialized"
        },
        "safety": "elements flagged [danger] (delete/trash/reset/destructive) prompt the user for approval automatically; for anything else irreversible, POST /api/agent/confirm first. denied actions return {ok:false,denied:true} - do not retry"
    })
}

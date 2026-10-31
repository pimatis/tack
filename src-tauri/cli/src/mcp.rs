use crate::agent;
use crate::db::*;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::io::{BufRead, Write};

// mcp stdio server: newline-delimited json-rpc on stdin/stdout. each tool
// call maps to one request on the app's loopback agent port

pub fn serve(conn: &Connection) -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => return Err(format!("stdin read failed: {}", e)),
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        // notifications carry no id and get no response
        if msg.get("id").is_none() {
            continue;
        }
        let id = msg["id"].clone();
        let method = msg["method"].as_str().unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        let out = match method {
            "initialize" => result(
                id,
                json!({
                    "protocolVersion": params["protocolVersion"].as_str().unwrap_or("2024-11-05"),
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "tack", "version": env!("CARGO_PKG_VERSION") }
                }),
            ),
            "ping" => result(id, json!({})),
            "tools/list" => result(id, json!({ "tools": tools() })),
            "tools/call" => match call(conn, &params) {
                Ok(content) => result(id, json!({ "content": content })),
                Err(e) => result(id, json!({ "content": [{ "type": "text", "text": e }], "isError": true })),
            },
            _ => error(id, -32601, &format!("method not found: {}", method)),
        };
        let mut lock = stdout.lock();
        let _ = writeln!(lock, "{}", serde_json::to_string(&out).unwrap_or_default());
        let _ = lock.flush();
    }
    Ok(())
}

fn result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn text(s: impl Into<String>) -> Value {
    json!({ "type": "text", "text": s.into() })
}

fn call(conn: &Connection, params: &Value) -> std::result::Result<Vec<Value>, String> {
    let name = params["name"].as_str().unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    match name {
        "tack_ping" => {
            agent::request_json(conn, "GET", "/api/agent/ping", None).map(|v| vec![text(v.to_string())])
        }
        "tack_snapshot" => {
            let (_, bytes) = agent::request(conn, "GET", "/api/agent/snapshot?format=text", None)?;
            Ok(vec![text(String::from_utf8_lossy(&bytes).to_string())])
        }
        "tack_screenshot" => {
            let (status, bytes) = agent::request(conn, "GET", "/api/agent/screenshot", None)?;
            if !(200..300).contains(&status) {
                let v: Value = serde_json::from_slice(&bytes).unwrap_or(json!({}));
                return Err(v["error"]
                    .as_str()
                    .unwrap_or("screenshot failed")
                    .to_string());
            }
            Ok(vec![json!({
                "type": "image",
                "data": base64_encode(&bytes),
                "mimeType": "image/png"
            })])
        }
        "tack_confirm" => {
            let body = serde_json::to_string(&json!({
                "title": args["title"].as_str().unwrap_or("Agent requests approval"),
                "body": args["body"].as_str().unwrap_or("")
            }))
            .map_err(|e| e.to_string())?;
            agent::request_json(conn, "POST", "/api/agent/confirm", Some(&body))
                .map(|v| vec![text(v.to_string())])
        }
        _ => {
            let action = name.strip_prefix("tack_").unwrap_or(name);
            let params = tool_params(action, &args);
            agent::action(conn, action, params).map(|v| vec![text(v.to_string())])
        }
    }
}

// map mcp tool arguments onto the agent action's params shape
fn tool_params(_action: &str, args: &Value) -> Value {
    let mut p = json!({});
    for key in [
        "ref", "x", "y", "text", "keys", "dx", "dy", "ms", "path", "code", "steps", "fromX",
        "fromY", "toX", "toY", "fromRef", "toRef", "confirmed",
    ] {
        if let Some(v) = args.get(key) {
            p[key] = v.clone();
        }
    }
    p
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn pos(desc: &str) -> Value {
    json!({
        "ref": { "type": "number", "description": format!("element ref from tack_snapshot{}", desc) },
        "x": { "type": "number", "description": "x coordinate in css pixels (alternative to ref)" },
        "y": { "type": "number", "description": "y coordinate in css pixels (alternative to ref)" }
    })
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required
        }
    })
}

fn tools() -> Vec<Value> {
    vec![
        tool(
            "tack_ping",
            "Check that the tack app is running and the agent api is reachable",
            json!({}),
            &[],
        ),
        tool(
            "tack_snapshot",
            "List every interactable element on screen with ref ids, roles, labels and positions. Elements flagged [danger] are destructive and will ask the user for approval when clicked",
            json!({}),
            &[],
        ),
        tool(
            "tack_screenshot",
            "Capture a png screenshot of the tack app window",
            json!({}),
            &[],
        ),
        tool(
            "tack_click",
            "Click an element by ref or x/y coordinates",
            pos(""),
            &[],
        ),
        tool(
            "tack_dblclick",
            "Double-click an element by ref or x/y coordinates",
            pos(""),
            &[],
        ),
        tool(
            "tack_rightclick",
            "Right-click an element (opens its context menu) by ref or x/y",
            pos(""),
            &[],
        ),
        tool(
            "tack_hover",
            "Move the pointer over an element by ref or x/y",
            pos(""),
            &[],
        ),
        tool(
            "tack_type",
            "Set the text of an input, textarea or contenteditable element by ref or x/y",
            json!({
                "ref": { "type": "number", "description": "element ref from tack_snapshot" },
                "x": { "type": "number" },
                "y": { "type": "number" },
                "text": { "type": "string", "description": "text to insert" }
            }),
            &["text"],
        ),
        tool(
            "tack_key",
            "Press a key or shortcut on the focused element, e.g. \"enter\", \"escape\", \"meta+k\" (meta = cmd on macos)",
            json!({
                "keys": { "type": "string", "description": "key name or combo like meta+k, ctrl+shift+z, enter, escape" }
            }),
            &["keys"],
        ),
        tool(
            "tack_scroll",
            "Scroll the page or the container under the given target. positive dy scrolls down",
            json!({
                "ref": { "type": "number" },
                "x": { "type": "number" },
                "y": { "type": "number" },
                "dx": { "type": "number" },
                "dy": { "type": "number", "description": "vertical delta; positive scrolls down" }
            }),
            &[],
        ),
        tool(
            "tack_drag",
            "Drag from one point/element to another (pointer and html5 drag sequence)",
            json!({
                "fromRef": { "type": "number" },
                "fromX": { "type": "number" },
                "fromY": { "type": "number" },
                "toRef": { "type": "number" },
                "toX": { "type": "number" },
                "toY": { "type": "number" },
                "steps": { "type": "number" }
            }),
            &[],
        ),
        tool(
            "tack_goto",
            "Navigate the app to a route like / or /settings or /stats or /trash",
            json!({
                "path": { "type": "string", "description": "route path, e.g. /settings" }
            }),
            &["path"],
        ),
        tool(
            "tack_wait",
            "Wait for the ui to settle after an action (max 5000 ms)",
            json!({
                "ms": { "type": "number" }
            }),
            &[],
        ),
        tool(
            "tack_eval",
            "Run arbitrary javascript inside the app and return the serialized result. Power tool: prefer the typed actions when they cover the job",
            json!({
                "code": { "type": "string", "description": "javascript expression or statement to evaluate" }
            }),
            &["code"],
        ),
        tool(
            "tack_confirm",
            "Ask the user to approve or deny an action in a native dialog. Call this before any irreversible or destructive operation that is not already flagged [danger]",
            json!({
                "title": { "type": "string" },
                "body": { "type": "string", "description": "what the user is being asked to approve" }
            }),
            &["body"],
        ),
    ]
}

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
                    "capabilities": { "tools": {}, "resources": {}, "prompts": {} },
                    "serverInfo": { "name": "tack", "version": env!("CARGO_PKG_VERSION") }
                }),
            ),
            "ping" => result(id, json!({})),
            "tools/list" => result(id, json!({ "tools": tools() })),
            "resources/list" => result(id, json!({ "resources": resources() })),
            "resources/read" => match read_resource(conn, &params) {
                Ok(text) => result(
                    id,
                    json!({ "contents": [{
                        "uri": params["uri"].as_str().unwrap_or(""),
                        "mimeType": "text/plain",
                        "text": text
                    }]}),
                ),
                Err(e) => error(id, -32602, &e),
            },
            "prompts/list" => result(id, json!({ "prompts": prompts() })),
            "prompts/get" => match get_prompt(&params) {
                Ok(p) => result(id, p),
                Err(e) => error(id, -32602, &e),
            },
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
        "tack_task_list" => {
            let mut qs = Vec::new();
            for key in ["status", "project", "trash"] {
                if let Some(v) = args.get(key).and_then(|v| v.as_str()) {
                    qs.push(format!("{}={}", key, v));
                }
            }
            let path = if qs.is_empty() {
                "/api/agent/tasks".to_string()
            } else {
                format!("/api/agent/tasks?{}", qs.join("&"))
            };
            agent::request_json(conn, "GET", &path, None).map(|v| vec![text(v.to_string())])
        }
        "tack_task_get" => {
            let id = args["id"].as_str().unwrap_or_default();
            agent::request_json(conn, "GET", &format!("/api/agent/tasks/{}", id), None)
                .map(|v| vec![text(v.to_string())])
        }
        "tack_task_create" => {
            let body = serde_json::to_string(&args).map_err(|e| e.to_string())?;
            agent::request_json(conn, "POST", "/api/agent/tasks", Some(&body))
                .map(|v| vec![text(v.to_string())])
        }
        "tack_task_update" => {
            let id = args["id"].as_str().unwrap_or_default().to_string();
            let mut fields = args.clone();
            fields.as_object_mut().map(|o| o.remove("id"));
            let body = serde_json::to_string(&fields).map_err(|e| e.to_string())?;
            agent::request_json(
                conn,
                "PATCH",
                &format!("/api/agent/tasks/{}", id),
                Some(&body),
            )
            .map(|v| vec![text(v.to_string())])
        }
        "tack_task_delete" => {
            let id = args["id"].as_str().unwrap_or_default();
            let permanent = args["permanent"].as_bool().unwrap_or(false);
            let path = if permanent {
                format!("/api/agent/tasks/{}?permanent=1", id)
            } else {
                format!("/api/agent/tasks/{}", id)
            };
            agent::request_json(conn, "DELETE", &path, None).map(|v| vec![text(v.to_string())])
        }
        "tack_task_restore" => {
            let id = args["id"].as_str().unwrap_or_default();
            agent::request_json(conn, "POST", &format!("/api/agent/tasks/{}/restore", id), None)
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

// context resources: plain-text views of the db an agent can pull in before
// deciding what to do. read straight from sqlite, so they work even while
// the app itself is closed
fn resources() -> Vec<Value> {
    vec![
        json!({
            "uri": "tack://tasks/open",
            "name": "Open tasks",
            "description": "All non-deleted tasks that are not done or canceled",
            "mimeType": "text/plain"
        }),
        json!({
            "uri": "tack://projects",
            "name": "Projects",
            "description": "Projects with their open task counts",
            "mimeType": "text/plain"
        }),
        json!({
            "uri": "tack://activity",
            "name": "Recent activity",
            "description": "Last 50 activity log entries across all tasks, with the source (gui/cli/agent) of each change",
            "mimeType": "text/plain"
        }),
    ]
}

fn read_resource(conn: &Connection, params: &Value) -> Result<String> {
    match params["uri"].as_str().unwrap_or("") {
        "tack://tasks/open" => {
            let mut stmt = conn
                .prepare(
                    "SELECT p.prefix, t.number, t.title, t.status, t.priority, t.due_date
                     FROM tasks t LEFT JOIN projects p ON t.project_id = p.id
                     WHERE t.deleted_at IS NULL AND t.status NOT IN ('done', 'canceled')
                     ORDER BY t.pinned DESC, t.due_date IS NULL, t.due_date, t.priority",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| {
                    Ok((
                        r.get::<_, Option<String>>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, Option<String>>(5)?,
                    ))
                })
                .map_err(|e| e.to_string())?;
            let mut out = String::new();
            for row in rows.flatten() {
                let (prefix, number, title, status, priority, due) = row;
                let num = match &prefix {
                    Some(p) => format!("{}-{}", p, number),
                    None => format!("#{}", number),
                };
                let due = due.map(|d| format!(" due {}", d)).unwrap_or_default();
                out.push_str(&format!("{} [{} p{}] {}{}\n", num, status, priority, title, due));
            }
            if out.is_empty() {
                out.push_str("(no open tasks)\n");
            }
            Ok(out)
        }
        "tack://projects" => {
            let mut stmt = conn
                .prepare(
                    "SELECT p.prefix, p.name,
                     (SELECT COUNT(*) FROM tasks t WHERE t.project_id = p.id
                      AND t.deleted_at IS NULL AND t.status NOT IN ('done', 'canceled')) AS open_tasks
                     FROM projects p ORDER BY p.name",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                    ))
                })
                .map_err(|e| e.to_string())?;
            let mut out = String::new();
            for row in rows.flatten() {
                let (prefix, name, open) = row;
                out.push_str(&format!("{} {} - {} open tasks\n", prefix, name, open));
            }
            if out.is_empty() {
                out.push_str("(no projects)\n");
            }
            Ok(out)
        }
        "tack://activity" => {
            let mut stmt = conn
                .prepare(
                    "SELECT a.created_at, COALESCE(p.prefix || '-' || t.number, '#' || t.number),
                     t.title, a.action, a.old_value, a.new_value, a.source
                     FROM activity_log a
                     JOIN tasks t ON t.id = a.task_id
                     LEFT JOIN projects p ON p.id = t.project_id
                     ORDER BY a.created_at DESC LIMIT 50",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, Option<String>>(4)?,
                        r.get::<_, Option<String>>(5)?,
                        r.get::<_, String>(6)?,
                    ))
                })
                .map_err(|e| e.to_string())?;
            let mut out = String::new();
            for row in rows.flatten() {
                let (at, num, title, action, old, new, source) = row;
                let detail = match (old, new) {
                    (Some(o), Some(n)) => format!(" {} -> {}", o, n),
                    (None, Some(n)) => format!(" -> {}", n),
                    (Some(o), None) => format!(" {} ->", o),
                    (None, None) => String::new(),
                };
                out.push_str(&format!(
                    "{} {} \"{}\" {}{} ({})\n",
                    at, num, title, action, detail, source
                ));
            }
            if out.is_empty() {
                out.push_str("(no activity yet)\n");
            }
            Ok(out)
        }
        other => Err(format!("unknown resource: {}", other)),
    }
}

fn prompts() -> Vec<Value> {
    vec![
        json!({
            "name": "tack_triage",
            "description": "Review my open tasks and propose a status/priority triage"
        }),
        json!({
            "name": "tack_plan_day",
            "description": "Look at due and open tasks and propose a plan for today"
        }),
    ]
}

fn get_prompt(params: &Value) -> Result<Value> {
    let message = |text: &str| -> Value {
        json!({
            "description": "tack prompt",
            "messages": [{
                "role": "user",
                "content": { "type": "text", "text": text }
            }]
        })
    };
    match params["name"].as_str().unwrap_or("") {
        "tack_triage" => Ok(message(
            "Fetch my open tasks with the tack_task_list tool, then triage them: \
             suggest which belong in backlog vs todo vs in_progress and what priority \
             (0 none, 1 urgent, 2 high, 3 medium, 4 low) each should have. Show me the \
             proposed changes first; apply them with tack_task_update only after I confirm.",
        )),
        "tack_plan_day" => Ok(message(
            "Fetch my open tasks with tack_task_list, look at due dates and priorities, \
             and propose a realistic plan for today: what to do first, what to defer, and \
             what to move to a later date. Ask before changing anything.",
        )),
        other => Err(format!("unknown prompt: {}", other)),
    }
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
        tool(
            "tack_task_list",
            "List tasks straight from the database. Prefer this over reading the ui",
            json!({
                "status": { "type": "string", "description": "backlog | todo | in_progress | done | canceled" },
                "project": { "type": "string", "description": "project id or prefix, e.g. TSK" },
                "trash": { "type": "string", "description": "pass \"1\" to list trashed tasks" }
            }),
            &[],
        ),
        tool(
            "tack_task_get",
            "One task in full, including its labels and subtasks",
            json!({
                "id": { "type": "string", "description": "task uuid or display number like TSK-4 or #4" }
            }),
            &["id"],
        ),
        tool(
            "tack_task_create",
            "Create a task. Returns the created task with its id and display number",
            json!({
                "title": { "type": "string" },
                "description": { "type": "string" },
                "status": { "type": "string", "description": "backlog | todo | in_progress | done | canceled (default: the app's defaultStatus)" },
                "priority": { "type": "number", "description": "0 none, 1 urgent, 2 high, 3 medium, 4 low" },
                "project": { "type": "string", "description": "project id or prefix" },
                "due_date": { "type": "string", "description": "YYYY-MM-DD" },
                "end_date": { "type": "string", "description": "YYYY-MM-DD" },
                "estimate_minutes": { "type": "number" },
                "parent": { "type": "string", "description": "parent task uuid or display number" }
            }),
            &["title"],
        ),
        tool(
            "tack_task_update",
            "Update task fields. Omit a field to keep it; pass null or \"\" to clear nullable fields (description, project, due_date, end_date, estimate_minutes, parent). Setting status to done or canceled banks a running timer",
            json!({
                "id": { "type": "string", "description": "task uuid or display number" },
                "title": { "type": "string" },
                "description": { "type": "string" },
                "status": { "type": "string" },
                "priority": { "type": "number" },
                "project": { "type": "string" },
                "due_date": { "type": "string" },
                "end_date": { "type": "string" },
                "estimate_minutes": { "type": "number" },
                "parent": { "type": "string" },
                "pinned": { "type": "boolean" }
            }),
            &["id"],
        ),
        tool(
            "tack_task_delete",
            "Move a task to trash (recoverable). permanent:true deletes a trashed task forever and asks the user to approve first",
            json!({
                "id": { "type": "string", "description": "task uuid or display number" },
                "permanent": { "type": "boolean", "description": "hard delete; only works on tasks already in the trash" }
            }),
            &["id"],
        ),
        tool(
            "tack_task_restore",
            "Restore a trashed task",
            json!({
                "id": { "type": "string", "description": "task uuid or display number" }
            }),
            &["id"],
        ),
    ]
}

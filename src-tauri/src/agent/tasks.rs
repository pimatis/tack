// structured task crud for agents: direct sqlite access with the same
// invariants the cli enforces (issue numbering, activity log, timer banking,
// trash-first deletion). the contract is data, not pixels - these keep
// working even when the ui moves around. every write is logged with
// source='agent' so the feed can tell agent work apart from human work
use super::http::{ask_user, err, json_response, read_body};
use chrono::{Local, SecondsFormat, TimeZone, Utc};
use super::Result;
use crate::db::app_db_path;
use crate::live::http::percent_decode;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::io::Cursor;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tiny_http::{Request, Response, StatusCode};

const STATUSES: [&str; 5] = ["backlog", "todo", "in_progress", "done", "canceled"];

const TASK_COLS: &str = "t.id, t.number, p.prefix, t.project_id, t.title, t.description,
    t.status, t.priority, t.due_date, t.end_date, t.reminder_at, t.recurrence_rule,
    t.parent_id, t.estimate_minutes, t.tracked_seconds, t.timer_started_at, t.pinned,
    t.created_at, t.updated_at, t.deleted_at, t.assignee";

fn open(app: &AppHandle) -> Result<Connection> {
    let conn = Connection::open(app_db_path(app)?).map_err(|e| e.to_string())?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    Ok(conn)
}

// push an instant refresh to the webview and any live-server clients instead
// of waiting for the wal watcher to notice the write
fn notify(app: &AppHandle) {
    let _ = app.emit("db-changed", ());
    app.state::<crate::live::LiveState>().hub.notify();
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn now_iso() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

fn log_activity(
    conn: &Connection,
    src: &str,
    task_id: &str,
    action: &str,
    field: Option<&str>,
    old_value: Option<&str>,
    new_value: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO activity_log (id, task_id, action, field, old_value, new_value, source, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![new_id(), task_id, action, field, old_value, new_value, src, now_iso()],
    )
    .map_err(|e| format!("failed to log activity: {}", e))?;
    if let Some(name) = src.strip_prefix("agent:") {
        ensure_agent(conn, name, src);
        let _ = conn.execute(
            "UPDATE agents SET last_seen_at = ?1 WHERE name = ?2",
            params![now_iso(), name],
        );
    }
    Ok(())
}

// deterministic badge color, same palette as the cli
const AGENT_COLORS: &[&str] = &[
    "indigo", "amber", "emerald", "rose", "sky", "violet",
    "orange", "teal", "pink", "lime", "cyan", "fuchsia",
];

fn agent_color(name: &str) -> &'static str {
    let h: u32 = name.bytes().fold(5381u32, |h, b| h.wrapping_mul(33).wrapping_add(b as u32));
    AGENT_COLORS[((h ^ (h >> 16)) as usize) % AGENT_COLORS.len()]
}

fn ensure_agent(conn: &Connection, name: &str, harness: &str) {
    let name = name.trim();
    if name.is_empty() {
        return;
    }
    let h = harness.strip_prefix("agent:").unwrap_or(harness);
    let _ = conn.execute(
        "INSERT OR IGNORE INTO agents (id, name, harness, color, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![new_id(), name, h, agent_color(name), now_iso()],
    );
}

// issue number as shown in the app: PREFIX-N, or #N without a project
fn display_number(number: i64, prefix: Option<&str>) -> String {
    match prefix {
        Some(p) => format!("{}-{}", p, number),
        None => format!("#{}", number),
    }
}

// resolve a task reference: uuid, "PREFIX-42", "#42" or bare "42" (same
// grammar the cli accepts)
fn resolve_task_id(conn: &Connection, input: &str) -> Result<String> {
    let is_uuid = input.len() == 36 && input.bytes().filter(|b| *b == b'-').count() == 4;
    if is_uuid {
        return Ok(input.to_string());
    }
    let (prefix, number_str) = match input.split_once('-') {
        Some((p, n)) if !p.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => (Some(p), n),
        _ => {
            let bare = input.strip_prefix('#').unwrap_or(input);
            if bare.bytes().all(|b| b.is_ascii_digit()) {
                (None, bare)
            } else {
                return Err(format!("task {} is neither an id nor a number", input));
            }
        }
    };
    let number: i64 = number_str
        .parse()
        .map_err(|_| format!("task {} is neither an id nor a number", input))?;
    let id: Option<String> = match prefix {
        Some(pfx) => conn
            .query_row(
                "SELECT t.id FROM tasks t JOIN projects p ON t.project_id = p.id
                 WHERE p.prefix = ?1 COLLATE NOCASE AND t.number = ?2",
                params![pfx, number],
                |row| row.get(0),
            )
            .optional(),
        None => conn
            .query_row(
                "SELECT t.id FROM tasks t WHERE t.number = ?1 AND t.project_id IS NULL",
                params![number],
                |row| row.get(0),
            )
            .optional(),
    }
    .map_err(|e| e.to_string())?;
    id.ok_or_else(|| format!("task {} not found", input))
}

// project by uuid or prefix
fn resolve_project_id(conn: &Connection, input: &str) -> Result<String> {
    let is_uuid = input.len() == 36 && input.bytes().filter(|b| *b == b'-').count() == 4;
    if is_uuid {
        let exists: Option<String> = conn
            .query_row(
                "SELECT id FROM projects WHERE id = ?1",
                params![input],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        return exists.ok_or_else(|| format!("project {} not found", input));
    }
    conn.query_row(
        "SELECT id FROM projects WHERE prefix = ?1 COLLATE NOCASE",
        params![input],
        |row| row.get(0),
    )
    .map_err(|_| format!("project '{}' not found", input))
}

fn task_ref(path: &str) -> String {
    let raw = path.strip_prefix("/api/agent/tasks/").unwrap_or_default();
    let raw = raw.strip_suffix("/restore").unwrap_or(raw);
    percent_decode(raw)
}

fn row_json(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    let number: i64 = r.get(1)?;
    let prefix: Option<String> = r.get(2)?;
    Ok(json!({
        "id": r.get::<_, String>(0)?,
        "number": number,
        "display_number": display_number(number, prefix.as_deref()),
        "project_id": r.get::<_, Option<String>>(3)?,
        "title": r.get::<_, String>(4)?,
        "description": r.get::<_, Option<String>>(5)?,
        "status": r.get::<_, String>(6)?,
        "priority": r.get::<_, i64>(7)?,
        "due_date": r.get::<_, Option<String>>(8)?,
        "end_date": r.get::<_, Option<String>>(9)?,
        "reminder_at": r.get::<_, Option<String>>(10)?,
        "recurrence_rule": r.get::<_, Option<String>>(11)?,
        "parent_id": r.get::<_, Option<String>>(12)?,
        "estimate_minutes": r.get::<_, Option<i64>>(13)?,
        "tracked_seconds": r.get::<_, i64>(14)?,
        "timer_started_at": r.get::<_, Option<String>>(15)?,
        "pinned": r.get::<_, i64>(16)? == 1,
        "created_at": r.get::<_, String>(17)?,
        "updated_at": r.get::<_, String>(18)?,
        "deleted_at": r.get::<_, Option<String>>(19)?,
        "assignee": r.get::<_, Option<String>>(20)?,
    }))
}

fn task_by_id(conn: &Connection, id: &str) -> Result<Value> {
    conn.query_row(
        &format!(
            "SELECT {} FROM tasks t LEFT JOIN projects p ON t.project_id = p.id WHERE t.id = ?1",
            TASK_COLS
        ),
        params![id],
        |r| row_json(r),
    )
    .map_err(|_| format!("task {} not found", id))
}

fn str_opt(body: &Value, key: &str) -> Option<Option<String>> {
    body.get(key).map(|v| {
        v.as_str()
            .map(str::to_string)
            .filter(|s| !s.is_empty())
    })
}

fn default_setting(conn: &Connection, key: &str, fallback: &str) -> String {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |r| r.get(0),
    )
    .unwrap_or_else(|_| fallback.to_string())
}

pub(super) fn list(app: &AppHandle, query: &str) -> Response<Cursor<Vec<u8>>> {
    let conn = match open(app) {
        Ok(c) => c,
        Err(e) => return err(500, e),
    };
    let params_map = crate::live::http::parse_query(query);
    let trash = params_map
        .get("trash")
        .is_some_and(|v| v == "1" || v == "true");
    let mut sql = format!(
        "SELECT {} FROM tasks t LEFT JOIN projects p ON t.project_id = p.id WHERE t.deleted_at IS {} NULL",
        TASK_COLS,
        if trash { "NOT" } else { "" }
    );
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(status) = params_map.get("status") {
        if !STATUSES.contains(&status.as_str()) {
            return err(400, format!("status must be one of {}", STATUSES.join(", ")));
        }
        sql.push_str(" AND t.status = ?");
        args.push(Box::new(status.clone()));
    }
    if let Some(project) = params_map.get("project") {
        match resolve_project_id(&conn, project) {
            Ok(pid) => {
                sql.push_str(" AND t.project_id = ?");
                args.push(Box::new(pid));
            }
            Err(e) => return err(404, e),
        }
    }
    if let Some(assignee) = params_map.get("assignee") {
        sql.push_str(" AND t.assignee = ? COLLATE NOCASE");
        args.push(Box::new(assignee.clone()));
    }
    sql.push_str(" ORDER BY t.pinned DESC, t.updated_at DESC");
    let tasks: Vec<Value> = match conn.prepare(&sql) {
        Ok(mut stmt) => {
            let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();
            stmt.query_map(&arg_refs[..], |r| row_json(r))
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
                .unwrap_or_default()
        }
        Err(e) => return err(500, e.to_string()),
    };
    json_response(StatusCode(200), json!({ "ok": true, "tasks": tasks }))
}

pub(super) fn get(app: &AppHandle, path: &str) -> Response<Cursor<Vec<u8>>> {
    let conn = match open(app) {
        Ok(c) => c,
        Err(e) => return err(500, e),
    };
    let id = match resolve_task_id(&conn, &task_ref(path)) {
        Ok(id) => id,
        Err(e) => return err(404, e),
    };
    let mut task = match task_by_id(&conn, &id) {
        Ok(t) => t,
        Err(e) => return err(404, e),
    };
    task["labels"] = conn
        .prepare(
            "SELECT l.id, l.name, l.color FROM labels l
             JOIN task_labels tl ON tl.label_id = l.id WHERE tl.task_id = ?1",
        )
        .and_then(|mut s| {
            s.query_map(params![id], |r| {
                Ok(json!({
                    "id": r.get::<_, String>(0)?,
                    "name": r.get::<_, String>(1)?,
                    "color": r.get::<_, Option<String>>(2)?,
                }))
            })
            .map(|rows| Value::Array(rows.filter_map(|r| r.ok()).collect()))
        })
        .unwrap_or(json!([]));
    task["subtasks"] = conn
        .prepare(
            "SELECT id, title, completed, sort_order FROM subtasks
             WHERE task_id = ?1 ORDER BY sort_order",
        )
        .and_then(|mut s| {
            s.query_map(params![id], |r| {
                Ok(json!({
                    "id": r.get::<_, String>(0)?,
                    "title": r.get::<_, String>(1)?,
                    "completed": r.get::<_, i64>(2)? == 1,
                }))
            })
            .map(|rows| Value::Array(rows.filter_map(|r| r.ok()).collect()))
        })
        .unwrap_or(json!([]));
    json_response(StatusCode(200), json!({ "ok": true, "task": task }))
}

pub(super) fn create(request: &mut Request, app: &AppHandle, src: &str) -> Response<Cursor<Vec<u8>>> {
    let body = match read_body(request) {
        Ok(b) => b,
        Err(e) => return err(400, e),
    };
    let conn = match open(app) {
        Ok(c) => c,
        Err(e) => return err(500, e),
    };
    let now_dt = Local::now();
    // "text" runs the natural-language quick-add parser; explicit body fields
    // always win over parsed values
    let parsed = body
        .get("text")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|t| crate::quickadd::parse(t, now_dt.date_naive(), now_dt.time()));

    let title = body["title"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| parsed.as_ref().map(|p| p.title.clone()))
        .unwrap_or_default();
    if title.is_empty() {
        return err(400, "missing or empty \"title\"");
    }
    let status = match body.get("status").and_then(|v| v.as_str()) {
        Some(s) if !STATUSES.contains(&s) => {
            return err(400, format!("status must be one of {}", STATUSES.join(", ")))
        }
        Some(s) => s.to_string(),
        None => default_setting(&conn, "defaultStatus", "todo"),
    };
    let priority = match body.get("priority").and_then(|v| v.as_i64()) {
        Some(p) if !(0..=4).contains(&p) => return err(400, "priority must be 0-4"),
        Some(p) => p,
        None => parsed
            .as_ref()
            .and_then(|p| p.priority.map(|v| v as i64))
            .unwrap_or_else(|| {
                default_setting(&conn, "defaultPriority", "0")
                    .parse()
                    .unwrap_or(0)
            }),
    };
    let project_ref = body
        .get("project")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| parsed.as_ref().and_then(|p| p.project_key.clone()));
    let project_id = match project_ref.as_deref() {
        Some(p) => match resolve_project_id(&conn, p) {
            Ok(pid) => Some(pid),
            Err(e) => return err(404, e),
        },
        None => None,
    };
    let parent_id = match body.get("parent").and_then(|v| v.as_str()) {
        Some(p) => match resolve_task_id(&conn, p) {
            Ok(id) => Some(id),
            Err(e) => return err(404, e),
        },
        None => None,
    };
    let estimate = match body.get("estimate_minutes").and_then(|v| v.as_i64()) {
        Some(m) if m < 0 => return err(400, "estimate_minutes must be 0 or greater"),
        Some(m) => Some(m),
        None => parsed.as_ref().and_then(|p| p.estimate_minutes),
    };
    let description = body.get("description").and_then(|v| v.as_str());
    let due_date = body
        .get("due_date")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| {
            parsed
                .as_ref()
                .and_then(|p| p.due_date.map(|d| d.format("%Y-%m-%d").to_string()))
        });
    let end_date = body
        .get("end_date")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| {
            parsed
                .as_ref()
                .and_then(|p| p.end_date.map(|d| d.format("%Y-%m-%d").to_string()))
        });
    let reminder_at = body
        .get("reminder_at")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| {
            parsed.as_ref().and_then(|p| {
                p.reminder.and_then(|naive| {
                    Local
                        .from_local_datetime(&naive)
                        .earliest()
                        .map(|dt| {
                            dt.with_timezone(&Utc)
                                .to_rfc3339_opts(SecondsFormat::Millis, true)
                        })
                })
            })
        });
    let recurrence_rule = body
        .get("recurrence_rule")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| parsed.as_ref().and_then(|p| p.recurrence.clone()));
    let pinned = body
        .get("pinned")
        .and_then(|v| v.as_bool())
        .unwrap_or_else(|| parsed.as_ref().is_some_and(|p| p.pinned));
    // explicit assignee wins; a named harness claims its own work
    let assignee = body
        .get("assignee")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| src.strip_prefix("agent:"));

    let id = new_id();
    let now = now_iso();
    let number: i64 = match &project_id {
        Some(pid) => conn.query_row(
            "SELECT COALESCE(MAX(number), 0) + 1 FROM tasks WHERE project_id = ?1",
            params![pid],
            |row| row.get(0),
        ),
        None => conn.query_row(
            "SELECT COALESCE(MAX(number), 0) + 1 FROM tasks WHERE project_id IS NULL",
            [],
            |row| row.get(0),
        ),
    }
    .map_err(|e| e.to_string())
    .unwrap_or(1);

    if let Err(e) = conn.execute(
        "INSERT INTO tasks (id, number, project_id, title, description, status, priority,
         due_date, end_date, parent_id, estimate_minutes, reminder_at, recurrence_rule, assignee, sort_order, pinned, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 0, ?15, ?16, ?17)",
        params![id, number, project_id, title, description, status, priority, due_date, end_date, parent_id, estimate, reminder_at, recurrence_rule, assignee, pinned as i64, now, now],
    ) {
        return err(500, format!("failed to create task: {}", e));
    }
    let _ = log_activity(&conn, src, &id, "created", None, None, None);
    if let Some(a) = assignee {
        ensure_agent(&conn, a, a);
        let _ = log_activity(&conn, src, &id, "assigned", Some("assignee"), None, Some(a));
    }
    let mut missing_labels = Vec::new();
    if let Some(p) = &parsed {
        for name in &p.label_names {
            let label_id = conn
                .query_row(
                    "SELECT id FROM labels WHERE name = ?1 COLLATE NOCASE OR name = ?2 COLLATE NOCASE",
                    params![name, name.replace('-', " ")],
                    |r| r.get::<_, String>(0),
                )
                .optional()
                .unwrap_or(None);
            match label_id {
                Some(lid) => {
                    let _ = conn.execute(
                        "INSERT OR IGNORE INTO task_labels (task_id, label_id) VALUES (?1, ?2)",
                        params![id, lid],
                    );
                }
                None => missing_labels.push(name.clone()),
            }
        }
    }
    notify(app);
    match task_by_id(&conn, &id) {
        Ok(task) => json_response(
            StatusCode(200),
            json!({ "ok": true, "task": task, "missing_labels": missing_labels }),
        ),
        Err(e) => err(500, e),
    }
}

// fields that can be updated; absent keys keep the current value, explicit
// null or "" clears the nullable ones
pub(super) fn update(request: &mut Request, app: &AppHandle, path: &str, src: &str) -> Response<Cursor<Vec<u8>>> {
    let body = match read_body(request) {
        Ok(b) => b,
        Err(e) => return err(400, e),
    };
    let conn = match open(app) {
        Ok(c) => c,
        Err(e) => return err(500, e),
    };
    let id = match resolve_task_id(&conn, &task_ref(path)) {
        Ok(id) => id,
        Err(e) => return err(404, e),
    };
    let current = match conn.query_row(
        "SELECT title, description, status, priority, due_date, end_date, project_id, parent_id, estimate_minutes, pinned, assignee
         FROM tasks WHERE id = ?1",
        params![id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<i64>>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, Option<String>>(10)?,
            ))
        },
    ) {
        Ok(c) => c,
        Err(_) => return err(404, format!("task {} not found", id)),
    };
    let (cur_title, cur_desc, cur_status, cur_priority, cur_due, cur_end, cur_project, cur_parent, cur_estimate, cur_pinned, cur_assignee) =
        current;

    if let Some(t) = body.get("title").and_then(|v| v.as_str()) {
        if t.trim().is_empty() {
            return err(400, "title cannot be empty");
        }
    }
    if let Some(s) = body.get("status").and_then(|v| v.as_str()) {
        if !STATUSES.contains(&s) {
            return err(400, format!("status must be one of {}", STATUSES.join(", ")));
        }
    }
    if let Some(p) = body.get("priority").and_then(|v| v.as_i64()) {
        if !(0..=4).contains(&p) {
            return err(400, "priority must be 0-4");
        }
    }

    let new_title = body
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or(&cur_title)
        .to_string();
    let new_desc = str_opt(&body, "description").unwrap_or(cur_desc.clone());
    let new_status = body
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or(&cur_status)
        .to_string();
    let new_priority = body
        .get("priority")
        .and_then(|v| v.as_i64())
        .unwrap_or(cur_priority);
    let new_due = str_opt(&body, "due_date").unwrap_or(cur_due.clone());
    let new_end = str_opt(&body, "end_date").unwrap_or(cur_end.clone());
    let new_estimate = match body.get("estimate_minutes") {
        Some(v) if v.is_null() => None,
        Some(v) => match v.as_i64() {
            Some(m) if m < 0 => return err(400, "estimate_minutes must be 0 or greater"),
            other => other,
        },
        None => cur_estimate,
    };
    let new_pinned = body
        .get("pinned")
        .and_then(|v| v.as_bool())
        .map(|b| b as i64)
        .unwrap_or(cur_pinned);
    let new_project = match body.get("project") {
        Some(v) if v.is_null() || v.as_str() == Some("") => None,
        Some(v) => match v.as_str().map(|p| resolve_project_id(&conn, p)) {
            Some(Ok(pid)) => Some(pid),
            Some(Err(e)) => return err(404, e),
            None => return err(400, "project must be an id or prefix"),
        },
        None => cur_project.clone(),
    };
    let new_parent = match body.get("parent") {
        Some(v) if v.is_null() || v.as_str() == Some("") => None,
        Some(v) => match v.as_str().map(|p| resolve_task_id(&conn, p)) {
            Some(Ok(pid)) => {
                // walk ancestors of the candidate; hitting our own id is a cycle
                let mut cursor = Some(pid.clone());
                let mut cycle = false;
                for _ in 0..100 {
                    match cursor.as_deref() {
                        None => break,
                        Some(c) if c == id => {
                            cycle = true;
                            break;
                        }
                        Some(c) => {
                            cursor = conn
                                .query_row(
                                    "SELECT parent_id FROM tasks WHERE id = ?1",
                                    params![c],
                                    |row| row.get::<_, Option<String>>(0),
                                )
                                .ok()
                                .flatten();
                        }
                    }
                }
                if cycle {
                    return err(400, "parent would create a cycle");
                }
                Some(pid)
            }
            Some(Err(e)) => return err(404, e),
            None => return err(400, "parent must be a task id or number"),
        },
        None => cur_parent.clone(),
    };

    let new_assignee = str_opt(&body, "assignee").unwrap_or(cur_assignee.clone());
    let now = now_iso();
    let changed = conn.execute(
        "UPDATE tasks SET title = ?1, description = ?2, status = ?3, priority = ?4,
         due_date = ?5, end_date = ?6, project_id = ?7, parent_id = ?8,
         estimate_minutes = ?9, pinned = ?10, assignee = ?11, updated_at = ?12 WHERE id = ?13",
        params![new_title, new_desc, new_status, new_priority, new_due, new_end, new_project, new_parent, new_estimate, new_pinned, new_assignee, now, id],
    );
    match changed {
        Ok(0) => return err(404, format!("task {} not found", id)),
        Err(e) => return err(500, format!("failed to update task: {}", e)),
        _ => {}
    }

    // a terminal status banks any running timer, same as the cli
    if new_status == "done" || new_status == "canceled" {
        let _ = conn.execute(
            "UPDATE tasks SET tracked_seconds = tracked_seconds + MAX(0, CAST((julianday(?1) - julianday(timer_started_at)) * 86400 AS INTEGER)), timer_started_at = NULL WHERE id = ?2 AND timer_started_at IS NOT NULL",
            params![now, id],
        );
    }

    if new_status != cur_status {
        let _ = log_activity(
            &conn,
            src,
            &id,
            "status_changed",
            Some("status"),
            Some(status_label(&cur_status)),
            Some(status_label(&new_status)),
        );
    }
    if new_priority != cur_priority {
        let _ = log_activity(
            &conn,
            src,
            &id,
            "priority_changed",
            Some("priority"),
            Some(priority_label(cur_priority)),
            Some(priority_label(new_priority)),
        );
    }
    if new_title != cur_title {
        let _ = log_activity(&conn, src, &id, "title_changed", Some("title"), Some(&cur_title), Some(&new_title));
    }
    if new_desc != cur_desc {
        let _ = log_activity(&conn, src, &id, "description_changed", Some("description"), None, None);
    }
    if new_due != cur_due {
        let _ = log_activity(&conn, src, &id, "due_date_changed", Some("due_date"), cur_due.as_deref(), new_due.as_deref());
    }
    if new_end != cur_end {
        let _ = log_activity(&conn, src, &id, "end_date_changed", Some("end_date"), cur_end.as_deref(), new_end.as_deref());
    }
    if new_estimate != cur_estimate {
        let _ = log_activity(
            &conn,
            src,
            &id,
            "estimate_changed",
            Some("estimate_minutes"),
            None,
            new_estimate.map(|m| m.to_string()).as_deref(),
        );
    }
    if new_parent != cur_parent {
        let _ = log_activity(&conn, src, &id, "parent_changed", Some("parent_id"), None, new_parent.as_deref());
    }
    if new_assignee != cur_assignee {
        let action = if new_assignee.is_some() { "assigned" } else { "unassigned" };
        if let Some(a) = &new_assignee {
            ensure_agent(&conn, a, a);
        }
        let _ = log_activity(&conn, src, &id, action, Some("assignee"), cur_assignee.as_deref(), new_assignee.as_deref());
    }

    notify(app);
    match task_by_id(&conn, &id) {
        Ok(task) => json_response(StatusCode(200), json!({ "ok": true, "task": task })),
        Err(e) => err(500, e),
    }
}

// default is a move to trash (recoverable). ?permanent=1 hard-deletes a
// trashed task and always asks the user first - it cannot be undone
pub(super) fn delete(app: &AppHandle, path: &str, query: &str, src: &str) -> Response<Cursor<Vec<u8>>> {
    let conn = match open(app) {
        Ok(c) => c,
        Err(e) => return err(500, e),
    };
    let id = match resolve_task_id(&conn, &task_ref(path)) {
        Ok(id) => id,
        Err(e) => return err(404, e),
    };
    let params_map = crate::live::http::parse_query(query);
    let permanent = params_map
        .get("permanent")
        .is_some_and(|v| v == "1" || v == "true");
    if permanent {
        let title: String = conn
            .query_row(
                "SELECT title FROM tasks WHERE id = ?1 AND deleted_at IS NOT NULL",
                params![id],
                |r| r.get(0),
            )
            .unwrap_or_default();
        if title.is_empty() {
            return err(404, "task is not in the trash - delete it first");
        }
        let approved = ask_user(
            app,
            "Agent wants to permanently delete a task",
            &format!("\"{}\" will be deleted forever.\n\nAllow?", title),
        );
        if !approved {
            return json_response(
                StatusCode(200),
                json!({ "ok": false, "denied": true, "summary": format!("permanently delete \"{}\"", title) }),
            );
        }
        match conn.execute("DELETE FROM tasks WHERE id = ?1", params![id]) {
            Ok(_) => {
                notify(app);
                return json_response(StatusCode(200), json!({ "ok": true, "deleted": id, "permanent": true }));
            }
            Err(e) => return err(500, format!("failed to delete task: {}", e)),
        }
    }
    let now = now_iso();
    match conn.execute(
        "UPDATE tasks SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
        params![now, id],
    ) {
        Ok(0) => err(404, "task not found or already trashed"),
        Ok(_) => {
            let _ = log_activity(&conn, src, &id, "trashed", None, None, None);
            notify(app);
            json_response(StatusCode(200), json!({ "ok": true, "trashed": id }))
        }
        Err(e) => err(500, format!("failed to delete task: {}", e)),
    }
}

pub(super) fn restore(app: &AppHandle, path: &str, src: &str) -> Response<Cursor<Vec<u8>>> {
    let conn = match open(app) {
        Ok(c) => c,
        Err(e) => return err(500, e),
    };
    let id = match resolve_task_id(&conn, &task_ref(path)) {
        Ok(id) => id,
        Err(e) => return err(404, e),
    };
    let now = now_iso();
    match conn.execute(
        "UPDATE tasks SET deleted_at = NULL, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NOT NULL",
        params![now, id],
    ) {
        Ok(0) => err(404, "task not found in trash"),
        Ok(_) => {
            let _ = log_activity(&conn, src, &id, "restored", None, None, None);
            notify(app);
            match task_by_id(&conn, &id) {
                Ok(task) => json_response(StatusCode(200), json!({ "ok": true, "task": task })),
                Err(e) => err(500, e),
            }
        }
        Err(e) => err(500, format!("failed to restore task: {}", e)),
    }
}

// the registered agents with their workload, same shape the cli reports
pub(super) fn agents(app: &AppHandle) -> Response<Cursor<Vec<u8>>> {
    let conn = match open(app) {
        Ok(c) => c,
        Err(e) => return err(500, e),
    };
    let rows: Vec<Value> = conn
        .prepare(
            "SELECT a.name, a.harness, a.color, a.last_seen_at,
             (SELECT COUNT(*) FROM tasks t WHERE t.assignee = a.name COLLATE NOCASE
               AND t.deleted_at IS NULL AND t.status NOT IN ('done','canceled')) AS open_tasks,
             (SELECT COUNT(*) FROM tasks t WHERE t.assignee = a.name COLLATE NOCASE
               AND t.deleted_at IS NULL AND t.status = 'in_progress') AS wip
             FROM agents a ORDER BY open_tasks DESC, a.name",
        )
        .and_then(|mut s| {
            s.query_map([], |r| {
                Ok(json!({
                    "name": r.get::<_, String>(0)?,
                    "harness": r.get::<_, Option<String>>(1)?,
                    "color": r.get::<_, String>(2)?,
                    "last_seen_at": r.get::<_, Option<String>>(3)?,
                    "open_tasks": r.get::<_, i64>(4)?,
                    "in_progress": r.get::<_, i64>(5)?,
                }))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<Value>>())
        })
        .unwrap_or_default();
    json_response(StatusCode(200), json!({ "ok": true, "agents": rows }))
}

// one read for everything an agent needs: task + labels + subtasks +
// relations + last 10 activity entries
pub(super) fn context(app: &AppHandle, path: &str) -> Response<Cursor<Vec<u8>>> {
    let conn = match open(app) {
        Ok(c) => c,
        Err(e) => return err(500, e),
    };
    let ref_id = path
        .strip_prefix("/api/agent/tasks/")
        .unwrap_or_default()
        .strip_suffix("/context")
        .unwrap_or_default();
    let id = match resolve_task_id(&conn, &percent_decode(ref_id)) {
        Ok(id) => id,
        Err(e) => return err(404, e),
    };
    let mut task = match task_by_id(&conn, &id) {
        Ok(t) => t,
        Err(e) => return err(404, e),
    };
    task["labels"] = conn
        .prepare("SELECT l.name FROM labels l JOIN task_labels tl ON tl.label_id = l.id WHERE tl.task_id = ?1")
        .and_then(|mut s| {
            s.query_map(params![id], |r| r.get::<_, String>(0))
                .map(|rows| Value::Array(rows.filter_map(|r| r.ok()).map(Value::String).collect()))
        })
        .unwrap_or(json!([]));
    task["subtasks"] = conn
        .prepare("SELECT title, completed FROM subtasks WHERE task_id = ?1 ORDER BY sort_order")
        .and_then(|mut s| {
            s.query_map(params![id], |r| {
                Ok(json!({ "title": r.get::<_, String>(0)?, "completed": r.get::<_, i64>(1)? == 1 }))
            })
            .map(|rows| Value::Array(rows.filter_map(|r| r.ok()).collect()))
        })
        .unwrap_or(json!([]));
    task["relations"] = conn
        .prepare(
            "SELECT r.type, r.related_task_id, o.title FROM task_relations r
             JOIN tasks o ON o.id = r.related_task_id WHERE r.task_id = ?1",
        )
        .and_then(|mut s| {
            s.query_map(params![id], |r| {
                Ok(json!({ "type": r.get::<_, String>(0)?, "task_id": r.get::<_, String>(1)?, "title": r.get::<_, String>(2)? }))
            })
            .map(|rows| Value::Array(rows.filter_map(|r| r.ok()).collect()))
        })
        .unwrap_or(json!([]));
    task["recent_activity"] = conn
        .prepare(
            "SELECT action, new_value, source, created_at FROM activity_log
             WHERE task_id = ?1 ORDER BY created_at DESC LIMIT 10",
        )
        .and_then(|mut s| {
            s.query_map(params![id], |r| {
                Ok(json!({
                    "action": r.get::<_, String>(0)?,
                    "new_value": r.get::<_, Option<String>>(1)?,
                    "source": r.get::<_, String>(2)?,
                    "created_at": r.get::<_, String>(3)?,
                }))
            })
            .map(|rows| Value::Array(rows.filter_map(|r| r.ok()).collect()))
        })
        .unwrap_or(json!([]));
    json_response(StatusCode(200), json!({ "ok": true, "task": task }))
}

pub(super) fn projects(app: &AppHandle) -> Response<Cursor<Vec<u8>>> {
    let conn = match open(app) {
        Ok(c) => c,
        Err(e) => return err(500, e),
    };
    let rows: Vec<Value> = conn
        .prepare(
            "SELECT p.id, p.name, p.prefix, p.description,
             (SELECT COUNT(*) FROM tasks t WHERE t.project_id = p.id AND t.deleted_at IS NULL AND t.status NOT IN ('done', 'canceled')) AS open_tasks
             FROM projects p ORDER BY p.name",
        )
        .and_then(|mut s| {
            s.query_map([], |r| {
                Ok(json!({
                    "id": r.get::<_, String>(0)?,
                    "name": r.get::<_, String>(1)?,
                    "prefix": r.get::<_, String>(2)?,
                    "description": r.get::<_, Option<String>>(3)?,
                    "open_tasks": r.get::<_, i64>(4)?,
                }))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<Value>>())
        })
        .unwrap_or_default();
    json_response(StatusCode(200), json!({ "ok": true, "projects": rows }))
}

pub(super) fn labels(app: &AppHandle) -> Response<Cursor<Vec<u8>>> {
    let conn = match open(app) {
        Ok(c) => c,
        Err(e) => return err(500, e),
    };
    let rows: Vec<Value> = conn
        .prepare("SELECT id, name, color FROM labels ORDER BY name")
        .and_then(|mut s| {
            s.query_map([], |r| {
                Ok(json!({
                    "id": r.get::<_, String>(0)?,
                    "name": r.get::<_, String>(1)?,
                    "color": r.get::<_, Option<String>>(2)?,
                }))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<Value>>())
        })
        .unwrap_or_default();
    json_response(StatusCode(200), json!({ "ok": true, "labels": rows }))
}

fn status_label(status: &str) -> &'static str {
    match status {
        "backlog" => "Backlog",
        "todo" => "Todo",
        "in_progress" => "In progress",
        "done" => "Done",
        "canceled" => "Canceled",
        _ => "Unknown",
    }
}

fn priority_label(priority: i64) -> &'static str {
    match priority {
        0 => "No priority",
        1 => "Urgent",
        2 => "High",
        3 => "Medium",
        4 => "Low",
        _ => "Unknown",
    }
}

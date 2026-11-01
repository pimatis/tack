use super::label;
use crate::db::*;
use crate::quickadd;
use crate::recurrence;
use chrono::{Local, SecondsFormat, TimeZone, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::json;

// accept an rfc3339 instant or a local "YYYY-MM-DDTHH:MM[:SS]"; stored as utc
// iso so the app's string comparison against now works
fn parse_reminder(value: &str) -> Result<Option<String>> {
    let v = value.trim();
    if v.is_empty() {
        return Ok(None);
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(v) {
        return Ok(Some(
            dt.with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        ));
    }
    for fmt in ["%Y-%m-%dT%H:%M", "%Y-%m-%dT%H:%M:%S"] {
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(v, fmt) {
            if let chrono::LocalResult::Single(dt) = Local.from_local_datetime(&naive) {
                return Ok(Some(
                    dt.with_timezone(&Utc)
                        .to_rfc3339_opts(SecondsFormat::Millis, true),
                ));
            }
        }
    }
    Err(format!(
        "Invalid reminder '{}': use ISO 8601 like 2026-09-14T09:00 or 2026-09-14T09:00:00Z",
        value
    ))
}

// columns shared by `list` and `search`, so both render identically
struct TaskRow {
    id: String,
    number: i32,
    title: String,
    status: String,
    priority: i32,
    due_date: Option<String>,
    end_date: Option<String>,
    pinned: bool,
    prefix: Option<String>,
    updated_at: String,
    created_at: String,
    recurrence_rule: Option<String>,
}

fn map_task_row(row: &rusqlite::Row) -> rusqlite::Result<TaskRow> {
    Ok(TaskRow {
        id: row.get(0)?,
        number: row.get(1)?,
        title: row.get(2)?,
        status: row.get(3)?,
        priority: row.get(4)?,
        due_date: row.get(5)?,
        end_date: row.get(6)?,
        pinned: row.get::<_, i32>(7)? == 1,
        prefix: row.get(8)?,
        updated_at: row.get(9)?,
        created_at: row.get(10)?,
        recurrence_rule: row.get(11)?,
    })
}

// issue number as shown in the app: PREFIX-N, or #N without a project
fn display_number(number: i32, prefix: Option<&str>) -> String {
    match prefix {
        Some(p) => format!("{}-{}", p, number),
        None => format!("#{}", number),
    }
}

fn render_task_rows(json: bool, rows: &[TaskRow]) -> Result<()> {
    if json {
        let items: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "number": r.number,
                    "display_number": display_number(r.number, r.prefix.as_deref()),
                    "title": r.title,
                    "status": r.status,
                    "priority": r.priority,
                    "due_date": r.due_date,
                    "end_date": r.end_date,
                    "pinned": r.pinned,
                    "recurrence_rule": r.recurrence_rule,
                    "repeat": r.recurrence_rule.as_deref().and_then(recurrence::label),
                    "project_prefix": r.prefix,
                    "updated_at": r.updated_at,
                    "created_at": r.created_at,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "tasks": items })).map_err(|e| e.to_string())?
        );
    } else {
        let table_rows: Vec<Vec<String>> = rows
            .iter()
            .map(|r| {
                vec![
                    r.id.clone(),
                    display_number(r.number, r.prefix.as_deref()),
                    r.title.clone(),
                    status_label(&r.status).to_string(),
                    priority_label(r.priority).to_string(),
                    r.due_date.clone().unwrap_or("-".to_string()),
                    if r.pinned { "yes" } else { "-" }.to_string(),
                    r.recurrence_rule
                        .as_deref()
                        .and_then(recurrence::label)
                        .unwrap_or("-".to_string()),
                ]
            })
            .collect();
        print_table(
            &[
                "ID", "NUMBER", "TITLE", "STATUS", "PRIORITY", "DUE DATE", "PINNED", "REPEAT",
            ],
            &table_rows,
        );
    }
    Ok(())
}

// wrap each term as an fts5 phrase so user input is matched literally and
// cannot inject fts syntax; mirrors the app's toFtsQuery
fn fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

// "90", "90m", "1h", "1h30m", "1.5h" -> minutes; empty clears the estimate
fn parse_estimate(value: &str) -> Result<Option<i64>> {
    let v: String = value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if v.is_empty() {
        return Ok(None);
    }
    let invalid = || format!("Invalid estimate '{}'", value);
    if let Some((hh, rest)) = v.split_once('h') {
        let hours: f64 = hh.parse().map_err(|_| invalid())?;
        let mins: i64 = rest.trim_end_matches('m').parse().unwrap_or(0);
        return Ok(Some((hours * 60.0).round() as i64 + mins));
    }
    v.trim_end_matches('m')
        .parse::<i64>()
        .map(Some)
        .map_err(|_| invalid())
}

// minutes -> "1h30m" / "45m", inverse of parse_estimate
fn format_estimate(minutes: i64) -> String {
    let h = minutes / 60;
    let m = minutes % 60;
    match (h, m) {
        (0, m) => format!("{}m", m),
        (h, 0) => format!("{}h", h),
        (h, m) => format!("{}h{}m", h, m),
    }
}

// structured "key:value" tokens pulled out of a search query; the rest stays
// free text for fts. mirrors src/lib/search/operators.ts
fn split_search(query: &str) -> (String, Vec<(String, String)>) {
    const KEYS: [&str; 10] = ["status", "priority", "p", "project", "label", "due", "pinned", "is", "has", "type"];
    let mut text = Vec::new();
    let mut ops = Vec::new();
    for token in query.split_whitespace() {
        if let Some((key, value)) = token.split_once(':') {
            let key = key.to_lowercase();
            if !value.is_empty() && KEYS.contains(&key.as_str()) {
                ops.push((key, value.to_lowercase()));
                continue;
            }
        }
        text.push(token.to_string());
    }
    (text.join(" "), ops)
}

// turn the extracted operators into sql conditions + args against alias t/p
fn search_op_sql(ops: &[(String, String)]) -> (String, Vec<String>) {
    let mut sql = String::new();
    let mut args: Vec<String> = Vec::new();
    for (key, value) in ops {
        match key.as_str() {
            "status" => {
                let status = match value.as_str() {
                    "inprogress" | "in-progress" | "doing" => "in_progress",
                    "cancelled" => "canceled",
                    "open" => "todo",
                    s => s,
                };
                sql.push_str(" AND t.status = ?");
                args.push(status.to_string());
            }
            "priority" | "p" => {
                let n = match value.as_str() {
                    "none" | "no" => 0,
                    "urgent" => 1,
                    "high" => 2,
                    "medium" => 3,
                    "low" => 4,
                    s => s.parse().unwrap_or(-1),
                };
                sql.push_str(" AND t.priority = ?");
                args.push(n.to_string());
            }
            "project" => {
                if value == "none" || value == "inbox" {
                    sql.push_str(" AND t.project_id IS NULL");
                } else {
                    sql.push_str(" AND (p.prefix = ? COLLATE NOCASE OR p.name = ? COLLATE NOCASE)");
                    args.push(value.clone());
                    args.push(value.clone());
                }
            }
            "label" => {
                sql.push_str(
                    " AND EXISTS (SELECT 1 FROM task_labels tl JOIN labels l ON l.id = tl.label_id WHERE tl.task_id = t.id AND l.name = ? COLLATE NOCASE)",
                );
                args.push(value.clone());
            }
            "due" => match value.as_str() {
                "today" => sql.push_str(" AND t.due_date = date('now', 'localtime')"),
                "tomorrow" => {
                    sql.push_str(" AND t.due_date = date('now', 'localtime', '+1 day')")
                }
                "week" => sql.push_str(
                    " AND t.due_date BETWEEN date('now', 'localtime') AND date('now', 'localtime', '+7 days')",
                ),
                "overdue" => sql.push_str(" AND t.due_date < date('now', 'localtime')"),
                "none" => sql.push_str(" AND t.due_date IS NULL AND t.end_date IS NULL"),
                _ => {}
            },
            "is" => {
                let exists = "SELECT 1 FROM task_relations r JOIN tasks b ON b.id = r.task_id WHERE r.related_task_id = t.id AND r.type = 'blocks' AND b.status NOT IN ('done', 'canceled') AND b.deleted_at IS NULL";
                if value == "blocked" {
                    sql.push_str(&format!(" AND EXISTS ({})", exists));
                } else if value == "unblocked" {
                    sql.push_str(&format!(" AND NOT EXISTS ({})", exists));
                }
            }
            "has" => {
                if value == "parent" {
                    sql.push_str(" AND t.parent_id IS NOT NULL");
                }
            }
            "type" => match value.as_str() {
                "subtask" => sql.push_str(" AND t.parent_id IS NOT NULL"),
                "toplevel" => sql.push_str(" AND t.parent_id IS NULL"),
                _ => {}
            },
            "pinned" => {
                if ["1", "true", "yes"].contains(&value.as_str()) {
                    sql.push_str(" AND t.pinned = 1");
                } else if ["0", "false", "no"].contains(&value.as_str()) {
                    sql.push_str(" AND t.pinned = 0");
                }
            }
            _ => {}
        }
    }
    (sql, args)
}

pub fn create(
    conn: &Connection,
    json: bool,
    title: &str,
    project: Option<&str>,
    project_prefix: Option<&str>,
    status: Option<&str>,
    priority: Option<i32>,
    due_date: Option<&str>,
    end_date: Option<&str>,
    description: Option<&str>,
    reminder: Option<&str>,
    repeat: Option<&str>,
    parent: Option<&str>,
    estimate: Option<&str>,
) -> Result<()> {
    let project_id = resolve_project_id(conn, project, project_prefix)?;
    let parent_id = match parent {
        Some(p) => Some(resolve_task_id(conn, p)?),
        None => None,
    };
    let estimate_minutes = match estimate {
        Some(e) => parse_estimate(e)?,
        None => None,
    };
    let reminder_at = match reminder {
        Some(value) => parse_reminder(value)?,
        None => None,
    };
    let recurrence_rule = match repeat {
        Some(value) => {
            let rule = recurrence::normalize(value)?;
            (!rule.is_empty()).then_some(rule)
        }
        None => None,
    };

    let final_status = status
        .map(|s| s.to_string())
        .unwrap_or_else(|| get_default_status(conn));
    let final_priority = priority.unwrap_or_else(|| get_default_priority(conn));

    let (id, number, now) = insert_task(
        conn,
        title,
        project_id.as_deref(),
        description,
        &final_status,
        final_priority,
        due_date,
        end_date,
        reminder_at.as_deref(),
        recurrence_rule.as_deref(),
        parent_id.as_deref(),
        estimate_minutes,
        false,
    )?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "task_created",
                "task": {
                    "id": id,
                    "number": number,
                    "title": title,
                    "status": final_status,
                    "priority": final_priority,
                    "due_date": due_date,
                    "end_date": end_date,
                    "reminder_at": reminder_at,
                    "recurrence_rule": recurrence_rule,
                    "project_id": project_id,
                    "created_at": now,
                }
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Created task: #{} - {}", number, title);
        println!("ID: {}", id);
        println!("Status: {}", status_label(&final_status));
        println!("Priority: {}", priority_label(final_priority));
        if let Some(rule) = recurrence_rule.as_deref() {
            println!("Repeat: {}", recurrence::label(rule).unwrap_or_default());
        }
    }
    Ok(())
}

// shared insert for `create` and `add`: assigns the per-project number, writes
// the row and logs the activity. returns (id, number, created_at)
#[allow(clippy::too_many_arguments)]
fn insert_task(
    conn: &Connection,
    title: &str,
    project_id: Option<&str>,
    description: Option<&str>,
    status: &str,
    priority: i32,
    due_date: Option<&str>,
    end_date: Option<&str>,
    reminder_at: Option<&str>,
    recurrence_rule: Option<&str>,
    parent_id: Option<&str>,
    estimate_minutes: Option<i64>,
    pinned: bool,
) -> Result<(String, i32, String)> {
    let id = new_id();
    let now = now_iso();
    let number: i32 = if let Some(pid) = project_id {
        conn.query_row(
            "SELECT COALESCE(MAX(number), 0) + 1 FROM tasks WHERE project_id = ?1",
            params![pid],
            |row| row.get(0),
        )
    } else {
        conn.query_row(
            "SELECT COALESCE(MAX(number), 0) + 1 FROM tasks WHERE project_id IS NULL",
            [],
            |row| row.get(0),
        )
    }
    .map_err(|e| format!("Failed to assign task number: {}", e))?;

    conn.execute(
        "INSERT INTO tasks (id, number, project_id, title, description, status, priority, due_date, end_date, reminder_at, recurrence_rule, parent_id, estimate_minutes, sort_order, pinned, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 0, ?14, ?15, ?16)",
        params![id, number, project_id, title, description, status, priority, due_date, end_date, reminder_at, recurrence_rule, parent_id, estimate_minutes, pinned, now, now],
    ).map_err(|e| format!("Failed to create task: {}", e))?;

    log_activity(conn, &id, "created", None, None, None, "cli")?;
    Ok((id, number, now))
}

// `tack task add tomorrow 15:00 finish report #work !high every friday` -
// the line parses into fields (see quickadd.rs), leftovers become the title
pub fn add(conn: &Connection, json: bool, text: &str, dry_run: bool) -> Result<()> {
    let now = Local::now();
    let parsed = quickadd::parse(text, now.date_naive(), now.time());
    if parsed.title.is_empty() {
        return Err("Nothing to create: no title text left after parsing".to_string());
    }

    let project_id = match parsed.project_key.as_deref() {
        Some(key) => Some(
            conn.query_row(
                "SELECT id FROM projects WHERE prefix = ?1 COLLATE NOCASE OR name = ?1 COLLATE NOCASE",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .map_err(|_| format!("No project matches '#{}'", key))?,
        ),
        None => None,
    };

    let mut label_ids = Vec::new();
    let mut missing = Vec::new();
    for name in &parsed.label_names {
        let spaced = name.replace('-', " ");
        match conn
            .query_row(
                "SELECT id FROM labels WHERE name = ?1 COLLATE NOCASE OR name = ?2 COLLATE NOCASE",
                params![name, spaced],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
        {
            Some(id) => label_ids.push(id),
            None => missing.push(name.clone()),
        }
    }

    let reminder_at = parsed.reminder.and_then(|naive| {
        Local
            .from_local_datetime(&naive)
            .earliest()
            .map(|dt| {
                dt.with_timezone(&Utc)
                    .to_rfc3339_opts(SecondsFormat::Millis, true)
            })
    });
    let status = get_default_status(conn);
    let priority = parsed
        .priority
        .unwrap_or_else(|| get_default_priority(conn));
    let due_date = parsed.due_date.map(|d| d.format("%Y-%m-%d").to_string());
    let end_date = parsed.end_date.map(|d| d.format("%Y-%m-%d").to_string());

    if dry_run {
        let out = json!({
            "action": "task_add_dry_run",
            "label_ids": label_ids,
            "missing_labels": missing,
            "task": {
                "title": parsed.title,
                "status": status,
                "priority": priority,
                "project_id": project_id,
                "due_date": due_date,
                "end_date": end_date,
                "reminder_at": reminder_at,
                "recurrence_rule": parsed.recurrence,
                "estimate_minutes": parsed.estimate_minutes,
                "pinned": parsed.pinned,
            }
        });
        if json {
            println!("{}", serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?);
        } else {
            println!("Would create: {}", out);
        }
        return Ok(());
    }

    let (id, number, created) = insert_task(
        conn,
        &parsed.title,
        project_id.as_deref(),
        None,
        &status,
        priority,
        due_date.as_deref(),
        end_date.as_deref(),
        reminder_at.as_deref(),
        parsed.recurrence.as_deref(),
        None,
        parsed.estimate_minutes,
        parsed.pinned,
    )?;

    if !label_ids.is_empty() {
        label::set_labels(conn, &id, &label_ids)?;
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "task_created",
                "task": {
                    "id": id,
                    "number": number,
                    "title": parsed.title,
                    "status": status,
                    "priority": priority,
                    "due_date": due_date,
                    "end_date": end_date,
                    "reminder_at": reminder_at,
                    "recurrence_rule": parsed.recurrence,
                    "project_id": project_id,
                    "pinned": parsed.pinned,
                    "estimate_minutes": parsed.estimate_minutes,
                    "created_at": created,
                },
                "label_ids": label_ids,
                "missing_labels": missing,
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Created task: #{} - {}", number, parsed.title);
        println!("ID: {}", id);
        println!("Status: {}", status_label(&status));
        println!("Priority: {}", priority_label(priority));
        if let Some(rule) = parsed.recurrence.as_deref() {
            println!("Repeat: {}", recurrence::label(rule).unwrap_or_default());
        }
        if !missing.is_empty() {
            println!("Note: unknown label(s) skipped: {}", missing.join(", "));
        }
    }
    Ok(())
}

pub fn list(
    conn: &Connection,
    json: bool,
    project: Option<&str>,
    project_prefix: Option<&str>,
    status: Option<&str>,
    priority: Option<i32>,
    pinned: bool,
    since: Option<&str>,
) -> Result<()> {
    let project_id = resolve_project_id(conn, project, project_prefix)?;

    let mut sql = String::from(
        "SELECT t.id, t.number, t.title, t.status, t.priority, t.due_date, t.end_date, t.pinned, p.prefix, t.updated_at, t.created_at, t.recurrence_rule
         FROM tasks t
         LEFT JOIN projects p ON t.project_id = p.id
         WHERE t.deleted_at IS NULL",
    );
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref pid) = project_id {
        sql.push_str(" AND t.project_id = ?");
        args.push(Box::new(pid.clone()));
    }
    if let Some(s) = status {
        sql.push_str(" AND t.status = ?");
        args.push(Box::new(s.to_string()));
    }
    if let Some(p) = priority {
        sql.push_str(" AND t.priority = ?");
        args.push(Box::new(p));
    }
    if pinned {
        sql.push_str(" AND t.pinned = 1");
    }
    if let Some(s) = since {
        sql.push_str(" AND t.updated_at > ?");
        args.push(Box::new(s.to_string()));
    }
    sql.push_str(" ORDER BY t.pinned DESC, t.updated_at DESC");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to query tasks: {}", e))?;

    let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();
    let rows: Vec<TaskRow> = stmt
        .query_map(&arg_refs[..], map_task_row)
        .map_err(|e| format!("Failed to query tasks: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    render_task_rows(json, &rows)
}

// full-text search over the task index: title, description, subtasks, label
// names, project name and the issue number. trashed tasks are excluded.
pub fn search(
    conn: &Connection,
    json: bool,
    query: &str,
    project: Option<&str>,
    project_prefix: Option<&str>,
    status: Option<&str>,
    limit: i64,
) -> Result<()> {
    let (text, ops) = split_search(query);
    let match_query = fts_query(&text);
    if match_query.is_empty() && ops.is_empty() {
        return Err("Empty search query".to_string());
    }
    let project_id = resolve_project_id(conn, project, project_prefix)?;

    // when the query is only operators ("status:todo due:week") there is no
    // text to match, so the fts table joins as a plain task listing
    let mut sql = if match_query.is_empty() {
        String::from(
            "SELECT t.id, t.number, t.title, t.status, t.priority, t.due_date, t.end_date, t.pinned, p.prefix, t.updated_at, t.created_at, t.recurrence_rule
             FROM tasks t
             LEFT JOIN projects p ON t.project_id = p.id
             WHERE t.deleted_at IS NULL",
        )
    } else {
        String::from(
            "SELECT t.id, t.number, t.title, t.status, t.priority, t.due_date, t.end_date, t.pinned, p.prefix, t.updated_at, t.created_at, t.recurrence_rule
             FROM tasks_fts
             JOIN tasks t ON t.id = tasks_fts.task_id
             LEFT JOIN projects p ON t.project_id = p.id
             WHERE tasks_fts MATCH ? AND t.deleted_at IS NULL",
        )
    };
    let has_text = !match_query.is_empty();
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if has_text {
        args.push(Box::new(match_query));
    }
    let (op_sql, op_args) = search_op_sql(&ops);
    sql.push_str(&op_sql);
    for a in op_args {
        args.push(Box::new(a));
    }

    if let Some(ref pid) = project_id {
        sql.push_str(" AND t.project_id = ?");
        args.push(Box::new(pid.clone()));
    }
    if let Some(s) = status {
        sql.push_str(" AND t.status = ?");
        args.push(Box::new(s.to_string()));
    }
    if has_text {
        sql.push_str(" ORDER BY rank LIMIT ?");
    } else {
        sql.push_str(" ORDER BY t.updated_at DESC LIMIT ?");
    }
    args.push(Box::new(limit));

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to search tasks: {}", e))?;
    let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();
    let rows: Vec<TaskRow> = stmt
        .query_map(&arg_refs[..], map_task_row)
        .map_err(|e| format!("Failed to search tasks: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    render_task_rows(json, &rows)
}

pub fn show(conn: &Connection, json: bool, id: &str) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let task = conn.query_row(
        "SELECT t.id, t.number, t.title, t.description, t.status, t.priority, t.due_date, t.end_date, t.pinned,
                t.project_id, p.name, p.prefix, t.created_at, t.updated_at, t.reminder_at,
                t.recurrence_rule, t.recurrence_parent_id, t.estimate_minutes, t.tracked_seconds
         FROM tasks t
         LEFT JOIN projects p ON t.project_id = p.id
         WHERE t.id = ?1",
        params![id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i32>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, i32>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, Option<String>>(15)?,
                row.get::<_, Option<String>>(16)?,
                row.get::<_, Option<i64>>(17)?,
                row.get::<_, i64>(18)?,
            ))
        },
    ).map_err(|_| format!("Task {} not found", id))?;

    let (
        tid,
        number,
        title,
        desc,
        status,
        priority,
        due,
        end,
        pinned,
        _project_id,
        project_name,
        project_prefix,
        created,
        updated,
        reminder,
        recurrence_rule,
        recurrence_parent_id,
        estimate_minutes,
        tracked_seconds,
    ) = task;

    let display_number = match &project_prefix {
        Some(p) => format!("{}-{}", p, number),
        None => format!("#{}", number),
    };

    // attachments: app saves files to disk and stores the path; cli-added ones live as base64 in db
    let attachments: Vec<(String, String, String, String, i64, String)> = conn
        .prepare(
            "SELECT id, file_name, file_path, mime_type, file_size, created_at
             FROM task_attachments WHERE task_id = ?1 ORDER BY created_at ASC",
        )
        .map_err(|e| e.to_string())?
        .query_map(params![id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    let display_path = |p: &str| {
        if p.starts_with("data:") {
            "(stored in database)".to_string()
        } else {
            p.to_string()
        }
    };

    if json {
        let labels: Vec<String> = conn
            .prepare("SELECT l.name FROM task_labels tl JOIN labels l ON tl.label_id = l.id WHERE tl.task_id = ?1")
            .map_err(|e| e.to_string())?
            .query_map(params![id], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        let subtask_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM subtasks WHERE task_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let completed: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM subtasks WHERE task_id = ?1 AND completed = 1",
                params![id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "task": {
                    "id": tid,
                    "number": number,
                    "display_number": display_number,
                    "title": title,
                    "description": desc,
                    "status": status,
                    "priority": priority,
                    "due_date": due,
                    "end_date": end,
                    "reminder_at": reminder,
                    "recurrence_rule": recurrence_rule,
                    "recurrence_parent_id": recurrence_parent_id,
                    "pinned": pinned == 1,
                    "estimate_minutes": estimate_minutes,
                    "tracked_seconds": tracked_seconds,
                    "project": {
                        "name": project_name,
                        "prefix": project_prefix,
                    },
                    "labels": labels,
                    "attachments": attachments.iter().map(|a| json!({
                        "id": a.0,
                        "file_name": a.1,
                        "file_path": display_path(&a.2),
                        "mime_type": a.3,
                        "file_size": a.4,
                        "created_at": a.5,
                    })).collect::<Vec<_>>(),
                    "subtasks_total": subtask_count,
                    "subtasks_completed": completed,
                    "created_at": created,
                    "updated_at": updated,
                }
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("ID:          {}", tid);
        println!("Number:      {}", display_number);
        println!("Title:       {}", title);
        println!("Description: {}", desc.unwrap_or("-".to_string()));
        println!("Status:      {}", status_label(&status));
        println!("Priority:    {}", priority_label(priority));
        println!("Due date:    {}", due.unwrap_or("-".to_string()));
        println!("End date:    {}", end.unwrap_or("-".to_string()));
        println!("Reminder:    {}", reminder.unwrap_or("-".to_string()));
        println!(
            "Repeat:      {}",
            recurrence_rule
                .as_deref()
                .and_then(recurrence::label)
                .unwrap_or("-".to_string())
        );
        println!("Pinned:      {}", if pinned == 1 { "yes" } else { "no" });
        println!(
            "Estimate:    {}",
            estimate_minutes
                .map(|m| format_estimate(m))
                .unwrap_or("-".to_string())
        );
        println!("Tracked:     {}", format_estimate(tracked_seconds / 60));
        if let Some(pname) = project_name {
            println!(
                "Project:     {} ({})",
                pname,
                project_prefix.unwrap_or_default()
            );
        } else {
            println!("Project:     -");
        }
        println!("Created:     {}", created);
        println!("Updated:     {}", updated);

        let labels: Vec<String> = conn
            .prepare("SELECT l.name FROM task_labels tl JOIN labels l ON tl.label_id = l.id WHERE tl.task_id = ?1")
            .map_err(|e| e.to_string())?
            .query_map(params![id], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        if !labels.is_empty() {
            println!("Labels:      {}", labels.join(", "));
        }

        if !attachments.is_empty() {
            println!("Attachments:");
            for a in &attachments {
                let size = if a.4 < 1024 {
                    format!("{} B", a.4)
                } else if a.4 < 1024 * 1024 {
                    format!("{} KB", a.4 / 1024)
                } else {
                    format!("{:.1} MB", a.4 as f64 / (1024.0 * 1024.0))
                };
                println!("  {} ({}, {}) -> {}", a.1, a.3, size, display_path(&a.2));
            }
        }

        let subtask_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM subtasks WHERE task_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if subtask_count > 0 {
            let completed: i32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM subtasks WHERE task_id = ?1 AND completed = 1",
                    params![id],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            println!("Subtasks:    {}/{} completed", completed, subtask_count);
        }
    }

    Ok(())
}

/// `tack task edit TSK-4 'yarın !high +bug'` — same grammar as `add`, applied
/// as a patch: only the fields the parser extracts are touched
pub fn edit(conn: &Connection, json: bool, id: &str, text: &str) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let now = Local::now();
    let parsed = quickadd::parse(text, now.date_naive(), now.time());

    let mut sets: Vec<&str> = Vec::new();
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if !parsed.title.is_empty() {
        sets.push("title = ?");
        args.push(Box::new(parsed.title.clone()));
    }
    if let Some(key) = &parsed.project_key {
        let pid = conn
            .query_row(
                "SELECT id FROM projects WHERE prefix = ?1 COLLATE NOCASE OR name = ?1 COLLATE NOCASE",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .map_err(|_| format!("No project matches '#{}'", key))?;
        sets.push("project_id = ?");
        args.push(Box::new(pid));
    }
    if let Some(p) = parsed.priority {
        sets.push("priority = ?");
        args.push(Box::new(p));
    }
    if parsed.pinned {
        sets.push("pinned = 1");
    }
    if let Some(d) = parsed.due_date {
        sets.push("due_date = ?");
        args.push(Box::new(d.format("%Y-%m-%d").to_string()));
    }
    if let Some(d) = parsed.end_date {
        sets.push("end_date = ?");
        args.push(Box::new(d.format("%Y-%m-%d").to_string()));
    }
    if let Some(naive) = parsed.reminder {
        let r = Local
            .from_local_datetime(&naive)
            .earliest()
            .map(|dt| {
                dt.with_timezone(&Utc)
                    .to_rfc3339_opts(SecondsFormat::Millis, true)
            })
            .ok_or_else(|| "Invalid reminder".to_string())?;
        sets.push("reminder_at = ?");
        args.push(Box::new(r));
    }
    if let Some(r) = &parsed.recurrence {
        sets.push("recurrence_rule = ?");
        args.push(Box::new(r.clone()));
    }
    if let Some(m) = parsed.estimate_minutes {
        sets.push("estimate_minutes = ?");
        args.push(Box::new(m));
    }
    sets.push("updated_at = ?");
    args.push(Box::new(now_iso()));

    if sets.len() > 1 {
        let sql = format!("UPDATE tasks SET {} WHERE id = ?", sets.join(", "));
        args.push(Box::new(id.clone()));
        let refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();
        conn.execute(&sql, &refs[..]).map_err(|e| e.to_string())?;
        let _ = log_activity(conn, &id, "updated", None, None, None, "cli");
    }

    // +label tokens add to the task's labels without dropping existing ones
    let mut missing = Vec::new();
    for name in &parsed.label_names {
        let spaced = name.replace('-', " ");
        let label_id = conn
            .query_row(
                "SELECT id FROM labels WHERE name = ?1 COLLATE NOCASE OR name = ?2 COLLATE NOCASE",
                params![name, spaced],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        match label_id {
            Some(lid) => {
                conn.execute(
                    "INSERT OR IGNORE INTO task_labels (task_id, label_id) VALUES (?1, ?2)",
                    params![id, lid],
                )
                .map_err(|e| e.to_string())?;
            }
            None => missing.push(name.clone()),
        }
    }
    if !missing.is_empty() {
        eprintln!("Warning: unknown labels skipped: {}", missing.join(", "));
    }
    show(conn, json, &id)
}

pub fn update(
    conn: &Connection,
    json: bool,
    id: &str,
    title: Option<&str>,
    description: Option<&str>,
    status: Option<&str>,
    priority: Option<i32>,
    due_date: Option<&str>,
    end_date: Option<&str>,
    reminder: Option<&str>,
    repeat: Option<&str>,
    parent: Option<&str>,
    estimate: Option<&str>,
) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let current = conn.query_row(
        "SELECT title, description, status, priority, due_date, end_date, reminder_at, recurrence_rule FROM tasks WHERE id = ?1",
        params![id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        },
    ).map_err(|_| format!("Task {} not found", id))?;

    let (cur_title, cur_desc, cur_status, cur_priority, cur_due, cur_end, cur_reminder, cur_rule) =
        current;

    let new_title = title.unwrap_or(&cur_title);
    let new_desc = description.map(Some).unwrap_or(cur_desc.as_deref());
    let new_status = status.unwrap_or(&cur_status);
    let new_priority = priority.unwrap_or(cur_priority);
    // empty string clears the date, None keeps the current value
    let new_due = due_date
        .map(|s| if s.is_empty() { None } else { Some(s) })
        .unwrap_or(cur_due.as_deref());
    let new_end = end_date
        .map(|s| if s.is_empty() { None } else { Some(s) })
        .unwrap_or(cur_end.as_deref());
    // empty string clears the reminder, None keeps the current value
    let new_reminder = match reminder {
        Some(value) => parse_reminder(value)?,
        None => cur_reminder.clone(),
    };
    let reminder_changed = new_reminder != cur_reminder;

    // empty string clears the schedule, None keeps the current value
    let new_rule = match repeat {
        Some(value) => {
            let rule = recurrence::normalize(value)?;
            (!rule.is_empty()).then_some(rule)
        }
        None => cur_rule.clone(),
    };
    let rule_changed = new_rule != cur_rule;

    let now = now_iso();
    let result = conn.execute(
        "UPDATE tasks SET title = ?1, description = ?2, status = ?3, priority = ?4, due_date = ?5, end_date = ?6, reminder_at = ?7, recurrence_rule = ?8, reminder_sent_at = CASE WHEN ?7 IS NOT ?9 THEN NULL ELSE reminder_sent_at END, updated_at = ?10 WHERE id = ?11",
        params![new_title, new_desc, new_status, new_priority, new_due, new_end, new_reminder, new_rule, cur_reminder, now, id],
    ).map_err(|e| format!("Failed to update task: {}", e))?;

    if result == 0 {
        return Err(format!("Task {} not found", id));
    }

    // parent and estimate are separate statements: empty string clears the
    // value, None keeps it. a cycle check guards the parent chain
    if let Some(p) = parent {
        let new_parent: Option<String> = if p.is_empty() {
            None
        } else {
            let pid = resolve_task_id(conn, p)?;
            // walk ancestors of the candidate; hitting our own id means a cycle
            let mut cursor = Some(pid.clone());
            for _ in 0..100 {
                match cursor.as_deref() {
                    None => break,
                    Some(c) if c == id => {
                        return Err("Cannot set parent: it would create a cycle".to_string());
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
            Some(pid)
        };
        conn.execute(
            "UPDATE tasks SET parent_id = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_parent, now, id],
        )
        .map_err(|e| format!("Failed to update task: {}", e))?;
        log_activity(conn, &id, "parent_changed", Some("parent_id"), None, new_parent.as_deref(), "cli")?;
    }
    if let Some(e) = estimate {
        let minutes = parse_estimate(e)?;
        conn.execute(
            "UPDATE tasks SET estimate_minutes = ?1, updated_at = ?2 WHERE id = ?3",
            params![minutes, now, id],
        )
        .map_err(|e| format!("Failed to update task: {}", e))?;
        log_activity(
            conn,
            &id,
            "estimate_changed",
            Some("estimate_minutes"),
            None,
            minutes.map(|m| m.to_string()).as_deref(),
            "cli",
        )?;
    }
    if new_status == "done" || new_status == "canceled" {
        bank_terminal_timers(conn, &[id.clone()])?;
    }

    if new_status != cur_status {
        log_activity(
            conn,
            &id,
            "status_changed",
            Some("status"),
            Some(status_label(&cur_status)),
            Some(status_label(new_status)),
            "cli",
        )?;
    }
    if new_priority != cur_priority {
        log_activity(
            conn,
            &id,
            "priority_changed",
            Some("priority"),
            Some(priority_label(cur_priority)),
            Some(priority_label(new_priority)),
            "cli",
        )?;
    }
    if new_title != cur_title {
        log_activity(
            conn,
            &id,
            "title_changed",
            Some("title"),
            Some(&cur_title),
            Some(new_title),
            "cli",
        )?;
    }

    if reminder_changed {
        log_activity(
            conn,
            &id,
            "reminder_changed",
            Some("reminder_at"),
            cur_reminder.as_deref(),
            new_reminder.as_deref(),
            "cli",
        )?;
    }

    if rule_changed {
        let old_label = cur_rule.as_deref().and_then(recurrence::label);
        let new_label = new_rule.as_deref().and_then(recurrence::label);
        log_activity(
            conn,
            &id,
            "recurrence_changed",
            Some("recurrence_rule"),
            old_label.as_deref(),
            new_label.as_deref(),
            "cli",
        )?;
    }

    // completing a recurring task rolls the series forward
    if cur_status != "done" && new_status == "done" {
        spawn_occurrence(conn, &id)?;
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "task_updated",
                "task": { "id": id, "recurrence_rule": new_rule }
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Updated task: {}", id);
    }
    Ok(())
}

// completing a recurring task creates its next occurrence and links it back
// through recurrence_parent_id; the same due date never spawns twice
fn spawn_occurrence(conn: &Connection, parent_id: &str) -> Result<Option<String>> {
    let parent = conn.query_row(
        "SELECT title, description, project_id, priority, due_date, end_date, reminder_at, recurrence_rule
         FROM tasks WHERE id = ?1",
        params![parent_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        },
    ).map_err(|_| format!("Task {} not found", parent_id))?;

    let (title, description, project_id, priority, due_date, end_date, reminder_at, rule) = parent;
    let Some(rule) = rule else {
        return Ok(None);
    };

    let today = Local::now().date_naive();
    let Some((next_date, delta_days)) = recurrence::next_due(&rule, due_date.as_deref(), today)
    else {
        return Ok(None);
    };
    let next_due = next_date.format("%Y-%m-%d").to_string();

    let existing: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE recurrence_parent_id = ?1 AND due_date = ?2 AND deleted_at IS NULL",
            params![parent_id, next_due],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if existing > 0 {
        return Ok(None);
    }

    let id = new_id();
    let now = now_iso();
    let number: i32 = if let Some(ref pid) = project_id {
        conn.query_row(
            "SELECT COALESCE(MAX(number), 0) + 1 FROM tasks WHERE project_id = ?1",
            params![pid],
            |row| row.get(0),
        )
    } else {
        conn.query_row(
            "SELECT COALESCE(MAX(number), 0) + 1 FROM tasks WHERE project_id IS NULL",
            [],
            |row| row.get(0),
        )
    }
    .map_err(|e| format!("Failed to assign task number: {}", e))?;

    let next_end = end_date.as_deref().and_then(|value| {
        recurrence::parse_date(value).map(|date| {
            (date + chrono::Duration::days(delta_days))
                .format("%Y-%m-%d")
                .to_string()
        })
    });
    let next_reminder = recurrence::shift_reminder(reminder_at.as_deref(), delta_days);

    conn.execute(
        "INSERT INTO tasks (id, number, project_id, title, description, status, priority, due_date, end_date, reminder_at, recurrence_rule, recurrence_parent_id, sort_order, pinned, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'todo', ?6, ?7, ?8, ?9, ?10, ?11, 0, 0, ?12, ?13)",
        params![id, number, project_id, title, description, priority, next_due, next_end, next_reminder, rule, parent_id, now, now],
    )
    .map_err(|e| format!("Failed to create next occurrence: {}", e))?;

    conn.execute(
        "INSERT INTO task_labels (task_id, label_id) SELECT ?1, label_id FROM task_labels WHERE task_id = ?2",
        params![id, parent_id],
    )
    .map_err(|e| format!("Failed to copy labels: {}", e))?;

    log_activity(conn, &id, "created", None, None, None, "cli")?;
    log_activity(
        conn,
        parent_id,
        "recurrence_spawned",
        Some("due_date"),
        due_date.as_deref(),
        Some(&next_due),
        "cli",
    )?;

    Ok(Some(id))
}

pub fn delete(conn: &Connection, json: bool, id: &str) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let now = now_iso();
    let result = conn.execute(
        "UPDATE tasks SET deleted_at = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
        params![now, now, id],
    ).map_err(|e| format!("Failed to delete task: {}", e))?;
    if result == 0 {
        return Err(format!("Task {} not found or already trashed", id));
    }
    log_activity(conn, &id, "trashed", None, None, None, "cli")?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "task_trashed",
                "task": { "id": id }
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Moved to trash: {}", id);
    }
    Ok(())
}

pub fn duplicate(conn: &Connection, json: bool, id: &str) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let original = conn.query_row(
        "SELECT title, description, status, priority, project_id, due_date, end_date, reminder_at, recurrence_rule FROM tasks WHERE id = ?1",
        params![id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
            ))
        },
    ).map_err(|_| format!("Task {} not found", id))?;

    let (title, desc, status, priority, project_id, due_date, end_date, reminder, rule) = original;
    let copy_title = format!("{} (copy)", title);

    create(
        conn,
        json,
        &copy_title,
        project_id.as_deref(),
        None,
        Some(&status),
        Some(priority),
        due_date.as_deref(),
        end_date.as_deref(),
        desc.as_deref(),
        reminder.as_deref(),
        rule.as_deref(),
        None,
        None,
    )?;
    Ok(())
}

pub fn toggle_pin(conn: &Connection, json: bool, id: &str, pin: bool) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let now = now_iso();
    let result = conn
        .execute(
            "UPDATE tasks SET pinned = ?1, updated_at = ?2 WHERE id = ?3",
            params![if pin { 1 } else { 0 }, now, id],
        )
        .map_err(|e| format!("Failed to pin task: {}", e))?;

    if result == 0 {
        return Err(format!("Task {} not found", id));
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": if pin { "task_pinned" } else { "task_unpinned" },
                "task": { "id": id, "pinned": pin }
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("{}", if pin { "Pinned" } else { "Unpinned" });
    }
    Ok(())
}

pub fn move_to_project(
    conn: &Connection,
    json: bool,
    id: &str,
    project: Option<&str>,
    project_prefix: Option<&str>,
) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let project_id = resolve_project_id(conn, project, project_prefix)?;
    let now = now_iso();
    let result = conn
        .execute(
            "UPDATE tasks SET project_id = ?1, updated_at = ?2 WHERE id = ?3",
            params![project_id, now, id],
        )
        .map_err(|e| format!("Failed to move task: {}", e))?;

    if result == 0 {
        return Err(format!("Task {} not found", id));
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "task_moved",
                "task": { "id": id, "project_id": project_id }
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!(
            "Moved task {} to project: {}",
            id,
            project_id.as_deref().unwrap_or("(none)")
        );
    }
    Ok(())
}

pub fn bulk_delete(conn: &Connection, json: bool, ids: &[String]) -> Result<()> {
    if ids.is_empty() {
        return Err("No task IDs provided".to_string());
    }
    let now = now_iso();
    let placeholders = ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 3))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "UPDATE tasks SET deleted_at = ?1, updated_at = ?2 WHERE id IN ({}) AND deleted_at IS NULL",
        placeholders
    );

    let mut args: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now.clone()), Box::new(now)];
    for id in ids {
        args.push(Box::new(id.clone()));
    }
    let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();

    let count = conn
        .execute(&sql, &arg_refs[..])
        .map_err(|e| format!("Failed to delete tasks: {}", e))?;

    for id in ids {
        let _ = log_activity(conn, &id, "trashed", None, None, None, "cli");
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "tasks_bulk_trashed",
                "count": count
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Moved {} task(s) to trash", count);
    }
    Ok(())
}

pub fn bulk_status(conn: &Connection, json: bool, ids: &[String], status: &str) -> Result<()> {
    if ids.is_empty() {
        return Err("No task IDs provided".to_string());
    }
    let now = now_iso();
    let placeholders = ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 3))
        .collect::<Vec<_>>()
        .join(", ");
    // completing recurring tasks rolls each series forward; read them before
    // the update clears their old status
    let recurring: Vec<String> = if status == "done" {
        let sql = format!(
            "SELECT id FROM tasks WHERE id IN ({}) AND status != 'done' AND recurrence_rule IS NOT NULL AND deleted_at IS NULL",
            ids.iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 1))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("Failed to read recurring tasks: {}", e))?;
        let arg_refs: Vec<&dyn rusqlite::ToSql> =
            ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        let rows = stmt
            .query_map(&arg_refs[..], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Failed to read recurring tasks: {}", e))?
            .filter_map(|r| r.ok())
            .collect();
        rows
    } else {
        Vec::new()
    };
    let sql = format!(
        "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id IN ({})",
        placeholders
    );

    let mut args: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(status.to_string()), Box::new(now)];
    for id in ids {
        args.push(Box::new(id.clone()));
    }
    let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();

    let count = conn
        .execute(&sql, &arg_refs[..])
        .map_err(|e| format!("Failed to update status: {}", e))?;

    if status == "done" || status == "canceled" {
        bank_terminal_timers(conn, ids)?;
    }

    for id in ids {
        let _ = log_activity(
            conn,
            &id,
            "status_changed",
            Some("status"),
            None,
            Some(status_label(status)),
            "cli",
        );
    }
    for id in &recurring {
        spawn_occurrence(conn, id)?;
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "tasks_bulk_status",
                "count": count,
                "status": status
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Updated {} task(s) to {}", count, status_label(status));
    }
    Ok(())
}

pub fn bulk_priority(conn: &Connection, json: bool, ids: &[String], priority: i32) -> Result<()> {
    if ids.is_empty() {
        return Err("No task IDs provided".to_string());
    }
    let now = now_iso();
    let placeholders = ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 3))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "UPDATE tasks SET priority = ?1, updated_at = ?2 WHERE id IN ({})",
        placeholders
    );

    let mut args: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(priority), Box::new(now)];
    for id in ids {
        args.push(Box::new(id.clone()));
    }
    let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();

    let count = conn
        .execute(&sql, &arg_refs[..])
        .map_err(|e| format!("Failed to update priority: {}", e))?;

    for id in ids {
        let _ = log_activity(
            conn,
            &id,
            "priority_changed",
            Some("priority"),
            None,
            Some(priority_label(priority)),
            "cli",
        );
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "tasks_bulk_priority",
                "count": count,
                "priority": priority
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Updated {} task(s) to {}", count, priority_label(priority));
    }
    Ok(())
}

pub fn bulk_move(
    conn: &Connection,
    json: bool,
    ids: &[String],
    project: Option<&str>,
    project_prefix: Option<&str>,
) -> Result<()> {
    if ids.is_empty() {
        return Err("No task IDs provided".to_string());
    }
    let project_id = resolve_project_id(conn, project, project_prefix)?;
    let project_id_json = project_id.clone();
    let now = now_iso();
    let placeholders = ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 3))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "UPDATE tasks SET project_id = ?1, updated_at = ?2 WHERE id IN ({})",
        placeholders
    );

    let mut args: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(project_id), Box::new(now)];
    for id in ids {
        args.push(Box::new(id.clone()));
    }
    let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();

    let count = conn
        .execute(&sql, &arg_refs[..])
        .map_err(|e| format!("Failed to move tasks: {}", e))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "tasks_bulk_moved",
                "count": count,
                "project_id": project_id_json
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Moved {} task(s)", count);
    }
    Ok(())
}

pub fn bulk_label(
    conn: &Connection,
    json: bool,
    ids: &[String],
    label_id: &str,
    remove: bool,
) -> Result<()> {
    if ids.is_empty() {
        return Err("No task IDs provided".to_string());
    }
    let label_name: String = conn
        .query_row(
            "SELECT name FROM labels WHERE id = ?1",
            params![label_id],
            |row| row.get(0),
        )
        .map_err(|_| format!("Label not found: {}", label_id))?;

    // add inserts per task so ignored duplicates don't inflate the count;
    // remove clears the label from every listed task in one statement
    let count = if remove {
        let placeholders = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 2))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "DELETE FROM task_labels WHERE label_id = ?1 AND task_id IN ({})",
            placeholders
        );
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(label_id.to_string())];
        for id in ids {
            args.push(Box::new(id.clone()));
        }
        let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();
        conn.execute(&sql, &arg_refs[..])
            .map_err(|e| format!("Failed to remove label: {}", e))?
    } else {
        let mut inserted = 0;
        for id in ids {
            inserted += conn
                .execute(
                    "INSERT OR IGNORE INTO task_labels (task_id, label_id) VALUES (?1, ?2)",
                    params![id, label_id],
                )
                .map_err(|e| format!("Failed to assign label: {}", e))?;
        }
        inserted
    };

    let action = if remove {
        "label_removed"
    } else {
        "label_added"
    };
    for id in ids {
        let _ = log_activity(
            conn,
            id,
            action,
            None,
            if remove { Some(&label_name) } else { None },
            if remove { None } else { Some(&label_name) },
            "cli",
        );
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "tasks_bulk_labeled",
                "count": count,
                "label_id": label_id,
                "removed": remove
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!(
            "{} label {} {} task(s)",
            if remove { "Removed" } else { "Added" },
            label_name,
            count
        );
    }
    Ok(())
}

pub fn trash_list(conn: &Connection, json: bool) -> Result<()> {
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.number, t.title, t.status, t.priority, t.deleted_at, p.prefix
         FROM tasks t
         LEFT JOIN projects p ON t.project_id = p.id
         WHERE t.deleted_at IS NOT NULL
         ORDER BY t.deleted_at DESC",
        )
        .map_err(|e| format!("Failed to query trashed tasks: {}", e))?;

    let rows: Vec<Vec<String>> = stmt
        .query_map([], |row| {
            let prefix: Option<String> = row.get(6)?;
            let number: i32 = row.get(1)?;
            let display_number = match &prefix {
                Some(p) => format!("{}-{}", p, number),
                None => format!("#{}", number),
            };
            let deleted: Option<String> = row.get(5)?;
            Ok(vec![
                row.get::<_, String>(0)?,
                display_number,
                row.get::<_, String>(2)?,
                status_label(&row.get::<_, String>(3)?).to_string(),
                priority_label(row.get::<_, i32>(4).unwrap_or(0)).to_string(),
                deleted.unwrap_or("-".to_string()),
            ])
        })
        .map_err(|e| format!("Failed to query trashed tasks: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    if json {
        let items: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                json!({
                    "id": r[0],
                    "number": r[1],
                    "title": r[2],
                    "status": r[3],
                    "priority": r[4],
                    "deleted_at": r[5],
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "trashed": items })).map_err(|e| e.to_string())?
        );
    } else {
        print_table(
            &["ID", "NUMBER", "TITLE", "STATUS", "PRIORITY", "DELETED AT"],
            &rows,
        );
    }
    Ok(())
}

pub fn restore(conn: &Connection, json: bool, id: &str) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let now = now_iso();
    let result = conn.execute(
        "UPDATE tasks SET deleted_at = NULL, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NOT NULL",
        params![now, id],
    ).map_err(|e| format!("Failed to restore task: {}", e))?;

    if result == 0 {
        return Err(format!("Task {} not found in trash", id));
    }
    log_activity(conn, &id, "restored", None, None, None, "cli")?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "task_restored",
                "task": { "id": id }
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Restored task: {}", id);
    }
    Ok(())
}

pub fn permanent_delete(conn: &Connection, json: bool, id: &str) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let result = conn
        .execute(
            "DELETE FROM tasks WHERE id = ?1 AND deleted_at IS NOT NULL",
            params![id],
        )
        .map_err(|e| format!("Failed to delete task: {}", e))?;

    if result == 0 {
        return Err(format!("Task {} not found in trash", id));
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "task_permanently_deleted",
                "task": { "id": id }
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Permanently deleted: {}", id);
    }
    Ok(())
}

pub fn empty_trash(conn: &Connection, json: bool) -> Result<()> {
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE deleted_at IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    conn.execute("DELETE FROM tasks WHERE deleted_at IS NOT NULL", [])
        .map_err(|e| format!("Failed to empty trash: {}", e))?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "trash_emptied",
                "count": count
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Permanently deleted {} task(s) from trash", count);
    }
    Ok(())
}

// ---- time tracking ----

// one running timer at a time: starting banks any other timer's elapsed
pub fn timer_start(conn: &Connection, json: bool, id: &str) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let now = now_iso();
    conn.execute(
        "UPDATE tasks SET tracked_seconds = tracked_seconds + MAX(0, CAST((julianday(?1) - julianday(timer_started_at)) * 86400 AS INTEGER)), timer_started_at = NULL, updated_at = ?1 WHERE timer_started_at IS NOT NULL",
        params![now],
    )
    .map_err(|e| format!("Failed to stop other timers: {}", e))?;
    conn.execute(
        "UPDATE tasks SET timer_started_at = ?1, updated_at = ?1 WHERE id = ?2",
        params![now, id],
    )
    .map_err(|e| format!("Failed to start timer: {}", e))?;
    log_activity(conn, &id, "timer_started", None, None, None, "cli")?;
    if json {
        println!("{}", json!({ "success": true, "action": "timer_started", "id": id }));
    } else {
        println!("Started timer on {}", id);
    }
    Ok(())
}

pub fn timer_stop(conn: &Connection, json: bool, id: &str) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let now = now_iso();
    conn.execute(
        "UPDATE tasks SET tracked_seconds = tracked_seconds + MAX(0, CAST((julianday(?1) - julianday(timer_started_at)) * 86400 AS INTEGER)), timer_started_at = NULL, updated_at = ?1 WHERE id = ?2 AND timer_started_at IS NOT NULL",
        params![now, id],
    )
    .map_err(|e| format!("Failed to stop timer: {}", e))?;
    log_activity(conn, &id, "timer_stopped", None, None, None, "cli")?;
    if json {
        println!("{}", json!({ "success": true, "action": "timer_stopped", "id": id }));
    } else {
        println!("Stopped timer on {}", id);
    }
    Ok(())
}

pub fn children(conn: &Connection, json: bool, id: &str) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.number, t.title, t.status, t.priority, t.due_date, t.end_date, t.pinned, p.prefix, t.updated_at, t.created_at, t.recurrence_rule
             FROM tasks t LEFT JOIN projects p ON t.project_id = p.id
             WHERE t.parent_id = ?1 AND t.deleted_at IS NULL
             ORDER BY t.sort_order, t.created_at",
        )
        .map_err(|e| e.to_string())?;
    let rows: Vec<TaskRow> = stmt
        .query_map(params![id], map_task_row)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    render_task_rows(json, &rows)
}

// ---- relations ----

pub fn relation_add(
    conn: &Connection,
    json: bool,
    id: &str,
    other: &str,
    kind: &str,
) -> Result<()> {
    if !["blocks", "relates", "duplicates"].contains(&kind) {
        return Err("Relation type must be: blocks, relates or duplicates".to_string());
    }
    let id = resolve_task_id(conn, id)?;
    let other_id = resolve_task_id(conn, other)?;
    if id == other_id {
        return Err("A task cannot relate to itself".to_string());
    }
    conn.execute(
        "INSERT OR IGNORE INTO task_relations (task_id, related_task_id, type, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, other_id, kind, now_iso()],
    )
    .map_err(|e| format!("Failed to add relation: {}", e))?;
    log_activity(conn, &id, "relation_added", Some("relation"), None, Some(&format!("{}:{}", kind, other_id)), "cli")?;
    if json {
        println!("{}", json!({ "success": true, "action": "relation_added", "task_id": id, "related_task_id": other_id, "type": kind }));
    } else {
        println!("Added '{}' relation {} -> {}", kind, id, other_id);
    }
    Ok(())
}

pub fn relation_remove(
    conn: &Connection,
    json: bool,
    id: &str,
    other: &str,
    kind: &str,
) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let other_id = resolve_task_id(conn, other)?;
    conn.execute(
        "DELETE FROM task_relations WHERE type = ?3 AND ((task_id = ?1 AND related_task_id = ?2) OR (task_id = ?2 AND related_task_id = ?1))",
        params![id, other_id, kind],
    )
    .map_err(|e| format!("Failed to remove relation: {}", e))?;
    log_activity(conn, &id, "relation_removed", Some("relation"), Some(&format!("{}:{}", kind, other_id)), None, "cli")?;
    if json {
        println!("{}", json!({ "success": true, "action": "relation_removed" }));
    } else {
        println!("Removed '{}' relation between {} and {}", kind, id, other_id);
    }
    Ok(())
}

pub fn relation_list(conn: &Connection, json: bool, id: &str) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let mut stmt = conn
        .prepare(
            "SELECT r.type, 'out' AS direction, o.number, o.title, p.prefix
             FROM task_relations r
             JOIN tasks o ON o.id = r.related_task_id
             LEFT JOIN projects p ON p.id = o.project_id
             WHERE r.task_id = ?1 AND o.deleted_at IS NULL
             UNION ALL
             SELECT r.type, 'in', o.number, o.title, p.prefix
             FROM task_relations r
             JOIN tasks o ON o.id = r.task_id
             LEFT JOIN projects p ON p.id = o.project_id
             WHERE r.related_task_id = ?1 AND o.deleted_at IS NULL",
        )
        .map_err(|e| e.to_string())?;
    let rows: Vec<(String, String, i32, String, Option<String>)> = stmt
        .query_map(params![id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    if json {
        let items: Vec<_> = rows
            .iter()
            .map(|(t, d, n, title, pfx)| {
                json!({ "type": t, "direction": d, "number": n, "title": title, "prefix": pfx })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items).map_err(|e| e.to_string())?);
        return Ok(());
    }
    if rows.is_empty() {
        println!("No relations");
        return Ok(());
    }
    for (kind, direction, number, title, pfx) in rows {
        let arrow = if direction == "out" { "->" } else { "<-" };
        println!(
            "{} {} {}-{}  {}",
            kind,
            arrow,
            pfx.unwrap_or_else(|| "TSK".to_string()),
            number,
            title
        );
    }
    Ok(())
}

// a terminal status must not leave a timer running: bank the elapsed segment
// and clear the start marker. runs after the status write so the stored
// status is the source of truth
fn bank_terminal_timers(conn: &Connection, ids: &[String]) -> Result<()> {
    let now = now_iso();
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "UPDATE tasks SET tracked_seconds = tracked_seconds + MAX(0, CAST((julianday(?1) - julianday(timer_started_at)) * 86400 AS INTEGER)), timer_started_at = NULL WHERE timer_started_at IS NOT NULL AND status IN ('done', 'canceled') AND id IN ({})",
        placeholders
    );
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now)];
    for id in ids {
        args.push(Box::new(id.clone()));
    }
    let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();
    conn.execute(&sql, &arg_refs[..])
        .map_err(|e| format!("Failed to stop timers: {}", e))?;
    Ok(())
}

// tasks spanning today: an open task whose due/end window covers the current
// date; mirrors the app's today filter
pub fn today(conn: &Connection, json: bool) -> Result<()> {
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.number, t.title, t.status, t.priority, t.due_date, t.end_date, t.pinned, p.prefix, t.updated_at, t.created_at, t.recurrence_rule
             FROM tasks t LEFT JOIN projects p ON t.project_id = p.id
             WHERE t.deleted_at IS NULL AND t.status NOT IN ('done', 'canceled')
               AND (t.due_date IS NOT NULL OR t.end_date IS NOT NULL)
               AND (t.due_date IS NULL OR t.due_date <= date('now', 'localtime'))
               AND (t.end_date IS NULL OR t.end_date >= date('now', 'localtime'))
             ORDER BY t.pinned DESC, t.priority ASC, t.due_date ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows: Vec<TaskRow> = stmt
        .query_map([], map_task_row)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    render_task_rows(json, &rows)
}

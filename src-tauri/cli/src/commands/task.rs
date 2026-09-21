use crate::db::*;
use chrono::{Local, SecondsFormat, TimeZone, Utc};
use rusqlite::{params, Connection};
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
                ]
            })
            .collect();
        print_table(
            &[
                "ID", "NUMBER", "TITLE", "STATUS", "PRIORITY", "DUE DATE", "PINNED",
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
) -> Result<()> {
    let project_id = resolve_project_id(conn, project, project_prefix)?;
    let id = new_id();
    let now = now_iso();
    let reminder_at = match reminder {
        Some(value) => parse_reminder(value)?,
        None => None,
    };

    let final_status = status
        .map(|s| s.to_string())
        .unwrap_or_else(|| get_default_status(conn));
    let final_priority = priority.unwrap_or_else(|| get_default_priority(conn));

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

    conn.execute(
        "INSERT INTO tasks (id, number, project_id, title, description, status, priority, due_date, end_date, reminder_at, sort_order, pinned, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, 0, ?11, ?12)",
        params![id, number, project_id, title, description, final_status, final_priority, due_date, end_date, reminder_at, now, now],
    ).map_err(|e| format!("Failed to create task: {}", e))?;

    log_activity(conn, &id, "created", None, None, None, "cli")?;

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
        "SELECT t.id, t.number, t.title, t.status, t.priority, t.due_date, t.end_date, t.pinned, p.prefix, t.updated_at, t.created_at
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
    let match_query = fts_query(query);
    if match_query.is_empty() {
        return Err("Empty search query".to_string());
    }
    let project_id = resolve_project_id(conn, project, project_prefix)?;

    let mut sql = String::from(
        "SELECT t.id, t.number, t.title, t.status, t.priority, t.due_date, t.end_date, t.pinned, p.prefix, t.updated_at, t.created_at
         FROM tasks_fts
         JOIN tasks t ON t.id = tasks_fts.task_id
         LEFT JOIN projects p ON t.project_id = p.id
         WHERE tasks_fts MATCH ? AND t.deleted_at IS NULL",
    );
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(match_query)];

    if let Some(ref pid) = project_id {
        sql.push_str(" AND t.project_id = ?");
        args.push(Box::new(pid.clone()));
    }
    if let Some(s) = status {
        sql.push_str(" AND t.status = ?");
        args.push(Box::new(s.to_string()));
    }
    sql.push_str(" ORDER BY rank LIMIT ?");
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
                t.project_id, p.name, p.prefix, t.created_at, t.updated_at, t.reminder_at
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
                    "pinned": pinned == 1,
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
        println!("Pinned:      {}", if pinned == 1 { "yes" } else { "no" });
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
) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let current = conn.query_row(
        "SELECT title, description, status, priority, due_date, end_date, reminder_at FROM tasks WHERE id = ?1",
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
            ))
        },
    ).map_err(|_| format!("Task {} not found", id))?;

    let (cur_title, cur_desc, cur_status, cur_priority, cur_due, cur_end, cur_reminder) = current;

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

    let now = now_iso();
    let result = conn.execute(
        "UPDATE tasks SET title = ?1, description = ?2, status = ?3, priority = ?4, due_date = ?5, end_date = ?6, reminder_at = ?7, reminder_sent_at = CASE WHEN ?7 IS NOT ?8 THEN NULL ELSE reminder_sent_at END, updated_at = ?9 WHERE id = ?10",
        params![new_title, new_desc, new_status, new_priority, new_due, new_end, new_reminder, cur_reminder, now, id],
    ).map_err(|e| format!("Failed to update task: {}", e))?;

    if result == 0 {
        return Err(format!("Task {} not found", id));
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

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "task_updated",
                "task": { "id": id }
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Updated task: {}", id);
    }
    Ok(())
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
        "SELECT title, description, status, priority, project_id, due_date, end_date, reminder_at FROM tasks WHERE id = ?1",
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

    let (title, desc, status, priority, project_id, due_date, end_date, reminder) = original;
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

    let action = if remove { "label_removed" } else { "label_added" };
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

// insert a task without printing; returns the new id (used by note → task)
pub fn create_quiet(conn: &Connection, title: &str, description: &str) -> Result<String> {
    let id = new_id();
    let now = now_iso();
    let priority = get_default_priority(conn);
    let status = get_default_status(conn);
    let number: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(number), 0) + 1 FROM tasks WHERE project_id IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to assign task number: {}", e))?;
    conn.execute(
        "INSERT INTO tasks (id, number, project_id, title, description, status, priority, sort_order, pinned, created_at, updated_at)
         VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, 0, 0, ?7, ?7)",
        params![id, number, title, description, status, priority, now],
    )
    .map_err(|e| format!("Failed to create task: {}", e))?;
    Ok(id)
}

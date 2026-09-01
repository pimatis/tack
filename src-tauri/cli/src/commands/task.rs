use rusqlite::{params, Connection};
use crate::db::*;
use serde_json::json;

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
) -> Result<()> {
    let project_id = resolve_project_id(conn, project, project_prefix)?;
    let id = new_id();
    let now = now_iso();

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
        "INSERT INTO tasks (id, number, project_id, title, description, status, priority, due_date, end_date, sort_order, pinned, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, 0, ?10, ?11)",
        params![id, number, project_id, title, description, final_status, final_priority, due_date, end_date, now, now],
    ).map_err(|e| format!("Failed to create task: {}", e))?;

    log_activity(conn, &id, "created", None, None, None, "cli")?;

    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
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
                "project_id": project_id,
                "created_at": now,
            }
        })).map_err(|e| e.to_string())?);
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

    let mut stmt = conn.prepare(&sql).map_err(|e| format!("Failed to query tasks: {}", e))?;

    let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();
    let rows: Vec<(String, i32, String, String, i32, Option<String>, Option<String>, i32, Option<String>, String, String)> = stmt
        .query_map(&arg_refs[..], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i32>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, i32>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
            ))
        })
        .map_err(|e| format!("Failed to query tasks: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    if json {
        let items: Vec<serde_json::Value> = rows.iter().map(|r| {
            let prefix = &r.8;
            let number = r.1;
            let display_number = match prefix {
                Some(p) => format!("{}-{}", p, number),
                None => format!("#{}", number),
            };
            json!({
                "id": r.0,
                "number": number,
                "display_number": display_number,
                "title": r.2,
                "status": r.3,
                "priority": r.4,
                "due_date": r.5,
                "end_date": r.6,
                "pinned": r.7 == 1,
                "project_prefix": prefix,
                "updated_at": r.9,
                "created_at": r.10,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&json!({ "tasks": items }))
            .map_err(|e| e.to_string())?);
    } else {
        let table_rows: Vec<Vec<String>> = rows.iter().map(|r| {
            let prefix = &r.8;
            let display_number = match prefix {
                Some(p) => format!("{}-{}", p, r.1),
                None => format!("#{}", r.1),
            };
            vec![
                r.0.clone(),
                display_number,
                r.2.clone(),
                status_label(&r.3).to_string(),
                priority_label(r.4).to_string(),
                r.5.clone().unwrap_or("-".to_string()),
                if r.7 == 1 { "yes".to_string() } else { "-".to_string() },
            ]
        }).collect();
        print_table(&["ID", "NUMBER", "TITLE", "STATUS", "PRIORITY", "DUE DATE", "PINNED"], &table_rows);
    }
    Ok(())
}

pub fn show(conn: &Connection, json: bool, id: &str) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let task = conn.query_row(
        "SELECT t.id, t.number, t.title, t.description, t.status, t.priority, t.due_date, t.end_date, t.pinned,
                t.project_id, p.name, p.prefix, t.created_at, t.updated_at
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
            ))
        },
    ).map_err(|_| format!("Task {} not found", id))?;

    let (tid, number, title, desc, status, priority, due, end, pinned, _project_id, project_name, project_prefix, created, updated) = task;

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
        let subtask_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM subtasks WHERE task_id = ?1",
            params![id], |row| row.get(0),
        ).unwrap_or(0);
        let completed: i32 = conn.query_row(
            "SELECT COUNT(*) FROM subtasks WHERE task_id = ?1 AND completed = 1",
            params![id], |row| row.get(0),
        ).unwrap_or(0);

        println!("{}", serde_json::to_string_pretty(&json!({
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
        })).map_err(|e| e.to_string())?);
    } else {
        println!("ID:          {}", tid);
        println!("Number:      {}", display_number);
        println!("Title:       {}", title);
        println!("Description: {}", desc.unwrap_or("-".to_string()));
        println!("Status:      {}", status_label(&status));
        println!("Priority:    {}", priority_label(priority));
        println!("Due date:    {}", due.unwrap_or("-".to_string()));
        println!("End date:    {}", end.unwrap_or("-".to_string()));
        println!("Pinned:      {}", if pinned == 1 { "yes" } else { "no" });
        if let Some(pname) = project_name {
            println!("Project:     {} ({})", pname, project_prefix.unwrap_or_default());
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

        let subtask_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM subtasks WHERE task_id = ?1",
            params![id],
            |row| row.get(0),
        ).unwrap_or(0);
        if subtask_count > 0 {
            let completed: i32 = conn.query_row(
                "SELECT COUNT(*) FROM subtasks WHERE task_id = ?1 AND completed = 1",
                params![id],
                |row| row.get(0),
            ).unwrap_or(0);
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
) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let current = conn.query_row(
        "SELECT title, description, status, priority, due_date, end_date FROM tasks WHERE id = ?1",
        params![id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        },
    ).map_err(|_| format!("Task {} not found", id))?;

    let (cur_title, cur_desc, cur_status, cur_priority, cur_due, cur_end) = current;

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

    let now = now_iso();
    let result = conn.execute(
        "UPDATE tasks SET title = ?1, description = ?2, status = ?3, priority = ?4, due_date = ?5, end_date = ?6, updated_at = ?7 WHERE id = ?8",
        params![new_title, new_desc, new_status, new_priority, new_due, new_end, now, id],
    ).map_err(|e| format!("Failed to update task: {}", e))?;

    if result == 0 {
        return Err(format!("Task {} not found", id));
    }

    if new_status != cur_status {
        log_activity(conn, &id, "status_changed", Some("status"), Some(status_label(&cur_status)), Some(status_label(new_status)), "cli")?;
    }
    if new_priority != cur_priority {
        log_activity(conn, &id, "priority_changed", Some("priority"), Some(priority_label(cur_priority)), Some(priority_label(new_priority)), "cli")?;
    }
    if new_title != cur_title {
        log_activity(conn, &id, "title_changed", Some("title"), Some(&cur_title), Some(new_title), "cli")?;
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "task_updated",
            "task": { "id": id }
        })).map_err(|e| e.to_string())?);
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
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "task_trashed",
            "task": { "id": id }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Moved to trash: {}", id);
    }
    Ok(())
}

pub fn duplicate(conn: &Connection, json: bool, id: &str) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let original = conn.query_row(
        "SELECT title, description, status, priority, project_id, due_date, end_date FROM tasks WHERE id = ?1",
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

    let (title, desc, status, priority, project_id, due_date, end_date) = original;
    let copy_title = format!("{} (copy)", title);

    create(conn, json, &copy_title, project_id.as_deref(), None, Some(&status), Some(priority), due_date.as_deref(), end_date.as_deref(), desc.as_deref())?;
    Ok(())
}

pub fn toggle_pin(conn: &Connection, json: bool, id: &str, pin: bool) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let now = now_iso();
    let result = conn.execute(
        "UPDATE tasks SET pinned = ?1, updated_at = ?2 WHERE id = ?3",
        params![if pin { 1 } else { 0 }, now, id],
    ).map_err(|e| format!("Failed to pin task: {}", e))?;

    if result == 0 {
        return Err(format!("Task {} not found", id));
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": if pin { "task_pinned" } else { "task_unpinned" },
            "task": { "id": id, "pinned": pin }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("{}", if pin { "Pinned" } else { "Unpinned" });
    }
    Ok(())
}

pub fn move_to_project(conn: &Connection, json: bool, id: &str, project: Option<&str>, project_prefix: Option<&str>) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let project_id = resolve_project_id(conn, project, project_prefix)?;
    let now = now_iso();
    let result = conn.execute(
        "UPDATE tasks SET project_id = ?1, updated_at = ?2 WHERE id = ?3",
        params![project_id, now, id],
    ).map_err(|e| format!("Failed to move task: {}", e))?;

    if result == 0 {
        return Err(format!("Task {} not found", id));
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "task_moved",
            "task": { "id": id, "project_id": project_id }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Moved task {} to project: {}", id, project_id.as_deref().unwrap_or("(none)"));
    }
    Ok(())
}

pub fn bulk_delete(conn: &Connection, json: bool, ids: &[String]) -> Result<()> {
    if ids.is_empty() {
        return Err("No task IDs provided".to_string());
    }
    let now = now_iso();
    let placeholders = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 3)).collect::<Vec<_>>().join(", ");
    let sql = format!("UPDATE tasks SET deleted_at = ?1, updated_at = ?2 WHERE id IN ({}) AND deleted_at IS NULL", placeholders);

    let mut args: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now.clone()), Box::new(now)];
    for id in ids {
        args.push(Box::new(id.clone()));
    }
    let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();

    let count = conn.execute(&sql, &arg_refs[..])
        .map_err(|e| format!("Failed to delete tasks: {}", e))?;

    for id in ids {
        let _ = log_activity(conn, &id, "trashed", None, None, None, "cli");
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "tasks_bulk_trashed",
            "count": count
        })).map_err(|e| e.to_string())?);
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
    let placeholders = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 3)).collect::<Vec<_>>().join(", ");
    let sql = format!("UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id IN ({})", placeholders);

    let mut args: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(status.to_string()), Box::new(now)];
    for id in ids {
        args.push(Box::new(id.clone()));
    }
    let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();

    let count = conn.execute(&sql, &arg_refs[..])
        .map_err(|e| format!("Failed to update status: {}", e))?;

    for id in ids {
        let _ = log_activity(conn, &id, "status_changed", Some("status"), None, Some(status_label(status)), "cli");
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "tasks_bulk_status",
            "count": count,
            "status": status
        })).map_err(|e| e.to_string())?);
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
    let placeholders = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 3)).collect::<Vec<_>>().join(", ");
    let sql = format!("UPDATE tasks SET priority = ?1, updated_at = ?2 WHERE id IN ({})", placeholders);

    let mut args: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(priority), Box::new(now)];
    for id in ids {
        args.push(Box::new(id.clone()));
    }
    let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();

    let count = conn.execute(&sql, &arg_refs[..])
        .map_err(|e| format!("Failed to update priority: {}", e))?;

    for id in ids {
        let _ = log_activity(conn, &id, "priority_changed", Some("priority"), None, Some(priority_label(priority)), "cli");
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "tasks_bulk_priority",
            "count": count,
            "priority": priority
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Updated {} task(s) to {}", count, priority_label(priority));
    }
    Ok(())
}

pub fn bulk_move(conn: &Connection, json: bool, ids: &[String], project: Option<&str>, project_prefix: Option<&str>) -> Result<()> {
    if ids.is_empty() {
        return Err("No task IDs provided".to_string());
    }
    let project_id = resolve_project_id(conn, project, project_prefix)?;
    let project_id_json = project_id.clone();
    let now = now_iso();
    let placeholders = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 3)).collect::<Vec<_>>().join(", ");
    let sql = format!("UPDATE tasks SET project_id = ?1, updated_at = ?2 WHERE id IN ({})", placeholders);

    let mut args: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(project_id), Box::new(now)];
    for id in ids {
        args.push(Box::new(id.clone()));
    }
    let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();

    let count = conn.execute(&sql, &arg_refs[..])
        .map_err(|e| format!("Failed to move tasks: {}", e))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "tasks_bulk_moved",
            "count": count,
            "project_id": project_id_json
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Moved {} task(s)", count);
    }
    Ok(())
}

pub fn trash_list(conn: &Connection, json: bool) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.number, t.title, t.status, t.priority, t.deleted_at, p.prefix
         FROM tasks t
         LEFT JOIN projects p ON t.project_id = p.id
         WHERE t.deleted_at IS NOT NULL
         ORDER BY t.deleted_at DESC"
    ).map_err(|e| format!("Failed to query trashed tasks: {}", e))?;

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
        let items: Vec<serde_json::Value> = rows.iter().map(|r| json!({
            "id": r[0],
            "number": r[1],
            "title": r[2],
            "status": r[3],
            "priority": r[4],
            "deleted_at": r[5],
        })).collect();
        println!("{}", serde_json::to_string_pretty(&json!({ "trashed": items }))
            .map_err(|e| e.to_string())?);
    } else {
        print_table(&["ID", "NUMBER", "TITLE", "STATUS", "PRIORITY", "DELETED AT"], &rows);
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
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "task_restored",
            "task": { "id": id }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Restored task: {}", id);
    }
    Ok(())
}

pub fn permanent_delete(conn: &Connection, json: bool, id: &str) -> Result<()> {
    let id = resolve_task_id(conn, id)?;
    let result = conn.execute(
        "DELETE FROM tasks WHERE id = ?1 AND deleted_at IS NOT NULL",
        params![id],
    ).map_err(|e| format!("Failed to delete task: {}", e))?;

    if result == 0 {
        return Err(format!("Task {} not found in trash", id));
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "task_permanently_deleted",
            "task": { "id": id }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Permanently deleted: {}", id);
    }
    Ok(())
}

pub fn empty_trash(conn: &Connection, json: bool) -> Result<()> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE deleted_at IS NOT NULL",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    conn.execute("DELETE FROM tasks WHERE deleted_at IS NOT NULL", [])
        .map_err(|e| format!("Failed to empty trash: {}", e))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "trash_emptied",
            "count": count
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Permanently deleted {} task(s) from trash", count);
    }
    Ok(())
}

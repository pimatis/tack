use rusqlite::{params, Connection};
use crate::db::*;
use serde_json::json;

pub fn add(conn: &Connection, json: bool, task_id: &str, title: &str) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?1)",
        params![task_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    if !exists {
        return Err(format!("Task {} not found", task_id));
    }

    let id = new_id();
    let now = now_iso();
    let sort_order: i32 = conn.query_row(
        "SELECT COUNT(*) FROM subtasks WHERE task_id = ?1",
        params![task_id],
        |row| row.get(0),
    ).unwrap_or(0);

    conn.execute(
        "INSERT INTO subtasks (id, task_id, title, completed, sort_order, created_at)
         VALUES (?1, ?2, ?3, 0, ?4, ?5)",
        params![id, task_id, title, sort_order, now],
    ).map_err(|e| format!("Failed to create subtask: {}", e))?;

    let _ = log_activity(conn, task_id, "subtask_added", None, None, Some(title), "cli");
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "subtask_added",
            "subtask": { "id": id, "task_id": task_id, "title": title, "completed": false, "sort_order": sort_order }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Created subtask: {}", title);
        println!("ID: {}", id);
    }
    Ok(())
}

pub fn list(conn: &Connection, json: bool, task_id: &str) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT id, title, completed, sort_order FROM subtasks WHERE task_id = ?1 ORDER BY sort_order ASC, created_at ASC")
        .map_err(|e| format!("Failed to query subtasks: {}", e))?;

    let rows: Vec<Vec<String>> = stmt
        .query_map(params![task_id], |row| {
            let completed: i32 = row.get(2)?;
            Ok(vec![
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                if completed == 1 { "x".to_string() } else { " ".to_string() },
            ])
        })
        .map_err(|e| format!("Failed to query subtasks: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    if json {
        let items: Vec<serde_json::Value> = rows.iter().map(|r| json!({
            "id": r[0],
            "title": r[1],
            "completed": r[2] == "x",
        })).collect();
        println!("{}", serde_json::to_string_pretty(&json!({ "subtasks": items }))
            .map_err(|e| e.to_string())?);
    } else {
        print_table(&["ID", "TITLE", "DONE"], &rows);
    }
    Ok(())
}

pub fn toggle(conn: &Connection, json: bool, id: &str) -> Result<()> {
    let (task_id, current): (String, i32) = conn.query_row(
        "SELECT task_id, completed FROM subtasks WHERE id = ?1",
        params![id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|_| format!("Subtask {} not found", id))?;

    let new_val = if current == 1 { 0 } else { 1 };
    conn.execute(
        "UPDATE subtasks SET completed = ?1 WHERE id = ?2",
        params![new_val, id],
    ).map_err(|e| format!("Failed to toggle subtask: {}", e))?;

    let action = if new_val == 1 { "subtask_completed" } else { "subtask_uncompleted" };
    let _ = log_activity(conn, &task_id, action, None, None, None, "cli");
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": action,
            "subtask": { "id": id, "completed": new_val == 1 }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("{}", if new_val == 1 { "Completed" } else { "Reopened" });
    }
    Ok(())
}

pub fn rename(conn: &Connection, json: bool, id: &str, title: &str) -> Result<()> {
    let result = conn.execute(
        "UPDATE subtasks SET title = ?1 WHERE id = ?2",
        params![title, id],
    ).map_err(|e| format!("Failed to rename subtask: {}", e))?;

    if result == 0 {
        return Err(format!("Subtask {} not found", id));
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "subtask_renamed",
            "subtask": { "id": id, "title": title }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Renamed subtask: {}", id);
    }
    Ok(())
}

pub fn delete(conn: &Connection, json: bool, id: &str) -> Result<()> {
    let task_id: String = conn.query_row(
        "SELECT task_id FROM subtasks WHERE id = ?1",
        params![id],
        |row| row.get(0),
    ).map_err(|_| format!("Subtask {} not found", id))?;

    let result = conn.execute("DELETE FROM subtasks WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete subtask: {}", e))?;

    if result == 0 {
        return Err(format!("Subtask {} not found", id));
    }
    let _ = log_activity(conn, &task_id, "subtask_removed", None, None, None, "cli");
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "subtask_deleted",
            "subtask": { "id": id }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Deleted subtask: {}", id);
    }
    Ok(())
}

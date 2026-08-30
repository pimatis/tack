use rusqlite::{params, Connection};
use crate::db::*;
use serde_json::json;

pub fn create(conn: &Connection, json: bool, name: &str, color: &str) -> Result<()> {
    let valid_colors = ["gray", "blue", "green", "amber", "red", "purple", "pink", "teal", "orange", "indigo"];
    if !valid_colors.contains(&color) {
        return Err(format!("Invalid color '{}'. Valid: {}", color, valid_colors.join(", ")));
    }

    let id = new_id();
    let now = now_iso();
    conn.execute(
        "INSERT INTO labels (id, name, color, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, name, color, now],
    ).map_err(|e| format!("Failed to create label: {}", e))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "label_created",
            "label": { "id": id, "name": name, "color": color, "created_at": now }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Created label: {} ({})", name, color);
        println!("ID: {}", id);
    }
    Ok(())
}

pub fn list(conn: &Connection, json: bool) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT id, name, color, created_at FROM labels ORDER BY name COLLATE NOCASE")
        .map_err(|e| format!("Failed to query labels: {}", e))?;

    let rows: Vec<Vec<String>> = stmt
        .query_map([], |row| {
            Ok(vec![
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ])
        })
        .map_err(|e| format!("Failed to query labels: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    if json {
        let items: Vec<serde_json::Value> = rows.iter().map(|r| json!({
            "id": r[0],
            "name": r[1],
            "color": r[2],
            "created_at": r[3],
        })).collect();
        println!("{}", serde_json::to_string_pretty(&json!({ "labels": items }))
            .map_err(|e| e.to_string())?);
    } else {
        print_table(&["ID", "NAME", "COLOR", "CREATED"], &rows);
    }
    Ok(())
}

pub fn update(conn: &Connection, json: bool, id: &str, name: Option<&str>, color: Option<&str>) -> Result<()> {
    if let Some(c) = color {
        let valid_colors = ["gray", "blue", "green", "amber", "red", "purple", "pink", "teal", "orange", "indigo"];
        if !valid_colors.contains(&c) {
            return Err(format!("Invalid color '{}'. Valid: {}", c, valid_colors.join(", ")));
        }
    }

    let mut assignments: Vec<String> = Vec::new();
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(n) = name {
        args.push(Box::new(n.to_string()));
        assignments.push(format!("name = ?{}", args.len()));
    }
    if let Some(c) = color {
        args.push(Box::new(c.to_string()));
        assignments.push(format!("color = ?{}", args.len()));
    }

    if assignments.is_empty() {
        if json {
            println!("{}", serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "label_unchanged",
                "label": { "id": id }
            })).map_err(|e| e.to_string())?);
        } else {
            println!("Nothing to update");
        }
        return Ok(());
    }

    args.push(Box::new(id.to_string()));
    let sql = format!("UPDATE labels SET {} WHERE id = ?", assignments.join(", "));
    let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();

    let result = conn.execute(&sql, &arg_refs[..])
        .map_err(|e| format!("Failed to update label: {}", e))?;

    if result == 0 {
        return Err(format!("Label {} not found", id));
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "label_updated",
            "label": { "id": id, "name": name, "color": color }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Updated label: {}", id);
    }
    Ok(())
}

pub fn delete(conn: &Connection, json: bool, id: &str) -> Result<()> {
    let result = conn.execute("DELETE FROM labels WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete label: {}", e))?;
    if result == 0 {
        return Err(format!("Label {} not found", id));
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "label_deleted",
            "label": { "id": id }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Deleted label: {}", id);
    }
    Ok(())
}

pub fn assign(conn: &Connection, json: bool, task_id: &str, label_ids: &[String]) -> Result<()> {
    conn.execute("DELETE FROM task_labels WHERE task_id = ?1", params![task_id])
        .map_err(|e| format!("Failed to clear labels: {}", e))?;

    for label_id in label_ids {
        conn.execute(
            "INSERT OR IGNORE INTO task_labels (task_id, label_id) VALUES (?1, ?2)",
            params![task_id, label_id],
        ).map_err(|e| format!("Failed to assign label: {}", e))?;
    }

    for label_id in label_ids {
        let label_name: String = conn.query_row(
            "SELECT name FROM labels WHERE id = ?1",
            params![label_id],
            |row| row.get(0),
        ).unwrap_or_else(|_| label_id.clone());
        let _ = log_activity(conn, task_id, "label_added", None, None, Some(&label_name), "cli");
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "labels_assigned",
            "task_id": task_id,
            "label_ids": label_ids,
            "count": label_ids.len()
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Assigned {} label(s) to task {}", label_ids.len(), task_id);
    }
    Ok(())
}

pub fn show(conn: &Connection, json: bool, task_id: &str) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT l.id, l.name, l.color FROM task_labels tl JOIN labels l ON tl.label_id = l.id WHERE tl.task_id = ?1 ORDER BY l.name COLLATE NOCASE")
        .map_err(|e| format!("Failed to query labels: {}", e))?;

    let rows: Vec<Vec<String>> = stmt
        .query_map(params![task_id], |row| {
            Ok(vec![
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ])
        })
        .map_err(|e| format!("Failed to query labels: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    if json {
        let items: Vec<serde_json::Value> = rows.iter().map(|r| json!({
            "id": r[0],
            "name": r[1],
            "color": r[2],
        })).collect();
        println!("{}", serde_json::to_string_pretty(&json!({ "task_id": task_id, "labels": items }))
            .map_err(|e| e.to_string())?);
    } else {
        print_table(&["ID", "NAME", "COLOR"], &rows);
    }
    Ok(())
}

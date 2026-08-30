use rusqlite::{params, Connection};
use crate::db::*;
use serde_json::json;

pub fn create(conn: &Connection, json: bool, name: &str, prefix: &str, description: Option<&str>) -> Result<()> {
    let id = new_id();
    let now = now_iso();
    conn.execute(
        "INSERT INTO projects (id, name, prefix, description, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, name, prefix, description, now, now],
    ).map_err(|e| format!("Failed to create project: {}", e))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "project_created",
            "project": { "id": id, "name": name, "prefix": prefix, "description": description, "created_at": now }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Created project: {} ({})", name, prefix);
        println!("ID: {}", id);
    }
    Ok(())
}

pub fn list(conn: &Connection, json: bool) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT id, name, prefix, description, created_at FROM projects ORDER BY name COLLATE NOCASE")
        .map_err(|e| format!("Failed to query projects: {}", e))?;

    let rows: Vec<Vec<String>> = stmt
        .query_map([], |row| {
            Ok(vec![
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?.unwrap_or("-".to_string()),
                row.get::<_, String>(4)?,
            ])
        })
        .map_err(|e| format!("Failed to query projects: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    if json {
        let items: Vec<serde_json::Value> = rows.iter().map(|r| json!({
            "id": r[0],
            "name": r[1],
            "prefix": r[2],
            "description": if r[3] == "-" { serde_json::Value::Null } else { json!(r[3]) },
            "created_at": r[4],
        })).collect();
        println!("{}", serde_json::to_string_pretty(&json!({ "projects": items }))
            .map_err(|e| e.to_string())?);
    } else {
        print_table(&["ID", "NAME", "PREFIX", "DESCRIPTION", "CREATED"], &rows);
    }
    Ok(())
}

pub fn update(conn: &Connection, json: bool, id: &str, name: &str, prefix: &str, description: Option<&str>) -> Result<()> {
    let now = now_iso();
    let result = conn.execute(
        "UPDATE projects SET name = ?1, prefix = ?2, description = ?3, updated_at = ?4 WHERE id = ?5",
        params![name, prefix, description, now, id],
    ).map_err(|e| format!("Failed to update project: {}", e))?;

    if result == 0 {
        return Err(format!("Project {} not found", id));
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "project_updated",
            "project": { "id": id, "name": name, "prefix": prefix, "description": description }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Updated project: {}", id);
    }
    Ok(())
}

pub fn delete(conn: &Connection, json: bool, id: &str) -> Result<()> {
    conn.execute("DELETE FROM tasks WHERE project_id = ?1", params![id])
        .map_err(|e| format!("Failed to delete project tasks: {}", e))?;
    let result = conn.execute("DELETE FROM projects WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete project: {}", e))?;

    if result == 0 {
        return Err(format!("Project {} not found", id));
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "project_deleted",
            "project": { "id": id }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Deleted project: {}", id);
    }
    Ok(())
}

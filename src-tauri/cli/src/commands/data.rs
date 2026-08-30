use rusqlite::{params, Connection};
use crate::db::*;
use crate::backup;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
struct ExportData {
    projects: Vec<HashMap<String, serde_json::Value>>,
    tasks: Vec<HashMap<String, serde_json::Value>>,
    labels: Vec<HashMap<String, serde_json::Value>>,
    taskLabels: Vec<HashMap<String, serde_json::Value>>,
    subtasks: Vec<HashMap<String, serde_json::Value>>,
    attachments: Vec<HashMap<String, serde_json::Value>>,
    activityLog: Vec<HashMap<String, serde_json::Value>>,
    exportedAt: String,
}

fn query_all(conn: &Connection, table: &str, columns: &str) -> Result<Vec<HashMap<String, serde_json::Value>>> {
    let sql = format!("SELECT {} FROM {}", columns, table);
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let col_count = stmt.column_count();

    let col_names: Vec<String> = (0..col_count)
        .map(|i| stmt.column_name(i).unwrap_or("").to_string())
        .collect();

    let rows: Vec<HashMap<String, serde_json::Value>> = stmt
        .query_map([], |row| {
            let mut map = HashMap::new();
            for i in 0..col_count {
                let col_name = &col_names[i];
                let val: rusqlite::types::Value = row.get(i)?;
                let json_val = match val {
                    rusqlite::types::Value::Null => serde_json::Value::Null,
                    rusqlite::types::Value::Integer(n) => serde_json::Value::Number(n.into()),
                    rusqlite::types::Value::Real(f) => serde_json::json!(f),
                    rusqlite::types::Value::Text(s) => serde_json::Value::String(s),
                    rusqlite::types::Value::Blob(b) => serde_json::Value::String(String::from_utf8_lossy(&b).to_string()),
                };
                map.insert(col_name.clone(), json_val);
            }
            Ok(map)
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rows)
}

pub fn export(conn: &Connection, json: bool, output_path: &str) -> Result<()> {
    let data = ExportData {
        projects: query_all(conn, "projects", "*")?,
        tasks: query_all(conn, "tasks", "*")?,
        labels: query_all(conn, "labels", "*")?,
        taskLabels: query_all(conn, "task_labels", "*")?,
        subtasks: query_all(conn, "subtasks", "*")?,
        attachments: query_all(conn, "task_attachments", "id, task_id, file_name, mime_type, file_size, created_at")?,
        activityLog: query_all(conn, "activity_log", "*")?,
        exportedAt: now_iso(),
    };

    let json_str = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("Failed to serialize: {}", e))?;

    if json {
        println!("{}", json_str);
    } else {
        let path = if output_path.is_empty() { "tack-export.json" } else { output_path };
        std::fs::write(path, json_str)
            .map_err(|e| format!("Failed to write file: {}", e))?;
        let total = data.projects.len() + data.tasks.len() + data.labels.len() + data.subtasks.len() + data.attachments.len() + data.activityLog.len();
        println!("Exported {} records to {}", total, path);
    }
    Ok(())
}

pub fn import(conn: &Connection, json: bool, file_path: &str) -> Result<()> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    let data: ExportData = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    reset(conn, json)?;

    for p in &data.projects {
        conn.execute(
            "INSERT INTO projects (id, name, prefix, description, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                get_str(p, "id"), get_str(p, "name"), get_str(p, "prefix"),
                get_opt_str(p, "description"), get_str(p, "created_at"), get_str(p, "updated_at"),
            ],
        ).map_err(|e| format!("Failed to import project: {}", e))?;
    }

    for t in &data.tasks {
        conn.execute(
            "INSERT INTO tasks (id, number, project_id, title, description, status, priority, due_date, sort_order, pinned, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                get_str(t, "id"), get_int(t, "number"), get_opt_str(t, "project_id"),
                get_str(t, "title"), get_opt_str(t, "description"),
                get_str(t, "status"), get_int(t, "priority"),
                get_opt_str(t, "due_date"), get_int(t, "sort_order"),
                get_int(t, "pinned"), get_str(t, "created_at"), get_str(t, "updated_at"),
                get_opt_str(t, "deleted_at"),
            ],
        ).map_err(|e| format!("Failed to import task: {}", e))?;
    }

    for l in &data.labels {
        conn.execute(
            "INSERT INTO labels (id, name, color, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![get_str(l, "id"), get_str(l, "name"), get_str(l, "color"), get_str(l, "created_at")],
        ).map_err(|e| format!("Failed to import label: {}", e))?;
    }

    for tl in &data.taskLabels {
        conn.execute(
            "INSERT OR IGNORE INTO task_labels (task_id, label_id) VALUES (?1, ?2)",
            params![get_str(tl, "task_id"), get_str(tl, "label_id")],
        ).map_err(|e| format!("Failed to import task label: {}", e))?;
    }

    for s in &data.subtasks {
        conn.execute(
            "INSERT INTO subtasks (id, task_id, title, completed, sort_order, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                get_str(s, "id"), get_str(s, "task_id"), get_str(s, "title"),
                get_int(s, "completed"), get_int(s, "sort_order"), get_str(s, "created_at"),
            ],
        ).map_err(|e| format!("Failed to import subtask: {}", e))?;
    }

    for a in &data.attachments {
        conn.execute(
            "INSERT INTO task_attachments (id, task_id, file_name, file_data, mime_type, file_size, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                get_str(a, "id"), get_str(a, "task_id"), get_str(a, "file_name"),
                "", get_str(a, "mime_type"), get_int(a, "file_size"), get_str(a, "created_at"),
            ],
        ).map_err(|e| format!("Failed to import attachment: {}", e))?;
    }

    for al in &data.activityLog {
        let source = get_opt_str(al, "source").unwrap_or_else(|| "gui".to_string());
        conn.execute(
            "INSERT INTO activity_log (id, task_id, action, field, old_value, new_value, source, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                get_str(al, "id"), get_str(al, "task_id"), get_str(al, "action"),
                get_opt_str(al, "field"), get_opt_str(al, "old_value"), get_opt_str(al, "new_value"),
                source, get_str(al, "created_at"),
            ],
        ).map_err(|e| format!("Failed to import activity log: {}", e))?;
    }

    let total = data.projects.len() + data.tasks.len() + data.labels.len() + data.subtasks.len() + data.attachments.len() + data.activityLog.len();
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "data_imported",
            "total": total,
            "file": file_path
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Imported {} records from {}", total, file_path);
    }
    Ok(())
}

pub fn reset(conn: &Connection, json: bool) -> Result<()> {
    for table in ["activity_log", "task_labels", "task_attachments", "subtasks", "tasks", "labels", "projects"] {
        conn.execute(&format!("DELETE FROM {}", table), [])
            .map_err(|e| format!("Failed to reset {}: {}", table, e))?;
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "data_reset"
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Database reset complete");
    }
    Ok(())
}

fn get_str(map: &HashMap<String, serde_json::Value>, key: &str) -> String {
    map.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn get_opt_str(map: &HashMap<String, serde_json::Value>, key: &str) -> Option<String> {
    map.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn get_int(map: &HashMap<String, serde_json::Value>, key: &str) -> i32 {
    map.get(key)
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32
}

pub fn backup(json: bool, db_path: &Path, keep: usize) -> Result<()> {
    let name = backup::create_backup(db_path, keep)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "backup_created",
                "name": name
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Backup created: {}", name);
    }
    Ok(())
}

pub fn backup_list(json: bool, db_path: &Path) -> Result<()> {
    let backups = backup::list_backups(db_path)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!(backups)).map_err(|e| e.to_string())?
        );
    } else if backups.is_empty() {
        println!("(no backups)");
    } else {
        let rows: Vec<Vec<String>> = backups
            .iter()
            .map(|b| vec![b.name.clone(), b.created_at.clone(), format!("{} bytes", b.size_bytes)])
            .collect();
        print_table(&["name", "created_at", "size"], &rows);
    }
    Ok(())
}

pub fn restore(json: bool, db_path: &Path, name: &str) -> Result<()> {
    backup::restore_backup(db_path, name)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "backup_restored",
                "name": name
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Restored backup: {}", name);
    }
    Ok(())
}

pub fn backup_delete(json: bool, db_path: &Path, name: &str) -> Result<()> {
    backup::delete_backup(db_path, name)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "backup_deleted",
                "name": name
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Deleted backup: {}", name);
    }
    Ok(())
}

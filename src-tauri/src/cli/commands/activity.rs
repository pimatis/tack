use rusqlite::{params, Connection};
use crate::db::*;
use serde_json::json;

pub fn list(conn: &Connection, json: bool, task_id: &str) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT id, action, field, old_value, new_value, source, created_at FROM activity_log WHERE task_id = ?1 ORDER BY created_at DESC LIMIT 50")
        .map_err(|e| format!("Failed to query activity: {}", e))?;

    let rows: Vec<Vec<String>> = stmt
        .query_map(params![task_id], |row| {
            let action: String = row.get(1)?;
            let field: Option<String> = row.get(2)?;
            let old_val: Option<String> = row.get(3)?;
            let new_val: Option<String> = row.get(4)?;
            let source: String = row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "gui".to_string());
            let created: String = row.get(6)?;

            let message = format_activity(&action, field.as_deref(), old_val.as_deref(), new_val.as_deref());
            Ok(vec![
                row.get::<_, String>(0)?,
                action,
                message,
                source,
                created,
            ])
        })
        .map_err(|e| format!("Failed to query activity: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    if json {
        let items: Vec<serde_json::Value> = rows.iter().map(|r| json!({
            "id": r[0],
            "action": r[1],
            "message": r[2],
            "source": r[3],
            "created_at": r[4],
        })).collect();
        println!("{}", serde_json::to_string_pretty(&json!({ "task_id": task_id, "activity": items }))
            .map_err(|e| e.to_string())?);
    } else {
        print_table(&["ID", "ACTION", "MESSAGE", "SOURCE", "CREATED"], &rows);
    }
    Ok(())
}

fn format_activity(action: &str, field: Option<&str>, old_val: Option<&str>, new_val: Option<&str>) -> String {
    let f = field.unwrap_or("");
    let old = old_val.unwrap_or("");
    let new = new_val.unwrap_or("");
    match action {
        "created" => "created this task".to_string(),
        "status_changed" => format!("changed status from {} to {}", old, new),
        "priority_changed" => format!("set priority to {}", new),
        "title_changed" => "updated the title".to_string(),
        "description_changed" => "updated the description".to_string(),
        "due_date_changed" => {
            if new.is_empty() { "removed the due date".to_string() }
            else { format!("set due date to {}", new) }
        }
        "label_added" => format!("added label {}", new),
        "label_removed" => format!("removed label {}", old),
        "attachment_added" => format!("attached {}", new),
        "attachment_removed" => "removed an attachment".to_string(),
        "subtask_added" => format!("added subtask \"{}\"", new),
        "subtask_completed" => format!("completed subtask \"{}\"", new),
        "subtask_uncompleted" => format!("reopened subtask \"{}\"", new),
        "subtask_removed" => "removed a subtask".to_string(),
        _ => format!("{} {} -> {}", f, old, new),
    }
}

use crate::backup;
use crate::db::*;
use rusqlite::{params, Connection};
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
    #[serde(default)]
    taskRelations: Vec<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    savedViews: Vec<HashMap<String, serde_json::Value>>,
    exportedAt: String,
}

fn query_all(
    conn: &Connection,
    table: &str,
    columns: &str,
) -> Result<Vec<HashMap<String, serde_json::Value>>> {
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
                    rusqlite::types::Value::Blob(b) => {
                        serde_json::Value::String(String::from_utf8_lossy(&b).to_string())
                    }
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

pub fn export(conn: &Connection, json: bool, output_path: &str, format: &str) -> Result<()> {
    match format {
        "csv" => return export_csv(conn, output_path),
        "markdown" | "md" => return export_markdown(conn, output_path),
        "json" => {}
        other => return Err(format!("Unknown format '{}': use json, csv or markdown", other)),
    }
    let data = ExportData {
        projects: query_all(conn, "projects", "*")?,
        tasks: query_all(conn, "tasks", "*")?,
        labels: query_all(conn, "labels", "*")?,
        taskLabels: query_all(conn, "task_labels", "*")?,
        subtasks: query_all(conn, "subtasks", "*")?,
        attachments: query_all(
            conn,
            "task_attachments",
            "id, task_id, file_name, mime_type, file_size, created_at",
        )?,
        activityLog: query_all(conn, "activity_log", "*")?,
        // tolerant: older dbs may lack these tables
        taskRelations: query_all(conn, "task_relations", "*").unwrap_or_default(),
        savedViews: query_all(conn, "saved_views", "*").unwrap_or_default(),
        exportedAt: now_iso(),
    };

    let json_str =
        serde_json::to_string_pretty(&data).map_err(|e| format!("Failed to serialize: {}", e))?;

    if json {
        println!("{}", json_str);
    } else {
        let path = if output_path.is_empty() {
            "tack-export.json"
        } else {
            output_path
        };
        std::fs::write(path, json_str).map_err(|e| format!("Failed to write file: {}", e))?;
        let total = data.projects.len()
            + data.tasks.len()
            + data.labels.len()
            + data.subtasks.len()
            + data.attachments.len()
            + data.activityLog.len()
            + data.taskRelations.len()
            + data.savedViews.len();
        println!("Exported {} records to {}", total, path);
    }
    Ok(())
}

pub fn import(conn: &Connection, json: bool, file_path: &str) -> Result<()> {
    let content =
        std::fs::read_to_string(file_path).map_err(|e| format!("Failed to read file: {}", e))?;
    if !content.trim_start().starts_with('{') {
        let count = import_csv(conn, &content)?;
        if json {
            println!("{}", json!({ "success": true, "action": "imported", "tasks": count }));
        } else {
            println!("Imported {} task(s)", count);
        }
        return Ok(());
    }
    let data: ExportData =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))?;

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
            "INSERT INTO tasks (id, number, project_id, title, description, status, priority, due_date, end_date, sort_order, pinned, created_at, updated_at, deleted_at, recurrence_rule, recurrence_parent_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                get_str(t, "id"), get_int(t, "number"), get_opt_str(t, "project_id"),
                get_str(t, "title"), get_opt_str(t, "description"),
                get_str(t, "status"), get_int(t, "priority"),
                get_opt_str(t, "due_date"), get_opt_str(t, "end_date"), get_int(t, "sort_order"),
                get_int(t, "pinned"), get_str(t, "created_at"), get_str(t, "updated_at"),
                get_opt_str(t, "deleted_at"), get_opt_str(t, "recurrence_rule"),
                get_opt_str(t, "recurrence_parent_id"),
            ],
        ).map_err(|e| format!("Failed to import task: {}", e))?;
    }

    for l in &data.labels {
        conn.execute(
            "INSERT INTO labels (id, name, color, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                get_str(l, "id"),
                get_str(l, "name"),
                get_str(l, "color"),
                get_str(l, "created_at")
            ],
        )
        .map_err(|e| format!("Failed to import label: {}", e))?;
    }

    for tl in &data.taskLabels {
        conn.execute(
            "INSERT OR IGNORE INTO task_labels (task_id, label_id) VALUES (?1, ?2)",
            params![get_str(tl, "task_id"), get_str(tl, "label_id")],
        )
        .map_err(|e| format!("Failed to import task label: {}", e))?;
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
            "INSERT INTO task_attachments (id, task_id, file_name, file_path, mime_type, file_size, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
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

    let total = data.projects.len()
        + data.tasks.len()
        + data.labels.len()
        + data.subtasks.len()
        + data.attachments.len()
        + data.activityLog.len()
        + data.taskRelations.len()
        + data.savedViews.len();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "data_imported",
                "total": total,
                "file": file_path
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Imported {} records from {}", total, file_path);
    }
    Ok(())
}

pub fn reset(conn: &Connection, json: bool) -> Result<()> {
    for table in [
        "activity_log",
        "task_labels",
        "task_attachments",
        "subtasks",
        "tasks",
        "labels",
        "projects",
    ] {
        conn.execute(&format!("DELETE FROM {}", table), [])
            .map_err(|e| format!("Failed to reset {}: {}", table, e))?;
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "data_reset"
            }))
            .map_err(|e| e.to_string())?
        );
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
    map.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn get_int(map: &HashMap<String, serde_json::Value>, key: &str) -> i32 {
    map.get(key).and_then(|v| v.as_i64()).unwrap_or(0) as i32
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
            .map(|b| {
                vec![
                    b.name.clone(),
                    b.created_at.clone(),
                    format!("{} bytes", b.size_bytes),
                ]
            })
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

// ---- csv + markdown export, csv import ----

fn csv_cell(value: &serde_json::Value) -> String {
    let s = match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

fn export_csv(conn: &Connection, output_path: &str) -> Result<()> {
    let rows = query_all(
        conn,
        "tasks t LEFT JOIN projects p ON p.id = t.project_id WHERE t.deleted_at IS NULL",
        "t.title, t.description, t.status, t.priority, p.prefix AS project, (SELECT GROUP_CONCAT(l.name, ';') FROM task_labels tl JOIN labels l ON l.id = tl.label_id WHERE tl.task_id = t.id) AS labels, t.due_date, t.end_date, t.estimate_minutes",
    )?;
    let mut out = String::from(
        "title,description,status,priority,project,labels,due_date,end_date,estimate_minutes\n",
    );
    for row in &rows {
        let cells: Vec<String> = [
            "title",
            "description",
            "status",
            "priority",
            "project",
            "labels",
            "due_date",
            "end_date",
            "estimate_minutes",
        ]
        .iter()
        .map(|c| csv_cell(row.get(*c).unwrap_or(&serde_json::Value::Null)))
        .collect();
        out.push_str(&cells.join(","));
        out.push('\n');
    }
    let path = if output_path.is_empty() {
        "tack-export.csv"
    } else {
        output_path
    };
    std::fs::write(path, out).map_err(|e| format!("Failed to write file: {}", e))?;
    println!("Exported {} tasks to {}", rows.len(), path);
    Ok(())
}

fn export_markdown(conn: &Connection, output_path: &str) -> Result<()> {
    let rows = query_all(
        conn,
        "tasks t LEFT JOIN projects p ON p.id = t.project_id WHERE t.deleted_at IS NULL",
        "t.title, t.status, t.due_date, t.number, p.prefix AS project",
    )?;
    let mut out = String::from("# Tasks\n\n");
    let mut last_status = String::new();
    for row in &rows {
        let status = row.get("status").and_then(|v| v.as_str()).unwrap_or("todo");
        if status != last_status {
            out.push_str(&format!("## {}\n\n", status.replace('_', " ")));
            last_status = status.to_string();
        }
        let checkbox = match status {
            "done" => "[x]",
            "canceled" => "[-]",
            _ => "[ ]",
        };
        let id = format!(
            "{}-{}",
            row.get("project").and_then(|v| v.as_str()).unwrap_or("TSK"),
            row.get("number").and_then(|v| v.as_i64()).unwrap_or(0)
        );
        let due = row
            .get("due_date")
            .and_then(|v| v.as_str())
            .map(|d| format!(" (due {})", d))
            .unwrap_or_default();
        let title = row.get("title").and_then(|v| v.as_str()).unwrap_or("");
        out.push_str(&format!("- {} **{}** {}{}\n", checkbox, id, title, due));
    }
    let path = if output_path.is_empty() {
        "tack-export.md"
    } else {
        output_path
    };
    std::fs::write(path, out).map_err(|e| format!("Failed to write file: {}", e))?;
    println!("Exported {} tasks to {}", rows.len(), path);
    Ok(())
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                cur.push(c);
            }
        } else if c == '"' {
            in_quotes = true;
        } else if c == ',' {
            cells.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    cells.push(cur);
    cells
}

// import task rows in the same column layout export_csv writes; ids/numbers
// are assigned fresh, projects and labels match by name or prefix
fn import_csv(conn: &Connection, text: &str) -> Result<usize> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header = match lines.next() {
        Some(h) => parse_csv_line(h),
        None => return Ok(0),
    };
    let idx = |name: &str| header.iter().position(|h| h.trim().eq_ignore_ascii_case(name));
    // todoist exports use TYPE,CONTENT,DESCRIPTION,PRIORITY,...,DATE,DATE_LANG;
    // detect them by the columns our own layout does not have
    let todoist = idx("content").is_some() && idx("date_lang").is_some();
    let title_col = if todoist { "content" } else { "title" };
    if idx(title_col).is_none() {
        return Err("CSV is missing a \"title\" column".to_string());
    }

    let mut imported = 0;
    for line in lines {
        let cells = parse_csv_line(line);
        let get = |name: &str| -> String {
            idx(name)
                .and_then(|i| cells.get(i))
                .map(|s| s.trim().to_string())
                .unwrap_or_default()
        };
        // todoist rows are tasks or notes; only tasks come across
        if todoist && get("type").eq_ignore_ascii_case("note") {
            continue;
        }
        let title = get(title_col);
        if title.is_empty() {
            continue;
        }
        let project_key = get("project").to_lowercase();
        let project_id: Option<String> = if project_key.is_empty() {
            None
        } else {
            let mut stmt = conn
                .prepare("SELECT id FROM projects WHERE prefix = ?1 COLLATE NOCASE OR name = ?1 COLLATE NOCASE")
                .map_err(|e| e.to_string())?;
            stmt.query_row(params![project_key], |r| r.get(0)).ok()
        };
        let status = get("status").to_lowercase();
        let status = if ["backlog", "todo", "in_progress", "done", "canceled"].contains(&status.as_str()) {
            status
        } else {
            "todo".to_string()
        };
        let priority: i32 = get("priority").parse().unwrap_or(0);
        let estimate: Option<i64> = get("estimate_minutes").parse().ok();
        let id = new_id();
        let now = now_iso();
        let number: i32 = match &project_id {
            Some(pid) => conn
                .query_row(
                    "SELECT COALESCE(MAX(number), 0) + 1 FROM tasks WHERE project_id = ?1",
                    params![pid],
                    |r| r.get(0),
                ),
            None => conn.query_row(
                "SELECT COALESCE(MAX(number), 0) + 1 FROM tasks WHERE project_id IS NULL",
                [],
                |r| r.get(0),
            ),
        }
        .map_err(|e| e.to_string())?;
        let description = get("description");
        // todoist DATE can carry a datetime; keep the YYYY-MM-DD part
        let due_raw = if todoist { get("date") } else { get("due_date") };
        let due_date = due_raw.get(..10).unwrap_or(&due_raw).to_string();
        let end_date = get("end_date");
        conn.execute(
            "INSERT INTO tasks (id, number, project_id, title, description, status, priority, due_date, end_date, estimate_minutes, sort_order, pinned, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, 0, ?11, ?12)",
            params![
                id,
                number,
                project_id,
                title,
                (!description.is_empty()).then_some(description),
                status,
                priority.clamp(0, 4),
                (!due_date.is_empty()).then_some(due_date),
                (!end_date.is_empty()).then_some(end_date),
                estimate,
                now,
                now
            ],
        )
        .map_err(|e| format!("Failed to import task: {}", e))?;
        let label_names = get("labels");
        for name in label_names.split(';').map(|n| n.trim()).filter(|n| !n.is_empty()) {
            let label_id: Option<String> = conn
                .query_row(
                    "SELECT id FROM labels WHERE name = ?1 COLLATE NOCASE",
                    params![name],
                    |r| r.get(0),
                )
                .ok();
            if let Some(lid) = label_id {
                conn.execute(
                    "INSERT OR IGNORE INTO task_labels (task_id, label_id) VALUES (?1, ?2)",
                    params![id, lid],
                )
                .ok();
            }
        }
        imported += 1;
    }
    Ok(imported)
}

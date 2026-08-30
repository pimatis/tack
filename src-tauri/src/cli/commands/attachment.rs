use rusqlite::{params, Connection};
use crate::db::*;
use serde_json::json;
use std::path::Path;

pub fn add(conn: &Connection, json: bool, task_id: &str, file_path: &str) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?1)",
        params![task_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    if !exists {
        return Err(format!("Task {} not found", task_id));
    }

    let path = Path::new(file_path);
    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("Invalid file path: {}", file_path))?;

    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("File not found: {}", e))?;
    let file_size = metadata.len() as i64;

    if file_size > 10 * 1024 * 1024 {
        return Err(format!("File {} exceeds 10MB limit", file_name));
    }

    let file_bytes = std::fs::read(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    let file_data = base64_encode(&file_bytes);

    let mime_type = mime_from_ext(path);

    let id = new_id();
    let now = now_iso();
    conn.execute(
        "INSERT INTO task_attachments (id, task_id, file_name, file_data, mime_type, file_size, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, task_id, file_name, file_data, mime_type, file_size, now],
    ).map_err(|e| format!("Failed to add attachment: {}", e))?;

    let _ = log_activity(conn, task_id, "attachment_added", None, None, Some(file_name), "cli");
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "attachment_added",
            "attachment": { "id": id, "task_id": task_id, "file_name": file_name, "mime_type": mime_type, "file_size": file_size, "created_at": now }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Added attachment: {} ({} bytes)", file_name, file_size);
        println!("ID: {}", id);
    }
    Ok(())
}

pub fn list(conn: &Connection, json: bool, task_id: &str) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT id, file_name, mime_type, file_size, created_at FROM task_attachments WHERE task_id = ?1 ORDER BY created_at ASC")
        .map_err(|e| format!("Failed to query attachments: {}", e))?;

    let rows: Vec<Vec<String>> = stmt
        .query_map(params![task_id], |row| {
            let size: i64 = row.get(3)?;
            let size_str = if size < 1024 {
                format!("{} B", size)
            } else if size < 1024 * 1024 {
                format!("{} KB", size / 1024)
            } else {
                format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
            };
            Ok(vec![
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                size_str,
                row.get::<_, String>(4)?,
            ])
        })
        .map_err(|e| format!("Failed to query attachments: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    if json {
        let items: Vec<serde_json::Value> = rows.iter().map(|r| json!({
            "id": r[0],
            "file_name": r[1],
            "mime_type": r[2],
            "size": r[3],
            "created_at": r[4],
        })).collect();
        println!("{}", serde_json::to_string_pretty(&json!({ "task_id": task_id, "attachments": items }))
            .map_err(|e| e.to_string())?);
    } else {
        print_table(&["ID", "FILE", "TYPE", "SIZE", "CREATED"], &rows);
    }
    Ok(())
}

pub fn delete(conn: &Connection, json: bool, id: &str) -> Result<()> {
    let task_id: String = conn.query_row(
        "SELECT task_id FROM task_attachments WHERE id = ?1",
        params![id],
        |row| row.get(0),
    ).map_err(|_| format!("Attachment {} not found", id))?;

    let result = conn.execute("DELETE FROM task_attachments WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete attachment: {}", e))?;

    if result == 0 {
        return Err(format!("Attachment {} not found", id));
    }
    let _ = log_activity(conn, &task_id, "attachment_removed", None, None, None, "cli");
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "attachment_deleted",
            "attachment": { "id": id }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Deleted attachment: {}", id);
    }
    Ok(())
}

pub fn download(conn: &Connection, json: bool, id: &str, output_path: &str) -> Result<()> {
    let (file_name, file_data): (String, String) = conn.query_row(
        "SELECT file_name, file_data FROM task_attachments WHERE id = ?1",
        params![id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|_| format!("Attachment {} not found", id))?;

    let data = base64_decode(&file_data)?;
    let out = if output_path.is_empty() { &file_name } else { output_path };
    std::fs::write(out, &data)
        .map_err(|e| format!("Failed to write file: {}", e))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "attachment_downloaded",
            "attachment": { "id": id, "file_name": file_name, "saved_to": out }
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Downloaded: {} -> {}", file_name, out);
    }
    Ok(())
}

// store with data url prefix so download can round-trip raw base64 strings
fn base64_encode(data: &[u8]) -> String {
    use base64::Engine as _;
    format!(
        "data:application/octet-stream;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(data)
    )
}

fn base64_decode(data: &str) -> Result<Vec<u8>> {
    let stripped = data.split(',').next_back().unwrap_or(data);
    let cleaned: Vec<u8> = stripped
        .bytes()
        .filter(|&b| !b.is_ascii_whitespace())
        .collect();
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(cleaned)
        .map_err(|e| format!("Invalid base64: {}", e))
}

fn mime_from_ext(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png".to_string(),
        Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
        Some("gif") => "image/gif".to_string(),
        Some("svg") => "image/svg+xml".to_string(),
        Some("pdf") => "application/pdf".to_string(),
        Some("txt") => "text/plain".to_string(),
        Some("json") => "application/json".to_string(),
        Some("html") | Some("htm") => "text/html".to_string(),
        Some("css") => "text/css".to_string(),
        Some("js") => "application/javascript".to_string(),
        Some("zip") => "application/zip".to_string(),
        Some("mp4") => "video/mp4".to_string(),
        Some("mp3") => "audio/mpeg".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

use rusqlite::{params, Connection};
use crate::db::*;
use serde_json::json;

pub fn get(conn: &Connection, json: bool) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings ORDER BY key")
        .map_err(|e| format!("Failed to query settings: {}", e))?;

    let mut rows: Vec<Vec<String>> = stmt
        .query_map([], |row| {
            Ok(vec![
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
            ])
        })
        .map_err(|e| format!("Failed to query settings: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let defaults = [
        ("theme", "dark"),
        ("sidebarCollapsed", "false"),
        ("compactMode", "false"),
        ("defaultViewMode", "list"),
        ("defaultStatus", "todo"),
        ("defaultPriority", "0"),
        ("dueSoonThreshold", "1"),
        ("prefixPadding", "0"),
        ("notificationsEnabled", "false"),
        ("notificationLeadTime", "24"),
    ];

    let existing_keys: std::collections::HashSet<String> = rows.iter().map(|r| r[0].clone()).collect();
    for (key, val) in &defaults {
        if !existing_keys.contains(*key) {
            rows.push(vec![key.to_string(), val.to_string()]);
        }
    }
    rows.sort_by(|a, b| a[0].cmp(&b[0]));

    if json {
        let map: serde_json::Map<String, serde_json::Value> = rows.iter()
            .map(|r| (r[0].clone(), json!(r[1])))
            .collect();
        println!("{}", serde_json::to_string_pretty(&json!({ "settings": map }))
            .map_err(|e| e.to_string())?);
    } else {
        print_table(&["KEY", "VALUE"], &rows);
    }
    Ok(())
}

pub fn set(conn: &Connection, json: bool, key: &str, value: &str) -> Result<()> {
    let valid_keys = [
        "theme", "sidebarCollapsed", "compactMode", "defaultViewMode",
        "defaultStatus", "defaultPriority", "dueSoonThreshold", "prefixPadding",
        "notificationsEnabled", "notificationLeadTime",
    ];

    if !valid_keys.contains(&key) {
        return Err(format!("Invalid key '{}'. Valid: {}", key, valid_keys.join(", ")));
    }

    if key == "theme" && !["dark", "light", "system"].contains(&value) {
        return Err("theme must be: dark, light, or system".to_string());
    }
    if key == "defaultViewMode" && !["list", "board"].contains(&value) {
        return Err("defaultViewMode must be: list or board".to_string());
    }
    if key == "defaultStatus" && !["todo", "in_progress"].contains(&value) {
        return Err("defaultStatus must be: todo or in_progress".to_string());
    }
    if key == "defaultPriority" {
        let p: i32 = value.parse().map_err(|_| "defaultPriority must be 0-4")?;
        if p < 0 || p > 4 {
            return Err("defaultPriority must be 0-4".to_string());
        }
    }

    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = ?2",
        params![key, value],
    ).map_err(|e| format!("Failed to set setting: {}", e))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&json!({
            "success": true,
            "action": "setting_set",
            "key": key,
            "value": value
        })).map_err(|e| e.to_string())?);
    } else {
        println!("Set {} = {}", key, value);
    }
    Ok(())
}

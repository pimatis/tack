use rusqlite::{params, Connection};
use crate::db::*;
use serde_json::json;

// keys mirror the GUI settings page (src/lib/types/settings.ts)
const DEFAULTS: &[(&str, &str)] = &[
    ("theme", "dark"),
    ("sidebarCollapsed", "false"),
    ("defaultViewMode", "list"),
    ("defaultStatus", "todo"),
    ("defaultPriority", "0"),
    ("dueSoonThreshold", "1"),
    ("prefixPadding", "0"),
    ("backupEnabled", "true"),
    ("backupIntervalHours", "24"),
    ("backupKeepCount", "7"),
    ("liveEnabled", "false"),
    ("livePort", "17890"),
    (
        "sidebarItems",
        "[{\"id\":\"pinned\",\"visible\":true},{\"id\":\"today\",\"visible\":true},{\"id\":\"upcoming\",\"visible\":true},{\"id\":\"overdue\",\"visible\":true},{\"id\":\"status\",\"visible\":true},{\"id\":\"priority\",\"visible\":true},{\"id\":\"quickStats\",\"visible\":true}]",
    ),
    (
        "shortcuts",
        "{\"command-palette\":[{\"key\":\"k\",\"mod\":\"metaOrCtrl\"}],\"new-task\":[{\"key\":\"c\"}],\"new-project\":[{\"key\":\"n\"}],\"toggle-sidebar\":[{\"key\":\"b\",\"mod\":\"metaOrCtrl\"}],\"toggle-view\":[{\"key\":\"\\\\\",\"mod\":\"metaOrCtrl\"}],\"select-all\":[{\"key\":\"a\",\"mod\":\"metaOrCtrl\"}],\"close\":[{\"key\":\"Escape\"}],\"save-task\":[{\"key\":\"Enter\",\"mod\":\"metaOrCtrl\"}]}",
    ),
];

fn validate_value(key: &str, value: &str) -> Result<()> {
    match key {
        "theme" => {
            if !["dark", "light", "system"].contains(&value) {
                return Err("theme must be: dark, light, or system".to_string());
            }
        }
        "sidebarCollapsed" | "backupEnabled" | "liveEnabled" => {
            if value != "true" && value != "false" {
                return Err(format!("{} must be true or false", key));
            }
        }
        "livePort" => {
            let p: u16 = value
                .parse()
                .map_err(|_| "livePort must be a number between 1024 and 65535".to_string())?;
            if !(1024..=65535).contains(&p) {
                return Err("livePort must be between 1024 and 65535".to_string());
            }
        }
        "defaultViewMode" => {
            if !["list", "board", "calendar"].contains(&value) {
                return Err("defaultViewMode must be: list, board, or calendar".to_string());
            }
        }
        "defaultStatus" => {
            if !["todo", "in_progress"].contains(&value) {
                return Err("defaultStatus must be: todo or in_progress".to_string());
            }
        }
        "defaultPriority" => {
            let p: i32 = value.parse().map_err(|_| "defaultPriority must be 0-4".to_string())?;
            if !(0..=4).contains(&p) {
                return Err("defaultPriority must be 0-4".to_string());
            }
        }
        "dueSoonThreshold" | "prefixPadding" | "backupIntervalHours" | "backupKeepCount" => {
            let n: i64 = value.parse().map_err(|_| format!("{} must be a number", key))?;
            if n < 0 {
                return Err(format!("{} must be 0 or greater", key));
            }
            if (key == "backupIntervalHours" || key == "backupKeepCount") && n == 0 {
                return Err(format!("{} must be greater than 0", key));
            }
        }
        "sidebarItems" => {
            let parsed: serde_json::Value = serde_json::from_str(value)
                .map_err(|_| "sidebarItems must be a valid JSON array".to_string())?;
            if !parsed.is_array() {
                return Err("sidebarItems must be a JSON array".to_string());
            }
        }
        "shortcuts" => {
            let parsed: serde_json::Value = serde_json::from_str(value)
                .map_err(|_| "shortcuts must be a valid JSON object".to_string())?;
            if !parsed.is_object() {
                return Err("shortcuts must be a JSON object".to_string());
            }
        }
        _ => {
            let valid = DEFAULTS.iter().map(|(k, _)| *k).collect::<Vec<_>>().join(", ");
            return Err(format!("Invalid key '{}'. Valid: {}", key, valid));
        }
    }
    Ok(())
}

pub fn get(conn: &Connection, json: bool) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings ORDER BY key")
        .map_err(|e| format!("Failed to query settings: {}", e))?;

    let db_rows: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| format!("Failed to query settings: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let db_map: std::collections::HashMap<&str, &str> = db_rows
        .iter()
        .filter_map(|(k, v)| DEFAULTS.iter().any(|(dk, _)| dk == k).then_some((k.as_str(), v.as_str())))
        .collect();

    let mut rows: Vec<Vec<String>> = DEFAULTS
        .iter()
        .map(|(key, default)| vec![key.to_string(), db_map.get(key).copied().unwrap_or(default).to_string()])
        .collect();
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
    validate_value(key, value)?;

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

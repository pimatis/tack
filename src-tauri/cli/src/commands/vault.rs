use rusqlite::{params, Connection};
use serde_json::json;

use crate::db::*;

// multi-vault support: the active notes folder lives in the notesFolder
// setting, saved vaults in the notesVaults json array (same keys as the gui)

fn load_vaults(conn: &Connection) -> Vec<String> {
    match get_setting(conn, "notesVaults") {
        Some(raw) => serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default(),
        None => vec![],
    }
}

fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = ?2",
        params![key, value],
    )
    .map_err(|e| format!("Failed to set setting: {}", e))?;
    Ok(())
}

pub fn list(conn: &Connection, json: bool) -> Result<()> {
    let current = get_setting(conn, "notesFolder").unwrap_or_default();
    let vaults = load_vaults(conn);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "current": current, "vaults": vaults }))
                .map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    let mut rows: Vec<Vec<String>> = Vec::new();
    if !current.is_empty() {
        rows.push(vec![current.clone(), "yes".to_string()]);
    }
    for v in &vaults {
        if *v != current {
            rows.push(vec![v.clone(), "no".to_string()]);
        }
    }
    if rows.is_empty() {
        println!("No vaults configured");
        return Ok(());
    }
    print_table(&["Folder", "Active"], &rows);
    Ok(())
}

// switch the active vault; the path must exist and is added to the list
pub fn use_vault(conn: &Connection, json: bool, path: &str) -> Result<()> {
    let abs =
        std::fs::canonicalize(path).map_err(|e| format!("Folder not found: {} ({})", path, e))?;
    if !abs.is_dir() {
        return Err(format!("Not a folder: {}", abs.display()));
    }
    let value = abs.to_string_lossy().into_owned();
    set_setting(conn, "notesFolder", &value)?;
    let mut vaults = load_vaults(conn);
    if !vaults.contains(&value) {
        vaults.push(value.clone());
        set_setting(
            conn,
            "notesVaults",
            &serde_json::to_string(&vaults).unwrap_or_default(),
        )?;
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({ "success": true, "action": "vault_switched", "folder": value })
            )
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Active vault: {}", value);
    }
    Ok(())
}

// register an existing folder in the vault list without switching to it
pub fn add(conn: &Connection, json: bool, path: &str) -> Result<()> {
    let abs =
        std::fs::canonicalize(path).map_err(|e| format!("Folder not found: {} ({})", path, e))?;
    if !abs.is_dir() {
        return Err(format!("Not a folder: {}", abs.display()));
    }
    let value = abs.to_string_lossy().into_owned();
    let mut vaults = load_vaults(conn);
    if vaults.contains(&value) {
        return Err(format!("Vault already registered: {}", value));
    }
    vaults.push(value.clone());
    set_setting(
        conn,
        "notesVaults",
        &serde_json::to_string(&vaults).unwrap_or_default(),
    )?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({ "success": true, "action": "vault_added", "folder": value })
            )
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Added vault: {}", value);
    }
    Ok(())
}

// forget a vault in the list; the active notesFolder is left untouched
pub fn remove(conn: &Connection, json: bool, path: &str) -> Result<()> {
    let vaults: Vec<String> = load_vaults(conn)
        .into_iter()
        .filter(|v| v != path)
        .collect();
    set_setting(
        conn,
        "notesVaults",
        &serde_json::to_string(&vaults).unwrap_or_default(),
    )?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "success": true, "action": "vault_removed" }))
                .map_err(|e| e.to_string())?
        );
    } else {
        println!("Removed vault: {}", path);
    }
    Ok(())
}

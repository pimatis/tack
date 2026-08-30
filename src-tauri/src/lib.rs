pub mod backup;

use notify::{Watcher, RecursiveMode, EventKind};
use tauri::{Emitter, Manager};
use std::path::PathBuf;

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn write_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

fn attachments_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let dir = data_dir.join("attachments");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

// decode base64 (data url prefix and whitespace tolerated) and write binary file to disk
#[tauri::command]
fn save_attachment(app: tauri::AppHandle, id: String, file_data: String) -> Result<String, String> {
    let dir = attachments_dir(&app)?;
    let bytes = base64_decode(&file_data)?;
    let path = dir.join(&id);
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

// read binary file from disk and return as base64 data url
#[tauri::command]
fn read_attachment(app: tauri::AppHandle, id: String, mime_type: String) -> Result<String, String> {
    let dir = attachments_dir(&app)?;
    let path = dir.join(&id);
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(format!("data:{};base64,{}", mime_type, base64_encode(&bytes)))
}

// delete file from disk
#[tauri::command]
fn delete_attachment(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let dir = attachments_dir(&app)?;
    let path = dir.join(&id);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// copy attachment file to user-selected path
#[tauri::command]
fn download_attachment(app: tauri::AppHandle, id: String, dest_path: String) -> Result<(), String> {
    let dir = attachments_dir(&app)?;
    let src = dir.join(&id);
    if !src.exists() {
        return Err("attachment file not found".to_string());
    }
    std::fs::copy(&src, &dest_path).map_err(|e| e.to_string())?;
    Ok(())
}

// strip data url prefix (e.g. "data:image/png;base64,") and whitespace, then decode
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.split(',').next_back().unwrap_or(input);
    let input = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect::<Vec<u8>>();
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|e| e.to_string())
}

fn app_db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("tack.db"))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_backup(app: tauri::AppHandle, keep: Option<usize>) -> Result<String, String> {
    backup::create_backup(&app_db_path(&app)?, keep.unwrap_or(7))
}

#[tauri::command]
fn list_backups(app: tauri::AppHandle) -> Result<Vec<backup::BackupInfo>, String> {
    backup::list_backups(&app_db_path(&app)?)
}

#[tauri::command]
fn restore_backup(app: tauri::AppHandle, name: String) -> Result<(), String> {
    backup::restore_backup(&app_db_path(&app)?, &name)
}

#[tauri::command]
fn delete_backup(app: tauri::AppHandle, name: String) -> Result<(), String> {
    backup::delete_backup(&app_db_path(&app)?, &name)
}

fn base64_encode(input: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(input)
}

fn start_db_watcher(app: tauri::AppHandle) {
    let data_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let watch_dir = data_dir.clone();
    let backups_dir = watch_dir.join("backups");
    let backups_for_event = backups_dir.clone();
    // ensure the backups dir exists so the watcher has something to watch
    let _ = std::fs::create_dir_all(&backups_dir);

    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, _>| {
        if let Ok(event) = res {
            let is_db_change = event.paths.iter().any(|p| {
                p.file_name().map(|n| n == "tack.db" || n == "tack.db-wal" || n == "tack.db-shm").unwrap_or(false)
            });
            let is_backup_change = event.paths.iter().any(|p| p.starts_with(&backups_for_event));
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    if is_db_change {
                        let _ = app.emit("db-changed", ());
                    }
                    if is_backup_change {
                        let _ = app.emit("backups-changed", ());
                    }
                }
                _ => {}
            }
        }
    }).expect("failed to create file watcher");

    let _ = watcher.watch(&watch_dir, RecursiveMode::NonRecursive);
    let _ = watcher.watch(&backups_dir, RecursiveMode::NonRecursive);

    // keep watcher alive for app lifetime
    std::mem::forget(watcher);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = vec![tauri_plugin_sql::Migration {
        version: 1,
        description: "create_tasks_table",
        sql: include_str!("../migrations/001_initial.sql"),
        kind: tauri_plugin_sql::MigrationKind::Up,
    }, tauri_plugin_sql::Migration {
        version: 2,
        description: "create_projects_table",
        sql: include_str!("../migrations/002_projects.sql"),
        kind: tauri_plugin_sql::MigrationKind::Up,
    }, tauri_plugin_sql::Migration {
        version: 3,
        description: "create_attachments_table",
        sql: include_str!("../migrations/003_attachments.sql"),
        kind: tauri_plugin_sql::MigrationKind::Up,
    }, tauri_plugin_sql::Migration {
        version: 4,
        description: "create_labels_table",
        sql: include_str!("../migrations/004_labels.sql"),
        kind: tauri_plugin_sql::MigrationKind::Up,
    }, tauri_plugin_sql::Migration {
        version: 5,
        description: "add_due_date_to_tasks",
        sql: include_str!("../migrations/005_due_date.sql"),
        kind: tauri_plugin_sql::MigrationKind::Up,
    }, tauri_plugin_sql::Migration {
        version: 6,
        description: "add_task_number",
        sql: include_str!("../migrations/006_task_number.sql"),
        kind: tauri_plugin_sql::MigrationKind::Up,
    }, tauri_plugin_sql::Migration {
        version: 7,
        description: "add_project_description",
        sql: include_str!("../migrations/007_project_description.sql"),
        kind: tauri_plugin_sql::MigrationKind::Up,
    }, tauri_plugin_sql::Migration {
        version: 8,
        description: "add_subtasks_activity_log_sort_order",
        sql: include_str!("../migrations/008_subtasks_activity_sort.sql"),
        kind: tauri_plugin_sql::MigrationKind::Up,
    }, tauri_plugin_sql::Migration {
        version: 9,
        description: "add_pinned_to_tasks",
        sql: include_str!("../migrations/009_pinned.sql"),
        kind: tauri_plugin_sql::MigrationKind::Up,
    }, tauri_plugin_sql::Migration {
        version: 10,
        description: "create_settings_table",
        sql: include_str!("../migrations/010_settings.sql"),
        kind: tauri_plugin_sql::MigrationKind::Up,
    }, tauri_plugin_sql::Migration {
        version: 11,
        description: "add_source_to_activity_log",
        sql: include_str!("../migrations/011_activity_source.sql"),
        kind: tauri_plugin_sql::MigrationKind::Up,
    }, tauri_plugin_sql::Migration {
        version: 12,
        description: "add_deleted_at_to_tasks",
        sql: include_str!("../migrations/012_trash.sql"),
        kind: tauri_plugin_sql::MigrationKind::Up,
    }, tauri_plugin_sql::Migration {
        version: 13,
        description: "migrate_attachment_file_path",
        sql: include_str!("../migrations/013_attachment_file_path.sql"),
        kind: tauri_plugin_sql::MigrationKind::Up,
    }, tauri_plugin_sql::Migration {
        version: 14,
        description: "create_fts_search_index",
        sql: include_str!("../migrations/014_fts_search.sql"),
        kind: tauri_plugin_sql::MigrationKind::Up,
    }, tauri_plugin_sql::Migration {
        version: 15,
        description: "fix_fts_triggers",
        sql: include_str!("../migrations/015_fix_fts_triggers.sql"),
        kind: tauri_plugin_sql::MigrationKind::Up,
    }];

    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:tack.db", migrations)
                .build(),
        )
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![get_app_version, write_file, read_file, save_attachment, read_attachment, delete_attachment, download_attachment, create_backup, list_backups, restore_backup, delete_backup])
        .setup(|app| {
            let handle = app.handle().clone();
            start_db_watcher(handle);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub mod backup;
mod live;

use notify::{Watcher, RecursiveMode, EventKind};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha384};
use std::sync::{Arc, Mutex};
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

pub(crate) fn attachments_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
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

pub(crate) fn app_db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| {
            std::fs::create_dir_all(&dir).ok();
            dir.join("tack.db")
        })
        .map_err(|e| e.to_string())
}

// apply pending migrations with a tolerant runner: tracking rows for
// migrations this binary does not know are dropped instead of erroring, so
// older binaries can open the db again (all migrations are additive, the
// leftover schema is harmless to them)
fn run_migrations(conn: &Connection) -> Result<(), String> {
    let migrations: &[(i32, &str, &str)] = &[
        (1, "create_tasks_table", include_str!("../migrations/001_initial.sql")),
        (2, "create_projects_table", include_str!("../migrations/002_projects.sql")),
        (3, "create_attachments_table", include_str!("../migrations/003_attachments.sql")),
        (4, "create_labels_table", include_str!("../migrations/004_labels.sql")),
        (5, "add_due_date_to_tasks", include_str!("../migrations/005_due_date.sql")),
        (6, "add_task_number", include_str!("../migrations/006_task_number.sql")),
        (7, "add_project_description", include_str!("../migrations/007_project_description.sql")),
        (8, "add_subtasks_activity_log_sort_order", include_str!("../migrations/008_subtasks_activity_sort.sql")),
        (9, "add_pinned_to_tasks", include_str!("../migrations/009_pinned.sql")),
        (10, "create_settings_table", include_str!("../migrations/010_settings.sql")),
        (11, "add_source_to_activity_log", include_str!("../migrations/011_activity_source.sql")),
        (12, "add_deleted_at_to_tasks", include_str!("../migrations/012_trash.sql")),
        (13, "migrate_attachment_file_path", include_str!("../migrations/013_attachment_file_path.sql")),
        (14, "create_fts_search_index", include_str!("../migrations/014_fts_search.sql")),
        (15, "fix_fts_triggers", include_str!("../migrations/015_fix_fts_triggers.sql")),
        (16, "add_end_date_to_tasks", include_str!("../migrations/016_end_date.sql")),
    ];
    let known: std::collections::HashSet<i64> = migrations.iter().map(|(v, _, _)| *v as i64).collect();

    // track applied versions via the same table the sqlx-based runner
    // (tauri-plugin-sql) used, so older binaries and the cli stay compatible
    let has_tracking = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    // db without tracking table but with tack tables was fully migrated by
    // the old untracked runner, treat everything as applied
    let has_tack_tables = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tasks'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    let mut applied: std::collections::HashSet<i64> = if has_tracking {
        let mut stmt = conn
            .prepare("SELECT version FROM _sqlx_migrations WHERE success = 1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, i64>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    } else if has_tack_tables {
        known.clone()
    } else {
        std::collections::HashSet::new()
    };

    // drop tracking rows for migrations this binary does not know, so a
    // downgrade opens cleanly instead of erroring (schema stays ahead)
    if has_tracking {
        for version in applied.difference(&known) {
            conn.execute(
                "DELETE FROM _sqlx_migrations WHERE version = ?1",
                params![version],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    applied.retain(|v| known.contains(v));

    for (version, description, sql) in migrations {
        if applied.contains(&(*version as i64)) {
            continue;
        }
        if let Err(e) = conn.execute_batch(sql) {
            let msg = e.to_string();
            if !msg.contains("duplicate column name") {
                return Err(format!("migration {} failed: {}", version, msg));
            }
        }
        // same schema sqlx creates; checksum matches sqlx's sha384 so older
        // binaries using tauri-plugin-sql can still verify it
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _sqlx_migrations (
                version BIGINT PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                success BOOLEAN NOT NULL,
                checksum BLOB NOT NULL,
                execution_time BIGINT NOT NULL
            );",
        )
        .map_err(|e| e.to_string())?;
        let checksum: Vec<u8> = Sha384::digest(sql.as_bytes()).to_vec();
        conn.execute(
            "INSERT OR IGNORE INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time)
             VALUES (?1, ?2, CURRENT_TIMESTAMP, 1, ?3, 0)",
            params![*version as i64, *description, checksum],
        )
        .map_err(|e| format!("failed to record migration {}: {}", version, e))?;
    }
    Ok(())
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

pub(crate) fn base64_encode(input: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(input)
}

fn start_db_watcher(app: tauri::AppHandle, hub: Arc<live::LiveHub>) {
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
                        hub.notify();
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

// resolvers for the bundled cli binary and its symlink target (macOS only)
const PATH_CANDIDATES: [&str; 2] = ["/usr/local/bin", "/opt/homebrew/bin"];

fn bundled_cli_bin(app: &tauri::AppHandle) -> Option<PathBuf> {
    let resource_dir = app.path().resource_dir().ok()?;
    let cli_bin = resource_dir.join("tack-cli");
    cli_bin.exists().then_some(cli_bin)
}

// pick a directory on PATH we can write to: a system bin dir, else the user's ~/.local/bin
fn cli_bin_dir() -> Option<PathBuf> {
    let system = PATH_CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|dir| dir.exists() && dir.join("tack").exists());
    if let Some(dir) = system {
        return Some(dir);
    }

    let home = dirs::home_dir()?;
    let local = home.join(".local").join("bin");
    if std::fs::create_dir_all(&local).is_ok() {
        Some(local)
    } else {
        None
    }
}

fn cli_symlink_target() -> Option<PathBuf> {
    cli_bin_dir().map(|dir| dir.join("tack"))
}

// make sure the chosen bin dir is on PATH (only needed when we fell back to ~/.local/bin)
fn ensure_cli_on_path(target: &std::path::Path) {
    let Some(dir) = target.parent() else {
        return;
    };
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let shell_rc = home.join(".zshrc");
    let dir_str = dir.to_string_lossy().to_string();
    let already = std::fs::read_to_string(&shell_rc)
        .map(|c| c.contains(&dir_str))
        .unwrap_or(false);
    if already {
        return;
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&shell_rc) {
        use std::io::Write;
        let _ = writeln!(f, "export PATH=\"{dir_str}:$PATH\"");
    }
}

// fsevents does not report appends to an already-open wal file (the gui's
// sqlx pool writes exactly that way), so poll the db file snapshots as a
// backstop: desktop and cli changes reach the live site within ~2s either way
fn start_db_poller(app: tauri::AppHandle, hub: Arc<live::LiveHub>) {
    std::thread::spawn(move || {
        let data_dir = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        let mut last: Option<_> = None;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            let now = live::db_files_snapshot(&data_dir);
            if let Some(prev) = last {
                if prev != now {
                    let _ = app.emit("db-changed", ());
                    hub.notify();
                }
            }
            last = Some(now);
        }
    });
}

// place a symlink to the bundled cli on PATH so `tack` works from any terminal
fn install_cli_link(app: &tauri::AppHandle) -> Result<bool, String> {
    if !cfg!(target_os = "macos") {
        return Ok(false);
    }
    let Some(cli_bin) = bundled_cli_bin(app) else {
        return Err("bundled cli not found".to_string());
    };
    let Some(target) = cli_symlink_target() else {
        return Err("no writable bin directory found".to_string());
    };
    if !target.exists() {
        std::os::unix::fs::symlink(&cli_bin, &target)
            .map_err(|e| format!("failed to link cli: {}", e))?;
    }
    ensure_cli_on_path(&target);
    Ok(true)
}

#[tauri::command]
fn install_cli(app: tauri::AppHandle) -> Result<bool, String> {
    install_cli_link(&app)
}

#[tauri::command]
fn cli_installed() -> bool {
    cli_symlink_target()
        .map(|target| target.exists())
        .unwrap_or(false)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![get_app_version, write_file, read_file, save_attachment, read_attachment, delete_attachment, download_attachment, create_backup, list_backups, restore_backup, delete_backup, install_cli, cli_installed, live::live_start, live::live_stop, live::live_status])
        .setup(|app| {
            let handle = app.handle().clone();
            let _ = install_cli_link(&handle);
            // migrate before the webview loads the db, so the sql plugin
            // never sees a version it does not know
            let conn = Connection::open(app_db_path(app.handle())?)?;
            conn.pragma_update(None, "journal_mode", "WAL")?;
            run_migrations(&conn)?;
            // live is session-scoped: clear the persisted flag at launch so
            // the webview never reads a stale true and auto-starts the server
            let _ = conn.execute("UPDATE settings SET value = 'false' WHERE key = 'liveEnabled'", []);
            let hub = Arc::new(live::LiveHub::default());
            app.manage(live::LiveState {
                server: Mutex::new(None),
                hub: hub.clone(),
            });
            start_db_watcher(handle.clone(), hub.clone());
            start_db_poller(handle, hub);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

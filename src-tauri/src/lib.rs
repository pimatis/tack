pub mod backup;
mod attachments;
mod cli;
mod db;
mod db_reporter;
mod live;
mod migrations;
mod notes;

use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri_plugin_window_state::StateFlags;

// reveal the window once the design is on screen; called from the command,
// the page-load hook, and the fallback timer
fn reveal_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn show_window(app: tauri::AppHandle) {
    reveal_main_window(&app);
}

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

#[tauri::command]
fn create_backup(app: tauri::AppHandle, keep: Option<usize>) -> Result<String, String> {
    backup::create_backup(&db::app_db_path(&app)?, keep.unwrap_or(7))
}

#[tauri::command]
fn list_backups(app: tauri::AppHandle) -> Result<Vec<backup::BackupInfo>, String> {
    backup::list_backups(&db::app_db_path(&app)?)
}

#[tauri::command]
fn restore_backup(app: tauri::AppHandle, name: String) -> Result<(), String> {
    backup::restore_backup(&db::app_db_path(&app)?, &name)
}

#[tauri::command]
fn delete_backup(app: tauri::AppHandle, name: String) -> Result<(), String> {
    backup::delete_backup(&db::app_db_path(&app)?, &name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_window_state::Builder::default()
                // visibility is managed by show_window after first paint
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_page_load(move |webview, payload| {
            // rust-side reveal: unlike requestAnimationFrame, this fires even
            // while the window is hidden (webkit pauses rAF off-screen)
            if payload.event() == tauri::webview::PageLoadEvent::Finished {
                reveal_main_window(webview.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![get_app_version, show_window, write_file, read_file, notes::list_notes, notes::read_notes, notes::rename_note, notes::delete_note, attachments::save_attachment, attachments::read_attachment, attachments::delete_attachment, attachments::download_attachment, create_backup, list_backups, restore_backup, delete_backup, cli::install_cli, cli::cli_installed, live::live_start, live::live_stop, live::live_status, live::hash_live_password])
        .setup(|app| {
            let handle = app.handle().clone();
            // fs + shellrc work: keep it off the critical path so the webview
            // spawns as early as possible
            std::thread::spawn(|| {
                let _ = cli::install_cli_link();
            });
            // migrate before the webview loads the db, so the sql plugin
            // never sees a version it does not know
            let conn = Connection::open(db::app_db_path(app.handle())?)?;
            conn.pragma_update(None, "journal_mode", "WAL")?;
            migrations::run_migrations(&conn)?;
            // live is session-scoped: clear the persisted flag at launch so
            // the webview never reads a stale true and auto-starts the server
            let _ = conn.execute("UPDATE settings SET value = 'false' WHERE key = 'liveEnabled'", []);
            let hub = Arc::new(live::LiveHub::default());
            app.manage(live::LiveState {
                server: Mutex::new(None),
                hub: hub.clone(),
            });
            db_reporter::start_db_reporter(handle.clone(), hub.clone());
            // safety net: if the frontend never boots, don't leave the user
            // with a permanently hidden window
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(5));
                reveal_main_window(&handle);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

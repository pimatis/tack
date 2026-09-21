mod attachments;
pub mod backup;
mod cli;
mod db;
mod db_reporter;
mod live;
#[cfg(target_os = "macos")]
mod macos_notifications;
mod migrations;
mod notes;

use rusqlite::Connection;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tauri_plugin_window_state::StateFlags;

static REVEALED: AtomicBool = AtomicBool::new(false);

// reveal the window once, on first paint; called from the command, the
// page-load hook, and the fallback timer. later page loads (dev hmr reloads
// while a build runs) must not steal focus from whatever the user is doing
fn reveal_main_window(app: &tauri::AppHandle) {
    if REVEALED.swap(true, Ordering::SeqCst) {
        return;
    }
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

// deep link into the os notification settings, where the notification
// permission lives; macOS only, other platforms have no equivalent pane
#[tauri::command]
fn open_notification_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // macOS 15 renamed the pane; older releases keep the extension id, and
        // pre-ventura uses the legacy preference pane
        let panes: &[&str] = match macos_major_version() {
            Some(v) if v >= 15 => &[
                "x-apple.systempreferences:com.apple.Notifications",
                "x-apple.systempreferences:com.apple.Notifications-Settings.extension",
            ],
            Some(v) if v >= 13 => &[
                "x-apple.systempreferences:com.apple.Notifications-Settings.extension",
                "x-apple.systempreferences:com.apple.Notifications",
            ],
            _ => &[
                "x-apple.systempreferences:com.apple.preference.notifications",
                "x-apple.systempreferences:com.apple.Notifications-Settings.extension",
            ],
        };
        for pane in panes {
            let opened = std::process::Command::new("open")
                .arg(pane)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if opened {
                return Ok(());
            }
        }
        Err("Could not open System Settings".to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Opening notification settings is only supported on macOS".to_string())
    }
}

#[cfg(target_os = "macos")]
fn macos_major_version() -> Option<u32> {
    let out = std::process::Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok()?;
    let version = String::from_utf8_lossy(&out.stdout);
    version.trim().split('.').next()?.parse().ok()
}

// deliver a task reminder as a native notification. on macOS it is delivered
// interactively (waits for a click) so a click can bring the window forward and
// tell the ui which task to open; elsewhere it falls back to the plugin
#[tauri::command]
#[allow(unused_variables)]
fn notify_reminder(
    app: tauri::AppHandle,
    task_id: String,
    title: String,
    body: String,
    subtitle: Option<String>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // a bundled app uses the modern center (real permission, foreground
        // banners, click routing); a dev binary falls back to the legacy path
        if macos_notifications::is_bundled() {
            macos_notifications::send(&task_id, &title, subtitle.as_deref(), &body);
            return Ok(());
        }
        // the wait blocks until the user interacts, so keep it off the main thread
        std::thread::spawn(move || {
            use mac_notification_sys::{Notification, NotificationResponse};
            let mut notification = Notification::new();
            notification
                .title(&title)
                .message(&body)
                .wait_for_click(true)
                .default_sound();
            if let Some(subtitle) = subtitle.as_deref() {
                notification.subtitle(subtitle);
            }
            let clicked = matches!(
                notification.send(),
                Ok(NotificationResponse::Click) | Ok(NotificationResponse::ActionButton(_))
            );
            if clicked {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                let _ = app.emit("reminder-clicked", task_id);
            }
        });
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        use tauri_plugin_notification::NotificationExt;
        app.notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|e| e.to_string())
    }
}

// real notification authorization status: "granted" | "denied" | "prompt" |
// "unsupported" (macOS outside a bundled app)
#[tauri::command]
fn notification_permission() -> String {
    #[cfg(target_os = "macos")]
    {
        macos_notifications::status().to_string()
    }
    #[cfg(not(target_os = "macos"))]
    {
        "granted".to_string()
    }
}

// asks the os; the prompt resolves asynchronously, so the ui re-reads the status
#[tauri::command]
fn request_notification_permission() {
    #[cfg(target_os = "macos")]
    {
        macos_notifications::request();
    }
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
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        // pasted note images are served through our own scheme: the asset
        // protocol refuses dotfile paths, and .tack/assets is one
        .register_uri_scheme_protocol("tackasset", |_ctx, request| {
            notes::asset::respond(request)
        })
        .on_page_load(move |webview, payload| {
            // rust-side reveal: unlike requestAnimationFrame, this fires even
            // while the window is hidden (webkit pauses rAF off-screen)
            if payload.event() == tauri::webview::PageLoadEvent::Finished {
                reveal_main_window(webview.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            show_window,
            open_notification_settings,
            notify_reminder,
            notification_permission,
            request_notification_permission,
            write_file,
            read_file,
            notes::list_notes,
            notes::read_notes,
            notes::rename_note,
            notes::delete_note,
            notes::list_note_folders,
            notes::list_notes_deep,
            notes::read_notes_deep,
            notes::create_folder,
            notes::delete_folder,
            notes::note_info,
            notes::save_note_with_history,
            notes::write_binary_file,
            notes::watch::watch_notes_dir,
            attachments::save_attachment,
            attachments::read_attachment,
            attachments::delete_attachment,
            attachments::download_attachment,
            create_backup,
            list_backups,
            restore_backup,
            delete_backup,
            cli::install_cli,
            cli::cli_installed,
            live::live_start,
            live::live_stop,
            live::live_status,
            live::live_presence,
            live::hash_live_password
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            // attribute notifications to tack itself (bundle icon and name)
            // instead of the dev binary or the terminal
            #[cfg(target_os = "macos")]
            {
                let identifier = app.handle().config().identifier.clone();
                let _ = mac_notification_sys::set_application(&identifier);
                // real permission + foreground banners + click routing; no-ops
                // unless the app is bundled (a dev binary has no bundle id)
                macos_notifications::install(app.handle());
            }
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
            let _ = conn.execute(
                "UPDATE settings SET value = 'false' WHERE key = 'liveEnabled'",
                [],
            );
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

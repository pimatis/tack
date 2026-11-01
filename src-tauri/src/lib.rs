mod agent;
mod attachments;
pub mod backup;
mod cli;
mod db;
mod db_reporter;
mod live;
#[cfg(target_os = "macos")]
mod macos_notifications;
mod migrations;
// the cli's natural-language parser, shared so agents get the same grammar
#[path = "../cli/src/quickadd.rs"]
mod quickadd;

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

// tray menu and the global hotkey call this; unlike reveal_main_window it
// runs every time, not just on first paint
fn show_and_focus(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

// tray pieces the frontend mutates at runtime: the title badge, the
// due-tasks submenu and the two check items (live server, do not disturb)
struct TrayItems {
    tray: tauri::tray::TrayIcon,
    tasks: tauri::menu::Submenu<tauri::Wry>,
    live: tauri::menu::CheckMenuItem<tauri::Wry>,
    dnd: tauri::menu::CheckMenuItem<tauri::Wry>,
}

#[derive(serde::Deserialize)]
struct TrayTask {
    id: String,
    title: String,
}

// the ui pushes its derived state here; rust owns no task/settings knowledge
#[tauri::command]
fn update_tray(
    app: tauri::AppHandle,
    count: u32,
    tasks: Vec<TrayTask>,
    live_on: bool,
    dnd_on: bool,
    tray_on: bool,
) -> Result<(), String> {
    use tauri::menu::MenuItemBuilder;
    let state = app.state::<Mutex<TrayItems>>();
    let items = state.lock().map_err(|e| e.to_string())?;
    let _ = items.tray.set_visible(tray_on);
    let title = if count == 0 {
        None
    } else {
        Some(count.to_string())
    };
    let _ = items.tray.set_title(title);
    let _ = items.live.set_checked(live_on);
    let _ = items.dnd.set_checked(dnd_on);
    // rebuild the submenu from scratch; a handful of items, no diffing needed
    if let Ok(existing) = items.tasks.items() {
        for item in existing {
            let _ = items.tasks.remove(&item);
        }
    }
    if tasks.is_empty() {
        let empty = MenuItemBuilder::with_id("noop", "Nothing due today")
            .enabled(false)
            .build(&app)
            .map_err(|e| e.to_string())?;
        items.tasks.append(&empty).map_err(|e| e.to_string())?;
        return Ok(());
    }
    for task in tasks.iter().take(10) {
        let item = MenuItemBuilder::with_id(format!("task:{}", task.id), &task.title)
            .build(&app)
            .map_err(|e| e.to_string())?;
        items.tasks.append(&item).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn setup_tray(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::menu::{CheckMenuItemBuilder, Menu, MenuItemBuilder, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;

    let open = MenuItemBuilder::with_id("open", "Open Tack")
        .build(app)
        .map_err(|e| e.to_string())?;
    let nav_today = MenuItemBuilder::with_id("nav_today", "Today")
        .build(app)
        .map_err(|e| e.to_string())?;
    let nav_overdue = MenuItemBuilder::with_id("nav_overdue", "Overdue")
        .build(app)
        .map_err(|e| e.to_string())?;
    let tasks_sub = tauri::menu::SubmenuBuilder::with_id(app, "due_tasks", "Due today")
        .build()
        .map_err(|e| e.to_string())?;
    let quick = MenuItemBuilder::with_id("quick_add", "Quick add task")
        .build(app)
        .map_err(|e| e.to_string())?;
    let clip = MenuItemBuilder::with_id("quick_add_clipboard", "Quick add from clipboard")
        .build(app)
        .map_err(|e| e.to_string())?;
    let live = CheckMenuItemBuilder::with_id("toggle_live", "Live server")
        .build(app)
        .map_err(|e| e.to_string())?;
    let dnd = CheckMenuItemBuilder::with_id("toggle_dnd", "Do not disturb")
        .build(app)
        .map_err(|e| e.to_string())?;
    let updates = MenuItemBuilder::with_id("check_updates", "Check for Updates…")
        .build(app)
        .map_err(|e| e.to_string())?;
    let quit = MenuItemBuilder::with_id("quit", "Quit Tack")
        .build(app)
        .map_err(|e| e.to_string())?;
    let menu = Menu::with_items(
        app,
        &[
            &open,
            &nav_today,
            &nav_overdue,
            &PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?,
            &tasks_sub,
            &PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?,
            &quick,
            &clip,
            &PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?,
            &live,
            &dnd,
            &PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?,
            &updates,
            &quit,
        ],
    )
    .map_err(|e| e.to_string())?;
    let mut tray = TrayIconBuilder::new().menu(&menu).on_menu_event(|app, event| {
        let id = event.id().as_ref();
        match id {
            "open" => show_and_focus(app),
            "quick_add" => {
                show_and_focus(app);
                let _ = app.emit("quick-add", ());
            }
            "quick_add_clipboard" => {
                show_and_focus(app);
                let _ = app.emit("quick-add-clipboard", ());
            }
            "nav_today" => {
                show_and_focus(app);
                let _ = app.emit("tray-navigate", "today");
            }
            "nav_overdue" => {
                show_and_focus(app);
                let _ = app.emit("tray-navigate", "overdue");
            }
            // these stay in the background; the ui flips the setting and
            // syncs the checked state back via update_tray
            "toggle_live" => {
                let _ = app.emit("tray-toggle-live", ());
            }
            "toggle_dnd" => {
                let _ = app.emit("tray-toggle-dnd", ());
            }
            "check_updates" => {
                show_and_focus(app);
                let _ = app.emit("check-updates", ());
            }
            "quit" => app.exit(0),
            _ => {
                if let Some(task_id) = id.strip_prefix("task:") {
                    show_and_focus(app);
                    let _ = app.emit("focus-task", task_id.to_string());
                }
            }
        }
    });
    // template icon: macOS tints it from the alpha channel, so the white logo
    // stays visible on both light and dark menu bars
    let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../../assets/logos/tray.png"))
        .map_err(|e| e.to_string())?;
    tray = tray.icon(tray_icon).icon_as_template(true);
    let tray = tray.build(app).map_err(|e| e.to_string())?;
    app.manage(Mutex::new(TrayItems {
        tray,
        tasks: tasks_sub,
        live,
        dnd,
    }));
    Ok(())
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
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
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
            update_tray,
            live::live_start,
            live::live_stop,
            live::live_status,
            live::live_presence,
            live::hash_live_password,
            agent::agent_start,
            agent::agent_stop,
            agent::agent_status,
            agent::agent_result
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
            // close hides to the tray instead of quitting; only the tray's
            // Quit item actually exits
            if let Some(window) = app.get_webview_window("main") {
                let w = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w.hide();
                    }
                });
            }
            setup_tray(app.handle())?;
            // global quick-add: focus the app and drop it on the new-task dialog
            use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
            app.global_shortcut()
                .on_shortcut("CmdOrCtrl+Alt+T", |app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        show_and_focus(app);
                        let _ = app.emit("quick-add", ());
                    }
                })
                .map_err(|e| e.to_string())?;
            let hub = Arc::new(live::LiveHub::default());
            app.manage(live::LiveState {
                server: Mutex::new(None),
                hub: hub.clone(),
            });
            app.manage(agent::AgentState {
                server: Mutex::new(None),
                pending: Mutex::new(std::collections::HashMap::new()),
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

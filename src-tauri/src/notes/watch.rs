use notify::{EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Emitter;

// notes live as plain .md files in a user-chosen folder; this watcher
// mirrors start_db_reporter's quiet-window pattern so a burst of cli
// writes becomes a single notes-changed event for the gui

static LAST_DIR: Mutex<Option<String>> = Mutex::new(None);

// the gui calls this on startup and whenever the notes folder changes
#[tauri::command]
pub fn watch_notes_dir(app: tauri::AppHandle, path: String) -> Result<(), String> {
    // same dir already watched: re-watching would stack duplicate threads
    {
        let mut last = LAST_DIR.lock().map_err(|_| "lock poisoned")?;
        if last.as_deref() == Some(path.as_str()) {
            return Ok(());
        }
        *last = Some(path.clone());
    }

    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(format!("notes folder does not exist: {}", path));
    }
    // the preview serves pasted images through our own uri scheme; it needs the
    // current notes root to resolve requests
    super::asset::set_root(&path);

    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, _>| {
        if let Ok(event) = res {
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    let _ = tx.send(());
                }
                _ => {}
            }
        }
    })
    .map_err(|e| e.to_string())?;
    watcher
        .watch(&dir, RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;
    // keep the watcher alive for the app lifetime (same trick as lib.rs)
    std::mem::forget(watcher);

    let emitter_app = app.clone();
    std::thread::spawn(move || {
        let quiet = std::time::Duration::from_millis(800);
        loop {
            // first event wakes the loop; further events within the quiet
            // window are absorbed so one save burst emits once
            match rx.recv() {
                Ok(()) => {}
                Err(_) => return,
            }
            loop {
                match rx.recv_timeout(quiet) {
                    Ok(()) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                    Err(_) => return,
                }
            }
            let _ = emitter_app.emit("notes-changed", ());
        }
    });

    Ok(())
}

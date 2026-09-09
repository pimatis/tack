use crate::live::LiveHub;
use notify::{EventKind, RecursiveMode, Watcher};
use std::sync::Arc;
use tauri::{Emitter, Manager};

// one reporter owns the db change signal: the file watcher feeds it events,
// and a 5s timer backstops writes fsevents misses (the gui's sqlx pool
// appends to the wal without firing events). the reporter waits for a quiet
// window, then reports only when the wal actually grew: appends are real
// data changes, while checkpoint cycles (wal absorbed into the db and
// recreated empty) and shm touches are not
pub(crate) fn start_db_reporter(app: tauri::AppHandle, hub: Arc<LiveHub>) {
    let data_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let watch_dir = data_dir.clone();
    let backups_dir = watch_dir.join("backups");
    let backups_for_event = backups_dir.clone();
    // ensure the backups dir exists so the watcher has something to watch
    let _ = std::fs::create_dir_all(&backups_dir);

    let (tx, rx) = std::sync::mpsc::channel::<()>();

    let reporter_app = app.clone();
    let watcher_tx = tx.clone();
    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, _>| {
        if let Ok(event) = res {
            let is_db_change = event.paths.iter().any(|p| {
                p.file_name().map(|n| n == "tack.db" || n == "tack.db-wal" || n == "tack.db-shm").unwrap_or(false)
            });
            let is_backup_change = event.paths.iter().any(|p| p.starts_with(&backups_for_event));
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    if is_db_change {
                        let _ = watcher_tx.send(());
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

    std::thread::spawn(move || {
        // backstop tick: catches writes that never fired a file event
        let poll_tx = tx.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            let _ = poll_tx.send(());
        });

        let quiet = std::time::Duration::from_secs(2);
        let wal = data_dir.join("tack.db-wal");
        let mut stored_len: Option<u64> = None;
        let mut quiet_since: Option<std::time::Instant> = None;
        loop {
            match rx.recv_timeout(std::time::Duration::from_secs(1)) {
                Ok(()) => quiet_since = Some(std::time::Instant::now()),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
            }
            let Some(since) = quiet_since else {
                continue;
            };
            if since.elapsed() < quiet {
                continue;
            }
            quiet_since = None;
            let len = std::fs::metadata(&wal).ok().map(|m| m.len());
            // the wal grows only on real writes; a checkpoint removes and
            // recreates it empty, which is the same data already reported
            let grew = match (stored_len, len) {
                (Some(prev), Some(cur)) => cur > prev,
                _ => false,
            };
            if let Some(len) = len {
                stored_len = Some(len);
            }
            if !grew {
                continue;
            }
            let _ = reporter_app.emit("db-changed", ());
            hub.notify();
        }
    });
}

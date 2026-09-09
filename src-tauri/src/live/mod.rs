mod attachments;
mod auth;
mod backups;
mod events;
mod hub;
mod http;
mod query;
mod server;

pub use hub::LiveHub;
pub use server::{LiveState, LiveStatus};

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub type Result<T> = std::result::Result<T, String>;

// shared state every request handler receives
#[derive(Clone)]
struct Ctx {
    db_path: PathBuf,
    attachments: PathBuf,
    frontend: PathBuf,
    hub: Arc<LiveHub>,
    // how long /api/events holds a quiet poll before answering
    events_timeout: Duration,
    // close flags for in-flight /api/events/stream connections
    sse: Arc<Mutex<Vec<Arc<AtomicBool>>>>,
}

#[tauri::command]
pub fn live_start(app: tauri::AppHandle, port: u16) -> Result<LiveStatus> {
    server::start(&app, port)
}

#[tauri::command]
pub fn live_stop(app: tauri::AppHandle) -> Result<()> {
    server::stop(&app)
}

#[tauri::command]
pub fn live_status(app: tauri::AppHandle) -> Option<LiveStatus> {
    server::status(&app)
}

#[tauri::command]
pub fn hash_live_password(password: String) -> Result<auth::PasswordHash> {
    auth::hash_live_password(password)
}

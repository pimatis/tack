// shared backup module: path helpers, db connection, and re-exports
mod create;
mod list;
mod restore;

pub use create::create_backup;
pub use list::list_backups;
pub use restore::{delete_backup, restore_backup};

use rusqlite::Connection;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub type Result<T> = std::result::Result<T, String>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub name: String,
    pub created_at: String,
    pub size_bytes: u64,
}

fn data_dir(db_path: &Path) -> PathBuf {
    db_path.parent().unwrap_or(Path::new(".")).to_path_buf()
}

fn backups_dir(db_path: &Path) -> PathBuf {
    data_dir(db_path).join("backups")
}

fn attachments_dir(db_path: &Path) -> PathBuf {
    data_dir(db_path).join("attachments")
}

fn backup_dirs(db_path: &Path) -> Result<Vec<PathBuf>> {
    let dir = backups_dir(db_path);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut dirs: Vec<PathBuf> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("backup-"))
                .unwrap_or(false)
        })
        .collect();
    // descending: newest first
    dirs.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    Ok(dirs)
}

fn open_conn(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    Ok(conn)
}

use super::{backup_dirs, BackupInfo, Result};
use std::fs;
use std::path::Path;

fn dir_size(dir: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let size = if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                dir_size(&entry.path())
            } else {
                entry.metadata().map(|m| m.len()).unwrap_or(0)
            };
            total += size;
        }
    }
    total
}

fn created_at_from_name(name: &str) -> String {
    let base = name.trim_start_matches("backup-");
    // second or millisecond precision (second names exist from older versions)
    let parsed = chrono::NaiveDateTime::parse_from_str(base, "%Y%m%d-%H%M%S%.3f")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(base, "%Y%m%d-%H%M%S"));
    parsed
        .map(|dt| dt.and_utc().to_rfc3339())
        .unwrap_or_else(|_| base.to_string())
}

pub fn list_backups(db_path: &Path) -> Result<Vec<BackupInfo>> {
    Ok(backup_dirs(db_path)?
        .into_iter()
        .filter_map(|dir| {
            let name = dir.file_name()?.to_str()?.to_string();
            if !dir.join("tack.db").exists() {
                return None;
            }
            Some(BackupInfo {
                created_at: created_at_from_name(&name),
                size_bytes: dir_size(&dir),
                name,
            })
        })
        .collect())
}

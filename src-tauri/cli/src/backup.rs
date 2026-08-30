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

fn copy_dir(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = dest.join(entry.file_name());
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

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

// consistent snapshot via VACUUM INTO (safe while wal is active)
pub fn create_backup(db_path: &Path, keep: usize) -> Result<String> {
    if !db_path.exists() {
        return Err("database not found".to_string());
    }
    let dir = backups_dir(db_path);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    // millisecond suffix avoids collision when two backups land in the same second
    let name = format!("backup-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S%.3f"));
    let dest = dir.join(&name);

    // build the snapshot in a staging dir, then rename into place only on success;
    // a failed vacuum or copy leaves no partial backup behind
    let staging = dir.join(format!(".staging-{}", name));
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging).map_err(|e| e.to_string())?;

    let result = (|| -> Result<()> {
        let conn = open_conn(db_path)?;
        conn.execute(
            "VACUUM INTO ?1",
            [staging.join("tack.db").to_string_lossy().as_ref()],
        )
        .map_err(|e| e.to_string())?;
        drop(conn);

        let attachments = attachments_dir(db_path);
        if attachments.exists() {
            copy_dir(&attachments, &staging.join("attachments"))?;
        }
        Ok(())
    })();

    match result {
        Ok(()) => {
            fs::rename(&staging, &dest).map_err(|e| e.to_string())?;
            rotate_backups(db_path, keep)?;
            Ok(name)
        }
        Err(e) => {
            let _ = fs::remove_dir_all(&staging);
            Err(e)
        }
    }
}

fn rotate_backups(db_path: &Path, keep: usize) -> Result<()> {
    if keep == 0 {
        return Ok(());
    }
    for dir in backup_dirs(db_path)?.into_iter().skip(keep) {
        fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
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

pub fn delete_backup(db_path: &Path, name: &str) -> Result<()> {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("invalid backup name".to_string());
    }
    let dir = backups_dir(db_path).join(name);
    if !dir.join("tack.db").exists() {
        return Err(format!("backup '{}' not found", name));
    }
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())
}

fn open_conn(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.busy_timeout(Duration::from_secs(5)).map_err(|e| e.to_string())?;
    Ok(conn)
}

pub fn restore_backup(db_path: &Path, name: &str) -> Result<()> {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("invalid backup name".to_string());
    }
    let src = backups_dir(db_path).join(name);
    if !src.join("tack.db").exists() {
        return Err(format!("backup '{}' not found", name));
    }

    // flush live wal into main db file before overwriting it
    if db_path.exists() {
        // ponytail: checkpoint does not block concurrent sqlx writers, tiny race window
        // remains on the file copy; single-user desktop app keeps this acceptable
        let conn = open_conn(db_path)?;
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        drop(conn);
    }

    // copy into temp files first, then swap into place so a failed copy never
    // leaves the live db or attachments half-restored
    let tmp_db = PathBuf::from(format!("{}.restore-tmp", db_path.display()));
    let tmp_attachments = PathBuf::from(format!("{}.restore-tmp", attachments_dir(db_path).display()));
    let old_db = PathBuf::from(format!("{}.restore-old", db_path.display()));
    let old_attachments = PathBuf::from(format!("{}.restore-old", attachments_dir(db_path).display()));
    let _ = fs::remove_file(&tmp_db);
    let _ = fs::remove_dir_all(&tmp_attachments);
    let _ = fs::remove_file(&old_db);
    let _ = fs::remove_dir_all(&old_attachments);

    let result = (|| -> Result<()> {
        fs::copy(src.join("tack.db"), &tmp_db).map_err(|e| e.to_string())?;

        let backup_attachments = src.join("attachments");
        if backup_attachments.exists() {
            copy_dir(&backup_attachments, &tmp_attachments)?;
        }

        // swap attachments first, then the db file last so the live db only
        // changes when every other piece is already in place
        let dest_attachments = attachments_dir(db_path);
        if backup_attachments.exists() {
            if dest_attachments.exists() {
                fs::rename(&dest_attachments, &old_attachments).map_err(|e| e.to_string())?;
            }
            if let Err(e) = fs::rename(&tmp_attachments, &dest_attachments) {
                let _ = fs::rename(&old_attachments, &dest_attachments);
                return Err(e.to_string());
            }
            let _ = fs::remove_dir_all(&old_attachments);
        } else {
            let _ = fs::remove_dir_all(&tmp_attachments);
        }

        // swap db file; on failure roll back to the previous db
        if db_path.exists() {
            fs::rename(db_path, &old_db).map_err(|e| e.to_string())?;
        }
        if let Err(e) = fs::rename(&tmp_db, db_path) {
            let _ = fs::rename(&old_db, db_path);
            return Err(e.to_string());
        }
        let _ = fs::remove_file(&old_db);
        for suffix in ["-wal", "-shm"] {
            let _ = fs::remove_file(PathBuf::from(format!("{}{}", db_path.display(), suffix)));
        }

        Ok(())
    })();

    if result.is_err() {
        // best-effort cleanup of any leftover temp artifacts
        let _ = fs::remove_file(&tmp_db);
        let _ = fs::remove_dir_all(&tmp_attachments);
        let _ = fs::remove_file(&old_db);
        let _ = fs::remove_dir_all(&old_attachments);
    }
    result
}

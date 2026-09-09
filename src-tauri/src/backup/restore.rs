use super::create::copy_dir;
use super::{attachments_dir, backups_dir, open_conn, Result};
use std::fs;
use std::path::{Path, PathBuf};

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

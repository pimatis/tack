use super::{attachments_dir, backups_dir, open_conn, Result};
use std::fs;
use std::path::Path;

pub(super) fn copy_dir(src: &Path, dest: &Path) -> Result<()> {
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
    for dir in super::backup_dirs(db_path)?.into_iter().skip(keep) {
        fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

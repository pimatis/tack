use rusqlite::{Connection, params};
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, String>;

pub fn get_db_path() -> PathBuf {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    let app_dir = data_dir.join("com.pimatis.tack");
    std::fs::create_dir_all(&app_dir).ok();
    app_dir.join("tack.db")
}

pub fn connect(db_path: &PathBuf) -> Result<Connection> {
    let conn = Connection::open(db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("Failed to set WAL mode: {}", e))?;
    run_migrations(&conn)?;
    Ok(conn)
}

fn run_migrations(conn: &Connection) -> Result<()> {
    let migrations: &[(i32, &str, &str)] = &[
        (1, "create_tasks_table", include_str!("../../migrations/001_initial.sql")),
        (2, "create_projects_table", include_str!("../../migrations/002_projects.sql")),
        (3, "create_attachments_table", include_str!("../../migrations/003_attachments.sql")),
        (4, "create_labels_table", include_str!("../../migrations/004_labels.sql")),
        (5, "add_due_date_to_tasks", include_str!("../../migrations/005_due_date.sql")),
        (6, "add_task_number", include_str!("../../migrations/006_task_number.sql")),
        (7, "add_project_description", include_str!("../../migrations/007_project_description.sql")),
        (8, "add_subtasks_activity_log_sort_order", include_str!("../../migrations/008_subtasks_activity_sort.sql")),
        (9, "add_pinned_to_tasks", include_str!("../../migrations/009_pinned.sql")),
        (10, "create_settings_table", include_str!("../../migrations/010_settings.sql")),
        (11, "add_source_to_activity_log", include_str!("../../migrations/011_activity_source.sql")),
        (12, "add_deleted_at_to_tasks", include_str!("../../migrations/012_trash.sql")),
        (13, "migrate_attachment_file_path", include_str!("../../migrations/013_attachment_file_path.sql")),
        (14, "create_fts_search_index", include_str!("../../migrations/014_fts_search.sql")),
        (15, "fix_fts_triggers", include_str!("../../migrations/015_fix_fts_triggers.sql")),
    ];

    // track applied versions via the same table the tauri app uses, so
    // non-idempotent migrations (e.g. 013 column rename) only run once
    let has_tracking = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    // db without tracking table but with tack tables was fully migrated by
    // the old untracked runner or the app itself, treat everything as applied
    let has_tack_tables = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tasks'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    let applied: std::collections::HashSet<i64> = if has_tracking {
        let mut stmt = conn
            .prepare("SELECT version FROM _sqlx_migrations WHERE success = 1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, i64>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    } else if has_tack_tables {
        migrations.iter().map(|(v, _, _)| *v as i64).collect()
    } else {
        std::collections::HashSet::new()
    };

    for (version, description, sql) in migrations {
        if applied.contains(&(*version as i64)) {
            continue;
        }
        if let Err(e) = conn.execute_batch(sql) {
            let msg = e.to_string();
            if !msg.contains("duplicate column name") {
                return Err(format!("migration {} failed: {}", version, msg));
            }
        }
        // same schema sqlx creates, empty checksum is fine because sqlx run()
        // skips applied versions without comparing checksums
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _sqlx_migrations (
                version BIGINT PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                success BOOLEAN NOT NULL,
                checksum BLOB NOT NULL,
                execution_time BIGINT NOT NULL
            );",
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR IGNORE INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time)
             VALUES (?1, ?2, CURRENT_TIMESTAMP, 1, x'', 0)",
            params![*version as i64, *description],
        )
        .map_err(|e| format!("failed to record migration {}: {}", version, e))?;
    }
    Ok(())
}

pub fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn log_activity(
    conn: &Connection,
    task_id: &str,
    action: &str,
    field: Option<&str>,
    old_value: Option<&str>,
    new_value: Option<&str>,
    source: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO activity_log (id, task_id, action, field, old_value, new_value, source, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![new_id(), task_id, action, field, old_value, new_value, source, now_iso()],
    ).map_err(|e| format!("Failed to log activity: {}", e))?;
    Ok(())
}

pub fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if rows.is_empty() {
        println!("(empty)");
        return;
    }

    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let header: String = headers
        .iter()
        .enumerate()
        .map(|(i, h)| format!("{:<width$}", h, width = widths[i]))
        .collect::<Vec<_>>()
        .join("  ");
    println!("{}", header);

    let sep: String = widths
        .iter()
        .map(|w| "-".repeat(*w))
        .collect::<Vec<_>>()
        .join("  ");
    println!("{}", sep);

    for row in rows {
        let line: String = row
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let w = if i < widths.len() { widths[i] } else { cell.len() };
                format!("{:<width$}", cell, width = w)
            })
            .collect::<Vec<_>>()
            .join("  ");
        println!("{}", line);
    }
}

pub fn status_label(status: &str) -> &'static str {
    match status {
        "todo" => "Todo",
        "in_progress" => "In progress",
        "done" => "Done",
        "canceled" => "Canceled",
        _ => "Unknown",
    }
}

pub fn priority_label(priority: i32) -> &'static str {
    match priority {
        0 => "No priority",
        1 => "Urgent",
        2 => "High",
        3 => "Medium",
        4 => "Low",
        _ => "Unknown",
    }
}

pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    ).ok()
}

pub fn get_default_status(conn: &Connection) -> String {
    get_setting(conn, "defaultStatus").unwrap_or_else(|| "todo".to_string())
}

pub fn get_default_priority(conn: &Connection) -> i32 {
    get_setting(conn, "defaultPriority")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

pub fn resolve_project_id(conn: &Connection, project: Option<&str>, prefix: Option<&str>) -> Result<Option<String>> {
    if let Some(id) = project {
        return Ok(Some(id.to_string()));
    }
    if let Some(pfx) = prefix {
        let id: String = conn
            .query_row(
                "SELECT id FROM projects WHERE prefix = ?1 COLLATE NOCASE",
                params![pfx],
                |row| row.get(0),
            )
            .map_err(|_| format!("Project with prefix '{}' not found", pfx))?;
        return Ok(Some(id));
    }
    Ok(None)
}

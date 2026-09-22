use rusqlite::{params, Connection};
use sha2::{Digest, Sha384};

// apply pending migrations with a tolerant runner: tracking rows for
// migrations this binary does not know are dropped instead of erroring, so
// older binaries can open the db again (all migrations are additive, the
// leftover schema is harmless to them)
pub(crate) fn run_migrations(conn: &Connection) -> Result<(), String> {
    let migrations: &[(i32, &str, &str)] = &[
        (
            1,
            "create_tasks_table",
            include_str!("../migrations/001_initial.sql"),
        ),
        (
            2,
            "create_projects_table",
            include_str!("../migrations/002_projects.sql"),
        ),
        (
            3,
            "create_attachments_table",
            include_str!("../migrations/003_attachments.sql"),
        ),
        (
            4,
            "create_labels_table",
            include_str!("../migrations/004_labels.sql"),
        ),
        (
            5,
            "add_due_date_to_tasks",
            include_str!("../migrations/005_due_date.sql"),
        ),
        (
            6,
            "add_task_number",
            include_str!("../migrations/006_task_number.sql"),
        ),
        (
            7,
            "add_project_description",
            include_str!("../migrations/007_project_description.sql"),
        ),
        (
            8,
            "add_subtasks_activity_log_sort_order",
            include_str!("../migrations/008_subtasks_activity_sort.sql"),
        ),
        (
            9,
            "add_pinned_to_tasks",
            include_str!("../migrations/009_pinned.sql"),
        ),
        (
            10,
            "create_settings_table",
            include_str!("../migrations/010_settings.sql"),
        ),
        (
            11,
            "add_source_to_activity_log",
            include_str!("../migrations/011_activity_source.sql"),
        ),
        (
            12,
            "add_deleted_at_to_tasks",
            include_str!("../migrations/012_trash.sql"),
        ),
        (
            13,
            "migrate_attachment_file_path",
            include_str!("../migrations/013_attachment_file_path.sql"),
        ),
        (
            14,
            "create_fts_search_index",
            include_str!("../migrations/014_fts_search.sql"),
        ),
        (
            15,
            "fix_fts_triggers",
            include_str!("../migrations/015_fix_fts_triggers.sql"),
        ),
        (
            16,
            "add_end_date_to_tasks",
            include_str!("../migrations/016_end_date.sql"),
        ),
        (
            17,
            "create_notes_fts_index",
            include_str!("../migrations/017_notes_fts.sql"),
        ),
        (
            18,
            "create_task_notes_table",
            include_str!("../migrations/018_task_notes.sql"),
        ),
        (
            19,
            "add_task_reminders",
            include_str!("../migrations/019_reminders.sql"),
        ),
        (
            20,
            "expand_task_search_scope",
            include_str!("../migrations/020_task_search_scope.sql"),
        ),
        (
            21,
            "create_notes_index_tables",
            include_str!("../migrations/021_notes_index_tables.sql"),
        ),
    ];
    let known: std::collections::HashSet<i64> =
        migrations.iter().map(|(v, _, _)| *v as i64).collect();

    // track applied versions via the same table the sqlx-based runner
    // (tauri-plugin-sql) used, so older binaries and the cli stay compatible
    let has_tracking = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    // db without tracking table but with tack tables was fully migrated by
    // the old untracked runner, treat everything as applied
    let has_tack_tables = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tasks'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    let mut applied: std::collections::HashSet<i64> = if has_tracking {
        let mut stmt = conn
            .prepare("SELECT version FROM _sqlx_migrations WHERE success = 1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, i64>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    } else if has_tack_tables {
        known.clone()
    } else {
        std::collections::HashSet::new()
    };

    // drop tracking rows for migrations this binary does not know, so a
    // downgrade opens cleanly instead of erroring (schema stays ahead)
    if has_tracking {
        for version in applied.difference(&known) {
            conn.execute(
                "DELETE FROM _sqlx_migrations WHERE version = ?1",
                params![version],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    applied.retain(|v| known.contains(v));

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
        // same schema sqlx creates; checksum matches sqlx's sha384 so older
        // binaries using tauri-plugin-sql can still verify it
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
        let checksum: Vec<u8> = Sha384::digest(sql.as_bytes()).to_vec();
        conn.execute(
            "INSERT OR IGNORE INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time)
             VALUES (?1, ?2, CURRENT_TIMESTAMP, 1, ?3, 0)",
            params![*version as i64, *description, checksum],
        )
        .map_err(|e| format!("failed to record migration {}: {}", version, e))?;
    }
    Ok(())
}

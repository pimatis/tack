use rusqlite::Connection;
use serde_json::json;

use crate::db::{now_iso, Result};

// workspace totals; tracked time banks any currently-running timer segments
pub fn stats(conn: &Connection, json: bool) -> Result<()> {
    let now = now_iso();
    conn.execute(
        "UPDATE tasks SET tracked_seconds = tracked_seconds + MAX(0, CAST((julianday(?1) - julianday(timer_started_at)) * 86400 AS INTEGER)), timer_started_at = NULL WHERE timer_started_at IS NOT NULL AND status IN ('done', 'canceled')",
        [now],
    )
    .ok();

    let (open, done, canceled, overdue, tracked, running): (
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
    ) = conn
        .query_row(
            "SELECT
                COALESCE(SUM(CASE WHEN status NOT IN ('done', 'canceled') THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'done' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'canceled' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status NOT IN ('done', 'canceled') AND due_date IS NOT NULL AND due_date < date('now', 'localtime') THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(tracked_seconds), 0) + COALESCE(SUM(CASE WHEN timer_started_at IS NOT NULL THEN MAX(0, CAST((julianday(?1) - julianday(timer_started_at)) * 86400 AS INTEGER)) ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN timer_started_at IS NOT NULL THEN 1 ELSE 0 END), 0)
             FROM tasks WHERE deleted_at IS NULL",
            [now_iso()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .map_err(|e| format!("Failed to read stats: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT p.prefix, p.name,
                COALESCE(SUM(CASE WHEN t.status NOT IN ('done', 'canceled') THEN 1 ELSE 0 END), 0) AS open,
                COALESCE(SUM(CASE WHEN t.status = 'done' THEN 1 ELSE 0 END), 0) AS done
             FROM projects p
             LEFT JOIN tasks t ON t.project_id = p.id AND t.deleted_at IS NULL
             GROUP BY p.id ORDER BY open DESC, p.name",
        )
        .map_err(|e| e.to_string())?;
    let projects: Vec<(String, String, i64, i64)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // workload per assignee: humans see nothing here until agents exist
    let agents: Vec<(String, i64, i64)> = conn
        .prepare(
            "SELECT t.assignee,
                COALESCE(SUM(CASE WHEN t.status NOT IN ('done', 'canceled') THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN t.status = 'done' THEN 1 ELSE 0 END), 0)
             FROM tasks t
             WHERE t.assignee IS NOT NULL AND t.deleted_at IS NULL
             GROUP BY t.assignee ORDER BY 2 DESC, t.assignee",
        )
        .and_then(|mut s| {
            s.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    if json {
        let items: Vec<_> = projects
            .iter()
            .map(|(pfx, name, o, d)| json!({ "prefix": pfx, "name": name, "open": o, "done": d }))
            .collect();
        let agent_items: Vec<_> = agents
            .iter()
            .map(|(name, o, d)| json!({ "name": name, "open": o, "done": d }))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "open": open,
                "done": done,
                "canceled": canceled,
                "overdue": overdue,
                "tracked_seconds": tracked,
                "running_timers": running,
                "projects": items,
                "agents": agent_items,
            }))
            .map_err(|e| e.to_string())?
        );
        return Ok(());
    }

    println!("Open:     {}", open);
    println!("Done:     {}", done);
    println!("Canceled: {}", canceled);
    println!("Overdue:  {}", overdue);
    let h = tracked / 3600;
    let m = (tracked % 3600) / 60;
    let tracked_label = if h > 0 { format!("{}h{}m", h, m) } else { format!("{}m", m) };
    println!("Tracked:  {}{}", tracked_label, if running > 0 { format!(" ({} running)", running) } else { String::new() });
    if !projects.is_empty() {
        println!("\nBy project:");
        for (pfx, name, o, d) in &projects {
            println!("  {:<5} {:<24} {} open / {} done", pfx, name, o, d);
        }
    }
    if !agents.is_empty() {
        println!("\nBy agent:");
        for (name, o, d) in &agents {
            println!("  {:<20} {} open / {} done", name, o, d);
        }
    }
    Ok(())
}

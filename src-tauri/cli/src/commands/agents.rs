use crate::db::*;
use rusqlite::Connection;
use serde_json::json;

// the registry is populated lazily: an agent row appears the first time it is
// assigned or writes to the activity log. harness is whoever registered it
const AGENT_STATS: &str = "
    (SELECT COUNT(*) FROM tasks t WHERE t.assignee = a.name COLLATE NOCASE
      AND t.deleted_at IS NULL AND t.status NOT IN ('done','canceled')),
    (SELECT COUNT(*) FROM tasks t WHERE t.assignee = a.name COLLATE NOCASE
      AND t.deleted_at IS NULL AND t.status = 'in_progress'),
    (SELECT COUNT(*) FROM tasks t WHERE t.assignee = a.name COLLATE NOCASE
      AND t.deleted_at IS NULL AND t.status = 'done')";

pub fn list(conn: &Connection, json: bool) -> Result<()> {
    let sql = format!(
        "SELECT a.name, a.harness, a.color, a.last_seen_at, {} FROM agents a ORDER BY 5 DESC, a.name",
        AGENT_STATS
    );
    let rows: Vec<(String, Option<String>, String, Option<String>, i64, i64, i64)> = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to list agents: {}", e))?
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?))
        })
        .map_err(|e| format!("Failed to list agents: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    if json {
        let items: Vec<_> = rows
            .iter()
            .map(|(name, harness, color, seen, open, wip, done)| {
                json!({
                    "name": name,
                    "harness": harness,
                    "color": color,
                    "last_seen_at": seen,
                    "open_tasks": open,
                    "in_progress": wip,
                    "done_tasks": done,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "agents": items })).map_err(|e| e.to_string())?
        );
        return Ok(());
    }

    let table: Vec<Vec<String>> = rows
        .iter()
        .map(|(name, harness, _color, seen, open, wip, done)| {
            vec![
                name.clone(),
                harness.clone().unwrap_or("-".to_string()),
                open.to_string(),
                wip.to_string(),
                done.to_string(),
                seen.clone().unwrap_or("-".to_string()),
            ]
        })
        .collect();
    print_table(
        &["AGENT", "HARNESS", "OPEN", "IN PROGRESS", "DONE", "LAST SEEN"],
        &table,
    );
    Ok(())
}

// one agent in full: registry info, workload and the last 10 things on its tasks
pub fn show(conn: &Connection, json: bool, name: &str) -> Result<()> {
    let sql = format!(
        "SELECT a.name, a.harness, a.color, a.created_at, a.last_seen_at, {} FROM agents a WHERE a.name = ?1 COLLATE NOCASE",
        AGENT_STATS
    );
    let agent = conn
        .query_row(&sql, rusqlite::params![name], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, i64>(7)?,
            ))
        })
        .map_err(|_| format!("Agent '{}' not found", name))?;
    let (name, harness, color, created, seen, open, wip, done) = agent;

    let recent: Vec<(String, String, String, Option<String>, String)> = conn
        .prepare(
            "SELECT a.action, a.created_at, t.title, a.new_value, a.source
             FROM activity_log a JOIN tasks t ON t.id = a.task_id
             WHERE t.assignee = ?1 COLLATE NOCASE AND t.deleted_at IS NULL
             ORDER BY a.created_at DESC LIMIT 10",
        )
        .and_then(|mut s| {
            s.query_map(rusqlite::params![name], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    if json {
        let entries: Vec<_> = recent
            .iter()
            .map(|(action, at, title, value, source)| {
                json!({ "action": action, "created_at": at, "task": title, "value": value, "source": source })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "agent": {
                    "name": name,
                    "harness": harness,
                    "color": color,
                    "created_at": created,
                    "last_seen_at": seen,
                    "open_tasks": open,
                    "in_progress": wip,
                    "done_tasks": done,
                },
                "recent_activity": entries,
            }))
            .map_err(|e| e.to_string())?
        );
        return Ok(());
    }

    println!("Agent:       {}", name);
    println!("Harness:     {}", harness.unwrap_or("-".to_string()));
    println!("Color:       {}", color);
    println!("Registered:  {}", created);
    println!("Last seen:   {}", seen.unwrap_or("-".to_string()));
    println!("Tasks:       {} open / {} in progress / {} done", open, wip, done);
    if !recent.is_empty() {
        println!("\nRecent:");
        for (action, at, title, _value, source) in &recent {
            println!("  {} {} on \"{}\" ({})", at, action.replace('_', " "), title, source);
        }
    }
    Ok(())
}

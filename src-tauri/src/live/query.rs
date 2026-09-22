use super::http::json_response;
use super::Ctx;
use crate::attachments::base64_encode;
use rusqlite::params_from_iter;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Read;
use std::path::Path;
use std::time::Duration;
use tiny_http::{Request, Response, StatusCode};

#[derive(Deserialize)]
struct QueryPayload {
    sql: String,
    #[serde(default)]
    params: Vec<Value>,
}

// reads stop here: the live server returns the app's own tables, and a client
// that forgets a where clause must not stream a whole vault of rows back
const MAX_ROWS: usize = 10_000;

// tables the app writes through the live server. tasks_fts is missing on
// purpose: triggers own that index and a direct write desyncs search
const WRITABLE_TABLES: [&str; 12] = [
    "activity_log",
    "labels",
    "note_links",
    "notes_fts",
    "notes_index_meta",
    "projects",
    "settings",
    "subtasks",
    "task_attachments",
    "task_labels",
    "task_notes",
    "tasks",
];

enum Statement {
    Read,
    Write,
}

// the endpoint carries the app's own sql, so only the statement kinds and
// tables the app itself uses get through; ddl, pragma, attach and anything
// else is refused before sqlite sees it
fn classify(sql: &str) -> Result<Statement, String> {
    // sqlite compiles one statement per prepare and drops the rest of the
    // text, so a stacked statement would silently run only its first half
    if sql.trim().trim_end_matches(';').contains(';') {
        return Err("only one statement per request is allowed".to_string());
    }

    let mut words = sql.split_whitespace();
    let verb = words.next().unwrap_or_default().to_ascii_uppercase();
    let table = match verb.as_str() {
        "SELECT" => return Ok(Statement::Read),
        // insert [or replace|or ignore] into <table>
        "INSERT" => words
            .find(|w| w.eq_ignore_ascii_case("into"))
            .and_then(|_| words.next()),
        // update [or rollback|or abort|or fail|or ignore|or replace] <table>
        "UPDATE" => {
            let first = words.next();
            if first.is_some_and(|w| w.eq_ignore_ascii_case("or")) {
                words.nth(1)
            } else {
                first
            }
        }
        // delete from <table>
        "DELETE" => words
            .find(|w| w.eq_ignore_ascii_case("from"))
            .and_then(|_| words.next()),
        _ => {
            return Err(format!(
                "statement type not allowed: {}",
                if verb.is_empty() { "<empty>" } else { &verb }
            ))
        }
    };

    let Some(table) = table else {
        return Err("could not tell which table the statement writes to".to_string());
    };
    let table = table
        .trim_matches(|c| c == '"' || c == '`' || c == '[' || c == ']')
        .split(|c| c == '(' || c == ';' || c == ',')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !WRITABLE_TABLES.contains(&table.as_str()) {
        return Err(format!("writes to '{}' are not allowed", table));
    }
    Ok(Statement::Write)
}

pub(super) fn run_query(request: &mut Request, ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    match execute_query(request, ctx) {
        Ok(body) => json_response(StatusCode(200), body),
        Err(e) => json_response(StatusCode(400), json!({ "error": e })),
    }
}

pub(super) fn execute_query(request: &mut Request, ctx: &Ctx) -> super::Result<Value> {
    let mut body = String::new();
    request
        .as_reader()
        .take(1024 * 1024)
        .read_to_string(&mut body)
        .map_err(|e| format!("Failed to read request body: {}", e))?;
    let payload: QueryPayload =
        serde_json::from_str(&body).map_err(|e| format!("Invalid request body: {}", e))?;

    // translate sqlx-style $n placeholders to rusqlite positional ones
    let sql = translate_placeholders(&payload.sql);
    let values: Vec<rusqlite::types::Value> = payload.params.iter().map(to_sqlite_value).collect();

    let read = matches!(classify(&sql)?, Statement::Read);

    let conn = open_conn(&ctx.db_path)?;
    if read {
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let names: Vec<String> = stmt.column_names().iter().map(|n| n.to_string()).collect();
        let mut rows = stmt
            .query(params_from_iter(values.iter()))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        let mut truncated = false;
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            if out.len() == MAX_ROWS {
                truncated = true;
                break;
            }
            let mut obj = serde_json::Map::new();
            for (i, name) in names.iter().enumerate() {
                let v: rusqlite::types::Value = row.get(i).map_err(|e| e.to_string())?;
                obj.insert(name.clone(), sqlite_to_json(v));
            }
            out.push(Value::Object(obj));
        }
        Ok(json!({ "rows": out, "truncated": truncated }))
    } else {
        let affected = conn
            .execute(&sql, params_from_iter(values.iter()))
            .map_err(|e| e.to_string())?;
        // only a statement that touched a row is a real change; an empty
        // UPDATE must stay silent or clients refreshing on db-changed notify
        // in a loop
        if affected > 0 {
            ctx.hub.notify();
        }
        Ok(json!({ "rowsAffected": affected }))
    }
}

// rewrite $n (sqlx style, used by the frontend) to ?n (rusqlite numbered),
// so a placeholder referenced twice binds the same value once
pub(super) fn translate_placeholders(sql: &str) -> String {
    let chars: Vec<char> = sql.chars().collect();
    let mut out = String::with_capacity(sql.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '$' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 {
                out.push('?');
                out.extend(chars[i + 1..j].iter());
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn to_sqlite_value(v: &Value) -> rusqlite::types::Value {
    match v {
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(b) => rusqlite::types::Value::Integer(*b as i64),
        Value::Number(n) => n
            .as_i64()
            .map(rusqlite::types::Value::Integer)
            .unwrap_or_else(|| rusqlite::types::Value::Real(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        _ => rusqlite::types::Value::Null,
    }
}

fn sqlite_to_json(v: rusqlite::types::Value) -> Value {
    match v {
        rusqlite::types::Value::Null => Value::Null,
        rusqlite::types::Value::Integer(i) => json!(i),
        rusqlite::types::Value::Real(r) => json!(r),
        rusqlite::types::Value::Text(t) => json!(t),
        rusqlite::types::Value::Blob(b) => json!(base64_encode(&b)),
    }
}

fn open_conn(path: &Path) -> super::Result<rusqlite::Connection> {
    let conn =
        rusqlite::Connection::open(path).map_err(|e| format!("Failed to open database: {}", e))?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_pass() {
        assert!(matches!(
            classify("SELECT id FROM tasks"),
            Ok(Statement::Read)
        ));
        assert!(matches!(
            classify("  select\n\tcount(*) FROM tasks"),
            Ok(Statement::Read)
        ));
    }

    #[test]
    fn ddl_and_unknown_verbs_are_refused() {
        for sql in [
            "DROP TABLE tasks",
            "ALTER TABLE tasks ADD COLUMN x TEXT",
            "CREATE TABLE evil (a TEXT)",
            "PRAGMA journal_mode = DELETE",
            "ATTACH DATABASE '/tmp/other.db' AS other",
            "VACUUM",
            "BEGIN TRANSACTION",
            "REPLACE INTO tasks (id) VALUES ('a')",
            "",
            "   ",
        ] {
            assert!(classify(sql).is_err(), "'{}' should be refused", sql);
        }
    }

    #[test]
    fn writes_reach_the_app_tables_only() {
        for sql in [
            "INSERT INTO tasks (id) VALUES ('a')",
            "insert into settings (key, value) values ('a', 'b')",
            "insert into note_links(src, dst, kind) values ('a', 'b', 'mention')",
            "UPDATE tasks SET title = 'x' WHERE id = 'a'",
            "UPDATE OR IGNORE subtasks SET completed = 1",
            "UPDATE \"task_labels\" SET label_id = 'a'",
            "DELETE FROM task_labels WHERE task_id = 'a'",
            "INSERT INTO notes_fts (path) VALUES ('a')",
        ] {
            assert!(
                matches!(classify(sql), Ok(Statement::Write)),
                "'{}' should pass",
                sql
            );
        }

        for sql in [
            "DELETE FROM tasks_fts",
            "INSERT INTO sqlite_master (name) VALUES ('x')",
            "UPDATE _sqlx_migrations SET success = 1",
            "DELETE FROM unknown_table",
            "INSERT INTO tasks_backup (id) VALUES ('a')",
        ] {
            assert!(classify(sql).is_err(), "'{}' should be refused", sql);
        }
    }

    #[test]
    fn stacked_statements_are_refused() {
        for sql in [
            "SELECT 1; DROP TABLE tasks",
            "UPDATE tasks SET title = 'x'; DELETE FROM tasks",
            "SELECT 1;SELECT 2",
        ] {
            assert!(classify(sql).is_err(), "'{}' should be refused", sql);
        }
        // a trailing semicolon is how the app ends its own sql
        assert!(matches!(classify("SELECT 1;"), Ok(Statement::Read)));
        assert!(matches!(classify("SELECT 1 ;"), Ok(Statement::Read)));
    }
}

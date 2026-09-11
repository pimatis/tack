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

pub(super) fn run_query(
    request: &mut Request,
    ctx: &Ctx,
    is_select: bool,
) -> Response<std::io::Cursor<Vec<u8>>> {
    match execute_query(request, ctx, is_select) {
        Ok(body) => json_response(StatusCode(200), body),
        Err(e) => json_response(StatusCode(400), json!({ "error": e })),
    }
}

pub(super) fn execute_query(
    request: &mut Request,
    ctx: &Ctx,
    is_select: bool,
) -> super::Result<Value> {
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

    let conn = open_conn(&ctx.db_path)?;
    if is_select {
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let names: Vec<String> = stmt.column_names().iter().map(|n| n.to_string()).collect();
        let mut rows = stmt
            .query(params_from_iter(values.iter()))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let mut obj = serde_json::Map::new();
            for (i, name) in names.iter().enumerate() {
                let v: rusqlite::types::Value = row.get(i).map_err(|e| e.to_string())?;
                obj.insert(name.clone(), sqlite_to_json(v));
            }
            out.push(Value::Object(obj));
        }
        Ok(json!({ "rows": out }))
    } else {
        let affected = conn
            .execute(&sql, params_from_iter(values.iter()))
            .map_err(|e| e.to_string())?;
        // only a statement that touched a row is a real change; no-op ddl
        // (CREATE TABLE IF NOT EXISTS, empty UPDATEs) must stay silent or
        // clients refreshing on db-changed would notify in a loop
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

use crate::db::*;
use rusqlite::Connection;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const DEFAULT_PORT: u16 = 17891;

pub fn port(conn: &Connection) -> u16 {
    get_setting(conn, "agentPort")
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

fn connect(port: u16) -> Result<TcpStream> {
    let stream = TcpStream::connect(("127.0.0.1", port)).map_err(|_| {
        format!(
            "agent server is not reachable on port {} - is the tack app open with agents enabled?",
            port
        )
    })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|e| format!("socket setup failed: {}", e))?;
    Ok(stream)
}

// minimal http/1.1 client for the agent server; responses close the
// connection, so the body is whatever follows the header block
pub fn request(
    conn: &Connection,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<(u16, Vec<u8>)> {
    let port = port(conn);
    let mut stream = connect(port)?;
    let body = body.unwrap_or("");
    // named agents get their writes attributed in the activity log
    let agent_header = crate::db::agent_name()
        .map(|n| format!("X-Tack-Agent: {}\r\n", n))
        .unwrap_or_default();
    write!(
        stream,
        "{} {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAccept: */*\r\nContent-Type: application/json\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        method,
        path,
        port,
        agent_header,
        body.len(),
        body
    )
    .map_err(|e| format!("request failed: {}", e))?;
    stream.flush().map_err(|e| e.to_string())?;

    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|e| format!("response failed: {}", e))?;
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| "malformed response from agent server".to_string())?;
    let headers = String::from_utf8_lossy(&raw[..split]);
    let status: u16 = headers
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| "malformed status line from agent server".to_string())?;
    let body = raw[split + 4..].to_vec();
    let body = if headers
        .to_lowercase()
        .contains("transfer-encoding: chunked")
    {
        decode_chunked(&body)?
    } else {
        body
    };
    Ok((status, body))
}

fn decode_chunked(data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut pos = 0;
    loop {
        let end = data[pos..]
            .windows(2)
            .position(|w| w == b"\r\n")
            .map(|i| pos + i)
            .ok_or_else(|| "truncated chunk size".to_string())?;
        let size_str = std::str::from_utf8(&data[pos..end])
            .map_err(|_| "bad chunk size".to_string())?;
        let size = usize::from_str_radix(size_str.trim(), 16)
            .map_err(|_| "bad chunk size".to_string())?;
        pos = end + 2;
        if size == 0 {
            break;
        }
        if pos + size > data.len() {
            return Err("truncated chunk data".to_string());
        }
        out.extend_from_slice(&data[pos..pos + size]);
        pos += size + 2;
    }
    Ok(out)
}

pub fn request_json(conn: &Connection, method: &str, path: &str, body: Option<&str>) -> Result<Value> {
    let (status, bytes) = request(conn, method, path, body)?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|e| format!("bad json from agent server: {}", e))?;
    if !(200..300).contains(&status) {
        let msg = value["error"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("agent server returned {}", status));
        return Err(msg);
    }
    Ok(value)
}

pub fn action(conn: &Connection, name: &str, params: Value) -> Result<Value> {
    let body = serde_json::to_string(&serde_json::json!({ "action": name, "params": params }))
        .map_err(|e| e.to_string())?;
    request_json(conn, "POST", "/api/agent/action", Some(&body))
}

// shared print: human-readable summary unless --json asked for the raw result
pub fn print_result(result: Value, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if result["ok"].as_bool().unwrap_or(false) {
        println!("{}", result["action"].as_str().unwrap_or("ok"));
        if !result["result"].is_null() {
            println!(
                "{}",
                serde_json::to_string_pretty(&result["result"]).map_err(|e| e.to_string())?
            );
        }
        return Ok(());
    }
    if result["denied"].as_bool().unwrap_or(false) {
        println!(
            "denied by user: {}",
            result["summary"].as_str().unwrap_or("action")
        );
        return Ok(());
    }
    Err(result["error"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| "action failed".to_string()))
}

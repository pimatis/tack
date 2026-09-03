use crate::db::*;
use rusqlite::{params, Connection};
use serde_json::json;

const DEFAULT_PORT: u16 = 17890;

fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = ?2",
        params![key, value],
    )
    .map_err(|e| format!("Failed to set setting: {}", e))?;
    Ok(())
}

// same local network address trick as the app, so phones on the wifi can join
fn url_for(port: u16) -> String {
    lan_ip()
        .map(|ip| format!("http://{}:{}", ip, port))
        .unwrap_or_else(|| format!("http://127.0.0.1:{}", port))
}

// best-effort local network address; udp connect never sends packets
fn lan_ip() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback()).then_some(ip.to_string())
}

fn current_port(conn: &Connection) -> u16 {
    get_setting(conn, "livePort")
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

pub fn on(conn: &Connection, json: bool, port: Option<u16>) -> Result<()> {
    let port = port.unwrap_or_else(|| current_port(conn));
    if !(1024..=65535).contains(&port) {
        return Err("port must be between 1024 and 65535".to_string());
    }
    set_setting(conn, "liveEnabled", "true")?;
    set_setting(conn, "livePort", &port.to_string())?;
    let url = url_for(port);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "live_on",
                "enabled": true,
                "port": port,
                "url": url
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Live server enabled on port {} ({})", port, url);
        println!("The server starts when the tack app is running");
    }
    Ok(())
}

pub fn off(conn: &Connection, json: bool) -> Result<()> {
    set_setting(conn, "liveEnabled", "false")?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "success": true,
                "action": "live_off",
                "enabled": false
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!("Live server disabled");
    }
    Ok(())
}

pub fn status(conn: &Connection, json: bool) -> Result<()> {
    let enabled = get_setting(conn, "liveEnabled").unwrap_or_else(|| "false".to_string()) == "true";
    let port = current_port(conn);
    let url = url_for(port);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "enabled": enabled,
                "port": port,
                "url": url
            }))
            .map_err(|e| e.to_string())?
        );
    } else {
        println!(
            "Live server: {}",
            if enabled { "enabled" } else { "disabled" }
        );
        if enabled {
            println!("Address: {}", url);
            println!("The server runs while the tack app is open");
        }
    }
    Ok(())
}

pub fn watch(conn: &Connection, json: bool) -> Result<()> {
    let enabled = get_setting(conn, "liveEnabled").unwrap_or_else(|| "false".to_string()) == "true";
    if !enabled {
        return Err("live server is disabled - run `tack live on` first".to_string());
    }
    let port = current_port(conn);
    // the app's server binds 0.0.0.0, so loopback always reaches it
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).map_err(|_| {
        format!("live server is not running on port {} - start the tack app", port)
    })?;
    use std::io::{BufRead, BufReader, Write};
    write!(
        stream,
        "GET /api/events/stream HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept: text/event-stream\r\nConnection: close\r\n\r\n"
    )
    .map_err(|e| format!("failed to open event stream: {}", e))?;
    stream.flush().map_err(|e| e.to_string())?;

    // parse the sse frames line by line; comment lines (: ping) are ignored
    let mut event = String::new();
    let mut data = String::new();
    for line in BufReader::new(stream).lines() {
        let line = line.map_err(|e| format!("event stream error: {}", e))?;
        if let Some(rest) = line.strip_prefix("event:") {
            event = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("data:") {
            data = rest.trim().to_string();
        } else if line.is_empty() && !data.is_empty() {
            // connected is the stream's hello event (current generation),
            // db-changed fires on every real write
            if event == "connected" || event == "db-changed" {
                print_event(json, &event, &data)?;
            }
            event.clear();
            data.clear();
        }
    }
    // the stream ended: the live server stopped (or the connection dropped)
    if json {
        println!(
            "{}",
            serde_json::to_string(&json!({ "stream": "closed" }))
                .map_err(|e| e.to_string())?
        );
    } else {
        println!("live stream closed (server stopped or connection lost)");
    }
    Ok(())
}

fn print_event(json: bool, kind: &str, data: &str) -> Result<()> {
    let payload: serde_json::Value =
        serde_json::from_str(data).map_err(|e| format!("bad event payload: {}", e))?;
    if json {
        let mut out = payload
            .as_object()
            .cloned()
            .unwrap_or_default();
        if kind == "connected" {
            out.insert("connected".to_string(), json!(true));
        }
        println!(
            "{}",
            serde_json::to_string(&serde_json::Value::Object(out)).map_err(|e| e.to_string())?
        );
    } else {
        let generation = payload["generation"].as_u64().unwrap_or(0);
        println!(
            "[{}] {} (generation {})",
            chrono::Local::now().format("%H:%M:%S"),
            kind,
            generation
        );
    }
    Ok(())
}

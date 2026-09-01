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

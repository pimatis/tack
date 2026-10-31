use crate::agent;
use crate::db::*;
use rusqlite::Connection;
use serde_json::json;

pub fn status(conn: &Connection, json: bool) -> Result<()> {
    let value = agent::request_json(conn, "GET", "/api/agent/ping", None)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?
        );
    } else {
        println!("Agent server is running on port {}", agent::port(conn));
    }
    Ok(())
}

pub fn snapshot(conn: &Connection, json: bool, text: bool) -> Result<()> {
    if text {
        let (_, bytes) = agent::request(conn, "GET", "/api/agent/snapshot?format=text", None)?;
        print!("{}", String::from_utf8_lossy(&bytes));
        return Ok(());
    }
    let value = agent::request_json(conn, "GET", "/api/agent/snapshot", None)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    println!(
        "{} ({})",
        value["url"].as_str().unwrap_or(""),
        value["title"].as_str().unwrap_or("")
    );
    if let Some(elements) = value["elements"].as_array() {
        for el in elements {
            let mut flags = String::new();
            if el["danger"].as_bool().unwrap_or(false) {
                flags.push_str(" [danger]");
            }
            if el["disabled"].as_bool().unwrap_or(false) {
                flags.push_str(" [disabled]");
            }
            if el["offscreen"].as_bool().unwrap_or(false) {
                flags.push_str(" [offscreen]");
            }
            println!(
                "[{}] {} \"{}\" @ {},{}{}",
                el["ref"].as_u64().unwrap_or(0),
                el["role"].as_str().unwrap_or(""),
                el["label"].as_str().unwrap_or(""),
                el["x"].as_i64().unwrap_or(0),
                el["y"].as_i64().unwrap_or(0),
                flags
            );
        }
    }
    Ok(())
}

pub fn screenshot(conn: &Connection, out: Option<String>) -> Result<()> {
    let (status, bytes) = agent::request(conn, "GET", "/api/agent/screenshot", None)?;
    if !(200..300).contains(&status) {
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap_or_else(|_| json!({"error": "screenshot failed"}));
        return Err(value["error"]
            .as_str()
            .unwrap_or("screenshot failed")
            .to_string());
    }
    let path = out.unwrap_or_else(|| {
        format!(
            "tack-screenshot-{}.png",
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        )
    });
    std::fs::write(&path, &bytes).map_err(|e| format!("could not write {}: {}", path, e))?;
    println!("{}", path);
    Ok(())
}

fn target(reference: Option<u64>, x: Option<f64>, y: Option<f64>) -> Result<serde_json::Value> {
    if let Some(r) = reference {
        return Ok(json!({ "ref": r }));
    }
    if let (Some(x), Some(y)) = (x, y) {
        return Ok(json!({ "x": x, "y": y }));
    }
    Err("target needs --ref or --x and --y".to_string())
}

pub fn click(
    conn: &Connection,
    json: bool,
    reference: Option<u64>,
    x: Option<f64>,
    y: Option<f64>,
    double: bool,
    right: bool,
) -> Result<()> {
    let params = target(reference, x, y)?;
    let name = if double {
        "dblclick"
    } else if right {
        "rightclick"
    } else {
        "click"
    };
    agent::print_result(agent::action(conn, name, params)?, json)
}

pub fn hover(
    conn: &Connection,
    json: bool,
    reference: Option<u64>,
    x: Option<f64>,
    y: Option<f64>,
) -> Result<()> {
    agent::print_result(agent::action(conn, "hover", target(reference, x, y)?)?, json)
}

pub fn type_text(
    conn: &Connection,
    json: bool,
    reference: Option<u64>,
    x: Option<f64>,
    y: Option<f64>,
    text: String,
) -> Result<()> {
    let mut params = target(reference, x, y)?;
    params["text"] = json!(text);
    agent::print_result(agent::action(conn, "type", params)?, json)
}

pub fn key(conn: &Connection, json: bool, keys: String) -> Result<()> {
    agent::print_result(agent::action(conn, "key", json!({ "keys": keys }))?, json)
}

pub fn scroll(
    conn: &Connection,
    json: bool,
    reference: Option<u64>,
    x: Option<f64>,
    y: Option<f64>,
    dx: f64,
    dy: f64,
) -> Result<()> {
    let mut params = if reference.is_some() || (x.is_some() && y.is_some()) {
        target(reference, x, y)?
    } else {
        json!({})
    };
    params["dx"] = json!(dx);
    params["dy"] = json!(dy);
    agent::print_result(agent::action(conn, "scroll", params)?, json)
}

fn parse_point(s: &str) -> Result<(f64, f64)> {
    let (x, y) = s
        .split_once(',')
        .ok_or_else(|| format!("expected \"x,y\", got \"{}\"", s))?;
    let x: f64 = x.trim().parse().map_err(|_| "bad x".to_string())?;
    let y: f64 = y.trim().parse().map_err(|_| "bad y".to_string())?;
    Ok((x, y))
}

pub fn drag(
    conn: &Connection,
    json: bool,
    from: Option<String>,
    to: Option<String>,
    from_ref: Option<u64>,
    to_ref: Option<u64>,
) -> Result<()> {
    let mut params = json!({});
    if let Some(r) = from_ref {
        params["fromRef"] = json!(r);
    } else if let Some(s) = &from {
        let (fx, fy) = parse_point(s)?;
        params["fromX"] = json!(fx);
        params["fromY"] = json!(fy);
    }
    if let Some(r) = to_ref {
        params["toRef"] = json!(r);
    } else if let Some(s) = &to {
        let (tx, ty) = parse_point(s)?;
        params["toX"] = json!(tx);
        params["toY"] = json!(ty);
    }
    if params.as_object().map(|o| o.len()).unwrap_or(0) == 0 {
        return Err("drag needs --from/--to \"x,y\" or --from-ref/--to-ref".to_string());
    }
    agent::print_result(agent::action(conn, "drag", params)?, json)
}

pub fn goto(conn: &Connection, json: bool, path: String) -> Result<()> {
    agent::print_result(agent::action(conn, "goto", json!({ "path": path }))?, json)
}

pub fn wait(conn: &Connection, json: bool, ms: u64) -> Result<()> {
    agent::print_result(agent::action(conn, "wait", json!({ "ms": ms }))?, json)
}

pub fn eval(conn: &Connection, json: bool, code: String) -> Result<()> {
    agent::print_result(agent::action(conn, "eval", json!({ "code": code }))?, json)
}

pub fn confirm(conn: &Connection, json: bool, title: String, body: Option<String>) -> Result<()> {
    let payload = json!({ "title": title, "body": body.unwrap_or_default() });
    let body_str = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let value = agent::request_json(conn, "POST", "/api/agent/confirm", Some(&body_str))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?
        );
    } else {
        println!(
            "{}",
            if value["approved"].as_bool().unwrap_or(false) {
                "approved"
            } else {
                "denied"
            }
        );
    }
    Ok(())
}

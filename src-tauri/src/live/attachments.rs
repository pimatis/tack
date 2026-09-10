use super::http::json_response;
use super::Ctx;
use serde_json::json;
use std::io::Read;
use tiny_http::{Header, Request, Response, StatusCode};

fn attachment_id(path: &str) -> Option<&str> {
    let id = path.strip_prefix("/api/attachment/")?;
    let valid = !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
    valid.then_some(id)
}

pub(super) fn serve_attachment(
    path: &str,
    query: &str,
    ctx: &Ctx,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(id) = attachment_id(path) else {
        return json_response(StatusCode(400), json!({ "error": "Invalid attachment id" }));
    };
    let mime = query
        .split("mime=")
        .nth(1)
        .map(|v| v.split('&').next().unwrap_or("").to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());
    match std::fs::read(ctx.attachments.join(id)) {
        Ok(data) => {
            let headers = vec![
                Header::from_bytes(b"Content-Type", mime.as_bytes()).unwrap(),
                Header::from_bytes(b"Cache-Control", b"no-cache").unwrap(),
                Header::from_bytes(b"Connection", b"close").unwrap(),
            ];
            let len = data.len();
            Response::new(
                StatusCode(200),
                headers,
                std::io::Cursor::new(data),
                Some(len),
                None,
            )
        }
        Err(_) => json_response(StatusCode(404), json!({ "error": "Attachment not found" })),
    }
}

pub(super) fn put_attachment(
    request: &mut Request,
    path: &str,
    ctx: &Ctx,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(id) = attachment_id(path) else {
        return json_response(StatusCode(400), json!({ "error": "Invalid attachment id" }));
    };
    let mut data = Vec::new();
    if let Err(e) = request
        .as_reader()
        .take(16 * 1024 * 1024)
        .read_to_end(&mut data)
    {
        return json_response(
            StatusCode(400),
            json!({ "error": format!("Failed to read upload: {}", e) }),
        );
    }
    match std::fs::write(ctx.attachments.join(id), &data) {
        Ok(()) => json_response(StatusCode(200), json!({ "ok": true })),
        Err(e) => json_response(
            StatusCode(500),
            json!({ "error": format!("Failed to save attachment: {}", e) }),
        ),
    }
}

pub(super) fn delete_attachment(path: &str, ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(id) = attachment_id(path) else {
        return json_response(StatusCode(400), json!({ "error": "Invalid attachment id" }));
    };
    match std::fs::remove_file(ctx.attachments.join(id)) {
        Ok(()) => json_response(StatusCode(200), json!({ "ok": true })),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            json_response(StatusCode(200), json!({ "ok": true }))
        }
        Err(e) => json_response(
            StatusCode(500),
            json!({ "error": format!("Failed to delete attachment: {}", e) }),
        ),
    }
}

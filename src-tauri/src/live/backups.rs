use super::http::{json_response, percent_decode};
use super::Ctx;
use crate::backup;
use serde::Deserialize;
use serde_json::json;
use std::io::Read;
use tiny_http::{Request, Response, StatusCode};

// "/api/backups/backup-20240101-000000.000[/restore]" -> "backup-20240101-000000.000"
fn backup_name_from_path(path: &str, with_restore_suffix: bool) -> Option<String> {
    let rest = path.strip_prefix("/api/backups/")?;
    let name = if with_restore_suffix {
        rest.strip_suffix("/restore")?
    } else {
        rest
    };
    (!name.is_empty()).then(|| percent_decode(name))
}

pub(super) fn serve_backups(ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    match backup::list_backups(&ctx.db_path) {
        Ok(list) => json_response(StatusCode(200), json!({ "backups": list })),
        Err(e) => json_response(StatusCode(500), json!({ "error": e })),
    }
}

#[derive(Deserialize)]
struct CreateBackupPayload {
    #[serde(default = "default_keep")]
    keep: usize,
}

fn default_keep() -> usize {
    7
}

pub(super) fn create_backup_http(request: &mut Request, ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    let mut body = String::new();
    let _ = request.as_reader().take(1024).read_to_string(&mut body);
    let keep = serde_json::from_str::<CreateBackupPayload>(&body)
        .map(|p| p.keep)
        .unwrap_or_else(|_| default_keep());
    match backup::create_backup(&ctx.db_path, keep) {
        Ok(name) => json_response(StatusCode(200), json!({ "name": name })),
        Err(e) => json_response(StatusCode(500), json!({ "error": e })),
    }
}

pub(super) fn restore_backup_http(path: &str, ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(name) = backup_name_from_path(path, true) else {
        return json_response(StatusCode(400), json!({ "error": "Invalid backup name" }));
    };
    match backup::restore_backup(&ctx.db_path, &name) {
        Ok(()) => json_response(StatusCode(200), json!({ "ok": true })),
        Err(e) => json_response(StatusCode(500), json!({ "error": e })),
    }
}

pub(super) fn delete_backup_http(path: &str, ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(name) = backup_name_from_path(path, false) else {
        return json_response(StatusCode(400), json!({ "error": "Invalid backup name" }));
    };
    match backup::delete_backup(&ctx.db_path, &name) {
        Ok(()) => json_response(StatusCode(200), json!({ "ok": true })),
        Err(e) => json_response(StatusCode(500), json!({ "error": e })),
    }
}

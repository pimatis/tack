//! the built-in asset protocol matches its scope with `require_literal_leading_dot`,
//! so it refuses dotfile paths on purpose - and every pasted screenshot lives under
//! `.tack/assets`. instead of widening that protocol, serve the files through our
//! own `tackasset://localhost/<path>` scheme, validated against the notes root.

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Mutex;

use tauri::http::{Request, Response, StatusCode};

use crate::live::http::{mime_for, percent_decode};

// the notes folder the gui is currently using; set when the folder is watched
static NOTES_ROOT: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn set_root(path: &str) {
    if let Ok(mut root) = NOTES_ROOT.lock() {
        *root = Some(PathBuf::from(path));
    }
}

// only files inside the notes root are served; both sides are canonicalized so
// symlinks and .. cannot escape
fn resolve_asset(root: &std::path::Path, requested: &str) -> Option<PathBuf> {
    let target = std::path::Path::new(requested);
    if !target.is_absolute() {
        return None;
    }
    let root = root.canonicalize().ok()?;
    let target = target.canonicalize().ok()?;
    (target.starts_with(&root) && target.is_file()).then_some(target)
}

fn empty(status: StatusCode) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(status)
        .header("Access-Control-Allow-Origin", "*")
        .body(Cow::Owned(Vec::new()))
        .expect("static response")
}

pub fn respond(request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    // convertFileSrc encodes the path, so the uri path is "/%2FUsers%2F...":
    // decoding leaves a leading "//" which we collapse to one separator
    let decoded = percent_decode(request.uri().path());
    let requested = format!("/{}", decoded.trim_start_matches('/'));
    let root = match NOTES_ROOT.lock() {
        Ok(root) => root.clone(),
        Err(_) => None,
    };
    let Some(root) = root else {
        return empty(StatusCode::FORBIDDEN);
    };
    let Some(path) = resolve_asset(&root, &requested) else {
        return empty(StatusCode::NOT_FOUND);
    };
    match std::fs::read(&path) {
        Ok(data) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", mime_for(&path))
            .header("Access-Control-Allow-Origin", "*")
            .body(Cow::Owned(data))
            .unwrap_or_else(|_| empty(StatusCode::INTERNAL_SERVER_ERROR)),
        Err(_) => empty(StatusCode::NOT_FOUND),
    }
}

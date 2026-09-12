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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_asset_stays_inside_the_notes_root() {
        let base = std::env::temp_dir().join(format!(
            "tack-asset-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let assets = base.join(".tack").join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        let image = assets.join("shot.png");
        std::fs::write(&image, [1u8, 2, 3]).unwrap();
        let base = base.canonicalize().unwrap();
        let image = image.canonicalize().unwrap();

        assert_eq!(
            resolve_asset(&base, &image.to_string_lossy()),
            Some(image.clone())
        );
        // a dotfile dir is still served (the built-in asset scope refuses these)
        assert!(image.to_string_lossy().contains("/.tack/"));
        // outside the root and relative paths are rejected
        assert_eq!(resolve_asset(&base, "/etc/hosts"), None);
        assert_eq!(resolve_asset(&base, "relative.png"), None);
        assert_eq!(
            resolve_asset(&base, &base.join("missing.png").to_string_lossy()),
            None
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    // the scheme url convertFileSrc builds: tackasset://localhost/<encoded path>
    #[test]
    fn respond_reads_encoded_asset_urls() {
        let base = std::env::temp_dir().join(format!(
            "tack-asset-url-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let assets = base.join(".tack").join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        let image = assets.join("shot.png");
        std::fs::write(&image, [1u8, 2, 3]).unwrap();
        let base = base.canonicalize().unwrap();
        let image = image.canonicalize().unwrap();
        set_root(&base.to_string_lossy());

        let encoded = image.to_string_lossy().replace('/', "%2F");
        let request = Request::builder()
            .uri(format!("tackasset://localhost/{}", encoded))
            .body(Vec::new())
            .unwrap();
        let response = respond(request);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.body().as_ref(), &[1u8, 2, 3]);

        // a path outside the notes root is refused
        let outside = Request::builder()
            .uri("tackasset://localhost/%2Fetc%2Fhosts")
            .body(Vec::new())
            .unwrap();
        assert_eq!(respond(outside).status(), StatusCode::NOT_FOUND);

        let _ = std::fs::remove_dir_all(&base);
    }
}

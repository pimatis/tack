use super::http::json_response;
use super::Ctx;
use crate::attachments::base64_encode;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use std::time::Duration;
use tiny_http::{Header, Request, Response, StatusCode};

// optional shared-password protection for the live server. hash and salt
// live in the settings table (written by the settings ui through the
// hash_live_password command); plaintext is never stored
pub(super) struct LiveAuth {
    salt: Vec<u8>,
    hash: [u8; 32],
    pub(super) token: String,
}

// salted kdf for the shared access password. no heavy work factor on
// purpose: whoever can read this hash already owns the db (and the tasks),
// so offline cracking wins nothing; online guessing is throttled by the
// delay in handle_auth. heavy pbkdf2 (310k+) is also unusably slow in
// debug builds (~4.5s vs ~5ms)
pub(super) const PBKDF2_ITERATIONS: u32 = 10_000;

#[derive(Serialize)]
pub struct PasswordHash {
    pub hash: String,
    pub salt: String,
}

pub(super) fn hash_live_password(password: String) -> super::Result<PasswordHash> {
    let bytes = password.as_bytes();
    if bytes.len() < 8 || bytes.len() > 128 {
        return Err("password must be 8-128 characters".to_string());
    }
    let salt = uuid::Uuid::new_v4();
    let hash = pbkdf2_sha256(bytes, salt.as_bytes(), PBKDF2_ITERATIONS);
    Ok(PasswordHash {
        hash: base64_encode(&hash),
        salt: base64_encode(salt.as_bytes()),
    })
}

// pbkdf2-hmac-sha256 (rfc 2898), single-block 32-byte output
pub(super) fn pbkdf2_sha256(password: &[u8], salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut key = [0u8; 64];
    if password.len() > 64 {
        key[..32].copy_from_slice(&Sha256::digest(password));
    } else {
        key[..password.len()].copy_from_slice(password);
    }
    let ipad: [u8; 64] = key.map(|b| b ^ 0x36);
    let opad: [u8; 64] = key.map(|b| b ^ 0x5c);

    let hmac = |msg: &[u8]| -> [u8; 32] {
        let inner = Sha256::new().chain_update(ipad).chain_update(msg).finalize();
        Sha256::new()
            .chain_update(opad)
            .chain_update(inner)
            .finalize()
            .into()
    };

    let mut block = Vec::with_capacity(salt.len() + 4);
    block.extend_from_slice(salt);
    block.extend_from_slice(&1u32.to_be_bytes());
    let mut u = hmac(&block);
    let mut out = u;
    for _ in 1..iterations {
        u = hmac(&u);
        for (o, x) in out.iter_mut().zip(u.iter()) {
            *o ^= x;
        }
    }
    out
}

// reads the shared-password config from the settings table on each request,
// so password changes apply immediately without a server restart.
// anything unexpected (db error, corrupt values) fails closed with an
// unmatchable token instead of silently opening the share
pub(super) fn load_auth(db_path: &Path) -> Option<LiveAuth> {
    let closed = || {
        Some(LiveAuth {
            salt: Vec::new(),
            hash: [0u8; 32],
            token: uuid::Uuid::new_v4().to_string(),
        })
    };
    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return closed(),
    };
    let read = |key: &str| -> std::result::Result<Option<String>, ()> {
        match conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(_) => Err(()),
        }
    };
    let hash_b64 = match read("livePasswordHash") {
        Ok(Some(v)) => v,
        // no shared secret configured
        Ok(None) => return None,
        Err(_) => return closed(),
    };
    let salt_b64 = match read("livePasswordSalt") {
        Ok(Some(v)) => v,
        Ok(None) => return closed(),
        Err(_) => return closed(),
    };
    if hash_b64.is_empty() || salt_b64.is_empty() {
        // password cleared
        return None;
    }
    let (Some(hash), Some(salt)) = (base64_decode(&hash_b64), base64_decode(&salt_b64)) else {
        return closed();
    };
    if hash.len() != 32 || salt.is_empty() {
        return closed();
    }
    let mut hash_arr = [0u8; 32];
    hash_arr.copy_from_slice(&hash);
    // session token derived from the stored hash: stable across requests
    // without server state, and rotates whenever the password changes
    let token = base64_encode(
        &Sha256::new()
            .chain_update(b"tack-live-token")
            .chain_update(hash_arr)
            .chain_update(&salt)
            .finalize(),
    );
    Some(LiveAuth { salt, hash: hash_arr, token })
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.decode(input).ok()
}

// constant-time comparison so auth timing leaks nothing
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

// session token from the tack_live cookie, an Authorization bearer header,
// or a ?token= query param (agents streaming events with curl)
pub(super) fn authorized(request: &Request, query: &str, auth: &LiveAuth) -> bool {
    let token = auth.token.as_bytes();
    for header in request.headers() {
        let name = header.field.as_str().as_str();
        let value = header.value.as_str();
        if name.eq_ignore_ascii_case("cookie") {
            for part in value.split(';') {
                if let Some(v) = part.trim().strip_prefix("tack_live=") {
                    if ct_eq(v.as_bytes(), token) {
                        return true;
                    }
                }
            }
        }
        if name.eq_ignore_ascii_case("authorization") {
            if let Some(v) = value.strip_prefix("Bearer ") {
                if ct_eq(v.trim().as_bytes(), token) {
                    return true;
                }
            }
        }
    }
    if let Some(v) = query.split("token=").nth(1).and_then(|v| v.split('&').next()) {
        if !v.is_empty() && ct_eq(v.as_bytes(), token) {
            return true;
        }
    }
    false
}

#[derive(Deserialize)]
struct AuthPayload {
    password: String,
}

// verifies the shared password and issues the session cookie. wrong
// attempts pay an extra delay on top of pbkdf2 to blunt brute force
pub(super) fn handle_auth(request: &mut Request, ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(auth) = load_auth(&ctx.db_path) else {
        return json_response(StatusCode(400), json!({ "error": "No password is set" }));
    };
    let mut body = String::new();
    let _ = request.as_reader().take(1024).read_to_string(&mut body);
    let payload: Option<AuthPayload> = serde_json::from_str(&body).ok();
    let ok = payload
        .as_ref()
        .map(|p| {
            ct_eq(
                &pbkdf2_sha256(p.password.as_bytes(), &auth.salt, PBKDF2_ITERATIONS),
                &auth.hash,
            )
        })
        .unwrap_or(false);
    if !ok {
        std::thread::sleep(Duration::from_millis(500));
        return json_response(StatusCode(401), json!({ "error": "Wrong password" }));
    }
    let cookie = format!(
        "tack_live={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800",
        auth.token
    );
    let data = b"{\"ok\":true}".to_vec();
    let headers = vec![
        Header::from_bytes(b"Content-Type", b"application/json; charset=utf-8").unwrap(),
        Header::from_bytes(b"Set-Cookie", cookie.as_bytes()).unwrap(),
        Header::from_bytes(b"Cache-Control", b"no-cache").unwrap(),
        Header::from_bytes(b"Connection", b"close").unwrap(),
    ];
    let len = data.len();
    Response::new(StatusCode(200), headers, std::io::Cursor::new(data), Some(len), None)
}

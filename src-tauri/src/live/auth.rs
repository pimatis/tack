use super::http::json_response;
use super::Ctx;
use crate::attachments::base64_encode;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tiny_http::{Header, Request, Response, StatusCode};

// one configured shared password and the session token derived from it
pub(super) struct Credential {
    salt: Vec<u8>,
    hash: [u8; 32],
    token: String,
}

// optional shared-password protection for the live server. hash and salt
// live in the settings table (written by the settings ui through the
// hash_live_password command); plaintext is never stored. two passwords may
// be configured: the full-access one and a read-only one for view-only
// visitors
pub(super) struct LiveAuth {
    full: Option<Credential>,
    read_only: Option<Credential>,
}

// what a presented session token unlocks
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Access {
    Full,
    ReadOnly,
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
        let inner = Sha256::new()
            .chain_update(ipad)
            .chain_update(msg)
            .finalize();
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
    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => {
            return Some(LiveAuth {
                full: Some(closed_credential()),
                read_only: None,
            })
        }
    };
    let full = load_credential(&conn, "livePasswordHash", "livePasswordSalt", b"tack-live-token");
    let read_only = load_credential(
        &conn,
        "liveReadPasswordHash",
        "liveReadPasswordSalt",
        b"tack-live-token-ro",
    );
    if full.is_none() && read_only.is_none() {
        // no shared secret configured
        return None;
    }
    Some(LiveAuth { full, read_only })
}

// a credential nobody can log in as: a random token that no client can
// present, plus a hash no password matches
fn closed_credential() -> Credential {
    Credential {
        salt: Vec::new(),
        hash: [0u8; 32],
        token: uuid::Uuid::new_v4().to_string(),
    }
}

// one configured password pair -> one credential. missing or cleared values
// mean the credential simply does not exist; corrupt rows fail closed
fn load_credential(
    conn: &Connection,
    hash_key: &str,
    salt_key: &str,
    domain: &[u8],
) -> Option<Credential> {
    let read = |key: &str| -> std::result::Result<Option<String>, ()> {
        match conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
            r.get(0)
        }) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(_) => Err(()),
        }
    };
    let hash_b64 = match read(hash_key) {
        Ok(Some(v)) => v,
        Ok(None) => return None,
        Err(_) => return Some(closed_credential()),
    };
    if hash_b64.is_empty() {
        // password cleared
        return None;
    }
    let salt_b64 = match read(salt_key) {
        Ok(Some(v)) => v,
        Ok(None) => return Some(closed_credential()),
        Err(_) => return Some(closed_credential()),
    };
    let (Some(hash), Some(salt)) = (base64_decode(&hash_b64), base64_decode(&salt_b64)) else {
        return Some(closed_credential());
    };
    if hash.len() != 32 || salt.is_empty() {
        return Some(closed_credential());
    }
    let mut hash_arr = [0u8; 32];
    hash_arr.copy_from_slice(&hash);
    // session token derived from the stored hash: stable across requests
    // without server state, and rotates whenever the password changes. the
    // domain differs per role so identical passwords still mint distinct
    // tokens
    let token = base64_encode(
        &Sha256::new()
            .chain_update(domain)
            .chain_update(hash_arr)
            .chain_update(&salt)
            .finalize(),
    );
    Some(Credential {
        salt,
        hash: hash_arr,
        token,
    })
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
// or a ?token= query param (agents streaming events with curl). the full
// token wins over the read-only one
pub(super) fn authorized(request: &Request, query: &str, auth: &LiveAuth) -> Option<Access> {
    if let Some(c) = &auth.full {
        if token_matches(request, query, &c.token) {
            return Some(Access::Full);
        }
    }
    if let Some(c) = &auth.read_only {
        if token_matches(request, query, &c.token) {
            return Some(Access::ReadOnly);
        }
    }
    None
}

fn token_matches(request: &Request, query: &str, token: &str) -> bool {
    let token = token.as_bytes();
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
    if let Some(v) = query
        .split("token=")
        .nth(1)
        .and_then(|v| v.split('&').next())
    {
        if !v.is_empty() && ct_eq(v.as_bytes(), token) {
            return true;
        }
    }
    false
}

// login throttling per client ip: AUTH_MAX_FAILURES wrong passwords inside
// AUTH_WINDOW lock the source out for AUTH_LOCKOUT. the map lives in memory
// only, so restarting the live server clears every lockout
const AUTH_MAX_FAILURES: u32 = 5;
const AUTH_WINDOW: Duration = Duration::from_secs(300);
const AUTH_LOCKOUT: Duration = Duration::from_secs(300);

struct Attempt {
    failures: u32,
    window_start: Instant,
    locked_until: Option<Instant>,
}

pub(super) struct AuthLimiter {
    attempts: Mutex<HashMap<IpAddr, Attempt>>,
}

impl AuthLimiter {
    pub(super) fn new() -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
        }
    }

    // remaining lockout for this ip, if any
    pub(super) fn locked(&self, ip: IpAddr) -> Option<Duration> {
        let mut map = self.attempts.lock().unwrap();
        let until = map.get(&ip)?.locked_until?;
        let now = Instant::now();
        if now < until {
            return Some(until - now);
        }
        // lockout expired: start this client fresh
        map.remove(&ip);
        None
    }

    pub(super) fn failure(&self, ip: IpAddr) {
        let now = Instant::now();
        let mut map = self.attempts.lock().unwrap();
        // drop stale entries here so a flood of distinct source ips cannot
        // grow the map forever
        map.retain(|_, a| {
            a.locked_until.is_some_and(|u| u > now)
                || now.duration_since(a.window_start) < AUTH_WINDOW
        });
        let attempt = map.entry(ip).or_insert(Attempt {
            failures: 0,
            window_start: now,
            locked_until: None,
        });
        attempt.failures += 1;
        if attempt.failures >= AUTH_MAX_FAILURES {
            attempt.locked_until = Some(now + AUTH_LOCKOUT);
        }
    }

    pub(super) fn success(&self, ip: IpAddr) {
        self.attempts.lock().unwrap().remove(&ip);
    }
}

#[derive(Deserialize)]
struct AuthPayload {
    password: String,
}

// verifies the shared password and issues the session cookie. wrong
// attempts pay an extra delay on top of pbkdf2, and repeated failures lock
// the source ip out entirely
pub(super) fn handle_auth(request: &mut Request, ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    let ip = request
        .remote_addr()
        .map(|a| a.ip())
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    if let Some(remaining) = ctx.auth_limiter.locked(ip) {
        return json_response(
            StatusCode(429),
            json!({ "error": "Too many attempts", "retry_after": remaining.as_secs() }),
        );
    }
    let Some(auth) = load_auth(&ctx.db_path) else {
        return json_response(StatusCode(400), json!({ "error": "No password is set" }));
    };
    let mut body = String::new();
    let _ = request.as_reader().take(1024).read_to_string(&mut body);
    let payload: Option<AuthPayload> = serde_json::from_str(&body).ok();
    let password = payload
        .as_ref()
        .map(|p| p.password.as_bytes())
        .unwrap_or_default();
    // the full-access password is checked first: the same string set for
    // both roles must not silently downgrade the owner to read-only
    let matched = auth
        .full
        .as_ref()
        .filter(|c| ct_eq(&pbkdf2_sha256(password, &c.salt, PBKDF2_ITERATIONS), &c.hash))
        .map(|c| (Access::Full, c.token.clone()))
        .or_else(|| {
            auth.read_only
                .as_ref()
                .filter(|c| {
                    ct_eq(
                        &pbkdf2_sha256(password, &c.salt, PBKDF2_ITERATIONS),
                        &c.hash,
                    )
                })
                .map(|c| (Access::ReadOnly, c.token.clone()))
        });
    let Some((access, token)) = matched else {
        ctx.auth_limiter.failure(ip);
        std::thread::sleep(Duration::from_millis(500));
        return json_response(StatusCode(401), json!({ "error": "Wrong password" }));
    };
    ctx.auth_limiter.success(ip);
    let cookie = format!(
        "tack_live={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800",
        token
    );
    let scope = match access {
        Access::Full => "full",
        Access::ReadOnly => "read",
    };
    let data = format!("{{\"ok\":true,\"scope\":\"{}\"}}", scope).into_bytes();
    let headers = vec![
        Header::from_bytes(b"Content-Type", b"application/json; charset=utf-8").unwrap(),
        Header::from_bytes(b"Set-Cookie", cookie.as_bytes()).unwrap(),
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


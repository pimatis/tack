use super::http::handle_request;
use super::{Ctx, LiveHub, Result};
use crate::attachments::attachments_dir;
use crate::db::app_db_path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Manager;
use tiny_http::Server;

#[derive(Clone, serde::Serialize)]
pub struct LiveStatus {
    pub port: u16,
    pub url: String,
}

pub struct LiveServer {
    pub port: u16,
    pub(super) server: Arc<Server>,
    // accept loop handle, joined on drop so the listener socket is
    // guaranteed closed (port freed) once the server is dropped
    pub(super) accept: Option<std::thread::JoinHandle<()>>,
    // close flags for in-flight sse streams; set on drop so live_stop ends
    // them instead of leaving them attached to the app-lifetime hub
    pub(super) sse: Arc<Mutex<Vec<Arc<AtomicBool>>>>,
}

impl Drop for LiveServer {
    fn drop(&mut self) {
        // unblock the accept loop and wait for it to exit; the last Arc
        // reference then drops the Server, which closes the listener
        self.server.unblock();
        if let Some(handle) = self.accept.take() {
            let _ = handle.join();
        }
        for flag in self.sse.lock().unwrap().iter() {
            flag.store(true, Ordering::Relaxed);
        }
    }
}

pub struct LiveState {
    pub server: Mutex<Option<LiveServer>>,
    pub hub: Arc<LiveHub>,
}

// tiny_http binds with a plain TcpListener::bind, which leaves the port
// unbindable for ~2*MSL when the app exits with browser connections open
// (TIME_WAIT sockets on the live port). bind with SO_REUSEADDR so a quick
// reopen can rebind the same port at once
fn bind_listener(port: u16) -> std::io::Result<std::net::TcpListener> {
    let socket = socket2::Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::STREAM,
        Some(socket2::Protocol::TCP),
    )?;
    socket.set_reuse_address(true)?;
    socket.bind(&std::net::SocketAddr::from(([0, 0, 0, 0], port)).into())?;
    socket.listen(128)?;
    Ok(socket.into())
}

fn url(port: u16) -> String {
    // prefer the local network address so phones on the same wifi can join
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

pub(super) fn start(app: &tauri::AppHandle, port: u16) -> Result<LiveStatus> {
    let state = app.state::<LiveState>();
    let mut guard = state.server.lock().unwrap();
    if let Some(existing) = guard.as_ref() {
        if existing.port == port {
            return Ok(LiveStatus {
                port,
                url: url(port),
            });
        }
        // port changed while running: fully shut the old server down first,
        // so its listener is closed before we try to bind again
        drop(guard.take());
    }

    let sse = Arc::new(Mutex::new(Vec::new()));
    let ctx = Ctx {
        db_path: app_db_path(app)?,
        attachments: attachments_dir(app)?,
        frontend: frontend_dir(app).ok_or_else(|| {
            "Frontend build not found - run `bun run build` and try again".to_string()
        })?,
        hub: state.hub.clone(),
        events_timeout: Duration::from_secs(20),
        sse: sse.clone(),
    };

    let server = Arc::new(
        // 0.0.0.0 so phones and other devices on the local network can connect
        Server::from_listener(
            bind_listener(port)
                .map_err(|e| format!("Could not start live server on port {}: {}", port, e))?,
            None,
        )
        .map_err(|e| format!("Could not start live server on port {}: {}", port, e))?,
    );

    let accept = server.clone();
    let handle = std::thread::spawn(move || {
        for request in accept.incoming_requests() {
            let ctx = ctx.clone();
            std::thread::spawn(move || handle_request(request, &ctx));
        }
    });

    *guard = Some(LiveServer {
        port,
        server,
        accept: Some(handle),
        sse,
    });
    Ok(LiveStatus {
        port,
        url: url(port),
    })
}

pub(super) fn stop(app: &tauri::AppHandle) -> Result<()> {
    let state = app.state::<LiveState>();
    let mut guard = state.server.lock().unwrap();
    // LiveServer's Drop impl unblocks the accept loop and joins it, so the
    // listener socket is fully closed and the port freed before we return
    drop(guard.take());
    Ok(())
}

pub(super) fn status(app: &tauri::AppHandle) -> Option<LiveStatus> {
    let state = app.state::<LiveState>();
    let guard = state.server.lock().unwrap();
    guard.as_ref().map(|live| LiveStatus {
        port: live.port,
        url: url(live.port),
    })
}

fn frontend_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    // packaged app: build dir copied into resources
    if let Ok(resource) = app.path().resource_dir() {
        let bundled = resource.join("build");
        if bundled.join("index.html").exists() {
            return Some(bundled);
        }
    }
    // dev: repo build dir
    let local = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../build");
    (local.join("index.html").exists()).then_some(local)
}

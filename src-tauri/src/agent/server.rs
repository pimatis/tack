use super::http::handle_request;
use super::Result;
use socket2::Socket;
use std::sync::Arc;
use tauri::Manager;
use tiny_http::Server;

#[derive(Clone, serde::Serialize)]
pub struct AgentStatus {
    pub port: u16,
    pub url: String,
}

pub struct AgentServer {
    pub port: u16,
    server: Arc<Server>,
    // accept loop handle, joined on drop so the listener socket is
    // guaranteed closed (port freed) once the server is dropped
    accept: Option<std::thread::JoinHandle<()>>,
}

impl Drop for AgentServer {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(handle) = self.accept.take() {
            let _ = handle.join();
        }
    }
}

// loopback only: this endpoint can drive the whole app, so it must never be
// reachable from the network. SO_REUSEADDR keeps a quick restart bindable
fn bind_listener(port: u16) -> std::io::Result<std::net::TcpListener> {
    let socket = Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::STREAM,
        Some(socket2::Protocol::TCP),
    )?;
    socket.set_reuse_address(true)?;
    socket.bind(&std::net::SocketAddr::from(([127, 0, 0, 1], port)).into())?;
    socket.listen(64)?;
    Ok(socket.into())
}

pub(super) fn start(app: &tauri::AppHandle, port: u16) -> Result<AgentStatus> {
    let state = app.state::<super::AgentState>();
    let mut guard = state.server.lock().unwrap();
    if let Some(existing) = guard.as_ref() {
        if existing.port == port {
            return Ok(status_of(port));
        }
        drop(guard.take());
    }

    let app_handle = app.clone();
    let server = Arc::new(
        Server::from_listener(
            bind_listener(port)
                .map_err(|e| format!("Could not start agent server on port {}: {}", port, e))?,
            None,
        )
        .map_err(|e| format!("Could not start agent server on port {}: {}", port, e))?,
    );

    let accept = server.clone();
    let handle = std::thread::spawn(move || {
        for request in accept.incoming_requests() {
            let app = app_handle.clone();
            std::thread::spawn(move || handle_request(request, &app));
        }
    });

    *guard = Some(AgentServer {
        port,
        server,
        accept: Some(handle),
    });
    Ok(status_of(port))
}

pub(super) fn stop(app: &tauri::AppHandle) -> Result<()> {
    let state = app.state::<super::AgentState>();
    let mut guard = state.server.lock().unwrap();
    drop(guard.take());
    Ok(())
}

pub(super) fn status(app: &tauri::AppHandle) -> Option<AgentStatus> {
    let state = app.state::<super::AgentState>();
    let guard = state.server.lock().unwrap();
    guard.as_ref().map(|s| status_of(s.port))
}

fn status_of(port: u16) -> AgentStatus {
    AgentStatus {
        port,
        url: format!("http://127.0.0.1:{}", port),
    }
}

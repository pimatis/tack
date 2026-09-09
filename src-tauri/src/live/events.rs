use super::http::json_response;
use super::Ctx;
use serde_json::json;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tiny_http::{Response, StatusCode};

pub(super) fn poll_events(ctx: &Ctx) -> Response<std::io::Cursor<Vec<u8>>> {
    // long-poll: hold the request until a db change lands (or a quiet timeout),
    // then answer with a complete response the browser can immediately re-poll
    let last = ctx.hub.generation();
    let changed = ctx.hub.changed_since(last, ctx.events_timeout);
    json_response(StatusCode(200), json!({ "changed": changed }))
}

// sse stream for agents: keep the connection open and push a db-changed event
// whenever the hub generation advances. the hello event carries the current
// generation, so a client that sees a jump larger than one knows it missed
// events and should re-query. the heartbeat doubles as a dead-client check:
// the next write fails on a dropped connection and ends the stream
pub(super) fn stream_events(writer: &mut dyn Write, ctx: &Ctx, closed: &AtomicBool) {
    let _ = write!(
        writer,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n"
    );
    let mut last = ctx.hub.generation();
    let _ = write!(
        writer,
        "event: connected\r\ndata: {{\"generation\":{last}}}\r\n\r\n"
    );
    let _ = writer.flush();
    while !closed.load(Ordering::Relaxed) {
        let changed = ctx.hub.changed_since(last, Duration::from_secs(5));
        if changed {
            last = ctx.hub.generation();
            let _ = write!(
                writer,
                "event: db-changed\r\ndata: {{\"generation\":{last}}}\r\n\r\n"
            );
        } else {
            let _ = write!(writer, ": ping\r\n\r\n");
        }
        if writer.flush().is_err() {
            return;
        }
    }
}

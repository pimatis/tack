use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// change signaling for browser clients: a generation counter + condvar.
// the live server long-polls on it, so every http response completes
// (tiny_http only flushes its writer once a response body finishes); the
// sse stream waits on the same condvar but writes frames straight to the
// connection writer (request.into_writer), flushing after each one
#[derive(Default)]
pub struct LiveHub {
    state: Mutex<LiveHubState>,
    cv: std::sync::Condvar,
    // live clients keyed by client id; one entry per client no matter how many
    // sse connections it holds (the browser opens one per subscription)
    presence: Mutex<HashMap<String, PresenceEntry>>,
}

#[derive(Default)]
struct LiveHubState {
    generation: u64,
}

struct PresenceEntry {
    name: String,
    kind: String,
    joined_ms: u64,
    connections: u32,
}

// one connected live client, as sent to the ui
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenceClient {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub connections: u32,
    pub joined_ms: u64,
}

fn epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl LiveHub {
    pub fn notify(&self) {
        let mut state = self.state.lock().unwrap();
        state.generation = state.generation.wrapping_add(1);
        self.cv.notify_all();
    }

    pub fn generation(&self) -> u64 {
        self.state.lock().unwrap().generation
    }

    // true when a change happened since `last`; waits up to `timeout` for one
    pub fn changed_since(&self, last: u64, timeout: Duration) -> bool {
        let state = self.state.lock().unwrap();
        if state.generation != last {
            return true;
        }
        let (guard, _) = self.cv.wait_timeout(state, timeout).unwrap();
        guard.generation != last
    }

    // register one live connection; repeated calls for the same id increment
    // its connection count instead of adding a duplicate entry
    pub fn presence_join(&self, id: &str, name: &str, kind: &str) {
        let mut map = self.presence.lock().unwrap();
        match map.get_mut(id) {
            Some(entry) => {
                entry.connections += 1;
                if !name.is_empty() {
                    entry.name = name.to_string();
                }
                entry.kind = kind.to_string();
            }
            None => {
                map.insert(
                    id.to_string(),
                    PresenceEntry {
                        name: name.to_string(),
                        kind: kind.to_string(),
                        joined_ms: epoch_ms(),
                        connections: 1,
                    },
                );
            }
        }
    }

    // drop one connection; the client disappears once its last one closes
    pub fn presence_leave(&self, id: &str) {
        let mut map = self.presence.lock().unwrap();
        if let Some(entry) = map.get_mut(id) {
            entry.connections = entry.connections.saturating_sub(1);
            if entry.connections == 0 {
                map.remove(id);
            }
        }
    }

    // connected clients, oldest first
    pub fn presence_snapshot(&self) -> Vec<PresenceClient> {
        let map = self.presence.lock().unwrap();
        let mut out: Vec<PresenceClient> = map
            .iter()
            .map(|(id, entry)| PresenceClient {
                id: id.clone(),
                name: entry.name.clone(),
                kind: entry.kind.clone(),
                connections: entry.connections,
                joined_ms: entry.joined_ms,
            })
            .collect();
        out.sort_by_key(|client| client.joined_ms);
        out
    }
}

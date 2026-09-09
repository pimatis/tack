use std::sync::Mutex;
use std::time::Duration;

// change signaling for browser clients: a generation counter + condvar.
// the live server long-polls on it, so every http response completes
// (tiny_http only flushes its writer once a response body finishes); the
// sse stream waits on the same condvar but writes frames straight to the
// connection writer (request.into_writer), flushing after each one
#[derive(Default)]
pub struct LiveHub {
    state: Mutex<LiveHubState>,
    cv: std::sync::Condvar,
}

#[derive(Default)]
struct LiveHubState {
    generation: u64,
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
}

mod http;
mod screenshot;
mod server;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Mutex};
use std::time::Duration;
use tauri::Manager;

pub use server::AgentStatus;

const AGENT_JS: &str = include_str!("agent.js");
static EVAL_SEQ: AtomicU64 = AtomicU64::new(0);

// agent evals never wait longer than this; a navigated-away or wedged
// webview must fail the http request instead of holding it forever
const EVAL_TIMEOUT: Duration = Duration::from_secs(10);

pub type Result<T> = std::result::Result<T, String>;

pub struct AgentState {
    pub server: Mutex<Option<server::AgentServer>>,
    // in-flight eval requests: id -> result channel
    pub(crate) pending: Mutex<HashMap<u64, mpsc::Sender<EvalOutcome>>>,
}

pub(crate) struct EvalOutcome {
    ok: bool,
    value: String,
}

// evaluates an expression in the main webview and waits for the result. the
// expression posts back through the agent_result command (window.eval cannot
// return a value), keyed by a unique request id
fn eval(app: &tauri::AppHandle, expr: &str) -> Result<serde_json::Value> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is not available".to_string())?;
    let id = EVAL_SEQ.fetch_add(1, Ordering::Relaxed);
    let (tx, rx) = mpsc::channel::<EvalOutcome>();
    app.state::<AgentState>()
        .pending
        .lock()
        .unwrap()
        .insert(id, tx);
    let script = format!(
        "(async()=>{{try{{const v=await({expr});window.__TAURI_INTERNALS__.invoke('agent_result',{{id:{id},ok:true,value:JSON.stringify(v===undefined?null:v)}})}}catch(e){{window.__TAURI_INTERNALS__.invoke('agent_result',{{id:{id},ok:false,value:String(e&&e.stack||e&&e.message||e)}})}}}})()",
        expr = expr,
        id = id
    );
    if let Err(e) = window.eval(&script) {
        app.state::<AgentState>().pending.lock().unwrap().remove(&id);
        return Err(format!("could not evaluate in the webview: {}", e));
    }
    match rx.recv_timeout(EVAL_TIMEOUT) {
        Ok(outcome) => {
            let value = serde_json::from_str(&outcome.value)
                .unwrap_or(serde_json::Value::String(outcome.value));
            if outcome.ok {
                Ok(value)
            } else {
                Err(value
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| "eval failed".to_string()))
            }
        }
        Err(_) => {
            app.state::<AgentState>().pending.lock().unwrap().remove(&id);
            Err("eval timed out - the webview may be reloading".to_string())
        }
    }
}

// lazily injects the agent runtime, then calls __tackAgent.act(name, params)
fn act(app: &tauri::AppHandle, name: &str, params: &serde_json::Value) -> Result<serde_json::Value> {
    let source = serde_json::to_string(AGENT_JS).map_err(|e| e.to_string())?;
    let args = serde_json::to_string(&json_map(name, params)).map_err(|e| e.to_string())?;
    let expr = format!(
        "(window.__tackAgent||(window.__tackAgent=eval({source}))).act({args}.name,{args}.params)",
        source = source,
        args = args
    );
    eval(app, &expr)
}

fn json_map(name: &str, params: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "name": name, "params": params })
}

#[tauri::command]
pub fn agent_result(app: tauri::AppHandle, id: u64, ok: bool, value: String) {
    let sender = app
        .state::<AgentState>()
        .pending
        .lock()
        .unwrap()
        .remove(&id);
    if let Some(tx) = sender {
        let _ = tx.send(EvalOutcome { ok, value });
    }
}

#[tauri::command]
pub fn agent_start(app: tauri::AppHandle, port: u16) -> Result<AgentStatus> {
    server::start(&app, port)
}

#[tauri::command]
pub fn agent_stop(app: tauri::AppHandle) -> Result<()> {
    server::stop(&app)
}

#[tauri::command]
pub fn agent_status(app: tauri::AppHandle) -> Option<AgentStatus> {
    server::status(&app)
}

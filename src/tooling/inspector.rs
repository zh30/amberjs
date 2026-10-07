//! CDP subset for `amber run --inspect` and `--inspect-brk`.
//!
//! This is not `v8::inspector` and not Chrome DevTools. The `v8` 152.2.0
//! `IsolateHandle::request_interrupt` callback must not reenter the isolate, so
//! `Runtime.evaluate` runs on the isolate thread only when that thread is free:
//! the `--inspect-brk` pause loop, or the host event loop between tasks.
//! A synchronous JavaScript turn is not preempted.

use anyhow::{anyhow, Result};
use rusty_v8 as v8;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::io::{ErrorKind, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub const INSPECT_ERROR_PREFIX: &str = "error: amber run:";
pub const UNSUPPORTED_METHOD_MESSAGE: &str = "Amber inspector does not support this method";
const EVALUATE_TIMEOUT: Duration = Duration::from_secs(20);

/// Set only while the isolate thread is inside the user script and can service
/// `Runtime.evaluate` between tasks. False for every non-inspect run.
static INSPECTOR_ARMED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static ACTIVE_INSPECTOR: RefCell<Option<Arc<InspectorShared>>> = const { RefCell::new(None) };
}

pub struct EvalRequest {
    pub expression: String,
    pub reply: Sender<String>,
}

struct InspectorShared {
    host: String,
    port: u16,
    script_name: String,
    target_id: String,
    break_on_start: bool,
    should_resume: AtomicBool,
    evaluate_tx: Sender<EvalRequest>,
    evaluate_rx: Mutex<Receiver<EvalRequest>>,
    servicing: AtomicBool,
}

pub struct InspectorServer {
    shared: Arc<InspectorShared>,
}

/// Clears the isolate-thread evaluate pump when the user script returns.
pub struct UserScriptGuard<'a>(&'a InspectorServer);

impl Drop for UserScriptGuard<'_> {
    fn drop(&mut self) {
        self.0.set_user_script_active(false);
    }
}

impl InspectorServer {
    pub fn new(host: &str, port: u16, script_name: &str, break_on_start: bool) -> Self {
        let target_id = format!("{:x}", rand::random::<u64>());
        let (evaluate_tx, evaluate_rx) = mpsc::channel();
        Self {
            shared: Arc::new(InspectorShared {
                host: host.to_string(),
                port,
                script_name: script_name.to_string(),
                target_id,
                break_on_start,
                should_resume: AtomicBool::new(false),
                evaluate_tx,
                evaluate_rx: Mutex::new(evaluate_rx),
                servicing: AtomicBool::new(false),
            }),
        }
    }

    pub fn port(&self) -> u16 {
        self.shared.port
    }

    pub fn target_id(&self) -> &str {
        &self.shared.target_id
    }

    /// Binds `host:port` and serves CDP HTTP plus the `/ws` socket.
    pub fn start(&self) -> Result<()> {
        if self.shared.port == 0 {
            return Err(anyhow!(
                "{INSPECT_ERROR_PREFIX} --inspect-port must be between 1 and 65535"
            ));
        }
        let addr = format!("{}:{}", self.shared.host, self.shared.port);
        let listener = TcpListener::bind(&addr).map_err(|err| {
            anyhow!("{INSPECT_ERROR_PREFIX} failed to bind inspector on {addr}: {err}")
        })?;

        let host = self.shared.host.clone();
        let port = self.shared.port;
        println!("Debugger listening on ws://{host}:{port}/ws");

        let shared = self.shared.clone();
        thread::Builder::new()
            .name("amber-inspector".to_string())
            .spawn(move || accept_loop(listener, shared))
            .map_err(|err| {
                anyhow!("{INSPECT_ERROR_PREFIX} failed to start inspector thread: {err}")
            })?;
        Ok(())
    }

    /// Runs `eval` on this thread for each `Runtime.evaluate` until resume.
    ///
    /// `eval` receives a classic-script source that returns a JSON description
    /// of the value. It must run that source on the user isolate.
    pub fn wait_while_evaluating<F>(&self, mut eval: F)
    where
        F: FnMut(&str) -> Result<String, String>,
    {
        println!("Debugger attached wait: waiting for DevTools to connect...");
        while !self.shared.should_resume.load(Ordering::SeqCst) {
            let batch = drain_evals(&self.shared.evaluate_rx);
            if batch.is_empty() {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            for req in batch {
                let source = evaluate_expression_source(&req.expression);
                let result = eval(&source);
                let _ = req.reply.send(cdp_payload_from_runtime_result(result));
            }
        }
        println!("Debugger connected and resumed execution.");
    }

    /// Arms between-task `Runtime.evaluate` for the user script on this thread.
    pub fn enter_user_script(&self) -> UserScriptGuard<'_> {
        self.set_user_script_active(true);
        UserScriptGuard(self)
    }

    fn set_user_script_active(&self, active: bool) {
        if active {
            ACTIVE_INSPECTOR.with(|slot| *slot.borrow_mut() = Some(self.shared.clone()));
            INSPECTOR_ARMED.store(true, Ordering::Release);
        } else {
            INSPECTOR_ARMED.store(false, Ordering::Release);
            ACTIVE_INSPECTOR.with(|slot| *slot.borrow_mut() = None);
        }
    }
}

/// Called from the isolate thread between tasks. No-op when inspect is off.
pub fn service_pending_evaluations(scope: &mut v8::PinScope) {
    if !INSPECTOR_ARMED.load(Ordering::Acquire) {
        return;
    }
    let shared = ACTIVE_INSPECTOR.with(|slot| slot.borrow().clone());
    let Some(shared) = shared else {
        return;
    };
    if shared.servicing.swap(true, Ordering::SeqCst) {
        return;
    }
    let batch = drain_evals(&shared.evaluate_rx);
    for req in batch {
        let source = evaluate_expression_source(&req.expression);
        let result = eval_source_on_scope(scope, &source);
        let _ = req.reply.send(cdp_payload_from_runtime_result(result));
    }
    shared.servicing.store(false, Ordering::SeqCst);
}

fn drain_evals(rx: &Mutex<Receiver<EvalRequest>>) -> Vec<EvalRequest> {
    let guard = rx.lock().expect("inspector evaluate queue");
    let mut batch = Vec::new();
    while let Ok(req) = guard.try_recv() {
        batch.push(req);
    }
    batch
}

/// Classic script whose completion value is a JSON object describing the
/// indirect-eval result. The leading comment skips the runtime TS transpile.
pub fn evaluate_expression_source(expression: &str) -> String {
    let encoded = serde_json::to_string(expression).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        r#"// @amberjs-no-runtime-typescript-transpile
(function () {{
  var __amber_expr = {encoded};
  function __amber_pack(status, extra) {{
    var out = {{ status: status }};
    if (extra) {{
      for (var key in extra) {{
        if (Object.prototype.hasOwnProperty.call(extra, key)) out[key] = extra[key];
      }}
    }}
    return JSON.stringify(out);
  }}
  try {{
    var __amber_value = (0, eval)(__amber_expr);
    if (__amber_value === undefined) {{
      return __amber_pack("ok", {{ type: "undefined", description: "undefined" }});
    }}
    if (__amber_value === null) {{
      return __amber_pack("ok", {{
        type: "object",
        subtype: "null",
        value: null,
        description: "null"
      }});
    }}
    var __amber_type = typeof __amber_value;
    if (__amber_type === "number") {{
      if (!Number.isFinite(__amber_value)) {{
        return __amber_pack("ok", {{
          type: "number",
          description: String(__amber_value),
          unserializable: true
        }});
      }}
      return __amber_pack("ok", {{
        type: "number",
        value: __amber_value,
        description: String(__amber_value)
      }});
    }}
    if (__amber_type === "boolean" || __amber_type === "string") {{
      return __amber_pack("ok", {{
        type: __amber_type,
        value: __amber_value,
        description: String(__amber_value)
      }});
    }}
    if (__amber_type === "bigint") {{
      return __amber_pack("ok", {{
        type: "bigint",
        description: __amber_value.toString() + "n",
        unserializable: true
      }});
    }}
    var __amber_description = __amber_type;
    try {{
      __amber_description = String(__amber_value);
    }} catch (e) {{}}
    var __amber_extra = {{ type: __amber_type, description: __amber_description }};
    if (Array.isArray(__amber_value)) __amber_extra.subtype = "array";
    return __amber_pack("ok", __amber_extra);
  }} catch (e) {{
    var text = "exception";
    try {{
      text = e && e.stack ? String(e.stack) : String(e);
    }} catch (e2) {{}}
    return __amber_pack("exception", {{ text: text }});
  }}
}})()
"#
    )
}

pub fn cdp_payload_from_runtime_result(result: Result<String, String>) -> String {
    match result {
        Ok(text) => match serde_json::from_str::<Value>(text.trim()) {
            Ok(packed) => remote_object_from_packed(&packed),
            Err(_) => exception_payload(&text),
        },
        Err(err) => exception_payload(&err),
    }
}

fn remote_object_from_packed(packed: &Value) -> String {
    if packed.get("status").and_then(|status| status.as_str()) == Some("exception") {
        let text = packed
            .get("text")
            .and_then(|value| value.as_str())
            .unwrap_or("exception");
        return exception_payload(text);
    }
    let mut result = serde_json::Map::new();
    for key in ["type", "subtype", "value", "description"] {
        if let Some(value) = packed.get(key) {
            result.insert(key.to_string(), value.clone());
        }
    }
    if packed
        .get("unserializable")
        .and_then(|value| value.as_bool())
        == Some(true)
    {
        if let Some(description) = packed.get("description") {
            result.insert("unserializableValue".to_string(), description.clone());
        }
    }
    json!({ "result": Value::Object(result) }).to_string()
}

fn exception_payload(text: &str) -> String {
    json!({
        "result": { "type": "object", "description": text },
        "exceptionDetails": {
            "text": text,
            "exception": { "type": "object", "description": text }
        }
    })
    .to_string()
}

fn unsupported_method(id: &Value) -> String {
    json!({
        "id": id,
        "error": { "code": -32601, "message": UNSUPPORTED_METHOD_MESSAGE }
    })
    .to_string()
}

fn evaluate_timeout(id: &Value) -> String {
    json!({
        "id": id,
        "error": {
            "code": -32000,
            "message": "Runtime.evaluate was not serviced on the isolate thread"
        }
    })
    .to_string()
}

#[derive(Debug, PartialEq, Eq)]
enum RequestKind {
    Version,
    List,
    WebSocket,
    NotFound,
}

fn classify_request(header: &str) -> RequestKind {
    let mut lines = header.split('\n');
    let request_line = lines.next().unwrap_or("").trim_end_matches('\r');
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let raw_path = parts.next().unwrap_or("");
    let path = raw_path.split('?').next().unwrap_or(raw_path);
    if !method.eq_ignore_ascii_case("GET") {
        return RequestKind::NotFound;
    }
    let websocket = header.lines().any(|line| {
        let lower = line.trim_end_matches('\r').to_ascii_lowercase();
        lower.starts_with("upgrade:")
            && lower
                .split(':')
                .nth(1)
                .is_some_and(|value| value.split(',').any(|token| token.trim() == "websocket"))
    });
    if websocket && path == "/ws" {
        return RequestKind::WebSocket;
    }
    match path {
        "/json/version" => RequestKind::Version,
        "/json" | "/json/list" => RequestKind::List,
        _ => RequestKind::NotFound,
    }
}

fn accept_loop(listener: TcpListener, shared: Arc<InspectorShared>) {
    for stream_res in listener.incoming() {
        let mut stream = match stream_res {
            Ok(stream) => stream,
            Err(_) => continue,
        };
        let Some(header) = peek_headers(&mut stream) else {
            let _ = write_http(&mut stream, "400 Bad Request", "", None);
            continue;
        };
        match classify_request(&header) {
            RequestKind::Version => {
                let body = json!({
                    "Browser": format!("Amber/{}", env!("CARGO_PKG_VERSION")),
                    "Protocol-Version": "1.3"
                })
                .to_string();
                let _ = write_http(
                    &mut stream,
                    "200 OK",
                    &body,
                    Some("application/json; charset=UTF-8"),
                );
            }
            RequestKind::List => {
                let body = target_list(&shared).to_string();
                let _ = write_http(
                    &mut stream,
                    "200 OK",
                    &body,
                    Some("application/json; charset=UTF-8"),
                );
            }
            RequestKind::WebSocket => {
                let _ = stream.set_read_timeout(None);
                let shared = shared.clone();
                thread::spawn(move || handle_websocket(stream, shared));
            }
            RequestKind::NotFound => {
                let _ = write_http(&mut stream, "404 Not Found", "", None);
            }
        }
    }
}

fn target_list(shared: &InspectorShared) -> Value {
    let ws_url = format!("ws://{}:{}/ws", shared.host, shared.port);
    json!([{
        "description": "Amber runtime",
        "devtoolsFrontendUrl": format!(
            "devtools://devtools/bundled/js_app.html?ws={}:{}/ws",
            shared.host, shared.port
        ),
        "id": shared.target_id,
        "title": shared.script_name,
        "type": "node",
        "url": format!("file://{}", shared.script_name),
        "webSocketDebuggerUrl": ws_url
    }])
}

fn peek_headers(stream: &mut TcpStream) -> Option<String> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut buf = [0u8; 8192];
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        match stream.peek(&mut buf) {
            Ok(0) => return None,
            Ok(n) => {
                if buf[..n].windows(4).any(|window| window == b"\r\n\r\n") {
                    return Some(String::from_utf8_lossy(&buf[..n]).into_owned());
                }
                thread::sleep(Duration::from_millis(5));
            }
            Err(err)
                if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut =>
            {
                thread::sleep(Duration::from_millis(5));
            }
            Err(_) => return None,
        }
    }
    None
}

fn write_http(
    stream: &mut TcpStream,
    status: &str,
    body: &str,
    content_type: Option<&str>,
) -> std::io::Result<()> {
    let content_type = content_type.unwrap_or("text/plain; charset=UTF-8");
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(resp.as_bytes())?;
    stream.flush()
}

fn handle_websocket(stream: TcpStream, shared: Arc<InspectorShared>) {
    let mut ws = match tungstenite::accept(stream) {
        Ok(ws) => ws,
        Err(_) => return,
    };

    if shared.break_on_start {
        let paused_event = json!({
            "method": "Debugger.paused",
            "params": {
                "callFrames": [{
                    "callFrameId": "0",
                    "functionName": "(user script)",
                    "location": { "scriptId": "1", "lineNumber": 0, "columnNumber": 0 },
                    "url": format!("file://{}", shared.script_name),
                    "scopeChain": [],
                    "this": { "type": "undefined" }
                }],
                "reason": "Break on start"
            }
        });
        let _ = ws.send(tungstenite::Message::Text(paused_event.to_string()));
    }

    loop {
        let msg = match ws.read() {
            Ok(message) => message,
            Err(_) => break,
        };
        let tungstenite::Message::Text(text) = msg else {
            continue;
        };
        let Ok(parsed) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        let id = parsed.get("id").cloned();
        let method = parsed
            .get("method")
            .and_then(|method| method.as_str())
            .unwrap_or("");

        match method {
            "Runtime.runIfWaitingForDebugger" | "Debugger.resume" => {
                shared.should_resume.store(true, Ordering::SeqCst);
                let resumed = json!({ "method": "Debugger.resumed", "params": {} });
                let _ = ws.send(tungstenite::Message::Text(resumed.to_string()));
                if let Some(req_id) = id {
                    let resp = json!({ "id": req_id, "result": {} });
                    let _ = ws.send(tungstenite::Message::Text(resp.to_string()));
                }
            }
            "Runtime.evaluate" => {
                let Some(req_id) = id else {
                    continue;
                };
                let expression = parsed
                    .get("params")
                    .and_then(|params| params.get("expression"))
                    .and_then(|expression| expression.as_str())
                    .unwrap_or("")
                    .to_string();
                let (reply_tx, reply_rx) = mpsc::channel();
                if shared
                    .evaluate_tx
                    .send(EvalRequest {
                        expression,
                        reply: reply_tx,
                    })
                    .is_err()
                {
                    let _ = ws.send(tungstenite::Message::Text(evaluate_timeout(&req_id)));
                    continue;
                }
                let payload = reply_rx
                    .recv_timeout(EVALUATE_TIMEOUT)
                    .unwrap_or_else(|_| evaluate_timeout(&req_id));
                let body = attach_id(&payload, &req_id);
                let _ = ws.send(tungstenite::Message::Text(body));
            }
            _ => {
                if let Some(req_id) = id {
                    let _ = ws.send(tungstenite::Message::Text(unsupported_method(&req_id)));
                }
            }
        }
    }
}

fn attach_id(payload: &str, id: &Value) -> String {
    let mut body: Value = serde_json::from_str(payload).unwrap_or_else(|_| {
        json!({
            "error": {
                "code": -32000,
                "message": "Runtime.evaluate was not serviced on the isolate thread"
            }
        })
    });
    if let Some(obj) = body.as_object_mut() {
        if !obj.contains_key("id") {
            obj.insert("id".to_string(), id.clone());
        }
        return body.to_string();
    }
    json!({
        "id": id,
        "error": {
            "code": -32000,
            "message": "Runtime.evaluate was not serviced on the isolate thread"
        }
    })
    .to_string()
}

fn eval_source_on_scope(scope: &mut v8::PinScope, source: &str) -> Result<String, String> {
    let Some(code) = v8::String::new(scope, source) else {
        return Err("failed to create evaluation source".to_string());
    };
    let Some(script) = v8::Script::compile(scope, code, None) else {
        return Err("failed to compile evaluation".to_string());
    };
    v8::tc_scope!(let try_catch, scope);
    match script.run(try_catch) {
        Some(value) => value
            .to_string(try_catch)
            .map(|text| text.to_rust_string_lossy(try_catch))
            .ok_or_else(|| "failed to read evaluation result".to_string()),
        None => {
            let message = try_catch
                .exception()
                .and_then(|exception| exception.to_string(try_catch))
                .map(|message| message.to_rust_string_lossy(try_catch))
                .unwrap_or_else(|| "evaluation threw".to_string());
            Err(message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspector_server_creation() {
        let inspector = InspectorServer::new("127.0.0.1", 9239, "test.js", true);
        assert_eq!(inspector.port(), 9239);
        assert!(!inspector.target_id().is_empty());
    }

    #[test]
    fn port_zero_is_rejected() {
        let inspector = InspectorServer::new("127.0.0.1", 0, "test.js", false);
        let err = inspector.start().unwrap_err().to_string();
        assert!(err.starts_with(INSPECT_ERROR_PREFIX), "{err}");
        assert!(err.contains("--inspect-port"), "{err}");
    }

    #[test]
    fn classify_discovery_and_websocket_paths() {
        assert_eq!(
            classify_request("GET /json/version HTTP/1.1\r\nHost: x\r\n\r\n"),
            RequestKind::Version
        );
        assert_eq!(
            classify_request("GET /json/list?x=1 HTTP/1.1\r\n\r\n"),
            RequestKind::List
        );
        assert_eq!(
            classify_request("GET /json HTTP/1.1\r\n\r\n"),
            RequestKind::List
        );
        assert_eq!(
            classify_request("GET /missing HTTP/1.1\r\n\r\n"),
            RequestKind::NotFound
        );
        assert_eq!(
            classify_request("GET /ws HTTP/1.1\r\nUpgrade: websocket\r\n\r\n"),
            RequestKind::WebSocket
        );
        assert_eq!(
            classify_request("GET /other HTTP/1.1\r\nUpgrade: websocket\r\n\r\n"),
            RequestKind::NotFound
        );
        assert_eq!(
            classify_request("GET /ws HTTP/1.1\r\nHost: x\r\n\r\n"),
            RequestKind::NotFound
        );
    }

    #[test]
    fn expression_source_is_json_encoded() {
        let expression = "\"); throw 1; //";
        let source = evaluate_expression_source(expression);
        let encoded = serde_json::to_string(expression).unwrap();
        assert!(
            source.contains(&format!("var __amber_expr = {encoded};")),
            "{source}"
        );
        assert!(source.starts_with("// @amberjs-no-runtime-typescript-transpile"));
    }

    #[test]
    fn packed_number_and_exception_become_cdp_objects() {
        let number = cdp_payload_from_runtime_result(Ok(
            r#"{"status":"ok","type":"number","value":2,"description":"2"}"#.to_string(),
        ));
        let parsed: Value = serde_json::from_str(&number).unwrap();
        assert_eq!(parsed["result"]["type"], "number");
        assert_eq!(parsed["result"]["value"], 2);

        let thrown = cdp_payload_from_runtime_result(Ok(
            r#"{"status":"exception","text":"Error: boom"}"#.to_string(),
        ));
        let parsed: Value = serde_json::from_str(&thrown).unwrap();
        assert_eq!(parsed["exceptionDetails"]["text"], "Error: boom");
        assert!(parsed["result"].get("value").is_none());
    }
}

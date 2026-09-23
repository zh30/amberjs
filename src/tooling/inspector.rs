//! Chrome DevTools Protocol (CDP) Inspector Server for Amber.
//!
//! HTTP discovery (`/json/version`, `/json/list`) plus a WebSocket channel.
//! `Runtime.evaluate` is forwarded to the V8 thread via a queue; `--inspect-brk`
//! waits until `Runtime.runIfWaitingForDebugger` / `Debugger.resume`.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tungstenite::{accept, Message};

pub struct EvalRequest {
    pub expression: String,
    pub reply: Sender<String>,
}

pub struct InspectorServer {
    pub host: String,
    pub port: u16,
    pub script_name: String,
    pub target_id: String,
    pub is_connected: Arc<AtomicBool>,
    pub should_resume: Arc<AtomicBool>,
    evaluate_tx: Sender<EvalRequest>,
    evaluate_rx: Arc<Mutex<Option<Receiver<EvalRequest>>>>,
}

impl InspectorServer {
    pub fn new(host: &str, port: u16, script_name: &str) -> Self {
        let target_id = format!("{:x}", rand::random::<u64>());
        let (evaluate_tx, evaluate_rx) = mpsc::channel();
        Self {
            host: host.to_string(),
            port,
            script_name: script_name.to_string(),
            target_id,
            is_connected: Arc::new(AtomicBool::new(false)),
            should_resume: Arc::new(AtomicBool::new(false)),
            evaluate_tx,
            evaluate_rx: Arc::new(Mutex::new(Some(evaluate_rx))),
        }
    }

    /// Starts the HTTP/WebSocket inspector server in a background thread.
    pub fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.host, self.port);
        let listener = TcpListener::bind(&addr)
            .map_err(|e| anyhow!("Failed to bind inspector on {}: {}", addr, e))?;
        let _ = listener.set_nonblocking(false);

        let host = self.host.clone();
        let port = self.port;
        let script_name = self.script_name.clone();
        let target_id = self.target_id.clone();
        let is_connected = self.is_connected.clone();
        let should_resume = self.should_resume.clone();
        let evaluate_tx = self.evaluate_tx.clone();

        println!("Debugger listening on ws://{}:{}/ws", host, port);
        println!(
            "For Chrome DevTools, open: devtools://devtools/bundled/js_app.html?ws={}:{}/ws",
            host, port
        );

        thread::spawn(move || {
            for stream_res in listener.incoming() {
                let mut stream = match stream_res {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let mut buf = [0u8; 2048];
                let n = match stream.peek(&mut buf) {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                let req_header = String::from_utf8_lossy(&buf[..n]);

                if req_header.starts_with("GET /json") {
                    handle_http_json(
                        &mut stream,
                        &req_header,
                        &host,
                        port,
                        &script_name,
                        &target_id,
                    );
                } else if req_header.contains("Upgrade: websocket")
                    || req_header.contains("upgrade: websocket")
                {
                    handle_websocket(
                        stream,
                        is_connected.clone(),
                        should_resume.clone(),
                        evaluate_tx.clone(),
                        script_name.clone(),
                    );
                } else {
                    let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
                    let _ = stream.write_all(resp.as_bytes());
                }
            }
        });

        Ok(())
    }

    /// Waits until debugger signals resume, draining `Runtime.evaluate` on this thread.
    pub fn wait_while_evaluating<F>(&self, mut eval: F)
    where
        F: FnMut(&str) -> Result<String, String>,
    {
        println!("Debugger attached wait: waiting for DevTools to connect...");
        let rx = self.evaluate_rx.lock().unwrap().take();
        let Some(rx) = rx else {
            self.wait_for_debugger();
            return;
        };
        while !self.should_resume.load(Ordering::SeqCst) {
            match rx.try_recv() {
                Ok(req) => {
                    let result = eval(&req.expression);
                    let payload = match result {
                        Ok(value) => cdp_remote_object(&value),
                        Err(err) => json!({
                            "result": {
                                "type": "string",
                                "value": err,
                                "description": err
                            },
                            "exceptionDetails": { "text": err }
                        })
                        .to_string(),
                    };
                    let _ = req.reply.send(payload);
                }
                Err(mpsc::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        }
        *self.evaluate_rx.lock().unwrap() = Some(rx);
        println!("Debugger connected and resumed execution.");
    }

    /// Waits until debugger connects and signals to resume.
    pub fn wait_for_debugger(&self) {
        println!("Debugger attached wait: waiting for DevTools to connect...");
        while !self.should_resume.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(50));
        }
        println!("Debugger connected and resumed execution.");
    }
}

fn cdp_remote_object(value: &str) -> String {
    let trimmed = value.trim();
    if let Ok(n) = trimmed.parse::<f64>() {
        return json!({
            "result": {
                "type": "number",
                "value": n,
                "description": trimmed
            }
        })
        .to_string();
    }
    if trimmed == "true" || trimmed == "false" {
        let b = trimmed == "true";
        return json!({
            "result": {
                "type": "boolean",
                "value": b,
                "description": trimmed
            }
        })
        .to_string();
    }
    json!({
        "result": {
            "type": "string",
            "value": trimmed,
            "description": trimmed
        }
    })
    .to_string()
}

fn handle_http_json(
    stream: &mut TcpStream,
    req_header: &str,
    host: &str,
    port: u16,
    script_name: &str,
    target_id: &str,
) {
    let ws_url = format!("ws://{}:{}/ws", host, port);

    let body = if req_header.contains("GET /json/version") {
        json!({
            "Browser": format!("Amber/{}", env!("CARGO_PKG_VERSION")),
            "Protocol-Version": "1.3"
        })
    } else {
        json!([
            {
                "description": "Amber runtime",
                "devtoolsFrontendUrl": format!("devtools://devtools/bundled/js_app.html?ws={}:{}/ws", host, port),
                "id": target_id,
                "title": script_name,
                "type": "node",
                "url": format!("file://{}", script_name),
                "webSocketDebuggerUrl": ws_url
            }
        ])
    };

    let body_str = body.to_string();
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=UTF-8\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}",
        body_str.len(),
        body_str
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.flush();
}

fn handle_websocket(
    stream: TcpStream,
    is_connected: Arc<AtomicBool>,
    should_resume: Arc<AtomicBool>,
    evaluate_tx: Sender<EvalRequest>,
    script_name: String,
) {
    thread::spawn(move || {
        let mut ws = match accept(stream) {
            Ok(ws) => ws,
            Err(_) => return,
        };

        is_connected.store(true, Ordering::SeqCst);

        let paused_event = json!({
            "method": "Debugger.paused",
            "params": {
                "callFrames": [{
                    "callFrameId": "0",
                    "functionName": "(user script)",
                    "location": { "scriptId": "1", "lineNumber": 0, "columnNumber": 0 },
                    "url": format!("file://{}", script_name),
                    "scopeChain": [],
                    "this": { "type": "undefined" }
                }],
                "reason": "Break on start"
            }
        });
        let _ = ws.send(Message::Text(paused_event.to_string()));

        loop {
            let msg = match ws.read() {
                Ok(m) => m,
                Err(_) => break,
            };

            if let Message::Text(text) = msg {
                if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                    let id = parsed.get("id").cloned();
                    let method = parsed.get("method").and_then(|m| m.as_str()).unwrap_or("");

                    match method {
                        "Runtime.runIfWaitingForDebugger" | "Debugger.resume" => {
                            should_resume.store(true, Ordering::SeqCst);
                            let resumed = json!({ "method": "Debugger.resumed", "params": {} });
                            let _ = ws.send(Message::Text(resumed.to_string()));

                            if let Some(req_id) = id {
                                let resp = json!({ "id": req_id, "result": {} });
                                let _ = ws.send(Message::Text(resp.to_string()));
                            }
                        }
                        "Runtime.evaluate" => {
                            let expression = parsed
                                .get("params")
                                .and_then(|p| p.get("expression"))
                                .and_then(|e| e.as_str())
                                .unwrap_or("")
                                .to_string();
                            let (reply_tx, reply_rx) = mpsc::channel();
                            let _ = evaluate_tx.send(EvalRequest {
                                expression,
                                reply: reply_tx,
                            });
                            let payload = reply_rx
                                .recv_timeout(Duration::from_secs(10))
                                .unwrap_or_else(|_| {
                                    json!({
                                        "result": {
                                            "type": "undefined"
                                        }
                                    })
                                    .to_string()
                                });
                            if let Some(req_id) = id {
                                let mut body: Value =
                                    serde_json::from_str(&payload).unwrap_or(json!({}));
                                if let Some(obj) = body.as_object_mut() {
                                    obj.insert("id".to_string(), req_id);
                                } else {
                                    body =
                                        json!({ "id": req_id, "result": { "type": "undefined" } });
                                }
                                let _ = ws.send(Message::Text(body.to_string()));
                            }
                        }
                        "Debugger.stepOver" | "Debugger.stepInto" | "Debugger.stepOut" => {
                            if let Some(req_id) = id {
                                let resp = json!({ "id": req_id, "result": {} });
                                let _ = ws.send(Message::Text(resp.to_string()));
                            }
                        }
                        _ => {
                            if let Some(req_id) = id {
                                let resp = json!({ "id": req_id, "result": {} });
                                let _ = ws.send(Message::Text(resp.to_string()));
                            }
                        }
                    }
                }
            }
        }

        is_connected.store(false, Ordering::SeqCst);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspector_server_creation() {
        let inspector = InspectorServer::new("127.0.0.1", 9239, "test.js");
        assert_eq!(inspector.port, 9239);
        assert!(!inspector.target_id.is_empty());
    }
}

//! Pins the Stable `amber run --inspect` / `--inspect-brk` contract.
//! See docs/INSPECT_CONTRACT.md.

use serde_json::{json, Value};
use serial_test::serial;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use tungstenite::{connect, Message, WebSocket};

fn amber() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

struct KillOnDrop(Option<Child>);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn spawn(args: &[String]) -> KillOnDrop {
    let child = Command::new(amber())
        .args(args)
        .env_remove("AMBER_WORKERS")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn amber {args:?}: {err}"));
    KillOnDrop(Some(child))
}

fn http_get(port: u16, path: &str) -> String {
    let mut last = String::new();
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(20) {
        if let Ok(mut stream) = std::net::TcpStream::connect(("127.0.0.1", port)) {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let req =
                format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
            let _ = stream.write_all(req.as_bytes());
            let mut buf = String::new();
            let _ = stream.read_to_string(&mut buf);
            if buf.contains("HTTP/1.1 ") {
                return buf;
            }
            last = buf;
        }
        thread::sleep(Duration::from_millis(50));
    }
    last
}

fn wait_until(mut ready: impl FnMut() -> bool) {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(20) {
        if ready() {
            return;
        }
        thread::sleep(Duration::from_millis(30));
    }
    panic!("timed out waiting for condition");
}

fn connect_ws(port: u16) -> WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>> {
    let start = Instant::now();
    let mut last = String::new();
    while start.elapsed() < Duration::from_secs(20) {
        match connect(format!("ws://127.0.0.1:{port}/ws")) {
            Ok((ws, _)) => return ws,
            Err(err) => last = err.to_string(),
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("ws connect failed: {last}");
}

fn read_text(
    ws: &mut WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
) -> String {
    match ws.read().expect("ws read") {
        Message::Text(text) => text,
        other => panic!("unexpected ws message: {other:?}"),
    }
}

fn send_call(
    ws: &mut WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    id: i64,
    method: &str,
    params: Value,
) {
    let text = json!({ "id": id, "method": method, "params": params }).to_string();
    ws.send(Message::Text(text)).expect("ws send");
}

fn read_until_id(
    ws: &mut WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    id: i64,
    allow_paused: bool,
) -> Value {
    for _ in 0..8 {
        let text = read_text(ws);
        let value: Value = serde_json::from_str(&text).unwrap_or_else(|err| {
            panic!("cdp json {err}: {text}");
        });
        if value.get("method").and_then(|method| method.as_str()) == Some("Debugger.paused")
            && !allow_paused
        {
            panic!("unexpected Debugger.paused: {value}");
        }
        if value.get("id").and_then(|value| value.as_i64()) == Some(id) {
            return value;
        }
    }
    panic!("no CDP reply for id {id}");
}

fn wait_success(child: &mut KillOnDrop) {
    let child = child.0.as_mut().expect("child");
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < Duration::from_secs(20) => {
                thread::sleep(Duration::from_millis(30));
            }
            Ok(None) => {
                let _ = child.kill();
                panic!("amber did not exit");
            }
            Err(err) => panic!("wait: {err}"),
        }
    };
    assert!(status.success(), "amber exit status: {status}");
}

fn read_stderr(child: &mut KillOnDrop) -> String {
    let child = child.0.as_mut().expect("child");
    let mut stderr = String::new();
    if let Some(pipe) = child.stderr.as_mut() {
        let _ = pipe.read_to_string(&mut stderr);
    }
    stderr
}

#[test]
#[serial]
fn inspect_brk_discovery_evaluate_and_resume() {
    let dir = tempdir().unwrap();
    let side = dir.path().join("side.txt");
    let preload = dir.path().join("preload.js");
    let script = dir.path().join("paused.js");
    std::fs::write(&preload, "globalThis.__fromPreload = 7;\n").unwrap();
    let side_json = serde_json::to_string(&side.to_string_lossy().as_ref()).unwrap();
    std::fs::write(
        &script,
        format!(
            "require('fs').writeFileSync({side_json}, String(globalThis.__amberInspect) + ':' + String(globalThis.__fromPreload));\n"
        ),
    )
    .unwrap();

    let port = free_port();
    let mut child = spawn(&[
        "run".into(),
        "--inspect-brk".into(),
        "--inspect-port".into(),
        port.to_string(),
        "--require".into(),
        preload.to_string_lossy().into(),
        script.to_string_lossy().into(),
    ]);

    let version = http_get(port, "/json/version");
    assert!(
        version.contains("HTTP/1.1 200"),
        "version status: {version}"
    );
    assert!(
        version.contains(&format!("Amber/{}", env!("CARGO_PKG_VERSION"))),
        "{version}"
    );
    assert!(
        version.contains("\"Protocol-Version\":\"1.3\""),
        "{version}"
    );

    let list = http_get(port, "/json/list");
    assert!(
        list.contains(&format!("ws://127.0.0.1:{port}/ws")),
        "{list}"
    );
    let missing = http_get(port, "/not-cdp");
    assert!(missing.contains("HTTP/1.1 404"), "{missing}");
    assert!(!side.exists(), "user script ran before resume");

    let mut ws = connect_ws(port);
    let paused = read_text(&mut ws);
    assert!(paused.contains("Debugger.paused"), "{paused}");
    assert!(paused.contains("Break on start"), "{paused}");
    assert!(paused.contains("\"scopeChain\":[]"), "{paused}");

    send_call(&mut ws, 1, "Runtime.evaluate", json!({"expression": "1+1"}));
    let one = read_until_id(&mut ws, 1, true);
    assert_eq!(one["result"]["type"], "number", "{one}");
    assert_eq!(one["result"]["value"], 2, "{one}");

    send_call(
        &mut ws,
        2,
        "Runtime.evaluate",
        json!({"expression": "globalThis.__fromPreload"}),
    );
    let pre = read_until_id(&mut ws, 2, true);
    assert_eq!(pre["result"]["value"], 7, "{pre}");

    send_call(
        &mut ws,
        3,
        "Runtime.evaluate",
        json!({"expression": "globalThis.__amberInspect = 40"}),
    );
    let assigned = read_until_id(&mut ws, 3, true);
    assert_eq!(assigned["result"]["value"], 40, "{assigned}");

    send_call(
        &mut ws,
        4,
        "Runtime.evaluate",
        json!({"expression": "null"}),
    );
    let null_value = read_until_id(&mut ws, 4, true);
    assert_eq!(null_value["result"]["subtype"], "null", "{null_value}");

    send_call(
        &mut ws,
        5,
        "Runtime.evaluate",
        json!({"expression": "undefined"}),
    );
    let undef = read_until_id(&mut ws, 5, true);
    assert_eq!(undef["result"]["type"], "undefined", "{undef}");
    assert!(undef["result"].get("value").is_none(), "{undef}");

    send_call(
        &mut ws,
        6,
        "Runtime.evaluate",
        json!({"expression": "true"}),
    );
    let flag = read_until_id(&mut ws, 6, true);
    assert_eq!(flag["result"]["type"], "boolean", "{flag}");
    assert_eq!(flag["result"]["value"], true, "{flag}");

    send_call(
        &mut ws,
        7,
        "Runtime.evaluate",
        json!({"expression": "\"ab\""}),
    );
    let text = read_until_id(&mut ws, 7, true);
    assert_eq!(text["result"]["type"], "string", "{text}");
    assert_eq!(text["result"]["value"], "ab", "{text}");

    send_call(
        &mut ws,
        8,
        "Runtime.evaluate",
        json!({"expression": "throw new Error(\"boom\")"}),
    );
    let thrown = read_until_id(&mut ws, 8, true);
    let details = thrown["exceptionDetails"]["text"].as_str().unwrap_or("");
    assert!(details.contains("boom"), "{thrown}");
    assert!(thrown["result"].get("value").is_none(), "{thrown}");

    send_call(&mut ws, 9, "Debugger.stepOver", json!({}));
    let step = read_until_id(&mut ws, 9, true);
    assert_eq!(step["error"]["code"], -32601, "{step}");
    assert!(
        step["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("does not support"),
        "{step}"
    );
    assert!(!side.exists(), "stepOver ran the user script");

    send_call(&mut ws, 10, "Debugger.resume", json!({}));
    let _resumed = read_until_id(&mut ws, 10, true);
    wait_success(&mut child);
    assert_eq!(
        std::fs::read_to_string(&side).unwrap(),
        "40:7",
        "user script must see pause-time assignments and the preload"
    );
}

#[test]
#[serial]
fn inspect_brk_resumes_on_run_if_waiting_for_debugger() {
    let dir = tempdir().unwrap();
    let side = dir.path().join("ran.txt");
    let script = dir.path().join("go.js");
    let side_json = serde_json::to_string(&side.to_string_lossy().as_ref()).unwrap();
    std::fs::write(
        &script,
        format!("require('fs').writeFileSync({side_json}, 'ran');\n"),
    )
    .unwrap();
    let port = free_port();
    let mut child = spawn(&[
        "run".into(),
        "--inspect-brk".into(),
        "--inspect-port".into(),
        port.to_string(),
        script.to_string_lossy().into(),
    ]);
    let _version = http_get(port, "/json/version");
    assert!(!side.exists());
    let mut ws = connect_ws(port);
    let _paused = read_text(&mut ws);
    send_call(&mut ws, 1, "Runtime.runIfWaitingForDebugger", json!({}));
    let _reply = read_until_id(&mut ws, 1, true);
    wait_success(&mut child);
    assert_eq!(std::fs::read_to_string(&side).unwrap(), "ran");
}

#[test]
#[serial]
fn inspect_runs_immediately_and_evaluates_on_the_live_isolate() {
    let dir = tempdir().unwrap();
    let side = dir.path().join("up.txt");
    let script = dir.path().join("live.js");
    let side_json = serde_json::to_string(&side.to_string_lossy().as_ref()).unwrap();
    std::fs::write(
        &script,
        format!(
            "require('fs').writeFileSync({side_json}, 'up');\nglobalThis.__amberInspect = 41;\nsetInterval(function () {{}}, 1000);\n"
        ),
    )
    .unwrap();
    let port = free_port();
    let child = spawn(&[
        "run".into(),
        "--inspect".into(),
        "--inspect-port".into(),
        port.to_string(),
        script.to_string_lossy().into(),
    ]);

    wait_until(|| side.exists());
    let version = http_get(port, "/json/version");
    assert!(
        version.contains(&format!("Amber/{}", env!("CARGO_PKG_VERSION"))),
        "{version}"
    );

    let mut ws = connect_ws(port);
    send_call(
        &mut ws,
        1,
        "Runtime.evaluate",
        json!({"expression": "globalThis.__amberInspect + 1"}),
    );
    let value = read_until_id(&mut ws, 1, false);
    assert_eq!(value["result"]["value"], 42, "{value}");
    drop(child);
}

#[test]
#[serial]
fn inspect_bind_failure_does_not_run_the_script() {
    let dir = tempdir().unwrap();
    let side = dir.path().join("side.txt");
    let script = dir.path().join("nope.js");
    let side_json = serde_json::to_string(&side.to_string_lossy().as_ref()).unwrap();
    std::fs::write(
        &script,
        format!("require('fs').writeFileSync({side_json}, 'ran');\n"),
    )
    .unwrap();
    let port = free_port();
    let _hold = TcpListener::bind(("127.0.0.1", port)).unwrap();
    let mut child = spawn(&[
        "run".into(),
        "--inspect-brk".into(),
        "--inspect-port".into(),
        port.to_string(),
        script.to_string_lossy().into(),
    ]);
    let child_ref = child.0.as_mut().unwrap();
    let start = Instant::now();
    let status = loop {
        match child_ref.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < Duration::from_secs(30) => {
                thread::sleep(Duration::from_millis(30));
            }
            Ok(None) => panic!("bind failure did not exit"),
            Err(err) => panic!("{err}"),
        }
    };
    let stderr = read_stderr(&mut child);
    assert!(!status.success(), "status {status}, stderr {stderr}");
    assert!(
        stderr.contains("error: amber run:") && stderr.contains("failed to bind inspector"),
        "{stderr}"
    );
    assert!(!side.exists(), "script ran after bind failure");
}

#[test]
#[serial]
fn inspect_rejects_port_zero_watch_and_workers() {
    let dir = tempdir().unwrap();
    let side = dir.path().join("side.txt");
    let script = dir.path().join("nope.js");
    let side_json = serde_json::to_string(&side.to_string_lossy().as_ref()).unwrap();
    std::fs::write(
        &script,
        format!("require('fs').writeFileSync({side_json}, 'ran');\n"),
    )
    .unwrap();
    let script_arg = script.to_string_lossy().to_string();

    let cases = [
        (
            vec![
                "run".into(),
                "--inspect".into(),
                "--inspect-port".into(),
                "0".into(),
                script_arg.clone(),
            ],
            "--inspect-port",
        ),
        (
            vec![
                "run".into(),
                "--inspect".into(),
                "--watch".into(),
                script_arg.clone(),
            ],
            "cannot be combined with --watch",
        ),
        (
            vec![
                "run".into(),
                "--inspect-brk".into(),
                "--workers".into(),
                "2".into(),
                script_arg.clone(),
            ],
            "cannot be combined with --workers",
        ),
    ];

    for (args, needle) in cases {
        let mut child = spawn(&args);
        let child_ref = child.0.as_mut().unwrap();
        let start = Instant::now();
        let status = loop {
            match child_ref.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if start.elapsed() < Duration::from_secs(15) => {
                    thread::sleep(Duration::from_millis(20));
                }
                Ok(None) => panic!("did not exit for {args:?}"),
                Err(err) => panic!("{err}"),
            }
        };
        let stderr = read_stderr(&mut child);
        assert!(
            !status.success(),
            "{args:?} status {status} stderr {stderr}"
        );
        assert!(
            stderr.contains("error: amber run:") && stderr.contains(needle),
            "{args:?} stderr {stderr}"
        );
        assert!(!side.exists(), "{args:?} ran the script");
    }

    let mut child = Command::new(amber())
        .args(["run", "--inspect", &script_arg])
        .env("AMBER_WORKERS", "2")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < Duration::from_secs(15) => {
                thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                panic!("AMBER_WORKERS did not exit");
            }
            Err(err) => panic!("{err}"),
        }
    };
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut stderr);
    }
    assert!(!status.success(), "{stderr}");
    assert!(
        stderr.contains("error: amber run:")
            && stderr.contains("cannot be combined with --workers"),
        "{stderr}"
    );
    assert!(!side.exists());
}

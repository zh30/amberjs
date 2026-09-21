use serial_test::serial;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use tungstenite::{connect, Message};

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

fn http_get(host_port: &str, path: &str) -> String {
    let mut last = String::new();
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        if let Ok(mut stream) = std::net::TcpStream::connect(host_port) {
            let req =
                format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
            let _ = stream.write_all(req.as_bytes());
            let mut buf = String::new();
            let _ = stream.read_to_string(&mut buf);
            if buf.contains("HTTP/1.1 200") {
                return buf;
            }
            last = buf;
        }
        thread::sleep(Duration::from_millis(50));
    }
    last
}

#[test]
#[serial]
fn inspect_brk_evaluate_and_resume() {
    let dir = tempdir().unwrap();
    let side = dir.path().join("side-effect.txt");
    let script = dir.path().join("paused.js");
    std::fs::write(
        &script,
        format!(
            "require('fs').writeFileSync({}, 'ran');\n",
            serde_json::to_string(&side.to_string_lossy().as_ref()).unwrap()
        ),
    )
    .unwrap();

    let port = free_port();
    let mut child = Command::new(amber())
        .args([
            "run",
            "--inspect-brk",
            "--inspect-port",
            &port.to_string(),
            script.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn amber run --inspect-brk");

    let version = http_get(&format!("127.0.0.1:{port}"), "/json/version");
    assert!(
        version.contains("Amber/") && version.contains(env!("CARGO_PKG_VERSION")),
        "GET /json/version: {version}"
    );

    assert!(!side.exists(), "user script must not run before resume");

    let (mut ws, _) = connect(format!("ws://127.0.0.1:{port}/ws")).expect("ws connect");
    let _paused = ws.read().ok();

    ws.send(Message::Text(
        r#"{"id":1,"method":"Runtime.evaluate","params":{"expression":"1+1"}}"#.into(),
    ))
    .unwrap();
    let eval_msg = ws.read().expect("evaluate reply");
    let eval_text = match eval_msg {
        Message::Text(t) => t,
        other => panic!("unexpected ws message: {other:?}"),
    };
    assert!(
        eval_text.contains("\"value\":2") || eval_text.contains("\"value\": 2"),
        "Runtime.evaluate 1+1 should be 2: {eval_text}"
    );

    ws.send(Message::Text(
        r#"{"id":2,"method":"Runtime.runIfWaitingForDebugger"}"#.into(),
    ))
    .unwrap();

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < Duration::from_secs(15) => {
                if side.exists() {
                    thread::sleep(Duration::from_millis(50));
                    let _ = child.try_wait();
                }
                thread::sleep(Duration::from_millis(50));
            }
            Ok(None) => {
                let _ = child.kill();
                panic!("amber did not exit after resume");
            }
            Err(e) => panic!("wait: {e}"),
        }
    };
    assert!(
        status.success(),
        "amber should exit 0 after resume: {status}"
    );
    assert!(side.exists(), "user script should run after resume");
    let _ = PathBuf::from(&side);
}

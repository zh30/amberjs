//! Executable contract for `amber run --watch` and `amber test --watch`.
//! See `docs/WATCH_CONTRACT.md`.

use amberjs::permissions::{
    global_resource_broker, PermissionAction, PermissionKind, ResourceBroker, ResourceId,
};
use amberjs::watcher::{FileChange, FileChangeType, HotReloader, WatcherConfigBuilder};
use amberjs::watcher_websocket::{HotReloadEvent, WebSocketConfig, WebSocketHotReloader};
use futures_util::{SinkExt, StreamExt};
use serial_test::serial;
use std::io::Read;
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

fn reset_broker() {
    amberjs::permissions::reset_runtime_permission_state();
    *global_resource_broker()
        .write()
        .expect("resource broker lock") = ResourceBroker::default();
}

fn quiet_reloader(debounce_ms: u64) -> HotReloader {
    HotReloader::with_config(
        WatcherConfigBuilder::new()
            .debounce_ms(debounce_ms)
            .clear_console(false)
            .show_notifications(false)
            .build(),
    )
}

fn recv_change(rx: &mpsc::Receiver<FileChange>, timeout: Duration) -> FileChange {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let slice =
            Duration::from_millis(200).min(deadline.saturating_duration_since(Instant::now()));
        match rx.recv_timeout(slice) {
            Ok(change) => return change,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => panic!("watcher disconnected"),
        }
    }
    panic!("timed out waiting for a file event");
}

/// macOS temp dirs are `/var/...` while notify reports `/private/var/...`.
fn canonical_path(path: &Path) -> std::path::PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    // A removed file cannot be canonicalized. Resolve the parent, which still exists.
    if let (Some(parent), Some(name)) = (path.parent(), path.file_name()) {
        if let Ok(parent) = parent.canonicalize() {
            return parent.join(name);
        }
    }
    path.to_path_buf()
}

fn assert_same_path(actual: &Path, expected: &Path) {
    let actual = canonical_path(actual);
    let expected = canonical_path(expected);
    assert_eq!(actual, expected);
}

fn assert_no_event(rx: &mpsc::Receiver<FileChange>, wait: Duration) {
    match rx.recv_timeout(wait) {
        Err(mpsc::RecvTimeoutError::Timeout) => {}
        Ok(change) => panic!("unexpected event {}", change.path.display()),
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("watcher disconnected"),
    }
}

#[test]
#[serial]
fn watch_emits_one_event_per_quiet_period_and_ignores_unwatched_paths() {
    reset_broker();
    let dir = TempDir::new().expect("tempdir");
    let app = dir.path().join("app.js");
    let nested = dir.path().join("nested");
    std::fs::create_dir_all(nested.join("node_modules")).expect("dirs");
    std::fs::create_dir_all(dir.path().join("target")).expect("target");
    std::fs::write(nested.join("keep.ts"), "export {}\n").expect("nested ts");
    std::fs::write(dir.path().join("notes.txt"), "nope").expect("txt");

    let mut reloader = quiet_reloader(200);
    let rx = reloader.watch(dir.path()).expect("watch");
    assert!(reloader.is_running());
    assert_eq!(reloader.get_stats().files_watched, 1);

    std::fs::write(dir.path().join("target").join("built.js"), "nope").expect("target js");
    std::fs::write(nested.join("node_modules").join("dep.js"), "nope").expect("dep");
    std::fs::write(dir.path().join("notes.txt"), "still nope").expect("txt write");
    std::thread::sleep(Duration::from_millis(200));

    std::fs::write(&app, "console.log('one');\n").expect("create");
    std::thread::sleep(Duration::from_millis(30));
    std::fs::write(&app, "console.log('two');\n").expect("rewrite");

    let change = recv_change(&rx, Duration::from_secs(3));
    assert_same_path(&change.path, &app);
    assert!(
        matches!(
            change.change_type,
            FileChangeType::Created | FileChangeType::Modified
        ),
        "create/write should reload, got {:?}",
        change.change_type
    );
    assert_no_event(&rx, Duration::from_millis(250));

    std::fs::remove_file(&app).expect("remove");
    let removed = recv_change(&rx, Duration::from_secs(3));
    assert_same_path(&removed.path, &app);
    assert!(
        matches!(
            removed.change_type,
            FileChangeType::Removed | FileChangeType::Modified
        ),
        "delete should reload, got {:?}",
        removed.change_type
    );

    reloader.stop();
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Ok(_) => {}
            Err(mpsc::RecvTimeoutError::Timeout) if Instant::now() > deadline => {
                panic!("stop() did not disconnect the watcher")
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    assert!(!reloader.is_running());
    reset_broker();
}

#[test]
#[serial]
fn non_recursive_watch_skips_nested_scripts() {
    reset_broker();
    let dir = TempDir::new().expect("tempdir");
    let nested = dir.path().join("nested");
    std::fs::create_dir(&nested).expect("nested");
    let mut reloader = HotReloader::with_config(
        WatcherConfigBuilder::new()
            .debounce_ms(50)
            .recursive(false)
            .clear_console(false)
            .show_notifications(false)
            .build(),
    );
    let rx = reloader.watch(dir.path()).expect("watch");
    assert_eq!(reloader.get_stats().files_watched, 0);

    std::fs::write(nested.join("inner.js"), "console.log('inner');\n").expect("inner");
    std::thread::sleep(Duration::from_millis(180));
    let top = dir.path().join("top.js");
    std::fs::write(&top, "console.log('top');\n").expect("top");
    let change = recv_change(&rx, Duration::from_secs(3));
    assert_same_path(&change.path, &top);
    reloader.stop();
    std::thread::sleep(Duration::from_millis(200));
    reset_broker();
}

#[test]
#[serial]
fn missing_watch_path_fails_before_running() {
    reset_broker();
    let missing = std::env::temp_dir().join(format!("amber-watch-missing-{}", std::process::id()));
    let mut reloader = quiet_reloader(50);
    let error = reloader
        .watch(&missing)
        .expect_err("missing path must fail");
    assert!(error.to_string().contains("does not exist"), "{error}");
    assert!(!reloader.is_running());
    assert_eq!(reloader.get_stats().files_watched, 0);
    reset_broker();
}

#[test]
#[serial]
fn watch_denies_unreadable_root_before_start() {
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(dir.path().join("app.js"), "console.log('x');\n").expect("app");
    let mut reloader = quiet_reloader(50);
    {
        let mut broker = global_resource_broker().write().expect("broker");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(dir.path().to_path_buf()),
        );
    }
    let error = reloader.watch(dir.path()).expect_err("denied");
    reloader.stop();
    reset_broker();
    assert!(error.to_string().contains("permission denied"), "{error}");
    assert!(!reloader.is_running());
    assert_eq!(reloader.get_stats().files_watched, 0);
}

#[tokio::test]
#[serial]
async fn websocket_client_receives_status_then_reload_and_stop_releases_port() {
    reset_broker();
    let reloader = WebSocketHotReloader::with_config(WebSocketConfig {
        port: 0,
        host: "127.0.0.1".to_string(),
        channel_capacity: 8,
    });
    let addr = reloader.start().await.expect("bind");
    assert!(reloader.is_running());
    assert_eq!(reloader.server_addr(), addr.to_string());
    let again = reloader.start().await.expect_err("second start");
    assert!(again.contains("already running"), "{again}");

    let (mut socket, _) = tokio_tungstenite::connect_async(format!("ws://{addr}"))
        .await
        .expect("client connect");
    let hello = next_text(&mut socket).await;
    let hello: HotReloadEvent = serde_json::from_str(&hello).expect("status json");
    assert_eq!(hello.event_type, "status");
    assert_eq!(hello.message.as_deref(), Some("connected"));

    socket
        .send(Message::Text("ping".to_string()))
        .await
        .expect("ping");
    let pong = next_text(&mut socket).await;
    assert_eq!(pong, "pong");

    reloader.broadcast_reload("app.js".to_string(), "modified".to_string());
    let reload = next_text(&mut socket).await;
    assert!(reload.contains("\"event_type\":\"reload\""), "{reload}");
    assert!(reload.contains("\"file_path\":\"app.js\""), "{reload}");
    assert!(reload.contains("\"change_type\":\"modified\""), "{reload}");

    reloader.stop();
    assert!(!reloader.is_running());
    let mut released = false;
    for _ in 0..50 {
        if std::net::TcpListener::bind(addr).is_ok() {
            released = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(released, "stop() should release {addr}");
    reset_broker();
}

async fn next_text(
    socket: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> String {
    let message = tokio::time::timeout(Duration::from_secs(3), socket.next())
        .await
        .expect("timed out waiting for websocket frame")
        .expect("socket closed")
        .expect("websocket error");
    match message {
        Message::Text(text) => text,
        other => panic!("expected text, got {other:?}"),
    }
}

struct AmberProc {
    child: Child,
    log: std::sync::Arc<Mutex<String>>,
}

impl AmberProc {
    fn spawn(args: &[&str], cwd: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_amber"))
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn amber");
        let log = std::sync::Arc::new(Mutex::new(String::new()));
        fn pump(mut stream: impl Read + Send + 'static, log: std::sync::Arc<Mutex<String>>) {
            std::thread::spawn(move || {
                let mut buf = [0u8; 1024];
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let text = String::from_utf8_lossy(&buf[..n]);
                            log.lock().expect("log").push_str(&text);
                        }
                    }
                }
            });
        }
        if let Some(stdout) = child.stdout.take() {
            pump(stdout, log.clone());
        }
        if let Some(stderr) = child.stderr.take() {
            pump(stderr, log.clone());
        }
        Self { child, log }
    }

    fn snapshot(&self) -> String {
        self.log.lock().expect("log").clone()
    }

    fn wait_for(&self, needle: &str, timeout: Duration) -> String {
        let deadline = Instant::now() + timeout;
        loop {
            let text = self.snapshot();
            if text.contains(needle) {
                return text;
            }
            if Instant::now() > deadline {
                return text;
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    }

    fn wait_exit(&mut self, timeout: Duration) -> Option<std::process::ExitStatus> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.try_wait().expect("try_wait") {
                std::thread::sleep(Duration::from_millis(50));
                return Some(status);
            }
            if Instant::now() > deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                return None;
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    }
}

impl Drop for AmberProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn assert_contains(text: &str, needle: &str) {
    assert!(text.contains(needle), "missing {needle} in:\n{text}");
}

#[test]
#[serial]
fn run_watch_reexecutes_entry_and_publishes_reload() {
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(
        dir.path().join("preload.js"),
        "console.log('PRELOAD_OK');\n",
    )
    .expect("preload");
    std::fs::write(dir.path().join("app.js"), "console.log('RUN_ALPHA');\n").expect("app");

    let proc = AmberProc::spawn(
        &[
            "run",
            "--watch",
            "--debounce",
            "50",
            "--websocket-port",
            "0",
            "--require",
            "preload.js",
            "app.js",
        ],
        dir.path(),
    );
    let started = proc.wait_for("RUN_ALPHA", Duration::from_secs(90));
    assert_contains(&started, "PRELOAD_OK");
    assert_contains(&started, "Watch mode enabled");
    assert_contains(&started, "WebSocket server ready on ws://");
    let marker = "WebSocket server ready on ws://";
    let addr = started
        .split(marker)
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or("")
        .trim()
        .to_string();
    assert!(!addr.is_empty(), "missing bound address in:\n{started}");

    let mut socket = connect_ws(&format!("ws://{addr}"));
    arm_read_timeout(&socket);
    let hello = read_ws_text(&mut socket);
    assert!(hello.contains("\"event_type\":\"status\""), "{hello}");

    std::fs::write(dir.path().join("app.js"), "console.log('RUN_BETA');\n").expect("rewrite");
    // RUN_BETA can land before the reload line is flushed. Wait for the line.
    let reloaded = proc.wait_for("Reloaded in", Duration::from_secs(20));
    assert_contains(&reloaded, "RUN_BETA");
    assert_contains(&reloaded, "PRELOAD_OK");
    assert!(
        reloaded.matches("PRELOAD_OK").count() >= 2,
        "preload should run again:\n{reloaded}"
    );
    assert_contains(&reloaded, "Reloaded in");

    let mut reload = String::new();
    for _ in 0..6 {
        let frame = read_ws_text(&mut socket);
        if frame.contains("\"event_type\":\"reload\"") {
            reload = frame;
            break;
        }
    }
    assert!(
        reload.contains("\"event_type\":\"reload\""),
        "frames lacked reload"
    );
    assert!(
        reload.contains("\"change_type\":\"modified\"")
            || reload.contains("\"change_type\":\"created\""),
        "{reload}"
    );
    assert!(reload.contains("app.js"), "{reload}");
}

fn connect_ws(url: &str) -> WebSocket<MaybeTlsStream<TcpStream>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut last = String::new();
    while Instant::now() < deadline {
        match tungstenite::connect(url) {
            Ok((socket, _)) => return socket,
            Err(error) => {
                last = error.to_string();
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    panic!("failed to connect to {url}: {last}");
}

fn arm_read_timeout(socket: &WebSocket<MaybeTlsStream<TcpStream>>) {
    if let MaybeTlsStream::Plain(tcp) = socket.get_ref() {
        tcp.set_read_timeout(Some(Duration::from_secs(15)))
            .expect("read timeout");
    }
}

fn read_ws_text(socket: &mut WebSocket<MaybeTlsStream<TcpStream>>) -> String {
    match socket.read() {
        Ok(Message::Text(text)) => text,
        Ok(other) => panic!("expected text, got {other:?}"),
        Err(error) => panic!("websocket read failed: {error}"),
    }
}

#[test]
#[serial]
fn run_watch_startup_failures_do_not_execute_or_claim_ready() {
    let dir = TempDir::new().expect("tempdir");
    let missing = dir.path().join("missing.js");
    let mut missing_proc = AmberProc::spawn(
        &["run", "--watch", missing.to_str().expect("utf8")],
        dir.path(),
    );
    let status = missing_proc
        .wait_exit(Duration::from_secs(30))
        .expect("missing entry should exit");
    let missing_log = missing_proc.snapshot();
    assert!(!status.success(), "{missing_log}");
    assert_contains(&missing_log, "error: amber watch:");
    assert_contains(&missing_log, "entry must be a file:");
    assert!(!missing_log.contains("Watch mode enabled"), "{missing_log}");

    let as_dir = dir.path().join("subdir");
    std::fs::create_dir(&as_dir).expect("dir");
    let mut dir_proc = AmberProc::spawn(
        &["run", "--watch", as_dir.to_str().expect("utf8")],
        dir.path(),
    );
    let status = dir_proc
        .wait_exit(Duration::from_secs(30))
        .expect("directory entry should exit");
    let dir_log = dir_proc.snapshot();
    assert!(!status.success(), "{dir_log}");
    assert_contains(&dir_log, "entry must be a file:");

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("hold port");
    let port = listener.local_addr().expect("addr").port();
    std::fs::write(dir.path().join("app.js"), "console.log('BIND_FAIL_RAN');\n").expect("app");
    let mut bind_proc = AmberProc::spawn(
        &[
            "run",
            "--watch",
            "--websocket-port",
            &port.to_string(),
            "app.js",
        ],
        dir.path(),
    );
    let status = bind_proc
        .wait_exit(Duration::from_secs(30))
        .expect("occupied port should exit");
    let bind_log = bind_proc.snapshot();
    assert!(!status.success(), "{bind_log}");
    assert_contains(&bind_log, "error: amber watch:");
    assert_contains(&bind_log, "Failed to bind");
    assert!(!bind_log.contains("WebSocket server ready"), "{bind_log}");
    assert!(!bind_log.contains("BIND_FAIL_RAN"), "{bind_log}");
    drop(listener);

    std::fs::write(dir.path().join("net.js"), "console.log('DENY_NET_RAN');\n").expect("net");
    let mut net_proc = AmberProc::spawn(&["run", "--deny-net", "--watch", "net.js"], dir.path());
    let status = net_proc
        .wait_exit(Duration::from_secs(30))
        .expect("denied listen should exit");
    let net_log = net_proc.snapshot();
    assert!(!status.success(), "{net_log}");
    assert_contains(&net_log, "error: amber watch:");
    assert!(
        net_log.contains("permission denied") && net_log.contains("Listen"),
        "{net_log}"
    );
    assert!(!net_log.contains("WebSocket server ready"), "{net_log}");
    assert!(!net_log.contains("DENY_NET_RAN"), "{net_log}");
    assert!(!net_log.contains("Watch mode enabled"), "{net_log}");
}

#[test]
#[serial]
fn package_script_name_runs_once_without_watch() {
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"hello":"echo PACKAGE_SCRIPT_ONCE"}}"#,
    )
    .expect("package.json");
    let mut proc = AmberProc::spawn(&["run", "--watch", "hello"], dir.path());
    let status = proc
        .wait_exit(Duration::from_secs(30))
        .expect("package script should exit");
    let log = proc.snapshot();
    assert!(status.success(), "{log}");
    assert_contains(&log, "PACKAGE_SCRIPT_ONCE");
    assert!(!log.contains("Watch mode enabled"), "{log}");
}

#[test]
#[serial]
fn test_watch_reruns_file_and_directory_without_regressing_plain_test() {
    let dir = TempDir::new().expect("tempdir");
    let passing = dir.path().join("ok.test.js");
    std::fs::write(&passing, "test('ok', () => {});\n").expect("ok");
    let mut pass = AmberProc::spawn(&["test", passing.to_str().expect("utf8")], dir.path());
    let status = pass
        .wait_exit(Duration::from_secs(90))
        .expect("passing test should exit");
    let pass_log = pass.snapshot();
    assert!(status.success(), "{pass_log}");
    assert!(!pass_log.contains("Watching for changes"), "{pass_log}");

    let failing = dir.path().join("bad.test.js");
    std::fs::write(
        &failing,
        "test('nope', () => { throw new Error('boom'); });\n",
    )
    .expect("bad");
    let mut fail = AmberProc::spawn(&["test", failing.to_str().expect("utf8")], dir.path());
    let status = fail
        .wait_exit(Duration::from_secs(90))
        .expect("failing test should exit");
    let fail_log = fail.snapshot();
    assert_eq!(status.code(), Some(1), "{fail_log}");

    let watched = dir.path().join("watch.test.js");
    std::fs::write(
        &watched,
        "test('watch', () => { console.log('TEST_WATCH_ALPHA'); });\n",
    )
    .expect("watch test");
    let proc = AmberProc::spawn(
        &["test", "--watch", watched.to_str().expect("utf8")],
        dir.path(),
    );
    proc.wait_for("TEST_WATCH_ALPHA", Duration::from_secs(90));
    // The test log can land before the banner is flushed. Wait for the line.
    let started = proc.wait_for("Watching for changes", Duration::from_secs(20));
    assert_contains(&started, "TEST_WATCH_ALPHA");
    assert_contains(&started, "Watching for changes");
    std::fs::write(
        &watched,
        "test('watch', () => { console.log('TEST_WATCH_BETA'); });\n",
    )
    .expect("rewrite test");
    let rerun = proc.wait_for("TEST_WATCH_BETA", Duration::from_secs(20));
    assert_contains(&rerun, "Re-running test");

    let suite = dir.path().join("suite");
    std::fs::create_dir(&suite).expect("suite");
    let one = suite.join("one.test.js");
    std::fs::write(
        &one,
        "test('dir', () => { console.log('DIR_WATCH_ALPHA'); });\n",
    )
    .expect("one");
    let dir_proc = AmberProc::spawn(
        &["test", "--watch", suite.to_str().expect("utf8")],
        dir.path(),
    );
    dir_proc.wait_for("DIR_WATCH_ALPHA", Duration::from_secs(90));
    // The test log can land before the banner is flushed. Wait for the line.
    let started = dir_proc.wait_for("Watching for changes", Duration::from_secs(20));
    assert_contains(&started, "DIR_WATCH_ALPHA");
    assert_contains(&started, "Watching for changes");
    std::fs::write(
        &one,
        "test('dir', () => { console.log('DIR_WATCH_BETA'); });\n",
    )
    .expect("rewrite suite");
    let rerun = dir_proc.wait_for("DIR_WATCH_BETA", Duration::from_secs(20));
    assert_contains(&rerun, "Re-running tests");
}

#[test]
#[serial]
fn test_parallel_still_exits_two_with_watch() {
    let dir = TempDir::new().expect("tempdir");
    let mut plain = AmberProc::spawn(&["test", "--parallel"], dir.path());
    let status = plain
        .wait_exit(Duration::from_secs(30))
        .expect("--parallel should exit");
    let plain_log = plain.snapshot();
    assert_eq!(status.code(), Some(2), "{plain_log}");
    assert!(plain_log.contains("not supported"), "{plain_log}");

    let mut both = AmberProc::spawn(&["test", "--parallel", "--watch"], dir.path());
    let status = both
        .wait_exit(Duration::from_secs(30))
        .expect("--parallel --watch should exit");
    let both_log = both.snapshot();
    assert_eq!(status.code(), Some(2), "{both_log}");
    assert!(both_log.contains("not supported"), "{both_log}");
    assert!(!both_log.contains("Watching for changes"), "{both_log}");
}

// Pins the Stable plain `amber serve` contract in docs/SERVE_HTTP_CONTRACT.md.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use amberjs::https_serve::{
    MAX_BODY_BYTES, MAX_HEADER_BYTES, MAX_HEADER_COUNT, READ_TIMEOUT, SERVE_ERROR_PREFIX,
};

fn amber() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

fn combined(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn assert_exit_2(output: &std::process::Output, needles: &[&str]) {
    let text = combined(output);
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit 2. output: {text}"
    );
    assert!(
        text.contains(SERVE_ERROR_PREFIX),
        "missing {SERVE_ERROR_PREFIX}. output: {text}"
    );
    assert!(
        !text.contains("Listening on http://") && !text.contains("Starting Amber Web Server"),
        "failure must not look like a running server: {text}"
    );
    for needle in needles {
        assert!(text.contains(needle), "missing {needle:?} in {text}");
    }
}

struct Server {
    child: std::process::Child,
    port: u16,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_http(dir: &Path, extra: &[&str]) -> Server {
    let mut child = Command::new(amber())
        .current_dir(dir)
        .args(extra)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn amber serve");
    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");
    let stderr_thread = thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        let mut all = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => all.push_str(&line),
                Err(_) => break,
            }
        }
        all
    });
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let start = Instant::now();
    let mut port = None;
    while start.elapsed() < Duration::from_secs(20) {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if let Some(found) = line
                    .split("http://")
                    .nth(1)
                    .and_then(|rest| rest.split_whitespace().next())
                    .and_then(|hostport| hostport.rsplit(':').next())
                    .and_then(|raw| raw.trim_end_matches(')').parse::<u16>().ok())
                {
                    // Prefer the Listening line (bound address); skip banners that
                    // still show the requested port before bind when present.
                    if line.contains("Listening on http://") {
                        port = Some(found);
                        break;
                    }
                    port = Some(found);
                }
            }
            Err(_) => break,
        }
    }
    thread::spawn(move || {
        let mut rest = String::new();
        let _ = reader.read_to_string(&mut rest);
    });
    let port = match port {
        Some(port) if port != 0 => port,
        _ => {
            let _ = child.kill();
            let err = stderr_thread.join().unwrap_or_default();
            panic!("amber serve never listened on a bound port. stderr: {err}");
        }
    };
    thread::spawn(move || {
        let _ = stderr_thread.join();
    });
    Server { child, port }
}

fn curl(port: u16, path: &str, args: &[&str]) -> (String, String) {
    let dir = tempfile::tempdir().unwrap();
    let headers = dir.path().join("headers");
    let body = dir.path().join("body");
    let url = format!("http://127.0.0.1:{port}{path}");
    let mut cmd = Command::new("curl");
    cmd.args([
        "-s",
        "--http1.1",
        "--max-time",
        "5",
        "-D",
        headers.to_str().unwrap(),
        "-o",
        body.to_str().unwrap(),
    ]);
    cmd.args(args);
    cmd.arg(&url);
    let output = cmd.output().expect("curl");
    let header_text = fs::read_to_string(&headers).unwrap_or_default();
    let body_text = fs::read_to_string(&body).unwrap_or_default();
    assert!(
        output.status.success(),
        "curl failed status={:?} stdout={} stderr={} headers={header_text} body={body_text}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (header_text, body_text)
}

fn raw_http(port: u16, request: &[u8]) -> String {
    let mut tcp = TcpStream::connect(("127.0.0.1", port)).expect("connect to amber serve");
    tcp.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    tcp.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
    tcp.write_all(request).expect("write request");
    let _ = tcp.shutdown(std::net::Shutdown::Write);
    let mut buf = Vec::new();
    let _ = tcp.read_to_end(&mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}

fn header_value(headers: &str, name: &str) -> String {
    headers
        .lines()
        .find(|line| line.to_ascii_lowercase().starts_with(&format!("{name}:")))
        .and_then(|line| line.split_once(':'))
        .map(|(_, value)| value.trim().to_string())
        .unwrap_or_default()
}

fn crate_version() -> String {
    let version = Command::new(amber()).arg("version").output().unwrap();
    let version_line = String::from_utf8_lossy(&version.stdout);
    version_line
        .lines()
        .next()
        .unwrap_or("")
        .trim_start_matches("Amber ")
        .trim()
        .to_string()
}

#[test]
fn documented_limits_match_the_contract() {
    assert_eq!(MAX_HEADER_BYTES, 16 * 1024);
    assert_eq!(MAX_BODY_BYTES, 1024 * 1024);
    assert_eq!(MAX_HEADER_COUNT, 64);
    assert_eq!(READ_TIMEOUT, Duration::from_secs(10));
    assert_eq!(SERVE_ERROR_PREFIX, "error: amber serve:");
}

#[test]
fn deny_net_and_script_errors_do_not_listen() {
    let denied = Command::new(amber())
        .args(["serve", "--deny-net", "--port", "9"])
        .output()
        .unwrap();
    let text = combined(&denied);
    assert!(!denied.status.success(), "{text}");
    assert!(text.contains("permission denied"), "{text}");
    assert!(text.contains("Listen"), "{text}");
    assert!(!text.contains("Listening on http://"), "{text}");

    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("app.js");
    fs::write(&script, "throw new Error(\"load-fail\");\n").unwrap();
    let failed = Command::new(amber())
        .current_dir(dir.path())
        .args([
            "serve",
            "--host",
            "127.0.0.1",
            "--port",
            "0",
            script.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_exit_2(&failed, &["failed to load script"]);
}

#[test]
fn bind_failure_exits_2_before_listen() {
    let holder = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = holder.local_addr().unwrap().port();
    let failed = Command::new(amber())
        .args(["serve", "--host", "127.0.0.1", "--port", &port.to_string()])
        .output()
        .unwrap();
    assert_exit_2(&failed, &["failed to bind"]);
    drop(holder);
}

#[test]
fn health_server_enforces_http11_limits() {
    let dir = tempfile::tempdir().unwrap();
    // Empty cwd so auto-discovery does not pick up a repo script.
    let server = spawn_http(dir.path(), &["serve", "--host", "127.0.0.1", "--port", "0"]);
    let version = crate_version();

    let (headers, body) = curl(server.port, "/", &[]);
    assert!(headers.starts_with("HTTP/1.1 200 OK"), "{headers}");
    assert!(
        headers.to_ascii_lowercase().contains("connection: close"),
        "{headers}"
    );
    assert!(
        headers.to_ascii_lowercase().contains("application/json"),
        "{headers}"
    );
    let json: serde_json::Value = serde_json::from_str(body.trim()).expect(&body);
    assert_eq!(json["runtime"], "amberjs");
    assert_eq!(json["ok"], true);
    assert_eq!(json["version"], version);
    assert!(body.ends_with('\n'), "{body:?}");

    let head = raw_http(server.port, b"HEAD / HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert!(head.contains("HTTP/1.1 200 OK"), "{head}");
    let length = header_value(&head, "content-length");
    assert_eq!(length.parse::<usize>().unwrap(), body.len(), "{head}");
    assert!(
        !head.contains("\"ok\":true") && !head.contains("\"runtime\":\"amberjs\""),
        "HEAD must not include a body: {head}"
    );

    let http10 = raw_http(server.port, b"GET / HTTP/1.0\r\nHost: localhost\r\n\r\n");
    assert!(
        http10.contains("505") && http10.contains("HTTP/1.1 required"),
        "{http10}"
    );

    let no_host = raw_http(server.port, b"GET / HTTP/1.1\r\n\r\n");
    assert!(
        no_host.contains("400") && no_host.contains("host header required"),
        "{no_host}"
    );

    let chunked = raw_http(
        server.port,
        b"POST / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
    );
    assert!(
        chunked.contains("501") && chunked.contains("transfer-encoding is not supported"),
        "{chunked}"
    );

    let expect = raw_http(
        server.port,
        b"GET / HTTP/1.1\r\nHost: localhost\r\nExpect: 100-continue\r\n\r\n",
    );
    assert!(
        expect.contains("417") && expect.contains("expectation failed"),
        "{expect}"
    );

    let mut huge = b"GET / HTTP/1.1\r\nHost: localhost\r\nX-Big: ".to_vec();
    huge.extend(std::iter::repeat_n(b'a', MAX_HEADER_BYTES));
    huge.extend_from_slice(b"\r\n\r\n");
    let too_many_headers = raw_http(server.port, &huge);
    assert!(
        too_many_headers.contains("431") && too_many_headers.contains("request headers too large"),
        "{too_many_headers}"
    );

    let too_big = format!(
        "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
        MAX_BODY_BYTES + 1
    );
    let payload = raw_http(server.port, too_big.as_bytes());
    assert!(
        payload.contains("413") && payload.contains("request body too large"),
        "{payload}"
    );

    // Still serving after rejected framing.
    let (again, _) = curl(server.port, "/still-up", &[]);
    assert!(again.starts_with("HTTP/1.1 200 OK"), "{again}");
}

#[test]
fn fetch_handler_round_trip_status_and_body() {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("handler.js");
    fs::write(
        &script,
        r#"
        module.exports = {
          async fetch(req) {
            const url = String(req.url || "");
            if (url.indexOf("/status") !== -1) {
              return new Response("created", { status: 201, headers: { "x-amber": "serve" } });
            }
            if (url.indexOf("/echo") !== -1) {
              const text = await req.text();
              return new Response(text);
            }
            if (url.indexOf("/boom") !== -1) {
              throw new Error("boom");
            }
            return new Response("ok");
          }
        };
        "#,
    )
    .unwrap();
    let server = spawn_http(
        dir.path(),
        &[
            "serve",
            "--host",
            "127.0.0.1",
            "--port",
            "0",
            script.to_str().unwrap(),
        ],
    );

    let (ok_headers, ok_body) = curl(server.port, "/", &[]);
    assert!(ok_headers.starts_with("HTTP/1.1 200 "), "{ok_headers}");
    assert_eq!(ok_body, "ok");

    let (status_headers, status_body) = curl(server.port, "/status", &[]);
    assert!(
        status_headers.starts_with("HTTP/1.1 201 Created"),
        "{status_headers}"
    );
    assert!(
        status_headers
            .to_ascii_lowercase()
            .contains("x-amber: serve"),
        "{status_headers}"
    );
    assert_eq!(status_body, "created");

    let (echo_headers, echo_body) = curl(
        server.port,
        "/echo",
        &["--data-binary", "hello", "-H", "Content-Type: text/plain"],
    );
    assert!(echo_headers.starts_with("HTTP/1.1 200 "), "{echo_headers}");
    assert_eq!(echo_body, "hello");

    let (boom_headers, boom_body) = curl(server.port, "/boom", &[]);
    assert!(
        boom_headers.starts_with("HTTP/1.1 500 Internal Server Error"),
        "{boom_headers}"
    );
    assert!(boom_body.contains("boom"), "{boom_body}");
}

#[test]
fn script_without_fetch_export_is_404() {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("empty.js");
    fs::write(&script, "module.exports = { name: 'no-handler' };\n").unwrap();
    let server = spawn_http(
        dir.path(),
        &[
            "serve",
            "--host",
            "127.0.0.1",
            "--port",
            "0",
            script.to_str().unwrap(),
        ],
    );
    let (headers, body) = curl(server.port, "/", &[]);
    assert!(headers.starts_with("HTTP/1.1 404 Not Found"), "{headers}");
    assert!(
        body.contains("Not Found: No fetch handler exported"),
        "{body}"
    );
}

#[test]
fn local_function_fetch_is_the_handler_not_global_client() {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("fn.js");
    fs::write(
        &script,
        "function fetch() { return new Response('local'); }\n",
    )
    .unwrap();
    let server = spawn_http(
        dir.path(),
        &[
            "serve",
            "--host",
            "127.0.0.1",
            "--port",
            "0",
            script.to_str().unwrap(),
        ],
    );
    let (headers, body) = curl(server.port, "/", &[]);
    assert!(headers.starts_with("HTTP/1.1 200 "), "{headers}");
    assert_eq!(body, "local");

    // Empty exports must not bind the runtime global fetch client as handler.
    let empty = dir.path().join("empty-exports.js");
    fs::write(&empty, "module.exports = {};\n").unwrap();
    let server2 = spawn_http(
        dir.path(),
        &[
            "serve",
            "--host",
            "127.0.0.1",
            "--port",
            "0",
            empty.to_str().unwrap(),
        ],
    );
    let (h2, b2) = curl(server2.port, "/", &[]);
    assert!(h2.starts_with("HTTP/1.1 404 Not Found"), "{h2}");
    assert!(b2.contains("Not Found: No fetch handler exported"), "{b2}");
}

#[test]
fn port_zero_prints_bound_address() {
    let dir = tempfile::tempdir().unwrap();
    let server = spawn_http(dir.path(), &["serve", "--host", "127.0.0.1", "--port", "0"]);
    assert_ne!(server.port, 0);
    let (headers, _) = curl(server.port, "/", &[]);
    assert!(headers.starts_with("HTTP/1.1 200 OK"), "{headers}");
}

#[test]
fn clap_cert_without_https_is_not_serve_prefix() {
    let clap = Command::new(amber())
        .args(["serve", "--cert", "cert.pem"])
        .output()
        .unwrap();
    let clap_text = combined(&clap);
    assert!(!clap.status.success(), "{clap_text}");
    assert!(!clap_text.contains("Listening on http://"), "{clap_text}");
    assert!(
        !clap_text.contains(SERVE_ERROR_PREFIX),
        "clap usage must not use the serve prefix: {clap_text}"
    );
}

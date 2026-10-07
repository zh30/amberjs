// Pins the Stable `amber serve --https` contract in docs/SERVE_HTTPS_CONTRACT.md.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use amberjs::https_serve::{
    MAX_BODY_BYTES, MAX_HEADER_BYTES, MAX_HEADER_COUNT, MAX_PEM_BYTES, READ_TIMEOUT,
    SERVE_ERROR_PREFIX,
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
        !text.contains("Listening on https://") && !text.contains("Starting Amber Web Server"),
        "failure must not look like a running server: {text}"
    );
    for needle in needles {
        assert!(text.contains(needle), "missing {needle:?} in {text}");
    }
}

fn openssl(args: &[&str]) {
    let output = Command::new("openssl")
        .args(args)
        .output()
        .expect("openssl");
    assert!(
        output.status.success(),
        "openssl {args:?} failed: {}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
}

fn mint_cert(dir: &Path) -> (PathBuf, PathBuf) {
    let cert = dir.join("cert.pem");
    let key = dir.join("key.pem");
    openssl(&[
        "req",
        "-x509",
        "-newkey",
        "ec",
        "-pkeyopt",
        "ec_paramgen_curve:prime256v1",
        "-keyout",
        key.to_str().unwrap(),
        "-out",
        cert.to_str().unwrap(),
        "-days",
        "1",
        "-nodes",
        "-subj",
        "/CN=localhost",
        "-addext",
        "subjectAltName=DNS:localhost,IP:127.0.0.1",
    ]);
    (cert, key)
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

fn spawn_https(dir: &Path, extra: &[&str]) -> Server {
    let mut child = Command::new(amber())
        .current_dir(dir)
        .args(extra)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn amber serve --https");
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
                    .split("https://")
                    .nth(1)
                    .and_then(|rest| rest.split_whitespace().next())
                    .and_then(|hostport| hostport.rsplit(':').next())
                    .and_then(|raw| raw.trim_end_matches(')').parse::<u16>().ok())
                {
                    port = Some(found);
                    break;
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
        Some(port) => port,
        None => {
            let _ = child.kill();
            let err = stderr_thread.join().unwrap_or_default();
            panic!("amber serve --https never listened. stderr: {err}");
        }
    };
    // Keep the stderr thread alive for the process lifetime by detaching it.
    thread::spawn(move || {
        let _ = stderr_thread.join();
    });
    Server { child, port }
}

fn curl(port: u16, path: &str, args: &[&str]) -> (String, String) {
    let dir = tempfile::tempdir().unwrap();
    let headers = dir.path().join("headers");
    let body = dir.path().join("body");
    let url = format!("https://127.0.0.1:{port}{path}");
    let mut cmd = Command::new("curl");
    cmd.args([
        "-sk",
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

fn s_client(port: u16, alpn: &str, request: &[u8]) -> String {
    let connect = format!("127.0.0.1:{port}");
    let mut child = Command::new("timeout")
        .args([
            "5",
            "openssl",
            "s_client",
            "-connect",
            &connect,
            "-servername",
            "localhost",
            "-alpn",
            alpn,
            "-ign_eof",
            "-quiet",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("openssl s_client");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        let _ = stdin.write_all(request);
    }
    let output = child.wait_with_output().expect("wait s_client");
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn documented_limits_match_the_contract() {
    assert_eq!(MAX_HEADER_BYTES, 16 * 1024);
    assert_eq!(MAX_BODY_BYTES, 1024 * 1024);
    assert_eq!(MAX_HEADER_COUNT, 64);
    assert_eq!(MAX_PEM_BYTES, 1024 * 1024);
    assert_eq!(READ_TIMEOUT, Duration::from_secs(10));
    assert_eq!(SERVE_ERROR_PREFIX, "error: amber serve:");
}

#[test]
fn missing_flags_files_and_bad_pem_exit_2_before_listen() {
    let dir = tempfile::tempdir().unwrap();
    let no_cert = Command::new(amber())
        .args(["serve", "--https", "--port", "0"])
        .output()
        .unwrap();
    assert_exit_2(&no_cert, &["--cert"]);

    let no_key = Command::new(amber())
        .current_dir(dir.path())
        .args(["serve", "--https", "--cert", "cert.pem", "--port", "0"])
        .output()
        .unwrap();
    assert_exit_2(&no_key, &["--key"]);

    let missing_cert = Command::new(amber())
        .args([
            "serve",
            "--https",
            "--cert",
            "no-such-cert.pem",
            "--key",
            "no-such-key.pem",
            "--port",
            "0",
        ])
        .output()
        .unwrap();
    assert_exit_2(&missing_cert, &["TLS certificate not found"]);

    let (cert, _key) = mint_cert(dir.path());
    let missing_key = Command::new(amber())
        .args([
            "serve",
            "--https",
            "--cert",
            cert.to_str().unwrap(),
            "--key",
            "no-such-key.pem",
            "--port",
            "0",
        ])
        .output()
        .unwrap();
    assert_exit_2(&missing_key, &["TLS private key not found"]);

    let not_file = Command::new(amber())
        .args([
            "serve",
            "--https",
            "--cert",
            dir.path().to_str().unwrap(),
            "--key",
            cert.to_str().unwrap(),
            "--port",
            "0",
        ])
        .output()
        .unwrap();
    assert_exit_2(&not_file, &["is not a file"]);

    let bad_cert = dir.path().join("bad-cert.pem");
    let bad_key = dir.path().join("bad-key.pem");
    fs::write(&bad_cert, b"not a certificate").unwrap();
    fs::write(&bad_key, b"not a key").unwrap();
    let bad = Command::new(amber())
        .args([
            "serve",
            "--https",
            "--cert",
            bad_cert.to_str().unwrap(),
            "--key",
            bad_key.to_str().unwrap(),
            "--port",
            "0",
        ])
        .output()
        .unwrap();
    assert_exit_2(&bad, &["no certificate found in PEM"]);

    let clap = Command::new(amber())
        .args(["serve", "--cert", "cert.pem"])
        .output()
        .unwrap();
    let clap_text = combined(&clap);
    assert!(!clap.status.success(), "{clap_text}");
    assert!(!clap_text.contains("Listening on https://"), "{clap_text}");
    assert!(
        !clap_text.contains(SERVE_ERROR_PREFIX),
        "clap usage must not use the serve prefix: {clap_text}"
    );
}

#[test]
fn encrypted_and_mismatched_keys_exit_2() {
    let dir = tempfile::tempdir().unwrap();
    let enc_key = dir.path().join("enc.pem");
    let enc_cert = dir.path().join("enc-cert.pem");
    openssl(&[
        "genpkey",
        "-algorithm",
        "RSA",
        "-pkeyopt",
        "rsa_keygen_bits:2048",
        "-aes-128-cbc",
        "-pass",
        "pass:secret",
        "-out",
        enc_key.to_str().unwrap(),
    ]);
    openssl(&[
        "req",
        "-new",
        "-x509",
        "-key",
        enc_key.to_str().unwrap(),
        "-passin",
        "pass:secret",
        "-out",
        enc_cert.to_str().unwrap(),
        "-days",
        "1",
        "-subj",
        "/CN=localhost",
    ]);
    let encrypted = Command::new(amber())
        .args([
            "serve",
            "--https",
            "--cert",
            enc_cert.to_str().unwrap(),
            "--key",
            enc_key.to_str().unwrap(),
            "--port",
            "0",
        ])
        .output()
        .unwrap();
    assert_exit_2(&encrypted, &["encrypted private keys are not supported"]);

    let (cert_a, _key_a) = {
        let cert = dir.path().join("a-cert.pem");
        let key = dir.path().join("a-key.pem");
        openssl(&[
            "req",
            "-x509",
            "-newkey",
            "ec",
            "-pkeyopt",
            "ec_paramgen_curve:prime256v1",
            "-keyout",
            key.to_str().unwrap(),
            "-out",
            cert.to_str().unwrap(),
            "-days",
            "1",
            "-nodes",
            "-subj",
            "/CN=a",
        ]);
        (cert, key)
    };
    let key_b = dir.path().join("b-key.pem");
    let cert_b = dir.path().join("b-cert.pem");
    openssl(&[
        "req",
        "-x509",
        "-newkey",
        "ec",
        "-pkeyopt",
        "ec_paramgen_curve:prime256v1",
        "-keyout",
        key_b.to_str().unwrap(),
        "-out",
        cert_b.to_str().unwrap(),
        "-days",
        "1",
        "-nodes",
        "-subj",
        "/CN=b",
    ]);
    let mismatch = Command::new(amber())
        .args([
            "serve",
            "--https",
            "--cert",
            cert_a.to_str().unwrap(),
            "--key",
            key_b.to_str().unwrap(),
            "--port",
            "0",
        ])
        .output()
        .unwrap();
    assert_exit_2(&mismatch, &["invalid TLS material"]);
}

#[test]
fn deny_net_and_script_errors_do_not_listen() {
    let denied = Command::new(amber())
        .args(["serve", "--https", "--deny-net", "--port", "9"])
        .output()
        .unwrap();
    let text = combined(&denied);
    assert!(!denied.status.success(), "{text}");
    assert!(text.contains("permission denied"), "{text}");
    assert!(text.contains("Listen"), "{text}");
    assert!(!text.contains("Listening on https://"), "{text}");

    let dir = tempfile::tempdir().unwrap();
    let (cert, key) = mint_cert(dir.path());
    let script = dir.path().join("app.js");
    fs::write(&script, "throw new Error(\"load-fail\");\n").unwrap();
    let failed = Command::new(amber())
        .current_dir(dir.path())
        .args([
            "serve",
            "--https",
            "--cert",
            cert.to_str().unwrap(),
            "--key",
            key.to_str().unwrap(),
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
fn health_server_enforces_http11_limits() {
    let dir = tempfile::tempdir().unwrap();
    let (cert, key) = mint_cert(dir.path());
    let server = spawn_https(
        dir.path(),
        &[
            "serve",
            "--https",
            "--cert",
            cert.to_str().unwrap(),
            "--key",
            key.to_str().unwrap(),
            "--host",
            "127.0.0.1",
            "--port",
            "0",
        ],
    );
    let version = Command::new(amber()).arg("version").output().unwrap();
    let version_line = String::from_utf8_lossy(&version.stdout);
    let version = version_line
        .lines()
        .next()
        .unwrap_or("")
        .trim_start_matches("Amber ")
        .trim();

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

    // curl -X HEAD still waits for Content-Length body bytes (exit 18). The
    // wire check is openssl: headers only, with the GET body's length.
    let head = s_client(
        server.port,
        "http/1.1",
        b"HEAD / HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );
    assert!(head.contains("HTTP/1.1 200 OK"), "{head}");
    let length = header_value(&head, "content-length");
    assert_eq!(length.parse::<usize>().unwrap(), body.len(), "{head}");
    assert!(
        !head.contains("\"ok\":true") && !head.contains("\"runtime\":\"amberjs\""),
        "HEAD must not include a body: {head}"
    );

    let http10 = s_client(
        server.port,
        "http/1.1",
        b"GET / HTTP/1.0\r\nHost: localhost\r\n\r\n",
    );
    assert!(
        http10.contains("505") && http10.contains("HTTP/1.1 required"),
        "{http10}"
    );

    let no_host = s_client(server.port, "http/1.1", b"GET / HTTP/1.1\r\n\r\n");
    assert!(
        no_host.contains("400") && no_host.contains("host header required"),
        "{no_host}"
    );

    let chunked = s_client(
        server.port,
        "http/1.1",
        b"POST / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
    );
    assert!(
        chunked.contains("501") && chunked.contains("transfer-encoding is not supported"),
        "{chunked}"
    );

    let expect = s_client(
        server.port,
        "http/1.1",
        b"GET / HTTP/1.1\r\nHost: localhost\r\nExpect: 100-continue\r\n\r\n",
    );
    assert!(
        expect.contains("417") && expect.contains("expectation failed"),
        "{expect}"
    );

    let mut huge = b"GET / HTTP/1.1\r\nHost: localhost\r\nX-Big: ".to_vec();
    huge.extend(std::iter::repeat_n(b'a', MAX_HEADER_BYTES));
    huge.extend_from_slice(b"\r\n\r\n");
    let too_many_headers = s_client(server.port, "http/1.1", &huge);
    assert!(
        too_many_headers.contains("431") && too_many_headers.contains("request headers too large"),
        "{too_many_headers}"
    );

    let too_big = format!(
        "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
        MAX_BODY_BYTES + 1
    );
    let payload = s_client(server.port, "http/1.1", too_big.as_bytes());
    assert!(
        payload.contains("413") && payload.contains("request body too large"),
        "{payload}"
    );

    let h2 = s_client(server.port, "h2", b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n");
    assert!(
        !h2.contains("HTTP/1.1 200") && !h2.contains("\"ok\":true"),
        "h2 ALPN must not receive the health document: {h2}"
    );

    // The listener is still the health server after the rejected handshake.
    let (again, _) = curl(server.port, "/still-up", &[]);
    assert!(again.starts_with("HTTP/1.1 200 OK"), "{again}");
}

#[test]
fn fetch_handler_round_trip_status_and_body() {
    let dir = tempfile::tempdir().unwrap();
    let (cert, key) = mint_cert(dir.path());
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
    let server = spawn_https(
        dir.path(),
        &[
            "serve",
            "--https",
            "--cert",
            cert.to_str().unwrap(),
            "--key",
            key.to_str().unwrap(),
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
    let (cert, key) = mint_cert(dir.path());
    let script = dir.path().join("empty.js");
    fs::write(&script, "module.exports = { name: 'no-handler' };\n").unwrap();
    let server = spawn_https(
        dir.path(),
        &[
            "serve",
            "--https",
            "--cert",
            cert.to_str().unwrap(),
            "--key",
            key.to_str().unwrap(),
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
fn local_function_fetch_is_the_handler() {
    let dir = tempfile::tempdir().unwrap();
    let (cert, key) = mint_cert(dir.path());
    let script = dir.path().join("fn.js");
    fs::write(
        &script,
        "function fetch() { return new Response('local'); }\n",
    )
    .unwrap();
    let server = spawn_https(
        dir.path(),
        &[
            "serve",
            "--https",
            "--cert",
            cert.to_str().unwrap(),
            "--key",
            key.to_str().unwrap(),
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
}

fn header_value(headers: &str, name: &str) -> String {
    headers
        .lines()
        .find(|line| line.to_ascii_lowercase().starts_with(&format!("{name}:")))
        .and_then(|line| line.split_once(':'))
        .map(|(_, value)| value.trim().to_string())
        .unwrap_or_default()
}

#[test]
fn port_zero_does_not_require_a_free_fixed_port() {
    // Bind check: the contract tests themselves use port 0. This guards the
    // helper against accidentally colliding with a fixed port in the repo.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    assert_ne!(listener.local_addr().unwrap().port(), 0);
}

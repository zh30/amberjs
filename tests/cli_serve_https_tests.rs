use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::tempdir;

fn bee() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

fn generate_tls_pair(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let cert = dir.join("cert.pem");
    let key = dir.join("key.pem");
    let status = Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-keyout",
            key.to_str().unwrap(),
            "-out",
            cert.to_str().unwrap(),
            "-days",
            "1",
            "-nodes",
            "-subj",
            "/CN=localhost",
        ])
        .status()
        .expect("openssl");
    assert!(
        status.success(),
        "openssl failed to mint a test certificate"
    );
    (cert, key)
}

#[test]
fn serve_https_without_cert_exits_nonzero() {
    let output = Command::new(bee())
        .args(["serve", "--https", "--port", "0"])
        .output()
        .expect("bee serve --https");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(
        !output.status.success(),
        "bee serve --https without cert must fail: {combined}"
    );
    assert_ne!(output.status.code(), Some(0));
    assert!(
        !combined.contains("Starting Beejs Web Server"),
        "must not print a successful-server banner: {combined}"
    );
    assert!(
        !combined.contains("TLS terminator integration"),
        "must not use the old success-path stub: {combined}"
    );
    assert!(
        combined.contains("--cert") || combined.to_lowercase().contains("certificate"),
        "error should mention --cert: {combined}"
    );
}

#[test]
fn serve_https_with_pem_serves_fetch_handler() {
    let dir = tempdir().unwrap();
    let (cert, key) = generate_tls_pair(dir.path());
    let app = dir.path().join("app.js");
    std::fs::write(
        &app,
        r#"module.exports = { fetch() { return new Response("ok"); } };"#,
    )
    .unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let mut child = Command::new(bee())
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
            &port.to_string(),
            app.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn bee serve");

    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let start = Instant::now();
    let mut listening = false;
    while start.elapsed() < Duration::from_secs(15) {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if line.contains("Listening on https://") {
                    listening = true;
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if !listening {
        let _ = child.kill();
        panic!("bee serve --https never printed Listening banner");
    }

    let curl = Command::new("curl")
        .args([
            "-sk",
            "--http1.1",
            "--max-time",
            "5",
            &format!("https://127.0.0.1:{port}/"),
        ])
        .output()
        .expect("curl");
    let body = String::from_utf8_lossy(&curl.stdout);
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        curl.status.success(),
        "curl failed: {} {}",
        String::from_utf8_lossy(&curl.stderr),
        body
    );
    assert_eq!(body.trim(), "ok", "unexpected HTTPS body: {body}");
}

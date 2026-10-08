// Pins the Stable fetch contract in docs/FETCH_CONTRACT.md.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use amberjs::runtime_minimal::MinimalRuntime;
use base64::Engine as _;
use sha2::Digest as _;

fn run_js(code: &str) -> Result<String, String> {
    let mut runtime = MinimalRuntime::new().map_err(|error| error.to_string())?;
    runtime
        .execute_code(code)
        .map(|value| value.trim().to_string())
        .map_err(|error| error.to_string())
}

fn spawn_one(raw: Vec<u8>, hits: Arc<AtomicUsize>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = listener.local_addr().expect("addr");
    thread::spawn(move || {
        listener.set_nonblocking(true).expect("nonblocking");
        let started = Instant::now();
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    hits.fetch_add(1, Ordering::SeqCst);
                    let _ = stream.set_nodelay(true);
                    let mut buffer = [0u8; 2048];
                    let _ = stream.read(&mut buffer);
                    let _ = stream.write_all(&raw);
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if started.elapsed() > Duration::from_secs(2) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(15));
                }
                Err(_) => break,
            }
        }
    });
    format!("http://{address}")
}

fn sha256_sri(bytes: &[u8]) -> String {
    let digest = sha2::Sha256::digest(bytes);
    format!(
        "sha256-{}",
        base64::engine::general_purpose::STANDARD.encode(digest)
    )
}

fn decode_reader_script(url: &str, init: &str) -> String {
    format!(
        r#"
        const response = fetch({url:?}, {init});
        const reader = response.body.getReader();
        const first = reader.read();
        let text = '';
        const bytes = first.value;
        if (bytes && typeof bytes.length === 'number') {{
            for (let i = 0; i < bytes.length; i++) text += String.fromCharCode(bytes[i]);
        }}
        text + ':' + first.done + ':' + (typeof response.body.getReader);
        "#
    )
}

#[test]
#[serial_test::serial]
fn fetch_streams_body_before_content_length_is_finished() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = listener.local_addr().expect("addr");
    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let _ = stream.set_nodelay(true);
        let mut buffer = [0u8; 1024];
        let _ = stream.read(&mut buffer);
        let head = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 6\r\nConnection: close\r\n\r\nAAA";
        let _ = stream.write_all(head);
        let _ = stream.flush();
        thread::sleep(Duration::from_millis(2000));
        let _ = stream.write_all(b"BBB");
    });
    let url = format!("http://{address}");
    let started = Instant::now();
    let output = run_js(&decode_reader_script(&url, "undefined")).expect("stream script");
    assert!(
        started.elapsed() < Duration::from_millis(1200),
        "fetch buffered the rest of the body: {output} after {:?}",
        started.elapsed()
    );
    assert_eq!(output, "AAA:false:function", "got {output}");
}

#[test]
#[serial_test::serial]
fn fetch_init_streams_reqwest_body_without_buffering_content_length() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = listener.local_addr().expect("addr");
    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let _ = stream.set_nodelay(true);
        let mut buffer = [0u8; 1024];
        let _ = stream.read(&mut buffer);
        let head = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 6\r\nConnection: close\r\n\r\nAAA";
        let _ = stream.write_all(head);
        let _ = stream.flush();
        thread::sleep(Duration::from_millis(2000));
        let _ = stream.write_all(b"BBB");
    });
    let url = format!("http://{address}");
    let started = Instant::now();
    let output = run_js(&decode_reader_script(&url, "{ method: 'GET' }")).expect("reqwest stream");
    assert!(
        started.elapsed() < Duration::from_millis(1200),
        "init fetch buffered the body: {output} after {:?}",
        started.elapsed()
    );
    assert_eq!(output, "AAA:false:function", "got {output}");
}

#[test]
#[serial_test::serial]
fn fetch_keeps_duplicate_set_cookie() {
    let hits = Arc::new(AtomicUsize::new(0));
    let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nConnection: close\r\n\r\nok".to_vec();
    let url = spawn_one(raw, hits);
    let hits_b = Arc::new(AtomicUsize::new(0));
    let raw_b = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nConnection: close\r\n\r\nok".to_vec();
    let url_b = spawn_one(raw_b, hits_b);
    let script = format!(
        r#"
        const streamed = fetch({url:?});
        const viaInit = fetch({url_b:?}, {{ method: 'GET' }});
        streamed.headers.getSetCookie().join('|') + '#' + viaInit.headers.getSetCookie().join('|');
        "#
    );
    let output = run_js(&script).expect("set-cookie");
    assert_eq!(output, "a=1|b=2#a=1|b=2", "got {output}");
}

#[test]
#[serial_test::serial]
fn fetch_cors_mode_filters_headers_and_strips_set_cookie() {
    let hits = Arc::new(AtomicUsize::new(0));
    let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nX-Secret: no\r\nX-Exposed: yes\r\nAccess-Control-Expose-Headers: x-exposed\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nConnection: close\r\n\r\nok".to_vec();
    let url = spawn_one(raw, hits);
    let script = format!(
        r#"
        const response = fetch({url:?}, {{ mode: 'cors' }});
        const secret = response.headers.get('x-secret');
        [
            response.type,
            response.headers.get('content-type'),
            secret === null ? 'null' : String(secret),
            response.headers.get('x-exposed'),
            response.headers.getSetCookie().length
        ].join('|');
        "#
    );
    let output = run_js(&script).expect("cors");
    assert_eq!(output, "cors|text/plain|null|yes|0", "got {output}");
}

#[test]
#[serial_test::serial]
fn fetch_same_origin_type_is_basic() {
    let hits = Arc::new(AtomicUsize::new(0));
    let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec();
    let url = spawn_one(raw, hits);
    let hits_b = Arc::new(AtomicUsize::new(0));
    let raw_b = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec();
    let url_b = spawn_one(raw_b, hits_b);
    let script = format!(
        r#"
        const plain = fetch({url:?});
        const viaInit = fetch({url_b:?}, {{ method: 'GET' }});
        plain.type + '|' + viaInit.type;
        "#
    );
    let output = run_js(&script).expect("basic type");
    assert_eq!(output, "basic|basic", "got {output}");
}

#[test]
#[serial_test::serial]
fn fetch_no_cors_is_opaque() {
    let hits = Arc::new(AtomicUsize::new(0));
    let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nX-Secret: no\r\nSet-Cookie: a=1\r\nConnection: close\r\n\r\nok".to_vec();
    let url = spawn_one(raw, hits);
    let script = format!(
        r#"
        const response = fetch({url:?}, {{ mode: 'no-cors' }});
        const typeHeader = response.headers.get('content-type');
        [
            response.type,
            response.status,
            typeHeader === null ? 'null' : String(typeHeader),
            response.text(),
            response.url,
            response.headers.getSetCookie().length
        ].join('|');
        "#
    );
    let output = run_js(&script).expect("no-cors");
    assert_eq!(output, "opaque|0|null|||0", "got {output}");
}

#[test]
#[serial_test::serial]
fn fetch_manual_redirect_returns_the_3xx_and_does_not_follow() {
    let target_hits = Arc::new(AtomicUsize::new(0));
    let target = spawn_one(
        b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nland".to_vec(),
        Arc::clone(&target_hits),
    );
    let redirect_hits = Arc::new(AtomicUsize::new(0));
    let redirect = spawn_one(
        format!("HTTP/1.1 302 Found\r\nLocation: {target}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .into_bytes(),
        redirect_hits,
    );
    let script = format!(
        r#"
        const response = fetch({redirect:?}, {{ redirect: 'manual' }});
        response.status + ':' + response.redirected + ':' + response.ok + ':' + response.headers.get('location');
        "#
    );
    let output = run_js(&script).expect("manual");
    assert_eq!(output, format!("302:false:false:{target}"), "got {output}");
    assert_eq!(target_hits.load(Ordering::SeqCst), 0);
}

#[test]
#[serial_test::serial]
fn fetch_aborted_signal_does_not_connect() {
    let hits = Arc::new(AtomicUsize::new(0));
    let url = spawn_one(
        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec(),
        Arc::clone(&hits),
    );
    let script = format!(
        r#"
        const controller = new AbortController();
        controller.abort();
        fetch({url:?}, {{ signal: controller.signal }});
        "#
    );
    let error = run_js(&script).expect_err("aborted fetch should throw");
    assert!(error.contains("aborted"), "expected abort, got {error}");
    assert_eq!(hits.load(Ordering::SeqCst), 0);
}

#[test]
#[serial_test::serial]
fn fetch_abort_is_checked_again_on_the_next_redirect_hop() {
    let target_hits = Arc::new(AtomicUsize::new(0));
    let target = spawn_one(
        b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nland".to_vec(),
        Arc::clone(&target_hits),
    );
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = listener.local_addr().expect("addr");
    let redirect = format!("http://{address}");
    let location = target.clone();
    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut buffer = [0u8; 1024];
        let _ = stream.read(&mut buffer);
        amberjs::web_api::fetch::abort_in_flight_fetch();
        let raw = format!(
            "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let _ = stream.write_all(raw.as_bytes());
    });
    let script = format!(r#"fetch({redirect:?}, {{ redirect: 'follow' }});"#);
    let error = run_js(&script).expect_err("second hop should abort");
    assert!(
        error.contains("aborted"),
        "expected abort before the next hop, got {error}"
    );
    thread::sleep(Duration::from_millis(150));
    assert_eq!(target_hits.load(Ordering::SeqCst), 0);
}

#[test]
#[serial_test::serial]
fn fetch_integrity_is_checked_on_the_redirect_hop_and_the_final_body() {
    let good = sha256_sri(b"yes");
    let target_hits = Arc::new(AtomicUsize::new(0));
    let target = spawn_one(
        b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nyes".to_vec(),
        Arc::clone(&target_hits),
    );
    let bad_redirect = spawn_one(
        format!("HTTP/1.1 302 Found\r\nLocation: {target}\r\nContent-Length: 3\r\nConnection: close\r\n\r\nno!")
            .into_bytes(),
        Arc::new(AtomicUsize::new(0)),
    );
    let bad_script =
        format!(r#"fetch({bad_redirect:?}, {{ redirect: 'follow', integrity: {good:?} }});"#);
    let error = run_js(&bad_script).expect_err("redirect body must match integrity");
    assert!(error.contains("integrity mismatch"), "got {error}");
    assert_eq!(target_hits.load(Ordering::SeqCst), 0);

    let land_hits = Arc::new(AtomicUsize::new(0));
    let land = spawn_one(
        b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nyes".to_vec(),
        land_hits,
    );
    let ok_redirect = spawn_one(
        format!("HTTP/1.1 302 Found\r\nLocation: {land}\r\nContent-Length: 3\r\nConnection: close\r\n\r\nyes")
            .into_bytes(),
        Arc::new(AtomicUsize::new(0)),
    );
    let ok_script = format!(
        r#"
        const response = fetch({ok_redirect:?}, {{ redirect: 'follow', integrity: {good:?} }});
        response.status + ':' + response.redirected + ':' + response.text();
        "#
    );
    let output = run_js(&ok_script).expect("matching integrity");
    assert_eq!(output, "200:true:yes", "got {output}");
}

struct SeenRequest {
    method: String,
    path: String,
    header_text: String,
    body: Vec<u8>,
}

fn read_http_request(stream: &mut std::net::TcpStream) -> SeenRequest {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if let Some(pos) = buf.windows(4).position(|window| window == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&buf[..pos]).to_string();
                    let mut body = buf[pos + 4..].to_vec();
                    let mut content_length = 0usize;
                    for line in header.lines().skip(1) {
                        let Some((name, value)) = line.split_once(':') else {
                            continue;
                        };
                        if name.eq_ignore_ascii_case("content-length") {
                            content_length = value.trim().parse().unwrap_or(0);
                        }
                    }
                    while body.len() < content_length {
                        match stream.read(&mut tmp) {
                            Ok(0) | Err(_) => break,
                            Ok(n) => body.extend_from_slice(&tmp[..n]),
                        }
                    }
                    body.truncate(content_length);
                    let request_line = header.lines().next().unwrap_or("");
                    let mut parts = request_line.split_whitespace();
                    return SeenRequest {
                        method: parts.next().unwrap_or("").to_string(),
                        path: parts.next().unwrap_or("").to_string(),
                        header_text: header,
                        body,
                    };
                }
            }
            Err(_) => break,
        }
    }
    SeenRequest {
        method: String::new(),
        path: String::new(),
        header_text: String::new(),
        body: Vec::new(),
    }
}

fn spawn_router(handler: impl Fn(SeenRequest) -> Vec<u8> + Send + 'static) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = listener.local_addr().expect("addr");
    thread::spawn(move || {
        listener.set_nonblocking(true).expect("nonblocking");
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(8) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_nodelay(true);
                    let request = read_http_request(&mut stream);
                    let raw = handler(request);
                    let _ = stream.write_all(&raw);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    });
    format!("http://{address}")
}

fn http_raw(status: u16, reason: &str, headers: &[(&str, &str)], body: &str) -> Vec<u8> {
    let mut raw = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in headers {
        raw.push_str(name);
        raw.push_str(": ");
        raw.push_str(value);
        raw.push_str("\r\n");
    }
    raw.push_str("Content-Length: ");
    raw.push_str(&body.len().to_string());
    raw.push_str("\r\nConnection: close\r\n\r\n");
    raw.push_str(body);
    raw.into_bytes()
}

#[test]
#[serial_test::serial]
fn fetch_redirect_error_rejects() {
    let url = spawn_one(
        b"HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec(),
        Arc::new(AtomicUsize::new(0)),
    );
    let script = format!(
        r#"
        try {{
            fetch({url:?}, {{ redirect: 'error' }});
            'no-throw';
        }} catch (error) {{
            String(error && error.message ? error.message : error);
        }}
        "#
    );
    let output = run_js(&script).expect("redirect error");
    assert!(
        output.contains("redirect not allowed"),
        "got {output}"
    );
}

#[test]
#[serial_test::serial]
fn fetch_follow_rewrites_303_and_preserves_307() {
    let url = spawn_router(|request| {
        if request.path == "/see-other" {
            return http_raw(303, "See Other", &[("Location", "/landed")], "");
        }
        if request.path == "/keep" {
            return http_raw(307, "Temporary Redirect", &[("Location", "/kept")], "");
        }
        let body = String::from_utf8_lossy(&request.body);
        http_raw(
            200,
            "OK",
            &[("Content-Type", "text/plain")],
            &format!("{}:{body}", request.method),
        )
    });
    let script = format!(
        r#"
        const rewritten = fetch({url:?} + '/see-other', {{
            method: 'POST',
            body: 'secret',
            redirect: 'follow'
        }});
        const preserved = fetch({url:?} + '/keep', {{
            method: 'POST',
            body: 'secret',
            redirect: 'follow'
        }});
        rewritten.status + ':' + rewritten.redirected + ':' + rewritten.text()
            + '#' + preserved.status + ':' + preserved.redirected + ':' + preserved.text();
        "#
    );
    let output = run_js(&script).expect("follow rewrite");
    assert_eq!(
        output, "200:true:GET:#200:true:POST:secret",
        "got {output}"
    );
}

#[test]
#[serial_test::serial]
fn fetch_follow_stops_after_twenty_hops() {
    let url = spawn_router(|_| http_raw(302, "Found", &[("Location", "/loop")], ""));
    let script = format!(
        r#"
        try {{
            fetch({url:?} + '/loop');
            'no-throw';
        }} catch (error) {{
            String(error && error.message ? error.message : error);
        }}
        "#
    );
    let output = run_js(&script).expect("redirect cap");
    assert!(output.contains("Too many redirects"), "got {output}");
}

#[test]
#[serial_test::serial]
fn fetch_follow_drops_authorization_on_cross_origin() {
    let destination = spawn_router(|request| {
        let kept = request
            .header_text
            .to_ascii_lowercase()
            .contains("authorization:");
        http_raw(
            200,
            "OK",
            &[("Content-Type", "text/plain")],
            if kept { "kept" } else { "stripped" },
        )
    });
    let location = destination.clone();
    let start = spawn_router(move |_| http_raw(302, "Found", &[("Location", &location)], ""));
    let script = format!(
        r#"
        const response = fetch({start:?} + '/', {{
            headers: {{ Authorization: 'Bearer secret' }},
            redirect: 'follow'
        }});
        response.status + ':' + response.redirected + ':' + response.text();
        "#
    );
    let output = run_js(&script).expect("cross-origin redirect");
    assert_eq!(output, "200:true:stripped", "got {output}");
}

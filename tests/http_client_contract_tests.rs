//! Pins docs/HTTP_CLIENT_CONTRACT.md — narrow Node `http` client (G72).
//! Wire-hitting against a Rust-side TcpListener test server: real TCP +
//! real HTTP/1.1 bytes both directions, buffered response delivered as
//! 'data'/'end' events, 'error' on refusal (never fake 200).

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

fn eval(code: &str) -> String {
    let mut runtime = MinimalRuntime::new().expect("runtime");
    runtime
        .execute_code(code)
        .unwrap_or_else(|error| panic!("{code} should evaluate: {error}"))
        .trim()
        .to_string()
}

/// 起一个一次性 Rust 测试服务器：读完整请求（含 body 到 Content-Length），
/// 把 raw 请求存进 channel，回固定响应。
fn serve_once(response: &str) -> (u16, std::sync::mpsc::Receiver<String>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = std::sync::mpsc::channel();
    let response = response.to_string();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            loop {
                match stream.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        buf.extend_from_slice(&chunk[..n]);
                        // 头收齐且 Content-Length 声明的 body 收齐即停
                        if let Some(he) = find_subslice(&buf, b"\r\n\r\n") {
                            let head = &buf[..he];
                            let need: usize = std::str::from_utf8(head)
                                .unwrap_or("")
                                .lines()
                                .find_map(|l| {
                                    l.to_ascii_lowercase()
                                        .strip_prefix("content-length:")
                                        .and_then(|v| v.trim().parse().ok())
                                })
                                .unwrap_or(0);
                            if buf.len() >= he + 4 + need {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
                if std::time::Instant::now() > deadline {
                    break;
                }
            }
            let _ = tx.send(String::from_utf8_lossy(&buf).to_string());
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    });
    thread::sleep(Duration::from_millis(30));
    (port, rx)
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

const OK_RESP: &str =
    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\n\r\nhello";

#[test]
#[serial]
fn http_get_returns_status_headers_and_streams_body_events() {
    let (port, rx) = serve_once(OK_RESP);
    let out = eval(&format!(
        r#"
        const http = require('http');
        const got = [];
        http.get('http://127.0.0.1:{port}/p?q=1', (res) => {{
          got.push('SC=' + res.statusCode);
          got.push('SM=' + res.statusMessage);
          got.push('CT=' + res.headers['content-type']);
          res.on('data', (c) => got.push('D=' + c));
          res.on('end', () => got.push('END'));
        }});
        got.join('|');
        "#,
        port = port
    ));
    let req = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("request bytes");
    assert!(req.starts_with("GET /p?q=1 HTTP/1.1"), "raw request: {req}");
    assert!(req.contains("Host: 127.0.0.1"), "Host header: {req}");
    assert_eq!(out, "SC=200|SM=OK|CT=text/plain|D=hello|END");
}

#[test]
#[serial]
fn http_request_sends_custom_headers_and_body() {
    let (port, rx) = serve_once("HTTP/1.1 201 Created\r\nContent-Length: 2\r\n\r\nok");
    let out = eval(&format!(
        r#"
        const http = require('http');
        const got = [];
        const req = http.request({{
          hostname: '127.0.0.1', port: {port}, path: '/submit', method: 'POST',
          headers: {{ 'x-token': 'abc', 'content-type': 'text/plain' }}
        }}, (res) => {{ got.push('SC=' + res.statusCode); }});
        req.write('part1');
        req.end('part2');
        got.join('|');
        "#,
        port = port
    ));
    let req = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("request bytes");
    assert!(req.starts_with("POST /submit HTTP/1.1"), "raw: {req}");
    assert!(req.contains("x-token: abc"), "custom header sent: {req}");
    assert!(req.contains("content-type: text/plain"), "header: {req}");
    assert!(req.contains("Content-Length: 10"), "content-length: {req}");
    assert!(req.ends_with("part1part2"), "body: {req}");
    assert_eq!(out, "SC=201");
}

#[test]
#[serial]
fn http_response_event_fires_and_pipe_delivers_to_writable() {
    let (port, _rx) = serve_once(OK_RESP);
    let out = eval(&format!(
        r#"
        const http = require('http');
        const {{ Writable }} = require('stream');
        const got = [];
        const req = http.request({{ hostname: '127.0.0.1', port: {port}, path: '/' }});
        req.on('response', (res) => {{
          got.push('RESP');
          const w = new Writable({{ write(c, e, cb) {{ got.push('W=' + c); cb(); }} }});
          w.on('finish', () => got.push('FIN'));
          res.pipe(w);
        }});
        req.end();
        got.join('|');
        "#,
        port = port
    ));
    assert_eq!(out, "RESP|W=hello|FIN");
}

#[test]
#[serial]
fn http_client_error_event_on_connection_refused_never_fakes_200() {
    // 挑一个几乎肯定没被占用的端口：bind 后立刻关闭释放
    let port = {
        let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        l.local_addr().unwrap().port()
    };
    let out = eval(&format!(
        r#"
        const http = require('http');
        const got = [];
        const req = http.get('http://127.0.0.1:{port}/');
        req.on('error', (e) => got.push('ERR:' + e.name));
        req.end();
        got.join('|');
        "#,
        port = port
    ));
    assert_eq!(out, "ERR:Error");
}

#[test]
#[serial]
fn http_get_options_object_form_and_end_callback() {
    let (port, rx) = serve_once(OK_RESP);
    let out = eval(&format!(
        r#"
        const http = require('http');
        const got = [];
        // http.request 不自动发送——'response' 监听先挂，end(cb) 后发
        const req = http.request({{ hostname: '127.0.0.1', port: {port}, path: '/obj' }}, (res) => {{
          got.push('SC=' + res.statusCode);
        }});
        req.on('response', () => got.push('EV'));
        req.end(() => got.push('ENDCB'));
        got.join('|');
        "#,
        port = port
    ));
    let req = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("request bytes");
    assert!(req.starts_with("GET /obj HTTP/1.1"), "raw: {req}");
    // 顺序：end 回调 → 'response' 事件 → 响应 cb（缓冲模型，见契约 Limits）
    assert_eq!(out, "ENDCB|EV|SC=200");
}

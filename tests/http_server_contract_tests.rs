//! Pins docs/HTTP_SERVER_CONTRACT.md — narrow Node `http` server (G37).
//! Wire-hitting: listen/bind, address(), request/response, Content-Length,
//! close, missing handler → 503, permission denial.
//! Does **not** pin http.request / http.get client, Agent, or https.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

fn setup_test_environment() {
    use amberjs::nodejs_core::http::reset_http_server_channel;
    reset_http_server_channel();
    thread::sleep(Duration::from_millis(200));
}

fn close_test_server(runtime: &mut MinimalRuntime) {
    let _ = runtime.execute_code(
        r#"
        if (globalThis._testServer && typeof globalThis._testServer.close === 'function') {
            globalThis._testServer.close();
            globalThis._testServer = null;
        }
        "#,
    );
    thread::sleep(Duration::from_millis(50));
}

fn wait_for_server(port: u16) {
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("Server did not start in time on port {port}");
}

fn send_request_and_get_response(port: u16, request: &str, runtime: &mut MinimalRuntime) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("Failed to connect");
    stream
        .set_nonblocking(true)
        .expect("set_nonblocking failed");
    stream
        .write_all(request.as_bytes())
        .expect("Failed to write");
    let _ = stream.shutdown(std::net::Shutdown::Write);

    let mut response = String::new();
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        let _ = runtime.pump_http_messages();
        let mut buffer = [0u8; 4096];
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                response.push_str(&String::from_utf8_lossy(&buffer[..n]));
                if response.contains("\r\n\r\n") {
                    // Wait briefly for body when Content-Length is present.
                    if let Some(cl) = response
                        .split("\r\n")
                        .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
                    {
                        if let Some(n_str) = cl.split(':').nth(1) {
                            if let Ok(need) = n_str.trim().parse::<usize>() {
                                if let Some(header_end) = response.find("\r\n\r\n") {
                                    let body_len = response.len() - header_end - 4;
                                    if body_len >= need {
                                        break;
                                    }
                                }
                            }
                        }
                    } else {
                        break;
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(e) => panic!("Read error: {e}"),
        }
        thread::sleep(Duration::from_millis(5));
    }
    response
}

#[test]
#[serial]
fn require_http_is_global_http() {
    setup_test_environment();
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let out = runtime
        .execute_code(
            r#"
            const mod = require('http');
            const node = require('node:http');
            [
              typeof http.createServer === 'function',
              mod === http,
              node === http,
              typeof mod.createServer === 'function'
            ].join('|');
            "#,
        )
        .expect("exec")
        .trim()
        .to_string();
    assert_eq!(out, "true|true|true|true");
}

#[test]
#[serial]
fn listen_bind_address_and_close() {
    setup_test_environment();
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let code = r#"
        const server = require('http').createServer((req, res) => {
            res.writeHead(200, { 'Content-Type': 'text/plain' });
            res.end('ok');
        });
        globalThis._testServer = server;
        let listeningFired = false;
        server.listen(3710, '127.0.0.1', () => { listeningFired = true; });
        const addr = server.address();
        [
          server.listening === true,
          listeningFired === true,
          addr && addr.port === 3710,
          addr && addr.address === '127.0.0.1',
          addr && addr.family === 'IPv4'
        ].join('|');
    "#;
    let out = runtime.execute_code(code).expect("exec").trim().to_string();
    assert_eq!(
        out, "true|true|true|true|true",
        "listen/address shape: {out}"
    );

    wait_for_server(3710);
    let response = send_request_and_get_response(
        3710,
        "GET /ping HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        &mut runtime,
    );
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "expected 200, got: {response}"
    );
    assert!(
        response.contains("Content-Length: 2"),
        "expected Content-Length for buffered body, got: {response}"
    );
    assert!(response.contains("ok"), "expected body ok, got: {response}");

    let closed = runtime
        .execute_code(
            r#"
            globalThis._testServer.close();
            const listening = globalThis._testServer.listening;
            globalThis._testServer = null;
            listening === false;
            "#,
        )
        .expect("close")
        .trim()
        .to_string();
    assert_eq!(closed, "true");
}

#[test]
#[serial]
fn listen_options_and_ephemeral_port() {
    setup_test_environment();
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let code = r#"
        const server = require('http').createServer((_req, res) => {
            res.end('e');
        });
        globalThis._testServer = server;
        server.listen({ port: 0, host: '127.0.0.1' });
        const addr = server.address();
        globalThis._boundPort = addr.port;
        [
          server.listening === true,
          typeof addr.port === 'number',
          addr.port > 0,
          addr.address === '127.0.0.1'
        ].join('|') + '|' + String(addr.port);
    "#;
    let out = runtime.execute_code(code).expect("exec").trim().to_string();
    let parts: Vec<&str> = out.split('|').collect();
    assert_eq!(
        &parts[..4],
        &["true", "true", "true", "true"],
        "ephemeral listen: {out}"
    );
    let port: u16 = parts[4].parse().expect("port");
    wait_for_server(port);
    let response = send_request_and_get_response(
        port,
        "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        &mut runtime,
    );
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "ephemeral wire: {response}"
    );
    close_test_server(&mut runtime);
}

#[test]
#[serial]
fn request_method_path_headers_body_and_buffered_response() {
    setup_test_environment();
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let code = r#"
        const server = require('http').createServer((req, res) => {
            let body = '';
            req.on('data', (chunk) => { body += chunk; });
            req.on('end', () => {
                res.setHeader('X-Echo-Method', req.method);
                res.setHeader('X-Echo-Path', req.path || req.url);
                res.setHeader('X-Echo-Ct', req.headers['content-type'] || '');
                res.writeHead(201, { 'Content-Type': 'text/plain' });
                res.write('got:');
                res.end(body);
            });
        });
        globalThis._testServer = server;
        server.listen(3711, '127.0.0.1');
    "#;
    runtime.execute_code(code).expect("exec");
    wait_for_server(3711);

    let request = concat!(
        "POST /api/echo?x=1 HTTP/1.1\r\n",
        "Host: localhost\r\n",
        "Content-Type: application/json\r\n",
        "Content-Length: 11\r\n",
        "Connection: close\r\n",
        "\r\n",
        "{\"a\":true}"
    );
    let response = send_request_and_get_response(3711, request, &mut runtime);
    close_test_server(&mut runtime);

    assert!(response.starts_with("HTTP/1.1 201"), "status: {response}");
    assert!(
        response.contains("X-Echo-Method: POST"),
        "method header: {response}"
    );
    assert!(
        response.contains("X-Echo-Path: /api/echo"),
        "path header: {response}"
    );
    assert!(
        response.contains("X-Echo-Ct: application/json"),
        "request content-type: {response}"
    );
    assert!(
        response.contains("Content-Length: 14"),
        "buffered Content-Length for got:{{\"a\":true}} (14 bytes): {response}"
    );
    assert!(response.contains("got:{\"a\":true}"), "body: {response}");
}

#[test]
#[serial]
fn missing_handler_returns_503_not_fake_200() {
    setup_test_environment();
    let mut runtime = MinimalRuntime::new().expect("runtime");
    // createServer with no request handler → fail-closed 503
    let code = r#"
        const server = require('http').createServer();
        globalThis._testServer = server;
        server.listen(3712, '127.0.0.1');
    "#;
    runtime.execute_code(code).expect("exec");
    wait_for_server(3712);

    let response = send_request_and_get_response(
        3712,
        "GET /no-handler HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        &mut runtime,
    );
    close_test_server(&mut runtime);

    assert!(
        response.starts_with("HTTP/1.1 503"),
        "missing handler must be 503, got: {response}"
    );
    assert!(
        !response.starts_with("HTTP/1.1 200"),
        "must not invent fake 200, got: {response}"
    );
}

#[test]
#[serial]
fn missing_dispatcher_returns_503_not_fake_200() {
    setup_test_environment();
    use amberjs::nodejs_core::http::{get_http_server_channel, reset_http_server_channel};

    let mut runtime = MinimalRuntime::new().expect("runtime");
    let code = r#"
        const server = require('http').createServer((req, res) => {
            res.writeHead(200, { 'Content-Type': 'text/plain' });
            res.end('should-not-run');
        });
        globalThis._testServer = server;
        server.listen(3713, '127.0.0.1');
    "#;
    runtime.execute_code(code).expect("exec");
    wait_for_server(3713);

    let channel = get_http_server_channel().expect("channel");
    {
        let mut guard = channel.lock().unwrap();
        *guard = None;
    }

    let mut stream = TcpStream::connect(("127.0.0.1", 3713)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("timeout");
    stream
        .write_all(
            b"GET /missing-dispatcher HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .expect("write");
    let _ = stream.shutdown(std::net::Shutdown::Write);

    let mut response_bytes = Vec::new();
    let mut buffer = [0u8; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => response_bytes.extend_from_slice(&buffer[..n]),
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(e) => panic!("Read error: {e}"),
        }
    }
    let response = String::from_utf8_lossy(&response_bytes).to_string();
    close_test_server(&mut runtime);
    reset_http_server_channel();

    let fail_closed = response.is_empty() || response.starts_with("HTTP/1.1 5");
    assert!(
        fail_closed,
        "missing dispatcher must fail closed, got: {response}"
    );
    assert!(
        !response.starts_with("HTTP/1.1 200"),
        "must not invent fake 200, got: {response}"
    );
}

#[test]
#[serial]
fn listen_permission_denial_fails_closed() {
    use amberjs::permissions::{
        global_resource_broker, PermissionAction, PermissionKind, ResourceBroker, ResourceId,
    };
    use std::sync::atomic::Ordering;

    struct RestorePermissions {
        broker: ResourceBroker,
    }
    impl Drop for RestorePermissions {
        fn drop(&mut self) {
            amberjs::permissions::reset_runtime_permission_state();
            if let Ok(mut broker) = global_resource_broker().write() {
                *broker = self.broker.clone();
            }
        }
    }

    setup_test_environment();
    let saved = global_resource_broker()
        .read()
        .expect("broker lock")
        .clone();
    let _guard = RestorePermissions { broker: saved };
    {
        let mut broker = global_resource_broker().write().expect("broker lock");
        *broker = ResourceBroker::default();
        broker.deny_all();
        // Allow nothing for Listen — Connect alone must not authorize listen.
        broker.allow(
            PermissionKind::Network,
            PermissionAction::Connect,
            ResourceId::Name("127.0.0.1".to_string()),
        );
    }
    amberjs::permissions::HAS_RESTRICTIONS.store(true, Ordering::SeqCst);

    let mut runtime = MinimalRuntime::new().expect("runtime");
    let out = runtime
        .execute_code(
            r#"
            let message = '';
            const server = require('http').createServer((_req, res) => res.end('x'));
            try {
                server.listen(3714, '127.0.0.1');
                message = 'no-throw';
            } catch (error) {
                message = String(error && error.message ? error.message : error);
            }
            message + '|' + String(server.listening === false);
            "#,
        )
        .expect("exec")
        .trim()
        .to_string();

    assert!(
        out.contains("permission denied"),
        "listen denial must mention permission denied, got: {out}"
    );
    assert!(
        out.ends_with("|true"),
        "denied listen must leave listening === false, got: {out}"
    );
    assert!(
        !out.starts_with("no-throw"),
        "denied listen must throw, got: {out}"
    );
}

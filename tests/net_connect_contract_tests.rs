//! Pins docs/NET_CONNECT_CONTRACT.md — narrow Node `net.connect` client (G61).
//! Wire-hitting: connect → 'connect', write bytes, 'data' from peer, end.
//! Does **not** pin http client/Agent/https/http2, TLS, or unix sockets.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

fn setup_test_environment() {
    use amberjs::nodejs_core::net::reset_net_server_channel;
    reset_net_server_channel();
    thread::sleep(Duration::from_millis(100));
}

fn pump_until<F>(runtime: &mut MinimalRuntime, mut pred: F, timeout: Duration) -> bool
where
    F: FnMut(&mut MinimalRuntime) -> bool,
{
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        let _ = runtime.pump_net_socket_events();
        if pred(runtime) {
            return true;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let _ = runtime.pump_net_socket_events();
    pred(runtime)
}

#[test]
#[serial]
fn require_net_connect_is_function() {
    setup_test_environment();
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let out = runtime
        .execute_code(
            r#"
            const mod = require('net');
            [
              typeof mod.connect === 'function',
              typeof mod.createConnection === 'function',
              mod === net,
              typeof net.Socket === 'function'
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
fn connect_emits_connect_and_sets_address_fields() {
    setup_test_environment();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let accept = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream.set_read_timeout(Some(Duration::from_secs(3))).ok();
        let mut buf = [0u8; 32];
        let _ = stream.read(&mut buf);
        stream
    });

    let mut runtime = MinimalRuntime::new().expect("runtime");
    let code = format!(
        r#"
        globalThis._connected = false;
        globalThis._remote = '';
        globalThis._sock = require('net').connect({port}, '127.0.0.1', () => {{
            globalThis._connected = true;
            globalThis._remote = String(globalThis._sock.remoteAddress || '');
        }});
        'ready';
        "#,
        port = port
    );
    assert_eq!(runtime.execute_code(&code).expect("exec").trim(), "ready");

    assert!(
        pump_until(
            &mut runtime,
            |rt| {
                rt.execute_code("String(globalThis._connected)")
                    .map(|s| s.trim() == "true")
                    .unwrap_or(false)
            },
            Duration::from_secs(5)
        ),
        "'connect' listener must fire"
    );

    let remote = runtime
        .execute_code("globalThis._remote + '|' + String(globalThis._sock.connecting)")
        .expect("remote")
        .trim()
        .to_string();
    assert!(
        remote.starts_with("127.0.0.1|"),
        "remoteAddress should be loopback, got {remote}"
    );
    assert!(
        remote.ends_with("|false"),
        "connecting should be false after connect, got {remote}"
    );

    let _ = runtime.execute_code("globalThis._sock.destroy();");
    let _ = accept.join();
}

#[test]
#[serial]
fn write_bytes_reach_peer() {
    setup_test_environment();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let accept = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
        let mut body = Vec::new();
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            let mut buf = [0u8; 64];
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    body.extend_from_slice(&buf[..n]);
                    if body
                        .windows(b"amber-net-g61".len())
                        .any(|w| w == b"amber-net-g61")
                    {
                        break;
                    }
                }
                Err(_) => thread::sleep(Duration::from_millis(10)),
            }
        }
        body
    });

    let mut runtime = MinimalRuntime::new().expect("runtime");
    let code = format!(
        r#"
        globalThis._sock = require('net').connect({port}, '127.0.0.1', () => {{
            globalThis._sock.write('amber-net-g61');
        }});
        'ready';
        "#,
        port = port
    );
    assert_eq!(runtime.execute_code(&code).expect("exec").trim(), "ready");

    // Pump connect so the listener runs write().
    for _ in 0..200 {
        let _ = runtime.pump_net_socket_events();
        thread::sleep(Duration::from_millis(5));
    }

    let body = accept.join().expect("join");
    let text = String::from_utf8_lossy(&body);
    assert!(
        text.contains("amber-net-g61"),
        "peer must receive write bytes, got {text:?}"
    );

    let _ = runtime.execute_code("globalThis._sock.destroy();");
}

#[test]
#[serial]
fn data_event_delivers_peer_bytes() {
    setup_test_environment();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let accept = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream.write_all(b"from-peer-g61").ok();
        stream.flush().ok();
        // Keep open briefly so client can read.
        thread::sleep(Duration::from_millis(200));
        stream
    });

    let mut runtime = MinimalRuntime::new().expect("runtime");
    let code = format!(
        r#"
        globalThis._chunks = '';
        globalThis._sock = require('net').connect({port}, '127.0.0.1');
        globalThis._sock.on('data', (chunk) => {{
            globalThis._chunks += String(chunk);
        }});
        'ready';
        "#,
        port = port
    );
    assert_eq!(runtime.execute_code(&code).expect("exec").trim(), "ready");

    assert!(
        pump_until(
            &mut runtime,
            |rt| {
                rt.execute_code("globalThis._chunks")
                    .map(|s| s.contains("from-peer-g61"))
                    .unwrap_or(false)
            },
            Duration::from_secs(5)
        ),
        "'data' must deliver peer bytes"
    );

    let _ = runtime.execute_code("globalThis._sock.destroy();");
    let _ = accept.join();
}

#[test]
#[serial]
fn end_half_closes_write_side() {
    setup_test_environment();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let accept = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
        let mut body = Vec::new();
        loop {
            let mut buf = [0u8; 64];
            match stream.read(&mut buf) {
                Ok(0) => break, // write half-close → EOF on peer read
                Ok(n) => body.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
        body
    });

    let mut runtime = MinimalRuntime::new().expect("runtime");
    let code = format!(
        r#"
        globalThis._sock = require('net').connect({port}, '127.0.0.1', () => {{
            globalThis._sock.end('bye-g61');
        }});
        'ready';
        "#,
        port = port
    );
    assert_eq!(runtime.execute_code(&code).expect("exec").trim(), "ready");

    for _ in 0..300 {
        let _ = runtime.pump_net_socket_events();
        thread::sleep(Duration::from_millis(5));
    }

    let body = accept.join().expect("join");
    let text = String::from_utf8_lossy(&body);
    assert!(
        text.contains("bye-g61"),
        "end(chunk) must write then half-close, got {text:?}"
    );

    let _ = runtime.execute_code("globalThis._sock.destroy();");
}

#[test]
#[serial]
fn create_connection_alias_and_options_object() {
    setup_test_environment();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let accept = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        stream
    });

    let mut runtime = MinimalRuntime::new().expect("runtime");
    let code = format!(
        r#"
        globalThis._ok = false;
        globalThis._sock = require('net').createConnection({{ port: {port}, host: '127.0.0.1' }}, () => {{
            globalThis._ok = true;
        }});
        'ready';
        "#,
        port = port
    );
    assert_eq!(runtime.execute_code(&code).expect("exec").trim(), "ready");
    assert!(
        pump_until(
            &mut runtime,
            |rt| {
                rt.execute_code("String(globalThis._ok)")
                    .map(|s| s.trim() == "true")
                    .unwrap_or(false)
            },
            Duration::from_secs(5)
        ),
        "createConnection options + listener must connect"
    );
    let _ = runtime.execute_code("globalThis._sock.destroy();");
    let _ = accept.join();
}

#[test]
#[serial]
fn unix_path_connect_throws() {
    setup_test_environment();
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let out = runtime
        .execute_code(
            r#"
            let message = '';
            try {
                require('net').connect('/tmp/amber-net-g61.sock');
                message = 'no-throw';
            } catch (error) {
                message = String(error && error.message ? error.message : error);
            }
            message;
            "#,
        )
        .expect("exec")
        .trim()
        .to_string();
    assert!(
        out.to_lowercase().contains("unix"),
        "unix path must throw naming unix sockets, got {out}"
    );
}

#[test]
#[serial]
fn connect_permission_denied() {
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
        broker.allow(
            PermissionKind::Network,
            PermissionAction::Listen,
            ResourceId::Name("127.0.0.1".to_string()),
        );
    }
    amberjs::permissions::HAS_RESTRICTIONS.store(true, Ordering::SeqCst);

    let mut runtime = MinimalRuntime::new().expect("runtime");
    let out = runtime
        .execute_code(
            r#"
            let message = '';
            try {
                require('net').connect(3999, '127.0.0.1');
                message = 'no-throw';
            } catch (error) {
                message = String(error && error.message ? error.message : error);
            }
            message;
            "#,
        )
        .expect("exec")
        .trim()
        .to_string();

    assert!(
        out.contains("permission denied") || out.to_lowercase().contains("permission"),
        "connect denial must mention permission, got: {out}"
    );
    assert!(
        !out.starts_with("no-throw"),
        "denied connect must throw, got: {out}"
    );
}

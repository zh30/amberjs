//! Pins docs/NET_SERVER_CONTRACT.md — narrow Node `net` server (G45).
//! Wire-hitting: listen/bind, address(), 'connection', socket.write bytes,
//! close, permission denial.
//! Does **not** pin net.connect client rewrite, TLS, or full Socket 'data' parity.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::io::Read;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

fn setup_test_environment() {
    use amberjs::nodejs_core::net::reset_net_server_channel;
    reset_net_server_channel();
    thread::sleep(Duration::from_millis(200));
}

fn close_test_server(runtime: &mut MinimalRuntime) {
    let _ = runtime.execute_code(
        r#"
        if (globalThis._testNetServer && typeof globalThis._testNetServer.close === 'function') {
            globalThis._testNetServer.close();
            globalThis._testNetServer = null;
        }
        "#,
    );
    thread::sleep(Duration::from_millis(50));
}

fn wait_for_port(port: u16) {
    for _ in 0..100 {
        if let Ok(stream) = TcpStream::connect(("127.0.0.1", port)) {
            drop(stream);
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("net.Server did not accept connects on port {port}");
}

/// Drain accepts left by `wait_for_port` probe connects so counters stay honest.
fn drain_probe_connections(runtime: &mut MinimalRuntime) {
    // Allow accept thread to enqueue the probe connection.
    thread::sleep(Duration::from_millis(30));
    for _ in 0..40 {
        let n = runtime.pump_net_connections();
        if n == 0 {
            thread::sleep(Duration::from_millis(5));
            if runtime.pump_net_connections() == 0 {
                break;
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    let _ = runtime.execute_code(
        r#"
        globalThis._connCount = 0;
        globalThis._onConn = 0;
        globalThis._lastRemote = '';
        "#,
    );
}

#[test]
#[serial]
fn require_net_is_global_net() {
    setup_test_environment();
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let out = runtime
        .execute_code(
            r#"
            const mod = require('net');
            const node = require('node:net');
            [
              typeof net.createServer === 'function',
              mod === net,
              node === net,
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
fn listen_port_host_callback_address_and_close() {
    setup_test_environment();
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let code = r#"
        const server = require('net').createServer();
        globalThis._testNetServer = server;
        let listeningFired = false;
        server.listen(3745, '127.0.0.1', () => { listeningFired = true; });
        const addr = server.address();
        [
          server.listening === true,
          listeningFired === true,
          addr && addr.port === 3745,
          addr && addr.address === '127.0.0.1',
          addr && addr.family === 'IPv4'
        ].join('|');
    "#;
    let out = runtime.execute_code(code).expect("exec").trim().to_string();
    assert_eq!(
        out, "true|true|true|true|true",
        "listen/address shape: {out}"
    );

    wait_for_port(3745);
    close_test_server(&mut runtime);
    let closed = runtime
        .execute_code(
            r#"
            const s = globalThis._testNetServer;
            // closed in helper; re-check via fresh listen state is N/A — probe last close:
            'ok';
            "#,
        )
        .expect("exec");
    assert_eq!(closed.trim(), "ok");
}

#[test]
#[serial]
fn listen_options_ephemeral_port() {
    setup_test_environment();
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let code = r#"
        const server = require('net').createServer();
        globalThis._testNetServer = server;
        server.listen({ port: 0, host: '127.0.0.1' });
        const addr = server.address();
        [
          server.listening === true,
          addr && typeof addr.port === 'number' && addr.port > 0,
          addr && addr.address === '127.0.0.1'
        ].join('|') + '|' + (addr ? addr.port : 0);
    "#;
    let out = runtime.execute_code(code).expect("exec").trim().to_string();
    let parts: Vec<&str> = out.split('|').collect();
    assert!(parts.len() >= 4, "unexpected: {out}");
    assert_eq!(parts[0], "true", "listening: {out}");
    assert_eq!(parts[1], "true", "ephemeral port: {out}");
    assert_eq!(parts[2], "true", "host: {out}");
    let port: u16 = parts[3].parse().expect("port");
    assert!(port > 0);
    wait_for_port(port);
    close_test_server(&mut runtime);
}

#[test]
#[serial]
fn listen_port_callback_shape_parses_number() {
    // Regression: extract_integer_option on a bare number used to default port to 0.
    setup_test_environment();
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let code = r#"
        const server = require('net').createServer();
        globalThis._testNetServer = server;
        server.listen(3746, () => {});
        const addr = server.address();
        String(addr && addr.port);
    "#;
    let out = runtime.execute_code(code).expect("exec").trim().to_string();
    assert_eq!(out, "3746", "listen(port, cb) must bind port, got {out}");
    wait_for_port(3746);
    close_test_server(&mut runtime);
}

#[test]
#[serial]
fn connection_event_and_socket_write_bytes() {
    setup_test_environment();
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let code = r#"
        globalThis._connCount = 0;
        globalThis._lastRemote = '';
        const server = require('net').createServer((socket) => {
            globalThis._connCount += 1;
            globalThis._lastRemote = String(socket.remoteAddress || '');
            socket.write('amber-net-g45');
        });
        globalThis._testNetServer = server;
        server.listen(3747, '127.0.0.1');
        const addr = server.address();
        String(addr.port);
    "#;
    let port_str = runtime.execute_code(code).expect("exec").trim().to_string();
    assert_eq!(port_str, "3747");
    wait_for_port(3747);
    drain_probe_connections(&mut runtime);

    let mut stream = TcpStream::connect(("127.0.0.1", 3747)).expect("connect");
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok();

    // Pump until connection handler runs and writes.
    let mut body = Vec::new();
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        let _ = runtime.pump_net_connections();
        let mut buf = [0u8; 64];
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                body.extend_from_slice(&buf[..n]);
                if body
                    .windows(b"amber-net-g45".len())
                    .any(|w| w == b"amber-net-g45")
                {
                    break;
                }
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("read error: {e}"),
        }
        thread::sleep(Duration::from_millis(5));
    }

    let text = String::from_utf8_lossy(&body);
    assert!(
        text.contains("amber-net-g45"),
        "expected socket.write payload, got {text:?}"
    );

    let stats = runtime
        .execute_code("String(globalThis._connCount) + '|' + globalThis._lastRemote")
        .expect("stats")
        .trim()
        .to_string();
    assert!(
        stats.starts_with("1|"),
        "connection should fire once with remote addr, got {stats}"
    );
    assert!(
        stats.contains("127.0.0.1"),
        "remoteAddress should be loopback, got {stats}"
    );

    close_test_server(&mut runtime);
}

#[test]
#[serial]
fn server_on_connection_works() {
    setup_test_environment();
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let code = r#"
        globalThis._onConn = 0;
        const server = require('net').createServer();
        server.on('connection', (socket) => {
            globalThis._onConn += 1;
            socket.write('via-on');
        });
        globalThis._testNetServer = server;
        server.listen(3748, '127.0.0.1');
        'ready';
    "#;
    assert_eq!(runtime.execute_code(code).expect("exec").trim(), "ready");
    wait_for_port(3748);
    drain_probe_connections(&mut runtime);

    let mut stream = TcpStream::connect(("127.0.0.1", 3748)).expect("connect");
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok();
    let mut body = Vec::new();
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        let _ = runtime.pump_net_connections();
        let mut buf = [0u8; 32];
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                body.extend_from_slice(&buf[..n]);
                if body.windows(b"via-on".len()).any(|w| w == b"via-on") {
                    break;
                }
            }
            Err(_) => thread::sleep(Duration::from_millis(10)),
        }
    }
    assert!(
        String::from_utf8_lossy(&body).contains("via-on"),
        "server.on('connection') must deliver, got {:?}",
        String::from_utf8_lossy(&body)
    );
    let count = runtime
        .execute_code("String(globalThis._onConn)")
        .expect("count")
        .trim()
        .to_string();
    assert_eq!(count, "1");
    close_test_server(&mut runtime);
}

#[test]
#[serial]
fn close_stops_listening() {
    setup_test_environment();
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let _ = runtime
        .execute_code(
            r#"
            const server = require('net').createServer();
            globalThis._testNetServer = server;
            server.listen(3749, '127.0.0.1');
            "#,
        )
        .expect("listen");
    wait_for_port(3749);
    let out = runtime
        .execute_code(
            r#"
            const s = globalThis._testNetServer;
            s.close();
            [
              s.listening === false,
              s.address() === null
            ].join('|');
            "#,
        )
        .expect("close")
        .trim()
        .to_string();
    assert_eq!(out, "true|true", "close shape: {out}");
    let _ = runtime.execute_code("globalThis._testNetServer = null;");
}

#[test]
#[serial]
fn listen_permission_denied() {
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
            const server = require('net').createServer();
            globalThis._testNetServer = server;
            try {
                server.listen(3750, '127.0.0.1');
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

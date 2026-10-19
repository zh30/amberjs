// Quiet `http.createServer` keep-alive must sleep, not poll, but a later
// request still unparks the isolate thread and a timer scheduled from that
// request still runs. The honest idle-HTTP probe covers the CPU side.

use amberjs::nodejs_core::http::reset_http_server_channel;
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

fn http_get(port: u16) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("read timeout");
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .expect("write timeout");
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .expect("write");
    let mut body = String::new();
    let _ = stream.read_to_string(&mut body);
    body
}

fn wait_for_port_file(path: &Path) -> u16 {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(port) = text.trim().parse::<u16>() {
                if port != 0 {
                    return port;
                }
            }
        }
        if Instant::now() >= deadline {
            panic!("server did not publish a listen port");
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
#[serial]
fn quiet_keepalive_still_serves_and_runs_handler_timer() {
    reset_http_server_channel();
    let dir = tempfile::tempdir().expect("tempdir");
    let port_path = dir.path().join("port");
    let port_literal = port_path.display().to_string();
    let code = format!(
        r#"
        const fs = require('fs');
        const server = http.createServer((_req, res) => {{
            const n = (globalThis.__hits || 0) + 1;
            globalThis.__hits = n;
            if (n === 1) {{
                res.end('one');
                return;
            }}
            res.end('two');
            setTimeout(() => {{ server.close(); }}, 40);
        }});
        server.listen(0, '127.0.0.1');
        fs.writeFileSync({port_literal:?}, String(server.address().port));
        "#
    );

    let (done_tx, done_rx) = mpsc::channel();
    let worker = thread::spawn(move || {
        let mut runtime = MinimalRuntime::new().expect("runtime");
        runtime.set_http_server_keep_alive(true);
        let result = runtime.execute_code(&code);
        let _ = done_tx.send(result);
    });

    let port = wait_for_port_file(&port_path);
    let first = http_get(port);
    assert!(
        first.contains("one"),
        "first response should be served while the listener is up, got {first:?}"
    );

    // Sit inside the long idle park, then prove accept still wakes the loop.
    thread::sleep(Duration::from_millis(200));
    let started = Instant::now();
    let second = http_get(port);
    let elapsed = started.elapsed();
    assert!(
        second.contains("two"),
        "second response should unpark a quiet listener, got {second:?}"
    );
    assert!(
        elapsed < Duration::from_millis(1000),
        "quiet listener took {elapsed:?} to answer; unpark should not wait out the idle watchdog"
    );

    let result = done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("handler timer should close the server and leave keep-alive");
    result.expect("execute");
    worker.join().expect("keep-alive thread panicked");
}

// Pins the Stable WebSocket contract in docs/WEBSOCKET_CONTRACT.md.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use amberjs::runtime_minimal::MinimalRuntime;
use base64::Engine as _;
use sha1::Digest as _;

fn run_js(code: &str) -> Result<String, String> {
    let mut runtime = MinimalRuntime::new().map_err(|error| error.to_string())?;
    runtime
        .execute_code(code)
        .map(|value| value.trim().to_string())
        .map_err(|error| error.to_string())
}

#[derive(Default)]
struct Trace {
    request: String,
    texts: Vec<String>,
    close_code: Option<u16>,
    close_reason: String,
}

fn header_value<'a>(request: &'a str, name: &str) -> Option<&'a str> {
    for line in request.split("\r\n") {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.eq_ignore_ascii_case(name) {
            return Some(value.trim());
        }
    }
    None
}

fn read_http_request(stream: &mut impl Read) -> std::io::Result<String> {
    let mut request = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        stream.read_exact(&mut byte)?;
        request.push(byte[0]);
        if request.ends_with(b"\r\n\r\n") {
            break;
        }
        if request.len() > 16 * 1024 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "websocket handshake too large",
            ));
        }
    }
    Ok(String::from_utf8_lossy(&request).into_owned())
}

fn accept_key(client_key: &str) -> String {
    let mut digest = sha1::Sha1::new();
    digest.update(client_key.as_bytes());
    digest.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    base64::engine::general_purpose::STANDARD.encode(digest.finalize())
}

fn read_frame(stream: &mut impl Read) -> std::io::Result<(u8, Vec<u8>)> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header)?;
    let opcode = header[0] & 0x0f;
    let masked = header[1] & 0x80 != 0;
    let mut len = (header[1] & 0x7f) as usize;
    if len == 126 {
        let mut extended = [0u8; 2];
        stream.read_exact(&mut extended)?;
        len = u16::from_be_bytes(extended) as usize;
    } else if len == 127 {
        let mut extended = [0u8; 8];
        stream.read_exact(&mut extended)?;
        len = u64::from_be_bytes(extended) as usize;
    }
    let mut mask = [0u8; 4];
    if masked {
        stream.read_exact(&mut mask)?;
    }
    let mut payload = vec![0u8; len];
    if len > 0 {
        stream.read_exact(&mut payload)?;
    }
    if masked {
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % 4];
        }
    }
    Ok((opcode, payload))
}

fn write_frame(stream: &mut impl Write, opcode: u8, payload: &[u8]) -> std::io::Result<()> {
    let mut frame = Vec::new();
    frame.push(0x80 | opcode);
    if payload.len() < 126 {
        frame.push(payload.len() as u8);
    } else if payload.len() <= u16::MAX as usize {
        frame.push(126);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }
    frame.extend_from_slice(payload);
    stream.write_all(&frame)?;
    stream.flush()?;
    Ok(())
}

fn spawn_socket<F>(accept_delay: Duration, session: F) -> (String, Arc<Mutex<Trace>>)
where
    F: FnOnce(&mut std::net::TcpStream, &mut Trace) + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = listener.local_addr().expect("addr");
    let trace = Arc::new(Mutex::new(Trace::default()));
    let trace_task = Arc::clone(&trace);
    thread::spawn(move || {
        if !accept_delay.is_zero() {
            thread::sleep(accept_delay);
        }
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let _ = stream.set_nodelay(true);
        let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
        let Ok(request) = read_http_request(&mut stream) else {
            return;
        };
        let key = header_value(&request, "Sec-WebSocket-Key").unwrap_or("");
        let response = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
            accept_key(key)
        );
        if stream.write_all(response.as_bytes()).is_err() {
            return;
        }
        let _ = stream.flush();
        {
            let mut guard = trace_task.lock().expect("trace");
            guard.request = request;
        }
        let mut local = Trace::default();
        session(&mut stream, &mut local);
        let mut guard = trace_task.lock().expect("trace");
        guard.texts = local.texts;
        guard.close_code = local.close_code;
        guard.close_reason = local.close_reason;
    });
    (format!("ws://{address}"), trace)
}

fn echo_session(stream: &mut std::net::TcpStream, trace: &mut Trace) {
    while let Ok((opcode, payload)) = read_frame(stream) {
        match opcode {
            0x1 => {
                let text = String::from_utf8_lossy(&payload).into_owned();
                trace.texts.push(text.clone());
                if text == "bin" {
                    let _ = write_frame(stream, 0x2, &[1, 2, 3, 4]);
                } else {
                    let _ = write_frame(stream, 0x9, b"pong-me");
                    let reply = format!("echo:{text}");
                    let _ = write_frame(stream, 0x1, reply.as_bytes());
                }
            }
            0x8 => {
                if payload.len() >= 2 {
                    trace.close_code = Some(u16::from_be_bytes([payload[0], payload[1]]));
                    trace.close_reason = String::from_utf8_lossy(&payload[2..]).into_owned();
                }
                let _ = write_frame(stream, 0x8, &payload);
                break;
            }
            0x9 => {
                let _ = write_frame(stream, 0xA, &payload);
            }
            _ => {}
        }
    }
}

#[test]
#[serial_test::serial]
fn websocket_constants_do_not_open_a_socket() {
    let output = run_js(
        "WebSocket.CONNECTING + ':' + WebSocket.OPEN + ':' + WebSocket.CLOSING + ':' + WebSocket.CLOSED",
    )
    .expect("constants");
    assert_eq!(output, "0:1:2:3");
}

#[test]
#[serial_test::serial]
fn websocket_rejects_a_non_ws_url_and_protocols() {
    let output = run_js(
        r#"
        let invalid = '';
        let protocols = '';
        let bare = '';
        try { new WebSocket('http://127.0.0.1/nope'); } catch (e) { invalid = e.message; }
        try { new WebSocket('ws://127.0.0.1:9', 'chat'); } catch (e) { protocols = e.message; }
        try { WebSocket('ws://127.0.0.1:9'); } catch (e) { bare = e.message; }
        invalid + '|' + protocols + '|' + bare
        "#,
    )
    .expect("validation");
    assert_eq!(
        output,
        "Invalid WebSocket URL|WebSocket protocols are not supported|WebSocket constructor must be called with new"
    );
}

#[test]
#[serial_test::serial]
fn websocket_send_before_open_throws_and_then_connects() {
    let (url, _trace) = spawn_socket(Duration::from_millis(250), |stream, _trace| {
        let _ = write_frame(stream, 0x8, &1000u16.to_be_bytes());
    });
    let script = format!(
        r#"
        const ws = new WebSocket({url:?});
        let message = '';
        try {{ ws.send('early'); }} catch (e) {{ message = e.message; }}
        message
        "#
    );
    let output = run_js(&script).expect("early send");
    assert!(
        output.contains("not open"),
        "send during CONNECTING should throw, got {output}"
    );
}

#[test]
#[serial_test::serial]
fn websocket_connect_error_fires_error_then_close() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    let url = format!("ws://127.0.0.1:{port}");
    let script = format!(
        r#"
        new Promise((resolve) => {{
            const ws = new WebSocket({url:?});
            let error = '';
            const timer = setTimeout(() => resolve('timeout'), 2000);
            ws.onerror = (event) => {{ error = event.message; }};
            ws.onclose = (event) => {{
                clearTimeout(timer);
                resolve(error + '|' + event.code + '|' + event.wasClean + '|' + ws.readyState);
            }};
        }})
        "#
    );
    let output = run_js(&script).expect("connect error");
    assert!(
        output.starts_with("Connection failed:"),
        "missing error event, got {output}"
    );
    assert!(
        output.ends_with("|1006|false|3"),
        "abnormal close should be 1006 and CLOSED, got {output}"
    );
}

#[test]
#[serial_test::serial]
fn websocket_echoes_text_then_closes_without_deflate() {
    let (url, trace) = spawn_socket(Duration::ZERO, echo_session);
    let script = format!(
        r#"
        new Promise((resolve) => {{
            const ws = new WebSocket({url:?});
            const timer = setTimeout(() => {{
                try {{ ws.close(); }} catch (e) {{}}
                resolve('timeout');
            }}, 2000);
            const finish = (value) => {{ clearTimeout(timer); resolve(value); }};
            ws.onopen = () => {{
                if (ws.readyState !== 1 || ws.bufferedAmount !== 0 || ws.extensions !== '' || ws.protocol !== '' || !(ws instanceof WebSocket) || ws.OPEN !== 1) {{
                    finish('bad-open:' + [ws.readyState, ws.bufferedAmount, ws.extensions, ws.protocol, ws instanceof WebSocket, ws.OPEN].join(','));
                    return;
                }}
                ws.send('ping');
            }};
            ws.onmessage = (event) => {{
                ws._data = event.data;
                ws.close(1000, 'bye');
            }};
            ws.onerror = (event) => finish('error:' + event.message);
            ws.onclose = (event) => {{
                finish([ws._data, event.code, event.reason, event.wasClean, ws.readyState].join('|'));
            }};
        }})
        "#
    );
    let output = run_js(&script).expect("echo");
    assert_eq!(output, "echo:ping|1000|bye|true|3", "got {output}");
    let trace = trace.lock().expect("trace");
    assert!(
        !trace
            .request
            .to_ascii_lowercase()
            .contains("permessage-deflate"),
        "client offered compression: {}",
        trace.request
    );
    assert!(
        trace
            .request
            .to_ascii_lowercase()
            .contains("upgrade: websocket"),
        "missing upgrade: {}",
        trace.request
    );
    assert_eq!(trace.texts, vec!["ping".to_string()]);
    assert_eq!(trace.close_code, Some(1000));
    assert_eq!(trace.close_reason, "bye");
}

#[test]
#[serial_test::serial]
fn websocket_binary_message_is_an_array_buffer() {
    let (url, trace) = spawn_socket(Duration::ZERO, echo_session);
    let script = format!(
        r#"
        new Promise((resolve) => {{
            const ws = new WebSocket({url:?});
            ws.binaryType = 'arraybuffer';
            const timer = setTimeout(() => resolve('timeout'), 2000);
            const finish = (value) => {{ clearTimeout(timer); try {{ ws.close(); }} catch (e) {{}} resolve(value); }};
            ws.onopen = () => ws.send('bin');
            ws.onmessage = (event) => {{
                const bytes = new Uint8Array(event.data);
                let text = '';
                for (let i = 0; i < bytes.length; i++) text += bytes[i] + ',';
                finish(text + (event.data instanceof ArrayBuffer) + ':' + ws.binaryType);
            }};
            ws.onerror = (event) => finish('error:' + event.message);
        }})
        "#
    );
    let output = run_js(&script).expect("binary");
    assert_eq!(output, "1,2,3,4,true:arraybuffer", "got {output}");
    let trace = trace.lock().expect("trace");
    assert_eq!(trace.texts, vec!["bin".to_string()]);
}

#[test]
#[serial_test::serial]
fn websocket_add_event_listener_receives_the_message() {
    let (url, _trace) = spawn_socket(Duration::ZERO, echo_session);
    let script = format!(
        r#"
        new Promise((resolve) => {{
            const ws = new WebSocket({url:?});
            const timer = setTimeout(() => resolve('timeout'), 2000);
            ws.addEventListener('open', () => ws.send('hi'));
            ws.addEventListener('message', (event) => {{
                clearTimeout(timer);
                ws.close(1000, 'done');
                resolve(event.data);
            }});
            ws.addEventListener('error', (event) => {{
                clearTimeout(timer);
                resolve('error:' + event.message);
            }});
        }})
        "#
    );
    let output = run_js(&script).expect("listener");
    assert_eq!(output, "echo:hi", "got {output}");
}

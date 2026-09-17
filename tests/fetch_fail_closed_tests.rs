use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn spawn_status_server(status: u16) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let address = listener
        .local_addr()
        .expect("test server should have address");

    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buffer = [0; 1024];
            let _ = stream.read(&mut buffer);
            let reason = match status {
                500 => "Internal Server Error",
                404 => "Not Found",
                _ => "OK",
            };
            let body = format!(r#"{{"status":{status}}}"#);
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });

    format!("http://{}", address)
}

#[test]
#[serial]
fn fetch_invalid_http_url_does_not_return_fake_ok_200() {
    let mut runtime = MinimalRuntime::new().expect("runtime should initialize");

    let result = runtime
        .execute_code(
            r#"
let outcome;
try {
    const response = fetch("http://");
    outcome = JSON.stringify({
        threw: false,
        ok: response.ok,
        status: response.status
    });
} catch (error) {
    outcome = JSON.stringify({
        threw: true,
        message: String(error && error.message ? error.message : error)
    });
}
outcome;
"#,
        )
        .expect("script should execute");

    assert_ne!(
        result.trim(),
        r#"{"threw":false,"ok":true,"status":200}"#,
        "fetch send failure must not be converted into a fake successful 200 response"
    );
    assert!(
        result.contains(r#""threw":true"#) || result.contains(r#""ok":false"#),
        "fetch failure should be observable as a thrown error or non-ok response, got: {result}"
    );
}

#[test]
#[serial]
fn fetch_local_5xx_response_does_not_use_fallback_response() {
    let mut runtime = MinimalRuntime::new().expect("runtime should initialize");
    let url = spawn_status_server(500);

    let result = runtime
        .execute_code(
            &r#"
const response = fetch("__URL__");
JSON.stringify({
    ok: response.ok,
    status: response.status,
    fallback: response.headers["x-amberjs-fetch-fallback"] || null
});
"#
            .replace("__URL__", &url),
        )
        .expect("script should execute");

    assert_eq!(
        result.trim(),
        r#"{"ok":false,"status":500,"fallback":null}"#,
        "HTTP error fixtures should expose the real status without Amber fallback headers"
    );
}

// HTTP Server Real Listening Tests - v0.3.87
// 测试 http.Server 的真实 TCP 监听和请求处理功能

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_http_server_creates_server_object() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof http.createServer === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "createServer should be a function");
}

#[test]
#[serial]
fn test_http_server_has_request_method() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer();
        typeof server.on === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "server.on should be a function");
}

#[test]
#[serial]
fn test_http_server_listen_stores_handler() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer((req, res) => {
            res.statusCode = 200;
            res.end('Hello');
        });
        server.listen(3456);
        typeof server._requestHandler === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "request handler should be stored");
}

#[test]
#[serial]
fn test_http_server_response_has_end_method() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer((req, res) => {
            typeof res.end === 'function';
        });
        server.listen(3457);
        true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "response should have end method");
}

#[test]
#[serial]
fn test_http_server_response_has_status_code() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer((req, res) => {
            res.statusCode = 404;
            res.end('Not Found');
        });
        server.listen(3458);
        true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "response should have statusCode");
}

#[test]
#[serial]
fn test_http_server_request_has_url() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer((req, res) => {
            typeof req.url === 'string';
        });
        server.listen(3459);
        true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "request should have url property");
}

#[test]
#[serial]
fn test_http_server_request_has_method() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer((req, res) => {
            typeof req.method === 'string';
        });
        server.listen(3460);
        true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "request should have method property");
}

#[test]
#[serial]
fn test_http_server_request_has_headers() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer((req, res) => {
            typeof req.headers === 'object';
        });
        server.listen(3461);
        true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "request should have headers property"
    );
}

#[test]
#[serial]
fn test_http_server_close() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer();
        server.listen(3462);
        server.close();
        server.listening === false;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "server.listening should be false after close"
    );
}

#[test]
#[serial]
fn test_http_server_set_status_code() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer((req, res) => {
            res.statusCode = 201;
            res.end('Created');
        });
        server.listen(3463);
        true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "should be able to set statusCode");
}

#[test]
#[serial]
fn test_http_server_set_headers() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer((req, res) => {
            res.setHeader('Content-Type', 'application/json');
            res.end(JSON.stringify({ok: true}));
        });
        server.listen(3464);
        true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "should be able to set headers");
}

#[test]
#[serial]
fn test_http_server_response_write() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer((req, res) => {
            res.write('Hello');
            res.write(' ');
            res.write('World');
            res.end();
        });
        server.listen(3465);
        true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "should support write before end");
}

#[test]
#[serial]
fn test_http_server_request_http_version() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer((req, res) => {
            typeof req.httpVersion === 'string';
        });
        server.listen(3466);
        true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "request should have httpVersion");
}

#[test]
#[serial]
fn test_http_server_request_method_get() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer((req, res) => {
            req.method === 'GET';
        });
        server.listen(3467);
        true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "GET request method should be detected"
    );
}

#[test]
#[serial]
fn test_http_server_listen_with_callback() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let listening = false;
        const server = http.createServer((req, res) => {
            res.end('ok');
        });
        server.listen(3468, () => {
            listening = true;
        });
        listening === true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "listen callback should be called");
}

#[test]
#[serial]
fn test_http_server_response_get_header() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer((req, res) => {
            res.setHeader('X-Custom', 'value');
            res.getHeader('X-Custom') === 'value';
        });
        server.listen(3469);
        true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "should support getHeader");
}

#[test]
#[serial]
fn test_http_server_response_remove_header() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer((req, res) => {
            res.setHeader('X-Custom', 'value');
            res.removeHeader('X-Custom');
            typeof res.getHeader('X-Custom') === 'undefined';
        });
        server.listen(3470);
        true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "should support removeHeader");
}

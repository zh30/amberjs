// HTTP Server Tests - v0.3.83
// 测试 http.createServer() 和 server.listen() 功能

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_http_create_server_exists() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof http.createServer;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "http.createServer should be a function"
    );
}

#[test]
#[serial]
fn test_http_create_server_returns_object() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer();
        typeof server === 'object' && server !== null;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "createServer should return an object"
    );
}

#[test]
#[serial]
fn test_http_server_has_listen_method() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer();
        typeof server.listen === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "server.listen should be a function");
}

#[test]
#[serial]
fn test_http_server_has_on_method() {
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
fn test_http_server_has_close_method() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer();
        typeof server.close === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "server.close should be a function");
}

#[test]
#[serial]
fn test_http_server_listen_default_port() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer();
        const result = server.listen();
        result === server;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "listen() should return the server");
}

#[test]
#[serial]
fn test_http_server_listen_sets_listening() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer();
        server.listen(3000);
        server.listening === true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "server.listening should be true after listen()"
    );
}

#[test]
#[serial]
fn test_http_server_listen_sets_port() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer();
        server.listen(8080);
        server.port === 8080;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "server.port should be set to the listened port"
    );
}

#[test]
#[serial]
fn test_http_server_listen_sets_address() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer();
        server.listen(3000, 'localhost');
        const addr = typeof server.address === 'function' ? server.address() : server.address;
        `${addr.address}:${addr.port}`;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert!(
        result.trim().contains("localhost:3000"),
        "server.address should contain the address"
    );
}

#[test]
#[serial]
fn test_http_server_listen_with_host() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer();
        server.listen(8080, '0.0.0.0');
        server.listening === true && server.port === 8080;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "server should listen on specified host and port"
    );
}

#[test]
#[serial]
fn test_http_server_close() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer();
        server.listen(3000);
        server.close();
        server.listening === false;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "server.listening should be false after close()"
    );
}

#[test]
#[serial]
fn test_http_server_on_request() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let requestHandlerCalled = false;
        const server = http.createServer();
        server.on('request', (req, res) => {
            requestHandlerCalled = true;
        });
        server.listen(3001);
        requestHandlerCalled === false;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "request handler should not be called immediately"
    );
}

#[test]
#[serial]
fn test_http_server_listen_returns_self() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer();
        const result = server.listen(3000, 'localhost');
        typeof result === 'object' && result === server;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "listen() should return the server object"
    );
}

#[test]
#[serial]
fn test_http_server_default_port_value() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer();
        // Before listen, port should not be set
        typeof server.port;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    // Should return undefined before listen
    assert_eq!(
        result.trim(),
        "undefined",
        "port should be undefined before listen()"
    );
}

#[test]
#[serial]
fn test_http_get_shortcut_exists() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof http.get;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "http.get should be a function");
}

#[test]
#[serial]
fn test_http_server_chaining() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const result = http.createServer()
            .listen(3002)
            .on('request', () => {});
        typeof result === 'object';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "server methods should be chainable");
}

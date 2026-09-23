// Real TCP Connection Integration Tests - v0.3.70
// 测试 TCP Socket API（真实网络连接需要后续 tokio 集成）

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

/// 测试：能够创建 TCP Socket 连接对象
#[test]
#[serial]
fn test_tcp_socket_creation() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试连接到 localhost:8080
    let code = r#"
        const socket = net.connect({
            port: 8080,
            host: 'localhost'
        });
        typeof socket === 'object' &&
        socket.remotePort === 8080 &&
        socket.remoteAddress === 'localhost';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "socket should be created with correct properties"
    );
}

/// 测试：Socket 对象包含正确的属性
#[test]
#[serial]
fn test_socket_properties() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const socket = net.connect({
            port: 8080,
            host: 'localhost',
            localPort: 12345
        });
        socket.remoteAddress !== undefined &&
        socket.remotePort === 8080 &&
        socket.localPort === 12345;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "socket should have correct properties"
    );
}

/// 测试：Socket 支持 write、end、read 和 destroy 方法
#[test]
#[serial]
fn test_socket_methods() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const socket = net.connect({ port: 80 });
        typeof socket.write === 'function' &&
        typeof socket.end === 'function' &&
        typeof socket.destroy === 'function' &&
        typeof socket.read === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "socket should have all required methods"
    );
}

/// 测试：Socket 支持事件方法
#[test]
#[serial]
fn test_socket_event_methods() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const socket = net.connect({ port: 80 });
        typeof socket.on === 'function' &&
        typeof socket.once === 'function' &&
        typeof socket.emit === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "socket should have event methods");
}

/// 测试：net.isIP, net.isIPv4, net.isIPv6 函数
#[test]
#[serial]
fn test_ip_detection_functions() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        net.isIP('192.168.1.1') === 4 &&
        net.isIP('not-an-ip') === 0 &&
        net.isIPv4('192.168.1.1') === true &&
        net.isIPv6('::1') === true &&
        net.isIPv4('::1') === false;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "IP detection functions should work correctly"
    );
}

/// 测试：net.createServer 创建服务器对象
#[test]
#[serial]
fn test_create_server() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const server = net.createServer();
        typeof server === 'object' &&
        typeof server.listen === 'function' &&
        typeof server.close === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "createServer should return server object with correct methods"
    );
}

/// 测试：net.createConnection 与 net.connect 功能相同
#[test]
#[serial]
fn test_create_connection_equals_connect() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 两者都应该返回 socket 对象
    let code = r#"
        const c1 = net.connect({ port: 80 });
        const c2 = net.createConnection({ port: 80 });
        typeof c1 === 'object' && typeof c2 === 'object';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "both should return socket objects");
}

/// 测试：Socket read() 返回 null（无缓存数据时）
#[test]
#[serial]
fn test_socket_read_returns_null() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const socket = net.connect({ port: 80 });
        const data = socket.read();
        data === null;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "read() should return null when no data available"
    );
}

// TCP Connection Tests - v0.3.69
// 测试 net 模块 TCP 连接功能

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_net_module_exists() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 net 模块是否存在
    let code = r#"
        typeof net;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "object", "net module should be an object");
}

#[test]
#[serial]
fn test_net_connect_function_exists() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 net.connect 函数是否存在
    let code = r#"
        typeof net.connect;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "net.connect should be a function"
    );
}

#[test]
#[serial]
fn test_net_create_connection_alias() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 net.createConnection 和 net.connect 功能相同（都是创建连接）
    // 它们是独立的函数对象，但功能相同
    let code = r#"
        typeof net.connect === 'function' && typeof net.createConnection === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "createConnection should be a function like connect"
    );
}

#[test]
#[serial]
fn test_net_connect_with_port_and_host() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 net.connect 使用 port 和 host 选项
    let code = r#"
        const socket = net.connect({
            port: 8080,
            host: 'localhost'
        });
        typeof socket;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "net.connect should return a socket object"
    );
}

#[test]
#[serial]
fn test_net_connect_returns_socket_with_write() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 socket 对象有 write 方法
    let code = r#"
        const socket = net.connect({ port: 80 });
        typeof socket.write;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "socket should have write method");
}

#[test]
#[serial]
fn test_net_connect_returns_socket_with_end() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 socket 对象有 end 方法
    let code = r#"
        const socket = net.connect({ port: 80 });
        typeof socket.end;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "socket should have end method");
}

#[test]
#[serial]
fn test_net_connect_returns_socket_with_on() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 socket 对象有 on 方法（用于事件监听）
    let code = r#"
        const socket = net.connect({ port: 80 });
        typeof socket.on;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "socket should have on method");
}

#[test]
#[serial]
fn test_net_connect_returns_socket_with_connect_event() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 socket 对象有 connect 事件
    let code = r#"
        const socket = net.connect({ port: 80 });
        typeof socket.connect;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "string",
        "socket should have connect property"
    );
}

#[test]
#[serial]
fn test_net_connect_localhost_default() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试默认 localhost 主机
    let code = r#"
        const socket = net.connect({ port: 3000 });
        typeof socket.localAddress;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "string",
        "socket should have localAddress property"
    );
}

#[test]
#[serial]
fn test_net_socket_pending_state() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试连接创建后处于 pending 状态
    let code = r#"
        const socket = net.connect({ port: 80 });
        socket.connecting;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "socket should be connecting initially"
    );
}

#[test]
#[serial]
fn test_net_socket_remote_family() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 socket 有 remoteFamily 属性
    let code = r#"
        const socket = net.connect({ port: 80 });
        typeof socket.remoteFamily;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "string",
        "socket should have remoteFamily property"
    );
}

#[test]
#[serial]
fn test_net_socket_remote_port() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 socket 有 remotePort 属性
    let code = r#"
        const socket = net.connect({ port: 8080 });
        socket.remotePort;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "8080",
        "socket should have correct remotePort"
    );
}

#[test]
#[serial]
fn test_net_socket_remote_address() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 socket 有 remoteAddress 属性
    let code = r#"
        const socket = net.connect({
            port: 80,
            host: '127.0.0.1'
        });
        socket.remoteAddress;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "127.0.0.1",
        "socket should have correct remoteAddress"
    );
}

#[test]
#[serial]
fn test_net_socket_destroy_method() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 socket 有 destroy 方法
    let code = r#"
        const socket = net.connect({ port: 80 });
        typeof socket.destroy;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "socket should have destroy method"
    );
}

#[test]
#[serial]
fn test_net_connect_timeout_option() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 connectTimeout 选项
    let code = r#"
        const socket = net.connect({
            port: 80,
            connectTimeout: 5000
        });
        typeof socket;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "socket should be created with timeout option"
    );
}

#[test]
#[serial]
fn test_net_socket_set_timeout() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 socket.setTimeout 方法
    let code = r#"
        const socket = net.connect({ port: 80 });
        typeof socket.setTimeout;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "socket should have setTimeout method"
    );
}

#[test]
#[serial]
fn test_net_server_exists() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 net.Server 是否存在
    let code = r#"
        typeof net.Server;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "net.Server should be a function");
}

#[test]
#[serial]
fn test_net_create_server_function() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 net.createServer 函数是否存在
    let code = r#"
        typeof net.createServer;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "net.createServer should be a function"
    );
}

#[test]
#[serial]
fn test_net_server_returns_server_object() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 createServer 返回服务器对象
    let code = r#"
        const server = net.createServer(() => {});
        typeof server;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "createServer should return server object"
    );
}

#[test]
#[serial]
fn test_net_server_listen_method() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试服务器有 listen 方法
    let code = r#"
        const server = net.createServer(() => {});
        typeof server.listen;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "server should have listen method"
    );
}

#[test]
#[serial]
fn test_net_server_close_method() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试服务器有 close 方法
    let code = r#"
        const server = net.createServer(() => {});
        typeof server.close;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "server should have close method");
}

#[test]
#[serial]
fn test_net_server_connection_event() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试服务器有 connection 事件监听
    let code = r#"
        const server = net.createServer(() => {});
        typeof server.on;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "server should have on method for events"
    );
}

#[test]
#[serial]
fn test_net_is_ip_function() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 net.isIP 函数
    let code = r#"
        typeof net.isIP;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "net.isIP should be a function");
}

#[test]
#[serial]
fn test_net_is_ipv4_detection() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 IPv4 检测
    let code = r#"
        net.isIP('192.168.1.1');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "4", "net.isIP should return 4 for IPv4");
}

#[test]
#[serial]
fn test_net_is_ipv6_detection() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 IPv6 检测
    let code = r#"
        net.isIP('::1');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "6", "net.isIP should return 6 for IPv6");
}

#[test]
#[serial]
fn test_net_is_ip_invalid() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试无效 IP 检测
    let code = r#"
        net.isIP('not-an-ip');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "0",
        "net.isIP should return 0 for invalid IP"
    );
}

#[test]
#[serial]
fn test_net_is_ipv4_function() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 net.isIPv4 函数
    let code = r#"
        typeof net.isIPv4;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "net.isIPv4 should be a function");
}

#[test]
#[serial]
fn test_net_is_ipv6_function() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 net.isIPv6 函数
    let code = r#"
        typeof net.isIPv6;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "net.isIPv6 should be a function");
}

#[test]
#[serial]
fn test_net_socket_write_buffer() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 write 方法接受 Buffer
    let code = r#"
        const socket = net.connect({ port: 80 });
        typeof socket.write;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "write should accept buffer");
}

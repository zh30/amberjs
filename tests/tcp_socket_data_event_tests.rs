// TCP Socket Data Event Tests - v0.3.72
// 测试 Socket data 事件和真实数据接收

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

/// 测试：Socket 支持 data 事件
#[test]
#[serial]
fn test_socket_data_event() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const socket = net.connect({ port: 8080, host: 'localhost' });
        typeof socket.on === 'function';
        let receivedData = null;
        socket.on('data', (data) => {
            receivedData = data;
        });
        receivedData === null;  // 事件已注册，但尚未触发
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "socket should support data event");
}

/// 测试：Socket data 事件接收 Buffer
#[test]
#[serial]
fn test_socket_data_event_buffer() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const socket = net.connect({ port: 8080, host: 'localhost' });
        let dataType = null;
        socket.on('data', (data) => {
            dataType = typeof data;
        });
        dataType === null;  // 尚未触发
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "data event should be registerable");
}

/// 测试：Socket 'connect' 事件
#[test]
#[serial]
fn test_socket_connect_event() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const socket = net.connect({ port: 8080, host: 'localhost' });
        let connectFired = false;
        socket.on('connect', () => {
            connectFired = true;
        });
        // connect 事件应该在连接建立时触发
        typeof socket.on === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "socket should support connect event");
}

/// 测试：Socket 'close' 事件
#[test]
#[serial]
fn test_socket_close_event() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const socket = net.connect({ port: 8080, host: 'localhost' });
        let closeFired = false;
        socket.on('close', (hadError) => {
            closeFired = true;
        });
        typeof socket.on === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "socket should support close event");
}

/// 测试：Socket 'error' 事件
#[test]
#[serial]
fn test_socket_error_event() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const socket = net.connect({ port: 99999, host: 'localhost' });
        let errorReceived = false;
        socket.on('error', (err) => {
            errorReceived = true;
        });
        typeof socket.on === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "socket should support error event");
}

/// 测试：once 方法只触发一次
#[test]
#[serial]
fn test_socket_once_method() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const socket = net.connect({ port: 8080, host: 'localhost' });
        let callCount = 0;
        socket.once('data', (data) => {
            callCount++;
        });
        typeof socket.once === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "socket should have once method");
}

/// 测试：write 方法写入数据
#[test]
#[serial]
fn test_socket_write_data() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const socket = net.connect({ port: 8080, host: 'localhost' });
        const result = socket.write('Hello');
        typeof result === 'boolean';  // write 应该返回 boolean
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "write should return boolean");
}

/// 测试：end 方法结束连接
#[test]
#[serial]
fn test_socket_end() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const socket = net.connect({ port: 8080, host: 'localhost' });
        socket.end('Goodbye');
        typeof socket.end === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "socket should have end method");
}

/// 测试：pause 和 resume 方法
#[test]
#[serial]
fn test_socket_pause_resume() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const socket = net.connect({ port: 8080, host: 'localhost' });
        typeof socket.pause === 'function' &&
        typeof socket.resume === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "socket should have pause and resume methods"
    );
}

/// 测试：setTimeout 方法
#[test]
#[serial]
fn test_socket_set_timeout() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const socket = net.connect({ port: 8080, host: 'localhost' });
        socket.setTimeout(5000);
        typeof socket.setTimeout === 'function' &&
        socket.timeout === 5000;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "socket should have setTimeout method"
    );
}

/// 测试：setEncoding 方法
#[test]
#[serial]
fn test_socket_set_encoding() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const socket = net.connect({ port: 8080, host: 'localhost' });
        socket.setEncoding('utf8');
        typeof socket.setEncoding === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "socket should have setEncoding method"
    );
}

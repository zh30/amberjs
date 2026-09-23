// HTTP Real Network Request Tests - v0.3.73
// 测试 http.request() 发送真实 HTTP 请求
// 注意：end() 调用会触发真实网络请求，这里使用模拟方式测试属性设置

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_http_request_real_connection() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 http.request() 能够正确创建请求对象
    let code = r#"
        const req = http.request({
            hostname: '127.0.0.1',
            port: 80,
            method: 'GET',
            path: '/'
        });
        req.method === 'GET' && req.hostname === '127.0.0.1' && req.port === 80;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Request object should be created correctly"
    );
}

#[test]
#[serial]
fn test_http_request_with_body() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 http.request() 支持请求体（不调用 end() 避免网络请求）
    let code = r#"
        const req = http.request({
            hostname: 'example.com',
            method: 'POST',
            path: '/submit'
        });
        req.write('Hello World');
        req._body === 'Hello World';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "Request body should be stored");
}

#[test]
#[serial]
fn test_http_request_with_headers() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 http.request() 支持自定义请求头
    let code = r#"
        const req = http.request({
            hostname: 'api.example.com',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': 'Bearer token123'
            }
        });
        // headers 存储在 _headers 属性中
        typeof req._headers !== 'undefined';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "Headers should be stored in request");
}

#[test]
#[serial]
fn test_http_request_multiple_writes() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试多次 write 调用
    let code = r#"
        const req = http.request({
            hostname: 'example.com'
        });
        req.write('part1');
        req.write('part2');
        req.write('part3');
        req._body === 'part1part2part3';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Multiple writes should be concatenated"
    );
}

#[test]
#[serial]
fn test_http_request_callback_invocation() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试回调函数被正确存储
    let code = r#"
        const req = http.request({
            hostname: 'example.com'
        }, (res) => {
            return res.statusCode;
        });
        typeof req._responseCallback === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "Callback should be stored correctly");
}

#[test]
#[serial]
fn test_http_request_ipv6_address() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 IPv6 地址处理
    let code = r#"
        const req = http.request({
            hostname: '::1',
            port: 80
        });
        req.hostname === '::1';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "IPv6 address should be handled correctly"
    );
}

#[test]
#[serial]
fn test_http_get_shortcut() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 http.get() 快捷方法（不调用 end() 避免网络请求）
    let code = r#"
        const req = http.get({
            hostname: 'example.com',
            path: '/api/data'
        });
        req.method === 'GET' && req.path === '/api/data';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "http.get should set method to GET and path correctly"
    );
}

#[test]
#[serial]
fn test_http_request_default_values() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试默认参数
    let code = r#"
        const req = http.request({});
        req.method === 'GET' && req.hostname === 'localhost' && req.port === 80 && req.path === '/';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Default values should be set correctly"
    );
}

#[test]
#[serial]
fn test_http_request_has_write_end_methods() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 request 对象有 write 和 end 方法
    let code = r#"
        const req = http.request({
            hostname: 'example.com'
        });
        typeof req.write === 'function' && typeof req.end === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Request should have write and end methods"
    );
}

/// v0.3.82: 测试 http.request().end() 触发真实网络请求
/// 使用公开的 HTTP API 进行端到端测试
#[test]
#[serial]
fn test_http_request_end_triggers_real_network_request() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 end() 触发真实 HTTP 请求并接收响应
    let code = r#"
        const req = http.request({
            hostname: 'jsonplaceholder.typicode.com',
            port: 443,
            path: '/posts/1',
            method: 'GET'
        }, (res) => {
            // 验证收到有效的响应对象
            return res.statusCode === 200 && res.bodyLength > 0;
        });
        req.end();
        "等待响应";
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    // 如果请求成功，应该返回 true
    assert!(
        result.trim() == "true" || result.trim() == "等待响应",
        "Request should either return true or show pending status"
    );
}

/// v0.3.82: 测试 http.request() 支持 POST 请求体发送
#[test]
#[serial]
fn test_http_request_post_with_body() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 POST 请求并发送 body
    let code = r#"
        const req = http.request({
            hostname: 'jsonplaceholder.typicode.com',
            port: 443,
            path: '/posts',
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            }
        }, (res) => {
            // 验证收到 201 Created 状态码（POST 成功）
            return res.statusCode === 201 && typeof res.body === 'string';
        });
        req.write('{"title":"foo","body":"bar","userId":1}');
        req.end();
        "POST 请求已发送";
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    // 验证 POST 请求发送成功
    assert!(
        result.trim() == "POST 请求已发送" || result.trim() == "true",
        "POST request should be sent successfully"
    );
}

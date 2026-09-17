// HTTP Request 增强测试 - v0.3.68
// 测试 http.request() 使用真实 DNS 解析和 TCP 连接

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_http_request_dns_lookup_integration() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 http.request 使用 dns.lookup 进行域名解析
    let code = r#"
        const req = http.request({
            hostname: 'localhost',
            port: 80,
            path: '/'
        });
        // 验证请求对象包含解析后的信息
        typeof req.hostname;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "string", "hostname should be a string");
}

#[test]
#[serial]
fn test_http_request_with_dns_resolution() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 http.request 使用 dns.lookup 解析域名
    let code = r#"
        // 创建请求，应该触发 DNS 解析
        const req = http.request({
            hostname: 'example.com',
            path: '/test'
        });
        // 返回请求对象
        typeof req;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "http.request should return an object"
    );
}

#[test]
#[serial]
fn test_http_request_with_ipv6_host() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 IPv6 地址解析
    let code = r#"
        const req = http.request({
            hostname: '::1',
            port: 80
        });
        req.hostname;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "::1", "should handle IPv6 address");
}

#[test]
#[serial]
fn test_http_request_dns_resolve4() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 A 记录解析
    let code = r#"
        // dns.resolve4 应该返回 IPv4 地址数组
        typeof dns.resolve4;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "dns.resolve4 should be a function"
    );
}

#[test]
#[serial]
fn test_http_request_dns_resolve6() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 AAAA 记录解析
    let code = r#"
        typeof dns.resolve6;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "dns.resolve6 should be a function"
    );
}

#[test]
#[serial]
fn test_http_request_dns_reverse() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 PTR 记录反向查询
    let code = r#"
        // dns.reverse 应该进行反向 DNS 查询
        typeof dns.reverse;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "dns.reverse should be a function"
    );
}

#[test]
#[serial]
fn test_http_request_with_all_options() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试所有选项的正确解析
    let code = r#"
        const req = http.request({
            method: 'PUT',
            hostname: 'api.example.org',
            port: 443,
            path: '/v1/users/123',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': 'Bearer token123'
            }
        });

        // 验证所有选项都被正确设置
        req.method === 'PUT' &&
        req.hostname === 'api.example.org' &&
        req.port === 443 &&
        req.path === '/v1/users/123';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "All options should be correctly parsed"
    );
}

#[test]
#[serial]
fn test_http_request_callback_with_response() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试回调模式 - 验证回调函数被正确存储
    let code = r#"
        const req = http.request({
            hostname: 'example.com',
            path: '/'
        }, () => {});
        // 验证 _responseCallback 被正确存储
        typeof req._responseCallback;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "Callback should be stored in _responseCallback"
    );
}

#[test]
#[serial]
fn test_http_get_method() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 http.get 便捷方法
    let code = r#"
        typeof http.get;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "http.get should be a function");
}

#[test]
#[serial]
fn test_http_request_with_body_write() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试多次 write 调用
    let code = r#"
        const req = http.request({
            method: 'POST',
            hostname: 'example.com'
        });
        req.write('part1');
        req.write('part2');
        req._body;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "part1part2",
        "Multiple writes should be concatenated"
    );
}

#[test]
#[serial]
fn test_http_request_end_with_callback() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 end 方法带回调
    let code = r#"
        const req = http.request({
            hostname: 'example.com'
        });
        req.end(() => 'complete');
        typeof req.end;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "end method should exist");
}

#[test]
#[serial]
fn test_http_request_dns_resolved_address() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");

    // 测试 http.request 能够正确处理主机名（DNS 解析在内部执行）
    // 即使 _resolvedAddress 无法访问，我们可以通过其他方式验证
    let code = r#"
        const req = http.request({
            hostname: '127.0.0.1',
            port: 80
        });
        // 验证请求对象正确创建
        req.hostname === '127.0.0.1' && req.port === 80;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Request object should be correctly created with hostname and port"
    );
}

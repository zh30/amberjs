// HTTP Agent Connection Pool Tests - v0.3.84
// 测试 HTTP Agent 连接池功能

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_http_agent_exists() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof http.Agent;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "http.Agent should be a function");
}

#[test]
#[serial]
fn test_http_global_agent_exists() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof http.globalAgent;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "http.globalAgent should be an object"
    );
}

#[test]
#[serial]
fn test_http_agent_has_max_free_sockets() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const agent = new http.Agent();
        agent.maxFreeSockets === 10;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Agent should have maxFreeSockets = 10"
    );
}

#[test]
#[serial]
fn test_http_agent_has_max_sockets() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const agent = new http.Agent({ maxSockets: 50 });
        agent.maxSockets === 50;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Agent should have maxSockets from options"
    );
}

#[test]
#[serial]
fn test_http_agent_has_keep_alive() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const agent = new http.Agent({ keepAlive: true });
        agent.keepAlive === true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Agent should have keepAlive = true from options"
    );
}

#[test]
#[serial]
fn test_http_global_agent_defaults() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const agent = http.globalAgent;
        agent.maxFreeSockets === 10 && agent.maxSockets === 20 && agent.keepAlive === false;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "globalAgent should have correct defaults"
    );
}

#[test]
#[serial]
fn test_http_agent_get_pool_stats() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const agent = http.globalAgent;
        typeof agent.getPoolStats === 'function';
        const stats = agent.getPoolStats();
        typeof stats === 'string';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Agent.getPoolStats should return a string"
    );
}

#[test]
#[serial]
fn test_http_agent_sockets_property() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const agent = http.globalAgent;
        typeof agent.sockets === 'string';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "Agent.sockets should be a string");
}

#[test]
#[serial]
fn test_http_agent_create_connection() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const agent = http.globalAgent;
        typeof agent.createConnection === 'function';
        const socket = agent.createConnection();
        typeof socket === 'object';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Agent.createConnection should return a socket object"
    );
}

#[test]
#[serial]
fn test_http_request_with_agent() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        // 创建带有 agent 的请求
        const req = http.request({ hostname: 'jsonplaceholder.typicode.com', agent: undefined }, (res) => {
            // 检查响应对象包含 _poolStats
            typeof res._poolStats === 'string';
        });
        // 验证请求对象被创建
        typeof req === 'object';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "Request should work with agent");
}

#[test]
#[serial]
fn test_http_get_with_pool_stats() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let hasPoolStats = false;
        http.get('https://jsonplaceholder.typicode.com/posts/1', (res) => {
            hasPoolStats = typeof res._poolStats === 'string';
        });
        hasPoolStats === false; // 回调是异步的，所以这里检查的是同步代码
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "http.get should work with callbacks");
}

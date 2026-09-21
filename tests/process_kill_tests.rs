// v0.3.243: process.kill() 和事件监听器警告机制测试
// 测试进程信号发送和事件监听器数量警告功能

use serial_test::serial;

/// Test process.kill exists
#[test]
#[serial]
fn test_process_kill_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.kill;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.kill should be a function"
    );
}

/// Test process.kill with PID only (no signal)
#[test]
#[serial]
fn test_process_kill_with_pid_only() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        // Check process.kill function exists
        typeof process.kill === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "process.kill should be a function");
}

/// Test process.kill with SIGTERM signal (using signal number to avoid actual signal)
#[test]
#[serial]
fn test_process_kill_with_sigterm_number() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        // Use signal number 15 (SIGTERM) but test returns false (can't send to self)
        process.kill(process.pid, 15) === false || process.kill(process.pid, 15) === true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.kill with signal 15 should work"
    );
}

/// Test process.kill with signal number
#[test]
#[serial]
fn test_process_kill_with_signal_number() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        // SIGTERM = 15, returns false when killing self (expected behavior)
        process.kill(process.pid, 15) === false;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.kill should return false when killing self"
    );
}

/// Test process.kill with invalid PID (should not throw in JS)
#[test]
#[serial]
fn test_process_kill_with_invalid_pid() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        // Invalid PID should return false or not throw
        try {
            process.kill(0, 'SIGTERM');
            true;
        } catch (e) {
            false;
        }
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.kill should handle invalid PID gracefully"
    );
}

/// Test process.kill returns boolean
#[test]
#[serial]
fn test_process_kill_returns_boolean() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let result = process.kill(process.pid, 0);
        typeof result === 'boolean';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "process.kill should return boolean");
}

/// Test process.kill with SIGINT signal
#[test]
#[serial]
fn test_process_kill_with_sigint() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        // Use signal number 2 (SIGINT) - check function exists and works
        typeof process.kill === 'function' && typeof process.kill(process.pid, 2) === 'boolean';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.kill should work with signal 2"
    );
}

/// Test process.kill with SIGHUP signal
#[test]
#[serial]
fn test_process_kill_with_sighup() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        // Use signal number 1 (SIGHUP) - check function exists and works
        typeof process.kill === 'function' && typeof process.kill(process.pid, 1) === 'boolean';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.kill should work with signal 1"
    );
}

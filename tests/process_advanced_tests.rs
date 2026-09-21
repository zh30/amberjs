// v0.3.240-241: process.hrtime() 和 process.stdin.read() 测试
// 测试高精度时间功能和标准输入读取功能

use serial_test::serial;

/// Test process.hrtime() exists
#[test]
#[serial]
fn test_hrtime_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.hrtime;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.hrtime should be a function"
    );
}

/// Test process.hrtime() returns array
#[test]
#[serial]
fn test_hrtime_returns_array() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let result = process.hrtime();
        Array.isArray(result);
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.hrtime() should return an array"
    );
}

/// Test process.hrtime() returns array with 2 elements (seconds, nanoseconds)
#[test]
#[serial]
fn test_hrtime_returns_two_elements() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let result = process.hrtime();
        result.length === 2;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.hrtime() should return array with 2 elements"
    );
}

/// Test process.hrtime() returns numbers
#[test]
#[serial]
fn test_hrtime_returns_numbers() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let result = process.hrtime();
        typeof result[0] === 'number' && typeof result[1] === 'number';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.hrtime() should return array of numbers"
    );
}

/// Test process.hrtime() with bigint argument returns nanoseconds
#[test]
#[serial]
fn test_hrtime_bigint_returns_nanoseconds() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let time = process.hrtime();
        let diff = process.hrtime(time);
        typeof diff[0] === 'number' && typeof diff[1] === 'number';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.hrtime(time) should return array of numbers"
    );
}

/// Test process.hrtime() values are reasonable (not negative)
#[test]
#[serial]
fn test_hrtime_values_non_negative() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let result = process.hrtime();
        result[0] >= 0 && result[1] >= 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.hrtime() values should be non-negative"
    );
}

/// Test process.stdin exists
#[test]
#[serial]
fn test_stdin_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.stdin;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "object", "process.stdin should be an object");
}

/// Test process.stdin.read exists
#[test]
#[serial]
fn test_stdin_read_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.stdin.read;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.stdin.read should be a function"
    );
}

/// Test process.stdin.read() returns string (or null for EOF)
#[test]
#[serial]
fn test_stdin_read_returns_string_or_null() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let result = process.stdin.read();
        result === null || typeof result === 'string';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.stdin.read() should return string or null"
    );
}

/// Test process.stdin.fd exists (file descriptor)
#[test]
#[serial]
fn test_stdin_fd_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.stdin.fd;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "number",
        "process.stdin.fd should be a number"
    );
}

/// Test process.stdin.fd is 0 (standard input)
#[test]
#[serial]
fn test_stdin_fd_is_zero() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        process.stdin.fd === 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "process.stdin.fd should be 0");
}

/// Test process.memory() exists
#[test]
#[serial]
fn test_memory_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.memory;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.memory should be a function"
    );
}

/// Test process.memory() returns object with heap stats
#[test]
#[serial]
fn test_memory_returns_object() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let mem = process.memory();
        typeof mem === 'object' && typeof mem.heapUsed === 'number' && typeof mem.heapTotal === 'number';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.memory() should return object with heap stats"
    );
}

/// Test process.uptime() exists
#[test]
#[serial]
fn test_uptime_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.uptime;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.uptime should be a function"
    );
}

/// Test process.uptime() returns number
#[test]
#[serial]
fn test_uptime_returns_number() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let uptime = process.uptime();
        typeof uptime === 'number' && uptime >= 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.uptime() should return non-negative number"
    );
}

/// Test process.cpuUsage() exists
#[test]
#[serial]
fn test_cpu_usage_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.cpuUsage;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.cpuUsage should be a function"
    );
}

/// Test process.cpuUsage() returns object
#[test]
#[serial]
fn test_cpu_usage_returns_object() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let cpu = process.cpuUsage();
        typeof cpu === 'object' && typeof cpu.user === 'number' && typeof cpu.system === 'number';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.cpuUsage() should return object with user/system times"
    );
}

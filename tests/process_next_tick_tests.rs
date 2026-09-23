// v0.3.239: process.nextTick() 和 stdout/stderr 测试
// 测试 nextTick 的微任务队列行为和 I/O 功能

use serial_test::serial;

/// Test process.nextTick() exists
#[test]
#[serial]
fn test_next_tick_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.nextTick;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.nextTick should be a function"
    );
}

/// Test process.nextTick() returns undefined
#[test]
#[serial]
fn test_next_tick_returns_undefined() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let result = process.nextTick(() => {});
        result === undefined;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.nextTick should return undefined"
    );
}

/// Test process.nextTick() with callback arguments.
/// nextTick runs after the synchronous script completes; execute_code returns
/// the main script completion value, so observe drained state in a second read.
#[test]
#[serial]
fn test_next_tick_with_args() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        globalThis.__nextTickReceived = null;
        process.nextTick((arg1, arg2) => {
            globalThis.__nextTickReceived = arg1 + arg2;
        }, 1, 2);
        globalThis.__nextTickReceived;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "null",
        "execute_code should return the main script completion before nextTick drain"
    );

    let observed = runtime
        .execute_code("globalThis.__nextTickReceived;")
        .expect("Execution failed");
    assert_eq!(
        observed.trim(),
        "3",
        "nextTick callback arguments should be passed correctly"
    );
}

/// Test process.nextTick() error when callback is not a function
#[test]
#[serial]
fn test_next_tick_requires_function() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        try {
            process.nextTick('not a function');
            false;
        } catch (e) {
            e.message.includes('must be a function');
        }
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "nextTick should throw when callback is not a function"
    );
}

/// Test process.stdout.write() exists
#[test]
#[serial]
fn test_stdout_write_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.stdout.write;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.stdout.write should be a function"
    );
}

/// Test process.stderr.write() exists
#[test]
#[serial]
fn test_stderr_write_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.stderr.write;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.stderr.write should be a function"
    );
}

/// Test process.stdout.write() returns boolean
#[test]
#[serial]
fn test_stdout_write_returns_boolean() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let result = process.stdout.write('test');
        typeof result;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "boolean",
        "process.stdout.write should return boolean"
    );
}

/// Test process.stderr.write() returns boolean
#[test]
#[serial]
fn test_stderr_write_returns_boolean() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let result = process.stderr.write('error');
        typeof result;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "boolean",
        "process.stderr.write should return boolean"
    );
}

/// Test process.stdout.write() with empty string
#[test]
#[serial]
fn test_stdout_write_empty_string() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let result = process.stdout.write('');
        result === true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.stdout.write('') should return true"
    );
}

/// Test process.stdout.write() with number (should convert to string)
#[test]
#[serial]
fn test_stdout_write_number() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let result = process.stdout.write(42);
        result === true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.stdout.write(42) should return true"
    );
}

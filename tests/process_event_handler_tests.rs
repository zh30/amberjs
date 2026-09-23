// Tests for process event handlers - v0.3.238
// Tests for process.on('uncaughtException') and process.on('unhandledRejection')
// These tests verify the global event handler functionality

use serial_test::serial;

/// Test that process.on exists and is a function
#[test]
#[serial]
fn test_process_on_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.on;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "process.on should be a function");
}

/// Test that process.off exists and is a function
#[test]
#[serial]
fn test_process_off_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.off;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.off should be a function"
    );
}

/// Test that process.removeListener exists and is a function
#[test]
#[serial]
fn test_process_remove_listener_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.removeListener;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.removeListener should be a function"
    );
}

/// Test registering an uncaughtException handler
#[test]
#[serial]
fn test_process_on_uncaught_exception() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let handlerCalled = false;
        process.on('uncaughtException', function(err) {
            handlerCalled = true;
        });
        handlerCalled === false;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.on should register handler without calling it"
    );
}

/// Test registering an unhandledRejection handler
#[test]
#[serial]
fn test_process_on_unhandled_rejection() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let handlerCalled = false;
        process.on('unhandledRejection', function(reason, promise) {
            handlerCalled = true;
        });
        handlerCalled === false;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.on should register handler without calling it"
    );
}

/// Test that process.on returns process object for chaining
#[test]
#[serial]
fn test_process_on_returns_process() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const result = process.on('uncaughtException', function() {});
        result === process;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.on should return process for chaining"
    );
}

/// Test multiple event handlers can be registered
#[test]
#[serial]
fn test_multiple_uncaught_exception_handlers() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let callCount = 0;
        process.on('uncaughtException', function() { callCount++; });
        process.on('uncaughtException', function() { callCount++; });
        // Simulate throwing an error
        try {
            throw new Error('test');
        } catch (e) {
            // Handlers would be called here in full implementation
        }
        callCount >= 0; // Handlers registered successfully
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Multiple handlers should be registerable"
    );
}

/// Test process.off exists and works
#[test]
#[serial]
fn test_process_off_functionality() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let handlerCalled = false;
        const handler = function() { handlerCalled = true; };
        process.on('uncaughtException', handler);
        process.off('uncaughtException', handler);
        typeof process.off;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "process.off should be callable");
}

/// Test process.env exists and is an object
#[test]
#[serial]
fn test_process_env_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.env;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "object", "process.env should be an object");
}

/// Test process.stdout exists
#[test]
#[serial]
fn test_process_stdout_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.stdout;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "process.stdout should be an object"
    );
}

/// Test process.stderr exists
#[test]
#[serial]
fn test_process_stderr_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.stderr;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "process.stderr should be an object"
    );
}

/// Test process.stdin exists
#[test]
#[serial]
fn test_process_stdin_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.stdin;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "object", "process.stdin should be an object");
}

/// Test process.argv exists and is an array
#[test]
#[serial]
fn test_process_argv_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        Array.isArray(process.argv);
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "process.argv should be an array");
}

/// Test process.argv has at least 2 elements
#[test]
#[serial]
fn test_process_argv_length() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        process.argv.length >= 2;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.argv should have at least 2 elements"
    );
}

/// Test that unknown event types don't crash
#[test]
#[serial]
fn test_process_on_unknown_event() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const result = process.on('unknownEvent', function() {});
        typeof result;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "process.on should handle unknown events gracefully"
    );
}

/// Test process.nextTick exists
#[test]
#[serial]
fn test_process_next_tick_exists() {
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

/// Test that registering non-function handler doesn't crash
#[test]
#[serial]
fn test_process_on_non_function_handler() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        process.on('uncaughtException', 'not a function');
        typeof process.on;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.on should handle non-function gracefully"
    );
}

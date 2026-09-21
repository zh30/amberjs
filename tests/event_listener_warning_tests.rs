// v0.3.243: 事件监听器警告机制测试
// 测试当监听器数量超过 maxListeners 时的警告功能

use serial_test::serial;

/// Test warning is emitted when adding more than maxListeners
#[test]
#[serial]
fn test_listener_warning_exceeds_max() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const EventEmitter = require('events').EventEmitter;
        const emitter = new EventEmitter();
        emitter.setMaxListeners(2);

        let warningCount = 0;
        const originalWarn = console.warn;
        console.warn = function(...args) {
            warningCount++;
            originalWarn.apply(console, args);
        };

        // Add 3 listeners (exceeds max of 2)
        emitter.on('test', () => {});
        emitter.on('test', () => {});
        emitter.on('test', () => {});

        console.warn = originalWarn;

        // Should have emitted a warning
        warningCount >= 1;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Warning should be emitted when exceeding maxListeners"
    );
}

/// Test no warning when at maxListeners
#[test]
#[serial]
fn test_no_warning_at_max() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const EventEmitter = require('events').EventEmitter;
        const emitter = new EventEmitter();
        emitter.setMaxListeners(2);

        let warningCount = 0;
        const originalWarn = console.warn;
        console.warn = function(...args) {
            warningCount++;
        };

        // Add exactly 2 listeners (at max)
        emitter.on('test', () => {});
        emitter.on('test', () => {});

        console.warn = originalWarn;

        // Should not have emitted a warning
        warningCount === 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "No warning should be emitted when at maxListeners"
    );
}

/// Test no warning when under maxListeners
#[test]
#[serial]
fn test_no_warning_under_max() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const EventEmitter = require('events').EventEmitter;
        const emitter = new EventEmitter();
        emitter.setMaxListeners(5);

        let warningCount = 0;
        const originalWarn = console.warn;
        console.warn = function(...args) {
            warningCount++;
        };

        // Add 3 listeners (under max of 5)
        emitter.on('test', () => {});
        emitter.on('test', () => {});
        emitter.on('test', () => {});

        console.warn = originalWarn;

        // Should not have emitted a warning
        warningCount === 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "No warning should be emitted when under maxListeners"
    );
}

/// Test warning contains MaxListenersExceeded
#[test]
#[serial]
fn test_warning_contains_max_listeners_exceeded() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const EventEmitter = require('events').EventEmitter;
        const emitter = new EventEmitter();
        emitter.setMaxListeners(1);

        let warningMessage = '';
        const originalWarn = console.warn;
        console.warn = function(msg) {
            warningMessage = msg;
        };

        emitter.on('test', () => {});
        emitter.on('test', () => {});

        console.warn = originalWarn;

        warningMessage.includes('MaxListenersExceeded') || warningMessage.includes('warning');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Warning should mention MaxListenersExceeded"
    );
}

/// Test default maxListeners is 10
#[test]
#[serial]
fn test_default_max_listeners_is_10() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const EventEmitter = require('events').EventEmitter;
        const emitter = new EventEmitter();
        emitter.getMaxListeners() === 10;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "Default maxListeners should be 10");
}

/// Test process.setMaxListeners affects events
#[test]
#[serial]
fn test_process_set_max_listeners_affects_events() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        process.setMaxListeners(15);
        process.getMaxListeners() === 15;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.setMaxListeners should set the value"
    );
}

/// Test unlimited maxListeners (0)
#[test]
#[serial]
fn test_unlimited_max_listeners() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const EventEmitter = require('events').EventEmitter;
        const emitter = new EventEmitter();
        emitter.setMaxListeners(0); // 0 means unlimited

        let warningCount = 0;
        const originalWarn = console.warn;
        console.warn = function(...args) {
            warningCount++;
        };

        // Add many listeners (should not warn with unlimited)
        for (let i = 0; i < 20; i++) {
            emitter.on('test', () => {});
        }

        console.warn = originalWarn;

        warningCount === 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "No warning should be emitted with unlimited maxListeners"
    );
}

/// Test once() also triggers warning
#[test]
#[serial]
fn test_once_warning_exceeds_max() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const EventEmitter = require('events').EventEmitter;
        const emitter = new EventEmitter();
        emitter.setMaxListeners(2);

        let warningCount = 0;
        const originalWarn = console.warn;
        console.warn = function(...args) {
            warningCount++;
        };

        // Add listeners using once (exceeds max of 2)
        emitter.once('test', () => {});
        emitter.once('test', () => {});
        emitter.once('test', () => {});

        console.warn = originalWarn;

        warningCount >= 1;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Warning should be emitted for once() when exceeding maxListeners"
    );
}

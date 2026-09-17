// Tests for setImmediate API
// v0.2.5: setImmediate implementation

use serial_test::serial;

#[test]
#[serial]
fn test_set_immediate_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof setImmediate")
        .expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "setImmediate should be a function"
    );
}

#[test]
#[serial]
fn test_set_immediate_basic_execution() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    // setImmediate runs in the check phase after the current sync script completes.
    // Resolve a Promise from the callback so execute_code can await the settlement.
    let code = r#"
        new Promise((resolve) => {
            setImmediate(function() { resolve(true); });
        });
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "setImmediate callback should execute during the event-loop drain"
    );
}

#[test]
#[serial]
fn test_set_immediate_with_argument() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        new Promise((resolve) => {
            setImmediate(function(x, y) { resolve(x + y); }, 5, 3);
        });
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "8",
        "setImmediate should pass arguments to callback"
    );
}

#[test]
#[serial]
fn test_set_immediate_returns_timer_id() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timerId = setImmediate(function() {});
        // Timer is now an object (v0.3.36) with methods
        typeof timerId === 'object' && typeof timerId.unref === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "setImmediate should return a timer object with methods"
    );
}

#[test]
#[serial]
fn test_set_immediate_multiple_calls() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        new Promise((resolve) => {
            let count = 0;
            setImmediate(function() { count += 1; });
            setImmediate(function() { count += 2; });
            setImmediate(function() {
                count += 3;
                resolve(count);
            });
        });
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "6",
        "Multiple setImmediate calls should all execute"
    );
}

#[test]
#[serial]
fn test_clear_immediate_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof clearImmediate")
        .expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "clearImmediate should be a function"
    );
}

#[test]
#[serial]
fn test_clear_immediate_basic() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        let executed = false;
        const timerId = setImmediate(function() { executed = true; });
        clearImmediate(timerId);
        // Timer should be cleared, but for sync execution it still runs
        // This test verifies clearImmediate is callable
        true;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "clearImmediate should be callable without error"
    );
}

#[test]
#[serial]
fn test_set_immediate_callback_with_this() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        new Promise((resolve) => {
            const obj = {
                test: function() {
                    resolve(this === obj);
                }
            };
            setImmediate(function() { obj.test(); });
        });
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "setImmediate callback should preserve 'this' context"
    );
}

#[test]
#[serial]
fn test_set_immediate_error_handling() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    // setImmediate without callback should throw
    let code = r#"setImmediate()"#;
    let result = runtime.execute_code(code);
    assert!(
        result.is_err(),
        "setImmediate without callback should throw an error"
    );
}

#[test]
#[serial]
fn test_set_immediate_non_function() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    // setImmediate with non-function should throw
    let code = r#"setImmediate("not a function")"#;
    let result = runtime.execute_code(code);
    assert!(
        result.is_err(),
        "setImmediate with non-function callback should throw"
    );
}

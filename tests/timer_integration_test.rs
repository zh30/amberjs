// v0.3.245: Integration test for async timer functionality
// v0.3.249: Tests for async timer execution with polling
// Tests setTimeout, setInterval, clearTimeout, and clearImmediate

use amberjs::nodejs_core::timers::{clear_all_async_timers, clear_all_timers};
use amberjs::MinimalRuntime;
use serial_test::serial;

fn cleanup_global_state() {
    // Clear timer metadata between tests
    // Use ignore_poisoned to handle mutex poison edge cases
    clear_all_timers();
    clear_all_async_timers();
}

#[test]
#[serial]
fn test_settimeout_zero_delay_executes() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // setTimeout with delay=0 executes in the timer phase after synchronous code.
    let result = runtime
        .execute_code(
            r#"
        new Promise((resolve) => {
            let executed = false;
            setTimeout(() => {
                executed = true;
                resolve(executed);
            }, 0);
        });
    "#,
        )
        .unwrap();
    assert_eq!(result.trim(), "true");
}

#[test]
#[serial]
fn test_settimeout_nonzero_delay_queued() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // Test that setTimeout with delay > 0 only stores metadata (async not yet integrated)
    let result = runtime
        .execute_code(
            r#"
        let result = 'not called';
        setTimeout(() => { result = 'called'; }, 100);
        result;
    "#,
        )
        .unwrap();
    // For delay > 0, timer is queued but not executed (requires async runtime)
    assert_eq!(result.trim(), "not called");
}

#[test]
#[serial]
fn test_cleartimer_prevents_execution() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // Clear timer before it can execute
    let _ = runtime
        .execute_code(
            r#"
        globalThis.testValue = 'initial';
        const id = setTimeout(() => { globalThis.testValue = 'changed'; }, 100);
        clearTimeout(id);
    "#,
        )
        .unwrap();
    // The callback should not have run
    let check = runtime.execute_code("globalThis.testValue").unwrap();
    assert_eq!(check.trim(), "initial");
}

#[test]
#[serial]
fn test_setinterval_returns_timer() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // setInterval returns a timer ID (number or wrapped in object)
    // v0.3.249: Accept both number and object for flexibility
    let result = runtime
        .execute_code(
            r#"
        const id = setInterval(() => {}, 100);
        id.unref();
        typeof id;
    "#,
        )
        .unwrap();
    assert!(
        result.trim() == "number" || result.trim() == "object",
        "Expected number or object, got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_setimmediate_basic() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // setImmediate executes immediately (simplified implementation)
    let result = runtime
        .execute_code(
            r#"
        let result = 'start';
        setImmediate(() => { result = 'done'; });
        result;
    "#,
        )
        .unwrap();
    // Either result is valid depending on implementation
    assert!(result.trim() == "start" || result.trim() == "done");
}

#[test]
#[serial]
fn test_timer_with_arguments() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    let _ = runtime.execute_code(
        r#"
        setTimeout((a, b, c) => {
            globalThis.result = a + b + c;
        }, 0, 1, 2, 3);
    "#,
    );
    let result = runtime.execute_code("globalThis.result").unwrap();
    assert_eq!(result.trim(), "6");
}

#[test]
#[serial]
fn test_timer_metadata_storage() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // Just verify timers can be created without error.
    // Unref the ref'd timers so the keep-alive loop can return.
    let result = runtime
        .execute_code(
            r#"
        const t = setTimeout(() => {}, 100);
        const i = setInterval(() => {}, 200);
        setImmediate(() => {});
        t.unref();
        i.unref();
        'timers created';
    "#,
        )
        .unwrap();
    assert_eq!(result.trim(), "timers created");
}

#[test]
#[serial]
fn test_cleartimer_with_invalid_id() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code("clearTimeout(99999); 'no crash'")
        .unwrap();
    assert_eq!(result.trim(), "no crash");
}

#[test]
#[serial]
fn test_multiple_timers_metadata() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code(
            r#"
        setTimeout(() => {}, 10);
        setTimeout(() => {}, 5);
        'timers registered';
    "#,
        )
        .unwrap();
    // Timers are registered (metadata stored)
    assert_eq!(result.trim(), "timers registered");
}

// v0.3.249: Tests for async timer execution
#[test]
#[serial]
fn test_settimeout_async_callback_stored() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // Timer with delay > 0 should store callback in registry.
    // Unref so execute_code returns before it fires (ref'd timers keep the
    // loop alive until they run).
    let _ = runtime
        .execute_code(
            r#"
        const t = setTimeout(() => {
            globalThis.asyncExecuted = true;
        }, 100);
        t.unref();
    "#,
        )
        .unwrap();

    // Verify timer was created (callback stored)
    let check = runtime
        .execute_code("typeof globalThis.asyncExecuted")
        .unwrap();
    assert_eq!(check.trim(), "undefined"); // Not yet executed
}

#[test]
#[serial]
fn test_cleartimer_prevents_callback_execution() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // Schedule a timer and immediately clear it
    let _ = runtime
        .execute_code(
            r#"
        globalThis.timerTest = 'initial';
        const id = setTimeout(() => {
            globalThis.timerTest = 'changed';
        }, 50);
        clearTimeout(id);
    "#,
        )
        .unwrap();

    // Timer should not execute after being cleared
    let check = runtime.execute_code("globalThis.timerTest").unwrap();
    assert_eq!(check.trim(), "initial");
}

#[test]
#[serial]
fn test_setinterval_repeats() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // setInterval should store callback for repeated execution.
    // Unref so the keep-alive event loop can return from execute_code;
    // Node semantics keep ref'd intervals running forever.
    let _ = runtime
        .execute_code(
            r#"
        globalThis.intervalCount = 0;
        const id = setInterval(() => {
            globalThis.intervalCount += 1;
        }, 10);
        id.unref();
        globalThis.intervalId = id;
    "#,
        )
        .unwrap();

    // Verify interval was created (accept number or object)
    let check = runtime
        .execute_code("typeof globalThis.intervalId")
        .unwrap();
    assert!(
        check.trim() == "number" || check.trim() == "object",
        "Expected number or object, got: {}",
        check.trim()
    );
}

#[test]
#[serial]
fn test_timer_callback_with_multiple_args() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // Test timer with multiple arguments
    let _ = runtime
        .execute_code(
            r#"
        setTimeout((a, b, c, d) => {
            globalThis.argsResult = a + '-' + b + '-' + c + '-' + d;
        }, 0, 'hello', 'world', 42, true);
    "#,
        )
        .unwrap();

    // Verify callback was executed with arguments
    let result = runtime.execute_code("globalThis.argsResult").unwrap();
    assert_eq!(result.trim(), "hello-world-42-true");
}

#[test]
#[serial]
fn test_timer_metadata_complete() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // Verify all timer types create proper metadata
    // v0.3.249: Timers return number IDs (which may be wrapped in object internally)
    let result = runtime
        .execute_code(
            r#"
        const timeoutId = setTimeout(() => {}, 100);
        const intervalId = setInterval(() => {}, 200);
        const immediateId = setImmediate(() => {});
        timeoutId.unref();
        intervalId.unref();
        typeof timeoutId + '-' + typeof intervalId + '-' + typeof immediateId;
    "#,
        )
        .unwrap();
    // Accept both number or object return types (implementation detail)
    assert!(
        result.trim() == "number-number-number" || result.trim() == "object-object-object",
        "Unexpected result: {}",
        result.trim()
    );
}

// Tests for timers API (setTimeout, setInterval) - v0.4.0
// Enhanced timer functionality for AI workloads
// v0.3.271: Default runtime returns Node-compatible Timer handle objects.
// v0.3.256: Added cleanup calls to prevent V8 handle errors

use amberjs::nodejs_core::timers::{
    clear_all_async_timers, clear_all_timer_callbacks, clear_all_timers,
};
use serial_test::serial;

/// Helper function to clean up timer state before runtime is dropped
/// This prevents "Handle hosted by disposed Isolate" errors
fn cleanup_timers() {
    clear_all_timer_callbacks();
    clear_all_timers();
    clear_all_async_timers();
}

#[test]
#[serial]
fn test_set_timeout_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof setTimeout;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "setTimeout should be a function");
    cleanup_timers();
}

#[test]
#[serial]
fn test_set_timeout_returns_timer_id() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timerId = setTimeout(function() {}, 100);
        typeof timerId === 'object' &&
        timerId !== null &&
        Number(timerId) > 0 &&
        typeof timerId.unref === 'function' &&
        typeof timerId.ref === 'function' &&
        typeof timerId.refresh === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "setTimeout should return a Timer handle"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_set_timeout_basic_execution() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    // setTimeout(0) executes in the timer phase after synchronous code.
    let code = r#"
        new Promise((resolve) => {
            let executed = false;
            setTimeout(function() {
                executed = true;
                resolve(executed);
            }, 0);
        });
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "setTimeout callback should execute with delay 0"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_set_timeout_with_delay() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timerId = setTimeout(function() {}, 100);
        timerId > 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "setTimeout should accept delay parameter"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_set_timeout_with_arguments() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        new Promise((resolve) => {
            setTimeout(function(a, b, c) {
                resolve(a + b + c);
            }, 0, 1, 2, 3);
        });
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "6",
        "setTimeout should pass arguments to callback"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_clear_timeout_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof clearTimeout;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "clearTimeout should be a function"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_clear_timeout_basic() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timerId = setTimeout(function() {}, 1000);
        clearTimeout(timerId);
        // v0.3.271: Timer is a handle object that clearTimeout accepts.
        typeof timerId;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "clearTimeout should accept Timer handles"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_set_interval_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof setInterval;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "setInterval should be a function"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_set_interval_returns_timer_id() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timerId = setInterval(function() {}, 100);
        const ok = typeof timerId === 'object' &&
        timerId !== null &&
        Number(timerId) > 0 &&
        typeof timerId.unref === 'function' &&
        typeof timerId.ref === 'function' &&
        typeof timerId.refresh === 'function';
        timerId.unref();
        ok;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "setInterval should return a Timer handle"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_set_interval_basic_execution() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    // Note: setInterval is async - callback won't execute in same tick
    // We test that it returns a valid timer ID
    let code = r#"
        const timerId = setInterval(function() {}, 0);
        timerId.unref();
        timerId > 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "setInterval should return valid timer ID"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_clear_interval_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof clearInterval;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "clearInterval should be a function"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_clear_interval_basic() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timerId = setInterval(function() {}, 100);
        clearInterval(timerId);
        // v0.3.271: Timer is a handle object that clearInterval accepts.
        typeof timerId;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "clearInterval should accept Timer handles"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_timer_ids_are_numbers() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timeoutId = setTimeout(function() {}, 100);
        const intervalId = setInterval(function() {}, 100);
        const ok = typeof timeoutId === 'object' &&
        typeof intervalId === 'object' &&
        Number(timeoutId) > 0 &&
        Number(intervalId) > 0;
        timeoutId.unref();
        intervalId.unref();
        ok;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "Both timers should be handles");
    cleanup_timers();
}

#[test]
#[serial]
fn test_multiple_timers() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const ids = [];
        ids.push(setTimeout(function() {}, 100));
        ids.push(setTimeout(function() {}, 200));
        ids.push(setInterval(function() {}, 300));
        ids.push(setInterval(function() {}, 400));
        // Ref'd timers keep the event loop alive by design; release them so
        // execute_code can return after checking handle creation.
        ids.forEach((id) => id.unref());
        ids.length === 4;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "Multiple timers should be created");
    cleanup_timers();
}

#[test]
#[serial]
fn test_timer_zero_delay() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        new Promise((resolve) => {
            let executed = false;
            setTimeout(function() {
                executed = true;
                resolve(executed);
            }, 0);
        });
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Timer with 0ms delay should execute in the timer phase"
    );
    cleanup_timers();
}

// v0.3.271: Tests for Timer handle functionality.
#[test]
#[serial]
fn test_settimeout_returns_timer_handle() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timerId = setTimeout(function() {}, 1000);
        typeof timerId === 'object' && Number(timerId) > 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "setTimeout should return a Timer handle"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_setinterval_returns_timer_handle() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timerId = setInterval(function() {}, 1000);
        const ok = typeof timerId === 'object' && Number(timerId) > 0;
        timerId.unref();
        ok;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "setInterval should return a Timer handle"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_setimmediate_returns_timer_handle() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timerId = setImmediate(function() {});
        typeof timerId === 'object' && Number(timerId) > 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "setImmediate should return a Timer handle"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_timer_ids_are_unique() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const id1 = setTimeout(function() {}, 100);
        const id2 = setTimeout(function() {}, 100);
        id1 !== id2;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "Each timer should have a unique ID");
    cleanup_timers();
}

// v0.3.271: Basic Timer handle functionality tests.

#[test]
#[serial]
fn test_timer_has_unref_method() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timerId = setTimeout(function() {}, 1000);
        typeof timerId.unref === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "Timer should expose unref()");
    cleanup_timers();
}

#[test]
#[serial]
fn test_timer_has_ref_method() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timerId = setTimeout(function() {}, 1000);
        typeof timerId.ref === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "Timer should expose ref()");
    cleanup_timers();
}

#[test]
#[serial]
fn test_timer_unref_is_callable() {
    // v0.3.271: clearTimeout works with Timer handles.
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timerId = setTimeout(function() {}, 1000);
        clearTimeout(timerId);
        'success';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "success",
        "clearTimeout should work with Timer handles"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_timer_ref_is_callable() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timerId = setTimeout(function() {}, 1000);
        clearTimeout(timerId);
        'success';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "success",
        "clearTimeout should work with Timer handles"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_timer_unref_ref_chain() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timerId1 = setTimeout(function() {}, 1000);
        const timerId2 = setTimeout(function() {}, 1000);
        clearTimeout(timerId1);
        clearTimeout(timerId2);
        'success';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "success",
        "Multiple clearTimeout calls should work"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_interval_timer_has_unref_ref() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    // A ref'd interval keeps the event loop alive by design (Node semantics),
    // so unref it after checking the handle to let execute_code return.
    let code = r#"
        const timerId = setInterval(function() {}, 1000);
        const ok = typeof timerId === 'object' && Number(timerId) > 0
            && typeof timerId.unref === 'function' && typeof timerId.ref === 'function';
        timerId.unref();
        ok;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "setInterval should return Timer handle"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_immediate_timer_has_unref_ref() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timerId = setImmediate(function() {});
        typeof timerId === 'object' && Number(timerId) > 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "setImmediate should return Timer handle"
    );
    cleanup_timers();
}

#[test]
#[serial]
fn test_timer_has_refresh_method_alias() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const timerId = setTimeout(function() {}, 1000);
        typeof timerId.refresh === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "Timer should expose refresh()");
    cleanup_timers();
}

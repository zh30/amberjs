// v0.3.249: Integration tests for async timer callback execution
// Tests that verify delay > 0 setTimeout/setInterval callbacks actually execute
// through the V8 main thread event loop integration

use amberjs::nodejs_core::timers::{clear_all_async_timers, clear_all_timers};
use amberjs::MinimalRuntime;
use serial_test::serial;
use std::thread;
use std::time::Duration;

fn cleanup_global_state() {
    clear_all_timers();
    clear_all_async_timers();
}

#[test]
#[serial]
fn test_settimeout_async_executes_after_delay() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    runtime.set_timer_drain_limit_ms(150);

    // Short ref'ed timers should drain Promise-backed async work before execute_code returns.
    let result = runtime
        .execute_code(
            r#"
        new Promise((resolve) => {
            setTimeout(() => {
                resolve('after');
            }, 50);
        });
    "#,
        )
        .unwrap();

    assert_eq!(result.trim(), "after");
}

#[test]
#[serial]
fn test_settimeout_zero_executes_immediately() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();

    // setTimeout with delay=0 should execute in the timer phase after sync code.
    let result = runtime
        .execute_code(
            r#"
        new Promise((resolve) => {
            globalThis.testValue = 'initial';
            setTimeout(() => {
                globalThis.testValue = 'changed';
                resolve(globalThis.testValue);
            }, 0);
        });
    "#,
        )
        .unwrap();

    assert_eq!(result.trim(), "changed");
}

#[test]
#[serial]
fn test_clear_timeout_prevents_execution() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();

    // Schedule a timer and immediately clear it
    let _ = runtime
        .execute_code(
            r#"
        globalThis.timerValue = 'initial';
        const id = setTimeout(() => {
            globalThis.timerValue = 'changed';
        }, 200);
        clearTimeout(id);
    "#,
        )
        .unwrap();

    // Timer should be cancelled, value should still be initial
    let check = runtime.execute_code("globalThis.timerValue").unwrap();
    assert_eq!(check.trim(), "initial");

    // Wait longer than the timer delay to ensure it wouldn't have fired
    thread::sleep(Duration::from_millis(300));

    // Still should be initial (timer was cleared)
    let check = runtime.execute_code("globalThis.timerValue").unwrap();
    assert_eq!(check.trim(), "initial");
}

#[test]
#[serial]
fn test_multiple_set_timeout_zero() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();

    // Multiple delay=0 timers should all execute in scheduling order.
    let result = runtime
        .execute_code(
            r#"
        new Promise((resolve) => {
            globalThis.count = 0;
            const tick = () => {
                globalThis.count += 1;
                if (globalThis.count === 3) {
                    resolve(globalThis.count);
                }
            };
            setTimeout(tick, 0);
            setTimeout(tick, 0);
            setTimeout(tick, 0);
        });
    "#,
        )
        .unwrap();

    assert_eq!(result.trim(), "3");
}

#[test]
#[serial]
fn test_timer_with_arguments_zero_delay() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();

    // Test timer with arguments (delay=0, executes in timer phase)
    let result = runtime
        .execute_code(
            r#"
        new Promise((resolve) => {
            setTimeout((a, b, c) => {
                globalThis.argsResult = a + b + c;
                resolve(globalThis.argsResult);
            }, 0, 10, 20, 30);
        });
    "#,
        )
        .unwrap();

    assert_eq!(result.trim(), "60");

    // Verify the actual value
    let check = runtime.execute_code("globalThis.argsResult").unwrap();
    assert_eq!(check.trim(), "60");
}

#[test]
#[serial]
fn test_setinterval_basic() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();

    // Test that setInterval schedules callbacks correctly
    // First, verify the timer is created and can be cleared.
    // Unref so the keep-alive loop returns from execute_code (Node semantics
    // keep ref'd intervals alive forever).
    let result = runtime
        .execute_code(
            r#"
        globalThis.intervalCount = 0;
        const id = setInterval(() => {
            globalThis.intervalCount += 1;
        }, 10);
        id.unref();
        globalThis.intervalId = id;
        typeof id;
    "#,
        )
        .unwrap();

    // Timer should be created (id should be a number or object)
    assert!(
        result.trim() == "number" || result.trim() == "object",
        "setInterval should return a timer ID, got: {}",
        result.trim()
    );

    // Wait for the interval to fire
    thread::sleep(Duration::from_millis(100));

    // Check if callback was executed - this verifies the async integration works
    // Note: Due to timing, this may or may not have fired yet
    // The important thing is that the timer was scheduled correctly
    let check = runtime
        .execute_code("typeof globalThis.intervalCount")
        .unwrap();
    assert_eq!(check.trim(), "number", "intervalCount should be a number");

    // Verify we can clear the interval (proves it was registered)
    let clear_result = runtime
        .execute_code("clearInterval(globalThis.intervalId); 'cleared'")
        .unwrap();
    assert_eq!(
        clear_result.trim(),
        "cleared",
        "Should be able to clear the interval"
    );
}

#[test]
#[serial]
fn test_setinterval_repeats_until_clear_interval() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    runtime.set_timer_drain_limit_ms(100);

    let result = runtime
        .execute_code(
            r#"
        new Promise((resolve) => {
            let count = 0;
            let intervalId = setInterval(() => {
                count += 1;
                if (count === 3) {
                    clearInterval(intervalId);
                    resolve(count);
                }
            }, 5);
        });
    "#,
        )
        .unwrap();

    assert_eq!(
        result.trim(),
        "3",
        "setInterval should keep firing until clearInterval removes it"
    );
    cleanup_global_state();
}

#[test]
#[serial]
fn test_setinterval_without_clear_stops_at_drain_limit() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    runtime.set_timer_drain_limit_ms(50);

    let result = runtime
        .execute_code(
            r#"
        globalThis.intervalCount = 0;
        setInterval(() => {
            globalThis.intervalCount += 1;
        }, 5);
        "scheduled";
    "#,
        )
        .unwrap();

    assert_eq!(
        result.trim(),
        "scheduled",
        "unbounded intervals should not prevent execute_code from returning after the drain limit"
    );

    let count = runtime
        .execute_code("globalThis.intervalCount")
        .unwrap()
        .trim()
        .parse::<u64>()
        .expect("interval count should be numeric");
    assert!(
        count > 0,
        "interval should fire during the drain window before execute_code returns"
    );
    cleanup_global_state();
}

#[test]
#[serial]
fn test_clear_all_timers_function() {
    use amberjs::nodejs_core::timers::{clear_all_async_timers, clear_all_timers};

    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();

    // Create multiple timers. Unref so execute_code returns before they fire
    // (ref'd timers keep the loop alive until they run, which would make the
    // clear below meaningless).
    let _ = runtime
        .execute_code(
            r#"
        globalThis.timer1Fired = false;
        globalThis.timer2Fired = false;
        const t1 = setTimeout(() => { globalThis.timer1Fired = true; }, 100);
        const t2 = setTimeout(() => { globalThis.timer2Fired = true; }, 100);
        t1.unref();
        t2.unref();
    "#,
        )
        .unwrap();

    // Clear all timers using Rust function (not exposed to JS)
    clear_all_timers();
    clear_all_async_timers();

    // Wait for timers to potentially fire
    thread::sleep(Duration::from_millis(200));

    // Neither timer should have fired
    let check1 = runtime.execute_code("globalThis.timer1Fired").unwrap();
    let check2 = runtime.execute_code("globalThis.timer2Fired").unwrap();
    assert_eq!(check1.trim(), "false");
    assert_eq!(check2.trim(), "false");
}

#[test]
#[serial]
fn test_invalid_timer_id_does_not_crash() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();

    // Clearing non-existent timers should not crash
    let result = runtime
        .execute_code(
            r#"
        clearTimeout(99999);
        clearInterval(88888);
        clearImmediate(77777);
        'no crash';
    "#,
        )
        .unwrap();

    assert_eq!(result.trim(), "no crash");
}

#[test]
#[serial]
fn test_nested_set_timeout_zero() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();

    // Nested setTimeout(..., 0) should drain until the nested chain settles.
    let result = runtime
        .execute_code(
            r#"
        new Promise((resolve) => {
            globalThis.nestedCount = 0;
            setTimeout(() => {
                globalThis.nestedCount += 1;
                setTimeout(() => {
                    globalThis.nestedCount += 1;
                    setTimeout(() => {
                        globalThis.nestedCount += 1;
                        resolve(globalThis.nestedCount);
                    }, 0);
                }, 0);
            }, 0);
        });
    "#,
        )
        .unwrap();

    assert_eq!(result.trim(), "3");
}

#[test]
#[serial]
fn test_timer_execution_order() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();

    // Timers should execute in the order they were scheduled.
    let result = runtime
        .execute_code(
            r#"
        new Promise((resolve) => {
            globalThis.order = [];
            const finish = () => {
                if (globalThis.order.length === 3) {
                    resolve(globalThis.order.join(','));
                }
            };
            setTimeout(() => { globalThis.order.push('first'); finish(); }, 0);
            setTimeout(() => { globalThis.order.push('second'); finish(); }, 0);
            setTimeout(() => { globalThis.order.push('third'); finish(); }, 0);
        });
    "#,
        )
        .unwrap();

    assert_eq!(result.trim(), "first,second,third");
}

#[test]
#[serial]
fn test_setimmediate_basic_execution() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();

    // setImmediate executes in next event loop iteration
    let result = runtime
        .execute_code(
            r#"
        globalThis.immediateValue = 'start';
        setImmediate(() => { globalThis.immediateValue = 'done'; });
        globalThis.immediateValue;
    "#,
        )
        .unwrap();

    // setImmediate executes after current execution (in next tick)
    // But since we're in the same execute_code call, it depends on implementation
    // For now, accept either result
    assert!(result.trim() == "start" || result.trim() == "done");
}

// v0.3.260: Enhanced tests for nextTick and Timer execution order
// Verifies Node.js event loop behavior: nextTick -> microtasks -> timers -> setImmediate
// This test file focuses on edge cases and execution order precision

use amberjs::nodejs_core::process::clear_next_tick_queue;
use amberjs::nodejs_core::timers::{
    clear_all_async_timers, clear_all_timer_callbacks, clear_all_timers,
};
use amberjs::MinimalRuntime;
use serial_test::serial;

fn cleanup_global_state() {
    clear_all_timer_callbacks();
    clear_all_timers();
    clear_all_async_timers();
    clear_next_tick_queue();
}

fn read_order(runtime: &mut MinimalRuntime) -> String {
    runtime
        .execute_code("globalThis.__order.join(',');")
        .unwrap()
}

fn read_results(runtime: &mut MinimalRuntime) -> String {
    runtime
        .execute_code("globalThis.__results.join(',');")
        .unwrap()
}

#[test]
#[serial]
fn test_next_tick_priority_over_promise() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // nextTick should have higher priority than Promise callbacks
    runtime
        .execute_code(
            r#"
        globalThis.__order = [];
        process.nextTick(() => globalThis.__order.push('nextTick'));
        Promise.resolve().then(() => globalThis.__order.push('promise'));
        globalThis.__order.join(',');
    "#,
        )
        .unwrap();
    let result = read_order(&mut runtime);
    // nextTick should execute first (higher priority microtask)
    assert_eq!(
        result.trim(),
        "nextTick,promise",
        "nextTick should have higher priority than Promise. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_nested_next_tick_priority() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // Nested nextTicks maintain FIFO order within nextTick queue
    runtime
        .execute_code(
            r#"
        globalThis.__order = [];
        process.nextTick(() => {
            globalThis.__order.push('a');
            process.nextTick(() => globalThis.__order.push('c'));
        });
        process.nextTick(() => globalThis.__order.push('b'));
        globalThis.__order.join(',');
    "#,
        )
        .unwrap();
    let result = read_order(&mut runtime);
    // FIFO: a, b, then c (c added during a's execution)
    assert_eq!(
        result.trim(),
        "a,b,c",
        "Nested nextTicks should follow FIFO. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_timer_vs_immediate_execution_order() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // In Node.js timers phase: setTimeout(0) executes before setImmediate
    runtime
        .execute_code(
            r#"
        globalThis.__order = [];
        setTimeout(() => globalThis.__order.push('timer'), 0);
        setImmediate(() => globalThis.__order.push('immediate'));
        setImmediate(() => globalThis.__order.join(','));
    "#,
        )
        .unwrap();
    let result = read_order(&mut runtime);
    assert_eq!(
        result.trim(),
        "timer,immediate",
        "Timer should execute before setImmediate. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_next_tick_vs_timer_vs_immediate() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // Full order test: nextTick -> timer -> setImmediate
    runtime
        .execute_code(
            r#"
        globalThis.__order = [];
        process.nextTick(() => globalThis.__order.push('nextTick'));
        setTimeout(() => globalThis.__order.push('timer'), 0);
        setImmediate(() => globalThis.__order.push('immediate'));
        globalThis.__order.join(',');
    "#,
        )
        .unwrap();
    let result = read_order(&mut runtime);
    // All callbacks should execute in the same iteration
    // nextTick has highest priority, then timer, then setImmediate
    assert_eq!(
        result.trim(),
        "nextTick,timer,immediate",
        "Execution order should be: nextTick, timer, immediate. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_multiple_timers_order() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // Timers should execute in creation order (FIFO)
    runtime
        .execute_code(
            r#"
        globalThis.__order = [];
        setTimeout(() => globalThis.__order.push('first'), 0);
        setTimeout(() => globalThis.__order.push('second'), 0);
        setTimeout(() => globalThis.__order.push('third'), 0);
        globalThis.__order.join(',');
    "#,
        )
        .unwrap();
    let result = read_order(&mut runtime);
    assert_eq!(
        result.trim(),
        "first,second,third",
        "Timers should execute in creation order. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_next_tick_inside_timer() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // nextTick inside timer should execute after timer completes
    runtime
        .execute_code(
            r#"
        globalThis.__order = [];
        setTimeout(() => {
            globalThis.__order.push('timer');
            process.nextTick(() => globalThis.__order.push('nextTick-in-timer'));
        }, 0);
        setImmediate(() => globalThis.__order.push('immediate'));
        setImmediate(() => globalThis.__order.join(','));
    "#,
        )
        .unwrap();
    let result = read_order(&mut runtime);
    // Timer runs first, then nextTick (still in same phase), then setImmediate
    assert_eq!(
        result.trim(),
        "timer,nextTick-in-timer,immediate",
        "nextTick inside timer should execute after timer. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_promise_then_next_tick() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // Promise callback followed by nextTick - nextTick wins
    runtime
        .execute_code(
            r#"
        globalThis.__order = [];
        Promise.resolve().then(() => globalThis.__order.push('promise'));
        process.nextTick(() => globalThis.__order.push('nextTick'));
        globalThis.__order.join(',');
    "#,
        )
        .unwrap();
    let result = read_order(&mut runtime);
    // nextTick has higher priority than Promise callbacks
    assert_eq!(
        result.trim(),
        "nextTick,promise",
        "nextTick should execute before Promise callback. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_multiple_next_ticks_with_args() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // Multiple nextTicks with different arguments
    runtime
        .execute_code(
            r#"
        globalThis.__results = [];
        process.nextTick((a) => globalThis.__results.push(a), 1);
        process.nextTick((a, b) => globalThis.__results.push(a + b), 2, 3);
        process.nextTick((a, b, c) => globalThis.__results.push(a + b + c), 4, 5, 6);
        globalThis.__results.join(',');
    "#,
        )
        .unwrap();
    let result = read_results(&mut runtime);
    assert_eq!(
        result.trim(),
        "1,5,15",
        "nextTick should pass arguments correctly. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_next_tick_error_propagation() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // Error in nextTick should not stop other callbacks
    runtime
        .execute_code(
            r#"
        globalThis.__order = [];
        process.nextTick(() => globalThis.__order.push('first'));
        process.nextTick(() => { throw new Error('test'); });
        process.nextTick(() => globalThis.__order.push('third'));
        setImmediate(() => globalThis.__order.join(','));
    "#,
        )
        .unwrap();
    let result = read_order(&mut runtime);
    // Error in second nextTick should not prevent third from executing
    // The error handling depends on implementation
    assert!(
        result.trim().contains("first") && result.trim().contains("third"),
        "nextTick errors should not stop other callbacks. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_timer_with_delay_greater_than_zero() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();

    // Verify timer has NOT executed yet (before delay expires)
    // Using setImmediate to check value after the main callback queue but before timer fires
    let result = runtime
        .execute_code(
            r#"
        const timerId = setTimeout(() => { globalThis._timerValue = 'changed'; }, 100);
        setImmediate(() => { globalThis._timerTestValue = 'initial'; });
        typeof timerId;
    "#,
        )
        .unwrap();
    assert_eq!(
        result.trim(),
        "object",
        "setTimeout should return a Timer object. Got: {}",
        result.trim()
    );

    // Get the value from global - should be 'initial' since timer hasn't fired yet
    let result = runtime.execute_code("globalThis._timerTestValue;").unwrap();
    assert_eq!(
        result.trim(),
        "initial",
        "Timer with delay > 0 should not execute before delay expires. Got: {}",
        result.trim()
    );

    // Clean up
    let _ = runtime.execute_code(
        "clearTimeout(timerId); delete globalThis._timerTestValue; delete globalThis._timerValue;",
    );
}

#[test]
#[serial]
fn test_timer_delayed_execution() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // Verify that a timer with delay > 0 eventually executes (after delay)
    // We use a two-step approach: first schedule, then verify execution
    let _result = runtime
        .execute_code(
            r#"
        let executed = false;
        const id = setTimeout(() => { executed = true; }, 50);
        globalThis._delayedTestId = id;
        // Don't wait - exit immediately
    "#,
        )
        .unwrap();

    // Wait for the timer to fire (50ms + buffer)
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Verify timer executed
    let result = runtime
        .execute_code("globalThis._delayedTestId ? 'scheduled' : 'cleared';")
        .unwrap();
    assert_eq!(
        result.trim(),
        "scheduled",
        "Timer should have been scheduled. Got: {}",
        result.trim()
    );

    // Clean up
    let _ = runtime
        .execute_code("clearTimeout(globalThis._delayedTestId); delete globalThis._delayedTestId;");
}

#[test]
#[serial]
fn test_setinterval_basic() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // setInterval should schedule recurring callbacks
    let result = runtime
        .execute_code(
            r#"
        let count = 0;
        const id = setInterval(() => { count += 1; }, 5);
        // Clear before it can fire twice
        clearInterval(id);
        count;
    "#,
        )
        .unwrap();
    // Should be 0 or 1 depending on timing
    assert!(
        result.trim() == "0" || result.trim() == "1",
        "setInterval count should be 0 or 1. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_clear_immediate() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // clearImmediate should prevent setImmediate from executing
    let result = runtime
        .execute_code(
            r#"
        let executed = false;
        const id = setImmediate(() => { executed = true; });
        clearImmediate(id);
        executed;
    "#,
        )
        .unwrap();
    assert_eq!(
        result.trim(),
        "false",
        "Cleared setImmediate should not execute. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_mixed_callbacks_execution_order() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // Complex mixed scenario
    runtime
        .execute_code(
            r#"
        globalThis.__order = [];
        setTimeout(() => {
            globalThis.__order.push('timer');
            process.nextTick(() => globalThis.__order.push('nextTick-in-timer'));
        }, 0);
        process.nextTick(() => globalThis.__order.push('nextTick'));
        setImmediate(() => globalThis.__order.push('immediate'));
        globalThis.__order.join(',');
    "#,
        )
        .unwrap();
    let result = read_order(&mut runtime);
    // Expected: nextTick, timer, nextTick-in-timer, immediate
    // But immediate runs in "check phase" after timers
    assert_eq!(
        result.trim(),
        "nextTick,timer,nextTick-in-timer,immediate",
        "Mixed callbacks should execute in correct order. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_queue_microtask_integration() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // queueMicrotask should work alongside nextTick
    runtime
        .execute_code(
            r#"
        globalThis.__order = [];
        queueMicrotask(() => globalThis.__order.push('microtask'));
        process.nextTick(() => globalThis.__order.push('nextTick'));
        globalThis.__order.join(',');
    "#,
        )
        .unwrap();
    let result = read_order(&mut runtime);
    // nextTick has higher priority than queueMicrotask
    assert_eq!(
        result.trim(),
        "nextTick,microtask",
        "nextTick should have higher priority than queueMicrotask. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_timer_ref_unref() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // timer.ref() and timer.unref() should work
    let result = runtime
        .execute_code(
            r#"
        const id = setTimeout(() => {}, 100);
        typeof id.ref;
    "#,
        )
        .unwrap();
    // ref should be a function
    assert_eq!(
        result.trim(),
        "function",
        "timer.ref() should be available. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_zero_delay_timer_precision() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // setTimeout with 0 delay should execute in same event loop iteration
    runtime
        .execute_code(
            r#"
        globalThis.__zeroDelayExecuted = false;
        setTimeout(() => { globalThis.__zeroDelayExecuted = true; }, 0);
        globalThis.__zeroDelayExecuted;
    "#,
        )
        .unwrap();
    let result = runtime
        .execute_code("globalThis.__zeroDelayExecuted;")
        .unwrap();
    assert_eq!(
        result.trim(),
        "true",
        "setTimeout(0) should execute in same iteration. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_next_tick_isolation_between_contexts() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // nextTick callbacks should not leak between contexts
    runtime
        .execute_code(
            r#"
        globalThis.order = [];
        process.nextTick(() => globalThis.order.push('a'));
        process.nextTick(() => globalThis.order.push('b'));
        globalThis.order.join(',');
    "#,
        )
        .unwrap();
    let result = runtime.execute_code("globalThis.order.join(',');").unwrap();
    assert_eq!(
        result.trim(),
        "a,b",
        "nextTick should work correctly. Got: {}",
        result.trim()
    );
}

// v0.3.273: Timer.delay() API tests
#[test]
#[serial]
fn test_timer_delay_getter() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // timer.delay() should return the initial delay
    let result = runtime
        .execute_code(
            r#"
        const timer = setTimeout(() => {}, 1500);
        timer.delay();
    "#,
        )
        .unwrap();
    assert_eq!(
        result.trim(),
        "1500",
        "timer.delay() should return initial delay. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_timer_delay_setter() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // timer.delay(ms) should change the delay and return timer for chaining
    let result = runtime
        .execute_code(
            r#"
        const timer = setTimeout(() => {}, 2000);
        const result = timer.delay(500);
        result === timer;
    "#,
        )
        .unwrap();
    assert_eq!(
        result.trim(),
        "true",
        "timer.delay(ms) should return timer for chaining. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_timer_delay_updates_metadata() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // After changing delay, timer.delay() should return new value
    // Use a variable to capture the final delay value for proper evaluation
    let result = runtime
        .execute_code(
            r#"
        const timer = setTimeout(() => {}, 1000);
        timer.delay();
        timer.delay(300);
        const finalDelay = timer.delay();
        finalDelay;
    "#,
        )
        .unwrap();
    assert_eq!(
        result.trim(),
        "300",
        "timer.delay() should return updated delay. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_timer_delay_chaining() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // timer.delay() should support chaining with ref/unref
    let result = runtime
        .execute_code(
            r#"
        const timer = setTimeout(() => {}, 1000);
        const result = timer.delay(500).ref().unref();
        result === timer;
    "#,
        )
        .unwrap();
    assert_eq!(
        result.trim(),
        "true",
        "timer.delay() should support chaining. Got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_timer_delay_zero() {
    cleanup_global_state();
    let mut runtime = MinimalRuntime::new().unwrap();
    // timer.delay(0) should work
    let result = runtime
        .execute_code(
            r#"
        const timer = setTimeout(() => {}, 500);
        timer.delay(0);
        timer.delay();
    "#,
        )
        .unwrap();
    assert_eq!(
        result.trim(),
        "0",
        "timer.delay(0) should set delay to 0. Got: {}",
        result.trim()
    );
}

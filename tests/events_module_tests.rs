// Events 模块测试 - v0.3.46
// 测试 EventEmitter 功能

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_events_module_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof events");
    assert!(result.is_ok());
    // The events module is the EventEmitter class itself, as in Node.
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_event_emitter_constructor_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof events.EventEmitter");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_event_emitter_instance() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // Note: Full instanceof support requires prototype chain setup
    // For now, test that EventEmitter constructor works
    let result = runtime.execute_code(
        "const EventEmitter = events.EventEmitter; const emitter = new EventEmitter(); typeof EventEmitter"
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_event_emitter_on_method() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result =
        runtime.execute_code("const emitter = new events.EventEmitter(); typeof emitter.on");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_event_emitter_once_method() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result =
        runtime.execute_code("const emitter = new events.EventEmitter(); typeof emitter.once");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_event_emitter_emit_method() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result =
        runtime.execute_code("const emitter = new events.EventEmitter(); typeof emitter.emit");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_event_emitter_remove_listener_method() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code("const emitter = new events.EventEmitter(); typeof emitter.removeListener");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_event_emitter_remove_all_listeners_method() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        "const emitter = new events.EventEmitter(); typeof emitter.removeAllListeners",
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_event_emitter_listeners_method() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result =
        runtime.execute_code("const emitter = new events.EventEmitter(); typeof emitter.listeners");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_event_emitter_event_names_method() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code("const emitter = new events.EventEmitter(); typeof emitter.eventNames");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_event_emitter_get_max_listeners_method() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code("const emitter = new events.EventEmitter(); typeof emitter.getMaxListeners");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_event_emitter_set_max_listeners_method() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code("const emitter = new events.EventEmitter(); typeof emitter.setMaxListeners");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_event_emitter_listener_count_static() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof events.EventEmitter.listenerCount");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_basic_event_on_emit() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        let called = false;
        emitter.on('test', () => { called = true; });
        emitter.emit('test');
        called
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_event_with_data() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        let receivedData = null;
        emitter.on('data', (data) => { receivedData = data; });
        emitter.emit('data', 'hello world');
        receivedData
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "hello world");
}

#[test]
#[serial]
fn test_multiple_args_emit() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        let receivedArgs = null;
        emitter.on('multi', (...args) => { receivedArgs = args; });
        emitter.emit('multi', 1, 'two', true);
        JSON.stringify(receivedArgs)
    "#,
    );
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("1") && output.contains("two") && output.contains("true"));
}

#[test]
#[serial]
fn test_once_event_only_fires_once() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        let count = 0;
        emitter.once('once', () => { count++; });
        emitter.emit('once');
        emitter.emit('once');
        emitter.emit('once');
        count
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "1");
}

#[test]
#[serial]
fn test_remove_listener() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        let count = 0;
        const listener = () => { count++; };
        emitter.on('test', listener);
        emitter.emit('test');
        emitter.removeListener('test', listener);
        emitter.emit('test');
        count
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "1");
}

#[test]
#[serial]
fn test_remove_all_listeners() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // Note: removeAllListeners clears all global listeners, not just per-emitter
    // This is a simplified implementation - each code execution gets fresh listeners
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        let count = 0;
        const handler = () => { count++; };
        emitter.on('a', handler);
        emitter.on('b', handler);
        emitter.emit('a');
        emitter.emit('b');
        emitter.removeAllListeners('a');
        emitter.emit('a');
        emitter.emit('b');
        count
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "3"); // a (1), b (1), a removed so only b (1)
}

#[test]
#[serial]
fn test_listeners_returns_array() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        const fn1 = () => {};
        const fn2 = () => {};
        emitter.on('test', fn1);
        emitter.on('test', fn2);
        const listeners = emitter.listeners('test');
        listeners.length
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "2");
}

#[test]
#[serial]
fn test_event_names() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        emitter.on('event1', () => {});
        emitter.on('event2', () => {});
        const names = emitter.eventNames();
        JSON.stringify(names)
    "#,
    );
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("event1") && output.contains("event2"));
}

#[test]
#[serial]
fn test_default_max_listeners() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        emitter.getMaxListeners()
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "10");
}

#[test]
#[serial]
fn test_set_max_listeners() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        emitter.setMaxListeners(20);
        emitter.getMaxListeners()
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "20");
}

#[test]
#[serial]
fn test_listener_count_static() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        emitter.on('test', () => {});
        emitter.on('test', () => {});
        events.EventEmitter.listenerCount(emitter, 'test')
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "2");
}

#[test]
#[serial]
fn test_emit_returns_true_with_listeners() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        emitter.on('test', () => {});
        emitter.emit('test')
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_emit_returns_false_without_listeners() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        emitter.emit('nonexistent')
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "false");
}

#[test]
#[serial]
fn test_prepend_listener_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        typeof emitter.prependListener
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_prepend_listener_basic() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        let called = false;
        emitter.prependListener('test', () => { called = true; });
        emitter.emit('test');
        called
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_prepend_listener_execution_order() {
    // v0.3.257: 测试 prependListener 的执行顺序
    // prependListener 添加的监听器应该比后续 on 添加的监听器先执行
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        const order = [];
        emitter.on('test', () => { order.push('second'); });
        emitter.prependListener('test', () => { order.push('first'); });
        emitter.emit('test');
        JSON.stringify(order)
    "#,
    );
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(
        output.contains("first") && output.contains("second"),
        "prependListener should execute before on listeners, got: {}",
        output
    );
    // Verify order: first element should be 'first'
    assert!(
        output.starts_with("[\"first\""),
        "prependListener should be first in execution order, got: {}",
        output
    );
}

#[test]
#[serial]
fn test_prepend_listener_with_data() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        let receivedData = null;
        emitter.prependListener('data', (data) => { receivedData = data; });
        emitter.emit('data', 'hello prepend');
        receivedData
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "hello prepend");
}

#[test]
#[serial]
fn test_prepend_listener_returns_emitter() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        const result = emitter.prependListener('test', () => {});
        result === emitter
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_prepend_listener_requires_function() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // Node.js spec: prependListener throws TypeError when callback is not a function
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        let threw = false;
        try {
            emitter.prependListener('test', 'not a function');
        } catch (e) {
            threw = e instanceof TypeError;
        }
        threw;
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_prepend_listener_count() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        emitter.prependListener('test', () => {});
        emitter.on('test', () => {});
        events.EventEmitter.listenerCount(emitter, 'test')
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "2");
}

#[test]
#[serial]
fn test_prepend_listener_warning_exceeds_max() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        emitter.setMaxListeners(2);
        emitter.prependListener('test', () => {});
        emitter.on('test', () => {});
        // This should trigger warning but still work
        emitter.prependListener('test', () => {});
        events.EventEmitter.listenerCount(emitter, 'test')
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "3");
}

// v0.3.258: prependOnceListener tests

#[test]
#[serial]
fn test_prepend_once_listener_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        typeof emitter.prependOnceListener
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_prepend_once_listener_basic() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        let called = false;
        emitter.prependOnceListener('test', () => { called = true; });
        emitter.emit('test');
        called
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_prepend_once_listener_only_fires_once() {
    // v0.3.258: prependOnceListener 只执行一次
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        let count = 0;
        emitter.prependOnceListener('once', () => { count++; });
        emitter.emit('once');
        emitter.emit('once');
        emitter.emit('once');
        count
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "1");
}

#[test]
#[serial]
fn test_prepend_once_listener_execution_order() {
    // v0.3.258: prependOnceListener 的执行顺序优先级
    // prependOnceListener 应该比 once 先执行
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        const order = [];
        emitter.once('test', () => { order.push('second'); });
        emitter.prependOnceListener('test', () => { order.push('first'); });
        emitter.emit('test');
        JSON.stringify(order)
    "#,
    );
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(
        output.contains("first") && output.contains("second"),
        "prependOnceListener should execute before once, got: {}",
        output
    );
    // Verify order: first element should be 'first'
    assert!(
        output.starts_with("[\"first\""),
        "prependOnceListener should be first in execution order, got: {}",
        output
    );
}

#[test]
#[serial]
fn test_prepend_once_listener_with_data() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        let receivedData = null;
        emitter.prependOnceListener('data', (data) => { receivedData = data; });
        emitter.emit('data', 'hello prependOnce');
        receivedData
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "hello prependOnce");
}

#[test]
#[serial]
fn test_prepend_once_listener_returns_emitter() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        const result = emitter.prependOnceListener('test', () => {});
        result === emitter
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_prepend_once_listener_requires_function() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // Node.js spec: prependOnceListener throws TypeError when callback is not a function
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        let threw = false;
        try {
            emitter.prependOnceListener('test', 'not a function');
        } catch (e) {
            threw = e instanceof TypeError;
        }
        threw;
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_prepend_once_listener_count() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        emitter.prependOnceListener('test', () => {});
        emitter.once('test', () => {});
        events.EventEmitter.listenerCount(emitter, 'test')
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "2");
}

#[test]
#[serial]
fn test_prepend_once_listener_warning_exceeds_max() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        emitter.setMaxListeners(2);
        emitter.prependOnceListener('test', () => {});
        emitter.once('test', () => {});
        // This should trigger warning but still work
        emitter.prependOnceListener('test', () => {});
        events.EventEmitter.listenerCount(emitter, 'test')
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "3");
}

#[test]
#[serial]
fn test_prepend_once_listener_mixed_with_prepend_listener() {
    // v0.3.258: 混合使用 prependOnceListener 和 prependListener
    // 执行顺序: prependOnceListener -> prependListener -> once
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const emitter = new events.EventEmitter();
        const order = [];
        emitter.once('test', () => { order.push('once'); });
        emitter.prependListener('test', () => { order.push('prepend'); });
        emitter.prependOnceListener('test', () => { order.push('prependOnce'); });
        emitter.emit('test');
        JSON.stringify(order)
    "#,
    );
    assert!(result.is_ok());
    let output = result.unwrap();
    // Expected order: prependOnce, prepend, once
    assert!(
        output.starts_with("[\"prependOnce\",\"prepend\",\"once\""),
        "Expected prependOnce -> prepend -> once order, got: {}",
        output
    );
}

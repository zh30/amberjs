// Stage 43.0: Node.js 兼容性测试
// 验证新实现的Node.js API兼容性

use amberjs::Runtime;
use serial_test::serial;

#[test]
#[serial]
fn test_nodejs_crypto_basic() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let code = r#"
        const crypto = require('crypto');
        const hash = crypto.createHash('sha256');
        hash.update('test data');
        const result = hash.digest('hex');
        result.length === 64;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(result_str.contains("true"));
}

#[test]
#[serial]
fn test_nodejs_buffer_basic() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let code = r#"
        const buf = Buffer.from('Hello, World!', 'utf8');
        buf.toString('utf8') === 'Hello, World!';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(result_str.contains("true"));
}

#[test]
#[serial]
fn test_nodejs_events_basic() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let code = r#"
        const EventEmitter = require('events');
        const emitter = new EventEmitter();
        let called = false;
        emitter.on('test', () => {
            called = true;
        });
        emitter.emit('test');
        called;
    "#;
    let result = runtime.execute_code(code);
    let result_str = result.expect("require('events') should be a constructor");
    assert!(result_str.contains("true"), "got: {result_str}");
}

/// Node's `events` module *is* the EventEmitter class, and carries a
/// self-reference so destructuring yields the same constructor.
#[test]
#[serial]
fn test_events_module_is_the_event_emitter_class() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let code = r#"
        const events = require('events');
        const { EventEmitter } = require('events');
        [
            typeof events,
            String(EventEmitter === events),
            String(new events().on !== undefined),
            String(new EventEmitter().emit !== undefined),
        ].join(",");
    "#;
    let result = runtime
        .execute_code(code)
        .expect("events module shape should match Node");
    assert!(result.contains("function,true,true,true"), "got: {result}");
}

#[test]
#[serial]
fn test_nodejs_util_basic() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let code = r#"
        const util = require('util');
        const obj = { name: 'test', value: 42 };
        const inspected = util.inspect(obj);
        inspected.includes('name');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(result_str.contains("true"));
}

#[test]
#[serial]
fn test_nodejs_os_basic() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let code = r#"
        const os = require('os');
        const platform = os.platform();
        typeof platform === 'string' && platform.length > 0;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(result_str.contains("true"));
}

#[test]
#[serial]
fn test_nodejs_path_basic() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let code = r#"
        const path = require('path');
        const result = path.join('foo', 'bar', 'baz');
        result.length > 0;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(result_str.contains("true"));
}

#[test]
#[serial]
fn test_nodejs_url_basic() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let code = r#"
        const { URL } = require('url');
        const url = new URL('https://example.com:8080/path?query=value#hash');
        url.hostname === 'example.com';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(result_str.contains("true"));
}

// Buffer API tests for Amber runtime
// v0.2.8: Comprehensive Buffer API testing

use serial_test::serial;

#[test]
#[serial]
fn test_buffer_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof Buffer")
        .expect("Execution failed");
    assert_eq!(result.trim(), "function", "Buffer should be a function");
}

#[test]
#[serial]
fn test_buffer_from_string() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.from('Hello', 'utf8');
        buf.length;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "5",
        "Buffer.from should create buffer with correct length"
    );
}

#[test]
#[serial]
fn test_buffer_from_string_content() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.from('Test', 'utf8');
        const str = buf.toString('utf8');
        str;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "Test",
        "Buffer.toString should return original string"
    );
}

#[test]
#[serial]
fn test_buffer_alloc() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.alloc(10);
        buf.length;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "10",
        "Buffer.alloc should create buffer with specified size"
    );
}

#[test]
#[serial]
fn test_buffer_concat() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf1 = Buffer.from('Hello', 'utf8');
        const buf2 = Buffer.from('World', 'utf8');
        const buf = Buffer.concat([buf1, buf2]);
        buf.toString('utf8');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "HelloWorld",
        "Buffer.concat should concatenate buffers"
    );
}

#[test]
#[serial]
fn test_buffer_byte_length() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const len = Buffer.byteLength('Hello', 'utf8');
        len;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "5",
        "Buffer.byteLength should return string byte length"
    );
}

#[test]
#[serial]
fn test_buffer_is_buffer() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.from('test', 'utf8');
        Buffer.isBuffer(buf);
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Buffer.isBuffer should return true for buffer"
    );
}

#[test]
#[serial]
fn test_buffer_slice() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.from('Hello World', 'utf8');
        const sliced = buf.slice(0, 5);
        sliced.toString('utf8');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "Hello",
        "Buffer.slice should return sliced buffer"
    );
}

#[test]
#[serial]
fn test_buffer_slice_length() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.from('Hello World', 'utf8');
        const sliced = buf.slice(6, 11);
        sliced.length;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "5",
        "Buffer.slice should preserve correct length"
    );
}

#[test]
#[serial]
fn test_buffer_slice_negative_start() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.from('Hello World', 'utf8');
        const sliced = buf.slice(-5);
        sliced.toString('utf8');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "World",
        "Buffer.slice with negative start should work"
    );
}

#[test]
#[serial]
fn test_buffer_slice_negative_end() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.from('Hello World', 'utf8');
        const sliced = buf.slice(0, -6);
        sliced.toString('utf8');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "Hello",
        "Buffer.slice with negative end should work"
    );
}

#[test]
#[serial]
fn test_buffer_slice_empty() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.from('Hello', 'utf8');
        const sliced = buf.slice(5, 5);
        sliced.length;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "0",
        "Buffer.slice with same start/end should return empty buffer"
    );
}

#[test]
#[serial]
fn test_buffer_slice_out_of_bounds() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.from('Hello', 'utf8');
        const sliced = buf.slice(0, 100);
        sliced.length;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "5",
        "Buffer.slice with out-of-bounds end should return full buffer"
    );
}

#[test]
#[serial]
fn test_global_this_buffer() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = globalThis.Buffer.from('Test', 'utf8');
        buf.toString('utf8');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "Test", "globalThis.Buffer should work");
}

#[test]
#[serial]
fn test_global_this_fetch() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof globalThis.fetch;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "globalThis.fetch should be a function"
    );
}

#[test]
#[serial]
fn test_global_this_console() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof globalThis.console;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "globalThis.console should be an object"
    );
}

#[test]
#[serial]
fn test_global_this_process() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof globalThis.process;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "globalThis.process should be an object"
    );
}

#[test]
#[serial]
fn test_global_this_set_timeout() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof globalThis.setTimeout;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "globalThis.setTimeout should be a function"
    );
}

#[test]
#[serial]
fn test_global_this_math() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof globalThis.Math;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "globalThis.Math should be an object"
    );
}

#[test]
#[serial]
fn test_global_this_json() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof globalThis.JSON;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "globalThis.JSON should be an object"
    );
}

#[test]
#[serial]
fn test_global_this_url() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof globalThis.URL;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "globalThis.URL should be a function"
    );
}

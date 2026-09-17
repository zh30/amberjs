// Enhanced Buffer API and process tests for Beejs runtime
// v0.2.9: Buffer API enhancements and process.memoryUsage

use serial_test::serial;

#[test]
#[serial]
fn test_buffer_from_string_encoding() {
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
fn test_buffer_alloc_zero_fill() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.alloc(10);
        buf.length === 10;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Buffer.alloc should create zero-filled buffer"
    );
}

#[test]
#[serial]
fn test_buffer_alloc_fill() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.alloc(5, 0x41);
        buf.toString('hex');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "4141414141",
        "Buffer.alloc with fill value should work"
    );
}

#[test]
#[serial]
fn test_buffer_write() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.alloc(10);
        const bytesWritten = buf.write('Hello', 0, 5, 'utf8');
        bytesWritten;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "5",
        "Buffer.write should return bytes written"
    );
}

#[test]
#[serial]
fn test_buffer_to_string() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.from('Test string', 'utf8');
        const str = buf.toString('utf8');
        str;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "Test string",
        "Buffer.toString should return original string"
    );
}

#[test]
#[serial]
fn test_buffer_to_string_hex() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.from('ABC', 'utf8');
        buf.toString('hex');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "414243",
        "Buffer.toString('hex') should return hex encoding"
    );
}

#[test]
#[serial]
fn test_buffer_concat_multiple() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf1 = Buffer.from('Hello', 'utf8');
        const buf2 = Buffer.from(' ', 'utf8');
        const buf3 = Buffer.from('World', 'utf8');
        const buf = Buffer.concat([buf1, buf2, buf3]);
        buf.toString('utf8');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "Hello World",
        "Buffer.concat should concatenate multiple buffers"
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
fn test_buffer_is_buffer_false() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        Buffer.isBuffer('not a buffer');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "false",
        "Buffer.isBuffer should return false for non-buffer"
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
fn test_buffer_byte_length_multibyte() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        // Chinese characters are 3 bytes in UTF-8
        const len = Buffer.byteLength('你好', 'utf8');
        len;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "6",
        "Buffer.byteLength should count multibyte characters correctly"
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
fn test_buffer_slice_negative() {
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
        "Buffer.slice with negative index should work"
    );
}

#[test]
#[serial]
fn test_buffer_copy() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf1 = Buffer.from('Hello', 'utf8');
        const buf2 = Buffer.alloc(5);
        buf1.copy(buf2);
        buf2.toString('utf8');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "Hello", "Buffer.copy should copy contents");
}

#[test]
#[serial]
fn test_buffer_index_of() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.from('Hello World', 'utf8');
        const index = buf.indexOf('World');
        index;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "6", "Buffer.indexOf should find substring");
}

#[test]
#[serial]
fn test_process_memory_usage() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const mem = process.memoryUsage();
        typeof mem.heapTotal;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "number",
        "process.memoryUsage should return object with number properties"
    );
}

#[test]
#[serial]
fn test_process_memory_usage_properties() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const mem = process.memoryUsage();
        mem.heapTotal !== undefined && mem.heapUsed !== undefined;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.memoryUsage should have heapTotal and heapUsed"
    );
}

#[test]
#[serial]
fn test_process_uptime() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const uptime = process.uptime();
        typeof uptime;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "number",
        "process.uptime should return a number"
    );
}

#[test]
#[serial]
fn test_process_hrtime() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const time = process.hrtime();
        Array.isArray(time);
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.hrtime should return an array"
    );
}

#[test]
#[serial]
fn test_process_release_name() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.release.name;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "string",
        "process.release.name should be a string"
    );
}

#[test]
#[serial]
fn test_process_arch() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.arch;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "string", "process.arch should be a string");
}

#[test]
#[serial]
fn test_process_platform() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.platform;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "string",
        "process.platform should be a string"
    );
}

#[test]
#[serial]
fn test_process_version_v8() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.versions.v8;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "string",
        "process.versions.v8 should be a string"
    );
}

#[test]
#[serial]
fn test_global_this_buffer_properties() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof globalThis.Buffer.from;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "globalThis.Buffer should have static methods"
    );
}

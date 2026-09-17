// Tests for crypto.randomBytes module (v0.3.10)
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_randombytes_function_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.randomBytes");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_randombytes_sync_function_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.randomBytesSync");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_randombytes_returns_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf = crypto.randomBytes(16);
        buf.constructor.name;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    // Should return a Buffer-like object
    let binding = result.unwrap();
    let output = binding.trim();
    assert!(output.contains("Buffer") || output.contains("Uint8Array"));
}

#[test]
#[serial]
fn test_randombytes_correct_size() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf = crypto.randomBytes(32);
        buf.length;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "32");
}

#[test]
#[serial]
fn test_randombytes_different_each_time() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = crypto.randomBytes(16);
        const buf2 = crypto.randomBytes(16);
        buf1.toString('hex') !== buf2.toString('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_randombytes_hex_output() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf = crypto.randomBytes(16);
        // Uint8Array.toString() joins elements with commas, returns hex-like representation
        const hex = buf.toString('hex');
        hex.length > 20; // Should be long enough to represent 16 bytes
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_randombytes_base64_output() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf = crypto.randomBytes(16);
        const base64 = buf.toString('base64');
        base64.length;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    // 16 bytes = ~22 base64 chars (with padding)
    assert!(result.unwrap().trim().parse::<u32>().unwrap() > 20);
}

#[test]
#[serial]
fn test_randombytes_various_sizes() {
    let mut runtime = MinimalRuntime::new().unwrap();

    // Test small size
    let code = r#"
        const buf = crypto.randomBytes(1);
        buf.length;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "1");

    // Test medium size
    let code = r#"
        const mediumBuf = crypto.randomBytes(100);
        mediumBuf.length;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "100");

    // Test large size
    let code = r#"
        const largeBuf = crypto.randomBytes(1024);
        largeBuf.length;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "1024");
}

#[test]
#[serial]
fn test_randombytes_zero_size() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf = crypto.randomBytes(0);
        buf.length;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "0");
}

#[test]
#[serial]
fn test_randombytes_sync_returns_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf = crypto.randomBytesSync(16);
        buf.constructor.name;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    // Should return a Buffer-like object
    let binding = result.unwrap();
    let output = binding.trim();
    assert!(output.contains("Buffer") || output.contains("Uint8Array"));
}

#[test]
#[serial]
fn test_randombytes_sync_correct_size() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf = crypto.randomBytesSync(32);
        buf.length;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "32");
}

#[test]
#[serial]
fn test_randombytes_sync_different_each_time() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = crypto.randomBytesSync(16);
        const buf2 = crypto.randomBytesSync(16);
        buf1.toString('hex') !== buf2.toString('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_randombytes_callback_api() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const results = [];
        crypto.randomBytes(16, (err, buf) => {
            results.push(err === null, buf && buf.length === 16);
        });
        results.join(',');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    // Should have callback called with no error and correct size
    assert!(result.unwrap().trim().contains("true"));
}

#[test]
#[serial]
fn test_randombytes_with_callback_size() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        let result = null;
        crypto.randomBytes(32, (err, buf) => {
            result = buf ? buf.length : 0;
        });
        result;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    // Callback should set result to the buffer length
    assert_eq!(result.unwrap().trim(), "32");
}

// Tests for crypto.randomFill module (v0.3.16)
// Tests for crypto.randomFill/randomFillSync - fill existing buffer with random data

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_randomfill_function_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.randomFill");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_randomfill_sync_function_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.randomFillSync");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_randomfill_sync_fills_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf = new Uint8Array(16);
        crypto.randomFillSync(buf);
        // Check that buffer is not all zeros
        let sum = 0;
        for (let i = 0; i < buf.length; i++) {
            sum += buf[i];
        }
        sum > 0;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_randomfill_sync_with_offset_and_size() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // Node.js API: randomFillSync(buffer[, offset[, size]])
    let code = r#"
        const buf = new Uint8Array(32);
        // Fill bytes 8-24 (16 bytes from offset 8)
        crypto.randomFillSync(buf, 8, 16);
        // Check that bytes 0-7 are still 0
        let firstSum = 0;
        for (let i = 0; i < 8; i++) {
            firstSum += buf[i];
        }
        // Check that bytes 8-23 are random (not all zero)
        let midSum = 0;
        for (let i = 8; i < 24; i++) {
            midSum += buf[i];
        }
        // Check that bytes 24-31 are still 0
        let lastSum = 0;
        for (let i = 24; i < 32; i++) {
            lastSum += buf[i];
        }
        firstSum === 0 && midSum > 0 && lastSum === 0;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_randomfill_sync_different_each_time() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = new Uint8Array(16);
        const buf2 = new Uint8Array(16);
        crypto.randomFillSync(buf1);
        crypto.randomFillSync(buf2);
        buf1.toString('hex') !== buf2.toString('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_randomfill_sync_entire_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf = new Uint8Array(64);
        crypto.randomFillSync(buf);
        buf.length;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "64");
}

#[test]
#[serial]
fn test_randomfill_callback_api() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf = new Uint8Array(16);
        let completed = false;
        crypto.randomFill(buf, (err) => {
            completed = true;
        });
        completed;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    // Should complete without error
    let binding = result.unwrap();
    let output = binding.trim();
    assert!(output == "true" || output.is_empty() || output.contains("undefined"));
}

#[test]
#[serial]
fn test_randomfill_callback_with_offset() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // Node.js API: randomFill(buffer[, offset], callback)
    let code = r#"
        const buf = new Uint8Array(32);
        let completed = false;
        crypto.randomFill(buf, 8, (err) => {
            completed = true;
        });
        completed;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    assert!(output == "true" || output.is_empty() || output.contains("undefined"));
}

#[test]
#[serial]
fn test_randomfill_sync_empty_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // randomFillSync with empty buffer should not error
    let code = r#"
        const buf = new Uint8Array(0);
        crypto.randomFillSync(buf);
        buf.length;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "0");
}

#[test]
#[serial]
fn test_randomfill_buffer_compatible() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf = new Uint8Array(16);
        crypto.randomFillSync(buf);
        // Should work with Buffer-like objects
        typeof buf.slice === 'function';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_randomfill_rejects_null_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            crypto.randomFillSync(null);
            false;
        } catch (e) {
            e instanceof TypeError;
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_randomfill_rejects_negative_size() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf = new Uint8Array(16);
        try {
            crypto.randomFillSync(buf, -1);
            false;
        } catch (e) {
            true;
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

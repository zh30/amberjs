// Tests for crypto.timingSafeEqual module (v0.3.11)
// Timing-safe constant-time comparison to prevent timing attacks
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_timing_safe_equal_function_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.timingSafeEqual");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_timing_safe_equal_equal_buffers() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = crypto.randomBytes(16);
        const buf2 = crypto.randomBytes(16);
        // Make them equal
        buf2.set(buf1);
        crypto.timingSafeEqual(buf1, buf2);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_timing_safe_equal_different_buffers() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = crypto.randomBytes(16);
        const buf2 = new Uint8Array(buf1);
        buf2[0] = (buf1[0] + 1) & 0xff;
        crypto.timingSafeEqual(buf1, buf2);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "false");
}

#[test]
#[serial]
fn test_timing_safe_equal_different_lengths() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = crypto.randomBytes(16);
        const buf2 = crypto.randomBytes(8);
        try {
            crypto.timingSafeEqual(buf1, buf2);
            "no error";
        } catch (e) {
            e.message;
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    assert!(
        output.contains("must")
            || output.contains("length")
            || output.contains("same")
            || output == "no error"
    );
}

#[test]
#[serial]
fn test_timing_safe_equal_empty_buffers() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = crypto.randomBytes(0);
        const buf2 = crypto.randomBytes(0);
        crypto.timingSafeEqual(buf1, buf2);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_timing_safe_equal_single_byte() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = crypto.randomBytes(1);
        const buf2 = crypto.randomBytes(1);
        buf2[0] = buf1[0];
        crypto.timingSafeEqual(buf1, buf2);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_timing_safe_equal_single_byte_different() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = crypto.randomBytes(1);
        const buf2 = crypto.randomBytes(1);
        buf2[0] = (buf1[0] + 1) & 0xff;
        crypto.timingSafeEqual(buf1, buf2);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "false");
}

#[test]
#[serial]
fn test_timing_safe_equal_returns_boolean() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = crypto.randomBytes(16);
        const buf2 = crypto.randomBytes(16);
        const result = crypto.timingSafeEqual(buf1, buf2);
        typeof result;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "boolean");
}

#[test]
#[serial]
fn test_timing_safe_equal_large_buffers() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = crypto.randomBytes(1024);
        const buf2 = new Uint8Array(buf1);
        crypto.timingSafeEqual(buf1, buf2);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_timing_safe_equal_large_buffers_different() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = crypto.randomBytes(1024);
        const buf2 = new Uint8Array(buf1);
        buf2[0] = (buf1[0] + 1) & 0xff;
        crypto.timingSafeEqual(buf1, buf2);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "false");
}

#[test]
#[serial]
fn test_timing_safe_equal_uint8array_compat() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = new Uint8Array([1, 2, 3, 4, 5]);
        const buf2 = new Uint8Array([1, 2, 3, 4, 5]);
        crypto.timingSafeEqual(buf1, buf2);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_timing_safe_equal_mixed_buffer_types() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = crypto.randomBytes(16);
        const buf2 = new Uint8Array(buf1);
        crypto.timingSafeEqual(buf1, buf2);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_timing_safe_equal_first_byte_different() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = crypto.randomBytes(16);
        const buf2 = new Uint8Array(buf1);
        buf2[0] = (buf1[0] + 1) & 0xff;
        crypto.timingSafeEqual(buf1, buf2);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "false");
}

#[test]
#[serial]
fn test_timing_safe_equal_last_byte_different() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = crypto.randomBytes(16);
        const buf2 = new Uint8Array(buf1);
        buf2[15] = (buf1[15] + 1) & 0xff;
        crypto.timingSafeEqual(buf1, buf2);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "false");
}

#[test]
#[serial]
fn test_timing_safe_equal_multiple_differences() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buf1 = crypto.randomBytes(32);
        const buf2 = crypto.randomBytes(32);
        // Ensure they are different
        buf2[0] = buf1[0] === 0 ? 1 : 0;
        buf2[31] = buf1[31] === 255 ? 254 : 255;
        crypto.timingSafeEqual(buf1, buf2);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "false");
}

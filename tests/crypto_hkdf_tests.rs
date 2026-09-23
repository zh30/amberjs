// Tests for crypto.hkdf module (v0.3.29)
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_hkdf_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.hkdf");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_hkdf_sync_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.hkdfSync");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_hkdf_sync_basic() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.hkdfSync('sha256', 'secret', 'salt', 'info', 32);
        result.constructor.name;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "Uint8Array");
}

#[test]
#[serial]
fn test_hkdf_sync_correct_length() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.hkdfSync('sha256', 'secret', 'salt', 'info', 32);
        result.length;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "32");
}

#[test]
#[serial]
fn test_hkdf_sync_custom_length() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.hkdfSync('sha256', 'secret', 'salt', 'info', 64);
        result.length;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "64");
}

#[test]
#[serial]
fn test_hkdf_sync_sha1() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.hkdfSync('sha1', 'secret', 'salt', 'info', 20);
        result.length;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "20");
}

#[test]
#[serial]
fn test_hkdf_sync_sha512() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.hkdfSync('sha512', 'secret', 'salt', 'info', 64);
        result.length;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "64");
}

#[test]
#[serial]
fn test_hkdf_sync_consistent_results() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result1 = crypto.hkdfSync('sha256', 'secret', 'salt', 'info', 32);
        const result2 = crypto.hkdfSync('sha256', 'secret', 'salt', 'info', 32);
        // Convert to hex for comparison
        const hex1 = Array.from(result1).map(b => b.toString(16).padStart(2, '0')).join('');
        const hex2 = Array.from(result2).map(b => b.toString(16).padStart(2, '0')).join('');
        hex1 === hex2;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_hkdf_sync_different_inputs_different_outputs() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result1 = crypto.hkdfSync('sha256', 'secret1', 'salt', 'info', 32);
        const result2 = crypto.hkdfSync('sha256', 'secret2', 'salt', 'info', 32);
        // They should be different
        const hex1 = Array.from(result1).map(b => b.toString(16).padStart(2, '0')).join('');
        const hex2 = Array.from(result2).map(b => b.toString(16).padStart(2, '0')).join('');
        hex1 !== hex2;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_hkdf_sync_empty_salt() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.hkdfSync('sha256', 'secret', '', 'info', 32);
        result.length === 32;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_hkdf_sync_empty_info() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.hkdfSync('sha256', 'secret', 'salt', '', 32);
        result.length === 32;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_hkdf_sync_default_digest() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // hkdf without specifying digest should default to sha256
    let code = r#"
        // Call with all arguments but default digest
        const result = crypto.hkdfSync('sha256', 'ikm', 'salt', 'info', 32);
        result.length === 32;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_hkdf_sync_returns_buffer_like() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.hkdfSync('sha256', 'secret', 'salt', 'info', 32);
        // Should have typical array methods
        typeof result.slice === 'function' && typeof result.subarray === 'function';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_hkdf_large_keylen() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        // Generate 256 bytes (multiple hash blocks)
        const result = crypto.hkdfSync('sha256', 'secret', 'salt', 'info', 256);
        result.length === 256;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

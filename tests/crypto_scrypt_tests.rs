// scrypt Tests - v0.3.25
// Tests for crypto.scrypt and crypto.scryptSync functions
// scrypt is a password-based key derivation function that is more resistant
// to hardware attacks than PBKDF2 due to its memory-hardness property.

use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn amberjs_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_amber"))
}

fn run_js_test(code: &str) -> String {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.js");
    fs::write(&test_file, code).unwrap();

    let output = Command::new(amberjs_path())
        .arg("run")
        .arg(&test_file)
        .output()
        .expect("Failed to execute amber");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    // Parse output - skip the "🐝 Running Amber on:" line and "Result:" line
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|line| !line.starts_with("🐝") && !line.starts_with("Result:"))
        .collect();
    lines.join("\n")
}

// ==================== scryptSync Tests ====================

#[test]
#[serial]
fn test_scrypt_sync_function_exists() {
    let code = r#"
console.log(typeof crypto.scryptSync === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.trim() == "PASS",
        "Expected scryptSync to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_sync_basic() {
    let code = r#"
const result = crypto.scryptSync('password', 'salt', 32);
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(result.length === 32 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected basic scryptSync to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_sync_known_rfc7914_vector() {
    let code = r#"
const expected = '77d6576238657b203b19ca42c18a0497f16b4844e3074ae8dfdffa3fede21442fcd0069ded0948f8326a753a0fc81f17e8d3e0fb2e0d3628cf35e20c38d18906';
const result = crypto.scryptSync('', '', 64, { N: 16, r: 1, p: 1 });
const hex = Array.from(result, b => b.toString(16).padStart(2, '0')).join('');
console.log(hex === expected ? 'PASS' : hex);
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected scryptSync to match RFC 7914 vector: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_sync_custom_keylen() {
    let code = r#"
const result16 = crypto.scryptSync('password', 'salt', 16);
const result64 = crypto.scryptSync('password', 'salt', 64);
console.log(result16.length === 16 ? 'PASS' : 'FAIL');
console.log(result64.length === 64 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected custom keylen to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_sync_with_options() {
    let code = r#"
const result = crypto.scryptSync('password', 'salt', 32, {
    N: 1024,
    r: 8,
    p: 1
});
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(result.length === 32 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected scryptSync with options to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_sync_different_salts() {
    let code = r#"
const result1 = crypto.scryptSync('password', 'salt1', 32);
const result2 = crypto.scryptSync('password', 'salt2', 32);
const areDifferent = result1.some((b, i) => b !== result2[i]);
console.log(areDifferent ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected different salts to produce different results: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_sync_different_passwords() {
    let code = r#"
const result1 = crypto.scryptSync('password1', 'salt', 32);
const result2 = crypto.scryptSync('password2', 'salt', 32);
const areDifferent = result1.some((b, i) => b !== result2[i]);
console.log(areDifferent ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected different passwords to produce different results: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_sync_empty_password() {
    let code = r#"
const result = crypto.scryptSync('', 'salt', 32);
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(result.length === 32 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected empty password to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_sync_empty_salt() {
    let code = r#"
const result = crypto.scryptSync('password', '', 32);
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(result.length === 32 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected empty salt to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_sync_returns_buffer() {
    let code = r#"
const result = crypto.scryptSync('password', 'salt', 64);
console.log(result.constructor.name === 'Uint8Array' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected Uint8Array return: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_sync_maxlen() {
    let code = r#"
const result = crypto.scryptSync('password', 'salt', 256);
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(result.length === 256 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected max keylen to work: {}",
        output
    );
}

// ==================== scrypt (Async) Tests ====================

#[test]
#[serial]
fn test_scrypt_function_exists() {
    let code = r#"
console.log(typeof crypto.scrypt === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.trim() == "PASS",
        "Expected scrypt to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_returns_promise() {
    let code = r#"
const result = crypto.scrypt('password', 'salt', 32);
console.log(result instanceof Promise ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected scrypt to return Promise: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_async_resolves() {
    let code = r#"
crypto.scrypt('password', 'salt', 32).then(result => {
    console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
    console.log(result.length === 32 ? 'PASS' : 'FAIL');
}).catch(() => {
    console.log('FAIL');
});
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected async scrypt to resolve: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_async_known_rfc7914_vector() {
    let code = r#"
(async () => {
    const expected = '77d6576238657b203b19ca42c18a0497f16b4844e3074ae8dfdffa3fede21442fcd0069ded0948f8326a753a0fc81f17e8d3e0fb2e0d3628cf35e20c38d18906';
    const result = await crypto.scrypt('', '', 64, { N: 16, r: 1, p: 1 });
    const hex = Array.from(result, b => b.toString(16).padStart(2, '0')).join('');
    console.log(hex === expected ? 'PASS' : hex);
})();
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected async scrypt to match RFC 7914 vector: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_async_basic() {
    let code = r#"
crypto.scrypt('password', 'salt', 32).then(result => {
    console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
}).catch(() => {
    console.log('FAIL');
});
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected async scrypt to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_async_with_options() {
    let code = r#"
crypto.scrypt('password', 'salt', 32, { N: 1024, r: 8, p: 1 }).then(result => {
    console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
    console.log(result.length === 32 ? 'PASS' : 'FAIL');
}).catch(() => {
    console.log('FAIL');
});
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected async scrypt with options to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_async_await() {
    let code = r#"
(async () => {
    const result = await crypto.scrypt('password', 'salt', 32);
    console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
    console.log(result.length === 32 ? 'PASS' : 'FAIL');
})();
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected async/await scrypt to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_async_consistent_with_sync() {
    let code = r#"
(async () => {
    const syncResult = crypto.scryptSync('password', 'salt', 32);
    const asyncResult = await crypto.scrypt('password', 'salt', 32);
    const areEqual = syncResult.every((b, i) => b === asyncResult[i]);
    console.log(areEqual ? 'PASS' : 'FAIL');
})();
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected async and sync to produce same result: {}",
        output
    );
}

// ==================== Callback Pattern Tests ====================

#[test]
#[serial]
fn test_scrypt_callback_pattern() {
    let code = r#"
crypto.scrypt('password', 'salt', 32, function(err, result) {
    if (err) {
        console.log('FAIL');
    } else {
        console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
    }
});
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected callback pattern to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_callback_with_options() {
    let code = r#"
crypto.scrypt('password', 'salt', 32, { N: 1024 }, function(err, result) {
    if (err) {
        console.log('FAIL');
    } else {
        console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
    }
});
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected callback with options to work: {}",
        output
    );
}

// ==================== Edge Cases ====================

#[test]
#[serial]
fn test_scrypt_sync_utf8_password() {
    let code = r#"
const result = crypto.scryptSync('密码', 'salt', 32);
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected UTF-8 password to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_sync_utf8_salt() {
    let code = r#"
const result = crypto.scryptSync('password', '盐', 32);
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected UTF-8 salt to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_sync_large_keylen() {
    let code = r#"
const result = crypto.scryptSync('password', 'salt', 128);
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(result.length === 128 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected large keylen to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_scrypt_sync_default_parameters() {
    let code = r#"
const result = crypto.scryptSync('password', 'salt', 32);
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(result.length === 32 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected default parameters to work: {}",
        output
    );
}

// ==================== Performance Note Tests ====================

#[test]
#[serial]
fn test_scrypt_low_cost_parameters() {
    // Use low cost parameters for fast testing
    let code = r#"
const result = crypto.scryptSync('password', 'salt', 16, { N: 16, r: 1, p: 1 });
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(result.length === 16 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected low cost parameters for fast testing: {}",
        output
    );
}

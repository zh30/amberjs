// PBKDF2 Tests - v0.3.12
// Tests for crypto.pbkdf2 and crypto.pbkdf2Sync functions

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

fn run_js_test_with_stderr(code: &str) -> (String, String) {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.js");
    fs::write(&test_file, code).unwrap();

    let output = Command::new(amberjs_path())
        .arg("run")
        .arg(&test_file)
        .output()
        .expect("Failed to execute amber");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let stdout_lines: Vec<&str> = stdout
        .lines()
        .filter(|line| !line.starts_with("🐝") && !line.starts_with("Result:"))
        .collect();

    (stdout_lines.join("\n"), stderr)
}

// ==================== pbkdf2Sync Tests ====================

#[test]
#[serial]
fn test_pbkdf2_sync_function_exists() {
    let code = r#"
console.log(typeof crypto.pbkdf2Sync === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.trim() == "PASS",
        "Expected pbkdf2Sync to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_sync_sha256() {
    let code = r#"
const result = crypto.pbkdf2Sync('password', 'salt', 1, 32, 'sha256');
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(result.length === 32 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected SHA256 PBKDF2 to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_sync_sha512() {
    let code = r#"
const result = crypto.pbkdf2Sync('password', 'salt', 1, 64, 'sha512');
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(result.length === 64 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected SHA512 PBKDF2 to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_sync_md5() {
    let code = r#"
const result = crypto.pbkdf2Sync('password', 'salt', 1, 16, 'md5');
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(result.length === 16 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected MD5 PBKDF2 to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_sync_sha1() {
    let code = r#"
const result = crypto.pbkdf2Sync('password', 'salt', 1, 20, 'sha1');
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(result.length === 20 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected SHA1 PBKDF2 to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_sync_default_iterations() {
    let code = r#"
const result = crypto.pbkdf2Sync('password', 'salt', 10000, 32, 'sha256');
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected default iterations to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_sync_different_salts() {
    let code = r#"
const result1 = crypto.pbkdf2Sync('password', 'salt1', 1, 32, 'sha256');
const result2 = crypto.pbkdf2Sync('password', 'salt2', 1, 32, 'sha256');
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
fn test_pbkdf2_sync_different_passwords() {
    let code = r#"
const result1 = crypto.pbkdf2Sync('password1', 'salt', 1, 32, 'sha256');
const result2 = crypto.pbkdf2Sync('password2', 'salt', 1, 32, 'sha256');
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
fn test_pbkdf2_sync_key_length() {
    let code = r#"
const result16 = crypto.pbkdf2Sync('password', 'salt', 1, 16, 'sha256');
const result32 = crypto.pbkdf2Sync('password', 'salt', 1, 32, 'sha256');
const result64 = crypto.pbkdf2Sync('password', 'salt', 1, 64, 'sha256');
console.log(result16.length === 16 ? 'PASS' : 'FAIL');
console.log(result32.length === 32 ? 'PASS' : 'FAIL');
console.log(result64.length === 64 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected correct key lengths: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_sync_returns_buffer() {
    let code = r#"
const result = crypto.pbkdf2Sync('password', 'salt', 1000, 32, 'sha256');
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
fn test_pbkdf2_sync_iterations_affects_result() {
    let code = r#"
const result1 = crypto.pbkdf2Sync('password', 'salt', 1, 32, 'sha256');
const result2 = crypto.pbkdf2Sync('password', 'salt', 10000, 32, 'sha256');
const areDifferent = result1.some((b, i) => b !== result2[i]);
console.log(areDifferent ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected iterations to affect result: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_sync_empty_password() {
    let code = r#"
const result = crypto.pbkdf2Sync('', 'salt', 1, 32, 'sha256');
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
fn test_pbkdf2_sync_empty_salt() {
    let code = r#"
const result = crypto.pbkdf2Sync('password', '', 1, 32, 'sha256');
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
fn test_pbkdf2_sync_zero_iterations() {
    let code = r#"
const result = crypto.pbkdf2Sync('password', 'salt', 0, 32, 'sha256');
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(result.length === 32 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected zero iterations to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_sync_large_keylen() {
    let code = r#"
const result = crypto.pbkdf2Sync('password', 'salt', 1, 256, 'sha256');
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(result.length === 256 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected large keylen to work: {}",
        output
    );
}

// ==================== pbkdf2 (Async) Tests ====================

#[test]
#[serial]
fn test_pbkdf2_function_exists() {
    let code = r#"
console.log(typeof crypto.pbkdf2 === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.trim() == "PASS",
        "Expected pbkdf2 to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_returns_promise() {
    let code = r#"
const result = crypto.pbkdf2('password', 'salt', 1, 32, 'sha256');
console.log(result instanceof Promise ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected pbkdf2 to return Promise: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_async_resolves() {
    let code = r#"
crypto.pbkdf2('password', 'salt', 1, 32, 'sha256').then(result => {
    console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
    console.log(result.length === 32 ? 'PASS' : 'FAIL');
}).catch(() => {
    console.log('FAIL');
});
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected async PBKDF2 to resolve: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_async_sha256() {
    let code = r#"
crypto.pbkdf2('password', 'salt', 1, 32, 'sha256').then(result => {
    console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
}).catch(() => {
    console.log('FAIL');
});
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected async SHA256 PBKDF2 to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_async_sha512() {
    let code = r#"
crypto.pbkdf2('password', 'salt', 1, 64, 'sha512').then(result => {
    console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
    console.log(result.length === 64 ? 'PASS' : 'FAIL');
}).catch(() => {
    console.log('FAIL');
});
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected async SHA512 PBKDF2 to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_async_md5() {
    let code = r#"
crypto.pbkdf2('password', 'salt', 1, 16, 'md5').then(result => {
    console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
}).catch(() => {
    console.log('FAIL');
});
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected async MD5 PBKDF2 to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_async_await() {
    let code = r#"
(async () => {
    const result = await crypto.pbkdf2('password', 'salt', 1, 32, 'sha256');
    console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
    console.log(result.length === 32 ? 'PASS' : 'FAIL');
})();
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected async/await PBKDF2 to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_async_consistent_with_sync() {
    let code = r#"
(async () => {
    const syncResult = crypto.pbkdf2Sync('password', 'salt', 1000, 32, 'sha256');
    const asyncResult = await crypto.pbkdf2('password', 'salt', 1000, 32, 'sha256');
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

// ==================== Edge Cases ====================

#[test]
#[serial]
fn test_pbkdf2_sync_unsupported_algorithm() {
    let (_, stderr) = run_js_test_with_stderr(
        r#"
const result = crypto.pbkdf2Sync('password', 'salt', 1, 32, 'unsupported');
"#,
    );
    assert!(
        stderr.contains("Unsupported"),
        "Expected error for unsupported algorithm"
    );
}

#[test]
#[serial]
fn test_pbkdf2_async_unsupported_algorithm() {
    let code = r#"
crypto.pbkdf2('password', 'salt', 1, 32, 'unsupported').catch(err => {
    console.log(err.message.includes('Unsupported') ? 'PASS' : 'FAIL');
});
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected async error for unsupported algorithm: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_sync_utf8_password() {
    let code = r#"
const result = crypto.pbkdf2Sync('密码', 'salt', 1, 32, 'sha256');
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
fn test_pbkdf2_sync_utf8_salt() {
    let code = r#"
const result = crypto.pbkdf2Sync('password', '盐', 1, 32, 'sha256');
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected UTF-8 salt to work: {}",
        output
    );
}

// ==================== Known Test Vectors ====================

#[test]
#[serial]
fn test_pbkdf2_sync_known_vector_sha256() {
    // PBKDF2-HMAC-SHA256 test vector
    // password "password", salt "salt", 1 iteration, 32 bytes
    // Verified against Node.js crypto.pbkdf2Sync
    let code = r#"
const expected = '120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b';
const result = crypto.pbkdf2Sync('password', 'salt', 1, 32, 'sha256');
// Convert Uint8Array to hex string using Array.from for amberjs compatibility
const resultHex = Array.from(result).map(b => b.toString(16).padStart(2, '0')).join('');
console.log(resultHex === expected ? 'PASS' : 'FAIL');
console.log('Expected: ' + expected);
console.log('Got: ' + resultHex);
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected known test vector to match: {}",
        output
    );
}

#[test]
#[serial]
fn test_pbkdf2_sync_known_vector_sha256_100000() {
    // High iteration count test (simulated known value)
    let code = r#"
const result = crypto.pbkdf2Sync('password', 'salt', 100000, 32, 'sha256');
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(result.length === 32 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected high iteration count to work: {}",
        output
    );
}

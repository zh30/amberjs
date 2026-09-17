// crypto.getHashes() Tests - v0.3.13
// Tests for crypto.getHashes() function to list supported hash algorithms

use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn beejs_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_amber"))
}

fn run_js_test(code: &str) -> String {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.js");
    fs::write(&test_file, code).unwrap();

    let output = Command::new(beejs_path())
        .arg("run")
        .arg(&test_file)
        .output()
        .expect("Failed to execute bee");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|line| !line.starts_with("🐝") && !line.starts_with("Result:"))
        .collect();
    lines.join("\n")
}

// ==================== getHashes Tests ====================

#[test]
#[serial]
fn test_get_hashes_function_exists() {
    let code = r#"
console.log(typeof crypto.getHashes === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.trim() == "PASS",
        "Expected getHashes to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_get_hashes_returns_array() {
    let code = r#"
const hashes = crypto.getHashes();
console.log(Array.isArray(hashes) ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected getHashes to return array: {}",
        output
    );
}

#[test]
#[serial]
fn test_get_hashes_contains_common_algorithms() {
    let code = r#"
const hashes = crypto.getHashes();
const hasSha256 = hashes.includes('sha256');
const hasSha512 = hashes.includes('sha512');
const hasMd5 = hashes.includes('md5');
const hasSha1 = hashes.includes('sha1');
console.log(hasSha256 && hasSha512 && hasMd5 && hasSha1 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected common algorithms to be present: {}",
        output
    );
}

#[test]
#[serial]
fn test_get_hashes_contains_blake3() {
    let code = r#"
const hashes = crypto.getHashes();
console.log(hashes.includes('blake3') ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected blake3 to be present: {}",
        output
    );
}

#[test]
#[serial]
fn test_get_hashes_no_duplicates() {
    let code = r#"
const hashes = crypto.getHashes();
const unique = new Set(hashes);
console.log(hashes.length === unique.size ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected no duplicate algorithms: {}",
        output
    );
}

#[test]
#[serial]
fn test_get_hashes_is_immutable() {
    let code = r#"
const hashes = crypto.getHashes();
const originalLength = hashes.length;
hashes.push('test');
console.log(crypto.getHashes().length === originalLength ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected getHashes to return immutable result: {}",
        output
    );
}

#[test]
#[serial]
fn test_get_hashes_minimum_count() {
    let code = r#"
const hashes = crypto.getHashes();
console.log(hashes.length >= 4 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected at least 4 hash algorithms: {}",
        output
    );
}

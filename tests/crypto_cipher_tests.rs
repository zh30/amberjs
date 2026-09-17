// crypto.createCipher/createDecipher Tests - v0.3.14
// Tests for crypto.createCipher and createDecipher symmetric encryption functions

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

// ==================== createCipher Tests ====================

#[test]
#[serial]
fn test_create_cipher_function_exists() {
    let code = r#"
console.log(typeof crypto.createCipher === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.trim() == "PASS",
        "Expected createCipher to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipher_aes_256_cbc() {
    let code = r#"
const cipher = crypto.createCipher('aes-256-cbc', 'password123');
console.log(cipher && typeof cipher.update === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected AES-256-CBC cipher to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipher_aes_128_cbc() {
    let code = r#"
const cipher = crypto.createCipher('aes-128-cbc', 'password123');
console.log(cipher && typeof cipher.update === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected AES-128-CBC cipher to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipher_aes_192_cbc() {
    let code = r#"
const cipher = crypto.createCipher('aes-192-cbc', 'password123');
console.log(cipher && typeof cipher.update === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected AES-192-CBC cipher to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipher_invalid_algorithm() {
    let code = r#"
try {
    crypto.createCipher('invalid-alg', 'password');
    console.log('FAIL');
} catch (e) {
    console.log(e.message.includes('unsupported algorithm') ? 'PASS' : 'FAIL');
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected error for invalid algorithm: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipher_has_update_method() {
    let code = r#"
const cipher = crypto.createCipher('aes-256-cbc', 'password');
console.log(typeof cipher.update === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected cipher to have update method: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipher_has_final_method() {
    let code = r#"
const cipher = crypto.createCipher('aes-256-cbc', 'password');
console.log(typeof cipher.final === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected cipher to have final method: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipher_has_set_auto_padding() {
    let code = r#"
const cipher = crypto.createCipher('aes-256-cbc', 'password');
console.log(typeof cipher.setAutoPadding === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected cipher to have setAutoPadding method: {}",
        output
    );
}

// ==================== createDecipher Tests ====================

#[test]
#[serial]
fn test_create_decipher_function_exists() {
    let code = r#"
console.log(typeof crypto.createDecipher === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.trim() == "PASS",
        "Expected createDecipher to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_decipher_aes_256_cbc() {
    let code = r#"
const decipher = crypto.createDecipher('aes-256-cbc', 'password123');
console.log(decipher && typeof decipher.update === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected AES-256-CBC decipher to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_decipher_aes_128_cbc() {
    let code = r#"
const decipher = crypto.createDecipher('aes-128-cbc', 'password123');
console.log(decipher && typeof decipher.update === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected AES-128-CBC decipher to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_decipher_aes_192_cbc() {
    let code = r#"
const decipher = crypto.createDecipher('aes-192-cbc', 'password123');
console.log(decipher && typeof decipher.update === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected AES-192-CBC decipher to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_decipher_invalid_algorithm() {
    let code = r#"
try {
    crypto.createDecipher('invalid-alg', 'password');
    console.log('FAIL');
} catch (e) {
    console.log(e.message.includes('unsupported algorithm') ? 'PASS' : 'FAIL');
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected error for invalid algorithm: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_decipher_has_update_method() {
    let code = r#"
const decipher = crypto.createDecipher('aes-256-cbc', 'password');
console.log(typeof decipher.update === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected decipher to have update method: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_decipher_has_final_method() {
    let code = r#"
const decipher = crypto.createDecipher('aes-256-cbc', 'password');
console.log(typeof decipher.final === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected decipher to have final method: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_decipher_has_set_auto_padding() {
    let code = r#"
const decipher = crypto.createDecipher('aes-256-cbc', 'password');
console.log(typeof decipher.setAutoPadding === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected decipher to have setAutoPadding method: {}",
        output
    );
}

// ==================== Encryption/Decryption Round-trip Tests ====================

#[test]
#[serial]
fn test_cipher_decipher_round_trip() {
    let code = r#"
const password = 'mysecretkey';
const plaintext = 'Hello, World!';

// Encrypt - need to combine update and final results
const cipher = crypto.createCipher('aes-256-cbc', password);
const encrypted_part1 = cipher.update(plaintext, 'utf8', 'buffer');
const encrypted_part2 = cipher.final('buffer');
const encrypted = Buffer.concat([encrypted_part1, encrypted_part2]);

// Decrypt
const decipher = crypto.createDecipher('aes-256-cbc', password);
const decrypted = decipher.update(encrypted, 'buffer', 'utf8');
const decrypted_final = decipher.final('utf8');
const full_decrypted = decrypted + decrypted_final;

console.log(full_decrypted === plaintext ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected encryption/decryption round-trip to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_cipher_update_returns_buffer() {
    let code = r#"
const cipher = crypto.createCipher('aes-256-cbc', 'password');
const result = cipher.update('test data', 'utf8', 'buffer');
console.log(result instanceof Uint8Array ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected update to return Uint8Array: {}",
        output
    );
}

#[test]
#[serial]
fn test_decipher_update_returns_string() {
    // For small inputs (< block size), decipher.update() returns empty string
    // because all data is accumulated and returned in final()
    // This is correct behavior for proper padding handling
    let code = r#"
const cipher = crypto.createCipher('aes-256-cbc', 'password');
const encrypted = cipher.update('test data', 'utf8', 'buffer');
cipher.final('buffer');

const decipher = crypto.createDecipher('aes-256-cbc', 'password');
const result = decipher.update(encrypted, 'buffer', 'utf8');
// For small inputs, update returns empty string (data accumulated for final)
console.log(result === '' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected decipher update to return empty string: {}",
        output
    );
}

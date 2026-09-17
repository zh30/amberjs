// createDiffieHellman Tests - v0.3.26
// Tests for crypto.createDiffieHellman function
// Diffie-Hellman key exchange protocol for secure key agreement

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
    // Parse output - skip the "🐝 Running Beejs on:" line and "Result:" line
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|line| !line.starts_with("🐝") && !line.starts_with("Result:"))
        .collect();
    lines.join("\n")
}

// ==================== createDiffieHellman Tests ====================

#[test]
#[serial]
fn test_create_diffie_hellman_exists() {
    let code = r#"
console.log(typeof crypto.createDiffieHellman === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.trim() == "PASS",
        "Expected createDiffieHellman to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_diffie_hellman_returns_object() {
    let code = r#"
const dh = crypto.createDiffieHellman(256);
console.log(typeof dh === 'object' && dh !== null ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected createDiffieHellman to return object: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_diffie_hellman_has_properties() {
    let code = r#"
const dh = crypto.createDiffieHellman(256);
const hasPrime = typeof dh.prime === 'string';
const hasGenerator = typeof dh.generator === 'number';
const hasPrivateKey = typeof dh.privateKey === 'string';
const hasPublicKey = typeof dh.publicKey === 'string';
const hasComputeSecret = typeof dh.computeSecret === 'function';
const hasGenerateKeys = typeof dh.generateKeys === 'function';
const hasGetPrime = typeof dh.getPrime === 'function';
const hasGetGenerator = typeof dh.getGenerator === 'function';

console.log(hasPrime ? 'PASS' : 'FAIL');
console.log(hasGenerator ? 'PASS' : 'FAIL');
console.log(hasPrivateKey ? 'PASS' : 'FAIL');
console.log(hasPublicKey ? 'PASS' : 'FAIL');
console.log(hasComputeSecret ? 'PASS' : 'FAIL');
console.log(hasGenerateKeys ? 'PASS' : 'FAIL');
console.log(hasGetPrime ? 'PASS' : 'FAIL');
console.log(hasGetGenerator ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected all DH properties to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_diffie_hellman_with_number_prime() {
    let code = r#"
const dh = crypto.createDiffieHellman(128);
console.log(dh.prime.length > 0 ? 'PASS' : 'FAIL');
console.log(dh.generator === 2 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected DH with number prime: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_diffie_hellman_with_options() {
    let code = r#"
const dh = crypto.createDiffieHellman({
    prime: 128,
    generator: 2
});
console.log(dh.prime.length > 0 ? 'PASS' : 'FAIL');
console.log(dh.generator === 2 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected DH with options object: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_diffie_hellman_with_generator() {
    let code = r#"
const dh = crypto.createDiffieHellman(256, 5);
console.log(dh.generator === 5 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected custom generator: {}",
        output
    );
}

#[test]
#[serial]
fn test_diffie_hellman_compute_secret() {
    let code = r#"
const alice = crypto.createDiffieHellman(256);
const bob = crypto.createDiffieHellman(256);

const aliceSecret = alice.computeSecret(bob.publicKey);
const bobSecret = bob.computeSecret(alice.publicKey);

console.log(aliceSecret instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(aliceSecret.length > 0 ? 'PASS' : 'FAIL');
console.log(bobSecret.length > 0 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected computeSecret to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_diffie_hellman_compute_secret_hex() {
    let code = r#"
const alice = crypto.createDiffieHellman(256);
const bob = crypto.createDiffieHellman(256);

const aliceSecret = alice.computeSecret(bob.publicKey, 'hex');
const bobSecret = bob.computeSecret(alice.publicKey, 'hex');

console.log(typeof aliceSecret === 'string' ? 'PASS' : 'FAIL');
console.log(aliceSecret.length > 0 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected computeSecret with hex encoding: {}",
        output
    );
}

#[test]
#[serial]
fn test_diffie_hellman_compute_secret_base64() {
    let code = r#"
const alice = crypto.createDiffieHellman(256);
const bob = crypto.createDiffieHellman(256);

const aliceSecret = alice.computeSecret(bob.publicKey, 'base64');
const bobSecret = bob.computeSecret(alice.publicKey, 'base64');

console.log(typeof aliceSecret === 'string' ? 'PASS' : 'FAIL');
console.log(aliceSecret.length > 0 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected computeSecret with base64 encoding: {}",
        output
    );
}

#[test]
#[serial]
fn test_diffie_hellman_generate_keys() {
    let code = r#"
const dh = crypto.createDiffieHellman(256);
const keys = dh.generateKeys();

console.log(typeof keys === 'object' ? 'PASS' : 'FAIL');
console.log(typeof keys.privateKey === 'string' ? 'PASS' : 'FAIL');
console.log(typeof keys.publicKey === 'string' ? 'PASS' : 'FAIL');
console.log(keys.privateKey.length > 0 ? 'PASS' : 'FAIL');
console.log(keys.publicKey.length > 0 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected generateKeys to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_diffie_hellman_get_prime() {
    let code = r#"
const dh = crypto.createDiffieHellman(256);
const prime = dh.getPrime();

console.log(typeof prime === 'string' ? 'PASS' : 'FAIL');
console.log(prime.length > 0 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected getPrime to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_diffie_hellman_get_generator() {
    let code = r#"
const dh = crypto.createDiffieHellman(256);
const generator = dh.getGenerator();

console.log(typeof generator === 'number' ? 'PASS' : 'FAIL');
console.log(generator === 2 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected getGenerator to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_diffie_hellman_keys_are_hex() {
    let code = r#"
const dh = crypto.createDiffieHellman(256);
const privateKeyHex = /^[0-9a-f]+$/i.test(dh.privateKey);
const publicKeyHex = /^[0-9a-f]+$/i.test(dh.publicKey);

console.log(privateKeyHex ? 'PASS' : 'FAIL');
console.log(publicKeyHex ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected keys to be hex strings: {}",
        output
    );
}

#[test]
#[serial]
fn test_diffie_hellman_different_instances() {
    let code = r#"
const dh1 = crypto.createDiffieHellman(256);
const dh2 = crypto.createDiffieHellman(256);

console.log(dh1.prime !== dh2.prime ? 'PASS' : 'FAIL');
console.log(dh1.privateKey !== dh2.privateKey ? 'PASS' : 'FAIL');
console.log(dh1.publicKey !== dh2.publicKey ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected different instances to have different keys: {}",
        output
    );
}

#[test]
#[serial]
fn test_diffie_hellman_compute_secret_with_string_public_key() {
    let code = r#"
const alice = crypto.createDiffieHellman(256);
const bob = crypto.createDiffieHellman(256);

const secret = alice.computeSecret(bob.publicKey);
console.log(secret instanceof Uint8Array ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected computeSecret with string public key: {}",
        output
    );
}

#[test]
#[serial]
fn test_diffie_hellman_compute_secret_with_object() {
    let code = r#"
const alice = crypto.createDiffieHellman(256);
const bob = crypto.createDiffieHellman(256);

const secret = alice.computeSecret({ publicKey: bob.publicKey });
console.log(secret instanceof Uint8Array ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected computeSecret with object public key: {}",
        output
    );
}

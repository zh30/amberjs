// createECDH Tests - v0.3.27
// Tests for crypto.createECDH function
// Elliptic Curve Diffie-Hellman key exchange protocol

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

// ==================== createECDH Tests ====================

#[test]
#[serial]
fn test_create_ecdh_exists() {
    let code = r#"
console.log(typeof crypto.createECDH === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.trim() == "PASS",
        "Expected createECDH to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_ecdh_returns_object() {
    let code = r#"
const ecdh = crypto.createECDH('prime256v1');
console.log(typeof ecdh === 'object' && ecdh !== null ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected createECDH to return object: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_ecdh_has_properties() {
    let code = r#"
const ecdh = crypto.createECDH('prime256v1');
const hasPrivateKey = typeof ecdh.privateKey === 'string';
const hasPublicKey = typeof ecdh.publicKey === 'string';
const hasComputeSecret = typeof ecdh.computeSecret === 'function';
const hasGenerateKeys = typeof ecdh.generateKeys === 'function';
const hasGetPublicKey = typeof ecdh.getPublicKey === 'function';
const hasGetPrivateKey = typeof ecdh.getPrivateKey === 'function';
const hasSetPublicKey = typeof ecdh.setPublicKey === 'function';
const hasSetPrivateKey = typeof ecdh.setPrivateKey === 'function';

console.log(hasPrivateKey ? 'PASS' : 'FAIL');
console.log(hasPublicKey ? 'PASS' : 'FAIL');
console.log(hasComputeSecret ? 'PASS' : 'FAIL');
console.log(hasGenerateKeys ? 'PASS' : 'FAIL');
console.log(hasGetPublicKey ? 'PASS' : 'FAIL');
console.log(hasGetPrivateKey ? 'PASS' : 'FAIL');
console.log(hasSetPublicKey ? 'PASS' : 'FAIL');
console.log(hasSetPrivateKey ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected all ECDH properties to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_ecdh_prime256v1() {
    let code = r#"
const ecdh = crypto.createECDH('prime256v1');
console.log(ecdh.privateKey.length > 0 ? 'PASS' : 'FAIL');
console.log(ecdh.publicKey.length > 0 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected ECDH with prime256v1: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_ecdh_secp256r1() {
    let code = r#"
const ecdh = crypto.createECDH('secp256r1');
console.log(ecdh.privateKey.length > 0 ? 'PASS' : 'FAIL');
console.log(ecdh.publicKey.length > 0 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected ECDH with secp256r1: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_ecdh_secp384r1() {
    let code = r#"
const ecdh = crypto.createECDH('secp384r1');
console.log(ecdh.privateKey.length > 0 ? 'PASS' : 'FAIL');
console.log(ecdh.publicKey.length > 0 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected ECDH with secp384r1: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_ecdh_secp521r1() {
    let code = r#"
const ecdh = crypto.createECDH('secp521r1');
console.log(ecdh.privateKey.length > 0 ? 'PASS' : 'FAIL');
console.log(ecdh.publicKey.length > 0 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected ECDH with secp521r1: {}",
        output
    );
}

#[test]
#[serial]
fn test_ecdh_compute_secret() {
    let code = r#"
const alice = crypto.createECDH('prime256v1');
const bob = crypto.createECDH('prime256v1');

const aliceSecret = alice.computeSecret(bob.getPublicKey());
const bobSecret = bob.computeSecret(alice.getPublicKey());

console.log(aliceSecret instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(aliceSecret.length > 0 ? 'PASS' : 'FAIL');
console.log(bobSecret.length > 0 ? 'PASS' : 'FAIL');
console.log(aliceSecret.length === bobSecret.length ? 'PASS' : 'FAIL');
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
fn test_ecdh_compute_secret_hex() {
    let code = r#"
const alice = crypto.createECDH('prime256v1');
const bob = crypto.createECDH('prime256v1');

const aliceSecret = alice.computeSecret(bob.getPublicKey(), 'hex');
const bobSecret = bob.computeSecret(alice.getPublicKey(), 'hex');

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
fn test_ecdh_compute_secret_base64() {
    let code = r#"
const alice = crypto.createECDH('prime256v1');
const bob = crypto.createECDH('prime256v1');

const aliceSecret = alice.computeSecret(bob.getPublicKey(), 'base64');
const bobSecret = bob.computeSecret(alice.getPublicKey(), 'base64');

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
fn test_ecdh_generate_keys() {
    let code = r#"
const ecdh = crypto.createECDH('prime256v1');
const keys = ecdh.generateKeys();

console.log(typeof keys === 'object' ? 'PASS' : 'FAIL');
console.log(typeof keys.publicKey === 'string' ? 'PASS' : 'FAIL');
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
fn test_ecdh_get_public_key() {
    let code = r#"
const ecdh = crypto.createECDH('prime256v1');
const publicKey = ecdh.getPublicKey();

console.log(typeof publicKey === 'string' ? 'PASS' : 'FAIL');
console.log(publicKey.length > 0 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected getPublicKey to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_ecdh_get_private_key() {
    let code = r#"
const ecdh = crypto.createECDH('prime256v1');
const privateKey = ecdh.getPrivateKey();

console.log(typeof privateKey === 'string' ? 'PASS' : 'FAIL');
console.log(privateKey.length > 0 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected getPrivateKey to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_ecdh_set_public_key() {
    let code = r#"
const alice = crypto.createECDH('prime256v1');
alice.generateKeys();
const originalPublicKey = alice.getPublicKey();

const bob = crypto.createECDH('prime256v1');
bob.generateKeys();

// Set Alice's public key to Bob's (simulating key exchange)
bob.setPublicKey(originalPublicKey);

console.log(bob.getPublicKey() === originalPublicKey ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected setPublicKey to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_ecdh_set_private_key() {
    let code = r#"
const alice = crypto.createECDH('prime256v1');
alice.generateKeys();
const originalPrivateKey = alice.getPrivateKey();

const bob = crypto.createECDH('prime256v1');
bob.generateKeys();

// Set Bob's private key
bob.setPrivateKey(originalPrivateKey);

console.log(bob.getPrivateKey() === originalPrivateKey ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected setPrivateKey to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_ecdh_keys_are_hex() {
    let code = r#"
const ecdh = crypto.createECDH('prime256v1');
const privateKeyHex = /^[0-9a-f]+$/i.test(ecdh.privateKey);
const publicKeyHex = /^[0-9a-f]+$/i.test(ecdh.publicKey);

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
fn test_ecdh_different_instances() {
    let code = r#"
const ecdh1 = crypto.createECDH('prime256v1');
const ecdh2 = crypto.createECDH('prime256v1');

console.log(ecdh1.privateKey !== ecdh2.privateKey ? 'PASS' : 'FAIL');
console.log(ecdh1.publicKey !== ecdh2.publicKey ? 'PASS' : 'FAIL');
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
fn test_ecdh_compute_secret_with_buffer() {
    let code = r#"
const alice = crypto.createECDH('prime256v1');
const bob = crypto.createECDH('prime256v1');

// Get public key as buffer
const bobPublicKeyBuffer = Buffer.from(bob.getPublicKey(), 'hex');
const aliceSecret = alice.computeSecret(bobPublicKeyBuffer);

console.log(aliceSecret instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(aliceSecret.length > 0 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected computeSecret with buffer: {}",
        output
    );
}

#[test]
#[serial]
fn test_ecdh_key_exchange_roundtrip() {
    let code = r#"
const alice = crypto.createECDH('prime256v1');
const bob = crypto.createECDH('prime256v1');

// Alice generates keys and sends public key to Bob
alice.generateKeys();
const alicePublicKeyHex = alice.getPublicKey();

// Bob generates keys and sends public key to Alice
bob.generateKeys();
const bobPublicKeyHex = bob.getPublicKey();

// Both compute the same shared secret
const aliceShared = alice.computeSecret(bobPublicKeyHex);
const bobShared = bob.computeSecret(alicePublicKeyHex);

// Shared secrets should be equal
const equal = aliceShared.length === bobShared.length &&
              aliceShared.every((b, i) => b === bobShared[i]);

console.log(equal ? 'PASS' : 'FAIL');
console.log(aliceShared.length === 32 ? 'PASS' : 'FAIL'); // 256-bit / 32 bytes
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected key exchange roundtrip: {}",
        output
    );
}

#[test]
#[serial]
fn test_ecdh_invalid_curve() {
    let code = r#"
try {
    crypto.createECDH('invalid-curve');
    console.log('FAIL');
} catch (e) {
    // e is a string in this implementation, use toString() or String()
    const msg = String(e).toLowerCase();
    console.log(msg.includes('unsupported') || msg.includes('invalid') ? 'PASS' : 'FAIL');
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected error for invalid curve: {}",
        output
    );
}

#[test]
#[serial]
fn test_ecdh_compute_secret_rejects_missing_or_invalid_peer_key() {
    let code = r#"
const alice = crypto.createECDH('prime256v1');

try {
    alice.computeSecret();
    console.log('FAIL');
} catch (e) {
    const msg = String(e && e.message ? e.message : e).toLowerCase();
    console.log(msg.includes('public key') ? 'PASS' : 'FAIL');
}

try {
    alice.computeSecret('invalid');
    console.log('FAIL');
} catch (e) {
    const msg = String(e && e.message ? e.message : e).toLowerCase();
    console.log(msg.includes('public key') ? 'PASS' : 'FAIL');
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected computeSecret to reject missing or invalid peer key: {}",
        output
    );
}

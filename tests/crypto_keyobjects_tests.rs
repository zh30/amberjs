// KeyObjects Tests - v0.3.28
// Tests for crypto.createPrivateKey, createPublicKey, createSecretKey
// KeyObjects API for cryptographic key management

use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::symm::Cipher;
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

fn encrypted_rsa_pkcs8_pair(passphrase: &[u8]) -> (String, String) {
    let rsa = Rsa::generate(2048).unwrap();
    let key = PKey::from_rsa(rsa).unwrap();
    let private_pem = key
        .private_key_to_pem_pkcs8_passphrase(Cipher::aes_256_cbc(), passphrase)
        .unwrap();
    let public_pem = key.public_key_to_pem().unwrap();

    (
        String::from_utf8(private_pem).unwrap(),
        String::from_utf8(public_pem).unwrap(),
    )
}

// ==================== createPrivateKey Tests ====================

#[test]
#[serial]
fn test_create_private_key_exists() {
    let code = r#"
console.log(typeof crypto.createPrivateKey === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.trim() == "PASS",
        "Expected createPrivateKey to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_private_key_returns_object() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const privateKey = crypto.createPrivateKey(generated.privateKey);
console.log(typeof privateKey === 'object' && privateKey !== null ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected createPrivateKey to return object: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_private_key_type_property() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const privateKey = crypto.createPrivateKey(generated.privateKey);
console.log(privateKey.type === 'private' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected type to be 'private': {}",
        output
    );
}

#[test]
#[serial]
fn test_create_private_key_asymmetric_key_type_rsa() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const privateKey = crypto.createPrivateKey(generated.privateKey);
console.log(privateKey.asymmetricKeyType === 'rsa' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected asymmetricKeyType to be 'rsa': {}",
        output
    );
}

#[test]
#[serial]
fn test_create_private_key_asymmetric_key_type_ec() {
    let code = r#"
const generated = crypto.generateKeyPairSync('ec', { namedCurve: 'prime256v1' });
const privateKey = crypto.createPrivateKey(generated.privateKey);
console.log(privateKey.asymmetricKeyType === 'ec' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected asymmetricKeyType to be 'ec': {}",
        output
    );
}

#[test]
#[serial]
fn test_create_private_key_has_export_method() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const privateKey = crypto.createPrivateKey(generated.privateKey);
console.log(typeof privateKey.export === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected export method to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_private_key_export_pem() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const privateKey = crypto.createPrivateKey(generated.privateKey);
const exported = privateKey.export('pem');
console.log(typeof exported === 'string' && exported.includes('BEGIN RSA PRIVATE KEY') ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected export('pem') to return PEM string: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_private_key_export_encrypted_pkcs8_pem_with_passphrase() {
    let code = r#"
const passphrase = 'beejs-keyobject-export-passphrase';
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const privateKey = crypto.createPrivateKey(generated.privateKey);
const exported = privateKey.export({
    type: 'pkcs8',
    format: 'pem',
    cipher: 'aes-256-cbc',
    passphrase
});

const imported = crypto.createPrivateKey({
    key: exported,
    type: 'pkcs8',
    format: 'pem',
    passphrase
});
const sign = crypto.createSign('RSA-SHA256');
sign.update('encrypted keyobject export');
const signature = sign.sign(imported, 'base64');

const verify = crypto.createVerify('RSA-SHA256');
verify.update('encrypted keyobject export');
console.log(typeof exported === 'string' ? 'PASS' : 'FAIL');
console.log(exported.includes('BEGIN ENCRYPTED PRIVATE KEY') ? 'PASS' : 'FAIL');
console.log(!exported.includes('BEGIN RSA PRIVATE KEY') ? 'PASS' : 'FAIL');
console.log(verify.verify(generated.publicKey, signature, 'base64') === true ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 4,
        "Expected private KeyObject encrypted PKCS8 PEM export to round-trip: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_private_key_export_pkcs8_der_returns_buffer() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const privateKey = crypto.createPrivateKey(generated.privateKey);
const exported = privateKey.export({ type: 'pkcs8', format: 'der' });
console.log(Buffer.isBuffer(exported) ? 'PASS' : 'FAIL');
console.log(exported.length > 256 ? 'PASS' : 'FAIL');
console.log(exported.toString('utf8').includes('BEGIN') ? 'FAIL' : 'PASS');
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 3,
        "Expected PKCS8 DER export to return binary Buffer data: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_private_key_import_pkcs8_der_round_trip() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
try {
const originalPrivateKey = crypto.createPrivateKey(generated.privateKey);
const der = originalPrivateKey.export({ type: 'pkcs8', format: 'der' });
const importedPrivateKey = crypto.createPrivateKey({ key: der, type: 'pkcs8', format: 'der' });

const sign = crypto.createSign('RSA-SHA256');
sign.update('pkcs8 der import');
const signature = sign.sign(importedPrivateKey, 'base64');

const verify = crypto.createVerify('RSA-SHA256');
verify.update('pkcs8 der import');
console.log(importedPrivateKey.type === 'private' ? 'PASS' : 'FAIL');
console.log(importedPrivateKey.asymmetricKeyType === 'rsa' ? 'PASS' : 'FAIL');
console.log(verify.verify(generated.publicKey, signature, 'base64') === true ? 'PASS' : 'FAIL');
} catch (error) {
    console.log('ERROR:' + String(error && error.message ? error.message : error));
}
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 3,
        "Expected PKCS8 DER import to produce usable private KeyObject: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_private_key_import_encrypted_pkcs8_pem_with_passphrase() {
    let passphrase = "beejs-test-passphrase";
    let (encrypted_private_pem, public_pem) = encrypted_rsa_pkcs8_pair(passphrase.as_bytes());
    let encrypted_private_pem = serde_json::to_string(&encrypted_private_pem).unwrap();
    let public_pem = serde_json::to_string(&public_pem).unwrap();
    let passphrase = serde_json::to_string(passphrase).unwrap();
    let code = format!(
        r#"
const encryptedPrivatePem = {encrypted_private_pem};
const publicPem = {public_pem};
const importedPrivateKey = crypto.createPrivateKey({{
    key: encryptedPrivatePem,
    type: 'pkcs8',
    format: 'pem',
    passphrase: {passphrase}
}});

const sign = crypto.createSign('RSA-SHA256');
sign.update('encrypted pkcs8 pem import');
const signature = sign.sign(importedPrivateKey, 'base64');

const verify = crypto.createVerify('RSA-SHA256');
verify.update('encrypted pkcs8 pem import');
console.log(importedPrivateKey.type === 'private' ? 'PASS' : 'FAIL');
console.log(importedPrivateKey.asymmetricKeyType === 'rsa' ? 'PASS' : 'FAIL');
console.log(verify.verify(publicPem, signature, 'base64') === true ? 'PASS' : 'FAIL');
"#
    );
    let output = run_js_test(&code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 3,
        "Expected encrypted PKCS8 PEM import with passphrase to produce usable private KeyObject: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_private_key_with_object_format() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const privateKey = crypto.createPrivateKey({ key: generated.privateKey });
console.log(typeof privateKey === 'object' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected createPrivateKey to accept object format: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_private_key_invalid_format() {
    let code = r#"
try {
    crypto.createPrivateKey('invalid-key-data');
    console.log('FAIL');
} catch (e) {
    console.log(String(e && e.message ? e.message : e).toLowerCase().includes('invalid') ? 'PASS' : 'FAIL');
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected error for invalid key format: {}",
        output
    );
}

// ==================== createPublicKey Tests ====================

#[test]
#[serial]
fn test_create_public_key_exists() {
    let code = r#"
console.log(typeof crypto.createPublicKey === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.trim() == "PASS",
        "Expected createPublicKey to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_public_key_returns_object() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const publicKey = crypto.createPublicKey(generated.publicKey);
console.log(typeof publicKey === 'object' && publicKey !== null ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected createPublicKey to return object: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_public_key_type_property() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const publicKey = crypto.createPublicKey(generated.publicKey);
console.log(publicKey.type === 'public' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected type to be 'public': {}",
        output
    );
}

#[test]
#[serial]
fn test_create_public_key_asymmetric_key_type() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const publicKey = crypto.createPublicKey(generated.publicKey);
console.log(publicKey.asymmetricKeyType === 'rsa' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected asymmetricKeyType to be 'rsa': {}",
        output
    );
}

#[test]
#[serial]
fn test_create_public_key_has_export_method() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const publicKey = crypto.createPublicKey(generated.publicKey);
console.log(typeof publicKey.export === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected export method to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_public_key_export_pem() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const publicKey = crypto.createPublicKey(generated.publicKey);
const exported = publicKey.export('pem');
console.log(typeof exported === 'string' && exported.includes('BEGIN PUBLIC KEY') ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected export('pem') to return PEM string: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_public_key_from_private_key_pem_derives_public_key() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const publicKey = crypto.createPublicKey(generated.privateKey);
const exported = publicKey.export({ type: 'spki', format: 'pem' });

const sign = crypto.createSign('RSA-SHA256');
sign.update('derive public from private pem');
const signature = sign.sign(generated.privateKey, 'base64');

const verify = crypto.createVerify('RSA-SHA256');
verify.update('derive public from private pem');
console.log(publicKey.type === 'public' ? 'PASS' : 'FAIL');
console.log(publicKey.asymmetricKeyType === 'rsa' ? 'PASS' : 'FAIL');
console.log(exported.includes('BEGIN PUBLIC KEY') && !exported.includes('PRIVATE KEY') ? 'PASS' : 'FAIL');
console.log(verify.verify(publicKey, signature, 'base64') === true ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 4,
        "Expected createPublicKey(private PEM) to derive a usable RSA public key: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_public_key_from_private_key_object_derives_ec_public_key() {
    let code = r#"
const generated = crypto.generateKeyPairSync('ec', { namedCurve: 'prime256v1' });
const privateKey = crypto.createPrivateKey(generated.privateKey);
const publicKey = crypto.createPublicKey(privateKey);
const exported = publicKey.export({ type: 'spki', format: 'pem' });

const sign = crypto.createSign('SHA256');
sign.update('derive ec public from private keyobject');
const signature = sign.sign(privateKey, 'base64');

const verify = crypto.createVerify('SHA256');
verify.update('derive ec public from private keyobject');
console.log(publicKey.type === 'public' ? 'PASS' : 'FAIL');
console.log(publicKey.asymmetricKeyType === 'ec' ? 'PASS' : 'FAIL');
console.log(exported.includes('BEGIN PUBLIC KEY') && !exported.includes('PRIVATE KEY') ? 'PASS' : 'FAIL');
console.log(verify.verify(publicKey, signature, 'base64') === true ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 4,
        "Expected createPublicKey(private KeyObject) to derive a usable EC public key: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_public_key_from_private_key_options_derives_public_key() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const publicKey = crypto.createPublicKey({ key: generated.privateKey, format: 'pem' });
const exported = publicKey.export({ type: 'spki', format: 'pem' });

const sign = crypto.createSign('RSA-SHA256');
sign.update('derive public from private options');
const signature = sign.sign(generated.privateKey, 'base64');

const verify = crypto.createVerify('RSA-SHA256');
verify.update('derive public from private options');
console.log(publicKey.type === 'public' ? 'PASS' : 'FAIL');
console.log(publicKey.asymmetricKeyType === 'rsa' ? 'PASS' : 'FAIL');
console.log(exported.includes('BEGIN PUBLIC KEY') && !exported.includes('PRIVATE KEY') ? 'PASS' : 'FAIL');
console.log(verify.verify(publicKey, signature, 'base64') === true ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 4,
        "Expected createPublicKey({{ key: private PEM }}) to derive a usable public key: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_public_key_export_pkcs1_pem_round_trip() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const publicKey = crypto.createPublicKey(generated.publicKey);
const pkcs1 = publicKey.export({ type: 'pkcs1', format: 'pem' });
const spki = publicKey.export({ type: 'spki', format: 'pem' });
const importedPublicKey = crypto.createPublicKey({ key: pkcs1, type: 'pkcs1', format: 'pem' });

const sign = crypto.createSign('RSA-SHA256');
sign.update('public pkcs1 pem export');
const signature = sign.sign(generated.privateKey, 'base64');

const verify = crypto.createVerify('RSA-SHA256');
verify.update('public pkcs1 pem export');
console.log(typeof pkcs1 === 'string' && pkcs1.includes('BEGIN RSA PUBLIC KEY') ? 'PASS' : 'FAIL');
console.log(spki.includes('BEGIN PUBLIC KEY') && !spki.includes('BEGIN RSA PUBLIC KEY') ? 'PASS' : 'FAIL');
console.log(importedPublicKey.asymmetricKeyType === 'rsa' ? 'PASS' : 'FAIL');
console.log(verify.verify(importedPublicKey, signature, 'base64') === true ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 4,
        "Expected RSA public PKCS#1 PEM export/import round-trip: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_public_key_export_spki_der_returns_buffer() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const publicKey = crypto.createPublicKey(generated.publicKey);
const exported = publicKey.export({ type: 'spki', format: 'der' });
console.log(Buffer.isBuffer(exported) ? 'PASS' : 'FAIL');
console.log(exported.length > 128 ? 'PASS' : 'FAIL');
console.log(exported.toString('utf8').includes('BEGIN') ? 'FAIL' : 'PASS');
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 3,
        "Expected SPKI DER export to return binary Buffer data: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_public_key_export_import_pkcs1_der_round_trip() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const publicKey = crypto.createPublicKey(generated.publicKey);
const exported = publicKey.export({ type: 'pkcs1', format: 'der' });
const importedPublicKey = crypto.createPublicKey({ key: exported, type: 'pkcs1', format: 'der' });

const sign = crypto.createSign('RSA-SHA256');
sign.update('public pkcs1 der export');
const signature = sign.sign(generated.privateKey, 'base64');

const verify = crypto.createVerify('RSA-SHA256');
verify.update('public pkcs1 der export');
console.log(Buffer.isBuffer(exported) ? 'PASS' : 'FAIL');
console.log(exported.length > 128 ? 'PASS' : 'FAIL');
console.log(exported.toString('utf8').includes('BEGIN') ? 'FAIL' : 'PASS');
console.log(importedPublicKey.asymmetricKeyType === 'rsa' ? 'PASS' : 'FAIL');
console.log(verify.verify(importedPublicKey, signature, 'base64') === true ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 5,
        "Expected RSA public PKCS#1 DER export/import round-trip: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_public_key_export_ec_pkcs1_reports_incompatible_key_options() {
    let code = r#"
const generated = crypto.generateKeyPairSync('ec', { namedCurve: 'prime256v1' });
const publicKey = crypto.createPublicKey(generated.publicKey);
for (const format of ['pem', 'der']) {
    try {
        publicKey.export({ type: 'pkcs1', format });
        console.log('FAIL');
    } catch (e) {
        console.log(e && e.code === 'ERR_CRYPTO_INCOMPATIBLE_KEY_OPTIONS' ? 'PASS' : 'FAIL');
        console.log(e && e.name === 'Error' ? 'PASS' : 'FAIL');
        console.log(e && e.message === 'The selected key encoding pkcs1 can only be used for RSA keys.' ? 'PASS' : 'FAIL');
    }
}
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 6,
        "Expected EC public pkcs1 export to report incompatible key options: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_public_key_import_spki_der_round_trip() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
try {
const originalPublicKey = crypto.createPublicKey(generated.publicKey);
const der = originalPublicKey.export({ type: 'spki', format: 'der' });
const importedPublicKey = crypto.createPublicKey({ key: der, type: 'spki', format: 'der' });

const sign = crypto.createSign('RSA-SHA256');
sign.update('spki der import');
const signature = sign.sign(generated.privateKey, 'base64');

const verify = crypto.createVerify('RSA-SHA256');
verify.update('spki der import');
console.log(importedPublicKey.type === 'public' ? 'PASS' : 'FAIL');
console.log(importedPublicKey.asymmetricKeyType === 'rsa' ? 'PASS' : 'FAIL');
console.log(verify.verify(importedPublicKey, signature, 'base64') === true ? 'PASS' : 'FAIL');
} catch (error) {
    console.log('ERROR:' + String(error && error.message ? error.message : error));
}
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 3,
        "Expected SPKI DER import to produce usable public KeyObject: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_public_key_detects_ec_spki_key_type() {
    let code = r#"
const generated = crypto.generateKeyPairSync('ec', { namedCurve: 'prime256v1' });
const publicKey = crypto.createPublicKey(generated.publicKey);
console.log(publicKey.asymmetricKeyType === 'ec' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected EC SPKI public key to be detected as ec: {}",
        output
    );
}

// ==================== createSecretKey Tests ====================

#[test]
#[serial]
fn test_create_secret_key_exists() {
    let code = r#"
console.log(typeof crypto.createSecretKey === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.trim() == "PASS",
        "Expected createSecretKey to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_secret_key_returns_object() {
    let code = r#"
const secretKey = crypto.createSecretKey(Buffer.from('my-secret-key'));
console.log(typeof secretKey === 'object' && secretKey !== null ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected createSecretKey to return object: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_secret_key_type_property() {
    let code = r#"
const secretKey = crypto.createSecretKey(Buffer.from('my-secret-key'));
console.log(secretKey.type === 'secret' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected type to be 'secret': {}",
        output
    );
}

#[test]
#[serial]
fn test_create_secret_key_length_property() {
    let code = r#"
const secretKey = crypto.createSecretKey(Buffer.from('my-secret-key'));
console.log(secretKey.length === 13 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected length to be 13: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_secret_key_asymmetric_key_type() {
    let code = r#"
const secretKey = crypto.createSecretKey(Buffer.from('my-secret-key'));
console.log(secretKey.asymmetricKeyType === 'secret' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected asymmetricKeyType to be 'secret': {}",
        output
    );
}

#[test]
#[serial]
fn test_create_secret_key_has_export_method() {
    let code = r#"
const secretKey = crypto.createSecretKey(Buffer.from('my-secret-key'));
console.log(typeof secretKey.export === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected export method to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_secret_key_export_raw() {
    let code = r#"
const secretKey = crypto.createSecretKey(Buffer.from('my-secret-key'));
const exported = secretKey.export('raw');
console.log(exported instanceof Uint8Array ? 'PASS' : 'FAIL');
console.log(exported.length === 13 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected export('raw') to return Uint8Array: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_secret_key_export_buffer() {
    let code = r#"
const secretKey = crypto.createSecretKey(Buffer.from('my-secret-key'));
const exported = secretKey.export('buffer');
console.log(exported instanceof Uint8Array ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected export('buffer') to return Uint8Array: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_secret_key_export_base64() {
    let code = r#"
const secretKey = crypto.createSecretKey(Buffer.from('my-secret-key'));
const exported = secretKey.export('base64');
console.log(typeof exported === 'string' ? 'PASS' : 'FAIL');
console.log(exported.length > 0 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected export('base64') to return string: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_secret_key_with_uint8array() {
    let code = r#"
const keyData = new Uint8Array([1, 2, 3, 4, 5]);
const secretKey = crypto.createSecretKey(keyData);
console.log(secretKey.length === 5 ? 'PASS' : 'FAIL');
console.log(secretKey.type === 'secret' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected createSecretKey to work with Uint8Array: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_secret_key_with_string() {
    let code = r#"
const secretKey = crypto.createSecretKey('my-string-key');
console.log(secretKey.length === 14 ? 'PASS' : 'FAIL');
console.log(secretKey.type === 'secret' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected createSecretKey to work with string: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_secret_key_with_arraybuffer() {
    let code = r#"
const buffer = new ArrayBuffer(8);
const view = new Uint8Array(buffer);
view.set([1,2,3,4,5,6,7,8]);
const secretKey = crypto.createSecretKey(buffer);
console.log(secretKey.length === 8 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected createSecretKey to work with ArrayBuffer: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_secret_key_invalid_format() {
    let code = r#"
try {
    crypto.createSecretKey(null);
    console.log('FAIL');
} catch (e) {
    console.log('PASS');
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected error for invalid key format: {}",
        output
    );
}

// ==================== KeyObjects Roundtrip Tests ====================

#[test]
#[serial]
fn test_keyobject_export_rsa_jwk_round_trip() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const privateKey = crypto.createPrivateKey(generated.privateKey);
const publicKey = crypto.createPublicKey(generated.publicKey);
const privateJwk = privateKey.export({ format: 'jwk' });
const publicJwk = publicKey.export({ format: 'jwk' });
const importedPrivateKey = crypto.createPrivateKey({ key: privateJwk, format: 'jwk' });
const importedPublicKey = crypto.createPublicKey({ key: publicJwk, format: 'jwk' });

const base64url = /^[A-Za-z0-9_-]+$/;
const fieldsOk =
  publicJwk.kty === 'RSA' &&
  base64url.test(publicJwk.n) &&
  base64url.test(publicJwk.e) &&
  privateJwk.kty === 'RSA' &&
  privateJwk.n === publicJwk.n &&
  privateJwk.e === publicJwk.e &&
  ['d', 'p', 'q', 'dp', 'dq', 'qi'].every((field) =>
    typeof privateJwk[field] === 'string' &&
    base64url.test(privateJwk[field]) &&
    !privateJwk[field].includes('=')
  );

const sign = crypto.createSign('RSA-SHA256');
sign.update('keyobject rsa jwk export');
const signature = sign.sign(importedPrivateKey, 'base64');

const verify = crypto.createVerify('RSA-SHA256');
verify.update('keyobject rsa jwk export');
console.log(fieldsOk ? 'PASS' : 'FAIL');
console.log(verify.verify(importedPublicKey, signature, 'base64') === true ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 2,
        "Expected RSA KeyObject JWK export to round-trip: {}",
        output
    );
}

#[test]
#[serial]
fn test_keyobject_export_ec_jwk_round_trip() {
    let code = r#"
const generated = crypto.generateKeyPairSync('ec', { namedCurve: 'prime256v1' });
const privateKey = crypto.createPrivateKey(generated.privateKey);
const publicKey = crypto.createPublicKey(generated.publicKey);
const privateJwk = privateKey.export({ format: 'jwk' });
const publicJwk = publicKey.export({ format: 'jwk' });
const importedPrivateKey = crypto.createPrivateKey({ key: privateJwk, format: 'jwk' });
const importedPublicKey = crypto.createPublicKey({ key: publicJwk, format: 'jwk' });

const base64url = /^[A-Za-z0-9_-]+$/;
const fieldsOk =
  publicJwk.kty === 'EC' &&
  publicJwk.crv === 'P-256' &&
  base64url.test(publicJwk.x) &&
  base64url.test(publicJwk.y) &&
  privateJwk.kty === 'EC' &&
  privateJwk.crv === 'P-256' &&
  privateJwk.x === publicJwk.x &&
  privateJwk.y === publicJwk.y &&
  base64url.test(privateJwk.d) &&
  !privateJwk.d.includes('=');

const sign = crypto.createSign('SHA256');
sign.update('keyobject ec jwk export');
const signature = sign.sign(importedPrivateKey, 'base64');

const verify = crypto.createVerify('SHA256');
verify.update('keyobject ec jwk export');
console.log(fieldsOk ? 'PASS' : 'FAIL');
console.log(verify.verify(importedPublicKey, signature, 'base64') === true ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 2,
        "Expected EC KeyObject JWK export to round-trip: {}",
        output
    );
}

#[test]
#[serial]
fn test_key_objects_export_import_roundtrip() {
    let code = r#"
const secretKey = crypto.createSecretKey(Buffer.from('roundtrip-test'));
const exported = secretKey.export('raw');
const imported = crypto.createSecretKey(exported);
console.log(imported.length === 14 ? 'PASS' : 'FAIL');
console.log(imported.type === 'secret' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected roundtrip to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_private_public_key_relationship() {
    let code = r#"
const generated = crypto.generateKeyPairSync('rsa', { modulusLength: 2048 });
const privateKey = crypto.createPrivateKey(generated.privateKey);
const publicKey = crypto.createPublicKey(generated.publicKey);
console.log(privateKey.type === 'private' ? 'PASS' : 'FAIL');
console.log(publicKey.type === 'public' ? 'PASS' : 'FAIL');
console.log(privateKey.asymmetricKeyType === publicKey.asymmetricKeyType ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected key types to match: {}",
        output
    );
}

#[test]
#[serial]
fn test_different_keys_are_independent() {
    let code = r#"
const key1 = crypto.createSecretKey(Buffer.from('key-one'));
const key2 = crypto.createSecretKey(Buffer.from('key-two'));
console.log(key1.length !== key2.length ? 'PASS' : 'FAIL');
console.log(key1.type === key2.type ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected different keys to be independent: {}",
        output
    );
}

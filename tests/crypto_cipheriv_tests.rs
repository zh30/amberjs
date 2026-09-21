// crypto.createCipheriv Tests - v0.3.15
// Tests for crypto.createCipheriv symmetric encryption function with explicit key and IV

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
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|line| !line.starts_with("🐝") && !line.starts_with("Result:"))
        .collect();
    lines.join("\n")
}

// ==================== createCipheriv Tests ====================

#[test]
#[serial]
fn test_create_cipheriv_function_exists() {
    let code = r#"
console.log(typeof crypto.createCipheriv === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.trim() == "PASS",
        "Expected createCipheriv to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_aes_256_cbc() {
    // AES-256-CBC requires 32-byte key and 16-byte IV
    let code = r#"
const key = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef'; // 64 hex chars = 32 bytes
const iv = 'abcdef0123456789abcdef0123456789'; // 32 hex chars = 16 bytes
const cipher = crypto.createCipheriv('aes-256-cbc', key, iv);
console.log(cipher && typeof cipher.update === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected AES-256-CBC cipheriv to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_aes_128_cbc() {
    // AES-128-CBC requires 16-byte key and 16-byte IV (32 hex chars for key, 32 for IV)
    let code = r#"
const key = '0123456789abcdef0123456789abcdef'; // 32 hex chars = 16 bytes
const iv = 'abcdef0123456789abcdef0123456789'; // 32 hex chars = 16 bytes
const cipher = crypto.createCipheriv('aes-128-cbc', key, iv);
console.log(cipher && typeof cipher.update === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected AES-128-CBC cipheriv to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_aes_192_cbc() {
    // AES-192-CBC requires 24-byte key and 16-byte IV (48 hex chars for key, 32 for IV)
    let code = r#"
const key = '0123456789abcdef0123456789abcdef0123456789abcdef'; // 48 hex chars = 24 bytes
const iv = 'abcdef0123456789abcdef0123456789'; // 32 hex chars = 16 bytes
const cipher = crypto.createCipheriv('aes-192-cbc', key, iv);
console.log(cipher && typeof cipher.update === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected AES-192-CBC cipheriv to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_decipheriv_node_aes_bare_aliases() {
    let code = r#"
const iv = '0102030405060708090a0b0c0d0e0f10';
const cases = [
    ['aes128', '00112233445566778899aabbccddeeff'],
    ['aes192', '00112233445566778899aabbccddeeff0011223344556677'],
    ['aes256', '00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff'],
];

for (const [algorithm, key] of cases) {
    try {
        const cipher = crypto.createCipheriv(algorithm, key, iv);
        const encrypted = cipher.update('Amber alias', 'utf8', 'hex') + cipher.final('hex');

        const decipher = crypto.createDecipheriv(algorithm, key, iv);
        const decrypted = decipher.update(encrypted, 'hex', 'utf8') + decipher.final('utf8');

        console.log(decrypted === 'Amber alias' ? 'PASS' : `FAIL:${algorithm}:${decrypted}`);
    } catch (error) {
        console.log(`FAIL:${algorithm}:${error.message}`);
    }
}
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 3,
        "Expected Node AES bare aliases to encrypt/decrypt as CBC aliases: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_node_aes_bare_alias_validates_key_length_at_creation() {
    let code = r#"
const iv = '0102030405060708090a0b0c0d0e0f10';

try {
    crypto.createCipheriv('aes256', '00112233445566778899aabbccddeeff', iv);
    console.log('FAIL:no-error');
} catch (error) {
    console.log(error.message.includes('invalid key length') ? 'PASS' : `FAIL:${error.message}`);
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected AES bare aliases to keep creation-time key length validation: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_has_update_method() {
    let code = r#"
const key = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
const iv = 'abcdef0123456789abcdef0123456789';
const cipher = crypto.createCipheriv('aes-256-cbc', key, iv);
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
fn test_create_cipheriv_has_finalize_method() {
    let code = r#"
const key = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
const iv = 'abcdef0123456789abcdef0123456789';
const cipher = crypto.createCipheriv('aes-256-cbc', key, iv);
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
fn test_create_cipheriv_invalid_algorithm() {
    let code = r#"
try {
    crypto.createCipheriv('invalid-alg', 'key', 'iv');
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
fn test_create_cipheriv_invalid_key_length() {
    let code = r#"
try {
    // AES-256 requires 32-byte key, using 16 bytes should fail
    crypto.createCipheriv('aes-256-cbc', '0123456789abcdef', 'abcdef0123456789abcdef0123456789');
    console.log('FAIL');
} catch (e) {
    console.log(e.message.includes('invalid') || e.message.includes('key') ? 'PASS' : 'FAIL');
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected error for invalid key length: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_invalid_iv_length() {
    let code = r#"
try {
    // CBC mode requires 16-byte IV, using 8 bytes should fail
    crypto.createCipheriv('aes-256-cbc', '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef', 'shortiv');
    console.log('FAIL');
} catch (e) {
    console.log(e.message.includes('iv') || e.message.includes('invalid') ? 'PASS' : 'FAIL');
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected error for invalid IV length: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_invalid_input_error_codes_match_node() {
    let code = r#"
function codeOf(fn) {
    try {
        fn();
        return 'NO_ERROR';
    } catch (e) {
        return e && e.code ? e.code : String(e && e.code);
    }
}

const validKey = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
const validIv = 'abcdef0123456789abcdef0123456789';

console.log('key:' + codeOf(() =>
    crypto.createCipheriv('aes-256-cbc', '0123456789abcdef', validIv)));
console.log('iv:' + codeOf(() =>
    crypto.createCipheriv('aes-256-cbc', validKey, 'shortiv')));
console.log('algorithm:' + codeOf(() =>
    crypto.createCipheriv('invalid-alg', validKey, validIv)));
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("key:ERR_CRYPTO_INVALID_KEYLEN"),
        "Expected Node-compatible invalid key error code: {}",
        output
    );
    assert!(
        output.contains("iv:ERR_CRYPTO_INVALID_IV"),
        "Expected Node-compatible invalid IV error code: {}",
        output
    );
    assert!(
        output.contains("algorithm:ERR_CRYPTO_UNKNOWN_CIPHER"),
        "Expected Node-compatible unknown cipher error code: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_aes_ctr_invalid_iv_length() {
    let code = r#"
try {
    // CTR mode also requires a 16-byte counter/IV for AES.
    crypto.createCipheriv('aes-256-ctr', '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef', 'shortiv');
    console.log('FAIL');
} catch (e) {
    console.log(e.message.includes('iv') || e.message.includes('invalid') ? 'PASS' : 'FAIL');
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected error for invalid AES-CTR IV length: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_update_and_final() {
    let code = r#"
const key = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
const iv = 'abcdef0123456789abcdef0123456789';
const cipher = crypto.createCipheriv('aes-256-cbc', key, iv);
const encrypted = cipher.update('Hello, World!', 'utf8', 'hex') + cipher.final('hex');
console.log(encrypted.length > 0 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected cipher to produce encrypted output: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_update_buffer() {
    let code = r#"
const key = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
const iv = 'abcdef0123456789abcdef0123456789';
const cipher = crypto.createCipheriv('aes-256-cbc', key, iv);
const encrypted = cipher.update(Buffer.from('Hello'), 'utf8', 'hex') + cipher.final('hex');
console.log(encrypted.length > 0 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected cipher to work with Buffer input: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_aes_cbc_split_updates_keep_chaining_state() {
    let code = r#"
const key = '00112233445566778899aabbccddeeff';
const iv = '0102030405060708090a0b0c0d0e0f10';
const first = '000102030405060708090a0b0c0d0e0f';
const second = '101112131415161718191a1b1c1d1e1f';
const expected = 'da1dca49b61ef24bdd0e15e681c8a1ba4a8588657b946e13ed4f5f6a3cc66cf5b04e433e26a6a25da21cdeedc9d34611';

const cipher = crypto.createCipheriv('aes-128-cbc', key, iv);
const encrypted = cipher.update(first, 'hex', 'hex') + cipher.update(second, 'hex', 'hex') + cipher.final('hex');

console.log(encrypted === expected ? 'PASS' : `FAIL:${encrypted}`);
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected split AES-CBC updates to keep CBC chaining state: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_decipheriv_aes_cbc_split_updates_read_node_ciphertext() {
    let code = r#"
const key = '00112233445566778899aabbccddeeff';
const iv = '0102030405060708090a0b0c0d0e0f10';
const firstCipherBlock = 'da1dca49b61ef24bdd0e15e681c8a1ba';
const restCiphertext = '4a8588657b946e13ed4f5f6a3cc66cf5b04e433e26a6a25da21cdeedc9d34611';
const expected = '000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f';

const decipher = crypto.createDecipheriv('aes-128-cbc', key, iv);
const decrypted = decipher.update(firstCipherBlock, 'hex', 'hex') + decipher.update(restCiphertext, 'hex', 'hex') + decipher.final('hex');

console.log(decrypted === expected ? 'PASS' : `FAIL:${decrypted}`);
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected split AES-CBC decrypt updates to read Node/OpenSSL ciphertext: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_decipheriv_aes_ctr_update_emits_before_final() {
    let code = r#"
const key = '00112233445566778899aabbccddeeff';
const iv = '0102030405060708090a0b0c0d0e0f10';

const cipher = crypto.createCipheriv('aes-128-ctr', key, iv);
const c1 = cipher.update('00', 'hex', 'hex');
const c2 = cipher.update('010203', 'hex', 'hex');
const c3 = cipher.final('hex');

const decipher = crypto.createDecipheriv('aes-128-ctr', key, iv);
const p1 = decipher.update(c1, 'hex', 'hex');
const p2 = decipher.update(c2, 'hex', 'hex');
const p3 = decipher.final('hex');

console.log(c1 === 'bf' ? 'PASS' : `FAIL:c1:${c1}`);
console.log(c2 === '642bbd' ? 'PASS' : `FAIL:c2:${c2}`);
console.log(c3 === '' ? 'PASS' : `FAIL:c3:${c3}`);
console.log(p1 === '00' ? 'PASS' : `FAIL:p1:${p1}`);
console.log(p2 === '010203' ? 'PASS' : `FAIL:p2:${p2}`);
console.log(p3 === '' ? 'PASS' : `FAIL:p3:${p3}`);
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 6,
        "Expected AES-CTR update() to emit output before final(): {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_decipheriv_aes_cbc_update_emits_node_compatible_chunks() {
    let code = r#"
const key = '00112233445566778899aabbccddeeff';
const iv = '0102030405060708090a0b0c0d0e0f10';
const first = '000102030405060708090a0b0c0d0e0f';
const second = '101112131415161718191a1b1c1d1e1f';

const cipher = crypto.createCipheriv('aes-128-cbc', key, iv);
const c1 = cipher.update(first, 'hex', 'hex');
const c2 = cipher.update(second, 'hex', 'hex');
const c3 = cipher.final('hex');

const decipher = crypto.createDecipheriv('aes-128-cbc', key, iv);
const p1 = decipher.update(c1, 'hex', 'hex');
const p2 = decipher.update(c2, 'hex', 'hex');
const p3 = decipher.update(c3, 'hex', 'hex');
const p4 = decipher.final('hex');

console.log(c1 === 'da1dca49b61ef24bdd0e15e681c8a1ba' ? 'PASS' : `FAIL:c1:${c1}`);
console.log(c2 === '4a8588657b946e13ed4f5f6a3cc66cf5' ? 'PASS' : `FAIL:c2:${c2}`);
console.log(c3 === 'b04e433e26a6a25da21cdeedc9d34611' ? 'PASS' : `FAIL:c3:${c3}`);
console.log(p1 === '' ? 'PASS' : `FAIL:p1:${p1}`);
console.log(p2 === first ? 'PASS' : `FAIL:p2:${p2}`);
console.log(p3 === second ? 'PASS' : `FAIL:p3:${p3}`);
console.log(p4 === '' ? 'PASS' : `FAIL:p4:${p4}`);
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 7,
        "Expected AES-CBC update()/final() chunks to match Node output timing: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_decipheriv_aes_cfb_ofb_update_emits_before_final() {
    let code = r#"
const key = '00112233445566778899aabbccddeeff';
const iv = '0102030405060708090a0b0c0d0e0f10';
const chunks = ['00', '010203', '040506070809'];
const expectedCipherChunks = ['bf', '642bbd', '697055acefbd'];
const expectedPlainChunks = chunks;
const algorithms = ['aes-128-cfb', 'aes-128-ofb'];

for (const algorithm of algorithms) {
    const cipher = crypto.createCipheriv(algorithm, key, iv);
    const encrypted = chunks.map((chunk) => cipher.update(chunk, 'hex', 'hex'));
    const cipherFinal = cipher.final('hex');

    const decipher = crypto.createDecipheriv(algorithm, key, iv);
    const decrypted = encrypted.map((chunk) => decipher.update(chunk, 'hex', 'hex'));
    const decipherFinal = decipher.final('hex');

    console.log(encrypted.join('|') === expectedCipherChunks.join('|') ? 'PASS' : `FAIL:${algorithm}:cipher:${encrypted.join('|')}`);
    console.log(cipherFinal === '' ? 'PASS' : `FAIL:${algorithm}:cipherFinal:${cipherFinal}`);
    console.log(decrypted.join('|') === expectedPlainChunks.join('|') ? 'PASS' : `FAIL:${algorithm}:plain:${decrypted.join('|')}`);
    console.log(decipherFinal === '' ? 'PASS' : `FAIL:${algorithm}:decipherFinal:${decipherFinal}`);
}
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 8,
        "Expected AES-CFB/OFB update() to emit output before final(): {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_decipheriv_aes_gcm_auth_tag_and_aad_match_node_vector() {
    let code = r#"
function bytes(hex) {
    const out = new Uint8Array(hex.length / 2);
    for (let i = 0; i < out.length; i++) {
        out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
    }
    return out;
}

function hex(data) {
    return Array.from(data).map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

const key = bytes('000102030405060708090a0b0c0d0e0f');
const iv = bytes('101112131415161718191a1b');
const aad = bytes('feedfacedeadbeef');
const plain = bytes('00112233445566778899aabbccddeeff');

const cipher = crypto.createCipheriv('aes-128-gcm', key, iv);
cipher.setAAD(aad);
const encrypted = cipher.update(plain, 'buffer', 'hex') + cipher.final('hex');
const tag = cipher.getAuthTag();

const decipher = crypto.createDecipheriv('aes-128-gcm', key, iv);
decipher.setAAD(aad);
decipher.setAuthTag(tag);
const decrypted = decipher.update(encrypted, 'hex', 'hex') + decipher.final('hex');

console.log(typeof cipher.setAAD === 'function' ? 'PASS' : 'FAIL:no-setAAD');
console.log(typeof cipher.getAuthTag === 'function' ? 'PASS' : 'FAIL:no-getAuthTag');
console.log(typeof decipher.setAuthTag === 'function' ? 'PASS' : 'FAIL:no-setAuthTag');
console.log(encrypted === 'c43f219c4b1ad0989f44f74e0bfa05c1' ? 'PASS' : `FAIL:cipher:${encrypted}`);
console.log(hex(tag) === '8904c9157db4cc2579d0d2226f069e32' ? 'PASS' : `FAIL:tag:${hex(tag)}`);
console.log(decrypted === '00112233445566778899aabbccddeeff' ? 'PASS' : `FAIL:plain:${decrypted}`);
"#;
    let output = run_js_test(code);
    let pass_count = output.lines().filter(|line| *line == "PASS").count();
    assert_eq!(
        pass_count, 6,
        "Expected AES-GCM cipher/decipher with AAD/authTag to match Node vector: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_decipheriv_aes_gcm_rejects_wrong_auth_tag() {
    let code = r#"
function bytes(hex) {
    const out = new Uint8Array(hex.length / 2);
    for (let i = 0; i < out.length; i++) {
        out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
    }
    return out;
}

const key = bytes('000102030405060708090a0b0c0d0e0f');
const iv = bytes('101112131415161718191a1b');
const aad = bytes('feedfacedeadbeef');
const encrypted = 'c43f219c4b1ad0989f44f74e0bfa05c1';
const wrongTag = bytes('00000000000000000000000000000000');

try {
    const decipher = crypto.createDecipheriv('aes-128-gcm', key, iv);
    decipher.setAAD(aad);
    decipher.setAuthTag(wrongTag);
    decipher.update(encrypted, 'hex', 'hex');
    decipher.final('hex');
    console.log('FAIL:no-error');
} catch (error) {
    const message = String(error && error.message || error);
    console.log(message.includes('auth') || message.includes('final') || message.includes('bad decrypt') ? 'PASS' : `FAIL:${message}`);
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected AES-GCM wrong auth tag to fail closed: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_set_auto_padding_false_encrypts_without_extra_block() {
    let code = r#"
const key = '00112233445566778899aabbccddeeff';
const iv = '0102030405060708090a0b0c0d0e0f10';
const block = '00112233445566778899aabbccddeeff';

const paddedCipher = crypto.createCipheriv('aes-128-cbc', key, iv);
const padded = paddedCipher.update(block, 'hex', 'hex') + paddedCipher.final('hex');

const rawCipher = crypto.createCipheriv('aes-128-cbc', key, iv);
const returned = rawCipher.setAutoPadding(false);
const raw = rawCipher.update(block, 'hex', 'hex') + rawCipher.final('hex');

console.log(returned === rawCipher && raw.length === 32 && padded.length === 64 && raw !== padded ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected setAutoPadding(false) to avoid a padding block and return this: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_set_auto_padding_false_rejects_partial_block() {
    let code = r#"
const key = '00112233445566778899aabbccddeeff';
const iv = '0102030405060708090a0b0c0d0e0f10';

try {
    const cipher = crypto.createCipheriv('aes-128-cbc', key, iv);
    cipher.setAutoPadding(false);
    cipher.update('001122', 'hex', 'hex');
    cipher.final('hex');
    console.log('FAIL');
} catch (e) {
    console.log(e.message.includes('final') || e.message.includes('block') ? 'PASS' : 'FAIL');
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected setAutoPadding(false) to reject partial final blocks: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_double_final_throws() {
    let code = r#"
const key = '00112233445566778899aabbccddeeff';
const iv = '0102030405060708090a0b0c0d0e0f10';

try {
    const cipher = crypto.createCipheriv('aes-128-cbc', key, iv);
    cipher.update('hello', 'utf8', 'hex');
    cipher.final('hex');
    cipher.final('hex');
    console.log('FAIL');
} catch (e) {
    console.log(e.message.includes('final') || e.message.includes('finalized') ? 'PASS' : 'FAIL');
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected cipher.final() after final() to throw: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_update_after_final_throws() {
    let code = r#"
const key = '00112233445566778899aabbccddeeff';
const iv = '0102030405060708090a0b0c0d0e0f10';

try {
    const cipher = crypto.createCipheriv('aes-128-cbc', key, iv);
    cipher.update('hello', 'utf8', 'hex');
    cipher.final('hex');
    cipher.update('again', 'utf8', 'hex');
    console.log('FAIL');
} catch (e) {
    console.log(e.message.includes('update') || e.message.includes('finalized') ? 'PASS' : 'FAIL');
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected cipher.update() after final() to throw: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_cipheriv_decrypt_round_trip() {
    let code = r#"
const key = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
const iv = 'abcdef0123456789abcdef0123456789';
const original = 'Hello, World!';

// Encrypt
const cipher = crypto.createCipheriv('aes-256-cbc', key, iv);
const encrypted = cipher.update(original, 'utf8', 'hex') + cipher.final('hex');

// Decrypt using createDecipheriv
const decipher = crypto.createDecipheriv('aes-256-cbc', key, iv);
const decrypted = decipher.update(encrypted, 'hex', 'utf8') + decipher.final('utf8');

console.log(decrypted === original ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected encrypt/decrypt round trip to work: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_decipheriv_update_after_final_throws() {
    let code = r#"
const key = '00112233445566778899aabbccddeeff';
const iv = '0102030405060708090a0b0c0d0e0f10';
const cipher = crypto.createCipheriv('aes-128-cbc', key, iv);
const encrypted = cipher.update('hello', 'utf8', 'hex') + cipher.final('hex');

try {
    const decipher = crypto.createDecipheriv('aes-128-cbc', key, iv);
    decipher.update(encrypted, 'hex', 'utf8');
    decipher.final('utf8');
    decipher.update(encrypted, 'hex', 'utf8');
    console.log('FAIL');
} catch (e) {
    console.log(e.message.includes('update') || e.message.includes('finalized') ? 'PASS' : 'FAIL');
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected decipher.update() after final() to throw: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_decipheriv_set_auto_padding_false_keeps_padding_bytes() {
    let code = r#"
const key = '00112233445566778899aabbccddeeff';
const iv = '0102030405060708090a0b0c0d0e0f10';

const cipher = crypto.createCipheriv('aes-128-cbc', key, iv);
const encrypted = cipher.update('hello', 'utf8', 'hex') + cipher.final('hex');

const decipher = crypto.createDecipheriv('aes-128-cbc', key, iv);
decipher.setAutoPadding(false);
const plain = decipher.update(encrypted, 'hex', 'latin1') + decipher.final('latin1');
const pad = plain.charCodeAt(plain.length - 1);

console.log(plain.length === 16 && plain.slice(0, 5) === 'hello' && pad === 11 ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected decipher setAutoPadding(false) to preserve PKCS#7 padding bytes: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_decipheriv_function_exists() {
    let code = r#"
console.log(typeof crypto.createDecipheriv === 'function' ? 'PASS' : 'FAIL');
"#;
    let output = run_js_test(code);
    assert!(
        output.trim() == "PASS",
        "Expected createDecipheriv to exist: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_decipheriv_has_update_method() {
    let code = r#"
const key = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
const iv = 'abcdef0123456789abcdef0123456789';
const decipher = crypto.createDecipheriv('aes-256-cbc', key, iv);
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
fn test_create_decipheriv_has_finalize_method() {
    let code = r#"
const key = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
const iv = 'abcdef0123456789abcdef0123456789';
const decipher = crypto.createDecipheriv('aes-256-cbc', key, iv);
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
fn test_create_decipheriv_invalid_algorithm() {
    let code = r#"
try {
    crypto.createDecipheriv('invalid-alg', 'key', 'iv');
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
fn test_create_decipheriv_invalid_key_length() {
    let code = r#"
try {
    // AES-256 requires a 32-byte key, using 8 decoded bytes should fail.
    crypto.createDecipheriv('aes-256-cbc', '0123456789abcdef', 'abcdef0123456789abcdef0123456789');
    console.log('FAIL');
} catch (e) {
    console.log(e.message.includes('invalid') || e.message.includes('key') ? 'PASS' : 'FAIL');
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected error for invalid decipheriv key length: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_decipheriv_invalid_iv_length() {
    let code = r#"
try {
    // CBC mode requires a 16-byte IV.
    crypto.createDecipheriv('aes-256-cbc', '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef', 'shortiv');
    console.log('FAIL');
} catch (e) {
    console.log(e.message.includes('iv') || e.message.includes('invalid') ? 'PASS' : 'FAIL');
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected error for invalid decipheriv IV length: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_decipheriv_invalid_input_error_codes_match_node() {
    let code = r#"
function codeOf(fn) {
    try {
        fn();
        return 'NO_ERROR';
    } catch (e) {
        return e && e.code ? e.code : String(e && e.code);
    }
}

const validKey = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
const validIv = 'abcdef0123456789abcdef0123456789';

console.log('key:' + codeOf(() =>
    crypto.createDecipheriv('aes-256-cbc', '0123456789abcdef', validIv)));
console.log('iv:' + codeOf(() =>
    crypto.createDecipheriv('aes-256-cbc', validKey, 'shortiv')));
console.log('algorithm:' + codeOf(() =>
    crypto.createDecipheriv('invalid-alg', validKey, validIv)));
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("key:ERR_CRYPTO_INVALID_KEYLEN"),
        "Expected Node-compatible invalid key error code: {}",
        output
    );
    assert!(
        output.contains("iv:ERR_CRYPTO_INVALID_IV"),
        "Expected Node-compatible invalid IV error code: {}",
        output
    );
    assert!(
        output.contains("algorithm:ERR_CRYPTO_UNKNOWN_CIPHER"),
        "Expected Node-compatible unknown cipher error code: {}",
        output
    );
}

#[test]
#[serial]
fn test_create_decipheriv_aes_ctr_invalid_iv_length() {
    let code = r#"
try {
    // CTR mode also requires a 16-byte counter/IV for AES.
    crypto.createDecipheriv('aes-256-ctr', '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef', 'shortiv');
    console.log('FAIL');
} catch (e) {
    console.log(e.message.includes('iv') || e.message.includes('invalid') ? 'PASS' : 'FAIL');
}
"#;
    let output = run_js_test(code);
    assert!(
        output.contains("PASS"),
        "Expected error for invalid decipheriv AES-CTR IV length: {}",
        output
    );
}

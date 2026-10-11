//! Pins docs/NODE_CRYPTO_CIPHERIV_GCM_CONTRACT.md — AES-*-GCM createCipheriv/createDecipheriv
//! plus getAuthTag/setAuthTag/setAAD (G51).
//! Does **not** pin password createCipher/createDecipher, AES-CTR/CFB/OFB/ECB as Stable.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn runtime() -> MinimalRuntime {
    MinimalRuntime::new().expect("Failed to create runtime")
}

fn run(code: &str) -> String {
    runtime()
        .execute_code(code)
        .unwrap_or_else(|err| panic!("Execution failed: {err}"))
        .trim()
        .to_string()
}

#[test]
#[serial]
fn require_crypto_exports_cipheriv_gcm_helpers() {
    let result = run(r#"
        const c = require('crypto');
        const n = require('node:crypto');
        const key = '000102030405060708090a0b0c0d0e0f';
        const iv = '101112131415161718191a1b';
        const cipher = c.createCipheriv('aes-128-gcm', key, iv);
        const decipher = c.createDecipheriv('aes-128-gcm', key, iv);
        [
          typeof c.createCipheriv,
          typeof c.createDecipheriv,
          c.createCipheriv === n.createCipheriv,
          typeof cipher.setAAD,
          typeof cipher.getAuthTag,
          typeof decipher.setAAD,
          typeof decipher.setAuthTag,
          typeof cipher.setAuthTag,
          typeof decipher.getAuthTag
        ].join('|');
        "#);
    // Encrypt exposes getAuthTag; decrypt exposes setAuthTag (not the reverse).
    assert_eq!(
        result,
        "function|function|true|function|function|function|function|undefined|undefined"
    );
}

#[test]
#[serial]
fn aes_gcm_algorithms_and_aliases_round_trip() {
    let result = run(r#"
        const c = require('crypto');
        const iv = '101112131415161718191a1b';
        const cases = [
          ['aes-128-gcm', '000102030405060708090a0b0c0d0e0f'],
          ['aes-192-gcm', '000102030405060708090a0b0c0d0e0f1011121314151617'],
          ['aes-256-gcm', '000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f'],
          ['aes128-gcm', '000102030405060708090a0b0c0d0e0f'],
          ['aes192-gcm', '000102030405060708090a0b0c0d0e0f1011121314151617'],
          ['aes256-gcm', '000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f'],
          ['AES-128-GCM', '000102030405060708090a0b0c0d0e0f'],
        ];
        const out = [];
        for (const [algorithm, key] of cases) {
          const cipher = c.createCipheriv(algorithm, key, iv);
          const encrypted = cipher.update('Amber GCM', 'utf8', 'hex') + cipher.final('hex');
          const tag = cipher.getAuthTag();
          const decipher = c.createDecipheriv(algorithm, key, iv);
          decipher.setAuthTag(tag);
          const decrypted = decipher.update(encrypted, 'hex', 'utf8') + decipher.final('utf8');
          out.push(decrypted === 'Amber GCM' ? 'ok' : `fail:${algorithm}`);
        }
        out.join('|');
        "#);
    assert_eq!(result, "ok|ok|ok|ok|ok|ok|ok");
}

#[test]
#[serial]
fn aes_128_gcm_known_vector_aad_and_auth_tag() {
    let result = run(r#"
        const c = require('crypto');
        function bytes(hex) {
          const out = new Uint8Array(hex.length / 2);
          for (let i = 0; i < out.length; i++) {
            out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
          }
          return out;
        }
        function hex(data) {
          return Array.from(data).map((b) => b.toString(16).padStart(2, '0')).join('');
        }
        const key = bytes('000102030405060708090a0b0c0d0e0f');
        const iv = bytes('101112131415161718191a1b');
        const aad = bytes('feedfacedeadbeef');
        const plain = bytes('00112233445566778899aabbccddeeff');
        const expectedCipher = 'c43f219c4b1ad0989f44f74e0bfa05c1';
        const expectedTag = '8904c9157db4cc2579d0d2226f069e32';

        const cipher = c.createCipheriv('aes-128-gcm', key, iv);
        const returned = cipher.setAAD(aad);
        // Match Preview pin / Node-shaped byte-view update (inputEncoding 'buffer').
        const encrypted = cipher.update(plain, 'buffer', 'hex') + cipher.final('hex');
        const tag = cipher.getAuthTag();

        const decipher = c.createDecipheriv('aes-128-gcm', key, iv);
        decipher.setAAD(aad);
        decipher.setAuthTag(tag);
        const decrypted = decipher.update(encrypted, 'hex', 'hex') + decipher.final('hex');

        [
          returned === cipher,
          encrypted,
          hex(tag),
          decrypted,
          encrypted === expectedCipher,
          hex(tag) === expectedTag,
          decrypted === '00112233445566778899aabbccddeeff'
        ].join('|');
        "#);
    assert_eq!(
        result,
        "true|c43f219c4b1ad0989f44f74e0bfa05c1|8904c9157db4cc2579d0d2226f069e32|00112233445566778899aabbccddeeff|true|true|true"
    );
}

#[test]
#[serial]
fn wrong_auth_tag_fails_closed_and_missing_tag_throws() {
    let result = run(r#"
        const c = require('crypto');
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

        let wrong = '';
        try {
          const d = c.createDecipheriv('aes-128-gcm', key, iv);
          d.setAAD(aad);
          d.setAuthTag(wrongTag);
          d.update(encrypted, 'hex', 'hex');
          d.final('hex');
          wrong = 'ok';
        } catch (e) {
          const m = String(e && e.message || e);
          wrong = (m.includes('auth') || m.includes('final') || m.includes('bad decrypt'))
            ? 'throw' : m;
        }

        let missing = '';
        try {
          const d = c.createDecipheriv('aes-128-gcm', key, iv);
          d.setAAD(aad);
          d.update(encrypted, 'hex', 'hex');
          d.final('hex');
          missing = 'ok';
        } catch (e) {
          const m = String(e && e.message || e);
          missing = m.includes('auth tag') ? 'throw' : m;
        }

        let beforeFinal = '';
        try {
          const cipher = c.createCipheriv('aes-128-gcm', key, iv);
          cipher.getAuthTag();
          beforeFinal = 'ok';
        } catch (e) {
          const m = String(e && e.message || e);
          beforeFinal = (m.includes('auth tag') || m.includes('before final'))
            ? 'throw' : m;
        }

        [wrong, missing, beforeFinal].join('|');
        "#);
    assert_eq!(result, "throw|throw|throw");
}

#[test]
#[serial]
fn invalid_key_iv_algorithm_error_codes_for_gcm() {
    let result = run(r#"
        const c = require('crypto');
        function codeOf(fn) {
          try { fn(); return 'NO_ERROR'; }
          catch (e) { return e && e.code ? e.code : String(e && e.message); }
        }
        const validKey = '000102030405060708090a0b0c0d0e0f';
        const validIv = '101112131415161718191a1b';
        [
          codeOf(() => c.createCipheriv('aes-128-gcm', '0123456789abcdef', validIv)),
          codeOf(() => c.createCipheriv('aes-128-gcm', validKey, 'shortiv')),
          codeOf(() => c.createCipheriv('aes-128-gcm', validKey, '101112131415161718191a1b1c1d1e1f')),
          codeOf(() => c.createCipheriv('invalid-alg', validKey, validIv)),
          codeOf(() => c.createDecipheriv('aes-128-gcm', '0123456789abcdef', validIv))
        ].join('|');
        "#);
    assert_eq!(
        result,
        "ERR_CRYPTO_INVALID_KEYLEN|ERR_CRYPTO_INVALID_IV|ERR_CRYPTO_INVALID_IV|ERR_CRYPTO_UNKNOWN_CIPHER|ERR_CRYPTO_INVALID_KEYLEN"
    );
}

#[test]
#[serial]
fn g43_cbc_and_g23_hash_untouched_password_create_cipher_stays_non_goal() {
    let result = run(r#"
        const c = require('crypto');
        const cbcKey = '00112233445566778899aabbccddeeff';
        const cbcIv = '0102030405060708090a0b0c0d0e0f10';
        const cipher = c.createCipheriv('aes-128-cbc', cbcKey, cbcIv);
        const encrypted = cipher.update('hello', 'utf8', 'hex') + cipher.final('hex');
        const decipher = c.createDecipheriv('aes-128-cbc', cbcKey, cbcIv);
        const plain = decipher.update(encrypted, 'hex', 'utf8') + decipher.final('utf8');
        const sha = c.createHash('sha256').update('hello').digest('hex');
        [
          plain,
          sha,
          typeof c.createCipher,
          typeof cipher.setAAD,
          typeof cipher.getAuthTag
        ].join('|');
        "#);
    // CBC objects do not expose GCM helpers; createCipher remains Preview (Non-goal).
    assert_eq!(
        result,
        "hello|2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824|function|undefined|undefined"
    );
}

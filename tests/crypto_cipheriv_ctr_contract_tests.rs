//! Pins docs/NODE_CRYPTO_CIPHERIV_CTR_CONTRACT.md — AES-*-CTR createCipheriv/createDecipheriv (G52).
//! Additive to G43 (CBC) / G51 (GCM). Does **not** pin password createCipher/createDecipher
//! or other cipher modes as Stable on this page.

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
fn require_crypto_exports_cipheriv_ctr_functions() {
    let result = run(r#"
        const c = require('crypto');
        const n = require('node:crypto');
        [
          typeof c.createCipheriv,
          typeof c.createDecipheriv,
          c.createCipheriv === n.createCipheriv,
          c.createCipheriv === crypto.createCipheriv,
          c.createDecipheriv === crypto.createDecipheriv
        ].join('|');
        "#);
    assert_eq!(result, "function|function|true|true|true");
}

#[test]
#[serial]
fn aes_ctr_algorithms_and_aliases_round_trip() {
    let result = run(r#"
        const c = require('crypto');
        const iv = '0102030405060708090a0b0c0d0e0f10';
        const cases = [
          ['aes-128-ctr', '00112233445566778899aabbccddeeff'],
          ['aes-192-ctr', '00112233445566778899aabbccddeeff0011223344556677'],
          ['aes-256-ctr', '00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff'],
          ['aes128-ctr', '00112233445566778899aabbccddeeff'],
          ['aes192-ctr', '00112233445566778899aabbccddeeff0011223344556677'],
          ['aes256-ctr', '00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff'],
        ];
        const out = [];
        for (const [algorithm, key] of cases) {
          const cipher = c.createCipheriv(algorithm, key, iv);
          const encrypted = cipher.update('Amber CTR', 'utf8', 'hex') + cipher.final('hex');
          const decipher = c.createDecipheriv(algorithm, key, iv);
          const decrypted = decipher.update(encrypted, 'hex', 'utf8') + decipher.final('utf8');
          out.push(decrypted === 'Amber CTR' ? 'ok' : `fail:${algorithm}`);
        }
        out.join('|');
        "#);
    assert_eq!(result, "ok|ok|ok|ok|ok|ok");
}

#[test]
#[serial]
fn aes_128_ctr_known_vector_and_split_updates() {
    let result = run(r#"
        const c = require('crypto');
        const key = '00112233445566778899aabbccddeeff';
        const iv = '0102030405060708090a0b0c0d0e0f10';

        const cipher = c.createCipheriv('aes-128-ctr', key, iv);
        const c1 = cipher.update('00', 'hex', 'hex');
        const c2 = cipher.update('010203', 'hex', 'hex');
        const c3 = cipher.final('hex');

        const decipher = c.createDecipheriv('aes-128-ctr', key, iv);
        const p1 = decipher.update(c1, 'hex', 'hex');
        const p2 = decipher.update(c2, 'hex', 'hex');
        const p3 = decipher.final('hex');

        [
          c1 === 'bf',
          c2 === '642bbd',
          c3 === '',
          p1 === '00',
          p2 === '010203',
          p3 === ''
        ].join('|');
        "#);
    assert_eq!(result, "true|true|true|true|true|true");
}

#[test]
#[serial]
fn invalid_key_iv_algorithm_error_codes() {
    let result = run(r#"
        const c = require('crypto');
        function codeOf(fn) {
          try { fn(); return 'NO_ERROR'; }
          catch (e) { return e && e.code ? e.code : String(e && e.message); }
        }
        const validKey = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
        const validIv = 'abcdef0123456789abcdef0123456789';
        [
          codeOf(() => c.createCipheriv('aes-256-ctr', '0123456789abcdef', validIv)),
          codeOf(() => c.createCipheriv('aes-256-ctr', validKey, 'shortiv')),
          codeOf(() => c.createCipheriv('invalid-alg', validKey, validIv)),
          codeOf(() => c.createDecipheriv('aes-256-ctr', '0123456789abcdef', validIv))
        ].join('|');
        "#);
    assert_eq!(
        result,
        "ERR_CRYPTO_INVALID_KEYLEN|ERR_CRYPTO_INVALID_IV|ERR_CRYPTO_UNKNOWN_CIPHER|ERR_CRYPTO_INVALID_KEYLEN"
    );
}

#[test]
#[serial]
fn update_after_final_throws_and_sibling_carves_untouched() {
    let result = run(r#"
        const c = require('crypto');
        const key = '00112233445566778899aabbccddeeff';
        const iv = '0102030405060708090a0b0c0d0e0f10';
        const cipher = c.createCipheriv('aes-128-ctr', key, iv);
        const encrypted = cipher.update('hello', 'utf8', 'hex') + cipher.final('hex');
        let afterFinal = '';
        try {
          const decipher = c.createDecipheriv('aes-128-ctr', key, iv);
          decipher.update(encrypted, 'hex', 'utf8');
          decipher.final('utf8');
          decipher.update(encrypted, 'hex', 'utf8');
          afterFinal = 'ok';
        } catch (e) {
          afterFinal = (String(e.message).includes('update') || String(e.message).includes('finalized'))
            ? 'throw' : String(e.message);
        }
        const sha = c.createHash('sha256').update('hello').digest('hex');
        const cbc = c.createCipheriv('aes-128-cbc', key, iv);
        const cbcEnc = cbc.update('hello', 'utf8', 'hex') + cbc.final('hex');
        const cbcDec = c.createDecipheriv('aes-128-cbc', key, iv);
        const cbcPlain = cbcDec.update(cbcEnc, 'hex', 'utf8') + cbcDec.final('utf8');
        [
          afterFinal,
          sha,
          typeof c.createCipher === 'function',
          cbcPlain === 'hello',
          typeof c.createCipheriv('aes-128-ctr', key, iv).setAAD
        ].join('|');
        "#);
    // createCipher exists (Preview Non-goal); CTR cipher has no setAAD (GCM-only helper).
    assert_eq!(
        result,
        "throw|2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824|true|true|undefined"
    );
}

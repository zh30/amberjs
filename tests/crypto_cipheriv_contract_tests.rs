//! Pins docs/NODE_CRYPTO_CIPHERIV_CONTRACT.md — AES-*-CBC createCipheriv/createDecipheriv (G43).
//! Does **not** pin password createCipher/createDecipher, AES-GCM, or CTR/CFB/OFB/ECB as Stable.

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
fn require_crypto_exports_cipheriv_cbc_functions() {
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
fn aes_cbc_algorithms_and_bare_aliases_round_trip() {
    let result = run(r#"
        const c = require('crypto');
        const iv = '0102030405060708090a0b0c0d0e0f10';
        const cases = [
          ['aes-128-cbc', '00112233445566778899aabbccddeeff'],
          ['aes-192-cbc', '00112233445566778899aabbccddeeff0011223344556677'],
          ['aes-256-cbc', '00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff'],
          ['aes128', '00112233445566778899aabbccddeeff'],
          ['aes192', '00112233445566778899aabbccddeeff0011223344556677'],
          ['aes256', '00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff'],
        ];
        const out = [];
        for (const [algorithm, key] of cases) {
          const cipher = c.createCipheriv(algorithm, key, iv);
          const encrypted = cipher.update('Amber CBC', 'utf8', 'hex') + cipher.final('hex');
          const decipher = c.createDecipheriv(algorithm, key, iv);
          const decrypted = decipher.update(encrypted, 'hex', 'utf8') + decipher.final('utf8');
          out.push(decrypted === 'Amber CBC' ? 'ok' : `fail:${algorithm}`);
        }
        out.join('|');
        "#);
    assert_eq!(result, "ok|ok|ok|ok|ok|ok");
}

#[test]
#[serial]
fn aes_128_cbc_known_vector_and_split_updates() {
    let result = run(r#"
        const c = require('crypto');
        const key = '00112233445566778899aabbccddeeff';
        const iv = '0102030405060708090a0b0c0d0e0f10';
        const first = '000102030405060708090a0b0c0d0e0f';
        const second = '101112131415161718191a1b1c1d1e1f';
        const expected = 'da1dca49b61ef24bdd0e15e681c8a1ba4a8588657b946e13ed4f5f6a3cc66cf5b04e433e26a6a25da21cdeedc9d34611';

        const cipher = c.createCipheriv('aes-128-cbc', key, iv);
        const c1 = cipher.update(first, 'hex', 'hex');
        const c2 = cipher.update(second, 'hex', 'hex');
        const c3 = cipher.final('hex');
        const encrypted = c1 + c2 + c3;

        const decipher = c.createDecipheriv('aes-128-cbc', key, iv);
        const p1 = decipher.update(c1, 'hex', 'hex');
        const p2 = decipher.update(c2, 'hex', 'hex');
        const p3 = decipher.update(c3, 'hex', 'hex');
        const p4 = decipher.final('hex');

        [
          encrypted === expected,
          c1 === 'da1dca49b61ef24bdd0e15e681c8a1ba',
          c2 === '4a8588657b946e13ed4f5f6a3cc66cf5',
          c3 === 'b04e433e26a6a25da21cdeedc9d34611',
          p1 === '',
          p2 === first,
          p3 === second,
          p4 === ''
        ].join('|');
        "#);
    assert_eq!(result, "true|true|true|true|true|true|true|true");
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
          codeOf(() => c.createCipheriv('aes-256-cbc', '0123456789abcdef', validIv)),
          codeOf(() => c.createCipheriv('aes-256-cbc', validKey, 'shortiv')),
          codeOf(() => c.createCipheriv('invalid-alg', validKey, validIv)),
          codeOf(() => c.createDecipheriv('aes-256-cbc', '0123456789abcdef', validIv))
        ].join('|');
        "#);
    assert_eq!(
        result,
        "ERR_CRYPTO_INVALID_KEYLEN|ERR_CRYPTO_INVALID_IV|ERR_CRYPTO_UNKNOWN_CIPHER|ERR_CRYPTO_INVALID_KEYLEN"
    );
}

#[test]
#[serial]
fn set_auto_padding_false_omits_extra_block_on_aligned_plaintext() {
    let result = run(r#"
        const c = require('crypto');
        const key = '00112233445566778899aabbccddeeff';
        const iv = '0102030405060708090a0b0c0d0e0f10';
        const block = '00112233445566778899aabbccddeeff';

        const paddedCipher = c.createCipheriv('aes-128-cbc', key, iv);
        const padded = paddedCipher.update(block, 'hex', 'hex') + paddedCipher.final('hex');

        const rawCipher = c.createCipheriv('aes-128-cbc', key, iv);
        const returned = rawCipher.setAutoPadding(false);
        const raw = rawCipher.update(block, 'hex', 'hex') + rawCipher.final('hex');

        [
          returned === rawCipher,
          raw.length === 32,
          padded.length === 64,
          raw !== padded
        ].join('|');
        "#);
    assert_eq!(result, "true|true|true|true");
}

#[test]
#[serial]
fn update_after_final_throws_and_g23_hash_untouched() {
    let result = run(r#"
        const c = require('crypto');
        const key = '00112233445566778899aabbccddeeff';
        const iv = '0102030405060708090a0b0c0d0e0f10';
        const cipher = c.createCipheriv('aes-128-cbc', key, iv);
        const encrypted = cipher.update('hello', 'utf8', 'hex') + cipher.final('hex');
        let afterFinal = '';
        try {
          const decipher = c.createDecipheriv('aes-128-cbc', key, iv);
          decipher.update(encrypted, 'hex', 'utf8');
          decipher.final('utf8');
          decipher.update(encrypted, 'hex', 'utf8');
          afterFinal = 'ok';
        } catch (e) {
          afterFinal = (String(e.message).includes('update') || String(e.message).includes('finalized'))
            ? 'throw' : String(e.message);
        }
        const sha = c.createHash('sha256').update('hello').digest('hex');
        [
          afterFinal,
          sha,
          typeof c.createCipher === 'function',
          typeof c.createCipheriv('aes-128-cbc', key, iv).setAAD
        ].join('|');
        "#);
    // createCipher exists (Preview Non-goal); CBC cipher has no setAAD (GCM-only helper).
    assert_eq!(
        result,
        "throw|2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824|true|undefined"
    );
}

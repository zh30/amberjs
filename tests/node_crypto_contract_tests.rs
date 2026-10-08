//! Pins docs/NODE_CRYPTO_CONTRACT.md. Node crypto subset, not Web Crypto G14.
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::process::Command;

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
fn require_crypto_exports_stable_subset_and_not_web_subtle_contract() {
    let result = run(
        r#"
        const c = require('crypto');
        const n = require('node:crypto');
        [
          typeof c.createHash,
          typeof c.createHmac,
          typeof c.randomBytes,
          typeof c.randomBytesSync,
          typeof c.randomUUID,
          typeof c.timingSafeEqual,
          c.createHash === n.createHash,
          c.createHash === crypto.createHash,
          c.randomUUID === crypto.randomUUID,
          typeof c.subtle,
          typeof crypto.subtle,
          // Web Crypto stays on the shared object; this contract does not pin it.
          typeof c.subtle?.digest === 'function'
        ].join('|');
        "#,
    );
    assert_eq!(
        result,
        "function|function|function|function|function|function|true|true|true|object|object|true"
    );
}

#[test]
#[serial]
fn create_hash_known_answers_encodings_copy_and_errors() {
    let result = run(
        r#"
        const c = require('crypto');
        const sha256 = c.createHash('sha256').update('hello').digest('hex');
        const sha384 = c.createHash('sha384').update('hello').digest('hex');
        const sha512 = c.createHash('sha512').update('hello').digest('hex');
        const md5 = c.createHash('md5').update('hello').digest('hex');
        const sha1 = c.createHash('sha1').update('abc').digest('hex');
        const alias = c.createHash('SHA-256').update('hello').digest('hex');
        const raw = c.createHash('md5').update('hello').digest();
        const b64 = c.createHash('sha1').update('abc').digest('base64');
        const b64u = c.createHash('sha1').update('abc').digest('base64url');
        const h = c.createHash('sha256');
        h.update('hel');
        const copied = h.copy();
        h.update('lo');
        const orig = h.digest('hex');
        const fromCopy = copied.update('lo').digest('hex');
        let unsupported = '';
        try { c.createHash('unsupported'); unsupported = 'ok'; }
        catch (e) { unsupported = String(e.message).includes('Unsupported hash algorithm') ? 'unsupported' : String(e.message); }
        let double = '';
        try {
          const x = c.createHash('md5');
          x.update('a');
          x.digest();
          x.digest();
          double = 'ok';
        } catch (e) { double = 'throw'; }
        const blake = c.createHash('blake3').update('x').digest('hex');
        [
          sha256,
          sha384,
          sha512,
          md5,
          sha1,
          alias === sha256,
          raw instanceof Uint8Array,
          raw.length,
          b64,
          b64u,
          orig === sha256,
          fromCopy === sha256,
          unsupported,
          double,
          blake.length
        ].join('|');
        "#,
    );
    assert_eq!(
        result,
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824|59e1748777448c69de6b800d7a33bbfb9ff1b463e44354c3553bcdb9c666fa90125a3c79f90397bdf5f6a13de828684f|9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca72323c3d99ba5c11d7c7acc6e14b8c5da0c4663475c2e5c3adef46f73bcdec043|5d41402abc4b2a76b9719d911017c592|a9993e364706816aba3e25717850c26c9cd0d89d|true|true|16|qZk+NkcGgWq6PiVxeFDCbJzQ2J0=|qZk-NkcGgWq6PiVxeFDCbJzQ2J0|true|true|unsupported|throw|64"
    );
}

#[test]
#[serial]
fn create_hmac_known_answers_and_digest_shapes() {
    let result = run(
        r#"
        const c = require('crypto');
        const hex = c.createHmac('sha256', 'key')
          .update('The quick brown fox jumps over the lazy dog')
          .digest('hex');
        const md5 = c.createHmac('md5', 'key')
          .update('The quick brown fox jumps over the lazy dog')
          .digest('hex');
        const raw = c.createHmac('sha256', 'key').update('msg').digest();
        const again = c.createHmac('sha256', Buffer.from('key')).update('msg').digest('hex');
        let unsupported = '';
        try { c.createHmac('nope', 'k'); unsupported = 'ok'; }
        catch (e) { unsupported = String(e.message).includes('Unsupported HMAC algorithm') ? 'unsupported' : String(e.message); }
        const h = c.createHmac('sha256', 'k');
        h.update('a');
        h.digest();
        const second = h.digest();
        const secondEnc = h.digest('hex');
        [
          hex,
          md5,
          raw instanceof Uint8Array,
          raw.length,
          again.slice(0, 16),
          unsupported,
          second instanceof Uint8Array,
          second.length,
          secondEnc,
          typeof h.copy
        ].join('|');
        "#,
    );
    assert_eq!(
        result,
        "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8|80070713463e7749b90c2dc24911e275|true|32|2d93cbc1be167bcb|unsupported|true|0||undefined"
    );
}

#[test]
#[serial]
fn random_bytes_size_callback_and_range_errors() {
    let result = run(
        r#"
        const c = require('crypto');
        const sync = c.randomBytes(16);
        const zero = c.randomBytes(0);
        const syncNamed = c.randomBytesSync(8);
        let order = [];
        order.push('before');
        const ret = c.randomBytes(4, (err, buf) => {
          order.push(!err && buf && buf.length === 4 ? 'cb' : 'bad');
        });
        order.push(ret instanceof Uint8Array && ret.length === 4 ? 'ret' : 'noret');
        let neg = '';
        try { c.randomBytes(-1); neg = 'ok'; }
        catch (e) { neg = e instanceof RangeError ? 'range' : e.name; }
        let huge = '';
        try { c.randomBytes(2147483648); huge = 'ok'; }
        catch (e) { huge = e instanceof RangeError ? 'range' : e.name; }
        [
          sync instanceof Uint8Array,
          sync.length,
          sync.constructor.name,
          zero.length,
          syncNamed.length,
          order.join(','),
          neg,
          huge
        ].join('|');
        "#,
    );
    assert_eq!(
        result,
        "true|16|Uint8Array|0|8|before,cb,ret|range|range"
    );
}

#[test]
#[serial]
fn random_uuid_and_timing_safe_equal() {
    let result = run(
        r#"
        const c = require('crypto');
        const uuid = c.randomUUID();
        const again = c.randomUUID();
        const a = Buffer.from('abcdef123456');
        const b = Buffer.from('abcdef123456');
        const d = Buffer.from('abcdef654321');
        let lenErr = '';
        try { c.timingSafeEqual(a, Buffer.from('short')); lenErr = 'ok'; }
        catch (e) { lenErr = String(e.message).toLowerCase().includes('length') ? 'length' : String(e.message); }
        [
          uuid.length,
          uuid[14],
          '89ab'.includes(uuid[19]),
          uuid !== again,
          /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(uuid),
          c.timingSafeEqual(a, b),
          c.timingSafeEqual(a, d),
          lenErr,
          c.randomUUID === crypto.randomUUID
        ].join('|');
        "#,
    );
    assert_eq!(
        result,
        "36|4|true|true|true|true|false|length|true"
    );
}

#[test]
#[serial]
fn seed_makes_random_bytes_reproducible_but_not_uuid() {
    let amber = env!("CARGO_BIN_EXE_amber");
    let script = std::env::temp_dir().join("amber_node_crypto_seed_contract.js");
    std::fs::write(
        &script,
        r#"
        const c = require('crypto');
        const bytes = Array.from(c.randomBytes(8)).join(',');
        const uuid = c.randomUUID();
        console.log(bytes + '|' + uuid);
        "#,
    )
    .expect("write script");

    let run_seeded = || {
        let output = Command::new(amber)
            .args(["run", "--seed", "42"])
            .arg(&script)
            .output()
            .expect("run amber");
        assert!(
            output.status.success(),
            "amber failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };

    let first = run_seeded();
    let second = run_seeded();
    let (bytes1, uuid1) = first.split_once('|').expect("first line");
    let (bytes2, uuid2) = second.split_once('|').expect("second line");
    assert_eq!(bytes1, bytes2, "randomBytes should follow --seed");
    assert_ne!(uuid1, uuid2, "randomUUID must not follow --seed");
    let _ = std::fs::remove_file(&script);
}

//! Pins docs/WEB_CRYPTO_CONTRACT.md.
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn run(code: &str) -> String {
    MinimalRuntime::new()
        .expect("runtime")
        .execute_code(code)
        .unwrap_or_else(|err| panic!("execution failed: {err}"))
        .trim()
        .to_string()
}

#[test]
#[serial]
fn random_uuid_and_get_random_values() {
    let uuid = run(r#"
        const uuid = crypto.randomUUID();
        const variant = uuid[19];
        const again = crypto.randomUUID();
        [
          uuid.length,
          uuid[14],
          '89ab'.includes(variant),
          uuid !== again,
          /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(uuid)
        ].join('|');
        "#);
    assert_eq!(uuid, "36|4|true|true|true");

    let random = run(r#"
        const buffer = new ArrayBuffer(8);
        const all = new Uint8Array(buffer);
        all.fill(7);
        const view = new Uint8Array(buffer, 2, 3);
        const output = crypto.getRandomValues(view);
        let quota = '';
        try { crypto.getRandomValues(new Uint8Array(65537)); quota = 'ok'; }
        catch (e) { quota = e.name; }
        let floatRejected = '';
        try { crypto.getRandomValues(new Float64Array(2)); floatRejected = 'ok'; }
        catch (e) { floatRejected = e instanceof TypeError; }
        [
          output === view,
          all[0] === 7 && all[1] === 7 && all[5] === 7,
          quota,
          floatRejected
        ].join('|');
        "#);
    assert_eq!(random, "true|true|RangeError|true");
}

#[test]
#[serial]
fn digest_known_answers_and_unknown_algorithm() {
    let digests = run(r#"
        const hex = (bytes) => Array.from(bytes).map((b) => b.toString(16).padStart(2, '0')).join('');
        const data = new TextEncoder().encode('abc');
        (async () => {
            const sha1 = await crypto.subtle.digest('SHA-1', data);
            const sha256 = await crypto.subtle.digest({ name: 'SHA-256' }, data);
            const sha384 = await crypto.subtle.digest('sha-384', data);
            const sha512 = await crypto.subtle.digest('SHA512', data);
            let unknown = '';
            try { await crypto.subtle.digest('MD5', data); unknown = 'resolved'; }
            catch (e) { unknown = String(e && e.message || e); }
            return [
              sha1 instanceof Uint8Array,
              sha1.byteLength + ',' + sha256.byteLength + ',' + sha384.byteLength + ',' + sha512.byteLength,
              hex(sha1),
              hex(sha256),
              hex(sha384),
              hex(sha512),
              unknown
            ].join('|');
        })();
        "#);
    assert_eq!(
        digests,
        "true|20,32,48,64|a9993e364706816aba3e25717850c26c9cd0d89d|ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad|cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7|ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f|Unsupported hash algorithm: MD5"
    );
}

#[test]
#[serial]
fn hmac_known_answers_import_export_and_key_shape() {
    let result = run(r#"
        const hex = (bytes) => Array.from(new Uint8Array(bytes)).map((b) => b.toString(16).padStart(2, '0')).join('');
        const keyBytes = new Uint8Array(20).fill(0x0b);
        const data = new TextEncoder().encode('Hi There');
        (async () => {
            const cases = [
              ['SHA-1', 'HS1', 'b617318655057264e28bc0b6fb378c8ef146be00'],
              ['SHA-256', 'HS256', 'b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7'],
              ['SHA-384', 'HS384', 'afd03944d84895626b0825f4ab46907f15f9dadbe4101ec682aa034c7cebc59cfaea9ea9076ede7f4af152e8b2fa9cb6'],
              ['SHA-512', 'HS512', '87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854']
            ];
            const parts = [];
            for (const [hash, alg, expected] of cases) {
              const key = await crypto.subtle.importKey('raw', keyBytes, { name: 'HMAC', hash }, true, ['sign', 'verify']);
              const signature = await crypto.subtle.sign({ name: 'HMAC' }, key, data);
              const ok = await crypto.subtle.verify({ name: 'HMAC' }, key, signature, data);
              const bad = await crypto.subtle.verify({ name: 'HMAC' }, key, signature, new TextEncoder().encode('Hi There!'));
              const jwk = await crypto.subtle.exportKey('jwk', key);
              parts.push([
                key.type === 'secret' && key.extractable === true && key.algorithm.name === 'HMAC' && key.algorithm.hash.name === hash && key.usages.join(',') === 'sign,verify',
                signature instanceof ArrayBuffer && hex(signature) === expected,
                ok === true && bad === false,
                jwk.kty === 'oct' && jwk.alg === alg && jwk.ext === true
              ].join(':'));
            }
            const generated = await crypto.subtle.generateKey({ name: 'hmac', hash: 'SHA-256' }, true, ['sign']);
            const raw = new Uint8Array(await crypto.subtle.exportKey('raw', generated));
            const generatedJwk = await crypto.subtle.exportKey('jwk', generated);
            let locked = '';
            try {
              const hidden = await crypto.subtle.importKey('raw', keyBytes, { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']);
              await crypto.subtle.exportKey('raw', hidden);
              locked = 'exported';
            } catch (e) { locked = String(e && e.message || e); }
            return parts.concat([raw.byteLength, generatedJwk.alg, locked]).join('|');
        })();
        "#);
    assert_eq!(
        result,
        "true:true:true:true|true:true:true:true|true:true:true:true|true:true:true:true|64|HS256|exportKey: key is not extractable"
    );
}

#[test]
#[serial]
fn aes_gcm_cbc_ctr_known_answers() {
    let result = run(r#"
        const hex = (bytes) => Array.from(new Uint8Array(bytes)).map((b) => b.toString(16).padStart(2, '0')).join('');
        const bytes = (n) => Uint8Array.from({ length: n }, (_, i) => i);
        const text = new TextEncoder().encode('hello webcrypto');
        (async () => {
            const gcm128 = await crypto.subtle.importKey('raw', bytes(16), { name: 'aes-gcm' }, true, ['encrypt', 'decrypt']);
            const gcm192 = await crypto.subtle.importKey('raw', bytes(24), { name: 'AES-GCM' }, true, ['encrypt', 'decrypt']);
            const gcm256 = await crypto.subtle.importKey('raw', bytes(32), { name: 'AES-GCM' }, true, ['encrypt', 'decrypt']);
            const aad = new TextEncoder().encode('aad');
            const iv12 = bytes(12);
            const ct128 = await crypto.subtle.encrypt({ name: 'AES-GCM', iv: iv12, additionalData: aad }, gcm128, text);
            const ct192 = await crypto.subtle.encrypt({ name: 'AES-GCM', iv: iv12, additionalData: aad, tagLength: 128 }, gcm192, text);
            const ct256 = await crypto.subtle.encrypt({ name: 'AES-GCM', iv: iv12 }, gcm256, text);
            const opened = new TextDecoder().decode(await crypto.subtle.decrypt({ name: 'AES-GCM', iv: iv12, additionalData: aad }, gcm192, ct192));
            let tamper = '';
            try {
              const bad = Uint8Array.from(new Uint8Array(ct128));
              bad[bad.length - 1] ^= 1;
              await crypto.subtle.decrypt({ name: 'AES-GCM', iv: iv12, additionalData: aad }, gcm128, bad);
              tamper = 'opened';
            } catch (e) { tamper = 'closed'; }
            let tag = '';
            try {
              await crypto.subtle.encrypt({ name: 'AES-GCM', iv: iv12, tagLength: 96 }, gcm128, text);
              tag = 'ok';
            } catch (e) { tag = String(e && e.message || e); }
            const jwk = await crypto.subtle.exportKey('jwk', gcm256);
            const cbcKey = await crypto.subtle.importKey('raw', bytes(16), { name: 'AES-CBC' }, false, ['encrypt', 'decrypt']);
            const cbc = await crypto.subtle.encrypt({ name: 'AES-CBC', iv: bytes(16) }, cbcKey, new TextEncoder().encode('hello'));
            const ctrKey = await crypto.subtle.importKey('raw', bytes(16), { name: 'AES-CTR' }, false, ['encrypt', 'decrypt']);
            const ctr = await crypto.subtle.encrypt({ name: 'AES-CTR', counter: bytes(16), length: 128 }, ctrKey, text);
            const ctrPlain = new TextDecoder().decode(await crypto.subtle.decrypt({ name: 'AES-CTR', counter: bytes(16), length: 128 }, ctrKey, ctr));
            return [
              ct128 instanceof ArrayBuffer,
              hex(ct128),
              hex(ct192),
              hex(ct256),
              opened,
              tamper,
              tag,
              jwk.kty + ':' + jwk.alg,
              hex(cbc),
              hex(ctr),
              ctrPlain
            ].join('|');
        })();
        "#);
    assert_eq!(
        result,
        "true|fb09cba2093b803129b113f346d71f59b79f4d532710bc643f0371142b373b|8e9c4ef7f699be6bb250aefb8fddb3fb63348d5da6b204a2ca92c73d8dc446|2f67ba77aac5b57eef22e5f2c19d1713300e20c10ccb16cc1ccba8157e189d|hello webcrypto|closed|encrypt: AES-GCM tagLength must be 128|oct:A256GCM|1dfe836df70e89310a970a31fa3351fd|62f167d92e4e872093a0e621b62785|hello webcrypto"
    );
}

#[test]
#[serial]
fn aes_kw_wrap_and_pbkdf2_derive_bits() {
    let result = run(r#"
        const hex = (bytes) => Array.from(new Uint8Array(bytes)).map((b) => b.toString(16).padStart(2, '0')).join('');
        const bytes = (n) => Uint8Array.from({ length: n }, (_, i) => i);
        (async () => {
            const kek = await crypto.subtle.importKey('raw', bytes(16), { name: 'AES-KW' }, true, ['wrapKey', 'unwrapKey']);
            const key = await crypto.subtle.importKey('raw', bytes(32), { name: 'AES-GCM' }, true, ['encrypt']);
            const wrapped = await crypto.subtle.wrapKey('raw', key, kek, { name: 'AES-KW' });
            const unwrapped = await crypto.subtle.unwrapKey('raw', wrapped, kek, { name: 'AES-KW' }, { name: 'AES-GCM' }, true, ['encrypt']);
            const round = hex(await crypto.subtle.exportKey('raw', unwrapped));
            let encryptKw = '';
            try { await crypto.subtle.encrypt({ name: 'AES-KW' }, kek, bytes(16)); encryptKw = 'ok'; }
            catch (e) { encryptKw = String(e && e.message || e).includes('not implemented'); }
            const base = await crypto.subtle.importKey('raw', new TextEncoder().encode('password'), { name: 'PBKDF2' }, false, ['deriveBits', 'deriveKey']);
            const bits = await crypto.subtle.deriveBits({ name: 'PBKDF2', salt: new TextEncoder().encode('salt'), iterations: 1, hash: 'SHA-256' }, base, 256);
            const derived = await crypto.subtle.deriveKey(
              { name: 'PBKDF2', salt: new TextEncoder().encode('salt'), iterations: 1, hash: 'SHA-256' },
              base,
              { name: 'AES-GCM', length: 256 },
              true,
              ['encrypt']
            );
            let hkdf = '';
            try { await crypto.subtle.generateKey({ name: 'HKDF' }, false, ['deriveBits']); hkdf = 'ok'; }
            catch (e) { hkdf = String(e && e.message || e).includes('HKDF'); }
            return [wrapped.byteLength, hex(wrapped), round, encryptKw, bits instanceof ArrayBuffer, hex(bits), derived.algorithm.name, derived.algorithm.length, hkdf].join('|');
        })();
        "#);
    assert_eq!(
        result,
        "40|0e7808f506f2c3e7aa6edad793ac4495b093eb482e5c7ca9c170c9faa07dc0cbbb87512e19fd4092|000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f|true|true|120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b|AES-GCM|256|true"
    );
}

#[test]
#[serial]
fn eddsa_sign_and_public_export() {
    let result = run(r#"
        (async () => {
            const out = [];
            for (const [name, rawLen, sigLen] of [['Ed25519', 32, 64], ['Ed448', 57, 114]]) {
              const pair = await crypto.subtle.generateKey({ name }, true, ['sign', 'verify']);
              const data = new TextEncoder().encode(name);
              const signature = await crypto.subtle.sign({ name }, pair.privateKey, data);
              const ok = await crypto.subtle.verify({ name }, pair.publicKey, signature, data);
              const bad = await crypto.subtle.verify({ name }, pair.publicKey, signature, new TextEncoder().encode(name + 'x'));
              const raw = new Uint8Array(await crypto.subtle.exportKey('raw', pair.publicKey));
              const jwk = await crypto.subtle.exportKey('jwk', pair.publicKey);
              let privateRaw = '';
              try { await crypto.subtle.exportKey('raw', pair.privateKey); privateRaw = 'exported'; }
              catch (e) { privateRaw = String(e && e.message || e).includes('raw'); }
              out.push([
                pair.privateKey.type === 'private' && pair.publicKey.algorithm.name === name,
                signature.byteLength === sigLen && ok === true && bad === false,
                raw.length === rawLen && jwk.kty === 'OKP' && jwk.crv === name,
                privateRaw
              ].join(':'));
            }
            return out.join('|');
        })();
        "#);
    assert_eq!(result, "true:true:true:true|true:true:true:true");
}

#[test]
#[serial]
fn ecdsa_ecdh_and_rsa_round_trip() {
    let result = run(r#"
        (async () => {
            const curves = [];
            for (const [curve, pointLen, sigLen] of [['P-256', 65, 64], ['P-384', 97, 96], ['P-521', 133, 132]]) {
              const pair = await crypto.subtle.generateKey({ name: 'ECDSA', namedCurve: curve }, true, ['sign', 'verify']);
              const data = new TextEncoder().encode(curve);
              const signature = await crypto.subtle.sign({ name: 'ECDSA', hash: 'SHA-256' }, pair.privateKey, data);
              const ok = await crypto.subtle.verify({ name: 'ECDSA', hash: 'SHA-256' }, pair.publicKey, signature, data);
              const point = new Uint8Array(await crypto.subtle.exportKey('raw', pair.publicKey));
              let jwk = '';
              try { await crypto.subtle.exportKey('jwk', pair.publicKey); jwk = 'exported'; }
              catch (e) { jwk = String(e && e.message || e).includes('JWK'); }
              curves.push([signature.byteLength === sigLen && ok === true, point.length === pointLen && point[0] === 4, jwk].join(':'));
            }
            const alice = await crypto.subtle.generateKey({ name: 'ECDH', namedCurve: 'P-256' }, true, ['deriveBits']);
            const bob = await crypto.subtle.generateKey({ name: 'ECDH', namedCurve: 'P-256' }, true, ['deriveBits']);
            const ab = new Uint8Array(await crypto.subtle.deriveBits({ name: 'ECDH', public: bob.publicKey }, alice.privateKey, 256));
            const ba = new Uint8Array(await crypto.subtle.deriveBits({ name: 'ECDH', public: alice.publicKey }, bob.privateKey, 256));
            const shared = ab.length === 32 && ab.every((byte, i) => byte === ba[i]) && ab.some((byte) => byte !== 0);
            const exponent = new Uint8Array([1, 0, 1]);
            const rsa = await crypto.subtle.generateKey({ name: 'RSA-OAEP', modulusLength: 2048, publicExponent: exponent, hash: 'SHA-256' }, true, ['encrypt', 'decrypt']);
            const secret = new TextEncoder().encode('rsa-oaep');
            const ciphertext = await crypto.subtle.encrypt({ name: 'RSA-OAEP' }, rsa.publicKey, secret);
            const opened = new TextDecoder().decode(await crypto.subtle.decrypt({ name: 'RSA-OAEP' }, rsa.privateKey, ciphertext));
            let rsaExport = '';
            try { await crypto.subtle.exportKey('raw', rsa.publicKey); rsaExport = 'exported'; }
            catch (e) { rsaExport = String(e && e.message || e).includes('not supported'); }
            const rsassa = await crypto.subtle.generateKey({ name: 'RSASSA-PKCS1-v1_5', modulusLength: 2048, publicExponent: exponent, hash: 'SHA-256' }, true, ['sign', 'verify']);
            const message = new TextEncoder().encode('rsassa');
            const signature = await crypto.subtle.sign({ name: 'RSASSA-PKCS1-v1_5' }, rsassa.privateKey, message);
            const ok = await crypto.subtle.verify({ name: 'RSASSA-PKCS1-v1_5' }, rsassa.publicKey, signature, message);
            const bad = await crypto.subtle.verify({ name: 'RSASSA-PKCS1-v1_5' }, rsassa.publicKey, signature, new TextEncoder().encode('rsassa!'));
            return curves.concat([shared, opened === 'rsa-oaep', ciphertext.byteLength === 256, rsaExport, signature.byteLength === 256 && ok === true && bad === false]).join('|');
        })();
        "#);
    assert_eq!(
        result,
        "true:true:true|true:true:true|true:true:true|true|true|true|true|true"
    );
}

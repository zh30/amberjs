// Tests for crypto.subtle.importKey implementation (v0.3.358)
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_import_key_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.subtle.importKey");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_import_key_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.importKey(
            'raw',
            new Uint8Array([1, 2, 3, 4]),
            { name: 'HMAC', hash: 'SHA-256' },
            false,
            ['sign', 'verify']
        );
        result && result.constructor && result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_import_key_returns_object() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.importKey(
            'raw',
            new Uint8Array([1, 2, 3, 4]),
            { name: 'HMAC', hash: 'SHA-256' },
            false,
            ['sign', 'verify']
        );
        typeof result.then;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    // The result should be a Promise (object with 'then' method)
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_import_key_hmac_returns_crypto_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.importKey(
            'raw',
            new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
            { name: 'HMAC', hash: 'SHA-256' },
            false,
            ['sign', 'verify']
        );
        result.then !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_import_key_aes_gcm_returns_crypto_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.importKey(
            'raw',
            new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]),
            { name: 'AES-GCM', length: 256 },
            true,
            ['encrypt', 'decrypt']
        );
        result.then !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_import_key_sign_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const key = crypto.subtle.importKey(
            'raw',
            new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
            { name: 'HMAC', hash: 'SHA-256' },
            false,
            ['sign']
        );
        const result = key.then(k => crypto.subtle.sign({ name: 'HMAC' }, k, new TextEncoder().encode('message')));
        typeof result.then;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_import_key_verify_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const key = crypto.subtle.importKey(
            'raw',
            new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
            { name: 'HMAC', hash: 'SHA-256' },
            false,
            ['verify']
        );
        const result = key.then(k => crypto.subtle.verify({ name: 'HMAC' }, k, new Uint8Array(32), new TextEncoder().encode('message')));
        typeof result.then;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_import_key_encrypt_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const key = crypto.subtle.importKey(
            'raw',
            new Uint8Array(32).fill(1),
            { name: 'AES-GCM', length: 256 },
            true,
            ['encrypt']
        );
        const iv = crypto.getRandomValues(new Uint8Array(12));
        const result = key.then(k => crypto.subtle.encrypt({ name: 'AES-GCM', iv: iv }, k, new TextEncoder().encode('message')));
        typeof result.then;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_import_key_decrypt_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const key = crypto.subtle.importKey(
            'raw',
            new Uint8Array(32).fill(1),
            { name: 'AES-GCM', length: 256 },
            true,
            ['decrypt']
        );
        const iv = crypto.getRandomValues(new Uint8Array(12));
        const result = key.then(k => crypto.subtle.decrypt({ name: 'AES-GCM', iv: iv }, k, new ArrayBuffer(32)));
        typeof result.then;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_import_key_supports_raw_format() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.importKey(
            'raw',
            new Uint8Array(16),
            { name: 'HMAC', hash: 'SHA-256' },
            false,
            ['sign']
        );
        result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_import_key_jwk_hmac_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const key = await crypto.subtle.importKey(
                'jwk',
                {
                    kty: 'oct',
                    k: 'AQIDBAUGBwgJCgsMDQ4PEA',
                    alg: 'HS256',
                    key_ops: ['sign', 'verify'],
                    ext: true
                },
                { name: 'HMAC', hash: 'SHA-256' },
                true,
                ['sign', 'verify']
            );
            const data = new TextEncoder().encode('jwk hmac import');
            const signature = await crypto.subtle.sign({ name: 'HMAC' }, key, data);
            const valid = await crypto.subtle.verify({ name: 'HMAC' }, key, signature, data);
            const raw = new Uint8Array(await crypto.subtle.exportKey('raw', key));
            return valid &&
                raw.length === 16 &&
                raw[0] === 1 &&
                raw[15] === 16 &&
                key.algorithm.name === 'HMAC' &&
                key.usages.join(',') === 'sign,verify';
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_import_key_jwk_aes_gcm_rejects_alg_mismatch() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                await crypto.subtle.importKey(
                    'jwk',
                    {
                        kty: 'oct',
                        k: 'A'.repeat(43),
                        alg: 'A128GCM',
                        key_ops: ['encrypt'],
                        ext: true
                    },
                    { name: 'AES-GCM', length: 256 },
                    true,
                    ['encrypt']
                );
                return 'accepted';
            } catch (error) {
                const message = String(error && error.message ? error.message : error);
                return message.includes('alg') ? 'rejected' : message;
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "rejected");
}

#[test]
#[serial]
fn test_import_key_jwk_rejects_key_ops_missing_requested_usage() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                await crypto.subtle.importKey(
                    'jwk',
                    {
                        kty: 'oct',
                        k: 'A'.repeat(43),
                        alg: 'A256GCM',
                        key_ops: ['decrypt'],
                        ext: true
                    },
                    { name: 'AES-GCM', length: 256 },
                    true,
                    ['encrypt']
                );
                return 'accepted';
            } catch (error) {
                const message = String(error && error.message ? error.message : error);
                return message.includes('key_ops') ? 'rejected' : message;
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "rejected");
}

#[test]
#[serial]
fn test_import_key_jwk_rejects_ext_false_when_extractable_true() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                await crypto.subtle.importKey(
                    'jwk',
                    {
                        kty: 'oct',
                        k: 'A'.repeat(43),
                        alg: 'A256GCM',
                        key_ops: ['encrypt'],
                        ext: false
                    },
                    { name: 'AES-GCM', length: 256 },
                    true,
                    ['encrypt']
                );
                return 'accepted';
            } catch (error) {
                const message = String(error && error.message ? error.message : error);
                return message.includes('ext') || message.includes('extractable') ? 'rejected' : message;
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "rejected");
}

#[test]
#[serial]
fn test_import_key_supports_hmac_algorithm() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.importKey(
            'raw',
            new Uint8Array(32),
            { name: 'HMAC', hash: 'SHA-256' },
            false,
            ['sign', 'verify']
        );
        result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_import_key_supports_aes_gcm_algorithm() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.importKey(
            'raw',
            new Uint8Array(32),
            { name: 'AES-GCM', length: 256 },
            true,
            ['encrypt', 'decrypt']
        );
        result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_import_key_supports_aes_cbc_algorithm() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.importKey(
            'raw',
            new Uint8Array(32),
            { name: 'AES-CBC', length: 256 },
            true,
            ['encrypt', 'decrypt']
        );
        result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

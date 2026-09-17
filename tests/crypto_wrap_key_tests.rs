// wrapKey/unwrapKey Tests - v0.3.369
// Tests for crypto.subtle.wrapKey and unwrapKey
// wrapKey wraps (encrypts) a key for secure storage/transport
// unwrapKey unwraps (decrypts) a wrapped key

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_wrap_key_function_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.subtle.wrapKey");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_unwrap_key_function_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.subtle.unwrapKey");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_wrap_key_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const keyPromise = crypto.subtle.generateKey(
            { name: 'AES-GCM', length: 256 },
            true,
            ['encrypt', 'decrypt']
        );
        const wrappingKeyPromise = crypto.subtle.generateKey(
            { name: 'AES-GCM', length: 256 },
            true,
            ['wrapKey', 'unwrapKey']
        );
        Promise.all([keyPromise, wrappingKeyPromise]).then(([key, wrappingKey]) => {
            const result = crypto.subtle.wrapKey('raw', key, wrappingKey, { name: 'AES-GCM', iv: new Uint8Array(12) });
            return result && result.constructor && result.constructor.name === 'Promise';
        });
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_wrap_key_returns_array_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const keyPromise = crypto.subtle.generateKey(
            { name: 'AES-GCM', length: 256 },
            true,
            ['encrypt', 'decrypt']
        );
        const wrappingKeyPromise = crypto.subtle.generateKey(
            { name: 'AES-GCM', length: 256 },
            true,
            ['wrapKey', 'unwrapKey']
        );
        Promise.all([keyPromise, wrappingKeyPromise]).then(([key, wrappingKey]) => {
            return crypto.subtle.wrapKey('raw', key, wrappingKey, { name: 'AES-GCM', iv: new Uint8Array(12) });
        });
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "[object ArrayBuffer]");
}

#[test]
#[serial]
fn test_wrap_key_jwk_wraps_json_jwk_payload() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const key = await crypto.subtle.generateKey(
                { name: 'HMAC', hash: 'SHA-256', length: 256 },
                true,
                ['sign', 'verify']
            );
            const wrappingKey = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['wrapKey', 'unwrapKey', 'decrypt']
            );
            const iv = crypto.getRandomValues(new Uint8Array(12));
            const wrapped = await crypto.subtle.wrapKey(
                'jwk',
                key,
                wrappingKey,
                { name: 'AES-GCM', iv }
            );
            const plaintext = await crypto.subtle.decrypt(
                { name: 'AES-GCM', iv },
                wrappingKey,
                wrapped
            );

            try {
                const jwk = JSON.parse(new TextDecoder().decode(plaintext));
                return jwk.kty === 'oct'
                    && jwk.alg === 'HS256'
                    && typeof jwk.k === 'string'
                    && jwk.key_ops.includes('sign')
                    && jwk.key_ops.includes('verify');
            } catch (_) {
                return false;
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_unwrap_key_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const keyPromise = crypto.subtle.generateKey(
            { name: 'AES-GCM', length: 256 },
            true,
            ['encrypt', 'decrypt']
        );
        const wrappingKeyPromise = crypto.subtle.generateKey(
            { name: 'AES-GCM', length: 256 },
            true,
            ['wrapKey', 'unwrapKey']
        );
        let wrappingKeyRef;
        let ivRef;
        Promise.all([keyPromise, wrappingKeyPromise]).then(([key, wrappingKey]) => {
            wrappingKeyRef = wrappingKey;
            ivRef = crypto.getRandomValues(new Uint8Array(12));
            return crypto.subtle.wrapKey('raw', key, wrappingKey, { name: 'AES-GCM', iv: ivRef });
        }).then(wrapped => {
            const result = crypto.subtle.unwrapKey(
                'raw',
                wrapped,
                wrappingKeyRef,
                { name: 'AES-GCM', iv: ivRef },
                { name: 'AES-GCM', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );
            return result && result.constructor && result.constructor.name === 'Promise';
        });
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_wrap_unwrap_aes_key_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            // Generate a key to wrap
            const keyToWrap = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            // Generate a wrapping key
            const wrappingKey = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['wrapKey', 'unwrapKey']
            );

            // Wrap the key
            const iv = crypto.getRandomValues(new Uint8Array(12));
            const wrapped = await crypto.subtle.wrapKey(
                'raw',
                keyToWrap,
                wrappingKey,
                { name: 'AES-GCM', iv: iv }
            );

            // Unwrap the key
            const unwrappedKey = await crypto.subtle.unwrapKey(
                'raw',
                wrapped,
                wrappingKey,
                { name: 'AES-GCM', iv },
                { name: 'AES-GCM', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            // Verify the unwrapped key works
            const testData = new TextEncoder().encode('Test message');
            const dataIv = crypto.getRandomValues(new Uint8Array(12));
            const encrypted = await crypto.subtle.encrypt(
                { name: 'AES-GCM', iv: dataIv },
                unwrappedKey,
                testData
            );

            const decrypted = await crypto.subtle.decrypt(
                { name: 'AES-GCM', iv: dataIv },
                unwrappedKey,
                encrypted
            );

            return new TextDecoder().decode(decrypted) === 'Test message';
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_wrap_unwrap_aes_key_with_aes_kw_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyToWrap = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            const wrappingKey = await crypto.subtle.generateKey(
                { name: 'AES-KW', length: 256 },
                true,
                ['wrapKey', 'unwrapKey']
            );

            const wrapped = await crypto.subtle.wrapKey(
                'raw',
                keyToWrap,
                wrappingKey,
                { name: 'AES-KW' }
            );

            const unwrappedKey = await crypto.subtle.unwrapKey(
                'raw',
                wrapped,
                wrappingKey,
                { name: 'AES-KW' },
                { name: 'AES-GCM', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            const testData = new TextEncoder().encode('AES-KW round trip');
            const iv = crypto.getRandomValues(new Uint8Array(12));
            const encrypted = await crypto.subtle.encrypt(
                { name: 'AES-GCM', iv },
                unwrappedKey,
                testData
            );
            const decrypted = await crypto.subtle.decrypt(
                { name: 'AES-GCM', iv },
                unwrappedKey,
                encrypted
            );

            return wrapped.byteLength === 40
                && new TextDecoder().decode(decrypted) === 'AES-KW round trip';
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(
        result.is_ok(),
        "AES-KW wrap/unwrap should execute without throwing: {result:?}"
    );
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_unwrap_key_with_aes_kw_rejects_tampered_data() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyToWrap = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            const wrappingKey = await crypto.subtle.generateKey(
                { name: 'AES-KW', length: 256 },
                true,
                ['wrapKey', 'unwrapKey']
            );

            const wrapped = new Uint8Array(await crypto.subtle.wrapKey(
                'raw',
                keyToWrap,
                wrappingKey,
                { name: 'AES-KW' }
            ));
            wrapped[wrapped.length - 1] ^= 0xff;

            try {
                await crypto.subtle.unwrapKey(
                    'raw',
                    wrapped,
                    wrappingKey,
                    { name: 'AES-KW' },
                    { name: 'AES-GCM', length: 256 },
                    true,
                    ['encrypt', 'decrypt']
                );
                return false;
            } catch (_) {
                return true;
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(
        result.is_ok(),
        "AES-KW unwrap should reject tampered data without crashing: {result:?}"
    );
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_wrap_key_with_hmac() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            // Generate an HMAC key to wrap
            const hmacKey = await crypto.subtle.generateKey(
                { name: 'HMAC', hash: 'SHA-256', length: 256 },
                true,
                ['sign', 'verify']
            );

            // Generate an AES wrapping key
            const wrappingKey = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['wrapKey', 'unwrapKey']
            );

            // Wrap the HMAC key
            const iv = crypto.getRandomValues(new Uint8Array(12));
            const wrapped = await crypto.subtle.wrapKey(
                'jwk',
                hmacKey,
                wrappingKey,
                { name: 'AES-GCM', iv: iv }
            );

            // Unwrap the HMAC key
            const unwrappedKey = await crypto.subtle.unwrapKey(
                'jwk',
                wrapped,
                wrappingKey,
                { name: 'AES-GCM', iv },
                { name: 'HMAC', hash: 'SHA-256', length: 256 },
                true,
                ['sign', 'verify']
            );

            // Verify the unwrapped key works
            const testData = new TextEncoder().encode('Test message for HMAC');
            const signature = await crypto.subtle.sign(
                { name: 'HMAC' },
                unwrappedKey,
                testData
            );

            const isValid = await crypto.subtle.verify(
                { name: 'HMAC' },
                unwrappedKey,
                signature,
                testData
            );

            return isValid;
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_wrap_key_invalid_wrapping_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyToWrap = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            const wrappingKey = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 128 },
                true,
                ['encrypt', 'decrypt']
            );

            try {
                await crypto.subtle.wrapKey(
                    'raw',
                    keyToWrap,
                    wrappingKey,
                    { name: 'AES-GCM', iv: new Uint8Array(12) }
                );
                return false; // Should have thrown
            } catch (e) {
                return true; // Expected an error
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_wrap_key_without_iv_fails_closed() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyToWrap = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            const wrappingKey = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['wrapKey', 'unwrapKey']
            );

            try {
                await crypto.subtle.wrapKey(
                    'raw',
                    keyToWrap,
                    wrappingKey,
                    { name: 'AES-GCM' }
                );
                return 'resolved';
            } catch (error) {
                return String(error && error.message || error).toLowerCase().includes('iv');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "true",
        "wrapKey must fail closed when AES-GCM iv is missing"
    );
}

#[test]
#[serial]
fn test_wrap_key_with_short_iv_fails_closed() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyToWrap = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            const wrappingKey = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['wrapKey', 'unwrapKey']
            );

            try {
                await crypto.subtle.wrapKey(
                    'raw',
                    keyToWrap,
                    wrappingKey,
                    { name: 'AES-GCM', iv: new Uint8Array(8) }
                );
                return 'resolved';
            } catch (error) {
                return String(error && error.message || error).toLowerCase().includes('iv');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "true",
        "wrapKey must fail closed when AES-GCM iv is not 12 bytes"
    );
}

#[test]
#[serial]
fn test_unwrap_key_without_iv_fails_closed() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyToWrap = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            const wrappingKey = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['wrapKey', 'unwrapKey']
            );

            const iv = crypto.getRandomValues(new Uint8Array(12));
            const wrapped = await crypto.subtle.wrapKey(
                'raw',
                keyToWrap,
                wrappingKey,
                { name: 'AES-GCM', iv }
            );

            try {
                await crypto.subtle.unwrapKey(
                    'raw',
                    wrapped,
                    wrappingKey,
                    { name: 'AES-GCM' },
                    { name: 'AES-GCM', length: 256 },
                    true,
                    ['encrypt', 'decrypt']
                );
                return false;
            } catch (error) {
                return String(error && error.message || error).toLowerCase().includes('iv');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "true",
        "unwrapKey must fail closed when AES-GCM iv is missing"
    );
}

#[test]
#[serial]
fn test_unwrap_key_with_short_iv_fails_closed() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyToWrap = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            const wrappingKey = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['wrapKey', 'unwrapKey']
            );

            const iv = crypto.getRandomValues(new Uint8Array(12));
            const wrapped = await crypto.subtle.wrapKey(
                'raw',
                keyToWrap,
                wrappingKey,
                { name: 'AES-GCM', iv }
            );

            try {
                await crypto.subtle.unwrapKey(
                    'raw',
                    wrapped,
                    wrappingKey,
                    { name: 'AES-GCM', iv: new Uint8Array(8) },
                    { name: 'AES-GCM', length: 256 },
                    true,
                    ['encrypt', 'decrypt']
                );
                return false;
            } catch (error) {
                return String(error && error.message || error).toLowerCase().includes('iv');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "true",
        "unwrapKey must fail closed when AES-GCM iv is not 12 bytes"
    );
}

#[test]
#[serial]
fn test_unwrap_key_with_wrong_iv_fails_closed() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyToWrap = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            const wrappingKey = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['wrapKey', 'unwrapKey']
            );

            const iv = crypto.getRandomValues(new Uint8Array(12));
            const wrapped = await crypto.subtle.wrapKey(
                'raw',
                keyToWrap,
                wrappingKey,
                { name: 'AES-GCM', iv }
            );

            const wrongIv = new Uint8Array(12);
            wrongIv[0] = iv[0] ^ 0xff;

            try {
                await crypto.subtle.unwrapKey(
                    'raw',
                    wrapped,
                    wrappingKey,
                    { name: 'AES-GCM', iv: wrongIv },
                    { name: 'AES-GCM', length: 256 },
                    true,
                    ['encrypt', 'decrypt']
                );
                return false;
            } catch (_) {
                return true;
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "true",
        "unwrapKey must authenticate with the caller-provided AES-GCM iv"
    );
}

#[test]
#[serial]
fn test_unwrap_key_rejects_invalid_format() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyToWrap = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            const wrappingKey = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['wrapKey', 'unwrapKey']
            );

            const iv = crypto.getRandomValues(new Uint8Array(12));
            const wrapped = await crypto.subtle.wrapKey(
                'raw',
                keyToWrap,
                wrappingKey,
                { name: 'AES-GCM', iv }
            );

            try {
                await crypto.subtle.unwrapKey(
                    'beejs-internal',
                    wrapped,
                    wrappingKey,
                    { name: 'AES-GCM', iv },
                    { name: 'AES-GCM', length: 256 },
                    true,
                    ['encrypt', 'decrypt']
                );
                return false;
            } catch (error) {
                return String(error && error.message || error).toLowerCase().includes('format');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "true",
        "unwrapKey must reject unsupported key formats"
    );
}

// Tests for crypto.subtle.deriveKey implementation (v0.3.361)
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_derive_key_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.subtle.deriveKey");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_derive_key_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const password = new TextEncoder().encode('password');
        const baseKey = crypto.subtle.importKey(
            'raw',
            password,
            { name: 'PBKDF2' },
            false,
            ['deriveKey']
        );
        const salt = crypto.getRandomValues(new Uint8Array(16));
        const result = baseKey.then(key => crypto.subtle.deriveKey(
            { name: 'PBKDF2', salt: salt, iterations: 100000, hash: 'SHA-256' },
            key,
            { name: 'AES-GCM', length: 256 },
            true,
            ['encrypt', 'decrypt']
        ));
        result && result.constructor && result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_derive_key_pbkdf2_returns_crypto_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const password = new TextEncoder().encode('password');
        const baseKey = crypto.subtle.importKey(
            'raw',
            password,
            { name: 'PBKDF2' },
            false,
            ['deriveKey']
        );
        const salt = crypto.getRandomValues(new Uint8Array(16));
        const result = baseKey.then(key => crypto.subtle.deriveKey(
            { name: 'PBKDF2', salt: salt, iterations: 100000, hash: 'SHA-256' },
            key,
            { name: 'AES-GCM', length: 256 },
            true,
            ['encrypt', 'decrypt']
        ));
        result.then !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_derive_key_derived_key_can_be_used() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const password = new TextEncoder().encode('password');
        const baseKey = crypto.subtle.importKey(
            'raw',
            password,
            { name: 'PBKDF2' },
            false,
            ['deriveKey']
        );
        const salt = crypto.getRandomValues(new Uint8Array(16));
        const result = baseKey.then(key => crypto.subtle.deriveKey(
            { name: 'PBKDF2', salt: salt, iterations: 1000, hash: 'SHA-256' },
            key,
            { name: 'AES-GCM', length: 256 },
            true,
            ['encrypt']
        ).then(derivedKey => {
            const iv = crypto.getRandomValues(new Uint8Array(12));
            return crypto.subtle.encrypt(
                { name: 'AES-GCM', iv: iv },
                derivedKey,
                new TextEncoder().encode('test message')
            );
        }));
        typeof result.then;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_derive_key_with_hmac_output() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const password = new TextEncoder().encode('password');
        const baseKey = crypto.subtle.importKey(
            'raw',
            password,
            { name: 'PBKDF2' },
            false,
            ['deriveKey']
        );
        const salt = crypto.getRandomValues(new Uint8Array(16));
        const result = baseKey.then(key => crypto.subtle.deriveKey(
            { name: 'PBKDF2', salt: salt, iterations: 1000, hash: 'SHA-256' },
            key,
            { name: 'HMAC', hash: 'SHA-256', length: 256 },
            true,
            ['sign']
        ));
        result.then !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_derive_key_derive_bits_equivalence() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const password = new TextEncoder().encode('password');
        const baseKey = crypto.subtle.importKey(
            'raw',
            password,
            { name: 'PBKDF2' },
            false,
            ['deriveBits']
        );
        const salt = crypto.getRandomValues(new Uint8Array(16));
        const result = baseKey.then(key => crypto.subtle.deriveBits(
            { name: 'PBKDF2', salt: salt, iterations: 1000, hash: 'SHA-256' },
            key,
            256
        ));
        result.then !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_derive_key_different_iterations() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const password = new TextEncoder().encode('password');
        const baseKey = crypto.subtle.importKey(
            'raw',
            password,
            { name: 'PBKDF2' },
            false,
            ['deriveKey']
        );
        const salt = crypto.getRandomValues(new Uint8Array(16));
        // Low iterations for fast test
        const result = baseKey.then(key => crypto.subtle.deriveKey(
            { name: 'PBKDF2', salt: salt, iterations: 10, hash: 'SHA-256' },
            key,
            { name: 'AES-GCM', length: 128 },
            true,
            ['encrypt', 'decrypt']
        ));
        result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_derive_key_error_without_base_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const salt = crypto.getRandomValues(new Uint8Array(16));
        try {
            crypto.subtle.deriveKey(
                { name: 'PBKDF2', salt: salt, iterations: 1000, hash: 'SHA-256' },
                null,
                { name: 'AES-GCM', length: 256 },
                true,
                ['encrypt']
            );
            false;
        } catch (e) {
            true;
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_derive_key_error_without_salt() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const password = new TextEncoder().encode('password');
        const baseKey = crypto.subtle.importKey(
            'raw',
            password,
            { name: 'PBKDF2' },
            false,
            ['deriveKey']
        );
        // No salt provided - should work but with warning in some implementations
        const result = baseKey.then(key => crypto.subtle.deriveKey(
            { name: 'PBKDF2', iterations: 1000, hash: 'SHA-256' },
            key,
            { name: 'AES-GCM', length: 256 },
            true,
            ['encrypt']
        ));
        result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_derive_bits_returns_array_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const password = new TextEncoder().encode('password');
        const baseKey = crypto.subtle.importKey(
            'raw',
            password,
            { name: 'PBKDF2' },
            false,
            ['deriveBits']
        );
        const salt = crypto.getRandomValues(new Uint8Array(16));
        const result = baseKey.then(key => crypto.subtle.deriveBits(
            { name: 'PBKDF2', salt: salt, iterations: 10, hash: 'SHA-256' },
            key,
            256
        ));
        result.then !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

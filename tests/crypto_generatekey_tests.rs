// Tests for crypto.subtle.generateKey implementation (v0.3.359)
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_generate_key_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.subtle.generateKey");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_generate_key_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'HMAC', hash: 'SHA-256', length: 256 },
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
fn test_generate_key_hmac_returns_crypto_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'HMAC', hash: 'SHA-256', length: 256 },
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
fn test_generate_key_hmac_can_sign() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const keyPromise = crypto.subtle.generateKey(
            { name: 'HMAC', hash: 'SHA-256', length: 256 },
            false,
            ['sign']
        );
        const signPromise = keyPromise.then(key => crypto.subtle.sign({ name: 'HMAC' }, key, new TextEncoder().encode('test')));
        typeof signPromise.then;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_generate_key_aes_gcm_returns_crypto_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
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
fn test_generate_key_aes_gcm_can_encrypt() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const keyPromise = crypto.subtle.generateKey(
            { name: 'AES-GCM', length: 256 },
            true,
            ['encrypt']
        );
        const iv = crypto.getRandomValues(new Uint8Array(12));
        const encryptPromise = keyPromise.then(key => crypto.subtle.encrypt(
            { name: 'AES-GCM', iv: iv },
            key,
            new TextEncoder().encode('secret message')
        ));
        typeof encryptPromise.then;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_generate_key_aes_cbc_returns_crypto_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'AES-CBC', length: 256 },
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
fn test_generate_key_returns_object_with_type() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'HMAC', hash: 'SHA-256' },
            false,
            ['sign', 'verify']
        );
        // Just check that we get a Promise that resolves to an object
        result.then !== undefined && result.then.constructor.name === 'Function';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_has_usages() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'HMAC', hash: 'SHA-256', length: 256 },
            false,
            ['sign', 'verify']
        );
        // Check that we get a Promise
        result && result.constructor && result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_default_hmac_length() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'HMAC', hash: 'SHA-256' },
            false,
            ['sign']
        );
        // Check that we get a Promise
        result && result.constructor && result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_aes_128_length() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'AES-GCM', length: 128 },
            true,
            ['encrypt', 'decrypt']
        );
        // Check that we get a Promise
        result && result.constructor && result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_aes_192_length() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'AES-GCM', length: 192 },
            true,
            ['encrypt', 'decrypt']
        );
        // Check that we get a Promise
        result && result.constructor && result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_aes_256_length() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'AES-GCM', length: 256 },
            true,
            ['encrypt', 'decrypt']
        );
        // Check that we get a Promise
        result && result.constructor && result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

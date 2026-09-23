// Tests for crypto.createSign module (v0.3.19)
// Digital signature creation using RSA keys
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_crypto_create_sign_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.createSign");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_create_sign_returns_sign_object() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // Use a valid PEM format private key for testing
    let code = r#"
        const privateKey = `-----BEGIN RSA PRIVATE KEY-----
MIIBOgIBAAJBALRiML3m2kE17bGNmH0U72m0/1D4Q4B5T+Ci5KqNL2a0o9a8
test-private-key-placeholder
-----END RSA PRIVATE KEY-----`;
        const sign = crypto.createSign('RSA-SHA256');
        typeof sign;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_sign_update_method_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const sign = crypto.createSign('RSA-SHA256');
        typeof sign.update;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_sign_method_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const sign = crypto.createSign('RSA-SHA256');
        typeof sign.sign;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_sign_chain_update_digest() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const sign = crypto.createSign('RSA-SHA256');
        const result = sign.update('test data');
        typeof result;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_sign_unsupported_algorithm() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            crypto.createSign('ECDSA-SHA256');
        } catch (e) {
            e.message;
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    assert!(output.contains("unsupported") || output.contains("RSA-SHA256"));
}

#[test]
#[serial]
fn test_sign_with_hex_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const sign = crypto.createSign('RSA-SHA256');
        sign.update('hello');
        const signature = sign.sign(privateKey, 'hex');
        typeof signature === 'string' && /^[0-9a-f]+$/.test(signature);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_sign_signature_length() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const sign = crypto.createSign('RSA-SHA256');
        sign.update('test message');
        const signature = sign.sign(privateKey, 'base64');
        signature.length > 0;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_sign_multiple_updates() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const sign = crypto.createSign('RSA-SHA256');
        sign.update('part1');
        sign.update('part2');
        sign.update('part3');
        const signature = sign.sign(privateKey, 'hex');
        typeof signature;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "string");
}

#[test]
#[serial]
fn test_sign_digest_without_update() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const sign = crypto.createSign('RSA-SHA256');
        const signature = sign.sign(privateKey, 'hex');
        typeof signature;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "string");
}

#[test]
#[serial]
fn test_sign_different_hash_algorithms() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        // Test RSA-SHA512 algorithm
        const sign = crypto.createSign('RSA-SHA512');
        typeof sign;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_sign_algorithm_property() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const sign = crypto.createSign('RSA-SHA256');
        sign._algorithm;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "RSA-SHA256");
}

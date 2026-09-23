// Tests for crypto.createVerify module (v0.3.20)
// Digital signature verification using RSA public keys
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_crypto_create_verify_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.createVerify");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_create_verify_returns_verify_object() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const publicKey = `-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA0Z3VS5JJcds3xfn/ygW
test-public-key-placeholder
-----END PUBLIC KEY-----`;
        const verify = crypto.createVerify('RSA-SHA256');
        typeof verify;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_verify_update_method_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const verify = crypto.createVerify('RSA-SHA256');
        typeof verify.update;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_verify_method_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const verify = crypto.createVerify('RSA-SHA256');
        typeof verify.verify;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_verify_chain_update_digest() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const verify = crypto.createVerify('RSA-SHA256');
        const result = verify.update('test data');
        typeof result;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_verify_unsupported_algorithm() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            crypto.createVerify('ECDSA-SHA256');
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
fn test_verify_returns_boolean() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const sign = crypto.createSign('RSA-SHA256');
        sign.update('test data');
        const signature = sign.sign(privateKey, 'hex');
        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('test data');
        const result = verify.verify(publicKey, signature, 'hex');
        typeof result;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "boolean");
}

#[test]
#[serial]
fn test_verify_with_hex_signature() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const sign = crypto.createSign('RSA-SHA256');
        sign.update('hello world');
        const signature = sign.sign(privateKey, 'hex');
        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('hello world');
        const result = verify.verify(publicKey, signature, 'hex');
        typeof result;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "boolean");
}

#[test]
#[serial]
fn test_verify_with_base64_signature() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const sign = crypto.createSign('RSA-SHA256');
        sign.update('test message');
        const signature = sign.sign(privateKey, 'base64');
        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('test message');
        const result = verify.verify(publicKey, signature, 'base64');
        typeof result;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "boolean");
}

#[test]
#[serial]
fn test_verify_multiple_updates() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const sign = crypto.createSign('RSA-SHA256');
        sign.update('part1');
        sign.update('part2');
        sign.update('part3');
        const signature = sign.sign(privateKey, 'hex');
        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('part1');
        verify.update('part2');
        verify.update('part3');
        const result = verify.verify(publicKey, signature, 'hex');
        typeof result;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "boolean");
}

#[test]
#[serial]
fn test_verify_digest_without_update() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const sign = crypto.createSign('RSA-SHA256');
        const signature = sign.sign(privateKey, 'hex');
        const verify = crypto.createVerify('RSA-SHA256');
        const result = verify.verify(publicKey, signature, 'hex');
        typeof result;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "boolean");
}

#[test]
#[serial]
fn test_verify_different_hash_algorithms() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const verify = crypto.createVerify('RSA-SHA512');
        typeof verify;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_verify_algorithm_property() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const verify = crypto.createVerify('RSA-SHA256');
        verify._algorithm;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "RSA-SHA256");
}

#[test]
#[serial]
fn test_sign_and_verify_workflow() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const sign = crypto.createSign('RSA-SHA256');
        sign.update('message to sign');
        const signature = sign.sign(privateKey, 'hex');

        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('message to sign');
        verify.verify(publicKey, signature, 'hex') === true;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_crypto_constants_exposes_rsa_pkcs1_pss_padding() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        typeof crypto.constants.RSA_PKCS1_PSS_PADDING === 'number';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_create_sign_rsa_pss_with_options_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const pss = {
            padding: crypto.constants.RSA_PKCS1_PSS_PADDING,
            saltLength: 32
        };

        const sign = crypto.createSign('RSA-SHA256');
        sign.update('pss message');
        const signature = sign.sign({ key: privateKey, ...pss }, 'base64');

        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('pss message');
        verify.verify({ key: publicKey, ...pss }, signature, 'base64') === true;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_create_verify_rsa_pss_rejects_pkcs1_signature() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const sign = crypto.createSign('RSA-SHA256');
        sign.update('same message');
        const pkcs1Signature = sign.sign(privateKey, 'base64');

        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('same message');
        verify.verify({
            key: publicKey,
            padding: crypto.constants.RSA_PKCS1_PSS_PADDING,
            saltLength: 32
        }, pkcs1Signature, 'base64') === false;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_create_verify_rsa_pss_wrong_salt_length_returns_false() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const sign = crypto.createSign('RSA-SHA256');
        sign.update('salt length message');
        const signature = sign.sign({
            key: privateKey,
            padding: crypto.constants.RSA_PKCS1_PSS_PADDING,
            saltLength: 32
        }, 'base64');

        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('salt length message');
        verify.verify({
            key: publicKey,
            padding: crypto.constants.RSA_PKCS1_PSS_PADDING,
            saltLength: 20
        }, signature, 'base64') === false;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

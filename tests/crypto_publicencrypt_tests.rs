// Tests for crypto.publicEncrypt/crypto.privateDecrypt module (v0.3.21)
// Public key encryption and decryption using RSA
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_crypto_public_encrypt_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.publicEncrypt");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_crypto_private_decrypt_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.privateDecrypt");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_public_encrypt_returns_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encrypted = crypto.publicEncrypt(publicKey, Buffer.from('test message'));
        Buffer.isBuffer(encrypted);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_private_decrypt_returns_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encrypted = crypto.publicEncrypt(publicKey, Buffer.from('encrypted_data'));
        const decrypted = crypto.privateDecrypt(privateKey, encrypted);
        Buffer.isBuffer(decrypted);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_public_encrypt_with_encoding() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encrypted = crypto.publicEncrypt({
            key: publicKey,
            padding: crypto.constants.RSA_PKCS1_OAEP_PADDING
        }, Buffer.from('test'));
        typeof encrypted;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_public_encrypt_with_rsa_padding() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encrypted = crypto.publicEncrypt({
            key: publicKey,
            padding: crypto.constants.RSA_PKCS1_PADDING
        }, Buffer.from('test data'));
        encrypted.length > 0;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_private_decrypt_with_encoding() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encryptedHex = crypto.publicEncrypt(
            publicKey,
            Buffer.from('test')
        ).toString('hex');
        const decrypted = crypto.privateDecrypt(privateKey, encryptedHex, 'hex');
        decrypted.toString('utf8') === 'test';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_public_encrypt_invalid_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            crypto.publicEncrypt('invalid-key', Buffer.from('test'));
        } catch (e) {
            e.message.includes('invalid') || e.message.includes('key');
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let binding = result.unwrap();
    assert!(binding.trim() == "true");
}

#[test]
#[serial]
fn test_private_decrypt_invalid_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            crypto.privateDecrypt('invalid-key', Buffer.from('test'));
        } catch (e) {
            e.message.includes('invalid') || e.message.includes('key');
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let binding = result.unwrap();
    assert!(binding.trim() == "true");
}

#[test]
#[serial]
fn test_encrypt_decrypt_roundtrip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const message = 'Secret message';
        const encrypted = crypto.publicEncrypt(publicKey, Buffer.from(message));
        const decrypted = crypto.privateDecrypt(privateKey, encrypted);
        decrypted.toString('utf8') === message;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_constants_rsa_padding() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        typeof crypto.constants.RSA_PKCS1_PADDING;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "number");
}

#[test]
#[serial]
fn test_public_encrypt_empty_data() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encrypted = crypto.publicEncrypt(publicKey, Buffer.from(''));
        encrypted.length > 0;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_private_decrypt_empty_data() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encrypted = crypto.publicEncrypt(publicKey, Buffer.from(''));
        const decrypted = crypto.privateDecrypt(privateKey, encrypted);
        decrypted.length === 0;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

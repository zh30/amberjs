// Tests for crypto.privateEncrypt/crypto.publicDecrypt module (v0.3.22)
// Private key encryption and public key decryption using RSA
// This is the inverse operation of publicEncrypt/privateDecrypt
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_crypto_private_encrypt_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.privateEncrypt");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_crypto_public_decrypt_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.publicDecrypt");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_private_encrypt_returns_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encrypted = crypto.privateEncrypt(privateKey, Buffer.from('test message'));
        Buffer.isBuffer(encrypted);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_public_decrypt_returns_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encrypted = crypto.privateEncrypt(privateKey, Buffer.from('encrypted_data'));
        const decrypted = crypto.publicDecrypt(publicKey, encrypted);
        Buffer.isBuffer(decrypted);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_private_encrypt_with_encoding() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encrypted = crypto.privateEncrypt({
            key: privateKey,
            padding: crypto.constants.RSA_PKCS1_PADDING
        }, Buffer.from('test'));
        typeof encrypted;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_public_decrypt_with_encoding() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encryptedHex = crypto.privateEncrypt(
            privateKey,
            Buffer.from('test')
        ).toString('hex');
        const decrypted = crypto.publicDecrypt(publicKey, encryptedHex, 'hex');
        decrypted.toString('utf8') === 'test';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_private_encrypt_with_rsa_padding() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encrypted = crypto.privateEncrypt({
            key: privateKey,
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
fn test_private_encrypt_invalid_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            crypto.privateEncrypt('invalid-key', Buffer.from('test'));
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
fn test_public_decrypt_invalid_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            crypto.publicDecrypt('invalid-key', Buffer.from('test'));
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
fn test_private_public_decrypt_roundtrip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const message = 'Secret message signed with private key';
        const encrypted = crypto.privateEncrypt(privateKey, Buffer.from(message));
        const decrypted = crypto.publicDecrypt(publicKey, encrypted);
        decrypted.toString('utf8') === message;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_private_encrypt_empty_data() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encrypted = crypto.privateEncrypt(privateKey, Buffer.from(''));
        encrypted.length > 0;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_public_decrypt_empty_data() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encrypted = crypto.privateEncrypt(privateKey, Buffer.from(''));
        const decrypted = crypto.publicDecrypt(publicKey, encrypted);
        decrypted.length === 0;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_private_encrypt_pkcs1_padding() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encrypted = crypto.privateEncrypt({
            key: privateKey,
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
fn test_public_decrypt_pkcs1_padding() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encrypted = crypto.privateEncrypt({
            key: privateKey,
            padding: crypto.constants.RSA_PKCS1_PADDING
        }, Buffer.from('test data'));
        const decrypted = crypto.publicDecrypt({
            key: publicKey,
            padding: crypto.constants.RSA_PKCS1_PADDING
        }, encrypted);
        decrypted.toString('utf8') === 'test data';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_constants_rsa_padding_available() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        // All RSA padding constants should be available
        const constants = crypto.constants;
        typeof constants.RSA_PKCS1_PADDING === 'number' &&
        typeof constants.RSA_PKCS1_OAEP_PADDING === 'number' &&
        typeof constants.RSA_NO_PADDING === 'number';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

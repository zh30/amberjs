// Tests for crypto.generateKeyPairSync module (v0.3.23)
// RSA and EC key pair generation for digital signatures and encryption
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_crypto_generate_key_pair_sync_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.generateKeyPairSync");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_rsa_returns_object() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048,
            publicKeyEncoding: { type: 'spki', format: 'pem' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        });
        typeof result;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_rsa_has_keys() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048,
            publicKeyEncoding: { type: 'spki', format: 'pem' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        });
        typeof result.publicKey === 'string' && typeof result.privateKey === 'string';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_rsa_key_format() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048,
            publicKeyEncoding: { type: 'spki', format: 'pem' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        });
        result.publicKey.indexOf('-----BEGIN PUBLIC KEY-----') >= 0;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_private_key_encoding_cipher_passphrase_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const passphrase = 'amberjs-generated-key-passphrase';
        const result = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048,
            publicKeyEncoding: { type: 'spki', format: 'pem' },
            privateKeyEncoding: {
                type: 'pkcs8',
                format: 'pem',
                cipher: 'aes-256-cbc',
                passphrase
            }
        });

        const privateKey = crypto.createPrivateKey({
            key: result.privateKey,
            type: 'pkcs8',
            format: 'pem',
            passphrase
        });
        const sign = crypto.createSign('RSA-SHA256');
        sign.update('generated encrypted private key');
        const signature = sign.sign(privateKey, 'base64');

        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('generated encrypted private key');
        result.privateKey.includes('BEGIN ENCRYPTED PRIVATE KEY') &&
            !result.privateKey.includes('BEGIN RSA PRIVATE KEY') &&
            verify.verify(result.publicKey, signature, 'base64') === true;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_public_key_encoding_der_returns_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048,
            publicKeyEncoding: { type: 'spki', format: 'der' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        });
        const importedPublicKey = crypto.createPublicKey({
            key: publicKey,
            type: 'spki',
            format: 'der'
        });

        const sign = crypto.createSign('RSA-SHA256');
        sign.update('generated public der');
        const signature = sign.sign(privateKey, 'base64');

        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('generated public der');
        Buffer.isBuffer(publicKey) &&
            publicKey.length > 128 &&
            !publicKey.toString('utf8').includes('BEGIN') &&
            verify.verify(importedPublicKey, signature, 'base64') === true;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_public_key_encoding_pkcs1_pem_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048,
            publicKeyEncoding: { type: 'pkcs1', format: 'pem' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        });
        const importedPublicKey = crypto.createPublicKey({
            key: publicKey,
            type: 'pkcs1',
            format: 'pem'
        });

        const sign = crypto.createSign('RSA-SHA256');
        sign.update('generated public pkcs1 pem');
        const signature = sign.sign(privateKey, 'base64');

        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('generated public pkcs1 pem');
        typeof publicKey === 'string' &&
            publicKey.includes('BEGIN RSA PUBLIC KEY') &&
            verify.verify(importedPublicKey, signature, 'base64') === true;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_public_key_encoding_pkcs1_der_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048,
            publicKeyEncoding: { type: 'pkcs1', format: 'der' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        });
        const importedPublicKey = crypto.createPublicKey({
            key: publicKey,
            type: 'pkcs1',
            format: 'der'
        });

        const sign = crypto.createSign('RSA-SHA256');
        sign.update('generated public pkcs1 der');
        const signature = sign.sign(privateKey, 'base64');

        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('generated public pkcs1 der');
        Buffer.isBuffer(publicKey) &&
            publicKey.length > 128 &&
            !publicKey.toString('utf8').includes('BEGIN') &&
            verify.verify(importedPublicKey, signature, 'base64') === true;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_private_key_encoding_der_returns_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048,
            publicKeyEncoding: { type: 'spki', format: 'pem' },
            privateKeyEncoding: { type: 'pkcs8', format: 'der' }
        });
        const importedPrivateKey = crypto.createPrivateKey({
            key: privateKey,
            type: 'pkcs8',
            format: 'der'
        });

        const sign = crypto.createSign('RSA-SHA256');
        sign.update('generated private der');
        const signature = sign.sign(importedPrivateKey, 'base64');

        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('generated private der');
        Buffer.isBuffer(privateKey) &&
            privateKey.length > 256 &&
            !privateKey.toString('utf8').includes('BEGIN') &&
            verify.verify(publicKey, signature, 'base64') === true;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_rsa_jwk_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048,
            publicKeyEncoding: { format: 'jwk' },
            privateKeyEncoding: { format: 'jwk' }
        });
        const importedPublicKey = crypto.createPublicKey({
            key: publicKey,
            format: 'jwk'
        });
        const importedPrivateKey = crypto.createPrivateKey({
            key: privateKey,
            format: 'jwk'
        });

        const base64url = /^[A-Za-z0-9_-]+$/;
        const publicFieldsOk =
            publicKey.kty === 'RSA' &&
            base64url.test(publicKey.n) &&
            base64url.test(publicKey.e) &&
            !publicKey.n.includes('=') &&
            privateKey.kty === 'RSA' &&
            privateKey.n === publicKey.n &&
            privateKey.e === publicKey.e &&
            ['d', 'p', 'q', 'dp', 'dq', 'qi'].every((field) =>
                typeof privateKey[field] === 'string' &&
                base64url.test(privateKey[field]) &&
                !privateKey[field].includes('=')
            );

        const sign = crypto.createSign('RSA-SHA256');
        sign.update('generated rsa jwk');
        const signature = sign.sign(importedPrivateKey, 'base64');

        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('generated rsa jwk');
        publicFieldsOk && verify.verify(importedPublicKey, signature, 'base64') === true;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_ec_jwk_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('ec', {
            namedCurve: 'prime256v1',
            publicKeyEncoding: { format: 'jwk' },
            privateKeyEncoding: { format: 'jwk' }
        });
        const importedPublicKey = crypto.createPublicKey({
            key: publicKey,
            format: 'jwk'
        });
        const importedPrivateKey = crypto.createPrivateKey({
            key: privateKey,
            format: 'jwk'
        });

        const base64url = /^[A-Za-z0-9_-]+$/;
        const fieldsOk =
            publicKey.kty === 'EC' &&
            publicKey.crv === 'P-256' &&
            base64url.test(publicKey.x) &&
            base64url.test(publicKey.y) &&
            !publicKey.x.includes('=') &&
            privateKey.kty === 'EC' &&
            privateKey.crv === 'P-256' &&
            privateKey.x === publicKey.x &&
            privateKey.y === publicKey.y &&
            base64url.test(privateKey.d) &&
            !privateKey.d.includes('=');

        const sign = crypto.createSign('SHA256');
        sign.update('generated ec jwk');
        const signature = sign.sign(importedPrivateKey, 'base64');

        const verify = crypto.createVerify('SHA256');
        verify.update('generated ec jwk');
        fieldsOk && verify.verify(importedPublicKey, signature, 'base64') === true;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_public_key_encoding_rejects_unsupported_format_once() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            crypto.generateKeyPairSync('rsa', {
                modulusLength: 2048,
                publicKeyEncoding: { type: 'spki', format: 'raw' },
                privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
            });
            false;
        } catch (e) {
            e.message.includes('generateKeyPairSync: unsupported publicKeyEncoding format') &&
                !e.message.includes('generateKeyPairSync: generateKeyPair:');
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_ec_pkcs1_public_encoding_reports_incompatible_key_options() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        let passCount = 0;
        for (const format of ['pem', 'der']) {
            try {
                crypto.generateKeyPairSync('ec', {
                    namedCurve: 'prime256v1',
                    publicKeyEncoding: { type: 'pkcs1', format },
                    privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
                });
            } catch (e) {
                if (e && e.code === 'ERR_CRYPTO_INCOMPATIBLE_KEY_OPTIONS') passCount++;
                if (e && e.name === 'Error') passCount++;
                if (e && e.message === 'The selected key encoding pkcs1 can only be used for RSA keys.') passCount++;
            }
        }
        passCount === 6;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_ec_returns_object() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.generateKeyPairSync('ec', {
            namedCurve: 'prime256v1',
            publicKeyEncoding: { type: 'spki', format: 'pem' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        });
        typeof result;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_ec_has_keys() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.generateKeyPairSync('ec', {
            namedCurve: 'prime256v1',
            publicKeyEncoding: { type: 'spki', format: 'pem' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        });
        typeof result.publicKey === 'string' && typeof result.privateKey === 'string';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_ec_key_format() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.generateKeyPairSync('ec', {
            namedCurve: 'prime256v1'
        });
        result.publicKey.indexOf('-----BEGIN PUBLIC KEY-----') >= 0;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_ec_sign_verify_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('ec', {
            namedCurve: 'prime256v1'
        });
        const sign = crypto.createSign('SHA256');
        sign.update('ec message');
        const signature = sign.sign(privateKey, 'base64');

        const verify = crypto.createVerify('SHA256');
        verify.update('ec message');
        verify.verify(publicKey, signature, 'base64') === true;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_ec_p384_sign_verify_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('ec', {
            namedCurve: 'secp384r1'
        });
        const sign = crypto.createSign('SHA384');
        sign.update('ec p384 message');
        const signature = sign.sign(privateKey, 'hex');

        const verify = crypto.createVerify('SHA384');
        verify.update('ec p384 message');
        verify.verify(publicKey, signature, 'hex') === true;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_rsa_different_modulus_lengths() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.generateKeyPairSync('rsa', {
            modulusLength: 4096
        });
        result.privateKey.length > 0;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_unsupported_type() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            crypto.generateKeyPairSync('dsa', { modulusLength: 2048 });
            false;
        } catch (e) {
            e.message.indexOf('unsupported') >= 0 || e.message.indexOf('not supported') >= 0;
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_missing_options() {
    // Node.js: generateKeyPairSync('rsa') should use default options (modulusLength: 2048)
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.generateKeyPairSync('rsa');
        typeof result === 'object' && typeof result.publicKey === 'string' && typeof result.privateKey === 'string';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_key_usage_in_signing() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const sign = crypto.createSign('RSA-SHA256');
        sign.update('test message');
        const signature = sign.sign(privateKey);
        typeof signature === 'string' && signature.length > 0;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_key_usage_in_verification() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const sign = crypto.createSign('RSA-SHA256');
        sign.update('test message');
        const signature = sign.sign(privateKey);
        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('test message');
        verify.verify(publicKey, signature) === true;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_multiple_calls_consistent() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result1 = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const result2 = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        // Each call should generate unique keys
        result1.publicKey !== result2.publicKey;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

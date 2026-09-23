// Tests for crypto.generateKeyPair module (v0.3.24)
// Asynchronous RSA/EC key pair generation with callback pattern
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_crypto_generate_key_pair_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.generateKeyPair");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_generate_key_pair_rsa_with_callback() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // Simple test - just verify the callback is invoked
    let code = r#"
        crypto.generateKeyPair('rsa', {
            modulusLength: 2048
        }, function(err, publicKey, privateKey) {
            callbackInvoked = true;
        });
        callbackInvoked;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_ec_with_callback() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.generateKeyPair('ec', {
            namedCurve: 'prime256v1'
        }, function(err, publicKey, privateKey) {
            callbackInvoked = true;
        });
        callbackInvoked;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_rsa_key_format() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.generateKeyPair('rsa', {
            modulusLength: 2048
        }, function(err, publicKey, privateKey) {
            hasPublicKey = (publicKey && publicKey.indexOf('-----BEGIN PUBLIC KEY-----') >= 0);
        });
        hasPublicKey;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_ec_key_format() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.generateKeyPair('ec', {
            namedCurve: 'prime256v1'
        }, function(err, publicKey, privateKey) {
            hasPublicKey = (publicKey && publicKey.indexOf('-----BEGIN PUBLIC KEY-----') >= 0);
        });
        hasPublicKey;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_ec_sign_verify_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.generateKeyPair('ec', {
            namedCurve: 'prime256v1'
        }, function(err, publicKey, privateKey) {
            if (err) {
                ecRoundTrip = false;
                return;
            }
            const sign = crypto.createSign('SHA256');
            sign.update('async ec message');
            const signature = sign.sign(privateKey, 'base64');

            const verify = crypto.createVerify('SHA256');
            verify.update('async ec message');
            ecRoundTrip = verify.verify(publicKey, signature, 'base64') === true;
        });
        ecRoundTrip;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_unsupported_type() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.generateKeyPair('dsa', { modulusLength: 2048 }, function(err, publicKey, privateKey) {
            gotError = (err && err.message && err.message.indexOf('unsupported') >= 0);
        });
        gotError;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_missing_callback_throws() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            crypto.generateKeyPair('rsa', {});
            false;
        } catch (e) {
            e.message.indexOf('callback') >= 0;
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_non_function_callback_throws() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            crypto.generateKeyPair('rsa', {}, 'not a function');
            false;
        } catch (e) {
            e.message.indexOf('callback') >= 0;
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_default_options() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.generateKeyPair('rsa', function(err, publicKey, privateKey) {
            callbackInvoked = true;
        });
        callbackInvoked;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_with_encoding_options() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.generateKeyPair('rsa', {
            modulusLength: 2048,
            publicKeyEncoding: { type: 'spki', format: 'pem' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        }, function(err, publicKey, privateKey) {
            hasKeys = !!(publicKey && privateKey);
        });
        hasKeys;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_private_key_encoding_cipher_passphrase_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const passphrase = 'amberjs-async-generated-key-passphrase';
        crypto.generateKeyPair('rsa', {
            modulusLength: 2048,
            publicKeyEncoding: { type: 'spki', format: 'pem' },
            privateKeyEncoding: {
                type: 'pkcs8',
                format: 'pem',
                cipher: 'aes-256-cbc',
                passphrase
            }
        }, function(err, publicKey, privateKeyPem) {
            if (err) {
                encryptedRoundTrip = false;
                return;
            }
            const privateKey = crypto.createPrivateKey({
                key: privateKeyPem,
                type: 'pkcs8',
                format: 'pem',
                passphrase
            });
            const sign = crypto.createSign('RSA-SHA256');
            sign.update('async generated encrypted private key');
            const signature = sign.sign(privateKey, 'base64');

            const verify = crypto.createVerify('RSA-SHA256');
            verify.update('async generated encrypted private key');
            encryptedRoundTrip =
                privateKeyPem.includes('BEGIN ENCRYPTED PRIVATE KEY') &&
                !privateKeyPem.includes('BEGIN RSA PRIVATE KEY') &&
                verify.verify(publicKey, signature, 'base64') === true;
        });
        encryptedRoundTrip;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_public_key_encoding_der_returns_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.generateKeyPair('rsa', {
            modulusLength: 2048,
            publicKeyEncoding: { type: 'spki', format: 'der' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        }, function(err, publicKey, privateKey) {
            if (err) {
                derRoundTrip = false;
                return;
            }
            const importedPublicKey = crypto.createPublicKey({
                key: publicKey,
                type: 'spki',
                format: 'der'
            });

            const sign = crypto.createSign('RSA-SHA256');
            sign.update('async generated public der');
            const signature = sign.sign(privateKey, 'base64');

            const verify = crypto.createVerify('RSA-SHA256');
            verify.update('async generated public der');
            derRoundTrip =
                Buffer.isBuffer(publicKey) &&
                publicKey.length > 128 &&
                !publicKey.toString('utf8').includes('BEGIN') &&
                verify.verify(importedPublicKey, signature, 'base64') === true;
        });
        derRoundTrip;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_public_key_encoding_pkcs1_der_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.generateKeyPair('rsa', {
            modulusLength: 2048,
            publicKeyEncoding: { type: 'pkcs1', format: 'der' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        }, function(err, publicKey, privateKey) {
            if (err) {
                derRoundTrip = false;
                return;
            }
            const importedPublicKey = crypto.createPublicKey({
                key: publicKey,
                type: 'pkcs1',
                format: 'der'
            });

            const sign = crypto.createSign('RSA-SHA256');
            sign.update('async generated public pkcs1 der');
            const signature = sign.sign(privateKey, 'base64');

            const verify = crypto.createVerify('RSA-SHA256');
            verify.update('async generated public pkcs1 der');
            derRoundTrip =
                Buffer.isBuffer(publicKey) &&
                publicKey.length > 128 &&
                !publicKey.toString('utf8').includes('BEGIN') &&
                verify.verify(importedPublicKey, signature, 'base64') === true;
        });
        derRoundTrip;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_private_key_encoding_der_returns_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.generateKeyPair('rsa', {
            modulusLength: 2048,
            publicKeyEncoding: { type: 'spki', format: 'pem' },
            privateKeyEncoding: { type: 'pkcs8', format: 'der' }
        }, function(err, publicKey, privateKey) {
            if (err) {
                derRoundTrip = false;
                return;
            }
            const importedPrivateKey = crypto.createPrivateKey({
                key: privateKey,
                type: 'pkcs8',
                format: 'der'
            });

            const sign = crypto.createSign('RSA-SHA256');
            sign.update('async generated private der');
            const signature = sign.sign(importedPrivateKey, 'base64');

            const verify = crypto.createVerify('RSA-SHA256');
            verify.update('async generated private der');
            derRoundTrip =
                Buffer.isBuffer(privateKey) &&
                privateKey.length > 256 &&
                !privateKey.toString('utf8').includes('BEGIN') &&
                verify.verify(publicKey, signature, 'base64') === true;
        });
        derRoundTrip;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_rsa_jwk_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.generateKeyPair('rsa', {
            modulusLength: 2048,
            publicKeyEncoding: { format: 'jwk' },
            privateKeyEncoding: { format: 'jwk' }
        }, function(err, publicKey, privateKey) {
            try {
                if (err) {
                    jwkRoundTrip = false;
                    return;
                }
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
                    publicKey.kty === 'RSA' &&
                    base64url.test(publicKey.n) &&
                    base64url.test(publicKey.e) &&
                    privateKey.kty === 'RSA' &&
                    privateKey.n === publicKey.n &&
                    privateKey.e === publicKey.e &&
                    ['d', 'p', 'q', 'dp', 'dq', 'qi'].every((field) =>
                        typeof privateKey[field] === 'string' &&
                        base64url.test(privateKey[field]) &&
                        !privateKey[field].includes('=')
                    );

                const sign = crypto.createSign('RSA-SHA256');
                sign.update('async generated rsa jwk');
                const signature = sign.sign(importedPrivateKey, 'base64');

                const verify = crypto.createVerify('RSA-SHA256');
                verify.update('async generated rsa jwk');
                jwkRoundTrip =
                    fieldsOk &&
                    verify.verify(importedPublicKey, signature, 'base64') === true;
            } catch (error) {
                jwkRoundTrip = false;
            }
        });
        jwkRoundTrip;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_ec_jwk_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.generateKeyPair('ec', {
            namedCurve: 'prime256v1',
            publicKeyEncoding: { format: 'jwk' },
            privateKeyEncoding: { format: 'jwk' }
        }, function(err, publicKey, privateKey) {
            try {
                if (err) {
                    jwkRoundTrip = false;
                    return;
                }
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
                    privateKey.kty === 'EC' &&
                    privateKey.crv === 'P-256' &&
                    privateKey.x === publicKey.x &&
                    privateKey.y === publicKey.y &&
                    base64url.test(privateKey.d) &&
                    !privateKey.d.includes('=');

                const sign = crypto.createSign('SHA256');
                sign.update('async generated ec jwk');
                const signature = sign.sign(importedPrivateKey, 'base64');

                const verify = crypto.createVerify('SHA256');
                verify.update('async generated ec jwk');
                jwkRoundTrip =
                    fieldsOk &&
                    verify.verify(importedPublicKey, signature, 'base64') === true;
            } catch (error) {
                jwkRoundTrip = false;
            }
        });
        jwkRoundTrip;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_public_key_encoding_rejects_unsupported_format() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.generateKeyPair('rsa', {
            modulusLength: 2048,
            publicKeyEncoding: { type: 'spki', format: 'raw' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        }, function(err, publicKey, privateKey) {
            rejected =
                !!err &&
                err.message.includes('generateKeyPair: unsupported publicKeyEncoding format') &&
                publicKey === null &&
                privateKey === null;
        });
        rejected;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_key_usage_in_signing() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.generateKeyPair('rsa', { modulusLength: 2048 }, function(err, publicKey, privateKey) {
            if (err) {
                signingWorks = false;
            } else {
                var sign = crypto.createSign('RSA-SHA256');
                sign.update('test message');
                var signature = sign.sign(privateKey);
                signingWorks = (typeof signature === 'string' && signature.length > 0);
            }
        });
        signingWorks;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_callback_sets_result() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.generateKeyPair('rsa', function(err, publicKey, privateKey) {
            callbackInvoked = true;
        });
        callbackInvoked;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

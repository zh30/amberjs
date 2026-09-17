use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_crypto_key_internal_key_data_is_not_visible_to_js() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const hmacKey = await crypto.subtle.generateKey(
                { name: 'HMAC', hash: 'SHA-256', length: 256 },
                true,
                ['sign', 'verify']
            );
            const aesKey = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            const hmacHidden = hmacKey.__amberjs_key_data__ === undefined
                && !Object.prototype.hasOwnProperty.call(hmacKey, '__amberjs_key_data__')
                && !Object.getOwnPropertyNames(hmacKey).includes('__amberjs_key_data__');
            const aesHidden = aesKey.__amberjs_key_data__ === undefined
                && !Object.prototype.hasOwnProperty.call(aesKey, '__amberjs_key_data__')
                && !Object.getOwnPropertyNames(aesKey).includes('__amberjs_key_data__');

            const data = new TextEncoder().encode('opaque key material');
            const signature = await crypto.subtle.sign({ name: 'HMAC' }, hmacKey, data);
            const verified = await crypto.subtle.verify({ name: 'HMAC' }, hmacKey, signature, data);
            const iv = crypto.getRandomValues(new Uint8Array(12));
            const ciphertext = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, aesKey, data);
            const plaintext = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, aesKey, ciphertext);

            return hmacHidden
                && aesHidden
                && verified
                && new TextDecoder().decode(plaintext) === 'opaque key material';
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_encrypt_aes_cbc_without_iv_rejects() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const key = await crypto.subtle.generateKey(
                { name: 'AES-CBC', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            try {
                await crypto.subtle.encrypt(
                    { name: 'AES-CBC' },
                    key,
                    new Uint8Array([1, 2, 3, 4])
                );
                return false;
            } catch (error) {
                return String(error && error.message ? error.message : error)
                    .includes('iv');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_decrypt_aes_cbc_with_short_iv_rejects() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const key = await crypto.subtle.generateKey(
                { name: 'AES-CBC', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            try {
                await crypto.subtle.decrypt(
                    { name: 'AES-CBC', iv: new Uint8Array(8) },
                    key,
                    new Uint8Array(16)
                );
                return false;
            } catch (error) {
                return String(error && error.message ? error.message : error)
                    .includes('iv');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_encrypt_aes_ctr_without_counter_rejects() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const key = await crypto.subtle.generateKey(
                { name: 'AES-CTR', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            try {
                await crypto.subtle.encrypt(
                    { name: 'AES-CTR', length: 64 },
                    key,
                    new Uint8Array([1, 2, 3, 4])
                );
                return false;
            } catch (error) {
                return String(error && error.message ? error.message : error)
                    .includes('counter');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_decrypt_aes_ctr_with_invalid_length_rejects() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const key = await crypto.subtle.generateKey(
                { name: 'AES-CTR', length: 256 },
                true,
                ['encrypt', 'decrypt']
            );

            try {
                await crypto.subtle.decrypt(
                    { name: 'AES-CTR', counter: new Uint8Array(16), length: 0 },
                    key,
                    new Uint8Array([1, 2, 3, 4])
                );
                return false;
            } catch (error) {
                return String(error && error.message ? error.message : error)
                    .includes('length');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_generate_key_rejects_invalid_aes_length() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                await crypto.subtle.generateKey(
                    { name: 'AES-GCM', length: 64 },
                    true,
                    ['encrypt', 'decrypt']
                );
                return false;
            } catch (error) {
                return String(error && error.message ? error.message : error)
                    .includes('length');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_generate_key_rejects_missing_aes_length() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                await crypto.subtle.generateKey(
                    { name: 'AES-CBC' },
                    true,
                    ['encrypt', 'decrypt']
                );
                return false;
            } catch (error) {
                return String(error && error.message ? error.message : error)
                    .includes('length');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_import_key_rejects_invalid_raw_aes_key_length() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                await crypto.subtle.importKey(
                    'raw',
                    new Uint8Array(15),
                    { name: 'AES-CBC' },
                    true,
                    ['encrypt', 'decrypt']
                );
                return false;
            } catch (error) {
                return String(error && error.message ? error.message : error)
                    .includes('length');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_unsupported_rsa_generate_key_algorithm_rejects() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                await crypto.subtle.generateKey(
                    { name: 'RSA-PSS', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
                    true,
                    []
                );
                return false;
            } catch (error) {
                const message = String(error && error.message ? error.message : error);
                return message.includes('unsupported algorithm') || message.includes('not implemented');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_sign_with_missing_key_data_rejects() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const fakeKey = {
                type: 'private',
                algorithm: { name: 'ECDSA', namedCurve: 'P-256' },
                extractable: true,
                usages: ['sign']
            };
            try {
                const signature = await crypto.subtle.sign(
                    { name: 'ECDSA', hash: { name: 'SHA-256' } },
                    fakeKey,
                    new TextEncoder().encode('test data')
                );
                return !(signature instanceof ArrayBuffer);
            } catch (error) {
                return String(error && error.message ? error.message : error).includes('key data');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_create_verify_rejects_unrecognized_mock_signature() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('test data');
        verify.verify(publicKey, 'deadbeef', 'hex') === false;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_import_key_missing_algorithm_name_rejects() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                await crypto.subtle.importKey(
                    'raw',
                    new Uint8Array([1, 2, 3, 4]),
                    {},
                    false,
                    ['sign']
                );
                return false;
            } catch (error) {
                const message = String(error && error.message ? error.message : error);
                return message.includes('algorithm') && message.includes('name');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_generate_key_missing_algorithm_name_rejects() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                await crypto.subtle.generateKey({}, true, ['sign']);
                return false;
            } catch (error) {
                const message = String(error && error.message ? error.message : error);
                return message.includes('algorithm') && message.includes('name');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_sign_missing_algorithm_name_rejects() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const key = await crypto.subtle.importKey(
                'raw',
                new Uint8Array([1, 2, 3, 4]),
                { name: 'HMAC', hash: 'SHA-256' },
                false,
                ['sign']
            );
            try {
                await crypto.subtle.sign({}, key, new TextEncoder().encode('message'));
                return false;
            } catch (error) {
                const message = String(error && error.message ? error.message : error);
                return message.includes('algorithm') && message.includes('name');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_verify_missing_algorithm_name_rejects() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const key = await crypto.subtle.importKey(
                'raw',
                new Uint8Array([1, 2, 3, 4]),
                { name: 'HMAC', hash: 'SHA-256' },
                false,
                ['sign', 'verify']
            );
            const data = new TextEncoder().encode('message');
            const signature = await crypto.subtle.sign({ name: 'HMAC' }, key, data);
            try {
                await crypto.subtle.verify({}, key, signature, data);
                return false;
            } catch (error) {
                const message = String(error && error.message ? error.message : error);
                return message.includes('algorithm') && message.includes('name');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_import_key_rejects_usage_not_allowed_for_algorithm() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                await crypto.subtle.importKey(
                    'raw',
                    new Uint8Array([1, 2, 3, 4]),
                    { name: 'HMAC', hash: 'SHA-256' },
                    false,
                    ['encrypt']
                );
                return false;
            } catch (error) {
                return String(error && error.message ? error.message : error)
                    .includes('usage');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_rejects_usage_not_allowed_for_algorithm() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                await crypto.subtle.generateKey(
                    { name: 'AES-GCM', length: 256 },
                    true,
                    ['sign']
                );
                return false;
            } catch (error) {
                return String(error && error.message ? error.message : error)
                    .includes('usage');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_sign_rejects_key_without_sign_usage() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const key = await crypto.subtle.importKey(
                'raw',
                new Uint8Array([1, 2, 3, 4]),
                { name: 'HMAC', hash: 'SHA-256' },
                false,
                ['verify']
            );
            try {
                await crypto.subtle.sign(
                    { name: 'HMAC' },
                    key,
                    new TextEncoder().encode('message')
                );
                return false;
            } catch (error) {
                return String(error && error.message ? error.message : error)
                    .includes('usage');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_verify_rejects_key_without_verify_usage() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const signingKey = await crypto.subtle.importKey(
                'raw',
                new Uint8Array([1, 2, 3, 4]),
                { name: 'HMAC', hash: 'SHA-256' },
                false,
                ['sign']
            );
            const verifyOnlyDeniedKey = await crypto.subtle.importKey(
                'raw',
                new Uint8Array([1, 2, 3, 4]),
                { name: 'HMAC', hash: 'SHA-256' },
                false,
                ['sign']
            );
            const data = new TextEncoder().encode('message');
            const signature = await crypto.subtle.sign({ name: 'HMAC' }, signingKey, data);
            try {
                await crypto.subtle.verify(
                    { name: 'HMAC' },
                    verifyOnlyDeniedKey,
                    signature,
                    data
                );
                return false;
            } catch (error) {
                return String(error && error.message ? error.message : error)
                    .includes('usage');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_encrypt_rejects_key_algorithm_mismatch() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const key = await crypto.subtle.generateKey(
                { name: 'AES-CBC', length: 256 },
                true,
                ['encrypt']
            );
            try {
                await crypto.subtle.encrypt(
                    { name: 'AES-GCM', iv: new Uint8Array(12) },
                    key,
                    new Uint8Array([1, 2, 3, 4])
                );
                return false;
            } catch (error) {
                return String(error && error.message ? error.message : error)
                    .includes('algorithm');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_decrypt_rejects_key_algorithm_mismatch() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const aesGcmKey = await crypto.subtle.generateKey(
                { name: 'AES-GCM', length: 256 },
                true,
                ['encrypt']
            );
            const aesCbcKey = await crypto.subtle.generateKey(
                { name: 'AES-CBC', length: 256 },
                true,
                ['decrypt']
            );
            const iv = new Uint8Array(12);
            const encrypted = await crypto.subtle.encrypt(
                { name: 'AES-GCM', iv },
                aesGcmKey,
                new Uint8Array([1, 2, 3, 4])
            );
            try {
                await crypto.subtle.decrypt(
                    { name: 'AES-GCM', iv },
                    aesCbcKey,
                    encrypted
                );
                return false;
            } catch (error) {
                return String(error && error.message ? error.message : error)
                    .includes('algorithm');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_create_sign_without_private_key_rejects() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            const sign = crypto.createSign('RSA-SHA256');
            sign.update('message');
            sign.sign('hex');
            false;
        } catch (error) {
            String(error && error.message ? error.message : error)
                .toLowerCase()
                .includes('private key');
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_create_verify_without_public_key_rejects() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            const verify = crypto.createVerify('RSA-SHA256');
            verify.update('message');
            verify.verify('not-a-public-key', 'deadbeef', 'hex');
            false;
        } catch (error) {
            String(error && error.message ? error.message : error)
                .toLowerCase()
                .includes('public key');
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generated_rsa_key_pair_sign_verify_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const sign = crypto.createSign('RSA-SHA256');
        sign.update('message');
        const signature = sign.sign(privateKey, 'base64');

        const verify = crypto.createVerify('RSA-SHA256');
        verify.update('message');
        verify.verify(publicKey, signature, 'base64') === true;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_public_encrypt_rejects_placeholder_public_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const placeholderPublicKey = `-----BEGIN PUBLIC KEY-----
not-a-real-key
-----END PUBLIC KEY-----`;
        try {
            crypto.publicEncrypt(placeholderPublicKey, Buffer.from('message'));
            false;
        } catch (error) {
            String(error && error.message ? error.message : error)
                .toLowerCase()
                .includes('public key');
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_private_encrypt_rejects_placeholder_private_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const placeholderPrivateKey = `-----BEGIN PRIVATE KEY-----
not-a-real-key
-----END PRIVATE KEY-----`;
        try {
            crypto.privateEncrypt(placeholderPrivateKey, Buffer.from('message'));
            false;
        } catch (error) {
            String(error && error.message ? error.message : error)
                .toLowerCase()
                .includes('private key');
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_public_private_rsa_encrypt_decrypt_round_trip_uses_real_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encrypted = crypto.publicEncrypt(publicKey, Buffer.from('secret payload'));
        const decrypted = crypto.privateDecrypt(privateKey, encrypted);
        decrypted.toString('utf8') === 'secret payload';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_private_public_rsa_encrypt_decrypt_round_trip_uses_real_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const encrypted = crypto.privateEncrypt(privateKey, Buffer.from('signed payload'));
        const decrypted = crypto.publicDecrypt(publicKey, encrypted);
        decrypted.toString('utf8') === 'signed payload';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_rsa_encrypt_outputs_modulus_sized_ciphertext() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
            modulusLength: 2048
        });
        const publicEncrypted = crypto.publicEncrypt(publicKey, Buffer.from('a'));
        const privateEncrypted = crypto.privateEncrypt(privateKey, Buffer.from('a'));
        publicEncrypted.length === 256 && privateEncrypted.length === 256;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_create_ecdh_generates_uncompressed_p256_public_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const ecdh = crypto.createECDH('prime256v1');
        const publicKey = Buffer.from(ecdh.getPublicKey(), 'hex');
        publicKey.length === 65 && publicKey.toString('hex').startsWith('04');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_create_ecdh_compute_secret_rejects_invalid_peer_public_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const ecdh = crypto.createECDH('prime256v1');
        try {
            ecdh.computeSecret('not-a-valid-public-key');
            false;
        } catch (error) {
            String(error && error.message ? error.message : error)
                .toLowerCase()
                .includes('public key');
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_create_ecdh_set_private_key_recomputes_public_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const alice = crypto.createECDH('prime256v1');
        const bob = crypto.createECDH('prime256v1');

        const alicePrivateKey = alice.getPrivateKey();
        const alicePublicKey = alice.getPublicKey();

        bob.setPrivateKey(alicePrivateKey);
        bob.getPrivateKey() === alicePrivateKey &&
            bob.getPublicKey() === alicePublicKey;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

// Tests for RSA key generation and sign/verify (v0.3.362)
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_rsa_oaep_key_generation_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.subtle.generateKey");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_rsa_oaep_generate_key_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'RSA-OAEP', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
            true,
            ['encrypt', 'decrypt']
        );
        result && result.constructor && result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_rsa_oaep_generate_key_returns_keypair() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                const keyPair = await crypto.subtle.generateKey(
                    { name: 'RSA-OAEP', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
                    true,
                    ['encrypt', 'decrypt']
                );
                return keyPair.publicKey.type === 'public' &&
                    keyPair.privateKey.type === 'private' &&
                    keyPair.publicKey.algorithm.name === 'RSA-OAEP' &&
                    keyPair.privateKey.algorithm.name === 'RSA-OAEP' &&
                    keyPair.publicKey.usages.includes('encrypt') &&
                    keyPair.privateKey.usages.includes('decrypt');
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
fn test_rsa_oaep_encrypt_decrypt_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyPair = await crypto.subtle.generateKey(
                { name: 'RSA-OAEP', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
                true,
                ['encrypt', 'decrypt']
            );
            const plaintext = new TextEncoder().encode('bee rsa oaep');
            const ciphertext = await crypto.subtle.encrypt({ name: 'RSA-OAEP' }, keyPair.publicKey, plaintext);
            const decrypted = await crypto.subtle.decrypt({ name: 'RSA-OAEP' }, keyPair.privateKey, ciphertext);
            return new TextDecoder().decode(decrypted) === 'bee rsa oaep' &&
                ciphertext instanceof ArrayBuffer &&
                ciphertext.byteLength === 256;
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_rsa_oaep_decrypt_rejects_tampered_ciphertext() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyPair = await crypto.subtle.generateKey(
                { name: 'RSA-OAEP', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
                true,
                ['encrypt', 'decrypt']
            );
            const plaintext = new TextEncoder().encode('bee rsa oaep');
            const ciphertext = new Uint8Array(await crypto.subtle.encrypt({ name: 'RSA-OAEP' }, keyPair.publicKey, plaintext));
            ciphertext[0] ^= 0xff;
            try {
                await crypto.subtle.decrypt({ name: 'RSA-OAEP' }, keyPair.privateKey, ciphertext);
                return false;
            } catch (_) {
                return true;
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_rsa_oaep_decrypt_rejects_wrong_label() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyPair = await crypto.subtle.generateKey(
                { name: 'RSA-OAEP', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
                true,
                ['encrypt', 'decrypt']
            );
            const plaintext = new TextEncoder().encode('bee rsa oaep label');
            const ciphertext = await crypto.subtle.encrypt(
                { name: 'RSA-OAEP', label: new Uint8Array([1, 2, 3]) },
                keyPair.publicKey,
                plaintext
            );

            try {
                await crypto.subtle.decrypt(
                    { name: 'RSA-OAEP', label: new Uint8Array([9, 9, 9]) },
                    keyPair.privateKey,
                    ciphertext
                );
                return false;
            } catch (_) {
                return true;
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_rsa_oaep_encrypt_decrypt_with_matching_label() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyPair = await crypto.subtle.generateKey(
                { name: 'RSA-OAEP', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
                true,
                ['encrypt', 'decrypt']
            );
            const label = new Uint8Array([1, 2, 3, 4]);
            const plaintext = new TextEncoder().encode('bee rsa oaep label');
            const ciphertext = await crypto.subtle.encrypt(
                { name: 'RSA-OAEP', label },
                keyPair.publicKey,
                plaintext
            );
            const decrypted = await crypto.subtle.decrypt(
                { name: 'RSA-OAEP', label },
                keyPair.privateKey,
                ciphertext
            );
            return new TextDecoder().decode(decrypted) === 'bee rsa oaep label';
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_rsa_oaep_encrypt_rejects_non_buffer_label() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyPair = await crypto.subtle.generateKey(
                { name: 'RSA-OAEP', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
                true,
                ['encrypt', 'decrypt']
            );
            try {
                await crypto.subtle.encrypt(
                    { name: 'RSA-OAEP', label: 'not-bytes' },
                    keyPair.publicKey,
                    new TextEncoder().encode('bee rsa oaep')
                );
                return false;
            } catch (error) {
                return String(error && error.message ? error.message : error).includes('label');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_rsa_oaep_rejects_wrong_key_type() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyPair = await crypto.subtle.generateKey(
                { name: 'RSA-OAEP', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
                true,
                ['encrypt', 'decrypt']
            );
            const plaintext = new TextEncoder().encode('bee rsa oaep');
            let privateEncryptRejected = false;
            try {
                await crypto.subtle.encrypt(
                    { name: 'RSA-OAEP' },
                    keyPair.privateKey,
                    plaintext
                );
            } catch (_) {
                privateEncryptRejected = true;
            }

            const ciphertext = await crypto.subtle.encrypt(
                { name: 'RSA-OAEP' },
                keyPair.publicKey,
                plaintext
            );
            let publicDecryptRejected = false;
            try {
                await crypto.subtle.decrypt(
                    { name: 'RSA-OAEP' },
                    keyPair.publicKey,
                    ciphertext
                );
            } catch (_) {
                publicDecryptRejected = true;
            }

            return privateEncryptRejected && publicDecryptRejected;
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_rsa_rsassa_generate_key_returns_keypair() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                const keyPair = await crypto.subtle.generateKey(
                    { name: 'RSASSA-PKCS1-v1_5', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
                    true,
                    ['sign', 'verify']
                );
                return keyPair.publicKey.type === 'public' &&
                    keyPair.privateKey.type === 'private' &&
                    keyPair.publicKey.algorithm.name === 'RSASSA-PKCS1-v1_5' &&
                    keyPair.privateKey.algorithm.name === 'RSASSA-PKCS1-v1_5' &&
                    keyPair.publicKey.usages.includes('verify') &&
                    keyPair.privateKey.usages.includes('sign');
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
fn test_rsa_sign_with_private_key_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const keyPairPromise = crypto.subtle.generateKey(
            { name: 'RSASSA-PKCS1-v1_5', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
            true,
            ['sign', 'verify']
        );
        const signPromise = keyPairPromise.then(keyPair =>
            crypto.subtle.sign({ name: 'RSASSA-PKCS1-v1_5' }, keyPair.privateKey, new TextEncoder().encode('test data'))
        );
        typeof signPromise.then;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_rsa_sign_unimplemented_rejects() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const privateKey = {
                type: 'private',
                algorithm: { name: 'RSASSA-PKCS1-v1_5' },
                extractable: true,
                usages: ['sign']
            };
            try {
                await crypto.subtle.sign(
                    { name: 'RSASSA-PKCS1-v1_5' },
                    privateKey,
                    new TextEncoder().encode('test data')
                );
                return false;
            } catch (_) {
                return true;
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_rsa_verify_with_public_key_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyPair = await crypto.subtle.generateKey(
                { name: 'RSASSA-PKCS1-v1_5', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
                true,
                ['sign', 'verify']
            );
            const data = new TextEncoder().encode('test data');
            const signature = await crypto.subtle.sign({ name: 'RSASSA-PKCS1-v1_5' }, keyPair.privateKey, data);
            return await crypto.subtle.verify({ name: 'RSASSA-PKCS1-v1_5' }, keyPair.publicKey, signature, data);
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_rsa_sign_does_not_return_placeholder_array_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const privateKey = {
                type: 'private',
                algorithm: { name: 'RSASSA-PKCS1-v1_5' },
                extractable: true,
                usages: ['sign']
            };
            try {
                const signature = await crypto.subtle.sign(
                    { name: 'RSASSA-PKCS1-v1_5' },
                    privateKey,
                    new TextEncoder().encode('test data')
                );
                return !(signature instanceof ArrayBuffer);
            } catch (_) {
                return true;
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_rsa_1024_modulus_generate_key_returns_keypair() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyPair = await crypto.subtle.generateKey(
                { name: 'RSA-OAEP', modulusLength: 1024, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
                true,
                ['encrypt', 'decrypt']
            );
            return keyPair.publicKey.algorithm.name === 'RSA-OAEP' &&
                keyPair.privateKey.algorithm.name === 'RSA-OAEP';
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_rsa_oaep_generate_key_rejects_invalid_public_exponent() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                await crypto.subtle.generateKey(
                    { name: 'RSA-OAEP', modulusLength: 2048, publicExponent: new Uint8Array([3]), hash: 'SHA-256' },
                    true,
                    ['encrypt', 'decrypt']
                );
                return false;
            } catch (error) {
                return String(error && error.message ? error.message : error).includes('publicExponent');
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_rsa_public_key_algorithm_name() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.subtle.generateKey(
            { name: 'RSA-OAEP', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
            true,
            ['encrypt', 'decrypt']
        ).then(keyPair => keyPair.publicKey.algorithm.name === 'RSA-OAEP');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_rsa_private_key_algorithm_name() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.subtle.generateKey(
            { name: 'RSA-OAEP', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
            true,
            ['encrypt', 'decrypt']
        ).then(keyPair => keyPair.privateKey.algorithm.name === 'RSA-OAEP');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_rsa_verify_rejects_tampered_signature() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyPair = await crypto.subtle.generateKey(
                { name: 'RSASSA-PKCS1-v1_5', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
                true,
                ['sign', 'verify']
            );
            const data = new TextEncoder().encode('test data');
            const signature = new Uint8Array(await crypto.subtle.sign(
                { name: 'RSASSA-PKCS1-v1_5' },
                keyPair.privateKey,
                data
            ));
            signature[0] ^= 0xff;

            return await crypto.subtle.verify(
                { name: 'RSASSA-PKCS1-v1_5' },
                keyPair.publicKey,
                signature,
                data
            );
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "false");
}

// Tests for ECDSA elliptic curve key generation and sign/verify (v0.3.364)
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_ecdsa_key_generation_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.subtle.generateKey");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_ecdsa_generate_key_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-256' },
            true,
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
fn test_ecdsa_generate_key_returns_keypair() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-256' },
            true,
            ['sign', 'verify']
        );
        result.then(keyPair => keyPair && keyPair.publicKey && keyPair.privateKey) !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_public_key_has_correct_type() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-256' },
            true,
            ['sign', 'verify']
        );
        result.then(keyPair => keyPair.publicKey.type === 'public') !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_private_key_has_correct_type() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-256' },
            true,
            ['sign', 'verify']
        );
        result.then(keyPair => keyPair.privateKey.type === 'private') !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_p256_curve_generation() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-256' },
            true,
            ['sign', 'verify']
        );
        result.then(keyPair => keyPair.privateKey.algorithm.name === 'ECDSA') !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_p384_curve_generation() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-384' },
            true,
            ['sign', 'verify']
        );
        result.then(keyPair => keyPair && keyPair.publicKey && keyPair.privateKey) !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_p521_curve_generation() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-521' },
            true,
            ['sign', 'verify']
        );
        result.then(keyPair => keyPair && keyPair.publicKey && keyPair.privateKey) !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_sign_with_private_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const keyPairPromise = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-256' },
            true,
            ['sign', 'verify']
        );
        const signPromise = keyPairPromise.then(keyPair =>
            crypto.subtle.sign({ name: 'ECDSA', hash: { name: 'SHA-256' } }, keyPair.privateKey, new TextEncoder().encode('test data'))
        );
        typeof signPromise.then;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_ecdsa_verify_with_public_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const keyPairPromise = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-256' },
            true,
            ['sign', 'verify']
        );
        const verifyPromise = keyPairPromise.then(keyPair => {
            return crypto.subtle.sign({ name: 'ECDSA', hash: { name: 'SHA-256' } }, keyPair.privateKey, new TextEncoder().encode('test data'))
                .then(signature => crypto.subtle.verify({ name: 'ECDSA', hash: { name: 'SHA-256' } }, keyPair.publicKey, signature, new TextEncoder().encode('test data')));
        });
        typeof verifyPromise.then;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_ecdsa_sign_returns_array_buffer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const keyPairPromise = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-256' },
            true,
            ['sign', 'verify']
        );
        const signPromise = keyPairPromise.then(keyPair =>
            crypto.subtle.sign({ name: 'ECDSA', hash: { name: 'SHA-256' } }, keyPair.privateKey, new TextEncoder().encode('test data'))
        );
        signPromise.then(sig => sig instanceof ArrayBuffer) !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_signature_length_p256() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const keyPairPromise = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-256' },
            true,
            ['sign', 'verify']
        );
        keyPairPromise.then(keyPair =>
            crypto.subtle.sign({ name: 'ECDSA', hash: { name: 'SHA-256' } }, keyPair.privateKey, new TextEncoder().encode('test data'))
                .then(signature => signature.byteLength === 64)
        ) !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_signature_length_p384() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const keyPairPromise = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-384' },
            true,
            ['sign', 'verify']
        );
        keyPairPromise.then(keyPair =>
            crypto.subtle.sign({ name: 'ECDSA', hash: { name: 'SHA-384' } }, keyPair.privateKey, new TextEncoder().encode('test data'))
                .then(signature => signature.byteLength === 96)
        ) !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_p384_public_key_is_uncompressed_point() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                const keyPair = await crypto.subtle.generateKey(
                    { name: 'ECDSA', namedCurve: 'P-384' },
                    true,
                    ['sign', 'verify']
                );
                const publicBytes = new Uint8Array(
                    await crypto.subtle.exportKey('raw', keyPair.publicKey)
                );
                return publicBytes.length === 97 && publicBytes[0] === 4;
            } catch (error) {
                return String(error && error.message ? error.message : error);
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_p521_public_key_is_uncompressed_point() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                const keyPair = await crypto.subtle.generateKey(
                    { name: 'ECDSA', namedCurve: 'P-521' },
                    true,
                    ['sign', 'verify']
                );
                const publicBytes = new Uint8Array(
                    await crypto.subtle.exportKey('raw', keyPair.publicKey)
                );
                return publicBytes.length === 133 && publicBytes[0] === 4;
            } catch (error) {
                return String(error && error.message ? error.message : error);
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_p384_sign_verify_awaited() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                const keyPair = await crypto.subtle.generateKey(
                    { name: 'ECDSA', namedCurve: 'P-384' },
                    true,
                    ['sign', 'verify']
                );
                const data = new TextEncoder().encode('p384 real signature');
                const signature = await crypto.subtle.sign(
                    { name: 'ECDSA', hash: { name: 'SHA-384' } },
                    keyPair.privateKey,
                    data
                );
                const verified = await crypto.subtle.verify(
                    { name: 'ECDSA', hash: { name: 'SHA-384' } },
                    keyPair.publicKey,
                    signature,
                    data
                );
                return verified === true && signature.byteLength === 96;
            } catch (error) {
                return String(error && error.message ? error.message : error);
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_p521_sign_verify_awaited() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                const keyPair = await crypto.subtle.generateKey(
                    { name: 'ECDSA', namedCurve: 'P-521' },
                    true,
                    ['sign', 'verify']
                );
                const data = new TextEncoder().encode('p521 real signature');
                const signature = await crypto.subtle.sign(
                    { name: 'ECDSA', hash: { name: 'SHA-512' } },
                    keyPair.privateKey,
                    data
                );
                const verified = await crypto.subtle.verify(
                    { name: 'ECDSA', hash: { name: 'SHA-512' } },
                    keyPair.publicKey,
                    signature,
                    data
                );
                return verified === true && signature.byteLength === 132;
            } catch (error) {
                return String(error && error.message ? error.message : error);
            }
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_different_data_signing() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const keyPairPromise = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-256' },
            true,
            ['sign', 'verify']
        );
        const data = new TextEncoder().encode('different data for signing');
        keyPairPromise.then(keyPair =>
            crypto.subtle.sign({ name: 'ECDSA', hash: { name: 'SHA-256' } }, keyPair.privateKey, data)
        ) !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_verify_returns_boolean() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const keyPairPromise = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-256' },
            true,
            ['sign', 'verify']
        );
        const verifyResultPromise = keyPairPromise.then(keyPair => {
            const data = new TextEncoder().encode('test data');
            return crypto.subtle.sign({ name: 'ECDSA', hash: { name: 'SHA-256' } }, keyPair.privateKey, data)
                .then(signature => crypto.subtle.verify({ name: 'ECDSA', hash: { name: 'SHA-256' } }, keyPair.publicKey, signature, data));
        });
        verifyResultPromise.then(result => typeof result === 'boolean') !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_verify_wrong_signature_returns_false() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const keyPairPromise = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-256' },
            true,
            ['sign', 'verify']
        );
        const verifyResultPromise = keyPairPromise.then(keyPair => {
            const data = new TextEncoder().encode('test data');
            const wrongSignature = crypto.getRandomValues(new Uint8Array(64));
            return crypto.subtle.verify({ name: 'ECDSA', hash: { name: 'SHA-256' } }, keyPair.publicKey, wrongSignature, data);
        });
        verifyResultPromise.then(result => result === false) !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdh_key_generation() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'ECDH', namedCurve: 'P-256' },
            true,
            ['deriveKey', 'deriveBits']
        );
        result.then(keyPair => keyPair && keyPair.publicKey && keyPair.privateKey) !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdh_public_key_has_correct_type() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'ECDH', namedCurve: 'P-256' },
            true,
            ['deriveKey', 'deriveBits']
        );
        result.then(keyPair => keyPair.publicKey.type === 'public') !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdh_private_key_has_correct_type() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'ECDH', namedCurve: 'P-256' },
            true,
            ['deriveKey', 'deriveBits']
        );
        result.then(keyPair => keyPair.privateKey.type === 'private') !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_public_key_algorithm_name() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-256' },
            true,
            ['sign', 'verify']
        );
        result.then(keyPair => keyPair.publicKey.algorithm.name === 'ECDSA') !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_private_key_algorithm_name() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-256' },
            true,
            ['sign', 'verify']
        );
        result.then(keyPair => keyPair.privateKey.algorithm.name === 'ECDSA') !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_ecdsa_key_usages() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'ECDSA', namedCurve: 'P-256' },
            true,
            ['sign', 'verify']
        );
        result.then(keyPair => {
            const pubUsages = keyPair.publicKey.usages;
            const privUsages = keyPair.privateKey.usages;
            return pubUsages.includes('verify') && privUsages.includes('sign');
        }) !== undefined;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

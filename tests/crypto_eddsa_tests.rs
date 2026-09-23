use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_generate_key_pair_sync_ed25519_sign_verify_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('ed25519', {
            publicKeyEncoding: { type: 'spki', format: 'pem' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        });
        const signature = crypto.sign(null, 'hello ed25519', privateKey);
        const valid = crypto.verify(null, 'hello ed25519', publicKey, signature);
        const invalid = crypto.verify(null, 'tampered', publicKey, signature);
        publicKey.includes('-----BEGIN PUBLIC KEY-----') &&
            privateKey.includes('-----BEGIN PRIVATE KEY-----') &&
            signature && signature.length === 64 &&
            valid === true &&
            invalid === false;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_ed25519_callback_sign_verify_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        let result = false;
        crypto.generateKeyPair('ed25519', {
            publicKeyEncoding: { type: 'spki', format: 'pem' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        }, (err, publicKey, privateKey) => {
            if (err) {
                result = err.message;
                return;
            }
            const signature = crypto.sign(null, 'async ed25519', privateKey);
            result = crypto.verify(null, 'async ed25519', publicKey, signature) === true &&
                crypto.verify(null, 'changed', publicKey, signature) === false;
        });
        result;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_ed448_sign_verify_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const { publicKey, privateKey } = crypto.generateKeyPairSync('ed448', {
            publicKeyEncoding: { type: 'spki', format: 'pem' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        });
        const signature = crypto.sign(null, 'hello ed448', privateKey);
        const valid = crypto.verify(null, 'hello ed448', publicKey, signature);
        const invalid = crypto.verify(null, 'tampered ed448', publicKey, signature);
        publicKey.includes('-----BEGIN PUBLIC KEY-----') &&
            privateKey.includes('-----BEGIN PRIVATE KEY-----') &&
            signature && signature.length === 114 &&
            valid === true &&
            invalid === false;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_ed448_callback_sign_verify_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        let result = false;
        crypto.generateKeyPair('ed448', {
            publicKeyEncoding: { type: 'spki', format: 'pem' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        }, (err, publicKey, privateKey) => {
            if (err) {
                result = err.message;
                return;
            }
            const signature = crypto.sign(null, 'async ed448', privateKey);
            result = signature.length === 114 &&
                crypto.verify(null, 'async ed448', publicKey, signature) === true &&
                crypto.verify(null, 'changed ed448', publicKey, signature) === false;
        });
        result;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_create_key_object_reports_eddsa_asymmetric_key_type() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const ed25519 = crypto.generateKeyPairSync('ed25519', {
            publicKeyEncoding: { type: 'spki', format: 'pem' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        });
        const ed448 = crypto.generateKeyPairSync('ed448', {
            publicKeyEncoding: { type: 'spki', format: 'pem' },
            privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
        });

        const ed25519Private = crypto.createPrivateKey(ed25519.privateKey);
        const ed25519Public = crypto.createPublicKey(ed25519.publicKey);
        const ed448Private = crypto.createPrivateKey(ed448.privateKey);
        const ed448Public = crypto.createPublicKey(ed448.publicKey);

        ed25519Private.asymmetricKeyType === 'ed25519' &&
            ed25519Public.asymmetricKeyType === 'ed25519' &&
            ed448Private.asymmetricKeyType === 'ed448' &&
            ed448Public.asymmetricKeyType === 'ed448';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_generate_key_pair_sync_eddsa_jwk_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        let ok = true;
        for (const [type, crv, xLength, signatureLength] of [
            ['ed25519', 'Ed25519', 43, 64],
            ['ed448', 'Ed448', 76, 114]
        ]) {
            const { publicKey, privateKey } = crypto.generateKeyPairSync(type, {
                publicKeyEncoding: { format: 'jwk' },
                privateKeyEncoding: { format: 'jwk' }
            });
            const importedPrivateKey = crypto.createPrivateKey({ key: privateKey, format: 'jwk' });
            const importedPublicKey = crypto.createPublicKey({ key: publicKey, format: 'jwk' });
            const exportedPrivate = importedPrivateKey.export({ format: 'jwk' });
            const exportedPublic = importedPublicKey.export({ format: 'jwk' });
            const signature = crypto.sign(null, `${type} jwk`, importedPrivateKey);
            ok = ok &&
                publicKey.kty === 'OKP' &&
                publicKey.crv === crv &&
                typeof publicKey.x === 'string' &&
                publicKey.x.length === xLength &&
                publicKey.d === undefined &&
                privateKey.kty === 'OKP' &&
                privateKey.crv === crv &&
                privateKey.x === publicKey.x &&
                typeof privateKey.d === 'string' &&
                privateKey.d.length === xLength &&
                !privateKey.x.includes('=') &&
                !privateKey.d.includes('=') &&
                importedPrivateKey.asymmetricKeyType === type &&
                importedPublicKey.asymmetricKeyType === type &&
                exportedPrivate.kty === 'OKP' &&
                exportedPrivate.crv === crv &&
                exportedPrivate.x === privateKey.x &&
                exportedPrivate.d === privateKey.d &&
                exportedPublic.kty === 'OKP' &&
                exportedPublic.crv === crv &&
                exportedPublic.x === publicKey.x &&
                exportedPublic.d === undefined &&
                signature.length === signatureLength &&
                crypto.verify(null, `${type} jwk`, importedPublicKey, signature) === true &&
                crypto.verify(null, `${type} changed`, importedPublicKey, signature) === false;
        }
        ok;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_subtle_ed25519_generate_sign_verify_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                const keyPair = await crypto.subtle.generateKey(
                    { name: 'Ed25519' },
                    true,
                    ['sign', 'verify']
                );
                const data = new TextEncoder().encode('webcrypto ed25519');
                const signature = await crypto.subtle.sign(
                    { name: 'Ed25519' },
                    keyPair.privateKey,
                    data
                );
                const valid = await crypto.subtle.verify(
                    { name: 'Ed25519' },
                    keyPair.publicKey,
                    signature,
                    data
                );
                const invalid = await crypto.subtle.verify(
                    { name: 'Ed25519' },
                    keyPair.publicKey,
                    signature,
                    new TextEncoder().encode('webcrypto ed25519 changed')
                );
                return keyPair.privateKey.algorithm.name === 'Ed25519' &&
                    keyPair.publicKey.algorithm.name === 'Ed25519' &&
                    keyPair.privateKey.type === 'private' &&
                    keyPair.publicKey.type === 'public' &&
                    keyPair.privateKey.usages.join(',') === 'sign' &&
                    keyPair.publicKey.usages.join(',') === 'verify' &&
                    signature.byteLength === 64 &&
                    valid === true &&
                    invalid === false;
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
fn test_subtle_ed448_generate_sign_verify_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                const keyPair = await crypto.subtle.generateKey(
                    { name: 'Ed448' },
                    true,
                    ['sign', 'verify']
                );
                const data = new TextEncoder().encode('webcrypto ed448');
                const signature = await crypto.subtle.sign(
                    { name: 'Ed448' },
                    keyPair.privateKey,
                    data
                );
                const valid = await crypto.subtle.verify(
                    { name: 'Ed448' },
                    keyPair.publicKey,
                    signature,
                    data
                );
                const invalid = await crypto.subtle.verify(
                    { name: 'Ed448' },
                    keyPair.publicKey,
                    signature,
                    new TextEncoder().encode('webcrypto ed448 changed')
                );
                return keyPair.privateKey.algorithm.name === 'Ed448' &&
                    keyPair.publicKey.algorithm.name === 'Ed448' &&
                    keyPair.privateKey.type === 'private' &&
                    keyPair.publicKey.type === 'public' &&
                    keyPair.privateKey.usages.join(',') === 'sign' &&
                    keyPair.publicKey.usages.join(',') === 'verify' &&
                    signature.byteLength === 114 &&
                    valid === true &&
                    invalid === false;
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
fn test_subtle_eddsa_raw_public_and_jwk_private_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const cases = [
                ['Ed25519', 32, 43, 64],
                ['Ed448', 57, 76, 114]
            ];

            for (const [name, rawLen, jwkLen, sigLen] of cases) {
                try {
                    const keyPair = await crypto.subtle.generateKey(
                        { name },
                        true,
                        ['sign', 'verify']
                    );
                    const rawPublic = new Uint8Array(await crypto.subtle.exportKey('raw', keyPair.publicKey));
                    const publicJwk = await crypto.subtle.exportKey('jwk', keyPair.publicKey);
                    const privateJwk = await crypto.subtle.exportKey('jwk', keyPair.privateKey);
                    const rawPrivateRejected = await crypto.subtle.exportKey('raw', keyPair.privateKey)
                        .then(() => false, (error) => String(error && error.message ? error.message : error).includes('raw'));

                    const importedPublicRaw = await crypto.subtle.importKey(
                        'raw',
                        rawPublic,
                        { name },
                        true,
                        ['verify']
                    );
                    const importedPublicJwk = await crypto.subtle.importKey(
                        'jwk',
                        publicJwk,
                        { name },
                        true,
                        ['verify']
                    );
                    const importedPrivateJwk = await crypto.subtle.importKey(
                        'jwk',
                        privateJwk,
                        { name },
                        true,
                        ['sign']
                    );

                    const data = new TextEncoder().encode(`${name} jwk/raw webcrypto`);
                    const signature = await crypto.subtle.sign({ name }, importedPrivateJwk, data);
                    const validRaw = await crypto.subtle.verify({ name }, importedPublicRaw, signature, data);
                    const validJwk = await crypto.subtle.verify({ name }, importedPublicJwk, signature, data);
                    const invalid = await crypto.subtle.verify(
                        { name },
                        importedPublicJwk,
                        signature,
                        new TextEncoder().encode(`${name} changed`)
                    );

                    const ok = rawPublic.length === rawLen &&
                        rawPrivateRejected === true &&
                        publicJwk.kty === 'OKP' &&
                        publicJwk.crv === name &&
                        publicJwk.alg === name &&
                        publicJwk.x.length === jwkLen &&
                        publicJwk.d === undefined &&
                        publicJwk.ext === true &&
                        publicJwk.key_ops.join(',') === 'verify' &&
                        privateJwk.kty === 'OKP' &&
                        privateJwk.crv === name &&
                        privateJwk.alg === name &&
                        privateJwk.x === publicJwk.x &&
                        privateJwk.d.length === jwkLen &&
                        privateJwk.ext === true &&
                        privateJwk.key_ops.join(',') === 'sign' &&
                        !publicJwk.x.includes('=') &&
                        !privateJwk.d.includes('=') &&
                        importedPublicRaw.algorithm.name === name &&
                        importedPublicJwk.algorithm.name === name &&
                        importedPrivateJwk.algorithm.name === name &&
                        importedPublicRaw.type === 'public' &&
                        importedPublicJwk.type === 'public' &&
                        importedPrivateJwk.type === 'private' &&
                        signature.byteLength === sigLen &&
                        validRaw === true &&
                        validJwk === true &&
                        invalid === false;

                    if (!ok) {
                        return JSON.stringify({
                            name,
                            rawPublicLength: rawPublic.length,
                            rawPrivateRejected,
                            publicJwk,
                            privateJwk,
                            importedPublicRaw: {
                                type: importedPublicRaw.type,
                                algorithm: importedPublicRaw.algorithm
                            },
                            importedPublicJwk: {
                                type: importedPublicJwk.type,
                                algorithm: importedPublicJwk.algorithm
                            },
                            importedPrivateJwk: {
                                type: importedPrivateJwk.type,
                                algorithm: importedPrivateJwk.algorithm
                            },
                            signatureLength: signature.byteLength,
                            validRaw,
                            validJwk,
                            invalid
                        });
                    }
                } catch (error) {
                    return `${name}: ${String(error && error.message ? error.message : error)}`;
                }
            }

            return 'true';
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_eddsa_spki_pkcs8_round_trip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const cases = [
                ['Ed25519', 44, 48, 64],
                ['Ed448', 69, 73, 114]
            ];

            for (const [name, spkiLen, pkcs8Len, sigLen] of cases) {
                try {
                    const keyPair = await crypto.subtle.generateKey(
                        { name },
                        true,
                        ['sign', 'verify']
                    );
                    const spki = new Uint8Array(await crypto.subtle.exportKey('spki', keyPair.publicKey));
                    const pkcs8 = new Uint8Array(await crypto.subtle.exportKey('pkcs8', keyPair.privateKey));
                    const publicPkcs8Rejected = await crypto.subtle.exportKey('pkcs8', keyPair.publicKey)
                        .then(() => false, (error) => String(error && error.message ? error.message : error).includes('pkcs8'));
                    const privateSpkiRejected = await crypto.subtle.exportKey('spki', keyPair.privateKey)
                        .then(() => false, (error) => String(error && error.message ? error.message : error).includes('spki'));

                    const importedPublic = await crypto.subtle.importKey(
                        'spki',
                        spki,
                        { name },
                        true,
                        ['verify']
                    );
                    const importedPrivate = await crypto.subtle.importKey(
                        'pkcs8',
                        pkcs8,
                        { name },
                        true,
                        ['sign']
                    );

                    const data = new TextEncoder().encode(`${name} spki pkcs8 webcrypto`);
                    const signature = await crypto.subtle.sign({ name }, importedPrivate, data);
                    const valid = await crypto.subtle.verify({ name }, importedPublic, signature, data);
                    const invalid = await crypto.subtle.verify(
                        { name },
                        importedPublic,
                        signature,
                        new TextEncoder().encode(`${name} spki pkcs8 changed`)
                    );

                    const ok = spki.length === spkiLen &&
                        pkcs8.length === pkcs8Len &&
                        publicPkcs8Rejected === true &&
                        privateSpkiRejected === true &&
                        importedPublic.type === 'public' &&
                        importedPrivate.type === 'private' &&
                        importedPublic.algorithm.name === name &&
                        importedPrivate.algorithm.name === name &&
                        importedPublic.usages.join(',') === 'verify' &&
                        importedPrivate.usages.join(',') === 'sign' &&
                        signature.byteLength === sigLen &&
                        valid === true &&
                        invalid === false;

                    if (!ok) {
                        return JSON.stringify({
                            name,
                            spkiLength: spki.length,
                            pkcs8Length: pkcs8.length,
                            publicPkcs8Rejected,
                            privateSpkiRejected,
                            importedPublic: {
                                type: importedPublic.type,
                                algorithm: importedPublic.algorithm,
                                usages: importedPublic.usages
                            },
                            importedPrivate: {
                                type: importedPrivate.type,
                                algorithm: importedPrivate.algorithm,
                                usages: importedPrivate.usages
                            },
                            signatureLength: signature.byteLength,
                            valid,
                            invalid
                        });
                    }
                } catch (error) {
                    return `${name}: ${String(error && error.message ? error.message : error)}`;
                }
            }

            return 'true';
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Execution failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "true");
}

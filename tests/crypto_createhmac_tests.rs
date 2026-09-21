// Tests for crypto.createHmac module (v0.3.9)
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_crypto_module_exists_hmac() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_create_hmac_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.createHmac");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_create_hmac_md5() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hmac = crypto.createHmac('md5', 'secret_key');
        hmac.update('hello');
        hmac.digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // HMAC-MD5("hello", "secret_key") - 32 hex chars
    assert_eq!(output.len(), 32);
}

#[test]
#[serial]
fn test_create_hmac_md5_known_vector() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hmac = crypto.createHmac('md5', 'key');
        hmac.update('The quick brown fox jumps over the lazy dog');
        hmac.digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "80070713463e7749b90c2dc24911e275");
}

#[test]
#[serial]
fn test_create_hmac_sha256() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hmac = crypto.createHmac('sha256', 'key');
        hmac.update('The quick brown fox jumps over the lazy dog');
        hmac.digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
    );
}

#[test]
#[serial]
fn test_hmac_update_accepts_uint8array_bytes() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto
          .createHmac('sha256', 'key')
          .update(new Uint8Array([0, 255, 1, 2, 128]))
          .digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "013a2441d7c8c365cf193c8448ee4831cfd6e67202863397b82b03443f418642"
    );
}

#[test]
#[serial]
fn test_hmac_supports_base64url_digest_and_update_input() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const bytes = new Uint8Array([0, 255, 1, 2, 128]);
        [
          crypto.createHmac('sha256', 'key').update(bytes).digest('base64url'),
          crypto.createHmac('sha256', 'key').update('AP8BAoA', 'base64url').digest('hex')
        ].join('\n');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "ATokQdfIw2XPGTyESO5IMc_W5nIChjOXuCsDRD9BhkI\n013a2441d7c8c365cf193c8448ee4831cfd6e67202863397b82b03443f418642"
    );
}

#[test]
#[serial]
fn test_hmac_accepts_uint8array_key_bytes() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto
          .createHmac('sha256', new Uint8Array([0, 255, 1, 2, 128]))
          .update('payload')
          .digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "349c0ebe857177913fe101c37e0de3fac145aa8c9f290c7cc9beeeb91f8cd439"
    );
}

#[test]
#[serial]
fn test_hmac_string_key_respects_encoding_option() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto
          .createHmac('sha256', 'ff', { encoding: 'hex' })
          .update('payload')
          .digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "0e7440983f74f82fecafc1e093b28982c3470b8d1f2bbe48aa69ac6000e3d1cc"
    );
}

#[test]
#[serial]
fn test_create_hmac_sha1_known_vector() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hmac = crypto.createHmac('sha1', 'key');
        hmac.update('The quick brown fox jumps over the lazy dog');
        hmac.digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "de7c9b85b8b78aa6bc8a7a36f70a90701c9db4d9"
    );
}

#[test]
#[serial]
fn test_create_hmac_sha384_and_alias_known_vector() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        [
          crypto.createHmac('sha384', 'key').update('hello').digest('hex'),
          crypto.createHmac('SHA-384', 'key').update('hello').digest('hex')
        ].join('\n');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "eacbad575c301fa68afb26dae48b25bf5cd42fd08ed28c08c274ce62df7928f01249976cd8aaf1ab0681d3accedc9543\neacbad575c301fa68afb26dae48b25bf5cd42fd08ed28c08c274ce62df7928f01249976cd8aaf1ab0681d3accedc9543"
    );
}

#[test]
#[serial]
fn test_create_hmac_sha512() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hmac = crypto.createHmac('sha512', 'key');
        hmac.update('data');
        hmac.digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // SHA512 produces 128 hex chars
    assert_eq!(output.len(), 128);
}

#[test]
#[serial]
fn test_create_hmac_sha512_known_vector() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hmac = crypto.createHmac('sha512', 'key');
        hmac.update('The quick brown fox jumps over the lazy dog');
        hmac.digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "b42af09057bac1e2d41708e48a902e09b5ff7f12ab428a4fe86653c73dd248fb82f948a549f7b791a5b41915ee4d1ec3935357e4e2317250d0372afa2ebeeb3a"
    );
}

#[test]
#[serial]
fn test_create_hmac_blake3() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hmac = crypto.createHmac('blake3', 'secret');
        hmac.update('test');
        hmac.digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // BLAKE3 produces 64 hex chars
    assert_eq!(output.len(), 64);
}

#[test]
#[serial]
fn test_hmac_chain_update() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hmac = crypto.createHmac('md5', 'key');
        hmac.update('part1').update('part2');
        hmac.digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // HMAC-MD5("part1part2", "key") - 32 hex chars
    assert_eq!(output.len(), 32);
}

#[test]
#[serial]
fn test_hmac_chain_update_sha256_known_vector() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hmac = crypto.createHmac('sha256', 'key');
        hmac
          .update('The quick ')
          .update('brown fox jumps over ')
          .update('the lazy dog')
          .digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
    );
}

#[test]
#[serial]
fn test_hmac_base64_encoding() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hmac = crypto.createHmac('sha256', 'secret');
        hmac.update('hello');
        hmac.digest('base64');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "iKqz7ejTrflNJquQ07r9SiCDBww7zOnAFO4EpEOEfAs="
    );
}

#[test]
#[serial]
fn test_hmac_latin1_and_binary_encodings_return_digest_bytes() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        function byteStringToHex(value) {
          let hex = '';
          for (let i = 0; i < value.length; i++) {
            hex += (value.charCodeAt(i) & 0xff).toString(16).padStart(2, '0');
          }
          return value.length + ':' + hex;
        }

        const message = 'The quick brown fox jumps over the lazy dog';
        const latin1 = crypto.createHmac('sha256', 'key').update(message).digest('latin1');
        const binary = crypto.createHmac('sha256', 'key').update(message).digest('binary');
        [byteStringToHex(latin1), byteStringToHex(binary)].join('\n');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "32:f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8\n32:f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
    );
}

#[test]
#[serial]
fn test_hmac_digest_without_encoding_returns_binary_object() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const digest = crypto
          .createHmac('sha256', 'key')
          .update('The quick brown fox jumps over the lazy dog')
          .digest();
        const hex = Array.from(digest).map(b => b.toString(16).padStart(2, '0')).join('');
        [digest instanceof Uint8Array, digest.length, hex].join('\n');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "true\n32\nf7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
    );
}

#[test]
#[serial]
fn test_hmac_allows_empty_second_digest_but_rejects_update_after_digest() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hmac = crypto.createHmac('sha256', 'key');
        hmac.update('hello');
        hmac.digest('hex');

        const secondDigest = hmac.digest('hex');
        let updateError = '';
        try {
          hmac.update('again');
        } catch (error) {
          updateError = error.message;
        }

        [secondDigest.length, secondDigest, updateError].join('\n');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "0\n\nDigest already called");
}

#[test]
#[serial]
fn test_hmac_unsupported_algorithm() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            crypto.createHmac('unsupported', 'key');
        } catch (e) {
            e.message;
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim().contains("unsupported"));
}

#[test]
#[serial]
fn test_hmac_update_returns_hmac_object() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hmac = crypto.createHmac('md5', 'key');
        const result = hmac.update('test');
        typeof result;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_hmac_algorithm_property() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hmac = crypto.createHmac('sha256', 'key');
        hmac._algorithm;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "sha256");
}

#[test]
#[serial]
fn test_hmac_key_property() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hmac = crypto.createHmac('sha256', 'my_secret_key');
        hmac._key;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "my_secret_key");
}

#[test]
#[serial]
fn test_hmac_empty_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hmac = crypto.createHmac('md5', '');
        hmac.update('message');
        hmac.digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "b32797ea4b4b90a5d6fe744b33b97632");
}

#[test]
#[serial]
fn test_hmac_empty_key_standard_algorithms() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        [
          crypto.createHmac('sha1', '').update('message').digest('hex'),
          crypto.createHmac('sha256', '').update('message').digest('hex'),
          crypto.createHmac('sha512', '').update('message').digest('hex')
        ].join('\n');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "d5d1ed05121417247616cfc8378f360a39da7cfa\neb08c1f56d5ddee07f7bdf80468083da06b64cf4fac64fe3a90883df5feacae4\n08fce52f6395d59c2a3fb8abb281d74ad6f112b9a9c787bcea290d94dadbc82b2ca3e5e12bf2277c7fedbb0154d5493e41bb7459f63c8e39554ea3651b812492"
    );
}

#[test]
#[serial]
fn test_hmac_empty_message() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hmac = crypto.createHmac('md5', 'key');
        hmac.update('');
        hmac.digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "63530468a04e386459855da0063b6596");
}

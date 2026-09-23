// Tests for crypto.createHash module (v0.3.8)
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_crypto_module_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_create_hash_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.createHash");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_create_hash_md5() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hash = crypto.createHash('md5');
        hash.update('hello');
        hash.digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "5d41402abc4b2a76b9719d911017c592");
}

#[test]
#[serial]
fn test_create_hash_sha256() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hash = crypto.createHash('sha256');
        hash.update('hello');
        hash.digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
#[serial]
fn test_create_hash_sha384_known_vector() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto.createHash('sha384').update('hello').digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "59e1748777448c69de6b800d7a33bbfb9ff1b463e44354c3553bcdb9c666fa90125a3c79f90397bdf5f6a13de828684f"
    );
}

#[test]
#[serial]
fn test_create_hash_accepts_common_algorithm_aliases() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        [
          crypto.createHash('SHA-256').update('hello').digest('hex'),
          crypto.createHash('sha-384').update('hello').digest('hex')
        ].join('\n');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824\n59e1748777448c69de6b800d7a33bbfb9ff1b463e44354c3553bcdb9c666fa90125a3c79f90397bdf5f6a13de828684f"
    );
}

#[test]
#[serial]
fn test_hash_update_accepts_uint8array_bytes() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        crypto
          .createHash('sha256')
          .update(new Uint8Array([0, 255, 1, 2, 128]))
          .digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "da4abdcb7872ab2ba2f84282f11a6a39199ca49ecca78c6237505a7b87b8843f"
    );
}

#[test]
#[serial]
fn test_hash_supports_base64url_digest_and_update_input() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const bytes = new Uint8Array([0, 255, 1, 2, 128]);
        [
          crypto.createHash('sha256').update(bytes).digest('base64url'),
          crypto.createHash('sha256').update('AP8BAoA', 'base64url').digest('hex')
        ].join('\n');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "2kq9y3hyqyui-EKC8RpqORmcpJ7Mp4xiN1Bae4e4hD8\nda4abdcb7872ab2ba2f84282f11a6a39199ca49ecca78c6237505a7b87b8843f"
    );
}

#[test]
#[serial]
fn test_create_hash_sha1() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // SHA1 of "hello" should be aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d
    let code = r#"
        const hash = crypto.createHash('sha1');
        hash.update('hello');
        hash.digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
    );
}

#[test]
#[serial]
fn test_create_hash_sha512() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hash = crypto.createHash('sha512');
        hash.update('test');
        hash.digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // SHA512 of "test"
    assert_eq!(output.len(), 128); // 512 bits = 128 hex chars
}

#[test]
#[serial]
fn test_create_hash_blake3() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hash = crypto.createHash('blake3');
        hash.update('hello');
        hash.digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // BLAKE3 produces 32 bytes = 64 hex chars
    assert_eq!(output.len(), 64);
}

#[test]
#[serial]
fn test_hash_chain_update() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hash = crypto.createHash('md5');
        hash.update('hello').update('world');
        hash.digest('hex');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    // MD5 of "helloworld"
    assert_eq!(result.unwrap().trim(), "fc5e038d38a57032085441e7fe7010b0");
}

#[test]
#[serial]
fn test_hash_copy_clones_partial_state() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hash = crypto.createHash('sha256');
        hash.update('prefix');
        const copy = hash.copy();
        copy.update('-copy');
        hash.update('-orig');
        [hash.digest('hex'), copy.digest('hex')].join('\n');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "e61b0fbc0c8a792fc778a68e970fbc343e91d985800ed4fa9bdde82bb0f0a3d1\ncd1c842414af3c26dbbaf603be300fa2c1f813bc8d531681c66d020a31397326"
    );
}

#[test]
#[serial]
fn test_hash_copy_after_digest_throws() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hash = crypto.createHash('sha256');
        hash.update('done');
        hash.digest('hex');
        try {
          hash.copy();
        } catch (error) {
          error.message;
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "Digest already called");
}

#[test]
#[serial]
fn test_hash_base64_encoding() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hash = crypto.createHash('md5');
        hash.update('hello');
        hash.digest('base64');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "XUFAKrxLKna5cZ2REBfFkg==");
}

#[test]
#[serial]
fn test_hash_latin1_and_binary_encodings_return_digest_bytes() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        function byteStringToHex(value) {
          let hex = '';
          for (let i = 0; i < value.length; i++) {
            hex += (value.charCodeAt(i) & 0xff).toString(16).padStart(2, '0');
          }
          return value.length + ':' + hex;
        }

        const latin1 = crypto.createHash('sha256').update('hello').digest('latin1');
        const binary = crypto.createHash('sha256').update('hello').digest('binary');
        [byteStringToHex(latin1), byteStringToHex(binary)].join('\n');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "32:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824\n32:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
#[serial]
fn test_hash_digest_without_encoding_returns_binary_object() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const digest = crypto.createHash('sha256').update('hello').digest();
        const hex = Array.from(digest).map(b => b.toString(16).padStart(2, '0')).join('');
        [digest instanceof Uint8Array, digest.length, hex].join('\n');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "true\n32\n2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
#[serial]
fn test_hash_rejects_digest_and_update_after_digest() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hash = crypto.createHash('sha256');
        hash.update('hello');
        hash.digest('hex');

        let digestError = '';
        try {
          hash.digest('hex');
        } catch (error) {
          digestError = error.message;
        }

        let updateError = '';
        try {
          hash.update('again');
        } catch (error) {
          updateError = error.message;
        }

        [digestError, updateError].join('\n');
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "Digest already called\nDigest already called"
    );
}

#[test]
#[serial]
fn test_hash_unsupported_algorithm() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 测试调用 digest() 时抛出错误
    let code = r#"crypto.createHash('unsupported').digest('hex');"#;
    let result = runtime.execute_code(code);
    // 验证错误被抛出（传播到 Rust）
    assert!(result.is_err(), "Expected error for unsupported algorithm");
}

#[test]
#[serial]
fn test_hash_rejects_unsupported_algorithm_at_creation() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
          crypto.createHash('unsupported');
        } catch (error) {
          error.message;
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "Unsupported hash algorithm: unsupported"
    );
}

#[test]
#[serial]
fn test_hash_update_returns_hash_object() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hash = crypto.createHash('md5');
        const result = hash.update('test');
        typeof result;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_hash_algorithm_property() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const hash = crypto.createHash('sha256');
        hash._algorithm;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "sha256");
}

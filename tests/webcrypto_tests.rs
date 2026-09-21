// Tests for Web Crypto API (crypto.subtle) - v0.3.30
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_subtle_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.subtle");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_get_random_values_returns_input_typed_array() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const input = new Uint8Array(12);
        const output = crypto.getRandomValues(input);
        output === input &&
          output instanceof Uint8Array &&
          output.byteLength === 12;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "true",
        "crypto.getRandomValues must return the input TypedArray"
    );
}

#[test]
#[serial]
fn test_get_random_values_only_fills_typed_array_view_window() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const buffer = new ArrayBuffer(8);
        const all = new Uint8Array(buffer);
        all.fill(7);
        const view = new Uint8Array(buffer, 2, 3);
        const output = crypto.getRandomValues(view);

        output === view &&
          all[0] === 7 &&
          all[1] === 7 &&
          all[5] === 7 &&
          all[6] === 7 &&
          all[7] === 7 &&
          (all[2] !== 7 || all[3] !== 7 || all[4] !== 7);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "true",
        "crypto.getRandomValues must only mutate the TypedArray view range"
    );
}

#[test]
#[serial]
fn test_get_random_values_rejects_float_typed_arrays() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            crypto.getRandomValues(new Float32Array(4));
            false;
        } catch (error) {
            error instanceof TypeError &&
              String(error && error.message ? error.message : error).includes('integer');
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().trim(),
        "true",
        "crypto.getRandomValues must reject Float32Array/Float64Array inputs"
    );
}

#[test]
#[serial]
fn test_subtle_digest_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.subtle.digest");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_subtle_digest_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.digest('SHA-256', new TextEncoder().encode('hello'));
        result && result.constructor && result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_digest_sha256_no_error() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            const result = crypto.subtle.digest('SHA-256', new TextEncoder().encode('hello'));
            result !== undefined && result.constructor.name === 'Promise';
        } catch (e) {
            false;
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_digest_sha512_no_error() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            const result = crypto.subtle.digest('SHA-512', new TextEncoder().encode('hello'));
            result !== undefined && result.constructor.name === 'Promise';
        } catch (e) {
            false;
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_digest_sha384_no_error() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            const result = crypto.subtle.digest('SHA-384', new TextEncoder().encode('hello'));
            result !== undefined && result.constructor.name === 'Promise';
        } catch (e) {
            false;
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_digest_sha512_string_matches_known_vector() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const digest = await crypto.subtle.digest('SHA-512', new TextEncoder().encode('abc'));
            return Array.from(new Uint8Array(digest))
                .map((byte) => byte.toString(16).padStart(2, '0'))
                .join('');
        })();
    "#;
    let result = runtime.execute_code(code).unwrap();

    assert_eq!(
        result.trim(),
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
    );
}

#[test]
#[serial]
fn test_subtle_digest_sha384_object_name_matches_known_vector() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const digest = await crypto.subtle.digest({ name: 'SHA-384' }, new TextEncoder().encode('abc'));
            return Array.from(new Uint8Array(digest))
                .map((byte) => byte.toString(16).padStart(2, '0'))
                .join('');
        })();
    "#;
    let result = runtime.execute_code(code).unwrap();

    assert_eq!(
        result.trim(),
        "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7"
    );
}

#[test]
#[serial]
fn test_subtle_digest_unknown_string_algorithm_fails_closed() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            try {
                await crypto.subtle.digest('MD5', new TextEncoder().encode('abc'));
                return 'resolved';
            } catch (error) {
                return String(error && error.message || error);
            }
        })();
    "#;
    let result = runtime.execute_code(code).unwrap();

    assert!(
        result.contains("Unsupported hash algorithm"),
        "unknown digest algorithms must fail closed, got: {result}"
    );
}

#[test]
#[serial]
fn test_subtle_digest_sha1_no_error() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        try {
            const result = crypto.subtle.digest('SHA-1', new TextEncoder().encode('hello'));
            result !== undefined && result.constructor.name === 'Promise';
        } catch (e) {
            false;
        }
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_import_key_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.subtle.importKey");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_subtle_import_key_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.importKey(
            'raw',
            new Uint8Array([1, 2, 3, 4]),
            { name: 'HMAC', hash: 'SHA-256' },
            false,
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
fn test_subtle_sign_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.subtle.sign");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_subtle_sign_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const key = crypto.subtle.importKey(
            'raw',
            new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
            { name: 'HMAC', hash: 'SHA-256' },
            false,
            ['sign']
        );
        const result = key.then(k => crypto.subtle.sign({ name: 'HMAC' }, k, new TextEncoder().encode('message')));
        result && result.constructor && result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_verify_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.subtle.verify");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_subtle_verify_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const key = crypto.subtle.importKey(
            'raw',
            new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
            { name: 'HMAC', hash: 'SHA-256' },
            false,
            ['verify']
        );
        const result = key.then(k => crypto.subtle.verify({ name: 'HMAC' }, k, new Uint8Array(32), new TextEncoder().encode('message')));
        result && result.constructor && result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_encrypt_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.subtle.encrypt");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_subtle_encrypt_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const key = crypto.subtle.generateKey(
            { name: 'AES-GCM', length: 256 },
            true,
            ['encrypt', 'decrypt']
        );
        const iv = crypto.getRandomValues(new Uint8Array(12));
        const result = key.then(k => crypto.subtle.encrypt(
            { name: 'AES-GCM', iv: iv },
            k,
            new TextEncoder().encode('message')
        ));
        result && result.constructor && result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_decrypt_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.subtle.decrypt");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_subtle_decrypt_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const key = crypto.subtle.generateKey(
            { name: 'AES-GCM', length: 256 },
            true,
            ['encrypt', 'decrypt']
        );
        const result = key.then(async k => {
            const iv = crypto.getRandomValues(new Uint8Array(12));
            const encrypted = await crypto.subtle.encrypt(
                { name: 'AES-GCM', iv: iv },
                k,
                new TextEncoder().encode('message')
            );
            return crypto.subtle.decrypt(
                { name: 'AES-GCM', iv: iv },
                k,
                encrypted
            );
        });
        result && result.constructor && result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_subtle_generate_key_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.subtle.generateKey");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_subtle_generate_key_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const result = crypto.subtle.generateKey(
            { name: 'AES-GCM', length: 256 },
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
fn test_subtle_export_key_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.subtle.exportKey");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_subtle_export_key_returns_promise() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const key = crypto.subtle.importKey(
            'raw',
            new Uint8Array(32).fill(1),
            { name: 'HMAC', hash: 'SHA-256' },
            true,
            ['sign']
        );
        const result = key.then(k => crypto.subtle.exportKey('raw', k));
        result && result.constructor && result.constructor.name === 'Promise';
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

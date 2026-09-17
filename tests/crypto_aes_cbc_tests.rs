// AES-CBC WebCrypto encryption/decryption tests.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_aes_cbc_encrypt_decrypt_roundtrip_with_imported_raw_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyData = new Uint8Array([
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
                0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
                0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
                0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f
            ]);
            const iv = new Uint8Array([
                0x1f, 0x1e, 0x1d, 0x1c, 0x1b, 0x1a, 0x19, 0x18,
                0x17, 0x16, 0x15, 0x14, 0x13, 0x12, 0x11, 0x10
            ]);
            const plaintext = new TextEncoder().encode('amberjs aes-cbc roundtrip');
            const key = await crypto.subtle.importKey(
                'raw',
                keyData,
                { name: 'AES-CBC' },
                false,
                ['encrypt', 'decrypt']
            );
            const ciphertext = await crypto.subtle.encrypt(
                { name: 'AES-CBC', iv },
                key,
                plaintext
            );
            const decrypted = await crypto.subtle.decrypt(
                { name: 'AES-CBC', iv },
                key,
                ciphertext
            );
            return new TextDecoder().decode(decrypted) === 'amberjs aes-cbc roundtrip' &&
                ciphertext.byteLength % 16 === 0 &&
                ciphertext.byteLength > plaintext.byteLength;
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(
        result.is_ok(),
        "AES-CBC roundtrip should execute without throwing: {result:?}"
    );
    assert_eq!(result.unwrap().trim(), "true");
}

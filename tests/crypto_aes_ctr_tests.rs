// AES-CTR WebCrypto encryption/decryption tests.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_aes_ctr_encrypt_decrypt_roundtrip_with_imported_raw_key() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (async () => {
            const keyData = new Uint8Array([
                0x60, 0x3d, 0xeb, 0x10, 0x15, 0xca, 0x71, 0xbe,
                0x2b, 0x73, 0xae, 0xf0, 0x85, 0x7d, 0x77, 0x81,
                0x1f, 0x35, 0x2c, 0x07, 0x3b, 0x61, 0x08, 0xd7,
                0x2d, 0x98, 0x10, 0xa3, 0x09, 0x14, 0xdf, 0xf4
            ]);
            const counter = new Uint8Array([
                0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7,
                0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff
            ]);
            const plaintext = new TextEncoder().encode('beejs aes-ctr roundtrip');
            const key = await crypto.subtle.importKey(
                'raw',
                keyData,
                { name: 'AES-CTR' },
                false,
                ['encrypt', 'decrypt']
            );
            const ciphertext = await crypto.subtle.encrypt(
                { name: 'AES-CTR', counter, length: 64 },
                key,
                plaintext
            );
            const decrypted = await crypto.subtle.decrypt(
                { name: 'AES-CTR', counter, length: 64 },
                key,
                ciphertext
            );
            const cipherBytes = new Uint8Array(ciphertext);
            const plainBytes = new Uint8Array(plaintext);
            let sameBytes = cipherBytes.length === plainBytes.length;
            for (let i = 0; i < cipherBytes.length; i++) {
                if (cipherBytes[i] !== plainBytes[i]) {
                    sameBytes = false;
                    break;
                }
            }
            return new TextDecoder().decode(decrypted) === 'beejs aes-ctr roundtrip' &&
                ciphertext.byteLength === plaintext.byteLength &&
                !sameBytes;
        })();
    "#;
    let result = runtime.execute_code(code);
    assert!(
        result.is_ok(),
        "AES-CTR roundtrip should execute without throwing: {result:?}"
    );
    assert_eq!(result.unwrap().trim(), "true");
}

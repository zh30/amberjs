// Web Crypto API implementation
// Implements Web Crypto API standard: https://www.w3.org/TR/WebCryptoAPI/
// Supports crypto.subtle for hashing, encryption, and key operations

use anyhow::Result;
use base64::Engine;
use openssl::bn::{BigNum, BigNumContext};
use openssl::cipher::{Cipher as OpenSslKeyWrapCipher, CipherRef as OpenSslCipherRef};
use openssl::cipher_ctx::{CipherCtx, CipherCtxFlags};
use openssl::derive::Deriver;
use openssl::ec::{EcGroup, EcGroupRef, EcKey, EcPoint, PointConversionForm};
use openssl::ecdsa::EcdsaSig;
use openssl::encrypt::{Decrypter, Encrypter};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{Id, PKey, Private, Public};
use openssl::rsa::{Padding, Rsa};
use openssl::sign::{Signer, Verifier};
use openssl::symm::{
    decrypt as openssl_decrypt, encrypt as openssl_encrypt, Cipher as OpensslCipher, Crypter, Mode,
};
use ring::aead::{Aad, Algorithm, LessSafeKey, Nonce, UnboundKey, AES_128_GCM, AES_256_GCM};
use rusty_v8 as v8;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};

/// Get bytes from an ArrayBuffer or TypedArray.
fn get_array_buffer_data(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> Option<Vec<u8>> {
    if value.is_array_buffer() {
        let buffer = v8::Local::<v8::ArrayBuffer>::try_from(value).ok()?;
        let len = buffer.byte_length();
        if len == 0 {
            return Some(Vec::new());
        }

        let backing_store = buffer.get_backing_store();
        let ptr = backing_store
            .data()
            .map(|p| p.as_ptr() as *const u8)
            .unwrap_or(std::ptr::null());
        if ptr.is_null() {
            return None;
        }

        return Some(unsafe { std::slice::from_raw_parts(ptr, len).to_vec() });
    }

    if !value.is_typed_array() {
        return None;
    }

    let typed_array = v8::Local::<v8::TypedArray>::try_from(value).ok()?;
    let buffer = typed_array.buffer(scope)?;
    let len = typed_array.byte_length();
    if len == 0 {
        return Some(Vec::new());
    }

    let backing_store = buffer.get_backing_store();
    let ptr = backing_store
        .data()
        .map(|p| p.as_ptr() as *const u8)
        .unwrap_or(std::ptr::null());
    if ptr.is_null() {
        return None;
    }

    Some(unsafe { std::slice::from_raw_parts(ptr.add(typed_array.byte_offset()), len).to_vec() })
}

fn aes_cbc_cipher_for_key_len(key_len: usize) -> Option<OpensslCipher> {
    match key_len {
        16 => Some(OpensslCipher::aes_128_cbc()),
        24 => Some(OpensslCipher::aes_192_cbc()),
        32 => Some(OpensslCipher::aes_256_cbc()),
        _ => None,
    }
}

fn aes_ctr_cipher_for_key_len(key_len: usize) -> Option<OpensslCipher> {
    match key_len {
        16 => Some(OpensslCipher::aes_128_ctr()),
        24 => Some(OpensslCipher::aes_192_ctr()),
        32 => Some(OpensslCipher::aes_256_ctr()),
        _ => None,
    }
}

fn aes_kw_cipher_for_key_len(key_len: usize) -> Option<&'static OpenSslCipherRef> {
    match key_len {
        16 => Some(OpenSslKeyWrapCipher::aes_128_wrap()),
        24 => Some(OpenSslKeyWrapCipher::aes_192_wrap()),
        32 => Some(OpenSslKeyWrapCipher::aes_256_wrap()),
        _ => None,
    }
}

fn aes_kw_wrap_key_data(
    cipher: &OpenSslCipherRef,
    wrapping_key: &[u8],
    plaintext_key: &[u8],
) -> Result<Vec<u8>, String> {
    if plaintext_key.len() < 16 || plaintext_key.len() % 8 != 0 {
        return Err("AES-KW data must be at least 16 bytes and a multiple of 8 bytes".to_string());
    }

    let mut ctx = CipherCtx::new().map_err(|error| error.to_string())?;
    ctx.set_flags(CipherCtxFlags::FLAG_WRAP_ALLOW);
    ctx.encrypt_init(Some(cipher), Some(wrapping_key), None::<&[u8]>)
        .map_err(|error| error.to_string())?;

    let mut output = vec![0u8; plaintext_key.len() + cipher.block_size() * 2 + 8];
    let count = ctx
        .cipher_update(plaintext_key, Some(&mut output))
        .map_err(|error| error.to_string())?;
    let rest = ctx
        .cipher_final(&mut output[count..])
        .map_err(|error| error.to_string())?;
    output.truncate(count + rest);
    Ok(output)
}

fn aes_kw_unwrap_key_data(
    cipher: &OpenSslCipherRef,
    wrapping_key: &[u8],
    wrapped_key: &[u8],
) -> Result<Vec<u8>, String> {
    if wrapped_key.len() < 24 || wrapped_key.len() % 8 != 0 {
        return Err(
            "AES-KW wrapped data must be at least 24 bytes and a multiple of 8 bytes".to_string(),
        );
    }

    let mut ctx = CipherCtx::new().map_err(|error| error.to_string())?;
    ctx.set_flags(CipherCtxFlags::FLAG_WRAP_ALLOW);
    ctx.decrypt_init(Some(cipher), Some(wrapping_key), None::<&[u8]>)
        .map_err(|error| error.to_string())?;

    let mut output = vec![0u8; wrapped_key.len() + cipher.block_size() * 2];
    let count = ctx
        .cipher_update(wrapped_key, Some(&mut output))
        .map_err(|error| error.to_string())?;
    let rest = ctx
        .cipher_final(&mut output[count..])
        .map_err(|error| error.to_string())?;
    output.truncate(count + rest);
    Ok(output)
}

fn get_required_algorithm_bytes_property(
    scope: &mut v8::PinScope,
    operation: &str,
    algorithm_name: &str,
    algo_obj: Option<&v8::Local<v8::Object>>,
    property_name: &str,
    expected_len: usize,
) -> Option<Vec<u8>> {
    let Some(obj) = algo_obj else {
        let error = v8::String::new(
            scope,
            &format!(
                "{}: {} requires {}",
                operation, algorithm_name, property_name
            ),
        )
        .unwrap();
        let error_obj = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return None;
    };

    let property_key = v8::String::new(scope, property_name).unwrap();
    match obj.get(scope, property_key.into()) {
        Some(property_val) => match get_array_buffer_data(scope, property_val) {
            Some(property_data) if property_data.len() == expected_len => Some(property_data),
            Some(_) => {
                let error = v8::String::new(
                    scope,
                    &format!(
                        "{}: {} {} must be {} bytes",
                        operation, algorithm_name, property_name, expected_len
                    ),
                )
                .unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
                None
            }
            None => {
                let error = v8::String::new(
                    scope,
                    &format!(
                        "{}: {} {} must be an ArrayBuffer or TypedArray",
                        operation, algorithm_name, property_name
                    ),
                )
                .unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                None
            }
        },
        None => {
            let error = v8::String::new(
                scope,
                &format!(
                    "{}: {} requires {}",
                    operation, algorithm_name, property_name
                ),
            )
            .unwrap();
            let error_obj = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            None
        }
    }
}

fn get_required_algorithm_iv(
    scope: &mut v8::PinScope,
    operation: &str,
    algorithm_name: &str,
    algo_obj: Option<&v8::Local<v8::Object>>,
    expected_len: usize,
) -> Option<Vec<u8>> {
    get_required_algorithm_bytes_property(
        scope,
        operation,
        algorithm_name,
        algo_obj,
        "iv",
        expected_len,
    )
}

fn get_optional_algorithm_label(
    scope: &mut v8::PinScope,
    operation: &str,
    algo_obj: Option<&v8::Local<v8::Object>>,
) -> Option<Option<Vec<u8>>> {
    let Some(obj) = algo_obj else {
        return Some(None);
    };

    let label_key = v8::String::new(scope, "label").unwrap();
    let Some(label_val) = obj.get(scope, label_key.into()) else {
        return Some(None);
    };
    if label_val.is_undefined() || label_val.is_null() {
        return Some(None);
    }

    match get_array_buffer_data(scope, label_val) {
        Some(label) => Some(Some(label)),
        None => {
            let error = v8::String::new(
                scope,
                &format!(
                    "{}: RSA-OAEP label must be an ArrayBuffer or TypedArray",
                    operation
                ),
            )
            .unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            None
        }
    }
}

fn get_aes_ctr_length_bits(
    scope: &mut v8::PinScope,
    operation: &str,
    algo_obj: Option<&v8::Local<v8::Object>>,
) -> Option<u8> {
    let Some(obj) = algo_obj else {
        let error =
            v8::String::new(scope, &format!("{}: AES-CTR requires length", operation)).unwrap();
        let error_obj = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return None;
    };

    let length_key = v8::String::new(scope, "length").unwrap();
    let Some(length_val) = obj.get(scope, length_key.into()) else {
        let error =
            v8::String::new(scope, &format!("{}: AES-CTR requires length", operation)).unwrap();
        let error_obj = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return None;
    };

    if !length_val.is_number() {
        let error = v8::String::new(
            scope,
            &format!("{}: AES-CTR length must be a number", operation),
        )
        .unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return None;
    }

    let length = length_val.integer_value(scope).unwrap_or(0);
    if !(1..=128).contains(&length) {
        let error = v8::String::new(
            scope,
            &format!("{}: AES-CTR length must be between 1 and 128", operation),
        )
        .unwrap();
        let error_obj = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return None;
    }

    Some(length as u8)
}

fn aes_ctr_counter_has_capacity(counter: &[u8], length_bits: u8, data_len: usize) -> bool {
    if data_len == 0 {
        return true;
    }

    let mut counter_bytes = [0u8; 16];
    counter_bytes.copy_from_slice(counter);
    let counter_value = u128::from_be_bytes(counter_bytes);
    let blocks = data_len.div_ceil(16) as u128;

    if length_bits == 128 {
        return counter_value == 0 || blocks <= (u128::MAX - counter_value + 1);
    }

    let counter_space = 1u128 << length_bits;
    let low_mask = counter_space - 1;
    let low_counter = counter_value & low_mask;
    blocks <= counter_space - low_counter
}

fn aes_ctr_transform(
    cipher: OpensslCipher,
    key: &[u8],
    counter: &[u8],
    data: &[u8],
) -> Result<Vec<u8>, String> {
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(counter))
        .map_err(|error| error.to_string())?;
    crypter.pad(false);

    let mut output = vec![0u8; data.len() + cipher.block_size()];
    let count = crypter
        .update(data, &mut output)
        .map_err(|error| error.to_string())?;
    let rest = crypter
        .finalize(&mut output[count..])
        .map_err(|error| error.to_string())?;
    output.truncate(count + rest);
    Ok(output)
}

/// Get algorithm hash name
fn get_algorithm_hash_name(scope: &mut v8::PinScope, algo_value: v8::Local<v8::Value>) -> String {
    if algo_value.is_string() {
        return algo_value
            .to_string(scope)
            .map(|value| value.to_rust_string_lossy(scope))
            .unwrap_or_default();
    }

    if algo_value.is_object() {
        let algo_obj = algo_value.to_object(scope).unwrap();
        let hash_key = v8::String::new(scope, "hash").unwrap();

        if let Some(hash_val) = algo_obj.get(scope, hash_key.into()) {
            if hash_val.is_string() {
                let hash_str = hash_val.to_string(scope).unwrap();
                return hash_str.to_rust_string_lossy(scope);
            } else if hash_val.is_object() {
                let hash_obj = hash_val.to_object(scope).unwrap();
                let name_key = v8::String::new(scope, "name").unwrap();
                if let Some(name_val) = hash_obj.get(scope, name_key.into()) {
                    if name_val.is_string() {
                        let name_str = name_val.to_string(scope).unwrap();
                        return name_str.to_rust_string_lossy(scope);
                    }
                }
            }
        }

        let name_key = v8::String::new(scope, "name").unwrap();
        if let Some(name_val) = algo_obj.get(scope, name_key.into()) {
            if name_val.is_string() {
                let name_str = name_val.to_string(scope).unwrap();
                return name_str.to_rust_string_lossy(scope);
            }
        }
    }

    "SHA-256".to_string()
}

fn is_hmac_algorithm_name(algorithm_name: &str) -> bool {
    matches!(
        algorithm_name.to_ascii_uppercase().as_str(),
        "HMAC" | "HS256" | "HS384" | "HS512"
    )
}

fn normalize_hmac_hash_name(hash_name: &str) -> Option<&'static str> {
    let normalized = hash_name.to_ascii_uppercase().replace('_', "-");
    match normalized.as_str() {
        "SHA-1" | "SHA1" => Some("SHA-1"),
        "SHA-256" | "SHA256" | "HS256" => Some("SHA-256"),
        "SHA-384" | "SHA384" | "HS384" => Some("SHA-384"),
        "SHA-512" | "SHA512" | "HS512" => Some("SHA-512"),
        _ => None,
    }
}

fn hmac_hash_name_for_algorithm(
    scope: &mut v8::PinScope,
    algorithm_value: v8::Local<v8::Value>,
    algorithm_name: &str,
) -> Result<&'static str, String> {
    if let Some(hash_name) = normalize_hmac_hash_name(algorithm_name) {
        return Ok(hash_name);
    }

    let hash_name = get_algorithm_hash_name(scope, algorithm_value);
    normalize_hmac_hash_name(&hash_name)
        .ok_or_else(|| format!("unsupported HMAC hash algorithm '{}'", hash_name))
}

fn hmac_jwk_alg_from_hash(hash_name: &str) -> Option<&'static str> {
    match normalize_hmac_hash_name(hash_name)? {
        "SHA-1" => Some("HS1"),
        "SHA-256" => Some("HS256"),
        "SHA-384" => Some("HS384"),
        "SHA-512" => Some("HS512"),
        _ => None,
    }
}

fn ring_hmac_algorithm(hash_name: &str) -> Result<ring::hmac::Algorithm, String> {
    match normalize_hmac_hash_name(hash_name) {
        Some("SHA-1") => Ok(ring::hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY),
        Some("SHA-256") => Ok(ring::hmac::HMAC_SHA256),
        Some("SHA-384") => Ok(ring::hmac::HMAC_SHA384),
        Some("SHA-512") => Ok(ring::hmac::HMAC_SHA512),
        _ => Err(format!("unsupported HMAC hash algorithm '{}'", hash_name)),
    }
}

/// Compute SHA digest
fn compute_sha_digest(data: &[u8], algorithm: &str) -> Result<Vec<u8>, String> {
    match algorithm {
        "SHA-1" | "sha-1" => {
            let mut hasher = Sha1::new();
            hasher.update(data);
            Ok(hasher.finalize().to_vec())
        }
        "SHA-256" | "sha-256" => {
            let mut hasher = Sha256::new();
            hasher.update(data);
            Ok(hasher.finalize().to_vec())
        }
        "SHA-384" | "sha-384" => {
            let mut hasher = Sha384::new();
            hasher.update(data);
            Ok(hasher.finalize().to_vec())
        }
        "SHA-512" | "sha-512" => {
            let mut hasher = Sha512::new();
            hasher.update(data);
            Ok(hasher.finalize().to_vec())
        }
        _ => Err(format!("Unsupported hash algorithm: {}", algorithm)),
    }
}

fn openssl_message_digest(hash_name: &str) -> Result<MessageDigest, String> {
    match normalize_hmac_hash_name(hash_name) {
        Some("SHA-1") => Ok(MessageDigest::sha1()),
        Some("SHA-256") => Ok(MessageDigest::sha256()),
        Some("SHA-384") => Ok(MessageDigest::sha384()),
        Some("SHA-512") => Ok(MessageDigest::sha512()),
        _ => Err(format!("unsupported RSA hash algorithm '{}'", hash_name)),
    }
}

fn get_key_hash_name(scope: &mut v8::PinScope, crypto_key: v8::Local<v8::Object>) -> String {
    get_key_hmac_hash_name(scope, crypto_key)
}

fn is_rsassa_algorithm_name(algorithm_name: &str) -> bool {
    algorithm_name.to_ascii_uppercase() == "RSASSA-PKCS1-V1_5"
}

fn rsa_hash_name_for_operation(
    scope: &mut v8::PinScope,
    algo_value: v8::Local<v8::Value>,
    crypto_key: v8::Local<v8::Object>,
) -> String {
    let requested_hash = get_algorithm_hash_name(scope, algo_value);
    if let Some(hash_name) = normalize_hmac_hash_name(&requested_hash) {
        return hash_name.to_string();
    }

    get_key_hash_name(scope, crypto_key)
}

fn rsa_generate_key_pair(
    modulus_length: u32,
    public_exponent: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), String> {
    if public_exponent != [0x01, 0x00, 0x01] {
        return Err("RSA: only publicExponent 65537 is supported".to_string());
    }
    if !(1024..=4096).contains(&modulus_length) || modulus_length % 8 != 0 {
        return Err("RSA: modulusLength must be a multiple of 8 between 1024 and 4096".to_string());
    }

    let rsa = Rsa::generate(modulus_length)
        .map_err(|error| format!("RSA: key generation failed: {}", error))?;
    let key = PKey::from_rsa(rsa).map_err(|error| format!("RSA: key setup failed: {}", error))?;
    let private_key = key
        .private_key_to_pem_pkcs8()
        .map_err(|error| format!("RSA: private key export failed: {}", error))?;
    let public_key = key
        .public_key_to_pem()
        .map_err(|error| format!("RSA: public key export failed: {}", error))?;
    Ok((private_key, public_key))
}

fn rsa_sign_result(private_key: &[u8], hash_name: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    let digest = openssl_message_digest(hash_name)?;
    let key = PKey::private_key_from_pem(private_key)
        .map_err(|error| format!("RSA: invalid private key: {}", error))?;
    if key.id() != Id::RSA {
        return Err("RSA: private key is not an RSA key".to_string());
    }

    let mut signer = Signer::new(digest, &key)
        .map_err(|error| format!("RSA: signer setup failed: {}", error))?;
    signer
        .set_rsa_padding(Padding::PKCS1)
        .map_err(|error| format!("RSA: padding setup failed: {}", error))?;
    signer
        .update(data)
        .map_err(|error| format!("RSA: signer update failed: {}", error))?;
    signer
        .sign_to_vec()
        .map_err(|error| format!("RSA: signing failed: {}", error))
}

fn rsa_verify_result(
    public_key: &[u8],
    hash_name: &str,
    signature: &[u8],
    data: &[u8],
) -> Result<bool, String> {
    let digest = openssl_message_digest(hash_name)?;
    let key = PKey::public_key_from_pem(public_key)
        .map_err(|error| format!("RSA: invalid public key: {}", error))?;
    if key.id() != Id::RSA {
        return Err("RSA: public key is not an RSA key".to_string());
    }

    let mut verifier = Verifier::new(digest, &key)
        .map_err(|error| format!("RSA: verifier setup failed: {}", error))?;
    verifier
        .set_rsa_padding(Padding::PKCS1)
        .map_err(|error| format!("RSA: padding setup failed: {}", error))?;
    verifier
        .update(data)
        .map_err(|error| format!("RSA: verifier update failed: {}", error))?;
    verifier
        .verify(signature)
        .map_err(|error| format!("RSA: verification failed: {}", error))
}

fn is_rsa_oaep_algorithm_name(algorithm_name: &str) -> bool {
    algorithm_name.to_ascii_uppercase() == "RSA-OAEP"
}

fn rsa_oaep_crypt_digest(hash_name: &str) -> Result<MessageDigest, String> {
    openssl_message_digest(hash_name)
}

fn rsa_oaep_encrypt_result(
    public_key: &[u8],
    hash_name: &str,
    label: Option<&[u8]>,
    data: &[u8],
) -> Result<Vec<u8>, String> {
    let digest = rsa_oaep_crypt_digest(hash_name)?;
    let key = PKey::public_key_from_pem(public_key)
        .map_err(|error| format!("RSA-OAEP: invalid public key: {}", error))?;
    if key.id() != Id::RSA {
        return Err("RSA-OAEP: public key is not an RSA key".to_string());
    }

    let mut encrypter = Encrypter::new(&key)
        .map_err(|error| format!("RSA-OAEP: encrypter setup failed: {}", error))?;
    encrypter
        .set_rsa_padding(Padding::PKCS1_OAEP)
        .map_err(|error| format!("RSA-OAEP: padding setup failed: {}", error))?;
    encrypter
        .set_rsa_oaep_md(digest)
        .map_err(|error| format!("RSA-OAEP: OAEP hash setup failed: {}", error))?;
    encrypter
        .set_rsa_mgf1_md(digest)
        .map_err(|error| format!("RSA-OAEP: MGF1 hash setup failed: {}", error))?;
    if let Some(label) = label {
        if !label.is_empty() {
            encrypter
                .set_rsa_oaep_label(label)
                .map_err(|error| format!("RSA-OAEP: label setup failed: {}", error))?;
        }
    }

    let output_len = encrypter
        .encrypt_len(data)
        .map_err(|error| format!("RSA-OAEP: encrypt length failed: {}", error))?;
    let mut output = vec![0u8; output_len];
    let written = encrypter
        .encrypt(data, &mut output)
        .map_err(|error| format!("RSA-OAEP: encryption failed: {}", error))?;
    output.truncate(written);
    Ok(output)
}

fn rsa_oaep_decrypt_result(
    private_key: &[u8],
    hash_name: &str,
    label: Option<&[u8]>,
    ciphertext: &[u8],
) -> Result<Vec<u8>, String> {
    let digest = rsa_oaep_crypt_digest(hash_name)?;
    let key = PKey::private_key_from_pem(private_key)
        .map_err(|error| format!("RSA-OAEP: invalid private key: {}", error))?;
    if key.id() != Id::RSA {
        return Err("RSA-OAEP: private key is not an RSA key".to_string());
    }

    let mut decrypter = Decrypter::new(&key)
        .map_err(|error| format!("RSA-OAEP: decrypter setup failed: {}", error))?;
    decrypter
        .set_rsa_padding(Padding::PKCS1_OAEP)
        .map_err(|error| format!("RSA-OAEP: padding setup failed: {}", error))?;
    decrypter
        .set_rsa_oaep_md(digest)
        .map_err(|error| format!("RSA-OAEP: OAEP hash setup failed: {}", error))?;
    decrypter
        .set_rsa_mgf1_md(digest)
        .map_err(|error| format!("RSA-OAEP: MGF1 hash setup failed: {}", error))?;
    if let Some(label) = label {
        if !label.is_empty() {
            decrypter
                .set_rsa_oaep_label(label)
                .map_err(|error| format!("RSA-OAEP: label setup failed: {}", error))?;
        }
    }

    let output_len = decrypter
        .decrypt_len(ciphertext)
        .map_err(|error| format!("RSA-OAEP: decrypt length failed: {}", error))?;
    let mut output = vec![0u8; output_len];
    let written = decrypter
        .decrypt(ciphertext, &mut output)
        .map_err(|error| format!("RSA-OAEP: decryption failed: {}", error))?;
    output.truncate(written);
    Ok(output)
}

fn is_integer_typed_array(value: v8::Local<v8::Value>) -> bool {
    value.is_int8_array()
        || value.is_uint8_array()
        || value.is_uint8_clamped_array()
        || value.is_int16_array()
        || value.is_uint16_array()
        || value.is_int32_array()
        || value.is_uint32_array()
        || value.is_big_int64_array()
        || value.is_big_uint64_array()
}

/// getRandomValues callback
fn get_random_values_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if args.length() < 1 || !args.get(0).is_typed_array() {
        let error =
            v8::String::new(scope, "getRandomValues requires a TypedArray argument").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let typed_array_arg = args.get(0);
    if !is_integer_typed_array(typed_array_arg) {
        let error =
            v8::String::new(scope, "getRandomValues requires an integer TypedArray").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    // Check if it's a valid TypedArray for getRandomValues
    let arr = match v8::Local::<v8::TypedArray>::try_from(typed_array_arg) {
        Ok(arr) => arr,
        Err(_) => {
            let error = v8::String::new(scope, "getRandomValues requires a TypedArray").unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
    };

    let byte_length = arr.byte_length();
    let byte_offset = arr.byte_offset();
    if byte_length > 65536 {
        let error = v8::String::new(
            scope,
            "getRandomValues: array size must not exceed 65536 bytes",
        )
        .unwrap();
        let error_obj = v8::Exception::range_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    // Generate random values using ring
    use ring::rand::SecureRandom;
    let rng = ring::rand::SystemRandom::new();

    let buffer = match arr.buffer(scope) {
        Some(buf) => buf,
        None => {
            let error = v8::String::new(scope, "Failed to get ArrayBuffer").unwrap();
            let error_obj = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
    };

    let backing_store = buffer.get_backing_store();
    if byte_offset.saturating_add(byte_length) > backing_store.len() {
        let error =
            v8::String::new(scope, "getRandomValues: TypedArray view is out of bounds").unwrap();
        let error_obj = v8::Exception::range_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let mut data = vec![0u8; byte_length];
    if let Some(seed) = crate::permissions::get_deterministic_seed() {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let count = COUNTER.fetch_add(byte_length as u64, std::sync::atomic::Ordering::SeqCst);
        let mut state = seed.wrapping_add(count);
        for byte in data.iter_mut() {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *byte = (state >> 33) as u8;
        }
    } else if let Err(e) = rng.fill(&mut data) {
        let error_msg = format!("Failed to generate random values: {}", e);
        let error = v8::String::new(scope, &error_msg).unwrap();
        let error_obj = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    // Copy random data back to the backing store
    for (i, &byte) in data.iter().enumerate() {
        backing_store[byte_offset + i].set(byte);
    }

    retval.set(typed_array_arg);
}

/// Parse algorithm name from algorithm object.
fn get_algorithm_name_option(
    scope: &mut v8::PinScope,
    algo_value: v8::Local<v8::Value>,
) -> Option<String> {
    if algo_value.is_string() {
        let name = algo_value
            .to_string(scope)
            .unwrap()
            .to_rust_string_lossy(scope);
        return (!name.is_empty()).then_some(name);
    }

    if algo_value.is_object() {
        let algo_obj = algo_value.to_object(scope).unwrap();
        let name_key = v8::String::new(scope, "name").unwrap();
        if let Some(name_val) = algo_obj.get(scope, name_key.into()) {
            if name_val.is_string() {
                let name = name_val
                    .to_string(scope)
                    .unwrap()
                    .to_rust_string_lossy(scope);
                return (!name.is_empty()).then_some(name);
            }
        }
    }

    None
}

fn get_algorithm_name(scope: &mut v8::PinScope, algo_value: v8::Local<v8::Value>) -> String {
    get_algorithm_name_option(scope, algo_value).unwrap_or_default()
}

fn throw_missing_algorithm_name(scope: &mut v8::PinScope, operation: &str) {
    let error =
        v8::String::new(scope, &format!("{}: algorithm.name is required", operation)).unwrap();
    let error_obj = v8::Exception::type_error(scope, error);
    scope.throw_exception(error_obj.into());
}

/// Create a CryptoKey object with proper structure
fn create_crypto_key<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    key_type: &str, // "secret", "public", "private"
    extractable: bool,
    algorithm_name: &str,
    algorithm_length: i32, // For AES, 128, 192, or 256
    usages: Vec<&str>,
) -> v8::Local<'a, v8::Object> {
    let crypto_key = v8::Object::new(scope);

    // Set type
    let type_key = v8::String::new(scope, "type").unwrap();
    let type_val = v8::String::new(scope, key_type).unwrap();
    crypto_key.set(scope, type_key.into(), type_val.into());

    // Set extractable
    let extractable_key = v8::String::new(scope, "extractable").unwrap();
    let extractable_val = v8::Boolean::new(scope, extractable);
    crypto_key.set(scope, extractable_key.into(), extractable_val.into());

    // Set algorithm object
    let algorithm_key = v8::String::new(scope, "algorithm").unwrap();
    let algorithm_obj = v8::Object::new(scope);

    let algo_name_key = v8::String::new(scope, "name").unwrap();
    let algo_name_val = v8::String::new(scope, algorithm_name).unwrap();
    algorithm_obj.set(scope, algo_name_key.into(), algo_name_val.into());

    // Add length for AES algorithms
    if algorithm_name.starts_with("AES-") {
        let length_key = v8::String::new(scope, "length").unwrap();
        let length_val = v8::Integer::new(scope, algorithm_length);
        algorithm_obj.set(scope, length_key.into(), length_val.into());
    } else if algorithm_name == "HMAC" {
        // For HMAC, we might want to store hash algorithm
        let hash_key = v8::String::new(scope, "hash").unwrap();
        let hash_obj = v8::Object::new(scope);
        let hash_name_key = v8::String::new(scope, "name").unwrap();
        let hash_name_val = v8::String::new(scope, "SHA-256").unwrap();
        hash_obj.set(scope, hash_name_key.into(), hash_name_val.into());
        algorithm_obj.set(scope, hash_key.into(), hash_obj.into());
    }

    crypto_key.set(scope, algorithm_key.into(), algorithm_obj.into());

    // Set usages
    let usages_key = v8::String::new(scope, "usages").unwrap();
    let usages_array = v8::Array::new(scope, usages.len() as i32);
    for (i, usage) in usages.iter().enumerate() {
        let usage_str = v8::String::new(scope, usage).unwrap();
        usages_array.set_index(scope, i as u32, usage_str.into());
    }
    crypto_key.set(scope, usages_key.into(), usages_array.into());

    crypto_key
}

fn set_crypto_key_hmac_hash(
    scope: &mut v8::PinScope,
    crypto_key: v8::Local<v8::Object>,
    hash_name: &str,
) {
    let algorithm_key = v8::String::new(scope, "algorithm").unwrap();
    let algorithm_obj = crypto_key
        .get(scope, algorithm_key.into())
        .and_then(|value| value.to_object(scope))
        .unwrap_or_else(|| v8::Object::new(scope));

    let hash_key = v8::String::new(scope, "hash").unwrap();
    let hash_obj = v8::Object::new(scope);
    let hash_name_key = v8::String::new(scope, "name").unwrap();
    let hash_name_val = v8::String::new(scope, hash_name).unwrap();
    hash_obj.set(scope, hash_name_key.into(), hash_name_val.into());
    algorithm_obj.set(scope, hash_key.into(), hash_obj.into());
    crypto_key.set(scope, algorithm_key.into(), algorithm_obj.into());
}

fn get_key_hmac_hash_name(scope: &mut v8::PinScope, crypto_key: v8::Local<v8::Object>) -> String {
    let algorithm_key = v8::String::new(scope, "algorithm").unwrap();
    let Some(algo_val) = crypto_key.get(scope, algorithm_key.into()) else {
        return "SHA-256".to_string();
    };
    let Some(algo_obj) = algo_val.to_object(scope) else {
        return "SHA-256".to_string();
    };

    let hash_key = v8::String::new(scope, "hash").unwrap();
    if let Some(hash_val) = algo_obj.get(scope, hash_key.into()) {
        if hash_val.is_string() {
            let hash_name = hash_val
                .to_string(scope)
                .map(|value| value.to_rust_string_lossy(scope))
                .unwrap_or_default();
            if let Some(normalized) = normalize_hmac_hash_name(&hash_name) {
                return normalized.to_string();
            }
        } else if let Some(hash_obj) = hash_val.to_object(scope) {
            let name_key = v8::String::new(scope, "name").unwrap();
            if let Some(name_val) = hash_obj.get(scope, name_key.into()) {
                let hash_name = name_val
                    .to_string(scope)
                    .map(|value| value.to_rust_string_lossy(scope))
                    .unwrap_or_default();
                if let Some(normalized) = normalize_hmac_hash_name(&hash_name) {
                    return normalized.to_string();
                }
            }
        }
    }

    let name_key = v8::String::new(scope, "name").unwrap();
    if let Some(name_val) = algo_obj.get(scope, name_key.into()) {
        let algorithm_name = name_val
            .to_string(scope)
            .map(|value| value.to_rust_string_lossy(scope))
            .unwrap_or_default();
        if let Some(normalized) = normalize_hmac_hash_name(&algorithm_name) {
            return normalized.to_string();
        }
    }

    "SHA-256".to_string()
}

/// Get string value from V8 value
fn get_string_value(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> Option<String> {
    if value.is_string() {
        Some(value.to_string(scope).unwrap().to_rust_string_lossy(scope))
    } else {
        None
    }
}

/// Get boolean value from V8 value
fn get_bool_value(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> bool {
    if value.is_boolean() {
        value.to_boolean(scope).boolean_value(scope)
    } else {
        false
    }
}

/// Parse key usages from V8 array
fn get_key_usages(scope: &mut v8::PinScope, usages_value: v8::Local<v8::Value>) -> Vec<String> {
    let mut usages = Vec::new();

    if usages_value.is_array() {
        let usages_array = v8::Local::<v8::Array>::try_from(usages_value).unwrap();
        let length = usages_array.length();
        for i in 0..length {
            if let Some(usage_val) = usages_array.get_index(scope, i as u32) {
                if let Some(usage_str) = get_string_value(scope, usage_val) {
                    usages.push(usage_str);
                }
            }
        }
    }

    usages
}

fn get_object_string_property(
    scope: &mut v8::PinScope,
    object: v8::Local<v8::Object>,
    name: &str,
) -> Option<String> {
    let key = v8::String::new(scope, name)?;
    object
        .get(scope, key.into())
        .and_then(|value| get_string_value(scope, value))
}

fn get_object_bool_property(
    scope: &mut v8::PinScope,
    object: v8::Local<v8::Object>,
    name: &str,
) -> Result<Option<bool>, String> {
    let key = v8::String::new(scope, name).ok_or_else(|| format!("JWK {} is invalid", name))?;
    match object.get(scope, key.into()) {
        Some(value) if value.is_undefined() || value.is_null() => Ok(None),
        Some(value) if value.is_boolean() => Ok(Some(value.boolean_value(scope))),
        Some(_) => Err(format!("JWK {} must be a boolean", name)),
        None => Ok(None),
    }
}

fn get_object_string_array_property(
    scope: &mut v8::PinScope,
    object: v8::Local<v8::Object>,
    name: &str,
) -> Result<Option<Vec<String>>, String> {
    let key = v8::String::new(scope, name).ok_or_else(|| format!("JWK {} is invalid", name))?;
    match object.get(scope, key.into()) {
        Some(value) if value.is_undefined() || value.is_null() => Ok(None),
        Some(value) if value.is_array() => {
            let array = v8::Local::<v8::Array>::try_from(value)
                .map_err(|_| format!("JWK {} must be an array", name))?;
            let mut entries = Vec::with_capacity(array.length() as usize);
            for i in 0..array.length() {
                let Some(item) = array.get_index(scope, i) else {
                    return Err(format!("JWK {} contains an invalid value", name));
                };
                let Some(item_string) = get_string_value(scope, item) else {
                    return Err(format!("JWK {} must contain only strings", name));
                };
                entries.push(item_string);
            }
            Ok(Some(entries))
        }
        Some(_) => Err(format!("JWK {} must be an array", name)),
        None => Ok(None),
    }
}

fn base64url_decode(input: &str) -> Result<Vec<u8>, String> {
    if input.contains('=') || input.contains('+') || input.contains('/') {
        return Err("expected unpadded base64url data".to_string());
    }

    let mut normalized = input.replace('-', "+").replace('_', "/");
    match normalized.len() % 4 {
        0 => {}
        2 => normalized.push_str("=="),
        3 => normalized.push('='),
        _ => return Err("invalid base64url length".to_string()),
    }

    base64::engine::general_purpose::STANDARD
        .decode(normalized.as_bytes())
        .map_err(|error| format!("invalid base64url data: {}", error))
}

struct JwkOctKeyData {
    key_data: Vec<u8>,
    alg: Option<String>,
    key_ops: Option<Vec<String>>,
    ext: Option<bool>,
}

fn get_jwk_oct_key_data(
    scope: &mut v8::PinScope,
    key_data_value: v8::Local<v8::Value>,
) -> Result<JwkOctKeyData, String> {
    if !key_data_value.is_object() {
        return Err("JWK keyData must be an object".to_string());
    }

    let jwk = key_data_value
        .to_object(scope)
        .ok_or_else(|| "JWK keyData must be an object".to_string())?;
    let kty = get_object_string_property(scope, jwk, "kty")
        .ok_or_else(|| "JWK kty is required".to_string())?;
    if kty != "oct" {
        return Err(format!("unsupported JWK kty '{}'. Supported: oct", kty));
    }

    let k = get_object_string_property(scope, jwk, "k")
        .ok_or_else(|| "JWK k is required".to_string())?;
    Ok(JwkOctKeyData {
        key_data: base64url_decode(&k)?,
        alg: get_object_string_property(scope, jwk, "alg"),
        key_ops: get_object_string_array_property(scope, jwk, "key_ops")?,
        ext: get_object_bool_property(scope, jwk, "ext")?,
    })
}

struct JwkOkpKeyData {
    public_key_data: Vec<u8>,
    private_key_data: Option<Vec<u8>>,
    canonical_name: &'static str,
    alg: Option<String>,
    key_ops: Option<Vec<String>>,
    ext: Option<bool>,
}

fn get_jwk_okp_key_data(
    scope: &mut v8::PinScope,
    key_data_value: v8::Local<v8::Value>,
) -> Result<JwkOkpKeyData, String> {
    if !key_data_value.is_object() {
        return Err("JWK keyData must be an object".to_string());
    }

    let jwk = key_data_value
        .to_object(scope)
        .ok_or_else(|| "JWK keyData must be an object".to_string())?;
    let kty = get_object_string_property(scope, jwk, "kty")
        .ok_or_else(|| "JWK kty is required".to_string())?;
    if kty != "OKP" {
        return Err(format!("unsupported JWK kty '{}'. Supported: OKP", kty));
    }

    let crv = get_object_string_property(scope, jwk, "crv")
        .ok_or_else(|| "JWK crv is required".to_string())?;
    let (_id, canonical_name, key_size) = eddsa_key_id(&crv)
        .ok_or_else(|| format!("unsupported JWK crv '{}'. Supported: Ed25519, Ed448", crv))?;

    let x = get_object_string_property(scope, jwk, "x")
        .ok_or_else(|| "JWK x is required".to_string())?;
    let public_key_data = base64url_decode(&x)?;
    if public_key_data.len() != key_size {
        return Err(format!("JWK x must be {} bytes", key_size));
    }

    let private_key_data = match get_object_string_property(scope, jwk, "d") {
        Some(d) => {
            let decoded = base64url_decode(&d)?;
            if decoded.len() != key_size {
                return Err(format!("JWK d must be {} bytes", key_size));
            }
            Some(decoded)
        }
        None => None,
    };

    Ok(JwkOkpKeyData {
        public_key_data,
        private_key_data,
        canonical_name,
        alg: get_object_string_property(scope, jwk, "alg"),
        key_ops: get_object_string_array_property(scope, jwk, "key_ops")?,
        ext: get_object_bool_property(scope, jwk, "ext")?,
    })
}

fn expected_jwk_alg(
    algorithm_name: &str,
    key_length_bits: i32,
    hmac_hash_name: Option<&str>,
) -> Option<String> {
    match algorithm_name.to_ascii_uppercase().as_str() {
        "HMAC" | "HS256" | "HS384" | "HS512" => hmac_hash_name
            .and_then(hmac_jwk_alg_from_hash)
            .map(|alg| alg.to_string()),
        "AES-GCM" => Some(format!("A{}GCM", key_length_bits)),
        "AES-CBC" => Some(format!("A{}CBC", key_length_bits)),
        "AES-CTR" => Some(format!("A{}CTR", key_length_bits)),
        "AES-KW" => Some(format!("A{}KW", key_length_bits)),
        _ => None,
    }
}

fn validate_jwk_oct_import(
    jwk: &JwkOctKeyData,
    expected_alg: Option<&str>,
    extractable: bool,
    usages: &[String],
) -> Result<(), String> {
    if let Some(false) = jwk.ext {
        if extractable {
            return Err("JWK ext false cannot be imported as extractable".to_string());
        }
    }

    if let Some(ref key_ops) = jwk.key_ops {
        for usage in usages {
            if !key_ops.iter().any(|key_op| key_op == usage) {
                return Err(format!(
                    "JWK key_ops does not allow requested usage '{}'",
                    usage
                ));
            }
        }
    }

    if let (Some(actual_alg), Some(expected_alg)) = (jwk.alg.as_deref(), expected_alg) {
        if actual_alg != expected_alg {
            return Err(format!(
                "JWK alg '{}' does not match requested algorithm '{}'",
                actual_alg, expected_alg
            ));
        }
    }

    Ok(())
}

fn get_json_string_property(
    object: &serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<Option<String>, String> {
    match object.get(name) {
        Some(value) if value.is_null() => Ok(None),
        Some(value) => value
            .as_str()
            .map(|value| Some(value.to_string()))
            .ok_or_else(|| format!("JWK {} must be a string", name)),
        None => Ok(None),
    }
}

fn get_json_bool_property(
    object: &serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<Option<bool>, String> {
    match object.get(name) {
        Some(value) if value.is_null() => Ok(None),
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| format!("JWK {} must be a boolean", name)),
        None => Ok(None),
    }
}

fn get_json_string_array_property(
    object: &serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<Option<Vec<String>>, String> {
    match object.get(name) {
        Some(value) if value.is_null() => Ok(None),
        Some(value) => {
            let array = value
                .as_array()
                .ok_or_else(|| format!("JWK {} must be an array", name))?;
            let mut entries = Vec::with_capacity(array.len());
            for item in array {
                let Some(item_string) = item.as_str() else {
                    return Err(format!("JWK {} must contain only strings", name));
                };
                entries.push(item_string.to_string());
            }
            Ok(Some(entries))
        }
        None => Ok(None),
    }
}

fn get_jwk_oct_key_data_from_json_bytes(key_data: &[u8]) -> Result<JwkOctKeyData, String> {
    let json = std::str::from_utf8(key_data)
        .map_err(|error| format!("JWK keyData must be UTF-8 JSON: {}", error))?;
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|error| format!("JWK keyData is invalid: {}", error))?;
    let object = value
        .as_object()
        .ok_or_else(|| "JWK keyData must be an object".to_string())?;

    let kty = get_json_string_property(object, "kty")?
        .ok_or_else(|| "JWK kty is required".to_string())?;
    if kty != "oct" {
        return Err(format!("unsupported JWK kty '{}'. Supported: oct", kty));
    }

    let k =
        get_json_string_property(object, "k")?.ok_or_else(|| "JWK k is required".to_string())?;
    Ok(JwkOctKeyData {
        key_data: base64url_decode(&k)?,
        alg: get_json_string_property(object, "alg")?,
        key_ops: get_json_string_array_property(object, "key_ops")?,
        ext: get_json_bool_property(object, "ext")?,
    })
}

fn validate_jwk_okp_import(
    jwk: &JwkOkpKeyData,
    expected_alg: &str,
    extractable: bool,
    usages: &[String],
) -> Result<(), String> {
    if jwk.canonical_name != expected_alg {
        return Err(format!(
            "JWK crv '{}' does not match requested algorithm '{}'",
            jwk.canonical_name, expected_alg
        ));
    }

    if let Some(false) = jwk.ext {
        if extractable {
            return Err("JWK ext false cannot be imported as extractable".to_string());
        }
    }

    if let Some(ref key_ops) = jwk.key_ops {
        for usage in usages {
            if !key_ops.iter().any(|key_op| key_op == usage) {
                return Err(format!(
                    "JWK key_ops does not allow requested usage '{}'",
                    usage
                ));
            }
        }
    }

    if let Some(actual_alg) = jwk.alg.as_deref() {
        if actual_alg != expected_alg {
            return Err(format!(
                "JWK alg '{}' does not match requested algorithm '{}'",
                actual_alg, expected_alg
            ));
        }
    }

    Ok(())
}

fn validate_eddsa_import_key_type_usages(
    scope: &mut v8::PinScope,
    key_type: &str,
    usages: &[String],
) -> bool {
    let disallowed = usages.iter().find(|usage| match key_type {
        "public" => usage.as_str() != "verify",
        "private" => usage.as_str() != "sign",
        _ => true,
    });

    let Some(disallowed) = disallowed else {
        return true;
    };

    let error = v8::String::new(
        scope,
        &format!(
            "importKey: usage '{}' is not allowed for {} EdDSA keys",
            disallowed, key_type
        ),
    )
    .unwrap();
    let error_obj = v8::Exception::type_error(scope, error);
    scope.throw_exception(error_obj.into());
    false
}

/// Setup crypto.subtle.importKey
fn import_key_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if args.length() < 5 {
        let error = v8::String::new(
            scope,
            "importKey requires 5 arguments: format, keyData, algorithm, extractable, keyUsages",
        )
        .unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let format_value = args.get(0);
    let key_data_value = args.get(1);
    let algorithm_value = args.get(2);
    let extractable_value = args.get(3);
    let usages_value = args.get(4);

    // Parse format
    let format = match get_string_value(scope, format_value) {
        Some(f) => f,
        None => {
            let error = v8::String::new(scope, "importKey: format must be a string").unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
    };

    // Validate format
    if format != "raw" && format != "pkcs8" && format != "spki" && format != "jwk" {
        let error_msg = format!(
            "importKey: unsupported format '{}'. Currently supported: 'raw'",
            format
        );
        let error = v8::String::new(scope, &error_msg).unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    // Parse algorithm
    let algorithm_name = match get_algorithm_name_option(scope, algorithm_value) {
        Some(name) => name,
        None => {
            throw_missing_algorithm_name(scope, "importKey");
            return;
        }
    };

    // Parse extractable
    let extractable = get_bool_value(scope, extractable_value);

    // Parse usages
    let usages = get_key_usages(scope, usages_value);
    if !validate_key_usages(scope, "importKey", &algorithm_name, &usages) {
        return;
    }

    let mut hmac_hash_name = None;
    let (key_type, length, key_data, crypto_key_algorithm_name) =
        if let Some((_id, canonical_name, key_size)) = eddsa_key_id(&algorithm_name) {
            let (key_type, key_data) = match format.as_str() {
                "raw" => {
                    let key_data = match get_array_buffer_data(scope, key_data_value) {
                        Some(data) => data,
                        None => {
                            let error = v8::String::new(
                                scope,
                                "importKey: raw EdDSA keyData must be an ArrayBuffer or TypedArray",
                            )
                            .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };
                    if key_data.len() != key_size {
                        let error = v8::String::new(
                            scope,
                            &format!(
                                "importKey: {} raw public key must be {} bytes",
                                canonical_name, key_size
                            ),
                        )
                        .unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                    ("public".to_string(), key_data)
                }
                "jwk" => {
                    let jwk = match get_jwk_okp_key_data(scope, key_data_value) {
                        Ok(data) => data,
                        Err(error_message) => {
                            let error =
                                v8::String::new(scope, &format!("importKey: {}", error_message))
                                    .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };
                    if let Err(error_message) =
                        validate_jwk_okp_import(&jwk, canonical_name, extractable, &usages)
                    {
                        let error =
                            v8::String::new(scope, &format!("importKey: {}", error_message))
                                .unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }

                    match jwk.private_key_data {
                        Some(private_key_data) => ("private".to_string(), private_key_data),
                        None => ("public".to_string(), jwk.public_key_data),
                    }
                }
                "spki" => {
                    let spki_der = match get_array_buffer_data(scope, key_data_value) {
                        Some(data) => data,
                        None => {
                            let error = v8::String::new(
                            scope,
                            "importKey: spki EdDSA keyData must be an ArrayBuffer or TypedArray",
                        )
                        .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };
                    let public_key = match eddsa_public_raw_from_spki_der(canonical_name, &spki_der)
                    {
                        Ok(public_key) => public_key,
                        Err(error_message) => {
                            let error =
                                v8::String::new(scope, &format!("importKey: {}", error_message))
                                    .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };
                    ("public".to_string(), public_key)
                }
                "pkcs8" => {
                    let pkcs8_der = match get_array_buffer_data(scope, key_data_value) {
                        Some(data) => data,
                        None => {
                            let error = v8::String::new(
                            scope,
                            "importKey: pkcs8 EdDSA keyData must be an ArrayBuffer or TypedArray",
                        )
                        .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };
                    let private_key =
                        match eddsa_private_raw_from_pkcs8_der(canonical_name, &pkcs8_der) {
                            Ok(private_key) => private_key,
                            Err(error_message) => {
                                let error = v8::String::new(
                                    scope,
                                    &format!("importKey: {}", error_message),
                                )
                                .unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            }
                        };
                    ("private".to_string(), private_key)
                }
                _ => {
                    let error = v8::String::new(
                        scope,
                        &format!("importKey: unsupported EdDSA format '{}'", format),
                    )
                    .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            if !validate_eddsa_import_key_type_usages(scope, &key_type, &usages) {
                return;
            }

            (
                key_type,
                (key_size * 8) as i32,
                key_data,
                canonical_name.to_string(),
            )
        } else {
            let mut jwk_data = None;
            let key_data = if format == "jwk" {
                match get_jwk_oct_key_data(scope, key_data_value) {
                    Ok(data) => {
                        let key_data = data.key_data.clone();
                        jwk_data = Some(data);
                        key_data
                    }
                    Err(error_message) => {
                        let error =
                            v8::String::new(scope, &format!("importKey: {}", error_message))
                                .unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                }
            } else {
                match get_array_buffer_data(scope, key_data_value) {
                    Some(data) => data,
                    None => {
                        let error = v8::String::new(
                            scope,
                            "importKey: keyData must be an ArrayBuffer or TypedArray",
                        )
                        .unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                }
            };

            let (key_type, length) = match algorithm_name.to_uppercase().as_str() {
                "HMAC" | "HS256" | "HS384" | "HS512" => {
                    ("secret".to_string(), (key_data.len() * 8) as i32)
                }
                "AES-GCM" | "AES-CBC" | "AES-CTR" | "AES-KW" => {
                    let Some(length) = aes_key_len_bits_from_bytes(key_data.len()) else {
                        let error = v8::String::new(
                            scope,
                            "importKey: AES key length must be 128, 192, or 256 bits",
                        )
                        .unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    };
                    ("secret".to_string(), length)
                }
                "PBKDF2" => {
                    // PBKDF2 uses password as key material (imported as raw)
                    // The length is derived from the key data
                    ("secret".to_string(), (key_data.len() * 8) as i32)
                }
                _ => {
                    let error_msg =
                        format!("importKey: unsupported algorithm '{}'", algorithm_name);
                    let error = v8::String::new(scope, &error_msg).unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            hmac_hash_name = if is_hmac_algorithm_name(&algorithm_name) {
                match hmac_hash_name_for_algorithm(scope, algorithm_value, &algorithm_name) {
                    Ok(hash_name) => Some(hash_name),
                    Err(error_message) => {
                        let error =
                            v8::String::new(scope, &format!("importKey: {}", error_message))
                                .unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                }
            } else {
                None
            };

            if let Some(ref jwk) = jwk_data {
                let expected_alg = expected_jwk_alg(&algorithm_name, length, hmac_hash_name)
                    .map(|alg| alg.to_string());
                if let Err(error_message) =
                    validate_jwk_oct_import(jwk, expected_alg.as_deref(), extractable, &usages)
                {
                    let error =
                        v8::String::new(scope, &format!("importKey: {}", error_message)).unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            }

            (key_type, length, key_data, algorithm_name.clone())
        };

    // Create CryptoKey object
    let crypto_key = create_crypto_key(
        scope,
        &key_type,
        extractable,
        &crypto_key_algorithm_name,
        length,
        usages.iter().map(|s| s.as_str()).collect(),
    );
    if let Some(hash_name) = hmac_hash_name {
        set_crypto_key_hmac_hash(scope, crypto_key, hash_name);
    }

    set_key_data(scope, crypto_key, &key_data);

    // Return promise resolving to the CryptoKey
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    resolver.resolve(scope, crypto_key.into());
    let promise = resolver.get_promise(scope);
    retval.set(promise.into());
}

/// Get key type from CryptoKey object
fn get_key_type(scope: &mut v8::PinScope, crypto_key: v8::Local<v8::Object>) -> String {
    let type_key = v8::String::new(scope, "type").unwrap();
    if let Some(type_val) = crypto_key.get(scope, type_key.into()) {
        if type_val.is_string() {
            return type_val
                .to_string(scope)
                .unwrap()
                .to_rust_string_lossy(scope);
        }
    }
    String::new()
}

/// Get curve name from CryptoKey object for ECDSA/ECDH
fn get_curve_name(scope: &mut v8::PinScope, crypto_key: v8::Local<v8::Object>) -> String {
    // First check __beejs_curve__ property
    let curve_key = v8::String::new(scope, "__beejs_curve__").unwrap();
    if let Some(curve_val) = crypto_key.get(scope, curve_key.into()) {
        if let Some(curve_str) = get_string_value(scope, curve_val) {
            return curve_str;
        }
    }

    // Fall back to algorithm.namedCurve
    let algo_key = v8::String::new(scope, "algorithm").unwrap();
    if let Some(algo_val) = crypto_key.get(scope, algo_key.into()) {
        if let Some(algo_obj) = algo_val.to_object(scope) {
            let named_curve_key = v8::String::new(scope, "namedCurve").unwrap();
            if let Some(curve_val) = algo_obj.get(scope, named_curve_key.into()) {
                if let Some(curve_str) = get_string_value(scope, curve_val) {
                    return curve_str;
                }
            }
        }
    }

    "P-256".to_string() // Default to P-256
}

/// Get key data from CryptoKey object
#[allow(dead_code)]
fn get_key_data(scope: &mut v8::PinScope, crypto_key: v8::Local<v8::Object>) -> Option<Vec<u8>> {
    let key_data_key_name = v8::String::new(scope, "BeeJS.CryptoKey#keyData").unwrap();
    let key_data_key = v8::Private::for_api(scope, Some(key_data_key_name));
    if let Some(key_data_value) = crypto_key.get_private(scope, key_data_key) {
        if key_data_value.is_undefined() {
            None
        } else {
            get_array_buffer_data(scope, key_data_value)
        }
    } else {
        None
    }
}

fn set_key_data(scope: &mut v8::PinScope, crypto_key: v8::Local<v8::Object>, key_data: &[u8]) {
    let key_data_key_name = v8::String::new(scope, "BeeJS.CryptoKey#keyData").unwrap();
    let key_data_key = v8::Private::for_api(scope, Some(key_data_key_name));
    let key_data_array = v8::ArrayBuffer::new(scope, key_data.len());
    let backing_store = key_data_array.get_backing_store();
    for (i, &byte) in key_data.iter().enumerate() {
        backing_store[i].set(byte);
    }
    crypto_key.set_private(scope, key_data_key, key_data_array.into());
}

fn get_required_key_data(
    scope: &mut v8::PinScope,
    operation: &str,
    crypto_key: v8::Local<v8::Object>,
) -> Option<Vec<u8>> {
    match get_key_data(scope, crypto_key) {
        Some(data) => Some(data),
        None => {
            let error =
                v8::String::new(scope, &format!("{}: key data is unavailable", operation)).unwrap();
            let error_obj = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            None
        }
    }
}

fn resolve_unwrapped_secret_key_promise(
    scope: &mut v8::PinScope,
    retval: &mut v8::ReturnValue,
    format_str: &str,
    key_algo_value: v8::Local<v8::Value>,
    extractable_value: v8::Local<v8::Value>,
    usages_value: v8::Local<v8::Value>,
    unwrapped_key_data: &[u8],
) {
    let algo_name_str = get_algorithm_name(scope, key_algo_value);
    let extractable = if extractable_value.is_boolean() {
        extractable_value.boolean_value(scope)
    } else {
        false
    };
    let usages = get_key_usages(scope, usages_value);

    let hmac_hash_name = if is_hmac_algorithm_name(&algo_name_str) {
        match hmac_hash_name_for_algorithm(scope, key_algo_value, &algo_name_str) {
            Ok(hash_name) => Some(hash_name),
            Err(error_message) => {
                let error =
                    v8::String::new(scope, &format!("unwrapKey: {}", error_message)).unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        }
    } else {
        None
    };

    let key_data = if format_str == "jwk" {
        let jwk = match get_jwk_oct_key_data_from_json_bytes(unwrapped_key_data) {
            Ok(jwk) => jwk,
            Err(error_message) => {
                let error =
                    v8::String::new(scope, &format!("unwrapKey: {}", error_message)).unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        };
        let key_length_bits = (jwk.key_data.len() * 8) as i32;
        let expected_alg = expected_jwk_alg(&algo_name_str, key_length_bits, hmac_hash_name)
            .map(|alg| alg.to_string());
        if let Err(error_message) =
            validate_jwk_oct_import(&jwk, expected_alg.as_deref(), extractable, &usages)
        {
            let error = v8::String::new(scope, &format!("unwrapKey: {}", error_message)).unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
        jwk.key_data
    } else {
        unwrapped_key_data.to_vec()
    };

    let crypto_key_obj = create_crypto_key(
        scope,
        "secret",
        extractable,
        &algo_name_str,
        (key_data.len() * 8) as i32,
        usages.iter().map(|usage| usage.as_str()).collect(),
    );
    if let Some(hash_name) = hmac_hash_name {
        set_crypto_key_hmac_hash(scope, crypto_key_obj, hash_name);
    };
    set_key_data(scope, crypto_key_obj, &key_data);

    let resolver = v8::PromiseResolver::new(scope).unwrap();
    resolver.resolve(scope, crypto_key_obj.into());
    let promise = resolver.get_promise(scope);
    retval.set(promise.into());
}

fn ecdh_curve_nid(curve: &str) -> Result<Nid, String> {
    match curve {
        "P-256" | "prime256v1" | "secp256r1" => Ok(Nid::X9_62_PRIME256V1),
        "P-384" | "secp384r1" => Ok(Nid::SECP384R1),
        "P-521" | "secp521r1" => Ok(Nid::SECP521R1),
        _ => Err(format!(
            "ECDH: unsupported curve '{}'. Supported: P-256, P-384, P-521",
            curve
        )),
    }
}

fn ecdh_group(curve: &str) -> Result<EcGroup, String> {
    let nid = ecdh_curve_nid(curve)?;
    EcGroup::from_curve_name(nid).map_err(|error| format!("ECDH: curve setup failed: {}", error))
}

fn ecdh_private_key_size(group: &EcGroupRef) -> usize {
    group.degree().div_ceil(8) as usize
}

fn left_pad_private_key(mut private_key: Vec<u8>, target_len: usize) -> Vec<u8> {
    if private_key.len() >= target_len {
        return private_key;
    }

    let mut padded = vec![0u8; target_len - private_key.len()];
    padded.append(&mut private_key);
    padded
}

fn ecdh_private_key_bytes(key: &EcKey<Private>) -> Vec<u8> {
    left_pad_private_key(
        key.private_key().to_vec(),
        ecdh_private_key_size(key.group()),
    )
}

fn ecdh_public_key_bytes(key: &EcKey<Private>) -> Result<Vec<u8>, String> {
    let mut ctx =
        BigNumContext::new().map_err(|error| format!("ECDH: BigNum context failed: {}", error))?;
    key.public_key()
        .to_bytes(key.group(), PointConversionForm::UNCOMPRESSED, &mut ctx)
        .map_err(|error| format!("ECDH: public key export failed: {}", error))
}

fn ecdh_key_from_private_bytes(curve: &str, private_key: &[u8]) -> Result<EcKey<Private>, String> {
    if private_key.is_empty() {
        return Err("ECDH: private key is empty".to_string());
    }

    let group = ecdh_group(curve)?;
    let private_number = BigNum::from_slice(private_key)
        .map_err(|error| format!("ECDH: invalid private key: {}", error))?;
    let mut ctx =
        BigNumContext::new().map_err(|error| format!("ECDH: BigNum context failed: {}", error))?;
    let mut public_key = EcPoint::new(&group)
        .map_err(|error| format!("ECDH: public key allocation failed: {}", error))?;
    public_key
        .mul_generator2(&group, &private_number, &mut ctx)
        .map_err(|error| format!("ECDH: public key derivation failed: {}", error))?;

    let key = EcKey::from_private_components(&group, &private_number, &public_key)
        .map_err(|error| format!("ECDH: private key setup failed: {}", error))?;
    key.check_key()
        .map_err(|error| format!("ECDH: invalid private key: {}", error))?;
    Ok(key)
}

fn ecdh_generate_key_pair(curve: &str) -> Result<(Vec<u8>, Vec<u8>), String> {
    let group = ecdh_group(curve)?;
    let key = EcKey::generate(&group)
        .map_err(|error| format!("ECDH: key generation failed: {}", error))?;
    let private_key = ecdh_private_key_bytes(&key);
    let public_key = ecdh_public_key_bytes(&key)?;
    Ok((private_key, public_key))
}

fn derive_ecdh_bits_result(
    curve: &str,
    private_key: &[u8],
    public_key: &[u8],
    length_bits: usize,
) -> Result<Vec<u8>, String> {
    if private_key.is_empty() || public_key.is_empty() {
        return Err("ECDH: valid key material is required".to_string());
    }

    let private_key = ecdh_key_from_private_bytes(curve, private_key)?;
    let group = private_key.group();
    let mut ctx =
        BigNumContext::new().map_err(|error| format!("ECDH: BigNum context failed: {}", error))?;
    let peer_point = EcPoint::from_bytes(group, public_key, &mut ctx)
        .map_err(|error| format!("ECDH: invalid public key: {}", error))?;
    if peer_point.is_infinity(group) {
        return Err("ECDH: invalid public key".to_string());
    }
    if !peer_point
        .is_on_curve(group, &mut ctx)
        .map_err(|error| format!("ECDH: public key validation failed: {}", error))?
    {
        return Err("ECDH: invalid public key".to_string());
    }

    let peer_key = EcKey::from_public_key(group, &peer_point)
        .map_err(|error| format!("ECDH: invalid public key: {}", error))?;
    peer_key
        .check_key()
        .map_err(|error| format!("ECDH: invalid public key: {}", error))?;

    let private_pkey = PKey::from_ec_key(private_key)
        .map_err(|error| format!("ECDH: private key failed: {}", error))?;
    let peer_pkey =
        PKey::from_ec_key(peer_key).map_err(|error| format!("ECDH: peer key failed: {}", error))?;
    let mut deriver =
        Deriver::new(&private_pkey).map_err(|error| format!("ECDH: deriver failed: {}", error))?;
    deriver
        .set_peer(&peer_pkey)
        .map_err(|error| format!("ECDH: peer setup failed: {}", error))?;
    let mut shared = deriver
        .derive_to_vec()
        .map_err(|error| format!("ECDH: derivation failed: {}", error))?;

    let output_len = length_bits.div_ceil(8);
    if output_len > shared.len() {
        return Err(format!(
            "ECDH: requested {} bits exceeds shared secret length",
            length_bits
        ));
    }
    shared.truncate(output_len);
    if length_bits % 8 != 0 && !shared.is_empty() {
        let keep_bits = (length_bits % 8) as u8;
        let mask = 0xFFu8 << (8 - keep_bits);
        if let Some(last) = shared.last_mut() {
            *last &= mask;
        }
    }
    Ok(shared)
}

fn ecdsa_group(curve: &str) -> Result<EcGroup, String> {
    ecdh_group(curve).map_err(|error| error.replacen("ECDH", "ECDSA", 1))
}

fn ecdsa_key_from_private_bytes(curve: &str, private_key: &[u8]) -> Result<EcKey<Private>, String> {
    ecdh_key_from_private_bytes(curve, private_key)
        .map_err(|error| error.replacen("ECDH", "ECDSA", 1))
}

fn ecdsa_generate_key_pair(curve: &str) -> Result<(Vec<u8>, Vec<u8>), String> {
    let group = ecdsa_group(curve)?;
    let key = EcKey::generate(&group)
        .map_err(|error| format!("ECDSA: key generation failed: {}", error))?;
    let private_key = ecdh_private_key_bytes(&key);
    let public_key =
        ecdh_public_key_bytes(&key).map_err(|error| error.replacen("ECDH", "ECDSA", 1))?;
    Ok((private_key, public_key))
}

fn ecdsa_public_key_from_bytes(curve: &str, public_key: &[u8]) -> Result<EcKey<Public>, String> {
    if public_key.is_empty() {
        return Err("ECDSA: public key is empty".to_string());
    }

    let group = ecdsa_group(curve)?;
    let mut ctx =
        BigNumContext::new().map_err(|error| format!("ECDSA: BigNum context failed: {}", error))?;
    let public_point = EcPoint::from_bytes(&group, public_key, &mut ctx)
        .map_err(|error| format!("ECDSA: invalid public key: {}", error))?;
    if public_point.is_infinity(&group) {
        return Err("ECDSA: invalid public key".to_string());
    }
    if !public_point
        .is_on_curve(&group, &mut ctx)
        .map_err(|error| format!("ECDSA: public key validation failed: {}", error))?
    {
        return Err("ECDSA: invalid public key".to_string());
    }

    let key = EcKey::from_public_key(&group, &public_point)
        .map_err(|error| format!("ECDSA: public key setup failed: {}", error))?;
    key.check_key()
        .map_err(|error| format!("ECDSA: invalid public key: {}", error))?;
    Ok(key)
}

fn ecdsa_signature_to_raw(signature: &EcdsaSig, component_size: usize) -> Result<Vec<u8>, String> {
    let component_size_i32 = i32::try_from(component_size)
        .map_err(|_| "ECDSA: signature component size is too large".to_string())?;
    let mut raw = signature
        .r()
        .to_vec_padded(component_size_i32)
        .map_err(|error| format!("ECDSA: signature encoding failed: {}", error))?;
    let mut s = signature
        .s()
        .to_vec_padded(component_size_i32)
        .map_err(|error| format!("ECDSA: signature encoding failed: {}", error))?;
    raw.append(&mut s);
    Ok(raw)
}

fn ecdsa_signature_from_raw(
    signature: &[u8],
    component_size: usize,
) -> Result<Option<EcdsaSig>, String> {
    if signature.len() != component_size * 2 {
        return Ok(None);
    }

    let r = BigNum::from_slice(&signature[..component_size])
        .map_err(|error| format!("ECDSA: invalid signature: {}", error))?;
    let s = BigNum::from_slice(&signature[component_size..])
        .map_err(|error| format!("ECDSA: invalid signature: {}", error))?;
    let signature = EcdsaSig::from_private_components(r, s)
        .map_err(|error| format!("ECDSA: invalid signature: {}", error))?;
    Ok(Some(signature))
}

fn ecdsa_sign_result(
    curve: &str,
    private_key: &[u8],
    hash_name: &str,
    data: &[u8],
) -> Result<Vec<u8>, String> {
    let key = ecdsa_key_from_private_bytes(curve, private_key)?;
    let digest = compute_sha_digest(data, hash_name)?;
    let signature = EcdsaSig::sign(&digest, &key)
        .map_err(|error| format!("ECDSA: signing failed: {}", error))?;
    ecdsa_signature_to_raw(&signature, ecdh_private_key_size(key.group()))
}

fn ecdsa_verify_result(
    curve: &str,
    public_key: &[u8],
    hash_name: &str,
    signature: &[u8],
    data: &[u8],
) -> Result<bool, String> {
    let key = ecdsa_public_key_from_bytes(curve, public_key)?;
    let digest = compute_sha_digest(data, hash_name)?;
    let component_size = ecdh_private_key_size(key.group());
    let Some(signature) = ecdsa_signature_from_raw(signature, component_size)? else {
        return Ok(false);
    };

    signature
        .verify(&digest, &key)
        .map_err(|error| format!("ECDSA: verification failed: {}", error))
}

fn eddsa_key_id(algorithm_name: &str) -> Option<(Id, &'static str, usize)> {
    match algorithm_name.to_ascii_uppercase().as_str() {
        "ED25519" => Some((Id::ED25519, "Ed25519", 32)),
        "ED448" => Some((Id::ED448, "Ed448", 57)),
        _ => None,
    }
}

fn is_eddsa_algorithm_name(algorithm_name: &str) -> bool {
    eddsa_key_id(algorithm_name).is_some()
}

fn eddsa_generate_key_pair(
    algorithm_name: &str,
) -> Result<(Vec<u8>, Vec<u8>, &'static str), String> {
    let (id, canonical_name, _key_size) = eddsa_key_id(algorithm_name).ok_or_else(|| {
        format!(
            "EdDSA: unsupported algorithm '{}'. Supported: Ed25519, Ed448",
            algorithm_name
        )
    })?;
    let key = match id {
        Id::ED25519 => PKey::generate_ed25519()
            .map_err(|error| format!("Ed25519: key generation failed: {}", error))?,
        Id::ED448 => PKey::generate_ed448()
            .map_err(|error| format!("Ed448: key generation failed: {}", error))?,
        _ => unreachable!("validated EdDSA key id"),
    };
    let private_key_data = key
        .raw_private_key()
        .map_err(|error| format!("{}: private key export failed: {}", canonical_name, error))?;
    let public_key_data = key
        .raw_public_key()
        .map_err(|error| format!("{}: public key export failed: {}", canonical_name, error))?;
    Ok((private_key_data, public_key_data, canonical_name))
}

fn eddsa_sign_result(
    algorithm_name: &str,
    private_key: &[u8],
    data: &[u8],
) -> Result<Vec<u8>, String> {
    let (id, canonical_name, key_size) = eddsa_key_id(algorithm_name).ok_or_else(|| {
        format!(
            "EdDSA: unsupported algorithm '{}'. Supported: Ed25519, Ed448",
            algorithm_name
        )
    })?;
    if private_key.len() != key_size {
        return Err(format!(
            "{}: private key must be {} bytes",
            canonical_name, key_size
        ));
    }
    let key = PKey::private_key_from_raw_bytes(private_key, id)
        .map_err(|error| format!("{}: invalid private key: {}", canonical_name, error))?;
    let mut signer = Signer::new_without_digest(&key)
        .map_err(|error| format!("{}: signer setup failed: {}", canonical_name, error))?;
    signer
        .sign_oneshot_to_vec(data)
        .map_err(|error| format!("{}: signing failed: {}", canonical_name, error))
}

fn eddsa_verify_result(
    algorithm_name: &str,
    public_key: &[u8],
    signature: &[u8],
    data: &[u8],
) -> Result<bool, String> {
    let (id, canonical_name, key_size) = eddsa_key_id(algorithm_name).ok_or_else(|| {
        format!(
            "EdDSA: unsupported algorithm '{}'. Supported: Ed25519, Ed448",
            algorithm_name
        )
    })?;
    if public_key.len() != key_size {
        return Err(format!(
            "{}: public key must be {} bytes",
            canonical_name, key_size
        ));
    }
    let key = PKey::public_key_from_raw_bytes(public_key, id)
        .map_err(|error| format!("{}: invalid public key: {}", canonical_name, error))?;
    let mut verifier = Verifier::new_without_digest(&key)
        .map_err(|error| format!("{}: verifier setup failed: {}", canonical_name, error))?;
    verifier
        .verify_oneshot(signature, data)
        .map_err(|error| format!("{}: verification failed: {}", canonical_name, error))
}

fn eddsa_public_key_from_private(
    algorithm_name: &str,
    private_key: &[u8],
) -> Result<Vec<u8>, String> {
    let (id, canonical_name, key_size) = eddsa_key_id(algorithm_name).ok_or_else(|| {
        format!(
            "EdDSA: unsupported algorithm '{}'. Supported: Ed25519, Ed448",
            algorithm_name
        )
    })?;
    if private_key.len() != key_size {
        return Err(format!(
            "{}: private key must be {} bytes",
            canonical_name, key_size
        ));
    }
    let key = PKey::private_key_from_raw_bytes(private_key, id)
        .map_err(|error| format!("{}: invalid private key: {}", canonical_name, error))?;
    key.raw_public_key()
        .map_err(|error| format!("{}: public key export failed: {}", canonical_name, error))
}

fn eddsa_public_der_from_raw(algorithm_name: &str, public_key: &[u8]) -> Result<Vec<u8>, String> {
    let (id, canonical_name, key_size) = eddsa_key_id(algorithm_name).ok_or_else(|| {
        format!(
            "EdDSA: unsupported algorithm '{}'. Supported: Ed25519, Ed448",
            algorithm_name
        )
    })?;
    if public_key.len() != key_size {
        return Err(format!(
            "{}: public key must be {} bytes",
            canonical_name, key_size
        ));
    }
    let key = PKey::public_key_from_raw_bytes(public_key, id)
        .map_err(|error| format!("{}: invalid public key: {}", canonical_name, error))?;
    key.public_key_to_der()
        .map_err(|error| format!("{}: SPKI export failed: {}", canonical_name, error))
}

fn eddsa_private_der_from_raw(algorithm_name: &str, private_key: &[u8]) -> Result<Vec<u8>, String> {
    let (id, canonical_name, key_size) = eddsa_key_id(algorithm_name).ok_or_else(|| {
        format!(
            "EdDSA: unsupported algorithm '{}'. Supported: Ed25519, Ed448",
            algorithm_name
        )
    })?;
    if private_key.len() != key_size {
        return Err(format!(
            "{}: private key must be {} bytes",
            canonical_name, key_size
        ));
    }
    let key = PKey::private_key_from_raw_bytes(private_key, id)
        .map_err(|error| format!("{}: invalid private key: {}", canonical_name, error))?;
    key.private_key_to_der()
        .map_err(|error| format!("{}: PKCS#8 export failed: {}", canonical_name, error))
}

fn eddsa_public_raw_from_spki_der(
    algorithm_name: &str,
    spki_der: &[u8],
) -> Result<Vec<u8>, String> {
    let (id, canonical_name, key_size) = eddsa_key_id(algorithm_name).ok_or_else(|| {
        format!(
            "EdDSA: unsupported algorithm '{}'. Supported: Ed25519, Ed448",
            algorithm_name
        )
    })?;
    let key = PKey::public_key_from_der(spki_der)
        .map_err(|error| format!("{}: invalid SPKI public key: {}", canonical_name, error))?;
    if key.id() != id {
        return Err(format!(
            "{}: SPKI key algorithm does not match requested algorithm",
            canonical_name
        ));
    }
    let raw = key
        .raw_public_key()
        .map_err(|error| format!("{}: public key export failed: {}", canonical_name, error))?;
    if raw.len() != key_size {
        return Err(format!(
            "{}: public key must be {} bytes",
            canonical_name, key_size
        ));
    }
    Ok(raw)
}

fn eddsa_private_raw_from_pkcs8_der(
    algorithm_name: &str,
    pkcs8_der: &[u8],
) -> Result<Vec<u8>, String> {
    let (id, canonical_name, key_size) = eddsa_key_id(algorithm_name).ok_or_else(|| {
        format!(
            "EdDSA: unsupported algorithm '{}'. Supported: Ed25519, Ed448",
            algorithm_name
        )
    })?;
    let key = PKey::private_key_from_der(pkcs8_der)
        .map_err(|error| format!("{}: invalid PKCS#8 private key: {}", canonical_name, error))?;
    if key.id() != id {
        return Err(format!(
            "{}: PKCS#8 key algorithm does not match requested algorithm",
            canonical_name
        ));
    }
    let raw = key
        .raw_private_key()
        .map_err(|error| format!("{}: private key export failed: {}", canonical_name, error))?;
    if raw.len() != key_size {
        return Err(format!(
            "{}: private key must be {} bytes",
            canonical_name, key_size
        ));
    }
    Ok(raw)
}

fn crypto_key_has_usage(
    scope: &mut v8::PinScope,
    crypto_key: v8::Local<v8::Object>,
    usage: &str,
) -> bool {
    let usages_key = v8::String::new(scope, "usages").unwrap();
    let Some(usages_val) = crypto_key.get(scope, usages_key.into()) else {
        return false;
    };
    let Ok(usages_array) = v8::Local::<v8::Array>::try_from(usages_val) else {
        return false;
    };

    for i in 0..usages_array.length() {
        if let Some(value) = usages_array.get_index(scope, i) {
            if let Some(value_str) = value.to_string(scope) {
                if value_str.to_rust_string_lossy(scope) == usage {
                    return true;
                }
            }
        }
    }

    false
}

fn is_key_usage_allowed_for_algorithm(algorithm_name: &str, usage: &str) -> bool {
    match algorithm_name.to_uppercase().as_str() {
        "HMAC" | "HS256" | "HS384" | "HS512" => matches!(usage, "sign" | "verify"),
        "AES-GCM" | "AES-CBC" | "AES-CTR" => {
            matches!(usage, "encrypt" | "decrypt" | "wrapKey" | "unwrapKey")
        }
        "AES-KW" => matches!(usage, "wrapKey" | "unwrapKey"),
        "PBKDF2" | "ECDH" => matches!(usage, "deriveKey" | "deriveBits"),
        "ECDSA" | "RSASSA-PKCS1-V1_5" | "ED25519" | "ED448" => {
            matches!(usage, "sign" | "verify")
        }
        "RSA-OAEP" => matches!(usage, "encrypt" | "decrypt" | "wrapKey" | "unwrapKey"),
        _ => false,
    }
}

fn validate_key_usages(
    scope: &mut v8::PinScope,
    operation: &str,
    algorithm_name: &str,
    usages: &[String],
) -> bool {
    for usage in usages {
        if !is_key_usage_allowed_for_algorithm(algorithm_name, usage) {
            let error = v8::String::new(
                scope,
                &format!(
                    "{}: usage '{}' is not allowed for algorithm '{}'",
                    operation, usage, algorithm_name
                ),
            )
            .unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return false;
        }
    }

    true
}

fn ensure_key_algorithm_matches(
    scope: &mut v8::PinScope,
    operation: &str,
    requested_algorithm: &str,
    crypto_key: v8::Local<v8::Object>,
) -> bool {
    let key_algorithm = get_key_algorithm_name(scope, crypto_key);
    if key_algorithm.eq_ignore_ascii_case(requested_algorithm) {
        return true;
    }

    let error = v8::String::new(
        scope,
        &format!(
            "{}: key algorithm '{}' does not match requested algorithm '{}'",
            operation, key_algorithm, requested_algorithm
        ),
    )
    .unwrap();
    let error_obj = v8::Exception::type_error(scope, error);
    scope.throw_exception(error_obj.into());
    false
}

fn ensure_crypto_key_usage(
    scope: &mut v8::PinScope,
    operation: &str,
    crypto_key: v8::Local<v8::Object>,
    usage: &str,
) -> bool {
    if crypto_key_has_usage(scope, crypto_key, usage) {
        return true;
    }

    let error = v8::String::new(
        scope,
        &format!("{}: key usage '{}' is not allowed", operation, usage),
    )
    .unwrap();
    let error_obj = v8::Exception::type_error(scope, error);
    scope.throw_exception(error_obj.into());
    false
}

/// HMAC sign callback
fn hmac_sign_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if args.length() < 3 {
        let error =
            v8::String::new(scope, "sign requires algorithm, key, and data arguments").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let algo_value = args.get(0);
    let key_value = args.get(1);
    let data_value = args.get(2);

    if !key_value.is_object() {
        let error = v8::String::new(scope, "sign: key must be a CryptoKey").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let data = match get_array_buffer_data(scope, data_value) {
        Some(d) => d,
        None => {
            let error =
                v8::String::new(scope, "sign: data must be an ArrayBuffer or TypedArray").unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
    };

    // Get key type to determine algorithm
    let key_obj = key_value.to_object(scope).unwrap();
    let key_type = get_key_type(scope, key_obj);
    let algo_name = match get_algorithm_name_option(scope, algo_value) {
        Some(name) => name,
        None => {
            throw_missing_algorithm_name(scope, "sign");
            return;
        }
    };

    // Get key algorithm from algorithm object
    let key_algorithm = get_key_algorithm_name(scope, key_obj);
    if !ensure_key_algorithm_matches(scope, "sign", &algo_name, key_obj) {
        return;
    }
    if !ensure_crypto_key_usage(scope, "sign", key_obj, "sign") {
        return;
    }

    if is_eddsa_algorithm_name(&algo_name) || is_eddsa_algorithm_name(&key_algorithm) {
        if key_type != "private" {
            let error = v8::String::new(scope, "sign: EdDSA requires a private key").unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
        let private_key_data = match get_key_data(scope, key_obj) {
            Some(data) => data,
            None => {
                let error = v8::String::new(scope, "sign: EdDSA key data is unavailable").unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        };
        let signature = match eddsa_sign_result(&key_algorithm, &private_key_data, &data) {
            Ok(signature) => signature,
            Err(error_message) => {
                let error = v8::String::new(scope, &format!("sign: {}", error_message)).unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        };

        let array_buffer = v8::ArrayBuffer::new(scope, signature.len());
        let backing_store = array_buffer.get_backing_store();
        for (i, &byte) in signature.iter().enumerate() {
            backing_store[i].set(byte);
        }

        let resolver = v8::PromiseResolver::new(scope).unwrap();
        resolver.resolve(scope, array_buffer.into());
        let promise = resolver.get_promise(scope);
        retval.set(promise.into());
    } else if algo_name == "ECDSA" || key_algorithm == "ECDSA" {
        let curve_name = get_curve_name(scope, key_obj);
        let hash_name = get_algorithm_hash_name(scope, algo_value);
        let private_key_data = match get_key_data(scope, key_obj) {
            Some(data) => data,
            None => {
                let error = v8::String::new(scope, "sign: ECDSA key data is unavailable").unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        };
        let signature = match ecdsa_sign_result(&curve_name, &private_key_data, &hash_name, &data) {
            Ok(signature) => signature,
            Err(error_message) => {
                let error = v8::String::new(scope, &format!("sign: {}", error_message)).unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        };

        let array_buffer = v8::ArrayBuffer::new(scope, signature.len());
        let backing_store = array_buffer.get_backing_store();
        for (i, &byte) in signature.iter().enumerate() {
            backing_store[i].set(byte);
        }

        let resolver = v8::PromiseResolver::new(scope).unwrap();
        resolver.resolve(scope, array_buffer.into());
        let promise = resolver.get_promise(scope);
        retval.set(promise.into());
    } else if is_rsassa_algorithm_name(&algo_name) || is_rsassa_algorithm_name(&key_algorithm) {
        if key_type != "private" {
            let error = v8::String::new(scope, "sign: RSASSA requires a private key").unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
        let private_key_data = match get_key_data(scope, key_obj) {
            Some(data) => data,
            None => {
                let error = v8::String::new(scope, "sign: RSA key data is unavailable").unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        };
        let hash_name = rsa_hash_name_for_operation(scope, algo_value, key_obj);
        let signature = match rsa_sign_result(&private_key_data, &hash_name, &data) {
            Ok(signature) => signature,
            Err(error_message) => {
                let error = v8::String::new(scope, &format!("sign: {}", error_message)).unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        };

        let array_buffer = v8::ArrayBuffer::new(scope, signature.len());
        let backing_store = array_buffer.get_backing_store();
        for (i, &byte) in signature.iter().enumerate() {
            backing_store[i].set(byte);
        }

        let resolver = v8::PromiseResolver::new(scope).unwrap();
        resolver.resolve(scope, array_buffer.into());
        let promise = resolver.get_promise(scope);
        retval.set(promise.into());
    } else if key_type == "private" || algo_name.starts_with("RSA") {
        let error = v8::String::new(scope, "sign: RSA algorithm is not implemented").unwrap();
        let error_obj = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
    } else {
        // HMAC signing
        let Some(key_data) = get_required_key_data(scope, "sign", key_obj) else {
            return;
        };

        use ring::hmac;
        let hash_name = get_key_hmac_hash_name(scope, key_obj);
        let hmac_algorithm = match ring_hmac_algorithm(&hash_name) {
            Ok(algorithm) => algorithm,
            Err(error_message) => {
                let error = v8::String::new(scope, &format!("sign: {}", error_message)).unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        };
        let sign_key = hmac::Key::new(hmac_algorithm, &key_data);
        let signature = hmac::sign(&sign_key, &data);

        let sig_bytes = signature.as_ref().to_vec();
        let array_buffer = v8::ArrayBuffer::new(scope, sig_bytes.len());
        let backing_store = array_buffer.get_backing_store();
        for (i, &byte) in sig_bytes.iter().enumerate() {
            backing_store[i].set(byte);
        }

        let resolver = v8::PromiseResolver::new(scope).unwrap();
        resolver.resolve(scope, array_buffer.into());
        let promise = resolver.get_promise(scope);
        retval.set(promise.into());
    }
}

/// HMAC verify callback - now supports both HMAC and RSA verification
fn hmac_verify_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if args.length() < 4 {
        let error = v8::String::new(
            scope,
            "verify requires algorithm, key, signature, and data arguments",
        )
        .unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let algo_value = args.get(0);
    let key_value = args.get(1);
    let signature_value = args.get(2);
    let data_value = args.get(3);

    if !key_value.is_object() {
        let error = v8::String::new(scope, "verify: key must be a CryptoKey").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let signature = match get_array_buffer_data(scope, signature_value) {
        Some(s) => s,
        None => {
            let error = v8::String::new(
                scope,
                "verify: signature must be an ArrayBuffer or TypedArray",
            )
            .unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
    };

    let data = match get_array_buffer_data(scope, data_value) {
        Some(d) => d,
        None => {
            let error = v8::String::new(scope, "verify: data must be an ArrayBuffer or TypedArray")
                .unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
    };

    // Get key type to determine algorithm
    let key_obj = key_value.to_object(scope).unwrap();
    let key_type = get_key_type(scope, key_obj);
    let algo_name = match get_algorithm_name_option(scope, algo_value) {
        Some(name) => name,
        None => {
            throw_missing_algorithm_name(scope, "verify");
            return;
        }
    };

    // Get key algorithm from algorithm object
    let key_algorithm = get_key_algorithm_name(scope, key_obj);
    if !ensure_key_algorithm_matches(scope, "verify", &algo_name, key_obj) {
        return;
    }
    if !ensure_crypto_key_usage(scope, "verify", key_obj, "verify") {
        return;
    }

    let result_bool = if is_eddsa_algorithm_name(&algo_name)
        || is_eddsa_algorithm_name(&key_algorithm)
    {
        if key_type != "public" {
            let error = v8::String::new(scope, "verify: EdDSA requires a public key").unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
        let public_key_data = match get_key_data(scope, key_obj) {
            Some(data) => data,
            None => {
                let error =
                    v8::String::new(scope, "verify: EdDSA key data is unavailable").unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        };
        let verified =
            match eddsa_verify_result(&key_algorithm, &public_key_data, &signature, &data) {
                Ok(verified) => verified,
                Err(error_message) => {
                    let error =
                        v8::String::new(scope, &format!("verify: {}", error_message)).unwrap();
                    let error_obj = v8::Exception::error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };
        v8::Boolean::new(scope, verified)
    } else if algo_name == "ECDSA" || key_algorithm == "ECDSA" {
        let curve_name = get_curve_name(scope, key_obj);
        let hash_name = get_algorithm_hash_name(scope, algo_value);
        let public_key_data = match get_key_data(scope, key_obj) {
            Some(data) => data,
            None => {
                let error =
                    v8::String::new(scope, "verify: ECDSA key data is unavailable").unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        };
        let verified =
            match ecdsa_verify_result(&curve_name, &public_key_data, &hash_name, &signature, &data)
            {
                Ok(verified) => verified,
                Err(error_message) => {
                    let error =
                        v8::String::new(scope, &format!("verify: {}", error_message)).unwrap();
                    let error_obj = v8::Exception::error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };
        v8::Boolean::new(scope, verified)
    } else if is_rsassa_algorithm_name(&algo_name) || is_rsassa_algorithm_name(&key_algorithm) {
        if key_type != "public" {
            let error = v8::String::new(scope, "verify: RSASSA requires a public key").unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
        let public_key_data = match get_key_data(scope, key_obj) {
            Some(data) => data,
            None => {
                let error = v8::String::new(scope, "verify: RSA key data is unavailable").unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        };
        let hash_name = rsa_hash_name_for_operation(scope, algo_value, key_obj);
        let verified = match rsa_verify_result(&public_key_data, &hash_name, &signature, &data) {
            Ok(verified) => verified,
            Err(error_message) => {
                let error = v8::String::new(scope, &format!("verify: {}", error_message)).unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        };
        v8::Boolean::new(scope, verified)
    } else if key_type == "public" || algo_name.starts_with("RSA") {
        // Unsupported RSA algorithms still fail closed; never report fake success.
        v8::Boolean::new(scope, false)
    } else {
        // HMAC verification
        let Some(key_data) = get_required_key_data(scope, "verify", key_obj) else {
            return;
        };

        use ring::hmac;
        let hash_name = get_key_hmac_hash_name(scope, key_obj);
        let hmac_algorithm = match ring_hmac_algorithm(&hash_name) {
            Ok(algorithm) => algorithm,
            Err(error_message) => {
                let error = v8::String::new(scope, &format!("verify: {}", error_message)).unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        };
        let sign_key = hmac::Key::new(hmac_algorithm, &key_data);
        let tag = hmac::sign(&sign_key, &data);

        #[allow(deprecated)]
        let result = ring::constant_time::verify_slices_are_equal(tag.as_ref(), &signature).is_ok();
        v8::Boolean::new(scope, result)
    };

    let resolver = v8::PromiseResolver::new(scope).unwrap();
    resolver.resolve(scope, result_bool.into());
    let promise = resolver.get_promise(scope);
    retval.set(promise.into());
}

/// AES-GCM encrypt callback - real cryptographic encryption using ring
fn aes_encrypt_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if args.length() < 3 {
        let error =
            v8::String::new(scope, "encrypt requires algorithm, key, and data arguments").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let algo_value = args.get(0);
    let key_value = args.get(1);
    let data_value = args.get(2);

    if !key_value.is_object() {
        let error = v8::String::new(scope, "encrypt: key must be a CryptoKey").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    // Get algorithm name and IV from algorithm
    let algo_name = get_algorithm_name(scope, algo_value);
    let algo_obj = if algo_value.is_object() {
        Some(algo_value.to_object(scope).unwrap())
    } else {
        None
    };

    // Get plaintext data
    let data = match get_array_buffer_data(scope, data_value) {
        Some(d) => d,
        None => {
            let error =
                v8::String::new(scope, "encrypt: data must be an ArrayBuffer or TypedArray")
                    .unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
    };

    if !algo_name.eq_ignore_ascii_case("AES-GCM")
        && !algo_name.eq_ignore_ascii_case("AES-CBC")
        && !algo_name.eq_ignore_ascii_case("AES-CTR")
        && !is_rsa_oaep_algorithm_name(&algo_name)
    {
        let error = v8::String::new(
            scope,
            &format!("encrypt: algorithm '{}' is not implemented", algo_name),
        )
        .unwrap();
        let error_obj = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let key_obj = key_value.to_object(scope).unwrap();
    if !ensure_key_algorithm_matches(scope, "encrypt", &algo_name, key_obj) {
        return;
    }
    if !ensure_crypto_key_usage(scope, "encrypt", key_obj, "encrypt") {
        return;
    }

    let Some(key_bytes) = get_required_key_data(scope, "encrypt", key_obj) else {
        return;
    };
    if is_rsa_oaep_algorithm_name(&algo_name) {
        let key_type = get_key_type(scope, key_obj);
        if key_type != "public" {
            let error = v8::String::new(scope, "encrypt: RSA-OAEP requires a public key").unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }

        let hash_name = get_key_hash_name(scope, key_obj);
        let label = match get_optional_algorithm_label(scope, "encrypt", algo_obj.as_ref()) {
            Some(label) => label,
            None => return,
        };
        match rsa_oaep_encrypt_result(&key_bytes, &hash_name, label.as_deref(), &data) {
            Ok(ciphertext) => {
                let array_buffer = v8::ArrayBuffer::new(scope, ciphertext.len());
                let backing_store = array_buffer.get_backing_store();
                for (i, &byte) in ciphertext.iter().enumerate() {
                    backing_store[i].set(byte);
                }

                let resolver = v8::PromiseResolver::new(scope).unwrap();
                resolver.resolve(scope, array_buffer.into());
                let promise = resolver.get_promise(scope);
                retval.set(promise.into());
            }
            Err(error_message) => {
                let error = v8::String::new(scope, &format!("encrypt: {}", error_message)).unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
            }
        }
        return;
    }

    if algo_name.eq_ignore_ascii_case("AES-CTR") {
        let counter = match get_required_algorithm_bytes_property(
            scope,
            "encrypt",
            "AES-CTR",
            algo_obj.as_ref(),
            "counter",
            16,
        ) {
            Some(counter) => counter,
            None => return,
        };
        let length_bits = match get_aes_ctr_length_bits(scope, "encrypt", algo_obj.as_ref()) {
            Some(length_bits) => length_bits,
            None => return,
        };
        let Some(cipher) = aes_ctr_cipher_for_key_len(key_bytes.len()) else {
            let error = v8::String::new(
                scope,
                "encrypt: invalid key length for AES-CTR (must be 128, 192, or 256 bits)",
            )
            .unwrap();
            let error_obj = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        };

        if !aes_ctr_counter_has_capacity(&counter, length_bits, data.len()) {
            let error =
                v8::String::new(scope, "encrypt: AES-CTR counter range exhausted for length")
                    .unwrap();
            let error_obj = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }

        match aes_ctr_transform(cipher, &key_bytes, &counter, &data) {
            Ok(ciphertext) => {
                let array_buffer = v8::ArrayBuffer::new(scope, ciphertext.len());
                let backing_store = array_buffer.get_backing_store();
                for (i, &byte) in ciphertext.iter().enumerate() {
                    backing_store[i].set(byte);
                }

                let resolver = v8::PromiseResolver::new(scope).unwrap();
                resolver.resolve(scope, array_buffer.into());
                let promise = resolver.get_promise(scope);
                retval.set(promise.into());
            }
            Err(e) => {
                let error =
                    v8::String::new(scope, &format!("encrypt: AES-CTR encryption failed: {}", e))
                        .unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
            }
        }
        return;
    }

    if algo_name.eq_ignore_ascii_case("AES-CBC") {
        let iv = match get_required_algorithm_iv(scope, "encrypt", "AES-CBC", algo_obj.as_ref(), 16)
        {
            Some(iv) => iv,
            None => return,
        };
        let Some(cipher) = aes_cbc_cipher_for_key_len(key_bytes.len()) else {
            let error = v8::String::new(
                scope,
                "encrypt: invalid key length for AES-CBC (must be 128, 192, or 256 bits)",
            )
            .unwrap();
            let error_obj = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        };

        match openssl_encrypt(cipher, &key_bytes, Some(&iv), &data) {
            Ok(ciphertext) => {
                let array_buffer = v8::ArrayBuffer::new(scope, ciphertext.len());
                let backing_store = array_buffer.get_backing_store();
                for (i, &byte) in ciphertext.iter().enumerate() {
                    backing_store[i].set(byte);
                }

                let resolver = v8::PromiseResolver::new(scope).unwrap();
                resolver.resolve(scope, array_buffer.into());
                let promise = resolver.get_promise(scope);
                retval.set(promise.into());
            }
            Err(e) => {
                let error =
                    v8::String::new(scope, &format!("encrypt: AES-CBC encryption failed: {}", e))
                        .unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
            }
        }
        return;
    }

    // Beejs' AES-GCM backend currently supports the standard 96-bit
    // nonce only; never substitute an all-zero nonce.
    let iv = match get_required_algorithm_iv(scope, "encrypt", "AES-GCM", algo_obj.as_ref(), 12) {
        Some(iv) => iv,
        None => return,
    };

    // Get additional authenticated data (AAD) if present.
    let aad = if let Some(ref obj) = algo_obj {
        let aad_key = v8::String::new(scope, "additionalData").unwrap();
        if let Some(aad_val) = obj.get(scope, aad_key.into()) {
            get_array_buffer_data(scope, aad_val)
        } else {
            None
        }
    } else {
        None
    };

    let algorithm: &'static Algorithm = if key_bytes.len() == 32 {
        &AES_256_GCM
    } else if key_bytes.len() == 16 {
        &AES_128_GCM
    } else {
        let error = v8::String::new(
            scope,
            "encrypt: invalid key length for AES-GCM (must be 128 or 256 bits)",
        )
        .unwrap();
        let error_obj = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    };

    match UnboundKey::new(algorithm, &key_bytes) {
        Ok(unbound_key) => {
            let less_safe_key = LessSafeKey::new(unbound_key);
            let nonce = Nonce::assume_unique_for_key(iv.try_into().unwrap());

            // Encrypt with optional AAD
            let aad_ref = aad
                .as_ref()
                .map(|v| Aad::from(v.as_slice()))
                .unwrap_or_else(|| Aad::from(&[][..]));
            let mut plaintext = data.clone();
            let result = less_safe_key.seal_in_place_append_tag(nonce, aad_ref, &mut plaintext);

            match result {
                Ok(()) => {
                    let array_buffer = v8::ArrayBuffer::new(scope, plaintext.len());
                    let backing_store = array_buffer.get_backing_store();
                    for (i, &byte) in plaintext.iter().enumerate() {
                        backing_store[i].set(byte);
                    }

                    let resolver = v8::PromiseResolver::new(scope).unwrap();
                    resolver.resolve(scope, array_buffer.into());
                    let promise = resolver.get_promise(scope);
                    retval.set(promise.into());
                }
                Err(e) => {
                    let error =
                        v8::String::new(scope, &format!("encrypt: encryption failed: {:?}", e))
                            .unwrap();
                    let error_obj = v8::Exception::error(scope, error);
                    scope.throw_exception(error_obj.into());
                }
            }
        }
        Err(e) => {
            let error = v8::String::new(scope, &format!("encrypt: invalid key: {:?}", e)).unwrap();
            let error_obj = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
        }
    }
}

/// AES-GCM decrypt callback - real cryptographic decryption using ring
fn aes_decrypt_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if args.length() < 3 {
        let error =
            v8::String::new(scope, "decrypt requires algorithm, key, and data arguments").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let algo_value = args.get(0);
    let key_value = args.get(1);
    let data_value = args.get(2);

    if !key_value.is_object() {
        let error = v8::String::new(scope, "decrypt: key must be a CryptoKey").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    // Get algorithm name and IV from algorithm
    let algo_name = get_algorithm_name(scope, algo_value);
    let algo_obj = if algo_value.is_object() {
        Some(algo_value.to_object(scope).unwrap())
    } else {
        None
    };

    let encrypted_data = match get_array_buffer_data(scope, data_value) {
        Some(d) => d,
        None => {
            let error =
                v8::String::new(scope, "decrypt: data must be an ArrayBuffer or TypedArray")
                    .unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
    };

    if !algo_name.eq_ignore_ascii_case("AES-GCM")
        && !algo_name.eq_ignore_ascii_case("AES-CBC")
        && !algo_name.eq_ignore_ascii_case("AES-CTR")
        && !is_rsa_oaep_algorithm_name(&algo_name)
    {
        let error = v8::String::new(
            scope,
            &format!("decrypt: algorithm '{}' is not implemented", algo_name),
        )
        .unwrap();
        let error_obj = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let key_obj = key_value.to_object(scope).unwrap();
    if !ensure_key_algorithm_matches(scope, "decrypt", &algo_name, key_obj) {
        return;
    }
    if !ensure_crypto_key_usage(scope, "decrypt", key_obj, "decrypt") {
        return;
    }

    let Some(key_bytes) = get_required_key_data(scope, "decrypt", key_obj) else {
        return;
    };
    if is_rsa_oaep_algorithm_name(&algo_name) {
        let key_type = get_key_type(scope, key_obj);
        if key_type != "private" {
            let error = v8::String::new(scope, "decrypt: RSA-OAEP requires a private key").unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }

        let hash_name = get_key_hash_name(scope, key_obj);
        let label = match get_optional_algorithm_label(scope, "decrypt", algo_obj.as_ref()) {
            Some(label) => label,
            None => return,
        };
        match rsa_oaep_decrypt_result(&key_bytes, &hash_name, label.as_deref(), &encrypted_data) {
            Ok(plaintext) => {
                let array_buffer = v8::ArrayBuffer::new(scope, plaintext.len());
                let backing_store = array_buffer.get_backing_store();
                for (i, &byte) in plaintext.iter().enumerate() {
                    backing_store[i].set(byte);
                }

                let resolver = v8::PromiseResolver::new(scope).unwrap();
                resolver.resolve(scope, array_buffer.into());
                let promise = resolver.get_promise(scope);
                retval.set(promise.into());
            }
            Err(error_message) => {
                let error = v8::String::new(scope, &format!("decrypt: {}", error_message)).unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
            }
        }
        return;
    }

    if algo_name.eq_ignore_ascii_case("AES-CTR") {
        let counter = match get_required_algorithm_bytes_property(
            scope,
            "decrypt",
            "AES-CTR",
            algo_obj.as_ref(),
            "counter",
            16,
        ) {
            Some(counter) => counter,
            None => return,
        };
        let length_bits = match get_aes_ctr_length_bits(scope, "decrypt", algo_obj.as_ref()) {
            Some(length_bits) => length_bits,
            None => return,
        };
        let Some(cipher) = aes_ctr_cipher_for_key_len(key_bytes.len()) else {
            let error = v8::String::new(
                scope,
                "decrypt: invalid key length for AES-CTR (must be 128, 192, or 256 bits)",
            )
            .unwrap();
            let error_obj = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        };

        if !aes_ctr_counter_has_capacity(&counter, length_bits, encrypted_data.len()) {
            let error =
                v8::String::new(scope, "decrypt: AES-CTR counter range exhausted for length")
                    .unwrap();
            let error_obj = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }

        match aes_ctr_transform(cipher, &key_bytes, &counter, &encrypted_data) {
            Ok(plaintext) => {
                let array_buffer = v8::ArrayBuffer::new(scope, plaintext.len());
                let backing_store = array_buffer.get_backing_store();
                for (i, &byte) in plaintext.iter().enumerate() {
                    backing_store[i].set(byte);
                }

                let resolver = v8::PromiseResolver::new(scope).unwrap();
                resolver.resolve(scope, array_buffer.into());
                let promise = resolver.get_promise(scope);
                retval.set(promise.into());
            }
            Err(e) => {
                let error =
                    v8::String::new(scope, &format!("decrypt: AES-CTR decryption failed: {}", e))
                        .unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
            }
        }
        return;
    }

    if algo_name.eq_ignore_ascii_case("AES-CBC") {
        let iv = match get_required_algorithm_iv(scope, "decrypt", "AES-CBC", algo_obj.as_ref(), 16)
        {
            Some(iv) => iv,
            None => return,
        };
        let Some(cipher) = aes_cbc_cipher_for_key_len(key_bytes.len()) else {
            let error = v8::String::new(
                scope,
                "decrypt: invalid key length for AES-CBC (must be 128, 192, or 256 bits)",
            )
            .unwrap();
            let error_obj = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        };

        match openssl_decrypt(cipher, &key_bytes, Some(&iv), &encrypted_data) {
            Ok(plaintext) => {
                let array_buffer = v8::ArrayBuffer::new(scope, plaintext.len());
                let backing_store = array_buffer.get_backing_store();
                for (i, &byte) in plaintext.iter().enumerate() {
                    backing_store[i].set(byte);
                }

                let resolver = v8::PromiseResolver::new(scope).unwrap();
                resolver.resolve(scope, array_buffer.into());
                let promise = resolver.get_promise(scope);
                retval.set(promise.into());
            }
            Err(e) => {
                let error =
                    v8::String::new(scope, &format!("decrypt: AES-CBC decryption failed: {}", e))
                        .unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
            }
        }
        return;
    }

    // Beejs' AES-GCM backend currently supports the standard 96-bit
    // nonce only; never substitute an all-zero nonce.
    let iv = match get_required_algorithm_iv(scope, "decrypt", "AES-GCM", algo_obj.as_ref(), 12) {
        Some(iv) => iv,
        None => return,
    };

    // Get additional authenticated data (AAD) if present.
    let aad = if let Some(ref obj) = algo_obj {
        let aad_key = v8::String::new(scope, "additionalData").unwrap();
        if let Some(aad_val) = obj.get(scope, aad_key.into()) {
            get_array_buffer_data(scope, aad_val)
        } else {
            None
        }
    } else {
        None
    };

    let algorithm: &'static Algorithm = if key_bytes.len() == 32 {
        &AES_256_GCM
    } else if key_bytes.len() == 16 {
        &AES_128_GCM
    } else {
        let error = v8::String::new(
            scope,
            "decrypt: invalid key length for AES-GCM (must be 128 or 256 bits)",
        )
        .unwrap();
        let error_obj = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    };

    match UnboundKey::new(algorithm, &key_bytes) {
        Ok(unbound_key) => {
            let less_safe_key = LessSafeKey::new(unbound_key);
            let nonce = Nonce::assume_unique_for_key(iv.try_into().unwrap());

            // Decrypt with optional AAD
            let aad_ref = aad
                .as_ref()
                .map(|v| Aad::from(v.as_slice()))
                .unwrap_or_else(|| Aad::from(&[][..]));
            let mut ciphertext = encrypted_data.clone();
            let result = less_safe_key.open_in_place(nonce, aad_ref, &mut ciphertext);

            match result {
                Ok(plaintext) => {
                    let array_buffer = v8::ArrayBuffer::new(scope, plaintext.len());
                    let backing_store = array_buffer.get_backing_store();
                    for (i, &byte) in plaintext.iter().enumerate() {
                        backing_store[i].set(byte);
                    }

                    let resolver = v8::PromiseResolver::new(scope).unwrap();
                    resolver.resolve(scope, array_buffer.into());
                    let promise = resolver.get_promise(scope);
                    retval.set(promise.into());
                }
                Err(e) => {
                    let error = v8::String::new(scope, &format!("decrypt: decryption failed (authentication failed or data corrupted): {:?}", e)).unwrap();
                    let error_obj = v8::Exception::error(scope, error);
                    scope.throw_exception(error_obj.into());
                }
            }
        }
        Err(e) => {
            let error = v8::String::new(scope, &format!("decrypt: invalid key: {:?}", e)).unwrap();
            let error_obj = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
        }
    }
}

/// wrapKey callback - wraps (encrypts) a key for secure storage/transport
fn wrap_key_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if args.length() < 4 {
        let error = v8::String::new(
            scope,
            "wrapKey requires format, key, wrappingKey, and algorithm arguments",
        )
        .unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let format_value = args.get(0);
    let key_value = args.get(1);
    let wrapping_key_value = args.get(2);
    let algo_value = args.get(3);

    // Get format
    let format_str = if format_value.is_string() {
        format_value
            .to_string(scope)
            .unwrap()
            .to_rust_string_lossy(scope)
    } else {
        let error = v8::String::new(scope, "wrapKey: format must be a string").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    };
    if format_str != "raw" && format_str != "jwk" {
        let error = v8::String::new(
            scope,
            &format!("wrapKey: unsupported format '{}'", format_str),
        )
        .unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    // Get algorithm name and IV
    let algo_obj = if algo_value.is_object() {
        Some(algo_value.to_object(scope).unwrap())
    } else {
        None
    };

    let algo_name = get_algorithm_name(scope, algo_value);

    let iv = if algo_name.eq_ignore_ascii_case("AES-GCM") {
        match algo_obj.as_ref() {
            Some(obj) => {
                let iv_key = v8::String::new(scope, "iv").unwrap();
                match obj.get(scope, iv_key.into()) {
                    Some(iv_val) => match get_array_buffer_data(scope, iv_val) {
                        Some(iv_data) if iv_data.len() == 12 => iv_data,
                        Some(_) => {
                            let error =
                                v8::String::new(scope, "wrapKey: AES-GCM iv must be 12 bytes")
                                    .unwrap();
                            let error_obj = v8::Exception::error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                        None => {
                            let error = v8::String::new(
                                scope,
                                "wrapKey: AES-GCM iv must be an ArrayBuffer or TypedArray",
                            )
                            .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    },
                    None => {
                        let error = v8::String::new(scope, "wrapKey: AES-GCM requires iv").unwrap();
                        let error_obj = v8::Exception::error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                }
            }
            None => {
                let error = v8::String::new(scope, "wrapKey: AES-GCM requires iv").unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        }
    } else {
        vec![0u8; 12]
    };

    // Export key material into the requested wrapping format.
    let key_obj = key_value.to_object(scope).unwrap();
    let Some(key_bytes) = get_required_key_data(scope, "wrapKey", key_obj) else {
        return;
    };
    let key_payload = match export_key_payload_for_wrap(scope, &format_str, key_obj, &key_bytes) {
        Ok(payload) => payload,
        Err(error_message) => {
            let error = v8::String::new(scope, &format!("wrapKey: {}", error_message)).unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
    };

    // Get wrapping key data
    let wrapping_key_obj = wrapping_key_value.to_object(scope).unwrap();
    if !crypto_key_has_usage(scope, wrapping_key_obj, "wrapKey") {
        let error =
            v8::String::new(scope, "wrapKey: wrapping key does not allow wrapKey usage").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }
    if !ensure_key_algorithm_matches(scope, "wrapKey", &algo_name, wrapping_key_obj) {
        return;
    }

    let Some(wrapping_key_bytes) = get_required_key_data(scope, "wrapKey", wrapping_key_obj) else {
        return;
    };
    if algo_name.eq_ignore_ascii_case("AES-KW") {
        let Some(cipher) = aes_kw_cipher_for_key_len(wrapping_key_bytes.len()) else {
            let error = v8::String::new(
                scope,
                "wrapKey: invalid key length for AES-KW (must be 128, 192, or 256 bits)",
            )
            .unwrap();
            let error_obj = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        };

        match aes_kw_wrap_key_data(cipher, &wrapping_key_bytes, &key_payload) {
            Ok(wrapped_key) => {
                let array_buffer = v8::ArrayBuffer::new(scope, wrapped_key.len());
                let backing_store = array_buffer.get_backing_store();
                for (i, &byte) in wrapped_key.iter().enumerate() {
                    backing_store[i].set(byte);
                }

                let resolver = v8::PromiseResolver::new(scope).unwrap();
                resolver.resolve(scope, array_buffer.into());
                let promise = resolver.get_promise(scope);
                retval.set(promise.into());
            }
            Err(error_message) => {
                let error = v8::String::new(
                    scope,
                    &format!("wrapKey: AES-KW encryption failed: {}", error_message),
                )
                .unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
            }
        }
    } else if algo_name.eq_ignore_ascii_case("AES-GCM") {
        let algorithm: &'static Algorithm = if wrapping_key_bytes.len() == 32 {
            &AES_256_GCM
        } else if wrapping_key_bytes.len() == 16 {
            &AES_128_GCM
        } else {
            let error = v8::String::new(
                scope,
                "wrapKey: invalid key length for AES-GCM (must be 128 or 256 bits)",
            )
            .unwrap();
            let error_obj = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        };

        match UnboundKey::new(algorithm, &wrapping_key_bytes) {
            Ok(unbound_key) => {
                let less_safe_key = LessSafeKey::new(unbound_key);
                let nonce = Nonce::assume_unique_for_key(
                    iv.clone().try_into().expect("validated AES-GCM iv length"),
                );

                let mut plaintext = key_payload.clone();
                let result = less_safe_key.seal_in_place_append_tag(
                    nonce,
                    Aad::from(&[][..]),
                    &mut plaintext,
                );

                match result {
                    Ok(_) => {
                        let array_buffer = v8::ArrayBuffer::new(scope, plaintext.len());
                        let backing_store = array_buffer.get_backing_store();
                        for (i, &byte) in plaintext.iter().enumerate() {
                            backing_store[i].set(byte);
                        }

                        let resolver = v8::PromiseResolver::new(scope).unwrap();
                        resolver.resolve(scope, array_buffer.into());
                        let promise = resolver.get_promise(scope);
                        retval.set(promise.into());
                    }
                    Err(_) => {
                        let error = v8::String::new(scope, "wrapKey: encryption failed").unwrap();
                        let error_obj = v8::Exception::error(scope, error);
                        scope.throw_exception(error_obj.into());
                    }
                }
            }
            Err(_) => {
                let error =
                    v8::String::new(scope, "wrapKey: failed to create encryption key").unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
            }
        }
    } else {
        let error = v8::String::new(
            scope,
            "wrapKey: only AES-GCM and AES-KW algorithms are supported",
        )
        .unwrap();
        let error_obj = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
    }
}

/// unwrapKey callback - unwraps (decrypts) a wrapped key
fn unwrap_key_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if args.length() < 6 {
        let error = v8::String::new(scope, "unwrapKey requires format, wrappedKey, unwrappingKey, unwrapAlgorithm, keyAlgorithm, and extractable arguments").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let format_value = args.get(0);
    let wrapped_key_value = args.get(1);
    let unwrapping_key_value = args.get(2);
    let unwrap_algo_value = args.get(3);
    let key_algo_value = args.get(4);
    let extractable_value = args.get(5);
    let usages_value = args.get(6);

    // Get format
    let format_str = if format_value.is_string() {
        format_value
            .to_string(scope)
            .unwrap()
            .to_rust_string_lossy(scope)
    } else {
        let error = v8::String::new(scope, "unwrapKey: format must be a string").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    };
    if format_str != "raw" && format_str != "jwk" {
        let error = v8::String::new(
            scope,
            &format!("unwrapKey: unsupported format '{}'", format_str),
        )
        .unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    // Get unwrap algorithm name and IV
    let unwrap_algo_name = match get_algorithm_name_option(scope, unwrap_algo_value) {
        Some(name) => name,
        None => {
            throw_missing_algorithm_name(scope, "unwrapKey");
            return;
        }
    };
    let unwrap_algo_obj = if unwrap_algo_value.is_object() {
        Some(unwrap_algo_value.to_object(scope).unwrap())
    } else {
        None
    };

    let unwrap_iv = if unwrap_algo_name.eq_ignore_ascii_case("AES-GCM") {
        match unwrap_algo_obj.as_ref() {
            Some(obj) => {
                let iv_key = v8::String::new(scope, "iv").unwrap();
                match obj.get(scope, iv_key.into()) {
                    Some(iv_val) => match get_array_buffer_data(scope, iv_val) {
                        Some(iv_data) if iv_data.len() == 12 => iv_data,
                        Some(_) => {
                            let error =
                                v8::String::new(scope, "unwrapKey: AES-GCM iv must be 12 bytes")
                                    .unwrap();
                            let error_obj = v8::Exception::error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                        None => {
                            let error = v8::String::new(
                                scope,
                                "unwrapKey: AES-GCM iv must be an ArrayBuffer or TypedArray",
                            )
                            .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    },
                    None => {
                        let error =
                            v8::String::new(scope, "unwrapKey: AES-GCM requires iv").unwrap();
                        let error_obj = v8::Exception::error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                }
            }
            None => {
                let error = v8::String::new(scope, "unwrapKey: AES-GCM requires iv").unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        }
    } else {
        Vec::new()
    };

    // Get wrapped key data
    let wrapped_key_data = match get_array_buffer_data(scope, wrapped_key_value) {
        Some(d) => d,
        None => {
            let error = v8::String::new(
                scope,
                "unwrapKey: wrappedKey must be an ArrayBuffer or TypedArray",
            )
            .unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
    };

    if wrapped_key_data.len() < 16 {
        let error = v8::String::new(scope, "unwrapKey: wrapped key data is too short").unwrap();
        let error_obj = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    // Get unwrapping key data
    let unwrapping_key_obj = unwrapping_key_value.to_object(scope).unwrap();
    if !crypto_key_has_usage(scope, unwrapping_key_obj, "unwrapKey") {
        let error = v8::String::new(
            scope,
            "unwrapKey: unwrapping key does not allow unwrapKey usage",
        )
        .unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }
    if !ensure_key_algorithm_matches(scope, "unwrapKey", &unwrap_algo_name, unwrapping_key_obj) {
        return;
    }

    let Some(unwrapping_key_bytes) = get_required_key_data(scope, "unwrapKey", unwrapping_key_obj)
    else {
        return;
    };
    if unwrap_algo_name.eq_ignore_ascii_case("AES-KW") {
        let Some(cipher) = aes_kw_cipher_for_key_len(unwrapping_key_bytes.len()) else {
            let error = v8::String::new(
                scope,
                "unwrapKey: invalid key length for AES-KW (must be 128, 192, or 256 bits)",
            )
            .unwrap();
            let error_obj = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        };

        match aes_kw_unwrap_key_data(cipher, &unwrapping_key_bytes, &wrapped_key_data) {
            Ok(unwrapped_key_data) => {
                resolve_unwrapped_secret_key_promise(
                    scope,
                    &mut retval,
                    &format_str,
                    key_algo_value,
                    extractable_value,
                    usages_value,
                    &unwrapped_key_data,
                );
            }
            Err(error_message) => {
                let error = v8::String::new(
                    scope,
                    &format!(
                        "unwrapKey: AES-KW decryption failed - invalid key or corrupted data: {}",
                        error_message
                    ),
                )
                .unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
            }
        }
    } else if unwrap_algo_name.eq_ignore_ascii_case("AES-GCM") {
        let algorithm: &'static Algorithm = if unwrapping_key_bytes.len() == 32 {
            &AES_256_GCM
        } else if unwrapping_key_bytes.len() == 16 {
            &AES_128_GCM
        } else {
            let error = v8::String::new(
                scope,
                "unwrapKey: invalid key length for AES-GCM (must be 128 or 256 bits)",
            )
            .unwrap();
            let error_obj = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        };

        match UnboundKey::new(algorithm, &unwrapping_key_bytes) {
            Ok(unbound_key) => {
                let less_safe_key = LessSafeKey::new(unbound_key);
                let nonce = Nonce::assume_unique_for_key(
                    unwrap_iv
                        .try_into()
                        .expect("validated AES-GCM unwrap iv length"),
                );

                let mut encrypted_data = wrapped_key_data.clone();
                let result =
                    less_safe_key.open_in_place(nonce, Aad::from(&[][..]), &mut encrypted_data);

                match result {
                    Ok(unencrypted_data) => {
                        resolve_unwrapped_secret_key_promise(
                            scope,
                            &mut retval,
                            &format_str,
                            key_algo_value,
                            extractable_value,
                            usages_value,
                            unencrypted_data,
                        );
                    }
                    Err(_) => {
                        let error = v8::String::new(
                            scope,
                            "unwrapKey: decryption failed - invalid key or corrupted data",
                        )
                        .unwrap();
                        let error_obj = v8::Exception::error(scope, error);
                        scope.throw_exception(error_obj.into());
                    }
                }
            }
            Err(_) => {
                let error =
                    v8::String::new(scope, "unwrapKey: failed to create decryption key").unwrap();
                let error_obj = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
            }
        }
    } else {
        let error = v8::String::new(
            scope,
            "unwrapKey: only AES-GCM and AES-KW algorithms are supported",
        )
        .unwrap();
        let error_obj = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
    }
}

/// Setup crypto.subtle API
fn setup_crypto_subtle_api(scope: &mut v8::PinScope, subtle_obj: &v8::Object) {
    // digest method
    let digest_key = v8::String::new(scope, "digest").unwrap();
    let digest_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            // Inline digest implementation to avoid lifetime issues
            if args.length() < 2 {
                let error =
                    v8::String::new(scope, "digest requires algorithm and data arguments").unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            let algo_value = args.get(0);
            let data_value = args.get(1);

            // Get hash algorithm
            let hash_name = get_algorithm_hash_name(scope, algo_value);

            // Get data
            let data = match get_array_buffer_data(scope, data_value) {
                Some(d) => d,
                None => {
                    let error = v8::String::new(
                        scope,
                        "digest requires an ArrayBuffer or TypedArray as data",
                    )
                    .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            // Compute hash
            let hash_result = compute_sha_digest(&data, &hash_name);

            match hash_result {
                Ok(hash) => {
                    let array_buffer = v8::ArrayBuffer::new(scope, hash.len());
                    let backing_store = array_buffer.get_backing_store();
                    for (i, byte) in hash.iter().enumerate() {
                        backing_store[i].set(*byte);
                    }
                    let uint8_array = match v8::Uint8Array::new(scope, array_buffer, 0, hash.len())
                    {
                        Some(arr) => arr,
                        None => {
                            let error =
                                v8::String::new(scope, "Failed to create Uint8Array").unwrap();
                            let error_obj = v8::Exception::error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };
                    // Inline create_resolved_promise to avoid lifetime issues
                    let resolver = v8::PromiseResolver::new(scope).unwrap();
                    resolver.resolve(scope, uint8_array.into());
                    let promise = resolver.get_promise(scope);
                    retval.set(promise.into());
                }
                Err(e) => {
                    let error = v8::String::new(scope, &e).unwrap();
                    let error_obj = v8::Exception::error(scope, error);
                    scope.throw_exception(error_obj.into());
                }
            }
        },
    );
    let digest_fn_instance = digest_fn.get_function(scope).unwrap();
    subtle_obj.set(scope, digest_key.into(), digest_fn_instance.into());

    // importKey method - fully implemented
    let import_key_key = v8::String::new(scope, "importKey").unwrap();
    let import_key_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, rv: v8::ReturnValue| {
            import_key_callback(scope, args, rv);
        },
    );
    let import_key_fn_instance = import_key_fn.get_function(scope).unwrap();
    subtle_obj.set(scope, import_key_key.into(), import_key_fn_instance.into());

    // encrypt method - implemented for AES-GCM
    let encrypt_key = v8::String::new(scope, "encrypt").unwrap();
    let encrypt_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, rv: v8::ReturnValue| {
            aes_encrypt_callback(scope, args, rv);
        },
    );
    let encrypt_fn_instance = encrypt_fn.get_function(scope).unwrap();
    subtle_obj.set(scope, encrypt_key.into(), encrypt_fn_instance.into());

    // decrypt method - implemented for AES-GCM
    let decrypt_key = v8::String::new(scope, "decrypt").unwrap();
    let decrypt_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, rv: v8::ReturnValue| {
            aes_decrypt_callback(scope, args, rv);
        },
    );
    let decrypt_fn_instance = decrypt_fn.get_function(scope).unwrap();
    subtle_obj.set(scope, decrypt_key.into(), decrypt_fn_instance.into());

    // sign method - implemented for HMAC
    let sign_key = v8::String::new(scope, "sign").unwrap();
    let sign_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, rv: v8::ReturnValue| {
            hmac_sign_callback(scope, args, rv);
        },
    );
    let sign_fn_instance = sign_fn.get_function(scope).unwrap();
    subtle_obj.set(scope, sign_key.into(), sign_fn_instance.into());

    // verify method - implemented for HMAC
    let verify_key = v8::String::new(scope, "verify").unwrap();
    let verify_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, rv: v8::ReturnValue| {
            hmac_verify_callback(scope, args, rv);
        },
    );
    let verify_fn_instance = verify_fn.get_function(scope).unwrap();
    subtle_obj.set(scope, verify_key.into(), verify_fn_instance.into());

    // generateKey method - fully implemented
    let generate_key_key = v8::String::new(scope, "generateKey").unwrap();
    let generate_key_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, rv: v8::ReturnValue| {
            generate_key_callback(scope, args, rv);
        },
    );
    let generate_key_fn_instance = generate_key_fn.get_function(scope).unwrap();
    subtle_obj.set(
        scope,
        generate_key_key.into(),
        generate_key_fn_instance.into(),
    );

    // deriveKey method - fully implemented (PBKDF2)
    let derive_key_key = v8::String::new(scope, "deriveKey").unwrap();
    let derive_key_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, rv: v8::ReturnValue| {
            derive_key_callback(scope, args, rv);
        },
    );
    let derive_key_fn_instance = derive_key_fn.get_function(scope).unwrap();
    subtle_obj.set(scope, derive_key_key.into(), derive_key_fn_instance.into());

    // exportKey method - fully implemented
    let export_key_key = v8::String::new(scope, "exportKey").unwrap();
    let export_key_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, rv: v8::ReturnValue| {
            export_key_callback(scope, args, rv);
        },
    );
    let export_key_fn_instance = export_key_fn.get_function(scope).unwrap();
    subtle_obj.set(scope, export_key_key.into(), export_key_fn_instance.into());

    // wrapKey method - fully implemented (AES-GCM)
    let wrap_key_key = v8::String::new(scope, "wrapKey").unwrap();
    let wrap_key_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, rv: v8::ReturnValue| {
            wrap_key_callback(scope, args, rv);
        },
    );
    let wrap_key_fn_instance = wrap_key_fn.get_function(scope).unwrap();
    subtle_obj.set(scope, wrap_key_key.into(), wrap_key_fn_instance.into());

    // unwrapKey method - fully implemented (AES-GCM)
    let unwrap_key_key = v8::String::new(scope, "unwrapKey").unwrap();
    let unwrap_key_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, rv: v8::ReturnValue| {
            unwrap_key_callback(scope, args, rv);
        },
    );
    let unwrap_key_fn_instance = unwrap_key_fn.get_function(scope).unwrap();
    subtle_obj.set(scope, unwrap_key_key.into(), unwrap_key_fn_instance.into());

    // deriveBits method - fully implemented (PBKDF2)
    let derive_bits_key = v8::String::new(scope, "deriveBits").unwrap();
    let derive_bits_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, rv: v8::ReturnValue| {
            derive_bits_callback(scope, args, rv);
        },
    );
    let derive_bits_fn_instance = derive_bits_fn.get_function(scope).unwrap();
    subtle_obj.set(
        scope,
        derive_bits_key.into(),
        derive_bits_fn_instance.into(),
    );
}

/// Generate random bytes for key material
fn generate_random_bytes(length: usize) -> Vec<u8> {
    let mut data = vec![0u8; length];
    use ring::rand::SecureRandom;
    let rng = ring::rand::SystemRandom::new();
    let _ = rng.fill(&mut data);
    data
}

/// Get algorithm length from algorithm object
fn get_algorithm_length(
    scope: &mut v8::PinScope,
    algo_value: v8::Local<v8::Value>,
    default_length: i32,
) -> i32 {
    if algo_value.is_object() {
        let algo_obj = algo_value.to_object(scope).unwrap();
        let length_key = v8::String::new(scope, "length").unwrap();
        if let Some(length_val) = algo_obj.get(scope, length_key.into()) {
            if length_val.is_number() {
                return length_val
                    .integer_value(scope)
                    .unwrap_or(default_length as i64) as i32;
            }
        }
    }
    default_length
}

fn aes_key_len_bytes_from_bits(length_bits: i32) -> Option<usize> {
    match length_bits {
        128 => Some(16),
        192 => Some(24),
        256 => Some(32),
        _ => None,
    }
}

fn aes_key_len_bits_from_bytes(length_bytes: usize) -> Option<i32> {
    match length_bytes {
        16 => Some(128),
        24 => Some(192),
        32 => Some(256),
        _ => None,
    }
}

fn get_required_aes_generate_key_length(
    scope: &mut v8::PinScope,
    algorithm_value: v8::Local<v8::Value>,
) -> Option<i32> {
    if !algorithm_value.is_object() {
        let error = v8::String::new(scope, "generateKey: AES key length is required").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return None;
    }

    let algo_obj = algorithm_value.to_object(scope).unwrap();
    let length_key = v8::String::new(scope, "length").unwrap();
    let Some(length_val) = algo_obj.get(scope, length_key.into()) else {
        let error = v8::String::new(scope, "generateKey: AES key length is required").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return None;
    };

    if !length_val.is_number() {
        let error = v8::String::new(scope, "generateKey: AES key length must be a number").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return None;
    }

    let length = length_val.integer_value(scope).unwrap_or(0) as i32;
    if aes_key_len_bytes_from_bits(length).is_none() {
        let error = v8::String::new(
            scope,
            "generateKey: AES key length must be 128, 192, or 256 bits",
        )
        .unwrap();
        let error_obj = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return None;
    }

    Some(length)
}

/// GenerateKey callback - generates cryptographic keys
fn generate_key_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if args.length() < 3 {
        let error = v8::String::new(
            scope,
            "generateKey requires 3 arguments: algorithm, extractable, keyUsages",
        )
        .unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let algorithm_value = args.get(0);
    let extractable_value = args.get(1);
    let usages_value = args.get(2);

    // Parse algorithm name
    let algorithm_name = match get_algorithm_name_option(scope, algorithm_value) {
        Some(name) => name,
        None => {
            throw_missing_algorithm_name(scope, "generateKey");
            return;
        }
    };

    // Parse extractable
    let extractable = get_bool_value(scope, extractable_value);

    // Parse usages
    let usages = get_key_usages(scope, usages_value);
    if !validate_key_usages(scope, "generateKey", &algorithm_name, &usages) {
        return;
    }

    // Generate key based on algorithm
    match algorithm_name.to_uppercase().as_str() {
        "HMAC" | "HS256" | "HS384" | "HS512" => {
            // Get hash algorithm
            let hash_name =
                match hmac_hash_name_for_algorithm(scope, algorithm_value, &algorithm_name) {
                    Ok(hash_name) => hash_name,
                    Err(error_message) => {
                        let error =
                            v8::String::new(scope, &format!("generateKey: {}", error_message))
                                .unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                };

            // Determine key length based on hash algorithm
            let key_length = match hash_name {
                "SHA-256" | "SHA-384" | "SHA-512" => 64, // Default to 512 bits
                _ => 64,
            };

            // Generate random key material
            let key_data = generate_random_bytes(key_length);

            // Create CryptoKey
            let crypto_key = create_crypto_key(
                scope,
                "secret",
                extractable,
                &algorithm_name,
                (key_length * 8) as i32,
                usages.iter().map(|s| s.as_str()).collect(),
            );
            set_crypto_key_hmac_hash(scope, crypto_key, hash_name);

            set_key_data(scope, crypto_key, &key_data);

            // Return promise resolving to the CryptoKey
            let resolver = v8::PromiseResolver::new(scope).unwrap();
            resolver.resolve(scope, crypto_key.into());
            let promise = resolver.get_promise(scope);
            retval.set(promise.into());
        }
        "AES-GCM" | "AES-CBC" | "AES-CTR" | "AES-KW" => {
            let Some(length) = get_required_aes_generate_key_length(scope, algorithm_value) else {
                return;
            };
            let key_bytes = aes_key_len_bytes_from_bits(length).expect("validated AES key length");

            // Generate random key material
            let key_data = generate_random_bytes(key_bytes);

            // Create CryptoKey
            let crypto_key = create_crypto_key(
                scope,
                "secret",
                extractable,
                &algorithm_name,
                length,
                usages.iter().map(|s| s.as_str()).collect(),
            );

            set_key_data(scope, crypto_key, &key_data);

            // Return promise resolving to the CryptoKey
            let resolver = v8::PromiseResolver::new(scope).unwrap();
            resolver.resolve(scope, crypto_key.into());
            let promise = resolver.get_promise(scope);
            retval.set(promise.into());
        }
        "RSA-OAEP" => {
            let Some(algo_obj) = algorithm_value.to_object(scope) else {
                let error =
                    v8::String::new(scope, "generateKey: RSA-OAEP requires parameters").unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            };

            let modulus_key = v8::String::new(scope, "modulusLength").unwrap();
            let modulus_length = match algo_obj
                .get(scope, modulus_key.into())
                .and_then(|value| value.is_number().then(|| value.integer_value(scope)))
            {
                Some(Some(value)) if value > 0 && value <= u32::MAX as i64 => value as u32,
                _ => {
                    let error = v8::String::new(
                        scope,
                        "generateKey: RSA-OAEP modulusLength must be a positive number",
                    )
                    .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            let exponent_key = v8::String::new(scope, "publicExponent").unwrap();
            let public_exponent = match algo_obj
                .get(scope, exponent_key.into())
                .and_then(|value| get_array_buffer_data(scope, value))
            {
                Some(exponent) => exponent,
                None => {
                    let error = v8::String::new(
                        scope,
                        "generateKey: RSA-OAEP publicExponent must be a Uint8Array",
                    )
                    .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            let hash_name_raw = get_algorithm_hash_name(scope, algorithm_value);
            let Some(hash_name) = normalize_hmac_hash_name(&hash_name_raw) else {
                let error = v8::String::new(
                    scope,
                    &format!(
                        "generateKey: unsupported RSA hash algorithm '{}'",
                        hash_name_raw
                    ),
                )
                .unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            };

            let (private_key_data, public_key_data) =
                match rsa_generate_key_pair(modulus_length, &public_exponent) {
                    Ok(pair) => pair,
                    Err(error_message) => {
                        let error =
                            v8::String::new(scope, &format!("generateKey: {}", error_message))
                                .unwrap();
                        let error_obj = v8::Exception::error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                };

            let public_usages: Vec<&str> = usages
                .iter()
                .filter_map(|usage| match usage.as_str() {
                    "encrypt" | "wrapKey" => Some(usage.as_str()),
                    _ => None,
                })
                .collect();
            let private_usages: Vec<&str> = usages
                .iter()
                .filter_map(|usage| match usage.as_str() {
                    "decrypt" | "unwrapKey" => Some(usage.as_str()),
                    _ => None,
                })
                .collect();

            let public_key = create_crypto_key(
                scope,
                "public",
                extractable,
                &algorithm_name,
                modulus_length as i32,
                public_usages,
            );
            let private_key = create_crypto_key(
                scope,
                "private",
                extractable,
                &algorithm_name,
                modulus_length as i32,
                private_usages,
            );
            set_crypto_key_hmac_hash(scope, public_key, hash_name);
            set_crypto_key_hmac_hash(scope, private_key, hash_name);
            set_key_data(scope, public_key, &public_key_data);
            set_key_data(scope, private_key, &private_key_data);

            let resolver = v8::PromiseResolver::new(scope).unwrap();
            let keypair_obj = v8::Object::new(scope);
            let public_key_key = v8::String::new(scope, "publicKey").unwrap();
            let private_key_key = v8::String::new(scope, "privateKey").unwrap();
            keypair_obj.set(scope, public_key_key.into(), public_key.into());
            keypair_obj.set(scope, private_key_key.into(), private_key.into());
            resolver.resolve(scope, keypair_obj.into());
            let promise = resolver.get_promise(scope);
            retval.set(promise.into());
        }
        "RSASSA-PKCS1-V1_5" => {
            let Some(algo_obj) = algorithm_value.to_object(scope) else {
                let error =
                    v8::String::new(scope, "generateKey: RSASSA-PKCS1-v1_5 requires parameters")
                        .unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            };

            let modulus_key = v8::String::new(scope, "modulusLength").unwrap();
            let modulus_length = match algo_obj
                .get(scope, modulus_key.into())
                .and_then(|value| value.is_number().then(|| value.integer_value(scope)))
            {
                Some(Some(value)) if value > 0 && value <= u32::MAX as i64 => value as u32,
                _ => {
                    let error = v8::String::new(
                        scope,
                        "generateKey: RSASSA-PKCS1-v1_5 modulusLength must be a positive number",
                    )
                    .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            let exponent_key = v8::String::new(scope, "publicExponent").unwrap();
            let public_exponent = match algo_obj
                .get(scope, exponent_key.into())
                .and_then(|value| get_array_buffer_data(scope, value))
            {
                Some(exponent) => exponent,
                None => {
                    let error = v8::String::new(
                        scope,
                        "generateKey: RSASSA-PKCS1-v1_5 publicExponent must be a Uint8Array",
                    )
                    .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            let hash_name_raw = get_algorithm_hash_name(scope, algorithm_value);
            let Some(hash_name) = normalize_hmac_hash_name(&hash_name_raw) else {
                let error = v8::String::new(
                    scope,
                    &format!(
                        "generateKey: unsupported RSA hash algorithm '{}'",
                        hash_name_raw
                    ),
                )
                .unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            };

            let (private_key_data, public_key_data) =
                match rsa_generate_key_pair(modulus_length, &public_exponent) {
                    Ok(pair) => pair,
                    Err(error_message) => {
                        let error =
                            v8::String::new(scope, &format!("generateKey: {}", error_message))
                                .unwrap();
                        let error_obj = v8::Exception::error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                };

            let private_usages: Vec<&str> = usages
                .iter()
                .filter_map(|usage| match usage.as_str() {
                    "sign" => Some("sign"),
                    _ => None,
                })
                .collect();
            let public_usages: Vec<&str> = usages
                .iter()
                .filter_map(|usage| match usage.as_str() {
                    "verify" => Some("verify"),
                    _ => None,
                })
                .collect();

            let private_key = create_crypto_key(
                scope,
                "private",
                extractable,
                &algorithm_name,
                modulus_length as i32,
                private_usages,
            );
            let public_key = create_crypto_key(
                scope,
                "public",
                extractable,
                &algorithm_name,
                modulus_length as i32,
                public_usages,
            );
            set_crypto_key_hmac_hash(scope, private_key, hash_name);
            set_crypto_key_hmac_hash(scope, public_key, hash_name);
            set_key_data(scope, private_key, &private_key_data);
            set_key_data(scope, public_key, &public_key_data);

            let resolver = v8::PromiseResolver::new(scope).unwrap();
            let keypair_obj = v8::Object::new(scope);
            let public_key_key = v8::String::new(scope, "publicKey").unwrap();
            let private_key_key = v8::String::new(scope, "privateKey").unwrap();
            keypair_obj.set(scope, public_key_key.into(), public_key.into());
            keypair_obj.set(scope, private_key_key.into(), private_key.into());
            resolver.resolve(scope, keypair_obj.into());
            let promise = resolver.get_promise(scope);
            retval.set(promise.into());
        }
        "ED25519" | "ED448" => {
            let (private_key_data, public_key_data, canonical_name) =
                match eddsa_generate_key_pair(&algorithm_name) {
                    Ok(pair) => pair,
                    Err(error_message) => {
                        let error = v8::String::new(scope, &error_message).unwrap();
                        let error_obj = v8::Exception::error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                };
            let key_size_bits = (private_key_data.len() * 8) as i32;
            let private_usages: Vec<&str> = usages
                .iter()
                .filter_map(|usage| match usage.as_str() {
                    "sign" => Some("sign"),
                    _ => None,
                })
                .collect();
            let public_usages: Vec<&str> = usages
                .iter()
                .filter_map(|usage| match usage.as_str() {
                    "verify" => Some("verify"),
                    _ => None,
                })
                .collect();

            let private_key = create_crypto_key(
                scope,
                "private",
                extractable,
                canonical_name,
                key_size_bits,
                private_usages,
            );
            let public_key = create_crypto_key(
                scope,
                "public",
                extractable,
                canonical_name,
                key_size_bits,
                public_usages,
            );

            set_key_data(scope, private_key, &private_key_data);
            set_key_data(scope, public_key, &public_key_data);

            let resolver = v8::PromiseResolver::new(scope).unwrap();
            let keypair_obj = v8::Object::new(scope);
            let public_key_key = v8::String::new(scope, "publicKey").unwrap();
            let private_key_key = v8::String::new(scope, "privateKey").unwrap();
            keypair_obj.set(scope, public_key_key.into(), public_key.into());
            keypair_obj.set(scope, private_key_key.into(), private_key.into());

            resolver.resolve(scope, keypair_obj.into());
            let promise = resolver.get_promise(scope);
            retval.set(promise.into());
        }
        "ECDSA" | "ECDH" => {
            let curve_name = if algorithm_value.is_object() {
                let algo_obj = algorithm_value.to_object(scope).unwrap();
                let curve_key = v8::String::new(scope, "namedCurve").unwrap();
                if let Some(curve_val) = algo_obj.get(scope, curve_key.into()) {
                    get_string_value(scope, curve_val).unwrap_or_else(|| "P-256".to_string())
                } else {
                    "P-256".to_string()
                }
            } else {
                "P-256".to_string()
            };

            if algorithm_name == "ECDH" {
                let (private_key_data, public_key_data) = match ecdh_generate_key_pair(&curve_name)
                {
                    Ok(pair) => pair,
                    Err(error_message) => {
                        let error = v8::String::new(scope, &error_message).unwrap();
                        let error_obj = v8::Exception::error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                };
                let private_key_size = (private_key_data.len() * 8) as i32;

                let private_key = create_crypto_key(
                    scope,
                    "private",
                    extractable,
                    &algorithm_name,
                    private_key_size,
                    usages.iter().map(|s| s.as_str()).collect(),
                );
                let public_key = create_crypto_key(
                    scope,
                    "public",
                    extractable,
                    &algorithm_name,
                    private_key_size,
                    Vec::new(),
                );

                let algo_key = v8::String::new(scope, "algorithm").unwrap();
                if let Some(pub_algo) = public_key.get(scope, algo_key.into()) {
                    if let Some(pub_algo_obj) = pub_algo.to_object(scope) {
                        let curve_key = v8::String::new(scope, "namedCurve").unwrap();
                        let curve_val = v8::String::new(scope, &curve_name).unwrap();
                        pub_algo_obj.set(scope, curve_key.into(), curve_val.into());
                    }
                }
                if let Some(priv_algo) = private_key.get(scope, algo_key.into()) {
                    if let Some(priv_algo_obj) = priv_algo.to_object(scope) {
                        let curve_key = v8::String::new(scope, "namedCurve").unwrap();
                        let curve_val = v8::String::new(scope, &curve_name).unwrap();
                        priv_algo_obj.set(scope, curve_key.into(), curve_val.into());
                    }
                }

                set_key_data(scope, private_key, &private_key_data);
                set_key_data(scope, public_key, &public_key_data);

                let curve_name_key = v8::String::new(scope, "__beejs_curve__").unwrap();
                let curve_name_val = v8::String::new(scope, &curve_name).unwrap();
                private_key.set(scope, curve_name_key.into(), curve_name_val.into());
                public_key.set(scope, curve_name_key.into(), curve_name_val.into());

                let resolver = v8::PromiseResolver::new(scope).unwrap();
                let keypair_obj = v8::Object::new(scope);
                let public_key_key = v8::String::new(scope, "publicKey").unwrap();
                let private_key_key = v8::String::new(scope, "privateKey").unwrap();
                keypair_obj.set(scope, public_key_key.into(), public_key.into());
                keypair_obj.set(scope, private_key_key.into(), private_key.into());

                resolver.resolve(scope, keypair_obj.into());
                let promise = resolver.get_promise(scope);
                retval.set(promise.into());
                return;
            }

            let (private_key_data, public_key_data) = match ecdsa_generate_key_pair(&curve_name) {
                Ok(pair) => pair,
                Err(error_message) => {
                    let error = v8::String::new(scope, &error_message).unwrap();
                    let error_obj = v8::Exception::error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };
            let private_key_size = (private_key_data.len() * 8) as i32;
            let private_usages: Vec<&str> = usages
                .iter()
                .filter_map(|usage| match usage.as_str() {
                    "sign" => Some("sign"),
                    _ => None,
                })
                .collect();
            let public_usages: Vec<&str> = usages
                .iter()
                .filter_map(|usage| match usage.as_str() {
                    "verify" => Some("verify"),
                    _ => None,
                })
                .collect();

            // Create private key CryptoKey
            let private_key = create_crypto_key(
                scope,
                "private",
                extractable,
                &algorithm_name,
                private_key_size,
                private_usages,
            );

            // Create public key CryptoKey
            let public_key = create_crypto_key(
                scope,
                "public",
                extractable,
                &algorithm_name,
                private_key_size,
                public_usages,
            );

            // Store curve information in algorithm object
            let algo_key = v8::String::new(scope, "algorithm").unwrap();
            if let Some(pub_algo) = public_key.get(scope, algo_key.into()) {
                if let Some(pub_algo_obj) = pub_algo.to_object(scope) {
                    let curve_key = v8::String::new(scope, "namedCurve").unwrap();
                    let curve_val = v8::String::new(scope, &curve_name).unwrap();
                    pub_algo_obj.set(scope, curve_key.into(), curve_val.into());
                }
            }
            if let Some(priv_algo) = private_key.get(scope, algo_key.into()) {
                if let Some(priv_algo_obj) = priv_algo.to_object(scope) {
                    let curve_key = v8::String::new(scope, "namedCurve").unwrap();
                    let curve_val = v8::String::new(scope, &curve_name).unwrap();
                    priv_algo_obj.set(scope, curve_key.into(), curve_val.into());
                }
            }

            set_key_data(scope, private_key, &private_key_data);
            set_key_data(scope, public_key, &public_key_data);

            // Store curve name for sign/verify
            let curve_name_key = v8::String::new(scope, "__beejs_curve__").unwrap();
            let curve_name_val = v8::String::new(scope, &curve_name).unwrap();
            private_key.set(scope, curve_name_key.into(), curve_name_val.into());
            public_key.set(scope, curve_name_key.into(), curve_name_val.into());

            // Return promise resolving to KeyPair object
            let resolver = v8::PromiseResolver::new(scope).unwrap();

            // Create KeyPair object with publicKey and privateKey
            let keypair_obj = v8::Object::new(scope);
            let public_key_key = v8::String::new(scope, "publicKey").unwrap();
            let private_key_key = v8::String::new(scope, "privateKey").unwrap();
            keypair_obj.set(scope, public_key_key.into(), public_key.into());
            keypair_obj.set(scope, private_key_key.into(), private_key.into());

            resolver.resolve(scope, keypair_obj.into());
            let promise = resolver.get_promise(scope);
            retval.set(promise.into());
        }
        _ => {
            let error_msg = format!("generateKey: unsupported algorithm '{}'", algorithm_name);
            let error = v8::String::new(scope, &error_msg).unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
        }
    }
}

/// Parse PBKDF2 algorithm parameters
fn parse_pbkdf2_params(
    scope: &mut v8::PinScope,
    algo_value: v8::Local<v8::Value>,
) -> Option<(Vec<u8>, String, u32)> {
    if !algo_value.is_object() {
        return None;
    }

    let algo_obj = algo_value.to_object(scope).unwrap();

    // Get salt
    let salt_key = v8::String::new(scope, "salt").unwrap();
    let salt = if let Some(salt_val) = algo_obj.get(scope, salt_key.into()) {
        get_array_buffer_data(scope, salt_val).unwrap_or_default()
    } else {
        vec![0u8; 16] // Default empty salt
    };

    // Get iterations
    let iterations_key = v8::String::new(scope, "iterations").unwrap();
    let iterations: u32 = if let Some(iter_val) = algo_obj.get(scope, iterations_key.into()) {
        iter_val.integer_value(scope).unwrap_or(100000) as u32
    } else {
        100000
    };

    // Get hash algorithm
    let hash_name = get_algorithm_hash_name(scope, algo_value);

    Some((salt, hash_name, iterations))
}

/// Derive bits using PBKDF2
fn derive_pbkdf2_bits(
    password: &[u8],
    salt: &[u8],
    iterations: u32,
    hash_name: &str,
    length_bits: usize,
) -> Result<Vec<u8>, String> {
    use ring::pbkdf2;
    use std::num::NonZeroU32;

    let output_len = (length_bits + 7) / 8;
    let mut output = vec![0u8; output_len];

    // Use ring's pbkdf2 derive (ring 0.17 API)
    // Note: iterations must be NonZeroU32, and we need to use the correct algorithm type
    let iterations_nz = NonZeroU32::new(iterations.max(1)).unwrap();
    let pbkdf2_algo = match hash_name {
        "SHA-256" => pbkdf2::PBKDF2_HMAC_SHA256,
        "SHA-384" => pbkdf2::PBKDF2_HMAC_SHA384,
        "SHA-512" => pbkdf2::PBKDF2_HMAC_SHA512,
        _ => return Err(format!("Unsupported hash for PBKDF2: {}", hash_name)),
    };
    pbkdf2::derive(pbkdf2_algo, iterations_nz, salt, password, &mut output);

    Ok(output)
}

/// deriveKey callback - derives a cryptographic key from a base key
fn derive_key_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if args.length() < 5 {
        let error = v8::String::new(scope, "deriveKey requires 5 arguments: algorithm, baseKey, derivedKeyAlgorithm, extractable, keyUsages").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let algorithm_value = args.get(0);
    let base_key_value = args.get(1);
    let derived_algorithm_value = args.get(2);
    let extractable_value = args.get(3);
    let usages_value = args.get(4);

    // Get base key data
    let key_data = if base_key_value.is_object() {
        let base_key_obj = base_key_value.to_object(scope).unwrap();
        get_key_data(scope, base_key_obj).unwrap_or_default()
    } else {
        vec![]
    };

    if key_data.is_empty() {
        let error = v8::String::new(scope, "deriveKey: baseKey must have key material").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    // Parse algorithm
    let algorithm_name = get_algorithm_name(scope, algorithm_value);

    match algorithm_name.to_uppercase().as_str() {
        "PBKDF2" => {
            let (salt, hash_name, iterations) = match parse_pbkdf2_params(scope, algorithm_value) {
                Some(params) => params,
                None => {
                    let error =
                        v8::String::new(scope, "deriveKey: invalid PBKDF2 parameters").unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            // Parse derived key algorithm to determine output length
            let derived_algo_name = get_algorithm_name(scope, derived_algorithm_value);
            let key_length = get_algorithm_length(scope, derived_algorithm_value, 256);

            // Calculate derived key length in bits
            let length_bits = key_length as usize;

            // Derive key material
            match derive_pbkdf2_bits(&key_data, &salt, iterations, &hash_name, length_bits) {
                Ok(derived_key_data) => {
                    // Parse extractable
                    let extractable = get_bool_value(scope, extractable_value);

                    // Parse usages
                    let usages = get_key_usages(scope, usages_value);

                    // Create CryptoKey
                    let crypto_key = create_crypto_key(
                        scope,
                        "secret",
                        extractable,
                        &derived_algo_name,
                        key_length as i32,
                        usages.iter().map(|s| s.as_str()).collect(),
                    );

                    set_key_data(scope, crypto_key, &derived_key_data);

                    // Create resolved promise
                    let resolver = v8::PromiseResolver::new(scope).unwrap();
                    resolver.resolve(scope, crypto_key.into());
                    let promise = resolver.get_promise(scope);
                    retval.set(promise.into());
                }
                Err(e) => {
                    let error = v8::String::new(scope, &e).unwrap();
                    let error_obj = v8::Exception::error(scope, error);
                    scope.throw_exception(error_obj.into());
                }
            }
        }
        "ECDH" => {
            // ECDH key derivation
            // Parse the algorithm to get the public key
            if !algorithm_value.is_object() {
                let error = v8::String::new(
                    scope,
                    "deriveKey: ECDH requires an algorithm object with 'public' key",
                )
                .unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            let algo_obj = algorithm_value.to_object(scope).unwrap();
            let public_key_key = v8::String::new(scope, "public").unwrap();
            let public_key_value = match algo_obj.get(scope, public_key_key.into()) {
                Some(pk) => pk,
                None => {
                    let error = v8::String::new(
                        scope,
                        "deriveKey: ECDH requires 'public' key in algorithm",
                    )
                    .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            if !public_key_value.is_object() {
                let error =
                    v8::String::new(scope, "deriveKey: ECDH 'public' must be a CryptoKey").unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            // Get the public key data
            let public_key_obj = public_key_value.to_object(scope).unwrap();
            let public_key_curve = get_curve_name(scope, public_key_obj);
            let public_key_data = match get_key_data(scope, public_key_obj) {
                Some(data) => data,
                None => {
                    let error = v8::String::new(
                        scope,
                        "deriveKey: ECDH public key material is unavailable",
                    )
                    .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            // Check if base key is an ECDH private key
            let base_key_obj = base_key_value.to_object(scope).unwrap();
            let base_key_algo = get_key_algorithm_name(scope, base_key_obj);
            let base_key_curve = get_curve_name(scope, base_key_obj);

            if base_key_algo != "ECDH" {
                let error =
                    v8::String::new(scope, "deriveKey: baseKey must be an ECDH private key")
                        .unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
            if base_key_curve != public_key_curve {
                let error =
                    v8::String::new(scope, "deriveKey: ECDH key curves must match").unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            // Get private key data from baseKey
            let private_key_data = match get_key_data(scope, base_key_obj) {
                Some(data) => data,
                None => {
                    let error = v8::String::new(
                        scope,
                        "deriveKey: ECDH private key material is unavailable",
                    )
                    .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            if private_key_data.is_empty() || public_key_data.is_empty() {
                let error =
                    v8::String::new(scope, "deriveKey: ECDH requires valid key material").unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            // Parse derived key algorithm to determine output length
            let derived_algo_name = get_algorithm_name(scope, derived_algorithm_value);
            let key_length = get_algorithm_length(scope, derived_algorithm_value, 256);

            let derived_key_data = match derive_ecdh_bits_result(
                &base_key_curve,
                &private_key_data,
                &public_key_data,
                key_length as usize,
            ) {
                Ok(bits) => bits,
                Err(error_message) => {
                    let error =
                        v8::String::new(scope, &format!("deriveKey: {}", error_message)).unwrap();
                    let error_obj = v8::Exception::error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            // Parse extractable
            let extractable = get_bool_value(scope, extractable_value);

            // Parse usages
            let usages = get_key_usages(scope, usages_value);

            // Create CryptoKey
            let crypto_key = create_crypto_key(
                scope,
                "secret",
                extractable,
                &derived_algo_name,
                key_length as i32,
                usages.iter().map(|s| s.as_str()).collect(),
            );

            set_key_data(scope, crypto_key, &derived_key_data);

            // Create resolved promise
            let resolver = v8::PromiseResolver::new(scope).unwrap();
            resolver.resolve(scope, crypto_key.into());
            let promise = resolver.get_promise(scope);
            retval.set(promise.into());
        }
        _ => {
            let error_msg = format!(
                "deriveKey: unsupported algorithm '{}'. Currently supported: 'PBKDF2', 'ECDH'",
                algorithm_name
            );
            let error = v8::String::new(scope, &error_msg).unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
        }
    }
}

/// deriveBits callback - derives bits from a base key
fn derive_bits_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if args.length() < 3 {
        let error = v8::String::new(
            scope,
            "deriveBits requires 3 arguments: algorithm, baseKey, length",
        )
        .unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let algorithm_value = args.get(0);
    let base_key_value = args.get(1);
    let length_value = args.get(2);

    // Get length in bits
    let length_bits = if length_value.is_number() {
        length_value.integer_value(scope).unwrap_or(256) as usize
    } else {
        256
    };

    // Get base key data
    let key_data = if base_key_value.is_object() {
        let base_key_obj = base_key_value.to_object(scope).unwrap();
        get_key_data(scope, base_key_obj).unwrap_or_default()
    } else {
        vec![]
    };

    if key_data.is_empty() {
        let error = v8::String::new(scope, "deriveBits: baseKey must have key material").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    // Parse algorithm
    let algorithm_name = get_algorithm_name(scope, algorithm_value);

    match algorithm_name.to_uppercase().as_str() {
        "PBKDF2" => {
            let (salt, hash_name, iterations) = match parse_pbkdf2_params(scope, algorithm_value) {
                Some(params) => params,
                None => {
                    let error =
                        v8::String::new(scope, "deriveBits: invalid PBKDF2 parameters").unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            match derive_pbkdf2_bits(&key_data, &salt, iterations, &hash_name, length_bits) {
                Ok(bits) => {
                    // Create ArrayBuffer with the derived bits
                    let array_buffer = v8::ArrayBuffer::new(scope, bits.len());
                    let backing_store = array_buffer.get_backing_store();
                    for (i, byte) in bits.iter().enumerate() {
                        backing_store[i].set(*byte);
                    }

                    // Create resolved promise
                    let resolver = v8::PromiseResolver::new(scope).unwrap();
                    resolver.resolve(scope, array_buffer.into());
                    let promise = resolver.get_promise(scope);
                    retval.set(promise.into());
                }
                Err(e) => {
                    let error = v8::String::new(scope, &e).unwrap();
                    let error_obj = v8::Exception::error(scope, error);
                    scope.throw_exception(error_obj.into());
                }
            }
        }
        "ECDH" => {
            // ECDH bits derivation
            // Parse the algorithm to get the public key
            if !algorithm_value.is_object() {
                let error = v8::String::new(
                    scope,
                    "deriveBits: ECDH requires an algorithm object with 'public' key",
                )
                .unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            let algo_obj = algorithm_value.to_object(scope).unwrap();
            let public_key_key = v8::String::new(scope, "public").unwrap();
            let public_key_value = match algo_obj.get(scope, public_key_key.into()) {
                Some(pk) => pk,
                None => {
                    let error = v8::String::new(
                        scope,
                        "deriveBits: ECDH requires 'public' key in algorithm",
                    )
                    .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            if !public_key_value.is_object() {
                let error = v8::String::new(scope, "deriveBits: ECDH 'public' must be a CryptoKey")
                    .unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            // Get the public key data
            let public_key_obj = public_key_value.to_object(scope).unwrap();
            let public_key_curve = get_curve_name(scope, public_key_obj);
            let public_key_data = match get_key_data(scope, public_key_obj) {
                Some(data) => data,
                None => {
                    let error = v8::String::new(
                        scope,
                        "deriveBits: ECDH public key material is unavailable",
                    )
                    .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            // Check if base key is an ECDH private key
            let base_key_obj = base_key_value.to_object(scope).unwrap();
            let base_key_algo = get_key_algorithm_name(scope, base_key_obj);
            let base_key_curve = get_curve_name(scope, base_key_obj);

            if base_key_algo != "ECDH" {
                let error =
                    v8::String::new(scope, "deriveBits: baseKey must be an ECDH private key")
                        .unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
            if base_key_curve != public_key_curve {
                let error =
                    v8::String::new(scope, "deriveBits: ECDH key curves must match").unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            // Get private key data from baseKey
            let private_key_data = match get_key_data(scope, base_key_obj) {
                Some(data) => data,
                None => {
                    let error = v8::String::new(
                        scope,
                        "deriveBits: ECDH private key material is unavailable",
                    )
                    .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            if private_key_data.is_empty() || public_key_data.is_empty() {
                let error =
                    v8::String::new(scope, "deriveBits: ECDH requires valid key material").unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            let bits = match derive_ecdh_bits_result(
                &base_key_curve,
                &private_key_data,
                &public_key_data,
                length_bits,
            ) {
                Ok(bits) => bits,
                Err(error_message) => {
                    let error =
                        v8::String::new(scope, &format!("deriveBits: {}", error_message)).unwrap();
                    let error_obj = v8::Exception::error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            };

            // Create ArrayBuffer with the derived bits
            let array_buffer = v8::ArrayBuffer::new(scope, bits.len());
            let backing_store = array_buffer.get_backing_store();
            for (i, byte) in bits.iter().enumerate() {
                backing_store[i].set(*byte);
            }

            // Create resolved promise
            let resolver = v8::PromiseResolver::new(scope).unwrap();
            resolver.resolve(scope, array_buffer.into());
            let promise = resolver.get_promise(scope);
            retval.set(promise.into());
        }
        _ => {
            let error_msg = format!(
                "deriveBits: unsupported algorithm '{}'. Currently supported: 'PBKDF2', 'ECDH'",
                algorithm_name
            );
            let error = v8::String::new(scope, &error_msg).unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
        }
    }
}

/// Get algorithm name from CryptoKey
fn get_key_algorithm_name(scope: &mut v8::PinScope, crypto_key: v8::Local<v8::Object>) -> String {
    let algorithm_key = v8::String::new(scope, "algorithm").unwrap();
    if let Some(algo_val) = crypto_key.get(scope, algorithm_key.into()) {
        if algo_val.is_object() {
            let algo_obj = algo_val.to_object(scope).unwrap();
            let name_key = v8::String::new(scope, "name").unwrap();
            if let Some(name_val) = algo_obj.get(scope, name_key.into()) {
                if name_val.is_string() {
                    return name_val
                        .to_string(scope)
                        .unwrap()
                        .to_rust_string_lossy(scope);
                }
            }
        }
    }
    String::new()
}

/// Check if key is extractable
fn is_key_extractable(scope: &mut v8::PinScope, crypto_key: v8::Local<v8::Object>) -> bool {
    let extractable_key = v8::String::new(scope, "extractable").unwrap();
    if let Some(extractable_val) = crypto_key.get(scope, extractable_key.into()) {
        return extractable_val.boolean_value(scope);
    }
    false
}

/// Base64URL encode (WebCrypto JWK format)
fn base64url_encode(data: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    let mut pos = 0;
    let len = data.len();

    while pos + 3 <= len {
        let b0 = data[pos] as u32;
        let b1 = data[pos + 1] as u32;
        let b2 = data[pos + 2] as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARSET[(n >> 18 & 0x3F) as usize] as char);
        result.push(CHARSET[(n >> 12 & 0x3F) as usize] as char);
        result.push(CHARSET[(n >> 6 & 0x3F) as usize] as char);
        result.push(CHARSET[(n & 0x3F) as usize] as char);

        pos += 3;
    }

    // Handle remaining bytes
    match len - pos {
        2 => {
            let b0 = data[pos] as u32;
            let b1 = data[pos + 1] as u32;
            let n = (b0 << 16) | (b1 << 8);
            result.push(CHARSET[(n >> 18 & 0x3F) as usize] as char);
            result.push(CHARSET[(n >> 12 & 0x3F) as usize] as char);
            result.push(CHARSET[(n >> 6 & 0x3F) as usize] as char);
        }
        1 => {
            let b0 = data[pos] as u32;
            let n = b0 << 16;
            result.push(CHARSET[(n >> 18 & 0x3F) as usize] as char);
            result.push(CHARSET[(n >> 12 & 0x3F) as usize] as char);
        }
        _ => {}
    }

    result
}

fn crypto_key_usages(scope: &mut v8::PinScope, key_obj: v8::Local<v8::Object>) -> Vec<String> {
    let usages_key = v8::String::new(scope, "usages").unwrap();
    key_obj
        .get(scope, usages_key.into())
        .map(|value| get_key_usages(scope, value))
        .unwrap_or_default()
}

fn export_key_payload_for_wrap(
    scope: &mut v8::PinScope,
    format_str: &str,
    key_obj: v8::Local<v8::Object>,
    key_data: &[u8],
) -> Result<Vec<u8>, String> {
    if !is_key_extractable(scope, key_obj) {
        return Err("key is not extractable".to_string());
    }

    let algo_name = get_key_algorithm_name(scope, key_obj);
    let key_type = get_key_type(scope, key_obj);

    match format_str {
        "raw" => {
            if is_eddsa_algorithm_name(&algo_name) && key_type != "public" {
                return Err(format!(
                    "Unable to export {} private key using raw format",
                    algo_name
                ));
            }

            Ok(key_data.to_vec())
        }
        "jwk" => {
            if let Some((_id, canonical_name, _key_size)) = eddsa_key_id(&algo_name) {
                let public_key_data = if key_type == "private" {
                    eddsa_public_key_from_private(canonical_name, key_data)?
                } else {
                    key_data.to_vec()
                };
                let usages = crypto_key_usages(scope, key_obj);
                let mut jwk = serde_json::Map::new();
                jwk.insert(
                    "key_ops".to_string(),
                    serde_json::Value::Array(
                        usages.into_iter().map(serde_json::Value::String).collect(),
                    ),
                );
                jwk.insert("ext".to_string(), serde_json::Value::Bool(true));
                jwk.insert(
                    "alg".to_string(),
                    serde_json::Value::String(canonical_name.to_string()),
                );
                jwk.insert(
                    "crv".to_string(),
                    serde_json::Value::String(canonical_name.to_string()),
                );
                if key_type == "private" {
                    jwk.insert(
                        "d".to_string(),
                        serde_json::Value::String(base64url_encode(key_data)),
                    );
                }
                jwk.insert(
                    "x".to_string(),
                    serde_json::Value::String(base64url_encode(&public_key_data)),
                );
                jwk.insert(
                    "kty".to_string(),
                    serde_json::Value::String("OKP".to_string()),
                );
                return serde_json::to_vec(&serde_json::Value::Object(jwk))
                    .map_err(|error| error.to_string());
            }

            let alg = match algo_name.as_str() {
                "HMAC" | "HS256" | "HS384" | "HS512" => {
                    let hash_name = get_key_hmac_hash_name(scope, key_obj);
                    hmac_jwk_alg_from_hash(&hash_name)
                        .unwrap_or("HS256")
                        .to_string()
                }
                "AES-GCM" => format!("A{}GCM", key_data.len() * 8),
                "AES-CBC" => format!("A{}CBC", key_data.len() * 8),
                "AES-CTR" => format!("A{}CTR", key_data.len() * 8),
                "AES-KW" => format!("A{}KW", key_data.len() * 8),
                _ => {
                    return Err(format!(
                        "unsupported key algorithm '{}' for JWK wrap",
                        algo_name
                    ));
                }
            };

            let usages = crypto_key_usages(scope, key_obj);
            let mut jwk = serde_json::Map::new();
            jwk.insert(
                "kty".to_string(),
                serde_json::Value::String("oct".to_string()),
            );
            jwk.insert("alg".to_string(), serde_json::Value::String(alg));
            jwk.insert(
                "key_ops".to_string(),
                serde_json::Value::Array(
                    usages.into_iter().map(serde_json::Value::String).collect(),
                ),
            );
            jwk.insert("ext".to_string(), serde_json::Value::Bool(true));
            jwk.insert(
                "k".to_string(),
                serde_json::Value::String(base64url_encode(key_data)),
            );

            serde_json::to_vec(&serde_json::Value::Object(jwk)).map_err(|error| error.to_string())
        }
        _ => Err(format!("unsupported format '{}'", format_str)),
    }
}

/// ExportKey callback - exports cryptographic keys
fn export_key_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if args.length() < 2 {
        let error = v8::String::new(scope, "exportKey requires 2 arguments: format, key").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let format_value = args.get(0);
    let key_value = args.get(1);

    if !format_value.is_string() {
        let error = v8::String::new(scope, "exportKey: format must be a string").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    if !key_value.is_object() {
        let error = v8::String::new(scope, "exportKey: key must be a CryptoKey object").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    // Get format string
    let format_str = format_value
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    let key_obj = key_value.to_object(scope).unwrap();

    // Get key data
    let key_data = match get_key_data(scope, key_obj) {
        Some(data) => data,
        None => {
            let error = v8::String::new(scope, "exportKey: could not extract key data").unwrap();
            let error_obj = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
    };

    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    retval.set(promise.into());

    // Check if key is extractable. This is an asynchronous Web Crypto
    // operation, so valid CryptoKey errors are surfaced as promise rejections.
    if !is_key_extractable(scope, key_obj) {
        let error = v8::String::new(scope, "exportKey: key is not extractable").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        resolver.reject(scope, error_obj.into());
        return;
    }

    // Get algorithm name
    let algo_name = get_key_algorithm_name(scope, key_obj);
    let key_type = get_key_type(scope, key_obj);

    // Export based on format
    match format_str.as_str() {
        "raw" => {
            if is_eddsa_algorithm_name(&algo_name) && key_type != "public" {
                let error = v8::String::new(
                    scope,
                    &format!(
                        "exportKey: Unable to export {} private key using raw format",
                        algo_name
                    ),
                )
                .unwrap();
                let error_obj = v8::Exception::error(scope, error);
                resolver.reject(scope, error_obj.into());
                return;
            }

            // Return raw key bytes as ArrayBuffer
            let arr_buf = v8::ArrayBuffer::new(scope, key_data.len());
            let backing_store = arr_buf.get_backing_store();
            for (i, &byte) in key_data.iter().enumerate() {
                backing_store[i].set(byte);
            }

            resolver.resolve(scope, arr_buf.into());
        }
        "jwk" => {
            if let Some((_id, canonical_name, _key_size)) = eddsa_key_id(&algo_name) {
                let public_key_data = if key_type == "private" {
                    match eddsa_public_key_from_private(canonical_name, &key_data) {
                        Ok(public_key_data) => public_key_data,
                        Err(error_message) => {
                            let error =
                                v8::String::new(scope, &format!("exportKey: {}", error_message))
                                    .unwrap();
                            let error_obj = v8::Exception::error(scope, error);
                            resolver.reject(scope, error_obj.into());
                            return;
                        }
                    }
                } else {
                    key_data.clone()
                };

                let jwk_obj = v8::Object::new(scope);

                let key_ops_key = v8::String::new(scope, "key_ops").unwrap();
                let usages_key = v8::String::new(scope, "usages").unwrap();
                if let Some(usages_val) = key_obj.get(scope, usages_key.into()) {
                    if usages_val.is_array() {
                        jwk_obj.set(scope, key_ops_key.into(), usages_val);
                    }
                }

                let ext_key = v8::String::new(scope, "ext").unwrap();
                let ext_val = v8::Boolean::new(scope, true);
                jwk_obj.set(scope, ext_key.into(), ext_val.into());

                let alg_key = v8::String::new(scope, "alg").unwrap();
                let alg_val = v8::String::new(scope, canonical_name).unwrap();
                jwk_obj.set(scope, alg_key.into(), alg_val.into());

                let crv_key = v8::String::new(scope, "crv").unwrap();
                let crv_val = v8::String::new(scope, canonical_name).unwrap();
                jwk_obj.set(scope, crv_key.into(), crv_val.into());

                if key_type == "private" {
                    let d_key = v8::String::new(scope, "d").unwrap();
                    let d_val = v8::String::new(scope, &base64url_encode(&key_data)).unwrap();
                    jwk_obj.set(scope, d_key.into(), d_val.into());
                }

                let x_key = v8::String::new(scope, "x").unwrap();
                let x_val = v8::String::new(scope, &base64url_encode(&public_key_data)).unwrap();
                jwk_obj.set(scope, x_key.into(), x_val.into());

                let kty_key = v8::String::new(scope, "kty").unwrap();
                let kty_val = v8::String::new(scope, "OKP").unwrap();
                jwk_obj.set(scope, kty_key.into(), kty_val.into());

                resolver.resolve(scope, jwk_obj.into());
                return;
            }

            // Create JWK object
            let jwk_obj = v8::Object::new(scope);

            // Set common JWK fields
            let kty_key = v8::String::new(scope, "kty").unwrap();
            let kty_val = v8::String::new(scope, "oct").unwrap();
            jwk_obj.set(scope, kty_key.into(), kty_val.into());

            // Set alg based on algorithm
            let alg_key = v8::String::new(scope, "alg").unwrap();
            let alg_val = match algo_name.as_str() {
                "HMAC" | "HS256" | "HS384" | "HS512" => {
                    let hash_name = get_key_hmac_hash_name(scope, key_obj);
                    let alg = hmac_jwk_alg_from_hash(&hash_name).unwrap_or("HS256");
                    v8::String::new(scope, alg).unwrap()
                }
                "AES-GCM" => {
                    let length = key_data.len() * 8;
                    v8::String::new(scope, &format!("A{}GCM", length)).unwrap()
                }
                "AES-CBC" => {
                    let length = key_data.len() * 8;
                    v8::String::new(scope, &format!("A{}CBC", length)).unwrap()
                }
                "AES-CTR" => {
                    let length = key_data.len() * 8;
                    v8::String::new(scope, &format!("A{}CTR", length)).unwrap()
                }
                "AES-KW" => {
                    let length = key_data.len() * 8;
                    v8::String::new(scope, &format!("A{}KW", length)).unwrap()
                }
                _ => v8::String::new(scope, "A256").unwrap(),
            };
            jwk_obj.set(scope, alg_key.into(), alg_val.into());

            // Set key operations
            let key_ops_key = v8::String::new(scope, "key_ops").unwrap();
            let usages_key = v8::String::new(scope, "usages").unwrap();
            if let Some(usages_val) = key_obj.get(scope, usages_key.into()) {
                if usages_val.is_array() {
                    jwk_obj.set(scope, key_ops_key.into(), usages_val);
                }
            }

            // Set extractable
            let ext_key = v8::String::new(scope, "ext").unwrap();
            let ext_val = v8::Boolean::new(scope, true);
            jwk_obj.set(scope, ext_key.into(), ext_val.into());

            // Set k (base64url encoded key data)
            let k_key = v8::String::new(scope, "k").unwrap();
            let k_val = v8::String::new(scope, &base64url_encode(&key_data)).unwrap();
            jwk_obj.set(scope, k_key.into(), k_val.into());

            resolver.resolve(scope, jwk_obj.into());
        }
        "spki" => {
            let Some((_id, canonical_name, _key_size)) = eddsa_key_id(&algo_name) else {
                let error_msg = format!("exportKey: unsupported format '{}'", format_str);
                let error = v8::String::new(scope, &error_msg).unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                resolver.reject(scope, error_obj.into());
                return;
            };
            if key_type != "public" {
                let error = v8::String::new(
                    scope,
                    &format!(
                        "exportKey: Unable to export {} private key using spki format",
                        canonical_name
                    ),
                )
                .unwrap();
                let error_obj = v8::Exception::error(scope, error);
                resolver.reject(scope, error_obj.into());
                return;
            }
            let spki_der = match eddsa_public_der_from_raw(canonical_name, &key_data) {
                Ok(spki_der) => spki_der,
                Err(error_message) => {
                    let error =
                        v8::String::new(scope, &format!("exportKey: {}", error_message)).unwrap();
                    let error_obj = v8::Exception::error(scope, error);
                    resolver.reject(scope, error_obj.into());
                    return;
                }
            };
            let arr_buf = v8::ArrayBuffer::new(scope, spki_der.len());
            let backing_store = arr_buf.get_backing_store();
            for (i, &byte) in spki_der.iter().enumerate() {
                backing_store[i].set(byte);
            }

            resolver.resolve(scope, arr_buf.into());
        }
        "pkcs8" => {
            let Some((_id, canonical_name, _key_size)) = eddsa_key_id(&algo_name) else {
                let error_msg = format!("exportKey: unsupported format '{}'", format_str);
                let error = v8::String::new(scope, &error_msg).unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                resolver.reject(scope, error_obj.into());
                return;
            };
            if key_type != "private" {
                let error = v8::String::new(
                    scope,
                    &format!(
                        "exportKey: Unable to export {} public key using pkcs8 format",
                        canonical_name
                    ),
                )
                .unwrap();
                let error_obj = v8::Exception::error(scope, error);
                resolver.reject(scope, error_obj.into());
                return;
            }
            let pkcs8_der = match eddsa_private_der_from_raw(canonical_name, &key_data) {
                Ok(pkcs8_der) => pkcs8_der,
                Err(error_message) => {
                    let error =
                        v8::String::new(scope, &format!("exportKey: {}", error_message)).unwrap();
                    let error_obj = v8::Exception::error(scope, error);
                    resolver.reject(scope, error_obj.into());
                    return;
                }
            };
            let arr_buf = v8::ArrayBuffer::new(scope, pkcs8_der.len());
            let backing_store = arr_buf.get_backing_store();
            for (i, &byte) in pkcs8_der.iter().enumerate() {
                backing_store[i].set(byte);
            }

            resolver.resolve(scope, arr_buf.into());
        }
        _ => {
            let error_msg = format!("exportKey: unsupported format '{}'", format_str);
            let error = v8::String::new(scope, &error_msg).unwrap();
            let error_obj = v8::Exception::type_error(scope, error);
            resolver.reject(scope, error_obj.into());
        }
    }
}

/// Setup crypto.randomUUID (for convenience)
fn setup_crypto_random_uuid_api(scope: &mut v8::PinScope, crypto_obj: &v8::Object) {
    let uuid_key = v8::String::new(scope, "randomUUID").unwrap();
    let uuid_fn = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            let uuid = uuid::Uuid::new_v4();
            let uuid_str = v8::String::new(_scope, &uuid.to_string()).unwrap();
            rv.set(uuid_str.into());
        },
    );
    let uuid_fn_instance = uuid_fn.get_function(scope).unwrap();
    crypto_obj.set(scope, uuid_key.into(), uuid_fn_instance.into());
}

pub fn setup_crypto_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);
    let crypto_key: v8::Local<v8::String> = v8::String::new(scope, "crypto").unwrap();

    let crypto_obj: v8::Local<v8::Object> = global
        .get(scope, crypto_key.into())
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
        .unwrap_or_else(|| v8::Object::new(scope));

    // Create subtle object
    let subtle_key = v8::String::new(scope, "subtle").unwrap();
    let subtle_obj: v8::Local<v8::Object> = v8::Object::new(scope);

    // Setup getRandomValues on crypto (not subtle)
    let get_random_key: v8::Local<v8::String> = v8::String::new(scope, "getRandomValues").unwrap();
    let get_random_func: v8::Local<v8::FunctionTemplate> = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, rv: v8::ReturnValue| {
            get_random_values_callback(scope, args, rv);
        },
    );
    let get_random_func_instance: v8::Local<v8::Function> =
        get_random_func.get_function(scope).unwrap();
    crypto_obj.set(
        scope,
        get_random_key.into(),
        get_random_func_instance.into(),
    );

    // Setup crypto.subtle API
    setup_crypto_subtle_api(scope, &subtle_obj);

    // Setup crypto.randomUUID
    setup_crypto_random_uuid_api(scope, &crypto_obj);

    // Set subtle on crypto
    crypto_obj.set(scope, subtle_key.into(), subtle_obj.into());

    // Set crypto on global
    global.set(scope, crypto_key.into(), crypto_obj.into());

    // Also set webkitGetUserEntries (Safari compatibility)
    let webkit_key: v8::Local<v8::String> = v8::String::new(scope, "webkitCrypto").unwrap();
    global.set(scope, webkit_key.into(), crypto_obj.into());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_digest() {
        let data = b"hello world";
        let result = compute_sha_digest(data, "SHA-256");
        assert!(result.is_ok());
        let hash = result.unwrap();
        assert_eq!(hash.len(), 32); // SHA-256 produces 32 bytes
                                    // b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
        assert_eq!(hex::encode(&hash[..8]), "b94d27b9934d3e08");
    }

    #[test]
    fn test_sha384_digest() {
        let data = b"hello world";
        let result = compute_sha_digest(data, "SHA-384");
        assert!(result.is_ok());
        let hash = result.unwrap();
        assert_eq!(hash.len(), 48); // SHA-384 produces 48 bytes
    }

    #[test]
    fn test_sha512_digest() {
        let data = b"hello world";
        let result = compute_sha_digest(data, "SHA-512");
        assert!(result.is_ok());
        let hash = result.unwrap();
        assert_eq!(hash.len(), 64); // SHA-512 produces 64 bytes
    }

    #[test]
    fn test_unsupported_algorithm() {
        let data = b"hello world";
        let result = compute_sha_digest(data, "MD5");
        assert!(result.is_err());
    }
}

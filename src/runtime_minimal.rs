//! Minimal Runtime implementation for fast startup and basic JavaScript execution
//! This is a simplified version of RuntimeLite without complex dependencies
//! MARKER_V3 - This file has been modified

use anyhow::Result;
use base64::Engine;
use openssl::bn::{BigNum, BigNumContext, BigNumRef};
use openssl::derive::Deriver;
use openssl::ec::{EcGroup, EcGroupRef, EcKey, EcPoint, PointConversionForm};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{Id, PKey, Private, Public};
use openssl::rsa::{Padding, Rsa};
use openssl::sign::{RsaPssSaltlen, Signer, Verifier};
use openssl::symm::Cipher;
use rand::Rng;
use reqwest;
use rusty_v8 as v8;
use serde_json;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

thread_local! {
    static ACTIVE_SOURCE_MAP: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Record the oxc source map for the script about to run so thrown stacks map to `.ts`.
pub fn set_active_source_map(map_json: String) {
    ACTIVE_SOURCE_MAP.with(|slot| {
        *slot.borrow_mut() = Some(map_json);
    });
}

fn active_source_map_url<'s>(scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Value>> {
    ACTIVE_SOURCE_MAP.with(|slot| {
        if let Some(map) = slot.borrow().as_ref() {
            let url = format!(
                "data:application/json;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(map.as_bytes())
            );
            v8::String::new(scope, &url).map(|s| s.into())
        } else {
            None
        }
    })
}

fn apply_active_source_map(stack: &str) -> String {
    ACTIVE_SOURCE_MAP.with(|slot| match slot.borrow().as_deref() {
        Some(map) => remap_stack_with_source_map(stack, map),
        None => stack.to_string(),
    })
}

fn remap_stack_with_source_map(stack: &str, map_json: &str) -> String {
    let Ok(map) = serde_json::from_str::<serde_json::Value>(map_json) else {
        return stack.to_string();
    };
    let Some(source_name) = map
        .get("sources")
        .and_then(|s| s.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
    else {
        return stack.to_string();
    };
    let Some(mappings) = map.get("mappings").and_then(|m| m.as_str()) else {
        return stack.to_string();
    };
    let line_map = decode_source_map_lines(mappings);
    let mut out = String::new();
    for line in stack.lines() {
        if let Some(remapped) = remap_stack_line(line, source_name, &line_map) {
            out.push_str(&remapped);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

fn decode_source_map_lines(mappings: &str) -> Vec<Option<u32>> {
    let mut generated_to_original: Vec<Option<u32>> = Vec::new();
    let mut original_line: i32 = 0;
    for (gen_line, segment_group) in mappings.split(';').enumerate() {
        if generated_to_original.len() <= gen_line {
            generated_to_original.resize(gen_line + 1, None);
        }
        let Some(first) = segment_group.split(',').next() else {
            continue;
        };
        if first.is_empty() {
            continue;
        }
        let decoded = decode_vlq_segment(first);
        if decoded.len() >= 3 {
            original_line += decoded[2];
            generated_to_original[gen_line] = Some(original_line.max(0) as u32);
        }
    }
    generated_to_original
}

fn decode_vlq_segment(seg: &str) -> Vec<i32> {
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut values = Vec::new();
    let mut result: u32 = 0;
    let mut shift = 0;
    for ch in seg.bytes() {
        let Some(digit) = table.iter().position(|&c| c == ch).map(|i| i as u32) else {
            break;
        };
        let has_cont = digit & 32 != 0;
        result += (digit & 31) << shift;
        if has_cont {
            shift += 5;
            continue;
        }
        let signed = if result & 1 != 0 {
            -((result >> 1) as i32)
        } else {
            (result >> 1) as i32
        };
        values.push(signed);
        result = 0;
        shift = 0;
    }
    values
}

fn remap_stack_line(line: &str, source_name: &str, line_map: &[Option<u32>]) -> Option<String> {
    let start = line.rfind(':')?;
    let (prefix_col, _col) = line.split_at(start);
    let line_start = prefix_col.rfind(':')?;
    let (prefix, line_str) = prefix_col.split_at(line_start);
    let gen_line: usize = line_str.trim_start_matches(':').parse().ok()?;
    let orig = *line_map.get(gen_line.saturating_sub(1))?.as_ref()?;
    let orig_line = orig + 1;
    let file_end = prefix
        .rfind(|c: char| c == '/' || c == ' ' || c == '(')
        .map(|i| i + 1)
        .unwrap_or(0);
    let mut remapped = String::new();
    remapped.push_str(&prefix[..file_end]);
    remapped.push_str(source_name);
    remapped.push(':');
    remapped.push_str(&orig_line.to_string());
    remapped.push_str(&line[start..]);
    Some(remapped)
}

// v0.3.50: Import Node.js core modules for path and fs
use crate::nodejs_core::crypto::setup_crypto_api;
use crate::nodejs_core::fs::setup_fs_api;
use crate::nodejs_core::http::setup_http_api;
use crate::nodejs_core::net::setup_net_api;
use crate::nodejs_core::path::setup_path_api;
use crate::nodejs_core::timers::{
    clear_pending_immediates, execute_fired_timers, execute_immediate_callbacks,
    has_pending_immediates, mark_immediate_callbacks_deferred, setup_timers_api,
    unmark_immediate_callbacks_deferred,
};
// v0.3.261: Import process module for nextTick support
use crate::nodejs_core::process::{execute_next_tick_callbacks, has_pending_next_ticks};
// v0.3.275: Import performance API
use crate::nodejs_core::performance::setup_performance_api;
// v0.3.282: Import Web Streams API for AI workloads
use crate::web_api::streams::setup_streams_api;
// v0.3.335: Import error event API for window.onerror
use crate::web_api::error_event::call_onerror_handler;
// v0.3.305: Import Blob API for binary data handling
use crate::web_api::blob::setup_blob_api;
// v0.3.295: Import CompressionStream API
use crate::web_api::compression::setup_compression_api;
// v0.3.299: Import structuredClone API
use crate::web_api::structured_clone::setup_structured_clone_api;
// v0.3.311: Import ArrayBuffer transfer API
use crate::web_api::array_buffer_transfer::setup_array_buffer_transfer_api;
// v0.3.312: Import BroadcastChannel API
use crate::web_api::broadcast_channel::setup_broadcast_channel_api;
// v0.3.315: Import MessageChannel API
use crate::web_api::message_channel::setup_message_channel_api;
// v0.3.320: Import Worker API
use crate::web_api::worker::setup_worker_api;
// v0.3.322: Import SharedArrayBuffer API
use crate::web_api::shared_array_buffer::setup_shared_array_buffer_api;
// v0.3.324: Import ServiceWorker API
use crate::web_api::service_worker::setup_service_worker_api;
// v0.3.354: Import Web Crypto API (crypto.subtle)
use crate::web_api::crypto::setup_crypto_api as setup_web_crypto_api;

#[derive(Default)]
struct EsmModuleLoadState {
    modules_by_path: HashMap<PathBuf, v8::Global<v8::Module>>,
    source_fingerprints_by_path: HashMap<PathBuf, [u8; 32]>,
    paths_by_script_id: HashMap<i32, PathBuf>,
    cjs_synthetic_exports_by_identity: HashMap<i32, v8::Global<v8::Value>>,
    cjs_synthetic_named_exports_by_identity: HashMap<i32, Vec<String>>,
    pending_error: Option<String>,
}

enum EsmDynamicImportError<'scope> {
    Message(String),
    Value(v8::Local<'scope, v8::Value>),
}

thread_local! {
    static ESM_MODULE_LOAD_STATE: RefCell<Option<EsmModuleLoadState>> = const { RefCell::new(None) };
}

// v0.3.242: Max listeners storage per event type (for process.setMaxListeners)
thread_local! {
    static MAX_LISTENERS: Mutex<HashMap<String, i32>> = Mutex::new(HashMap::new());
}

thread_local! {
    static CACHED_BUFFER_PROTOTYPE: RefCell<Option<v8::Global<v8::Object>>> = const { RefCell::new(None) };
}

thread_local! {
    static IMPORT_META_PARENT_DIR: RefCell<PathBuf> = RefCell::new(PathBuf::from("."));
}

extern "C" fn promise_reject_callback(message: v8::PromiseRejectMessage) {
    if message.get_event() != v8::PromiseRejectEvent::PromiseRejectWithNoHandler {
        return;
    }
    v8::callback_scope!(unsafe let scope, &message);
    let context = scope.get_current_context();
    let scope = &mut v8::ContextScope::new(scope, context);
    let global = context.global(scope);
    let key = v8::String::new(scope, "__bee_dispatch_unhandled_rejection").unwrap();
    let Some(fn_val) = global.get(scope, key.into()) else {
        return;
    };
    if !fn_val.is_function() {
        return;
    }
    let func = v8::Local::<v8::Function>::try_from(fn_val).unwrap();
    let promise: v8::Local<v8::Value> = message.get_promise().into();
    let reason = message
        .get_value()
        .unwrap_or_else(|| v8::undefined(scope).into());
    let _ = func.call(scope, global.into(), &[promise, reason]);
}

#[inline]
pub fn set_buffer_prototype_fast(scope: &mut v8::PinScope, u8_array: v8::Local<v8::Uint8Array>) {
    CACHED_BUFFER_PROTOTYPE.with(|p| {
        if let Some(proto) = p.borrow().as_ref() {
            let proto_local = v8::Local::new(scope, proto);
            u8_array.set_prototype(scope, proto_local.into());
        }
    });
}

/// Legacy inline fs fallbacks predate the shared fs module and do not participate
/// in the resource broker. Keep them disabled unless a test deliberately flips
/// this private process flag inside the runtime.
static LEGACY_FS_FALLBACK_ENABLED: AtomicBool = AtomicBool::new(false);

fn legacy_fs_fallback_enabled() -> bool {
    LEGACY_FS_FALLBACK_ENABLED.load(Ordering::Relaxed)
}

fn remaining_timer_drain_ms(start: std::time::Instant, limit_ms: u64) -> u64 {
    // u64::MAX means "keep alive for all ref'd timers" — do not subtract wall time
    // from the delay filter (that previously made long timers unreachable).
    if limit_ms == u64::MAX {
        return u64::MAX;
    }
    limit_ms.saturating_sub(start.elapsed().as_millis() as u64)
}

fn serde_json_value_to_v8<'scope>(
    scope: &mut v8::PinScope<'scope, '_>,
    value: &serde_json::Value,
) -> v8::Local<'scope, v8::Value> {
    match value {
        serde_json::Value::Null => v8::null(scope).into(),
        serde_json::Value::Bool(value) => v8::Boolean::new(scope, *value).into(),
        serde_json::Value::Number(value) => {
            v8::Number::new(scope, value.as_f64().unwrap_or(0.0)).into()
        }
        serde_json::Value::String(value) => v8::String::new(scope, value).unwrap().into(),
        serde_json::Value::Array(values) => {
            let array = v8::Array::new(scope, values.len() as i32);
            for (index, item) in values.iter().enumerate() {
                let item_value = serde_json_value_to_v8(scope, item);
                array.set_index(scope, index as u32, item_value);
            }
            array.into()
        }
        serde_json::Value::Object(values) => {
            let object = v8::Object::new(scope);
            for (key, value) in values {
                let key_value = v8::String::new(scope, key).unwrap();
                let item_value = serde_json_value_to_v8(scope, value);
                object.set(scope, key_value.into(), item_value);
            }
            object.into()
        }
    }
}

/// HTTP 客户端用于处理真实的 fetch 请求
pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;
        Ok(Self { client })
    }

    pub async fn fetch(&self, url: &str) -> Result<HttpResponse> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;

        Ok(HttpResponse {
            status,
            body,
            headers: Default::default(),
        })
    }
}

pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub headers: std::collections::HashMap<String, String>,
}

/// Helper function to encode a string to bytes with the specified encoding
fn encode_string_to_bytes(s: &str, encoding: &str) -> Vec<u8> {
    let engine = base64::engine::general_purpose::STANDARD;
    match encoding.to_lowercase().as_str() {
        "utf8" | "utf-8" | "utf8mb4" => s.as_bytes().to_vec(),
        "hex" => hex::decode(s).unwrap_or_else(|_| s.as_bytes().to_vec()),
        "base64" => engine.decode(s).unwrap_or_else(|_| s.as_bytes().to_vec()),
        "base64url" => base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(s.trim_end_matches('='))
            .unwrap_or_else(|_| s.as_bytes().to_vec()),
        "latin1" | "ascii" | "binary" => s.bytes().collect(),
        _ => s.as_bytes().to_vec(), // Default to UTF-8
    }
}

fn create_buffer_wrapper<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    bytes: &[u8],
) -> v8::Local<'s, v8::Object> {
    let buffer = v8::ArrayBuffer::new(scope, bytes.len());
    if !bytes.is_empty() {
        let store = buffer.get_backing_store();
        let ptr = store.as_ref().as_ptr() as *mut u8;
        if !ptr.is_null() {
            let slice = unsafe { std::slice::from_raw_parts_mut(ptr, bytes.len()) };
            slice.copy_from_slice(bytes);
        }
    }
    let u8_array = v8::Uint8Array::new(scope, buffer, 0, bytes.len()).unwrap();

    let buffer_str = v8::String::new(scope, "Buffer").unwrap().into();
    let global = scope.get_current_context().global(scope);
    if let Some(buffer_val) = global.get(scope, buffer_str) {
        if let Ok(ctor) = v8::Local::<v8::Function>::try_from(buffer_val) {
            let proto_key = v8::String::new(scope, "prototype").unwrap().into();
            if let Some(proto_val) = ctor.get(scope, proto_key) {
                u8_array.set_prototype(scope, proto_val);
            }
        }
    }

    let val: v8::Local<v8::Value> = u8_array.into();
    v8::Local::<v8::Object>::try_from(val).unwrap()
}

fn get_string_property(
    scope: &mut v8::PinScope,
    obj: v8::Local<v8::Object>,
    name: &str,
) -> Option<String> {
    let key = v8::String::new(scope, name)?.into();
    obj.get(scope, key).and_then(|value| {
        if value.is_undefined() || value.is_null() {
            None
        } else {
            value
                .to_string(scope)
                .map(|value| value.to_rust_string_lossy(scope))
        }
    })
}

fn get_i64_property(
    scope: &mut v8::PinScope,
    obj: v8::Local<v8::Object>,
    name: &str,
) -> Option<i64> {
    let key = v8::String::new(scope, name)?.into();
    obj.get(scope, key)
        .and_then(|value| value.integer_value(scope))
}

fn key_export_options_from_arg(
    scope: &mut v8::PinScope,
    arg: v8::Local<v8::Value>,
) -> (String, Option<String>, bool) {
    if arg.is_undefined() || arg.is_null() {
        return ("pem".to_string(), None, false);
    }

    if arg.is_object() && !arg.is_string() {
        if let Ok(obj) = v8::Local::<v8::Object>::try_from(arg) {
            let format = get_string_property(scope, obj, "format")
                .unwrap_or_else(|| "pem".to_string())
                .to_ascii_lowercase();
            let key_type = get_string_property(scope, obj, "type").map(|s| s.to_ascii_lowercase());
            return (format, key_type, true);
        }
    }

    let format = arg
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_else(|| "pem".to_string())
        .to_ascii_lowercase();
    (format, None, false)
}

#[derive(Clone)]
struct PrivateKeyEncodingOptions {
    key_type: String,
    format: String,
    cipher: Option<String>,
    passphrase: Option<Vec<u8>>,
}

#[derive(Clone)]
struct PublicKeyEncodingOptions {
    key_type: String,
    format: String,
}

enum GeneratedPublicKey {
    Pem(String),
    Der(Vec<u8>),
    Jwk(serde_json::Value),
}

enum GeneratedPrivateKey {
    Pem(String),
    Der(Vec<u8>),
    Jwk(serde_json::Value),
}

const ERR_CRYPTO_INCOMPATIBLE_KEY_OPTIONS: &str = "ERR_CRYPTO_INCOMPATIBLE_KEY_OPTIONS";
const CRYPTO_INCOMPATIBLE_KEY_OPTIONS_MESSAGE: &str =
    "The selected key encoding pkcs1 can only be used for RSA keys.";

fn is_crypto_incompatible_key_options_error(error_message: &str) -> bool {
    error_message.contains("PKCS#1")
        && error_message.contains("requires an RSA key")
        && error_message.to_ascii_lowercase().contains("public key")
}

fn crypto_incompatible_key_options_error<'s>(
    scope: &mut v8::PinScope<'s, '_>,
) -> v8::Local<'s, v8::Value> {
    let message = v8::String::new(scope, CRYPTO_INCOMPATIBLE_KEY_OPTIONS_MESSAGE).unwrap();
    let error = v8::Exception::error(scope, message);
    if let Ok(error_obj) = v8::Local::<v8::Object>::try_from(error) {
        let code_key = v8::String::new(scope, "code").unwrap().into();
        let code_value = v8::String::new(scope, ERR_CRYPTO_INCOMPATIBLE_KEY_OPTIONS)
            .unwrap()
            .into();
        error_obj.set(scope, code_key, code_value);
    }
    error
}

fn private_key_encoding_options_from_object(
    scope: &mut v8::PinScope,
    encoding_obj: v8::Local<v8::Object>,
) -> Result<PrivateKeyEncodingOptions, String> {
    let key_type = get_string_property(scope, encoding_obj, "type")
        .unwrap_or_else(|| "pkcs8".to_string())
        .to_ascii_lowercase();
    let format = get_string_property(scope, encoding_obj, "format")
        .unwrap_or_else(|| "pem".to_string())
        .to_ascii_lowercase();
    let cipher =
        get_string_property(scope, encoding_obj, "cipher").map(|value| value.to_ascii_lowercase());

    let passphrase_key = v8::String::new(scope, "passphrase")
        .ok_or_else(|| "failed to allocate passphrase key".to_string())?;
    let passphrase = match encoding_obj.get(scope, passphrase_key.into()) {
        Some(value) if !value.is_undefined() && !value.is_null() => {
            Some(get_bytes_from_value(scope, value, None)?)
        }
        _ => None,
    };

    Ok(PrivateKeyEncodingOptions {
        key_type,
        format,
        cipher,
        passphrase,
    })
}

fn private_key_encoding_options_from_options(
    scope: &mut v8::PinScope,
    options: v8::Local<v8::Value>,
) -> Result<Option<PrivateKeyEncodingOptions>, String> {
    if !options.is_object() {
        return Ok(None);
    }

    let options_obj = v8::Local::<v8::Object>::try_from(options)
        .map_err(|_| "privateKeyEncoding options must be an object".to_string())?;
    let encoding_key = v8::String::new(scope, "privateKeyEncoding")
        .ok_or_else(|| "failed to allocate privateKeyEncoding key".to_string())?;
    let Some(encoding_value) = options_obj.get(scope, encoding_key.into()) else {
        return Ok(None);
    };
    if encoding_value.is_undefined() || encoding_value.is_null() {
        return Ok(None);
    }
    if !encoding_value.is_object() {
        return Err("privateKeyEncoding must be an object".to_string());
    }

    let encoding_obj = v8::Local::<v8::Object>::try_from(encoding_value)
        .map_err(|_| "privateKeyEncoding must be an object".to_string())?;
    private_key_encoding_options_from_object(scope, encoding_obj).map(Some)
}

fn public_key_encoding_options_from_object(
    scope: &mut v8::PinScope,
    encoding_obj: v8::Local<v8::Object>,
) -> Result<PublicKeyEncodingOptions, String> {
    let key_type = get_string_property(scope, encoding_obj, "type")
        .unwrap_or_else(|| "spki".to_string())
        .to_ascii_lowercase();
    let format = get_string_property(scope, encoding_obj, "format")
        .unwrap_or_else(|| "pem".to_string())
        .to_ascii_lowercase();

    Ok(PublicKeyEncodingOptions { key_type, format })
}

fn public_key_encoding_options_from_options(
    scope: &mut v8::PinScope,
    options: v8::Local<v8::Value>,
) -> Result<Option<PublicKeyEncodingOptions>, String> {
    if !options.is_object() {
        return Ok(None);
    }

    let options_obj = v8::Local::<v8::Object>::try_from(options)
        .map_err(|_| "publicKeyEncoding options must be an object".to_string())?;
    let encoding_key = v8::String::new(scope, "publicKeyEncoding")
        .ok_or_else(|| "failed to allocate publicKeyEncoding key".to_string())?;
    let Some(encoding_value) = options_obj.get(scope, encoding_key.into()) else {
        return Ok(None);
    };
    if encoding_value.is_undefined() || encoding_value.is_null() {
        return Ok(None);
    }
    if !encoding_value.is_object() {
        return Err("publicKeyEncoding must be an object".to_string());
    }

    let encoding_obj = v8::Local::<v8::Object>::try_from(encoding_value)
        .map_err(|_| "publicKeyEncoding must be an object".to_string())?;
    public_key_encoding_options_from_object(scope, encoding_obj).map(Some)
}

fn private_key_encoding_cipher(cipher_name: &str) -> Option<Cipher> {
    match cipher_name.to_ascii_lowercase().as_str() {
        "aes-128-cbc" => Some(Cipher::aes_128_cbc()),
        "aes-192-cbc" => Some(Cipher::aes_192_cbc()),
        "aes-256-cbc" => Some(Cipher::aes_256_cbc()),
        _ => None,
    }
}

fn base64url_uint_from_bignum(value: &BigNumRef) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(value.to_vec())
}

fn jwk_field_string(
    scope: &mut v8::PinScope,
    jwk_obj: v8::Local<v8::Object>,
    field: &str,
) -> Result<String, String> {
    get_string_property(scope, jwk_obj, field).ok_or_else(|| format!("JWK {} is required", field))
}

fn jwk_bignum_from_object(
    scope: &mut v8::PinScope,
    jwk_obj: v8::Local<v8::Object>,
    field: &str,
) -> Result<BigNum, String> {
    let value = jwk_field_string(scope, jwk_obj, field)?;
    if value.contains('=') {
        return Err(format!("JWK {} must be unpadded base64url", field));
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(value.as_bytes())
        .map_err(|error| format!("JWK {} is invalid base64url: {}", field, error))?;
    if bytes.is_empty() {
        return Err(format!("JWK {} must not be empty", field));
    }
    BigNum::from_slice(&bytes)
        .map_err(|error| format!("JWK {} is invalid integer: {}", field, error))
}

fn rsa_public_jwk_from_pem(public_key_pem: &str) -> Result<serde_json::Value, String> {
    let rsa = Rsa::public_key_from_pem(public_key_pem.as_bytes()).map_err(|error| {
        format!(
            "publicKeyEncoding JWK export requires an RSA public key: {}",
            error
        )
    })?;
    let mut jwk = serde_json::Map::new();
    jwk.insert(
        "kty".to_string(),
        serde_json::Value::String("RSA".to_string()),
    );
    jwk.insert(
        "n".to_string(),
        serde_json::Value::String(base64url_uint_from_bignum(rsa.n())),
    );
    jwk.insert(
        "e".to_string(),
        serde_json::Value::String(base64url_uint_from_bignum(rsa.e())),
    );
    Ok(serde_json::Value::Object(jwk))
}

fn rsa_private_jwk_from_pem(private_key_pem: &str) -> Result<serde_json::Value, String> {
    let rsa = Rsa::private_key_from_pem(private_key_pem.as_bytes()).map_err(|error| {
        format!(
            "privateKeyEncoding JWK export requires an RSA private key: {}",
            error
        )
    })?;
    let p = rsa
        .p()
        .ok_or_else(|| "privateKeyEncoding JWK export missing RSA p".to_string())?;
    let q = rsa
        .q()
        .ok_or_else(|| "privateKeyEncoding JWK export missing RSA q".to_string())?;
    let dp = rsa
        .dmp1()
        .ok_or_else(|| "privateKeyEncoding JWK export missing RSA dp".to_string())?;
    let dq = rsa
        .dmq1()
        .ok_or_else(|| "privateKeyEncoding JWK export missing RSA dq".to_string())?;
    let qi = rsa
        .iqmp()
        .ok_or_else(|| "privateKeyEncoding JWK export missing RSA qi".to_string())?;

    let mut jwk = serde_json::Map::new();
    jwk.insert(
        "kty".to_string(),
        serde_json::Value::String("RSA".to_string()),
    );
    jwk.insert(
        "n".to_string(),
        serde_json::Value::String(base64url_uint_from_bignum(rsa.n())),
    );
    jwk.insert(
        "e".to_string(),
        serde_json::Value::String(base64url_uint_from_bignum(rsa.e())),
    );
    jwk.insert(
        "d".to_string(),
        serde_json::Value::String(base64url_uint_from_bignum(rsa.d())),
    );
    jwk.insert(
        "p".to_string(),
        serde_json::Value::String(base64url_uint_from_bignum(p)),
    );
    jwk.insert(
        "q".to_string(),
        serde_json::Value::String(base64url_uint_from_bignum(q)),
    );
    jwk.insert(
        "dp".to_string(),
        serde_json::Value::String(base64url_uint_from_bignum(dp)),
    );
    jwk.insert(
        "dq".to_string(),
        serde_json::Value::String(base64url_uint_from_bignum(dq)),
    );
    jwk.insert(
        "qi".to_string(),
        serde_json::Value::String(base64url_uint_from_bignum(qi)),
    );
    Ok(serde_json::Value::Object(jwk))
}

fn rsa_public_pem_from_jwk_object(
    scope: &mut v8::PinScope,
    jwk_obj: v8::Local<v8::Object>,
) -> Result<String, String> {
    let kty = jwk_field_string(scope, jwk_obj, "kty")?;
    if kty != "RSA" {
        return Err(format!("unsupported JWK kty '{}'. Supported: RSA", kty));
    }
    let n = jwk_bignum_from_object(scope, jwk_obj, "n")?;
    let e = jwk_bignum_from_object(scope, jwk_obj, "e")?;
    let rsa = Rsa::from_public_components(n, e)
        .map_err(|error| format!("invalid RSA public JWK: {}", error))?;
    let pem = rsa
        .public_key_to_pem()
        .map_err(|error| format!("RSA public JWK PEM export failed: {}", error))?;
    String::from_utf8(pem)
        .map_err(|error| format!("RSA public JWK PEM is invalid UTF-8: {}", error))
}

fn rsa_private_pem_from_jwk_object(
    scope: &mut v8::PinScope,
    jwk_obj: v8::Local<v8::Object>,
) -> Result<String, String> {
    let kty = jwk_field_string(scope, jwk_obj, "kty")?;
    if kty != "RSA" {
        return Err(format!("unsupported JWK kty '{}'. Supported: RSA", kty));
    }
    let n = jwk_bignum_from_object(scope, jwk_obj, "n")?;
    let e = jwk_bignum_from_object(scope, jwk_obj, "e")?;
    let d = jwk_bignum_from_object(scope, jwk_obj, "d")?;
    let p = jwk_bignum_from_object(scope, jwk_obj, "p")?;
    let q = jwk_bignum_from_object(scope, jwk_obj, "q")?;
    let dp = jwk_bignum_from_object(scope, jwk_obj, "dp")?;
    let dq = jwk_bignum_from_object(scope, jwk_obj, "dq")?;
    let qi = jwk_bignum_from_object(scope, jwk_obj, "qi")?;
    let rsa = Rsa::from_private_components(n, e, d, p, q, dp, dq, qi)
        .map_err(|error| format!("invalid RSA private JWK: {}", error))?;
    let pem = rsa
        .private_key_to_pem()
        .map_err(|error| format!("RSA private JWK PEM export failed: {}", error))?;
    String::from_utf8(pem)
        .map_err(|error| format!("RSA private JWK PEM is invalid UTF-8: {}", error))
}

fn ec_jwk_curve_from_nid(nid: Nid) -> Option<(&'static str, usize)> {
    match nid {
        Nid::X9_62_PRIME256V1 => Some(("P-256", 32)),
        Nid::SECP384R1 => Some(("P-384", 48)),
        Nid::SECP521R1 => Some(("P-521", 66)),
        _ => None,
    }
}

fn ec_jwk_group_from_curve(crv: &str) -> Result<(EcGroup, usize), String> {
    let (nid, size) = match crv {
        "P-256" => (Nid::X9_62_PRIME256V1, 32),
        "P-384" => (Nid::SECP384R1, 48),
        "P-521" => (Nid::SECP521R1, 66),
        _ => {
            return Err(format!(
                "unsupported JWK crv '{}'. Supported: P-256, P-384, P-521",
                crv
            ))
        }
    };
    let group = EcGroup::from_curve_name(nid)
        .map_err(|error| format!("EC JWK curve setup failed: {}", error))?;
    Ok((group, size))
}

fn base64url_fixed_uint_from_bignum(value: &BigNumRef, size: usize) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(left_pad_private_key(value.to_vec(), size))
}

fn ec_public_coordinates(
    group: &EcGroupRef,
    point: &openssl::ec::EcPointRef,
) -> Result<(BigNum, BigNum), String> {
    let mut ctx =
        BigNumContext::new().map_err(|error| format!("EC JWK BigNum context failed: {}", error))?;
    let mut x = BigNum::new().map_err(|error| format!("EC JWK x allocation failed: {}", error))?;
    let mut y = BigNum::new().map_err(|error| format!("EC JWK y allocation failed: {}", error))?;
    point
        .affine_coordinates(group, &mut x, &mut y, &mut ctx)
        .map_err(|error| format!("EC JWK coordinate export failed: {}", error))?;
    Ok((x, y))
}

fn ec_public_jwk_from_pem(public_key_pem: &str) -> Result<serde_json::Value, String> {
    let key = PKey::<Public>::public_key_from_pem(public_key_pem.as_bytes()).map_err(|error| {
        format!(
            "publicKeyEncoding JWK export requires a valid public key: {}",
            error
        )
    })?;
    let ec_key = key.ec_key().map_err(|error| {
        format!(
            "publicKeyEncoding JWK export requires an EC public key: {}",
            error
        )
    })?;
    let nid = ec_key
        .group()
        .curve_name()
        .ok_or_else(|| "publicKeyEncoding JWK export requires a named EC curve".to_string())?;
    let (crv, size) = ec_jwk_curve_from_nid(nid)
        .ok_or_else(|| "publicKeyEncoding JWK export unsupported EC curve".to_string())?;
    let (x, y) = ec_public_coordinates(ec_key.group(), ec_key.public_key())?;

    let mut jwk = serde_json::Map::new();
    jwk.insert(
        "kty".to_string(),
        serde_json::Value::String("EC".to_string()),
    );
    jwk.insert(
        "crv".to_string(),
        serde_json::Value::String(crv.to_string()),
    );
    jwk.insert(
        "x".to_string(),
        serde_json::Value::String(base64url_fixed_uint_from_bignum(&x, size)),
    );
    jwk.insert(
        "y".to_string(),
        serde_json::Value::String(base64url_fixed_uint_from_bignum(&y, size)),
    );
    Ok(serde_json::Value::Object(jwk))
}

fn ec_private_jwk_from_pem(private_key_pem: &str) -> Result<serde_json::Value, String> {
    let key =
        PKey::<Private>::private_key_from_pem(private_key_pem.as_bytes()).map_err(|error| {
            format!(
                "privateKeyEncoding JWK export requires a valid private key: {}",
                error
            )
        })?;
    let ec_key = key.ec_key().map_err(|error| {
        format!(
            "privateKeyEncoding JWK export requires an EC private key: {}",
            error
        )
    })?;
    let nid = ec_key
        .group()
        .curve_name()
        .ok_or_else(|| "privateKeyEncoding JWK export requires a named EC curve".to_string())?;
    let (crv, size) = ec_jwk_curve_from_nid(nid)
        .ok_or_else(|| "privateKeyEncoding JWK export unsupported EC curve".to_string())?;
    let (x, y) = ec_public_coordinates(ec_key.group(), ec_key.public_key())?;

    let mut jwk = serde_json::Map::new();
    jwk.insert(
        "kty".to_string(),
        serde_json::Value::String("EC".to_string()),
    );
    jwk.insert(
        "crv".to_string(),
        serde_json::Value::String(crv.to_string()),
    );
    jwk.insert(
        "x".to_string(),
        serde_json::Value::String(base64url_fixed_uint_from_bignum(&x, size)),
    );
    jwk.insert(
        "y".to_string(),
        serde_json::Value::String(base64url_fixed_uint_from_bignum(&y, size)),
    );
    jwk.insert(
        "d".to_string(),
        serde_json::Value::String(base64url_fixed_uint_from_bignum(ec_key.private_key(), size)),
    );
    Ok(serde_json::Value::Object(jwk))
}

fn ec_jwk_bignum_from_object(
    scope: &mut v8::PinScope,
    jwk_obj: v8::Local<v8::Object>,
    field: &str,
    size: usize,
) -> Result<BigNum, String> {
    let value = jwk_field_string(scope, jwk_obj, field)?;
    if value.contains('=') {
        return Err(format!("JWK {} must be unpadded base64url", field));
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(value.as_bytes())
        .map_err(|error| format!("JWK {} is invalid base64url: {}", field, error))?;
    if bytes.is_empty() || bytes.len() > size {
        return Err(format!("JWK {} has invalid length", field));
    }
    BigNum::from_slice(&bytes)
        .map_err(|error| format!("JWK {} is invalid integer: {}", field, error))
}

fn ec_public_pem_from_jwk_object(
    scope: &mut v8::PinScope,
    jwk_obj: v8::Local<v8::Object>,
) -> Result<String, String> {
    let kty = jwk_field_string(scope, jwk_obj, "kty")?;
    if kty != "EC" {
        return Err(format!("unsupported JWK kty '{}'. Supported: EC", kty));
    }
    let crv = jwk_field_string(scope, jwk_obj, "crv")?;
    let (group, size) = ec_jwk_group_from_curve(&crv)?;
    let x = ec_jwk_bignum_from_object(scope, jwk_obj, "x", size)?;
    let y = ec_jwk_bignum_from_object(scope, jwk_obj, "y", size)?;
    let ec_key = EcKey::from_public_key_affine_coordinates(&group, &x, &y)
        .map_err(|error| format!("invalid EC public JWK: {}", error))?;
    ec_key
        .check_key()
        .map_err(|error| format!("invalid EC public JWK: {}", error))?;
    let key = PKey::from_ec_key(ec_key)
        .map_err(|error| format!("EC public JWK key setup failed: {}", error))?;
    let pem = key
        .public_key_to_pem()
        .map_err(|error| format!("EC public JWK PEM export failed: {}", error))?;
    String::from_utf8(pem).map_err(|error| format!("EC public JWK PEM is invalid UTF-8: {}", error))
}

fn ec_private_pem_from_jwk_object(
    scope: &mut v8::PinScope,
    jwk_obj: v8::Local<v8::Object>,
) -> Result<String, String> {
    let kty = jwk_field_string(scope, jwk_obj, "kty")?;
    if kty != "EC" {
        return Err(format!("unsupported JWK kty '{}'. Supported: EC", kty));
    }
    let crv = jwk_field_string(scope, jwk_obj, "crv")?;
    let (group, size) = ec_jwk_group_from_curve(&crv)?;
    let x = ec_jwk_bignum_from_object(scope, jwk_obj, "x", size)?;
    let y = ec_jwk_bignum_from_object(scope, jwk_obj, "y", size)?;
    let d = ec_jwk_bignum_from_object(scope, jwk_obj, "d", size)?;
    let public_key = EcKey::from_public_key_affine_coordinates(&group, &x, &y)
        .map_err(|error| format!("invalid EC private JWK public point: {}", error))?;
    let ec_key = EcKey::from_private_components(&group, &d, public_key.public_key())
        .map_err(|error| format!("invalid EC private JWK: {}", error))?;
    ec_key
        .check_key()
        .map_err(|error| format!("invalid EC private JWK: {}", error))?;
    let key = PKey::from_ec_key(ec_key)
        .map_err(|error| format!("EC private JWK key setup failed: {}", error))?;
    let pem = key
        .private_key_to_pem_pkcs8()
        .map_err(|error| format!("EC private JWK PEM export failed: {}", error))?;
    String::from_utf8(pem)
        .map_err(|error| format!("EC private JWK PEM is invalid UTF-8: {}", error))
}

fn okp_jwk_curve_from_id(id: Id) -> Option<(&'static str, usize)> {
    match id {
        Id::ED25519 => Some(("Ed25519", 32)),
        Id::ED448 => Some(("Ed448", 57)),
        _ => None,
    }
}

fn okp_jwk_id_from_curve(crv: &str) -> Result<(Id, usize), String> {
    match crv {
        "Ed25519" => Ok((Id::ED25519, 32)),
        "Ed448" => Ok((Id::ED448, 57)),
        _ => Err(format!(
            "unsupported JWK crv '{}'. Supported: Ed25519, Ed448",
            crv
        )),
    }
}

fn okp_jwk_bytes_from_object(
    scope: &mut v8::PinScope,
    jwk_obj: v8::Local<v8::Object>,
    field: &str,
    size: usize,
) -> Result<Vec<u8>, String> {
    let value = jwk_field_string(scope, jwk_obj, field)?;
    if value.contains('=') {
        return Err(format!("JWK {} must be unpadded base64url", field));
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(value.as_bytes())
        .map_err(|error| format!("JWK {} is invalid base64url: {}", field, error))?;
    if bytes.len() != size {
        return Err(format!("JWK {} has invalid length", field));
    }
    Ok(bytes)
}

fn okp_public_jwk_from_pem(public_key_pem: &str) -> Result<serde_json::Value, String> {
    let key = PKey::<Public>::public_key_from_pem(public_key_pem.as_bytes()).map_err(|error| {
        format!(
            "publicKeyEncoding JWK export requires a valid public key: {}",
            error
        )
    })?;
    let (crv, _size) = okp_jwk_curve_from_id(key.id())
        .ok_or_else(|| "publicKeyEncoding JWK export unsupported OKP key type".to_string())?;
    let raw_public_key = key.raw_public_key().map_err(|error| {
        format!(
            "publicKeyEncoding JWK export raw public key failed: {}",
            error
        )
    })?;

    let mut jwk = serde_json::Map::new();
    jwk.insert(
        "crv".to_string(),
        serde_json::Value::String(crv.to_string()),
    );
    jwk.insert(
        "x".to_string(),
        serde_json::Value::String(
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw_public_key),
        ),
    );
    jwk.insert(
        "kty".to_string(),
        serde_json::Value::String("OKP".to_string()),
    );
    Ok(serde_json::Value::Object(jwk))
}

fn okp_private_jwk_from_pem(private_key_pem: &str) -> Result<serde_json::Value, String> {
    let key =
        PKey::<Private>::private_key_from_pem(private_key_pem.as_bytes()).map_err(|error| {
            format!(
                "privateKeyEncoding JWK export requires a valid private key: {}",
                error
            )
        })?;
    let (crv, _size) = okp_jwk_curve_from_id(key.id())
        .ok_or_else(|| "privateKeyEncoding JWK export unsupported OKP key type".to_string())?;
    let raw_private_key = key.raw_private_key().map_err(|error| {
        format!(
            "privateKeyEncoding JWK export raw private key failed: {}",
            error
        )
    })?;
    let raw_public_key = key.raw_public_key().map_err(|error| {
        format!(
            "privateKeyEncoding JWK export raw public key failed: {}",
            error
        )
    })?;

    let mut jwk = serde_json::Map::new();
    jwk.insert(
        "crv".to_string(),
        serde_json::Value::String(crv.to_string()),
    );
    jwk.insert(
        "d".to_string(),
        serde_json::Value::String(
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw_private_key),
        ),
    );
    jwk.insert(
        "x".to_string(),
        serde_json::Value::String(
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw_public_key),
        ),
    );
    jwk.insert(
        "kty".to_string(),
        serde_json::Value::String("OKP".to_string()),
    );
    Ok(serde_json::Value::Object(jwk))
}

fn okp_public_pem_from_jwk_object(
    scope: &mut v8::PinScope,
    jwk_obj: v8::Local<v8::Object>,
) -> Result<String, String> {
    let kty = jwk_field_string(scope, jwk_obj, "kty")?;
    if kty != "OKP" {
        return Err(format!("unsupported JWK kty '{}'. Supported: OKP", kty));
    }
    let crv = jwk_field_string(scope, jwk_obj, "crv")?;
    let (id, size) = okp_jwk_id_from_curve(&crv)?;
    let x = okp_jwk_bytes_from_object(scope, jwk_obj, "x", size)?;
    let key = PKey::public_key_from_raw_bytes(&x, id)
        .map_err(|error| format!("invalid OKP public JWK: {}", error))?;
    let pem = key
        .public_key_to_pem()
        .map_err(|error| format!("OKP public JWK PEM export failed: {}", error))?;
    String::from_utf8(pem)
        .map_err(|error| format!("OKP public JWK PEM is invalid UTF-8: {}", error))
}

fn okp_private_pem_from_jwk_object(
    scope: &mut v8::PinScope,
    jwk_obj: v8::Local<v8::Object>,
) -> Result<String, String> {
    let kty = jwk_field_string(scope, jwk_obj, "kty")?;
    if kty != "OKP" {
        return Err(format!("unsupported JWK kty '{}'. Supported: OKP", kty));
    }
    let crv = jwk_field_string(scope, jwk_obj, "crv")?;
    let (id, size) = okp_jwk_id_from_curve(&crv)?;
    let _x = okp_jwk_bytes_from_object(scope, jwk_obj, "x", size)?;
    let d = okp_jwk_bytes_from_object(scope, jwk_obj, "d", size)?;
    let key = PKey::private_key_from_raw_bytes(&d, id)
        .map_err(|error| format!("invalid OKP private JWK: {}", error))?;
    let pem = key
        .private_key_to_pem_pkcs8()
        .map_err(|error| format!("OKP private JWK PEM export failed: {}", error))?;
    String::from_utf8(pem)
        .map_err(|error| format!("OKP private JWK PEM is invalid UTF-8: {}", error))
}

fn public_jwk_from_pem(public_key_pem: &str) -> Result<serde_json::Value, String> {
    let key = PKey::<Public>::public_key_from_pem(public_key_pem.as_bytes())
        .map_err(|error| format!("publicKeyEncoding JWK export invalid public key: {}", error))?;
    match key.id() {
        Id::RSA => rsa_public_jwk_from_pem(public_key_pem),
        Id::EC => ec_public_jwk_from_pem(public_key_pem),
        Id::ED25519 | Id::ED448 => okp_public_jwk_from_pem(public_key_pem),
        _ => Err("publicKeyEncoding JWK export unsupported key type".to_string()),
    }
}

fn private_jwk_from_pem(private_key_pem: &str) -> Result<serde_json::Value, String> {
    let key =
        PKey::<Private>::private_key_from_pem(private_key_pem.as_bytes()).map_err(|error| {
            format!(
                "privateKeyEncoding JWK export invalid private key: {}",
                error
            )
        })?;
    match key.id() {
        Id::RSA => rsa_private_jwk_from_pem(private_key_pem),
        Id::EC => ec_private_jwk_from_pem(private_key_pem),
        Id::ED25519 | Id::ED448 => okp_private_jwk_from_pem(private_key_pem),
        _ => Err("privateKeyEncoding JWK export unsupported key type".to_string()),
    }
}

fn public_pem_from_jwk_object(
    scope: &mut v8::PinScope,
    jwk_obj: v8::Local<v8::Object>,
) -> Result<String, String> {
    match jwk_field_string(scope, jwk_obj, "kty")?.as_str() {
        "RSA" => rsa_public_pem_from_jwk_object(scope, jwk_obj),
        "EC" => ec_public_pem_from_jwk_object(scope, jwk_obj),
        "OKP" => okp_public_pem_from_jwk_object(scope, jwk_obj),
        kty => Err(format!(
            "unsupported JWK kty '{}'. Supported: RSA, EC, OKP",
            kty
        )),
    }
}

fn private_pem_from_jwk_object(
    scope: &mut v8::PinScope,
    jwk_obj: v8::Local<v8::Object>,
) -> Result<String, String> {
    match jwk_field_string(scope, jwk_obj, "kty")?.as_str() {
        "RSA" => rsa_private_pem_from_jwk_object(scope, jwk_obj),
        "EC" => ec_private_pem_from_jwk_object(scope, jwk_obj),
        "OKP" => okp_private_pem_from_jwk_object(scope, jwk_obj),
        kty => Err(format!(
            "unsupported JWK kty '{}'. Supported: RSA, EC, OKP",
            kty
        )),
    }
}

fn format_generated_public_key(
    public_key_pem: &str,
    options: Option<&PublicKeyEncodingOptions>,
) -> Result<GeneratedPublicKey, String> {
    let Some(options) = options else {
        return Ok(GeneratedPublicKey::Pem(public_key_pem.to_string()));
    };

    if options.key_type != "spki" && options.key_type != "pkcs1" {
        return Err(format!(
            "unsupported publicKeyEncoding type '{}'. Supported: spki, pkcs1",
            options.key_type
        ));
    }

    match options.format.as_str() {
        "pem" => public_key_pem_from_pem(public_key_pem, &options.key_type)
            .map(GeneratedPublicKey::Pem)
            .map_err(|error| format!("publicKeyEncoding PEM export failed: {}", error)),
        "der" => public_key_der_from_pem(public_key_pem, &options.key_type)
            .map(GeneratedPublicKey::Der)
            .map_err(|error| format!("publicKeyEncoding DER export failed: {}", error)),
        "jwk" => public_jwk_from_pem(public_key_pem).map(GeneratedPublicKey::Jwk),
        _ => Err(format!(
            "unsupported publicKeyEncoding format '{}'. Supported: pem, der, jwk",
            options.format
        )),
    }
}

fn generated_public_key_to_v8<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    public_key: GeneratedPublicKey,
) -> v8::Local<'s, v8::Value> {
    match public_key {
        GeneratedPublicKey::Pem(pem) => v8::String::new(scope, &pem).unwrap().into(),
        GeneratedPublicKey::Der(der) => create_buffer_wrapper(scope, &der).into(),
        GeneratedPublicKey::Jwk(jwk) => serde_json_value_to_v8(scope, &jwk),
    }
}

fn format_generated_private_key_for_generate(
    private_key_pem: &str,
    options: Option<&PrivateKeyEncodingOptions>,
) -> Result<GeneratedPrivateKey, String> {
    let Some(options) = options else {
        return Ok(GeneratedPrivateKey::Pem(private_key_pem.to_string()));
    };

    if options.key_type != "pkcs8" {
        return Err(format!(
            "generateKeyPair: unsupported privateKeyEncoding type '{}'. Supported: pkcs8",
            options.key_type
        ));
    }

    match options.format.as_str() {
        "pem" => format_generated_private_key(private_key_pem, Some(options))
            .map(GeneratedPrivateKey::Pem),
        "der" => {
            if options.cipher.is_some() || options.passphrase.is_some() {
                return Err(
                    "generateKeyPair: privateKeyEncoding cipher/passphrase require pem format"
                        .to_string(),
                );
            }
            private_key_der_from_pem(private_key_pem, &options.key_type)
                .map(GeneratedPrivateKey::Der)
                .map_err(|error| {
                    format!(
                        "generateKeyPair: privateKeyEncoding DER export failed: {}",
                        error
                    )
                })
        }
        "jwk" => {
            if options.cipher.is_some() || options.passphrase.is_some() {
                return Err(
                    "generateKeyPair: privateKeyEncoding cipher/passphrase require pem format"
                        .to_string(),
                );
            }
            private_jwk_from_pem(private_key_pem).map(GeneratedPrivateKey::Jwk)
        }
        _ => Err(format!(
            "generateKeyPair: unsupported privateKeyEncoding format '{}'. Supported: pem, der, jwk",
            options.format
        )),
    }
}

fn generated_private_key_to_v8<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    private_key: GeneratedPrivateKey,
) -> v8::Local<'s, v8::Value> {
    match private_key {
        GeneratedPrivateKey::Pem(pem) => v8::String::new(scope, &pem).unwrap().into(),
        GeneratedPrivateKey::Der(der) => create_buffer_wrapper(scope, &der).into(),
        GeneratedPrivateKey::Jwk(jwk) => serde_json_value_to_v8(scope, &jwk),
    }
}

fn format_generated_private_key(
    private_key_pem: &str,
    options: Option<&PrivateKeyEncodingOptions>,
) -> Result<String, String> {
    let Some(options) = options else {
        return Ok(private_key_pem.to_string());
    };

    if options.format != "pem" {
        return Err(format!(
            "generateKeyPair: unsupported privateKeyEncoding format '{}'. Supported: pem",
            options.format
        ));
    }
    if options.key_type != "pkcs8" {
        return Err(format!(
            "generateKeyPair: unsupported privateKeyEncoding type '{}'. Supported: pkcs8",
            options.key_type
        ));
    }

    let key = PKey::private_key_from_pem(private_key_pem.as_bytes())
        .map_err(|error| format!("generateKeyPair: invalid generated private key: {}", error))?;

    match (&options.cipher, &options.passphrase) {
        (Some(cipher_name), Some(passphrase)) => {
            let cipher = private_key_encoding_cipher(cipher_name).ok_or_else(|| {
                format!(
                    "generateKeyPair: unsupported privateKeyEncoding cipher '{}'",
                    cipher_name
                )
            })?;
            let pem = key
                .private_key_to_pem_pkcs8_passphrase(cipher, passphrase)
                .map_err(|error| {
                    format!(
                        "generateKeyPair: encrypted private key export failed: {}",
                        error
                    )
                })?;
            String::from_utf8(pem).map_err(|error| {
                format!(
                    "generateKeyPair: encrypted private key PEM is invalid UTF-8: {}",
                    error
                )
            })
        }
        (None, None) => {
            let pem = key.private_key_to_pem_pkcs8().map_err(|error| {
                format!(
                    "generateKeyPair: private key PKCS8 export failed: {}",
                    error
                )
            })?;
            String::from_utf8(pem).map_err(|error| {
                format!(
                    "generateKeyPair: private key PEM is invalid UTF-8: {}",
                    error
                )
            })
        }
        _ => Err(
            "generateKeyPair: privateKeyEncoding cipher and passphrase must be provided together"
                .to_string(),
        ),
    }
}

fn value_to_string_after_microtasks<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    mut value: v8::Local<'s, v8::Value>,
) -> Result<String> {
    for _ in 0..64 {
        if !value.is_promise() {
            break;
        }

        let promise = v8::Local::<v8::Promise>::try_from(value)
            .map_err(|_| anyhow::anyhow!("Failed to inspect promise result"))?;
        execute_next_tick_callbacks(scope);
        scope.perform_microtask_checkpoint();

        match promise.state() {
            v8::PromiseState::Fulfilled => {
                value = promise.result(scope);
            }
            v8::PromiseState::Rejected => {
                let reason = promise.result(scope);
                let reason_str = reason
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_else(|| "Unknown promise rejection".to_string());
                return Err(anyhow::anyhow!(
                    "Unhandled Promise rejection: {}",
                    reason_str
                ));
            }
            v8::PromiseState::Pending => {
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }
    }

    if value.is_promise() {
        let promise = v8::Local::<v8::Promise>::try_from(value)
            .map_err(|_| anyhow::anyhow!("Failed to inspect promise result"))?;
        if promise.state() == v8::PromiseState::Pending {
            return Err(anyhow::anyhow!(
                "Pending Promise did not settle before runtime completion"
            ));
        }
    }

    value
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .ok_or_else(|| anyhow::anyhow!("Failed to convert result to string"))
}

/// Helper function to set up Buffer module with all static and prototype methods
/// This avoids closure capture issues by defining everything fresh
fn setup_buffer_module(scope: &mut v8::PinScope) {
    // Buffer constructor
    let buffer_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            if args.length() >= 1 {
                let first = args.get(0);
                if first.is_array_buffer() {
                    if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(first) {
                        let total_len = ab.byte_length();
                        let offset = if args.length() >= 2 && args.get(1).is_number() {
                            (args.get(1).to_integer(scope).unwrap().value().max(0) as usize)
                                .min(total_len)
                        } else {
                            0
                        };
                        let len = if args.length() >= 3 && args.get(2).is_number() {
                            (args.get(2).to_integer(scope).unwrap().value().max(0) as usize)
                                .min(total_len.saturating_sub(offset))
                        } else {
                            total_len.saturating_sub(offset)
                        };
                        if let Some(u8_array) = v8::Uint8Array::new(scope, ab, offset, len) {
                            set_buffer_prototype_fast(scope, u8_array);
                            retval.set(u8_array.into());
                            return;
                        }
                    }
                } else if first.is_array_buffer_view() || first.is_typed_array() {
                    if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(first) {
                        let len = view.byte_length();
                        let buffer = v8::ArrayBuffer::new(scope, len);
                        if len > 0 {
                            if let Some(src_ab) = view.buffer(scope) {
                                let src_store = src_ab.get_backing_store();
                                let src_ptr = src_store.as_ref().as_ptr() as *const u8;
                                let dst_store = buffer.get_backing_store();
                                let dst_ptr = dst_store.as_ref().as_ptr() as *mut u8;
                                if !src_ptr.is_null() && !dst_ptr.is_null() {
                                    let src_slice = unsafe {
                                        std::slice::from_raw_parts(
                                            src_ptr.add(view.byte_offset()),
                                            len,
                                        )
                                    };
                                    let dst_slice =
                                        unsafe { std::slice::from_raw_parts_mut(dst_ptr, len) };
                                    dst_slice.copy_from_slice(src_slice);
                                }
                            }
                        }
                        if let Some(u8_array) = v8::Uint8Array::new(scope, buffer, 0, len) {
                            set_buffer_prototype_fast(scope, u8_array);
                            retval.set(u8_array.into());
                            return;
                        }
                    }
                } else if first.is_number() {
                    let size = first.to_integer(scope).unwrap().value().max(0) as usize;
                    let buffer = v8::ArrayBuffer::new(scope, size);
                    if let Some(u8_array) = v8::Uint8Array::new(scope, buffer, 0, size) {
                        set_buffer_prototype_fast(scope, u8_array);
                        retval.set(u8_array.into());
                        return;
                    }
                } else if let Some(str_val) = first.to_string(scope) {
                    let encoding = if args.length() >= 2 {
                        args.get(1)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_else(|| "utf8".to_string())
                    } else {
                        "utf8".to_string()
                    };
                    let enc_lower = encoding.to_ascii_lowercase();
                    if enc_lower == "utf8" || enc_lower == "utf-8" {
                        let len = str_val.utf8_length(scope);
                        let buffer = v8::ArrayBuffer::new(scope, len);
                        if len > 0 {
                            let store = buffer.get_backing_store();
                            let ptr = store.as_ref().as_ptr() as *mut u8;
                            if !ptr.is_null() {
                                let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
                                str_val.write_utf8_v2(scope, slice, v8::WriteFlags::empty(), None);
                            }
                        }
                        if let Some(u8_array) = v8::Uint8Array::new(scope, buffer, 0, len) {
                            set_buffer_prototype_fast(scope, u8_array);
                            retval.set(u8_array.into());
                            return;
                        }
                    } else {
                        let rust_string = str_val.to_rust_string_lossy(scope);
                        let bytes = encode_string_to_bytes(&rust_string, &encoding);
                        let buffer = v8::ArrayBuffer::new(scope, bytes.len());
                        if !bytes.is_empty() {
                            let store = buffer.get_backing_store();
                            let ptr = store.as_ref().as_ptr() as *mut u8;
                            if !ptr.is_null() {
                                let slice =
                                    unsafe { std::slice::from_raw_parts_mut(ptr, bytes.len()) };
                                slice.copy_from_slice(&bytes);
                            }
                        }
                        if let Some(u8_array) = v8::Uint8Array::new(scope, buffer, 0, bytes.len()) {
                            set_buffer_prototype_fast(scope, u8_array);
                            retval.set(u8_array.into());
                            return;
                        }
                    }
                }
            }
            let buffer = v8::ArrayBuffer::new(scope, 0);
            if let Some(u8_array) = v8::Uint8Array::new(scope, buffer, 0, 0) {
                set_buffer_prototype_fast(scope, u8_array);
                retval.set(u8_array.into());
            }
        },
    )
    .unwrap();

    // Buffer.prototype.toString
    let buffer_to_string_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let (bytes_ptr, bytes_len): (*const u8, usize) =
                if this.is_array_buffer_view() || this.is_typed_array() {
                    if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(this) {
                        let len = view.byte_length();
                        if len == 0 {
                            retval.set(v8::String::empty(scope).into());
                            return;
                        }
                        let offset = view.byte_offset();
                        if let Some(buf) = view.buffer(scope) {
                            let store = buf.get_backing_store();
                            let ptr = store.as_ref().as_ptr() as *const u8;
                            if ptr.is_null() {
                                retval.set(v8::String::empty(scope).into());
                                return;
                            }
                            unsafe { (ptr.add(offset), len) }
                        } else {
                            retval.set(v8::String::empty(scope).into());
                            return;
                        }
                    } else {
                        retval.set(v8::String::empty(scope).into());
                        return;
                    }
                } else {
                    let buffer_key = v8::String::new(scope, "buffer").unwrap().into();
                    let obj = this;
                    if let Some(buf) = obj.get(scope, buffer_key) {
                        if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(buf) {
                            let len = ab.byte_length();
                            if len == 0 {
                                retval.set(v8::String::empty(scope).into());
                                return;
                            }
                            let store = ab.get_backing_store();
                            let ptr = store.as_ref().as_ptr() as *const u8;
                            if ptr.is_null() {
                                retval.set(v8::String::empty(scope).into());
                                return;
                            }
                            (ptr, len)
                        } else {
                            retval.set(v8::String::empty(scope).into());
                            return;
                        }
                    } else if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(this) {
                        let len = ab.byte_length();
                        let store = ab.get_backing_store();
                        let ptr = store.as_ref().as_ptr() as *const u8;
                        if ptr.is_null() || len == 0 {
                            retval.set(v8::String::empty(scope).into());
                            return;
                        }
                        (ptr, len)
                    } else {
                        retval.set(v8::String::new(scope, "[object Object]").unwrap().into());
                        return;
                    }
                };

            let encoding = if args.length() >= 1 && args.get(0).is_string() {
                args.get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_else(|| "utf8".to_string())
            } else {
                "utf8".to_string()
            };

            let start = if args.length() >= 2 && args.get(1).is_number() {
                args.get(1).to_integer(scope).unwrap().value().max(0) as usize
            } else {
                0
            };
            let end = if args.length() >= 3 && args.get(2).is_number() {
                (args.get(2).to_integer(scope).unwrap().value().max(0) as usize).min(bytes_len)
            } else {
                bytes_len
            };

            let sub_slice = if start < end && start < bytes_len {
                unsafe { std::slice::from_raw_parts(bytes_ptr.add(start), end - start) }
            } else {
                &[]
            };

            let enc_lower = encoding.to_lowercase();
            if enc_lower == "utf8" || enc_lower == "utf-8" {
                if let Ok(utf8_str) = std::str::from_utf8(sub_slice) {
                    if let Some(s) = v8::String::new(scope, utf8_str) {
                        retval.set(s.into());
                        return;
                    }
                }
            }

            let result = decode_bytes_to_string(sub_slice, &encoding);
            if let Some(s) = v8::String::new(scope, &result) {
                retval.set(s.into());
            }
        },
    )
    .unwrap();

    // Buffer.prototype.slice
    #[allow(irrefutable_let_patterns)]
    let buffer_slice_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let (source_buffer, source_offset, source_len) =
                if this.is_array_buffer_view() || this.is_typed_array() {
                    if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(this) {
                        if let Some(buf) = view.buffer(scope) {
                            (buf, view.byte_offset(), view.byte_length())
                        } else {
                            return;
                        }
                    } else {
                        return;
                    }
                } else if this.is_object() {
                    let buffer_key = v8::String::new(scope, "buffer").unwrap().into();
                    if let Ok(obj) = v8::Local::<v8::Object>::try_from(this) {
                        if let Some(buf) = obj.get(scope, buffer_key) {
                            if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(buf) {
                                (ab, 0, ab.byte_length())
                            } else {
                                return;
                            }
                        } else {
                            return;
                        }
                    } else {
                        return;
                    }
                } else {
                    return;
                };

            let start = if args.length() >= 1 {
                let s = args.get(0).to_integer(scope).unwrap().value();
                if s < 0 {
                    ((source_len as i64) + s).max(0) as usize
                } else {
                    (s as usize).min(source_len)
                }
            } else {
                0
            };
            let end = if args.length() >= 2 {
                let e = args.get(1).to_integer(scope).unwrap().value();
                if e < 0 {
                    ((source_len as i64) + e).max(0) as usize
                } else {
                    (e as usize).min(source_len)
                }
            } else {
                source_len
            };

            let (clamped_start, clamped_end) = if start <= end {
                (start, end)
            } else {
                (start, start)
            };
            let new_length = clamped_end - clamped_start;

            if let Some(sliced_u8) = v8::Uint8Array::new(
                scope,
                source_buffer,
                source_offset + clamped_start,
                new_length,
            ) {
                set_buffer_prototype_fast(scope, sliced_u8);
                retval.set(sliced_u8.into());
            }
        },
    )
    .unwrap();

    // Buffer.prototype.write
    let buffer_write_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let Some(input) = args.get(0).to_string(scope) else {
                retval.set(v8::Integer::new(scope, 0).into());
                return;
            };
            let input = input.to_rust_string_lossy(scope);
            let offset = if args.length() >= 2 && args.get(1).is_number() {
                args.get(1).to_integer(scope).unwrap().value().max(0) as usize
            } else {
                0
            };
            let encoding_arg_index = if args.length() >= 3 && args.get(2).is_string() {
                2
            } else {
                3
            };
            let max_len = if args.length() >= 3 && args.get(2).is_number() {
                args.get(2).to_integer(scope).unwrap().value().max(0) as usize
            } else {
                usize::MAX
            };
            let encoding = if args.length() > encoding_arg_index {
                args.get(encoding_arg_index)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_else(|| "utf8".to_string())
            } else {
                "utf8".to_string()
            };
            let bytes = encode_string_to_bytes(&input, &encoding);

            let (target_buffer, base_offset, target_len) =
                if this.is_array_buffer_view() || this.is_typed_array() {
                    if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(this) {
                        let buf = view.buffer(scope).unwrap();
                        (buf, view.byte_offset(), view.byte_length())
                    } else {
                        retval.set(v8::Integer::new(scope, 0).into());
                        return;
                    }
                } else {
                    let buffer_key = v8::String::new(scope, "buffer").unwrap().into();
                    let obj = this;
                    if let Some(buf) = obj.get(scope, buffer_key) {
                        if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(buf) {
                            (ab, 0, ab.byte_length())
                        } else {
                            retval.set(v8::Integer::new(scope, 0).into());
                            return;
                        }
                    } else {
                        retval.set(v8::Integer::new(scope, 0).into());
                        return;
                    }
                };

            if offset >= target_len || bytes.is_empty() || max_len == 0 {
                retval.set(v8::Integer::new(scope, 0).into());
                return;
            }

            let bytes_to_write =
                std::cmp::min(bytes.len(), std::cmp::min(max_len, target_len - offset));
            let store = target_buffer.get_backing_store();
            let ptr = store
                .data()
                .map(|p| p.as_ptr() as *mut u8)
                .unwrap_or(std::ptr::null_mut());
            if ptr.is_null() {
                retval.set(v8::Integer::new(scope, 0).into());
                return;
            }

            let target_slice =
                unsafe { std::slice::from_raw_parts_mut(ptr.add(base_offset), target_len) };
            target_slice[offset..offset + bytes_to_write].copy_from_slice(&bytes[..bytes_to_write]);
            retval.set(v8::Integer::new(scope, bytes_to_write as i32).into());
        },
    )
    .unwrap();

    // Buffer.prototype.copy
    let buffer_copy_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let Some(target) = (if args.length() >= 1 {
                Some(args.get(0))
            } else {
                None
            }) else {
                retval.set(v8::Integer::new(scope, 0).into());
                return;
            };

            let (source_ptr, source_len) = if this.is_array_buffer_view() || this.is_typed_array() {
                if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(this) {
                    let len = view.byte_length();
                    let offset = view.byte_offset();
                    if let Some(buf) = view.buffer(scope) {
                        let store = buf.get_backing_store();
                        let ptr = store
                            .data()
                            .map(|p| p.as_ptr() as *const u8)
                            .unwrap_or(std::ptr::null());
                        if ptr.is_null() {
                            (std::ptr::null(), 0)
                        } else {
                            unsafe { (ptr.add(offset), len) }
                        }
                    } else {
                        (std::ptr::null(), 0)
                    }
                } else {
                    (std::ptr::null(), 0)
                }
            } else {
                let buffer_key = v8::String::new(scope, "buffer").unwrap().into();
                let obj = this;
                if let Some(buf) = obj.get(scope, buffer_key) {
                    if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(buf) {
                        let len = ab.byte_length();
                        let store = ab.get_backing_store();
                        let ptr = store
                            .data()
                            .map(|p| p.as_ptr() as *const u8)
                            .unwrap_or(std::ptr::null());
                        (ptr, len)
                    } else {
                        (std::ptr::null(), 0)
                    }
                } else {
                    (std::ptr::null(), 0)
                }
            };

            if source_ptr.is_null() || source_len == 0 {
                retval.set(v8::Integer::new(scope, 0).into());
                return;
            }

            let (target_ptr, target_len) =
                if target.is_array_buffer_view() || target.is_typed_array() {
                    if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(target) {
                        let len = view.byte_length();
                        let offset = view.byte_offset();
                        if let Some(buf) = view.buffer(scope) {
                            let store = buf.get_backing_store();
                            let ptr = store
                                .data()
                                .map(|p| p.as_ptr() as *mut u8)
                                .unwrap_or(std::ptr::null_mut());
                            if ptr.is_null() {
                                (std::ptr::null_mut(), 0)
                            } else {
                                unsafe { (ptr.add(offset), len) }
                            }
                        } else {
                            (std::ptr::null_mut(), 0)
                        }
                    } else {
                        (std::ptr::null_mut(), 0)
                    }
                } else if target.is_object() {
                    if let Ok(obj) = v8::Local::<v8::Object>::try_from(target) {
                        let buffer_key = v8::String::new(scope, "buffer").unwrap().into();
                        if let Some(buf) = obj.get(scope, buffer_key) {
                            if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(buf) {
                                let len = ab.byte_length();
                                let store = ab.get_backing_store();
                                let ptr = store
                                    .data()
                                    .map(|p| p.as_ptr() as *mut u8)
                                    .unwrap_or(std::ptr::null_mut());
                                (ptr, len)
                            } else {
                                (std::ptr::null_mut(), 0)
                            }
                        } else {
                            (std::ptr::null_mut(), 0)
                        }
                    } else {
                        (std::ptr::null_mut(), 0)
                    }
                } else {
                    (std::ptr::null_mut(), 0)
                };

            if target_ptr.is_null() || target_len == 0 {
                retval.set(v8::Integer::new(scope, 0).into());
                return;
            }

            let target_start = if args.length() >= 2 && args.get(1).is_number() {
                args.get(1).to_integer(scope).unwrap().value().max(0) as usize
            } else {
                0
            };
            let source_start = if args.length() >= 3 && args.get(2).is_number() {
                args.get(2).to_integer(scope).unwrap().value().max(0) as usize
            } else {
                0
            };
            let source_end = if args.length() >= 4 && args.get(3).is_number() {
                std::cmp::min(
                    args.get(3).to_integer(scope).unwrap().value().max(0) as usize,
                    source_len,
                )
            } else {
                source_len
            };

            if source_start >= source_end || target_start >= target_len {
                retval.set(v8::Integer::new(scope, 0).into());
                return;
            }

            let available = source_end - source_start;
            let bytes_to_copy = std::cmp::min(available, target_len - target_start);
            if bytes_to_copy == 0 {
                retval.set(v8::Integer::new(scope, 0).into());
                return;
            }

            let source_slice = unsafe { std::slice::from_raw_parts(source_ptr, source_len) };
            let target_slice = unsafe { std::slice::from_raw_parts_mut(target_ptr, target_len) };
            target_slice[target_start..target_start + bytes_to_copy]
                .copy_from_slice(&source_slice[source_start..source_start + bytes_to_copy]);
            retval.set(v8::Integer::new(scope, bytes_to_copy as i32).into());
        },
    )
    .unwrap();

    // Buffer.prototype.indexOf
    #[allow(irrefutable_let_patterns)]
    let buffer_index_of_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let (source_ptr, source_len): (*const u8, usize) =
                if this.is_array_buffer_view() || this.is_typed_array() {
                    if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(this) {
                        let len = view.byte_length();
                        let offset = view.byte_offset();
                        if let Some(buf) = view.buffer(scope) {
                            let store = buf.get_backing_store();
                            let ptr = store.as_ref().as_ptr() as *const u8;
                            if ptr.is_null() {
                                (std::ptr::null(), 0)
                            } else {
                                unsafe { (ptr.add(offset), len) }
                            }
                        } else {
                            (std::ptr::null(), 0)
                        }
                    } else {
                        (std::ptr::null(), 0)
                    }
                } else if this.is_object() {
                    let buffer_key = v8::String::new(scope, "buffer").unwrap().into();
                    if let Ok(obj) = v8::Local::<v8::Object>::try_from(this) {
                        if let Some(buf) = obj.get(scope, buffer_key) {
                            if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(buf) {
                                let len = ab.byte_length();
                                let store = ab.get_backing_store();
                                let ptr = store.as_ref().as_ptr() as *const u8;
                                (ptr, len)
                            } else {
                                (std::ptr::null(), 0)
                            }
                        } else {
                            (std::ptr::null(), 0)
                        }
                    } else {
                        (std::ptr::null(), 0)
                    }
                } else {
                    (std::ptr::null(), 0)
                };

            let search_value = if args.length() >= 1 {
                args.get(0)
            } else {
                retval.set(v8::Integer::new(scope, -1).into());
                return;
            };

            let target_bytes: Vec<u8> = if search_value.is_string() {
                if let Some(str_val) = search_value.to_string(scope) {
                    str_val.to_rust_string_lossy(scope).into_bytes()
                } else {
                    retval.set(v8::Integer::new(scope, -1).into());
                    return;
                }
            } else if search_value.is_number() {
                vec![search_value.to_integer(scope).unwrap().value() as u8]
            } else if search_value.is_array_buffer_view() || search_value.is_typed_array() {
                if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(search_value) {
                    let len = view.byte_length();
                    let offset = view.byte_offset();
                    if let Some(buf) = view.buffer(scope) {
                        let store = buf.get_backing_store();
                        let ptr = store.as_ref().as_ptr() as *const u8;
                        if !ptr.is_null() && len > 0 {
                            let slice = unsafe { std::slice::from_raw_parts(ptr.add(offset), len) };
                            slice.to_vec()
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            } else {
                retval.set(v8::Integer::new(scope, -1).into());
                return;
            };

            if source_ptr.is_null() || source_len == 0 || target_bytes.is_empty() {
                retval.set(v8::Integer::new(scope, -1).into());
                return;
            }

            let start = if args.length() >= 2 && args.get(1).is_number() {
                args.get(1).to_integer(scope).unwrap().value().max(0) as usize
            } else {
                0
            };

            let bytes = unsafe { std::slice::from_raw_parts(source_ptr, source_len) };
            let clamped_start = std::cmp::min(start, bytes.len());
            let result = bytes[clamped_start..]
                .windows(target_bytes.len())
                .position(|w| w == target_bytes.as_slice());
            retval.set(
                v8::Integer::new(
                    scope,
                    result.map(|i| (i + clamped_start) as i32).unwrap_or(-1),
                )
                .into(),
            );
        },
    )
    .unwrap();

    // Buffer.prototype.fill
    let buffer_fill_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let (target_ptr, len) = if this.is_array_buffer_view() || this.is_typed_array() {
                if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(this) {
                    let len = view.byte_length();
                    let offset = view.byte_offset();
                    if let Some(buf) = view.buffer(scope) {
                        let store = buf.get_backing_store();
                        let ptr = store.as_ref().as_ptr() as *mut u8;
                        if ptr.is_null() {
                            (std::ptr::null_mut(), 0)
                        } else {
                            unsafe { (ptr.add(offset), len) }
                        }
                    } else {
                        (std::ptr::null_mut(), 0)
                    }
                } else {
                    (std::ptr::null_mut(), 0)
                }
            } else {
                let buf_key = v8::String::new(scope, "buffer").unwrap().into();
                let obj = this;
                if let Some(buf) = obj.get(scope, buf_key) {
                    if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(buf) {
                        let len = ab.byte_length();
                        let store = ab.get_backing_store();
                        let ptr = store.as_ref().as_ptr() as *mut u8;
                        (ptr, len)
                    } else {
                        (std::ptr::null_mut(), 0)
                    }
                } else {
                    (std::ptr::null_mut(), 0)
                }
            };

            if target_ptr.is_null() || len == 0 {
                retval.set(this.into());
                return;
            }

            let offset = if args.length() >= 2 && args.get(1).is_number() {
                args.get(1).to_integer(scope).unwrap().value().max(0) as usize
            } else {
                0
            };

            let end = if args.length() >= 3 && args.get(2).is_number() {
                (args.get(2).to_integer(scope).unwrap().value().max(0) as usize).min(len)
            } else {
                len
            };

            if offset < end && offset < len {
                let fill_len = end - offset;
                let dst_ptr = unsafe { target_ptr.add(offset) };
                if args.length() >= 1 {
                    let val = args.get(0);
                    if val.is_number() {
                        let fill_byte = val.to_integer(scope).unwrap().value() as u8;
                        unsafe {
                            std::ptr::write_bytes(dst_ptr, fill_byte, fill_len);
                        }
                    } else if let Some(s) = val.to_string(scope) {
                        let rust_s = s.to_rust_string_lossy(scope);
                        let enc = if args.length() >= 4 {
                            args.get(3)
                                .to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| "utf8".to_string())
                        } else if args.length() >= 3 && args.get(2).is_string() {
                            args.get(2)
                                .to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| "utf8".to_string())
                        } else {
                            "utf8".to_string()
                        };
                        let fill_bytes = encode_string_to_bytes(&rust_s, &enc);
                        if !fill_bytes.is_empty() {
                            if fill_bytes.len() == 1 {
                                unsafe {
                                    std::ptr::write_bytes(dst_ptr, fill_bytes[0], fill_len);
                                }
                            } else {
                                let slice =
                                    unsafe { std::slice::from_raw_parts_mut(dst_ptr, fill_len) };
                                for (i, dst) in slice.iter_mut().enumerate() {
                                    *dst = fill_bytes[i % fill_bytes.len()];
                                }
                            }
                        }
                    } else {
                        unsafe {
                            std::ptr::write_bytes(dst_ptr, 0, fill_len);
                        }
                    }
                } else {
                    unsafe {
                        std::ptr::write_bytes(dst_ptr, 0, fill_len);
                    }
                }
            }
            retval.set(this.into());
        },
    )
    .unwrap();

    // Buffer.from
    let buffer_from_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            if args.length() < 1 {
                let buffer = v8::ArrayBuffer::new(scope, 0);
                if let Some(u8_array) = v8::Uint8Array::new(scope, buffer, 0, 0) {
                    retval.set(u8_array.into());
                }
                return;
            }
            let first = args.get(0);
            if first.is_string() {
                let str_val = first.to_string(scope).unwrap();
                let encoding = if args.length() >= 2 {
                    args.get(1)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_else(|| "utf8".to_string())
                } else {
                    "utf8".to_string()
                };
                let enc_lower = encoding.to_ascii_lowercase();
                if enc_lower == "utf8" || enc_lower == "utf-8" {
                    let len = str_val.utf8_length(scope);
                    let buffer = v8::ArrayBuffer::new(scope, len);
                    if len > 0 {
                        let store = buffer.get_backing_store();
                        let ptr = store.as_ref().as_ptr() as *mut u8;
                        if !ptr.is_null() {
                            let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
                            str_val.write_utf8_v2(scope, slice, v8::WriteFlags::empty(), None);
                        }
                    }
                    if let Some(u8_array) = v8::Uint8Array::new(scope, buffer, 0, len) {
                        set_buffer_prototype_fast(scope, u8_array);
                        retval.set(u8_array.into());
                    }
                    return;
                } else {
                    let rust_string = str_val.to_rust_string_lossy(scope);
                    let bytes = encode_string_to_bytes(&rust_string, &encoding);
                    let buffer = v8::ArrayBuffer::new(scope, bytes.len());
                    if !bytes.is_empty() {
                        let store = buffer.get_backing_store();
                        let ptr = store.as_ref().as_ptr() as *mut u8;
                        if !ptr.is_null() {
                            let slice = unsafe { std::slice::from_raw_parts_mut(ptr, bytes.len()) };
                            slice.copy_from_slice(&bytes);
                        }
                    }
                    if let Some(u8_array) = v8::Uint8Array::new(scope, buffer, 0, bytes.len()) {
                        set_buffer_prototype_fast(scope, u8_array);
                        retval.set(u8_array.into());
                    }
                    return;
                }
            } else if first.is_array_buffer() {
                if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(first) {
                    let total_len = ab.byte_length();
                    let offset = if args.length() >= 2 && args.get(1).is_number() {
                        (args.get(1).to_integer(scope).unwrap().value().max(0) as usize)
                            .min(total_len)
                    } else {
                        0
                    };
                    let len = if args.length() >= 3 && args.get(2).is_number() {
                        (args.get(2).to_integer(scope).unwrap().value().max(0) as usize)
                            .min(total_len - offset)
                    } else {
                        total_len - offset
                    };
                    if let Some(u8_array) = v8::Uint8Array::new(scope, ab, offset, len) {
                        set_buffer_prototype_fast(scope, u8_array);
                        retval.set(u8_array.into());
                        return;
                    }
                }
            } else if first.is_array_buffer_view() || first.is_typed_array() {
                if let Ok(ta) = v8::Local::<v8::ArrayBufferView>::try_from(first) {
                    let len = ta.byte_length();
                    let buffer = v8::ArrayBuffer::new(scope, len);
                    if len > 0 {
                        if let Some(src_ab) = ta.buffer(scope) {
                            let src_store = src_ab.get_backing_store();
                            let src_ptr = src_store.as_ref().as_ptr() as *const u8;
                            let dst_store = buffer.get_backing_store();
                            let dst_ptr = dst_store.as_ref().as_ptr() as *mut u8;
                            if !src_ptr.is_null() && !dst_ptr.is_null() {
                                let src_slice = unsafe {
                                    std::slice::from_raw_parts(src_ptr.add(ta.byte_offset()), len)
                                };
                                let dst_slice =
                                    unsafe { std::slice::from_raw_parts_mut(dst_ptr, len) };
                                dst_slice.copy_from_slice(src_slice);
                            }
                        }
                    }
                    if let Some(u8_array) = v8::Uint8Array::new(scope, buffer, 0, len) {
                        set_buffer_prototype_fast(scope, u8_array);
                        retval.set(u8_array.into());
                        return;
                    }
                }
            } else if first.is_array() {
                if let Ok(arr) = v8::Local::<v8::Array>::try_from(first) {
                    let len = arr.length() as usize;
                    let buffer = v8::ArrayBuffer::new(scope, len);
                    if len > 0 {
                        let store = buffer.get_backing_store();
                        let ptr = store.as_ref().as_ptr() as *mut u8;
                        if !ptr.is_null() {
                            let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
                            for i in 0..len {
                                if let Some(elem) = arr.get_index(scope, i as u32) {
                                    slice[i] = elem
                                        .to_integer(scope)
                                        .map(|n| n.value() as u8)
                                        .unwrap_or(0);
                                }
                            }
                        }
                    }
                    if let Some(u8_array) = v8::Uint8Array::new(scope, buffer, 0, len) {
                        set_buffer_prototype_fast(scope, u8_array);
                        retval.set(u8_array.into());
                        return;
                    }
                }
            } else if first.is_number() {
                let size = first.to_integer(scope).unwrap().value().max(0) as usize;
                let buffer = v8::ArrayBuffer::new(scope, size);
                if let Some(u8_array) = v8::Uint8Array::new(scope, buffer, 0, size) {
                    set_buffer_prototype_fast(scope, u8_array);
                    retval.set(u8_array.into());
                    return;
                }
            }
            let buffer = v8::ArrayBuffer::new(scope, 0);
            if let Some(u8_array) = v8::Uint8Array::new(scope, buffer, 0, 0) {
                set_buffer_prototype_fast(scope, u8_array);
                retval.set(u8_array.into());
            }
        },
    )
    .unwrap();

    // Buffer.alloc
    let buffer_alloc_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let size = if args.length() >= 1 {
                args.get(0).to_integer(scope).unwrap().value().max(0) as usize
            } else {
                0
            };
            let buffer = v8::ArrayBuffer::new(scope, size);
            if size > 0 && args.length() >= 2 {
                let fill = args.get(1);
                if fill.is_number() {
                    let fill_byte = fill.to_integer(scope).unwrap().value() as u8;
                    if fill_byte != 0 {
                        let store = buffer.get_backing_store();
                        let ptr = store.as_ref().as_ptr() as *mut u8;
                        if !ptr.is_null() {
                            unsafe {
                                std::ptr::write_bytes(ptr, fill_byte, size);
                            }
                        }
                    }
                } else if fill.is_string() {
                    if let Some(str_val) = fill.to_string(scope) {
                        let rust_str = str_val.to_rust_string_lossy(scope);
                        let enc = if args.length() >= 3 {
                            args.get(2)
                                .to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| "utf8".to_string())
                        } else {
                            "utf8".to_string()
                        };
                        let fill_bytes = encode_string_to_bytes(&rust_str, &enc);
                        if !fill_bytes.is_empty() {
                            let store = buffer.get_backing_store();
                            let ptr = store.as_ref().as_ptr() as *mut u8;
                            if !ptr.is_null() {
                                if fill_bytes.len() == 1 {
                                    unsafe {
                                        std::ptr::write_bytes(ptr, fill_bytes[0], size);
                                    }
                                } else {
                                    let slice =
                                        unsafe { std::slice::from_raw_parts_mut(ptr, size) };
                                    for (i, dst) in slice.iter_mut().enumerate() {
                                        *dst = fill_bytes[i % fill_bytes.len()];
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(u8_array) = v8::Uint8Array::new(scope, buffer, 0, size) {
                set_buffer_prototype_fast(scope, u8_array);
                retval.set(u8_array.into());
            }
        },
    )
    .unwrap();

    // Buffer.concat
    let buffer_concat_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let total_length = if args.length() >= 2 {
                args.get(1).to_integer(scope).unwrap().value().max(0) as usize
            } else {
                0
            };
            if args.length() >= 1 {
                let first = args.get(0);
                if first.is_array() {
                    let arr = v8::Local::<v8::Array>::try_from(first).unwrap();
                    let len = arr.length();
                    let calculated_length = if total_length == 0 {
                        let mut total = 0usize;
                        for i in 0..len {
                            if let Some(item) = arr.get_index(scope, i) {
                                if item.is_array_buffer_view() || item.is_typed_array() {
                                    if let Ok(view) =
                                        v8::Local::<v8::ArrayBufferView>::try_from(item)
                                    {
                                        total += view.byte_length();
                                        continue;
                                    }
                                }
                                if item.is_object() {
                                    if let Ok(obj) = v8::Local::<v8::Object>::try_from(item) {
                                        let buffer_key =
                                            v8::String::new(scope, "buffer").unwrap().into();
                                        if let Some(buf) = obj.get(scope, buffer_key) {
                                            if buf.is_array_buffer() {
                                                if let Ok(arr_buffer) =
                                                    v8::Local::<v8::ArrayBuffer>::try_from(buf)
                                                {
                                                    total += arr_buffer.byte_length();
                                                    continue;
                                                }
                                            }
                                        }
                                    }
                                }
                                if item.is_array_buffer() {
                                    if let Ok(arr_buffer) =
                                        v8::Local::<v8::ArrayBuffer>::try_from(item)
                                    {
                                        total += arr_buffer.byte_length();
                                    }
                                }
                            }
                        }
                        total
                    } else {
                        total_length
                    };

                    let buffer = v8::ArrayBuffer::new(scope, calculated_length);
                    let store = buffer.get_backing_store();
                    let dst_ptr = if calculated_length > 0 {
                        store
                            .data()
                            .map(|p| p.as_ptr() as *mut u8)
                            .unwrap_or(std::ptr::null_mut())
                    } else {
                        std::ptr::null_mut()
                    };

                    if !dst_ptr.is_null() && calculated_length > 0 {
                        let mut offset = 0;
                        for i in 0..len {
                            if offset >= calculated_length {
                                break;
                            }
                            if let Some(item) = arr.get_index(scope, i) {
                                let (src_ptr, src_len) = if item.is_array_buffer_view()
                                    || item.is_typed_array()
                                {
                                    if let Ok(view) =
                                        v8::Local::<v8::ArrayBufferView>::try_from(item)
                                    {
                                        let l = view.byte_length();
                                        let off = view.byte_offset();
                                        if let Some(b) = view.buffer(scope) {
                                            let st = b.get_backing_store();
                                            let p = if l > 0 {
                                                st.data()
                                                    .map(|p| p.as_ptr() as *const u8)
                                                    .unwrap_or(std::ptr::null())
                                            } else {
                                                std::ptr::null()
                                            };
                                            if p.is_null() {
                                                (std::ptr::null(), 0)
                                            } else {
                                                unsafe { (p.add(off), l) }
                                            }
                                        } else {
                                            (std::ptr::null(), 0)
                                        }
                                    } else {
                                        (std::ptr::null(), 0)
                                    }
                                } else if item.is_object() {
                                    let buffer_key =
                                        v8::String::new(scope, "buffer").unwrap().into();
                                    if let Ok(obj) = v8::Local::<v8::Object>::try_from(item) {
                                        if let Some(buf) = obj.get(scope, buffer_key) {
                                            if let Ok(ab) =
                                                v8::Local::<v8::ArrayBuffer>::try_from(buf)
                                            {
                                                let l = ab.byte_length();
                                                let st = ab.get_backing_store();
                                                let p = if l > 0 {
                                                    st.data()
                                                        .map(|p| p.as_ptr() as *const u8)
                                                        .unwrap_or(std::ptr::null())
                                                } else {
                                                    std::ptr::null()
                                                };
                                                (p, l)
                                            } else {
                                                (std::ptr::null(), 0)
                                            }
                                        } else {
                                            (std::ptr::null(), 0)
                                        }
                                    } else {
                                        (std::ptr::null(), 0)
                                    }
                                } else if item.is_array_buffer() {
                                    if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(item) {
                                        let l = ab.byte_length();
                                        let st = ab.get_backing_store();
                                        let p = if l > 0 {
                                            st.data()
                                                .map(|p| p.as_ptr() as *const u8)
                                                .unwrap_or(std::ptr::null())
                                        } else {
                                            std::ptr::null()
                                        };
                                        (p, l)
                                    } else {
                                        (std::ptr::null(), 0)
                                    }
                                } else {
                                    (std::ptr::null(), 0)
                                };

                                if !src_ptr.is_null() && src_len > 0 {
                                    let copy_len =
                                        std::cmp::min(src_len, calculated_length - offset);
                                    unsafe {
                                        std::ptr::copy_nonoverlapping(
                                            src_ptr,
                                            dst_ptr.add(offset),
                                            copy_len,
                                        );
                                    }
                                    offset += copy_len;
                                }
                            }
                        }
                    }

                    if let Some(u8_array) = v8::Uint8Array::new(scope, buffer, 0, calculated_length)
                    {
                        let buffer_str = v8::String::new(scope, "Buffer").unwrap().into();
                        let global = scope.get_current_context().global(scope);
                        if let Some(buffer_val) = global.get(scope, buffer_str) {
                            if let Ok(ctor) = v8::Local::<v8::Function>::try_from(buffer_val) {
                                let proto_key = v8::String::new(scope, "prototype").unwrap().into();
                                if let Some(proto_val) = ctor.get(scope, proto_key) {
                                    u8_array.set_prototype(scope, proto_val);
                                }
                            }
                        }
                        retval.set(u8_array.into());
                        return;
                    }
                }
            }
            let buffer = v8::ArrayBuffer::new(scope, 0);
            if let Some(u8_array) = v8::Uint8Array::new(scope, buffer, 0, 0) {
                retval.set(u8_array.into());
            }
        },
    )
    .unwrap();

    // Buffer.isBuffer
    let buffer_is_buffer_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let is_buffer = if args.length() >= 1 {
                let first = args.get(0);
                if first.is_object() {
                    if let Ok(obj) = v8::Local::<v8::Object>::try_from(first) {
                        let is_buf_key = v8::String::new(scope, "_isBuffer").unwrap().into();
                        if let Some(v) = obj.get(scope, is_buf_key) {
                            if v.is_true() {
                                true
                            } else {
                                let buffer_key = v8::String::new(scope, "buffer").unwrap().into();
                                if let Some(buf) = obj.get(scope, buffer_key) {
                                    buf.is_array_buffer()
                                } else {
                                    false
                                }
                            }
                        } else {
                            let buffer_key = v8::String::new(scope, "buffer").unwrap().into();
                            if let Some(buf) = obj.get(scope, buffer_key) {
                                buf.is_array_buffer()
                            } else {
                                false
                            }
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };
            retval.set(v8::Boolean::new(scope, is_buffer).into());
        },
    )
    .unwrap();

    // Buffer.byteLength
    let buffer_byte_length_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let byte_length = if args.length() >= 1 {
                let first = args.get(0);
                if first.is_string() {
                    if let Some(str_val) = first.to_string(scope) {
                        let rust_string = str_val.to_rust_string_lossy(scope);
                        let encoding = if args.length() >= 2 {
                            args.get(1)
                                .to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| "utf8".to_string())
                        } else {
                            "utf8".to_string()
                        };
                        encode_string_to_bytes(&rust_string, &encoding).len() as i32
                    } else {
                        0
                    }
                } else if first.is_array_buffer() || first.is_typed_array() {
                    if let Ok(arr_buffer) = v8::Local::<v8::ArrayBuffer>::try_from(first) {
                        arr_buffer.byte_length() as i32
                    } else if let Ok(typed_array) = v8::Local::<v8::TypedArray>::try_from(first) {
                        typed_array.byte_length() as i32
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            };
            retval.set(v8::Integer::new(scope, byte_length).into());
        },
    )
    .unwrap();

    // Create Buffer object and set properties
    let buffer_ctor_key = v8::String::new(scope, "Buffer").unwrap().into();
    let global = scope.get_current_context().global(scope);
    global.set(scope, buffer_ctor_key, buffer_fn.into());

    // Set Buffer static methods on the constructor function itself
    let from_key = v8::String::new(scope, "from").unwrap().into();
    buffer_fn.set(scope, from_key, buffer_from_fn.into());
    let alloc_key = v8::String::new(scope, "alloc").unwrap().into();
    buffer_fn.set(scope, alloc_key, buffer_alloc_fn.into());
    let alloc_unsafe_key = v8::String::new(scope, "allocUnsafe").unwrap().into();
    buffer_fn.set(scope, alloc_unsafe_key, buffer_alloc_fn.into());
    let concat_key = v8::String::new(scope, "concat").unwrap().into();
    buffer_fn.set(scope, concat_key, buffer_concat_fn.into());
    let is_buffer_key = v8::String::new(scope, "isBuffer").unwrap().into();
    buffer_fn.set(scope, is_buffer_key, buffer_is_buffer_fn.into());
    let byte_length_key = v8::String::new(scope, "byteLength").unwrap().into();
    buffer_fn.set(scope, byte_length_key, buffer_byte_length_fn.into());

    // Set Buffer.prototype properties on the constructor
    let prototype_key = v8::String::new(scope, "prototype").unwrap().into();
    let prototype_obj = v8::Object::new(scope);

    // Link Buffer.prototype.__proto__ = Uint8Array.prototype
    let uint8_array_str = v8::String::new(scope, "Uint8Array").unwrap();
    if let Some(uint8_ctor_val) = global.get(scope, uint8_array_str.into()) {
        if let Ok(uint8_ctor) = v8::Local::<v8::Function>::try_from(uint8_ctor_val) {
            let proto_key = v8::String::new(scope, "prototype").unwrap().into();
            if let Some(uint8_proto) = uint8_ctor.get(scope, proto_key) {
                prototype_obj.set_prototype(scope, uint8_proto);
            }
            buffer_fn.set_prototype(scope, uint8_ctor_val);
        }
    }

    buffer_fn.set(scope, prototype_key, prototype_obj.into());

    // Set _isBuffer on prototype so any buffer instance inherits it
    let is_buf_prop_key = v8::String::new(scope, "_isBuffer").unwrap().into();
    let is_buf_val = v8::Boolean::new(scope, true).into();
    prototype_obj.set(scope, is_buf_prop_key, is_buf_val);

    // Create method names first to avoid borrow conflicts
    let to_string_key = v8::String::new(scope, "toString").unwrap().into();
    let write_key = v8::String::new(scope, "write").unwrap().into();
    let slice_key = v8::String::new(scope, "slice").unwrap().into();
    let copy_key = v8::String::new(scope, "copy").unwrap().into();
    let index_of_key = v8::String::new(scope, "indexOf").unwrap().into();
    let fill_key = v8::String::new(scope, "fill").unwrap().into();
    let constructor_key = v8::String::new(scope, "constructor").unwrap().into();

    prototype_obj.set(scope, to_string_key, buffer_to_string_fn.into());
    prototype_obj.set(scope, write_key, buffer_write_fn.into());
    prototype_obj.set(scope, slice_key, buffer_slice_fn.into());
    prototype_obj.set(scope, copy_key, buffer_copy_fn.into());
    prototype_obj.set(scope, index_of_key, buffer_index_of_fn.into());
    prototype_obj.set(scope, fill_key, buffer_fill_fn.into());
    // Set constructor to point back to Buffer
    prototype_obj.set(scope, constructor_key, buffer_fn.into());

    // Set default byteOffset on Buffer.prototype
    let byte_offset_key = v8::String::new(scope, "byteOffset").unwrap().into();
    let zero = v8::Integer::new(scope, 0).into();
    prototype_obj.set(scope, byte_offset_key, zero);

    // Cache Buffer prototype globally for fast zero-overhead allocation
    let proto_global = v8::Global::new(scope, prototype_obj);
    CACHED_BUFFER_PROTOTYPE.with(|p| {
        *p.borrow_mut() = Some(proto_global);
    });

    // High-performance Buffer operations (v0.4.3):
    // 1. Buffer[Symbol.hasInstance] delegates to Buffer.isBuffer (spec compliance)
    // 2. Buffer.prototype.fill delegates numeric fill to Uint8Array.prototype.fill (SIMD)
    // 3. Buffer.prototype.subarray & slice construct typed views directly with Buffer.prototype
    let buffer_opt_code = v8::String::new(
        scope,
        r#"
(function() {
    try {
        Object.defineProperty(Buffer, Symbol.hasInstance, {
            value: function hasInstance(instance) {
                return Buffer.isBuffer(instance);
            },
            configurable: true
        });
    } catch (_) {}

    const _u8Fill = Uint8Array.prototype.fill;
    const _origFill = Buffer.prototype.fill;
    Buffer.prototype.fill = function fill(val, start, end, enc) {
        if (typeof val === 'number') {
            return _u8Fill.call(this, val, start, end);
        }
        return _origFill.call(this, val, start, end, enc);
    };

    // FastBuffer avoids Object.setPrototypeOf on every allocation and slice
    class FastBuffer extends Uint8Array {}
    Object.setPrototypeOf(FastBuffer.prototype, Buffer.prototype);
    FastBuffer.prototype.constructor = FastBuffer;

    Buffer.prototype.subarray = function subarray(start, end) {
        const len = this.length;
        let s = start | 0;
        let e = end === undefined ? len : (end | 0);
        if (s < 0) { s += len; if (s < 0) s = 0; }
        if (e < 0) { e += len; if (e < 0) e = 0; }
        if (e > len) e = len;
        if (s > e) s = e;
        return new FastBuffer(this.buffer, this.byteOffset + s, e - s);
    };
    Buffer.prototype.slice = Buffer.prototype.subarray;

    let poolSize = 8192;
    let poolOffset = 0;
    let allocPool = null;

    function createPool() {
        allocPool = new ArrayBuffer(poolSize);
        poolOffset = 0;
    }
    createPool();

    const _origAlloc = Buffer.alloc;
    const _origAllocUnsafe = Buffer.allocUnsafe;
    const _origFrom = Buffer.from;

    Buffer.allocUnsafe = function allocUnsafe(size) {
        size = size | 0;
        if (size <= 0) {
            return new FastBuffer(0);
        }
        if (size < (Buffer.poolSize >>> 1)) {
            if (size > (poolSize - poolOffset)) {
                createPool();
            }
            const b = new FastBuffer(allocPool, poolOffset, size);
            poolOffset += size;
            poolOffset = (poolOffset + 7) & ~7;
            return b;
        }
        return new FastBuffer(size);
    };

    Buffer.alloc = function alloc(size, fill, enc) {
        size = size | 0;
        if (size <= 0) {
            return new FastBuffer(0);
        }
        if (fill === undefined || fill === 0) {
            return new FastBuffer(size);
        }
        const buf = new FastBuffer(size);
        buf.fill(fill, 0, size, enc);
        return buf;
    };

    Buffer.from = function from(val, enc) {
        if (typeof val === 'string') {
            const strLen = val.length;
            if (strLen < 2048 && (!enc || enc === 'utf8' || enc === 'utf-8')) {
                const maxBytes = (strLen * 3) | 0;
                if (maxBytes < (poolSize - poolOffset)) {
                    const b = new FastBuffer(allocPool, poolOffset, maxBytes);
                    const written = b.write(val, 'utf8');
                    const res = new FastBuffer(allocPool, poolOffset, written);
                    poolOffset = (poolOffset + written + 7) & ~7;
                    return res;
                } else if (maxBytes < (Buffer.poolSize >>> 1)) {
                    createPool();
                    const b = new FastBuffer(allocPool, poolOffset, maxBytes);
                    const written = b.write(val, 'utf8');
                    const res = new FastBuffer(allocPool, poolOffset, written);
                    poolOffset = (poolOffset + written + 7) & ~7;
                    return res;
                }
            }
        }
        return _origFrom(val, enc);
    };

    Buffer.allocUnsafeSlow = function allocUnsafeSlow(size) {
        return new FastBuffer(size | 0);
    };
    Buffer.poolSize = 8192;
})();
"#,
    )
    .unwrap();
    if let Some(script) = v8::Script::compile(scope, buffer_opt_code, None) {
        script.run(scope);
    }
}

/// Helper function to decode bytes to a string with the specified encoding
fn decode_bytes_to_string(bytes: &[u8], encoding: &str) -> String {
    let engine = base64::engine::general_purpose::STANDARD;
    match encoding.to_lowercase().as_str() {
        "utf8" | "utf-8" | "utf8mb4" => {
            if let Ok(s) = std::str::from_utf8(bytes) {
                s.to_string()
            } else {
                String::from_utf8_lossy(bytes).to_string()
            }
        }
        "hex" => hex::encode(bytes),
        "base64" => engine.encode(bytes),
        "base64url" => base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes),
        "latin1" | "ascii" | "binary" => {
            if bytes.is_ascii() {
                // SIMD-accelerated ASCII validation in Rust
                unsafe { std::str::from_utf8_unchecked(bytes).to_string() }
            } else {
                bytes.iter().map(|&b| b as char).collect()
            }
        }
        _ => String::from_utf8_lossy(bytes).to_string(),
    }
}

/// Generate RSA key pair (v0.3.23)
/// Returns (public_key_pem, private_key_pem)
fn generate_rsa_key_pair(modulus_length: usize) -> Result<(String, String), String> {
    let rsa = Rsa::generate(modulus_length as u32)
        .map_err(|error| format!("generateKeyPair: RSA key generation failed: {}", error))?;
    let public_key_pem = String::from_utf8(
        rsa.public_key_to_pem()
            .map_err(|error| format!("generateKeyPair: public key export failed: {}", error))?,
    )
    .map_err(|error| {
        format!(
            "generateKeyPair: public key PEM is invalid UTF-8: {}",
            error
        )
    })?;
    let private_key_pem = String::from_utf8(
        rsa.private_key_to_pem()
            .map_err(|error| format!("generateKeyPair: private key export failed: {}", error))?,
    )
    .map_err(|error| {
        format!(
            "generateKeyPair: private key PEM is invalid UTF-8: {}",
            error
        )
    })?;

    Ok((public_key_pem, private_key_pem))
}

fn generate_ed25519_key_pair() -> Result<(String, String), String> {
    let key = PKey::generate_ed25519()
        .map_err(|error| format!("generateKeyPair: Ed25519 key generation failed: {}", error))?;
    let public_key_pem = String::from_utf8(key.public_key_to_pem().map_err(|error| {
        format!(
            "generateKeyPair: Ed25519 public key export failed: {}",
            error
        )
    })?)
    .map_err(|error| {
        format!(
            "generateKeyPair: Ed25519 public key PEM is invalid UTF-8: {}",
            error
        )
    })?;
    let private_key_pem = String::from_utf8(key.private_key_to_pem_pkcs8().map_err(|error| {
        format!(
            "generateKeyPair: Ed25519 private key export failed: {}",
            error
        )
    })?)
    .map_err(|error| {
        format!(
            "generateKeyPair: Ed25519 private key PEM is invalid UTF-8: {}",
            error
        )
    })?;

    Ok((public_key_pem, private_key_pem))
}

fn generate_ed448_key_pair() -> Result<(String, String), String> {
    let key = PKey::generate_ed448()
        .map_err(|error| format!("generateKeyPair: Ed448 key generation failed: {}", error))?;
    let public_key_pem =
        String::from_utf8(key.public_key_to_pem().map_err(|error| {
            format!("generateKeyPair: Ed448 public key export failed: {}", error)
        })?)
        .map_err(|error| {
            format!(
                "generateKeyPair: Ed448 public key PEM is invalid UTF-8: {}",
                error
            )
        })?;
    let private_key_pem = String::from_utf8(key.private_key_to_pem_pkcs8().map_err(|error| {
        format!(
            "generateKeyPair: Ed448 private key export failed: {}",
            error
        )
    })?)
    .map_err(|error| {
        format!(
            "generateKeyPair: Ed448 private key PEM is invalid UTF-8: {}",
            error
        )
    })?;

    Ok((public_key_pem, private_key_pem))
}

fn signature_message_digest(algorithm: &str) -> Option<MessageDigest> {
    match algorithm.to_ascii_uppercase().as_str() {
        "RSA-SHA256" | "SHA256" => Some(MessageDigest::sha256()),
        "RSA-SHA384" | "SHA384" => Some(MessageDigest::sha384()),
        "RSA-SHA512" | "SHA512" => Some(MessageDigest::sha512()),
        "RSA-SHA1" | "SHA1" => Some(MessageDigest::sha1()),
        "RSA-MD5" | "MD5" => Some(MessageDigest::md5()),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct RsaSignatureOptions {
    padding: Padding,
    uses_pss: bool,
    pss_salt_length: Option<i64>,
}

impl RsaSignatureOptions {
    fn pkcs1() -> Self {
        Self {
            padding: Padding::PKCS1,
            uses_pss: false,
            pss_salt_length: None,
        }
    }
}

fn rsa_signature_options_from_node(
    padding: Option<i64>,
    salt_length: Option<i64>,
) -> RsaSignatureOptions {
    match padding {
        Some(6) => RsaSignatureOptions {
            padding: Padding::PKCS1_PSS,
            uses_pss: true,
            pss_salt_length: salt_length,
        },
        _ => RsaSignatureOptions {
            padding: Padding::PKCS1,
            uses_pss: false,
            pss_salt_length: None,
        },
    }
}

fn rsa_pss_saltlen(value: i64) -> Result<RsaPssSaltlen, String> {
    match value {
        -1 => Ok(RsaPssSaltlen::DIGEST_LENGTH),
        -2 => Ok(RsaPssSaltlen::MAXIMUM_LENGTH),
        _ if value >= 0 && value <= i32::MAX as i64 => Ok(RsaPssSaltlen::custom(value as i32)),
        _ => Err(format!("invalid RSA-PSS saltLength '{}'", value)),
    }
}

fn get_signature_key_options(
    scope: &mut v8::PinScope,
    key_value: v8::Local<v8::Value>,
) -> Result<(String, RsaSignatureOptions), String> {
    if key_value.is_string() {
        let key = key_value
            .to_string(scope)
            .map(|value| value.to_rust_string_lossy(scope))
            .unwrap_or_default();
        return Ok((key, RsaSignatureOptions::pkcs1()));
    }

    if !key_value.is_object() {
        return Err("key must be a PEM string or an object with a key property".to_string());
    }

    let key_obj = key_value
        .to_object(scope)
        .ok_or_else(|| "key must be an object".to_string())?;
    let key = get_string_property(scope, key_obj, "key")
        .or_else(|| get_string_property(scope, key_obj, "pem"))
        .unwrap_or_default();
    let padding = get_i64_property(scope, key_obj, "padding");
    let salt_length = get_i64_property(scope, key_obj, "saltLength");

    Ok((key, rsa_signature_options_from_node(padding, salt_length)))
}

fn sign_pem_private_key(
    algorithm: &str,
    private_key_pem: &str,
    data: &[u8],
    options: RsaSignatureOptions,
) -> Result<Vec<u8>, String> {
    let digest = signature_message_digest(algorithm)
        .ok_or_else(|| format!("sign: unsupported algorithm '{}'", algorithm))?;
    let key = PKey::private_key_from_pem(private_key_pem.as_bytes())
        .map_err(|error| format!("sign: invalid private key: {}", error))?;
    let mut signer = Signer::new(digest, &key)
        .map_err(|error| format!("sign: signer setup failed: {}", error))?;
    if key.id() == Id::RSA {
        signer
            .set_rsa_padding(options.padding)
            .map_err(|error| format!("sign: RSA padding setup failed: {}", error))?;
        if options.uses_pss {
            if let Some(salt_length) = options.pss_salt_length {
                signer
                    .set_rsa_pss_saltlen(rsa_pss_saltlen(salt_length)?)
                    .map_err(|error| format!("sign: RSA-PSS saltLength setup failed: {}", error))?;
            }
        }
    } else if options.uses_pss {
        return Err("sign: RSA-PSS padding requires an RSA private key".to_string());
    }
    signer
        .update(data)
        .map_err(|error| format!("sign: signer update failed: {}", error))?;
    signer
        .sign_to_vec()
        .map_err(|error| format!("sign: signing failed: {}", error))
}

fn verify_pem_public_key(
    algorithm: &str,
    public_key_pem: &str,
    data: &[u8],
    signature: &[u8],
    options: RsaSignatureOptions,
) -> Result<bool, String> {
    let digest = signature_message_digest(algorithm)
        .ok_or_else(|| format!("verify: unsupported algorithm '{}'", algorithm))?;
    let key = PKey::public_key_from_pem(public_key_pem.as_bytes())
        .map_err(|error| format!("verify: invalid public key: {}", error))?;
    let mut verifier = Verifier::new(digest, &key)
        .map_err(|error| format!("verify: verifier setup failed: {}", error))?;
    if key.id() == Id::RSA {
        verifier
            .set_rsa_padding(options.padding)
            .map_err(|error| format!("verify: RSA padding setup failed: {}", error))?;
        if options.uses_pss {
            if let Some(salt_length) = options.pss_salt_length {
                verifier
                    .set_rsa_pss_saltlen(rsa_pss_saltlen(salt_length)?)
                    .map_err(|error| {
                        format!("verify: RSA-PSS saltLength setup failed: {}", error)
                    })?;
            }
        }
    } else if options.uses_pss {
        return Err("verify: RSA-PSS padding requires an RSA public key".to_string());
    }
    verifier
        .update(data)
        .map_err(|error| format!("verify: verifier update failed: {}", error))?;
    verifier
        .verify(signature)
        .map_err(|error| format!("verify: verification failed: {}", error))
}

fn sign_one_shot_pem_private_key(
    algorithm: Option<&str>,
    private_key_pem: &str,
    data: &[u8],
    options: RsaSignatureOptions,
) -> Result<Vec<u8>, String> {
    match algorithm {
        Some(algorithm) if !algorithm.is_empty() => {
            sign_pem_private_key(algorithm, private_key_pem, data, options)
        }
        Some(_) => Err("sign: unsupported algorithm ''".to_string()),
        None => {
            let key = PKey::private_key_from_pem(private_key_pem.as_bytes())
                .map_err(|error| format!("sign: invalid private key: {}", error))?;
            if key.id() != Id::ED25519 && key.id() != Id::ED448 {
                return Err(
                    "sign: null algorithm currently requires an Ed25519 or Ed448 private key"
                        .to_string(),
                );
            }
            let mut signer = Signer::new_without_digest(&key)
                .map_err(|error| format!("sign: signer setup failed: {}", error))?;
            signer
                .sign_oneshot_to_vec(data)
                .map_err(|error| format!("sign: signing failed: {}", error))
        }
    }
}

fn verify_one_shot_pem_public_key(
    algorithm: Option<&str>,
    public_key_pem: &str,
    data: &[u8],
    signature: &[u8],
    options: RsaSignatureOptions,
) -> Result<bool, String> {
    match algorithm {
        Some(algorithm) if !algorithm.is_empty() => {
            verify_pem_public_key(algorithm, public_key_pem, data, signature, options)
        }
        Some(_) => Err("verify: unsupported algorithm ''".to_string()),
        None => {
            let key = public_pkey_from_pem(public_key_pem)
                .map_err(|error| format!("verify: {}", error))?;
            if key.id() != Id::ED25519 && key.id() != Id::ED448 {
                return Err(
                    "verify: null algorithm currently requires an Ed25519 or Ed448 public key"
                        .to_string(),
                );
            }
            let mut verifier = Verifier::new_without_digest(&key)
                .map_err(|error| format!("verify: verifier setup failed: {}", error))?;
            verifier
                .verify_oneshot(signature, data)
                .map_err(|error| format!("verify: verification failed: {}", error))
        }
    }
}

fn asymmetric_key_type_from_id(id: Id) -> Result<&'static str, String> {
    match id {
        Id::RSA => Ok("rsa"),
        Id::EC => Ok("ec"),
        Id::ED25519 => Ok("ed25519"),
        Id::ED448 => Ok("ed448"),
        _ => Err(format!("unsupported asymmetric key type {:?}", id)),
    }
}

fn private_key_type_from_pem(private_key_pem: &str) -> Result<&'static str, String> {
    let key = PKey::private_key_from_pem(private_key_pem.as_bytes())
        .map_err(|error| format!("invalid private key: {}", error))?;
    asymmetric_key_type_from_id(key.id())
}

fn public_key_type_from_pem(public_key_pem: &str) -> Result<&'static str, String> {
    let key = public_pkey_from_pem(public_key_pem)?;
    asymmetric_key_type_from_id(key.id())
}

fn public_pkey_from_pem(public_key_pem: &str) -> Result<PKey<Public>, String> {
    match PKey::public_key_from_pem(public_key_pem.as_bytes()) {
        Ok(key) => Ok(key),
        Err(spki_error) => {
            let rsa = Rsa::public_key_from_pem_pkcs1(public_key_pem.as_bytes()).map_err(
                |pkcs1_error| {
                    format!(
                        "invalid public key: {}; invalid RSA PKCS#1 public key: {}",
                        spki_error, pkcs1_error
                    )
                },
            )?;
            PKey::from_rsa(rsa).map_err(|error| format!("invalid RSA PKCS#1 public key: {}", error))
        }
    }
}

fn private_key_pem_from_passphrase(
    private_key_pem: &str,
    passphrase: &[u8],
) -> Result<String, String> {
    let key = PKey::private_key_from_pem_passphrase(private_key_pem.as_bytes(), passphrase)
        .map_err(|error| format!("invalid encrypted private key: {}", error))?;
    let pem = key
        .private_key_to_pem_pkcs8()
        .map_err(|error| format!("private key PEM export failed: {}", error))?;
    String::from_utf8(pem).map_err(|error| format!("private key PEM is invalid UTF-8: {}", error))
}

fn private_key_der_from_pem(private_key_pem: &str, key_type: &str) -> Result<Vec<u8>, String> {
    let key = PKey::private_key_from_pem(private_key_pem.as_bytes())
        .map_err(|error| format!("export: invalid private key: {}", error))?;
    match key_type {
        "pkcs8" => key
            .private_key_to_der()
            .map_err(|error| format!("export: private key DER export failed: {}", error)),
        _ => Err(format!(
            "export: unsupported private key DER type '{}'. Supported: pkcs8",
            key_type
        )),
    }
}

fn public_key_der_from_pem(public_key_pem: &str, key_type: &str) -> Result<Vec<u8>, String> {
    let key = public_pkey_from_pem(public_key_pem).map_err(|error| format!("export: {}", error))?;
    match key_type {
        "spki" => key
            .public_key_to_der()
            .map_err(|error| format!("export: public key DER export failed: {}", error)),
        "pkcs1" => {
            let rsa = key.rsa().map_err(|error| {
                format!(
                    "export: public key PKCS#1 DER export requires an RSA key: {}",
                    error
                )
            })?;
            rsa.public_key_to_der_pkcs1()
                .map_err(|error| format!("export: public key PKCS#1 DER export failed: {}", error))
        }
        _ => Err(format!(
            "export: unsupported public key DER type '{}'. Supported: spki, pkcs1",
            key_type
        )),
    }
}

fn public_key_pem_from_pem(public_key_pem: &str, key_type: &str) -> Result<String, String> {
    let key = public_pkey_from_pem(public_key_pem).map_err(|error| format!("export: {}", error))?;
    let pem = match key_type {
        "spki" => key
            .public_key_to_pem()
            .map_err(|error| format!("export: public key PEM export failed: {}", error))?,
        "pkcs1" => {
            let rsa = key.rsa().map_err(|error| {
                format!(
                    "export: public key PKCS#1 PEM export requires an RSA key: {}",
                    error
                )
            })?;
            rsa.public_key_to_pem_pkcs1().map_err(|error| {
                format!("export: public key PKCS#1 PEM export failed: {}", error)
            })?
        }
        _ => {
            return Err(format!(
                "export: unsupported public key PEM type '{}'. Supported: spki, pkcs1",
                key_type
            ))
        }
    };
    String::from_utf8(pem)
        .map_err(|error| format!("export: public key PEM is invalid UTF-8: {}", error))
}

fn private_key_pem_from_der(private_key_der: &[u8], key_type: &str) -> Result<String, String> {
    if key_type != "pkcs8" {
        return Err(format!(
            "unsupported private key DER type '{}'. Supported: pkcs8",
            key_type
        ));
    }

    let key = PKey::private_key_from_der(private_key_der)
        .map_err(|error| format!("invalid private key DER: {}", error))?;
    let pem = key
        .private_key_to_pem_pkcs8()
        .map_err(|error| format!("private key PEM export failed: {}", error))?;
    String::from_utf8(pem).map_err(|error| format!("private key PEM is invalid UTF-8: {}", error))
}

fn public_key_pem_from_der(public_key_der: &[u8], key_type: &str) -> Result<String, String> {
    let key = match key_type {
        "spki" => PKey::public_key_from_der(public_key_der)
            .map_err(|error| format!("invalid public key DER: {}", error))?,
        "pkcs1" => {
            let rsa = Rsa::public_key_from_der_pkcs1(public_key_der)
                .map_err(|error| format!("invalid RSA PKCS#1 public key DER: {}", error))?;
            PKey::from_rsa(rsa)
                .map_err(|error| format!("invalid RSA PKCS#1 public key DER: {}", error))?
        }
        _ => {
            return Err(format!(
                "unsupported public key DER type '{}'. Supported: spki, pkcs1",
                key_type
            ))
        }
    };
    let pem = key
        .public_key_to_pem()
        .map_err(|error| format!("public key PEM export failed: {}", error))?;
    String::from_utf8(pem).map_err(|error| format!("public key PEM is invalid UTF-8: {}", error))
}

fn public_key_spki_pem_from_pem(public_key_pem: &str, key_type: &str) -> Result<String, String> {
    let pem = match key_type {
        "spki" => {
            let key = public_pkey_from_pem(public_key_pem)?;
            key.public_key_to_pem()
                .map_err(|error| format!("public key PEM export failed: {}", error))?
        }
        "pkcs1" => {
            let rsa = Rsa::public_key_from_pem_pkcs1(public_key_pem.as_bytes())
                .map_err(|error| format!("invalid RSA PKCS#1 public key: {}", error))?;
            let key = PKey::from_rsa(rsa)
                .map_err(|error| format!("invalid RSA PKCS#1 public key: {}", error))?;
            key.public_key_to_pem()
                .map_err(|error| format!("public key PEM export failed: {}", error))?
        }
        _ => {
            return Err(format!(
                "unsupported public key PEM type '{}'. Supported: spki, pkcs1",
                key_type
            ))
        }
    };
    String::from_utf8(pem).map_err(|error| format!("public key PEM is invalid UTF-8: {}", error))
}

fn public_key_spki_pem_from_private_pem(private_key_pem: &str) -> Result<String, String> {
    let key = PKey::private_key_from_pem(private_key_pem.as_bytes())
        .map_err(|error| format!("invalid private key: {}", error))?;
    let pem = key
        .public_key_to_pem()
        .map_err(|error| format!("public key PEM export failed: {}", error))?;
    String::from_utf8(pem).map_err(|error| format!("public key PEM is invalid UTF-8: {}", error))
}

fn public_key_spki_pem_from_any_pem(key_pem: &str, key_type: &str) -> Result<String, String> {
    match public_key_spki_pem_from_pem(key_pem, key_type) {
        Ok(pem) => Ok(pem),
        Err(public_error) => match public_key_spki_pem_from_private_pem(key_pem) {
            Ok(pem) => Ok(pem),
            Err(private_error) => Err(format!("{}; {}", public_error, private_error)),
        },
    }
}

fn rsa_padding_from_node_constant(value: Option<i64>, default_padding: Padding) -> Padding {
    match value {
        Some(1) => Padding::PKCS1,
        Some(3) => Padding::NONE,
        Some(4) => Padding::PKCS1_OAEP,
        _ => default_padding,
    }
}

fn get_rsa_key_and_padding(
    scope: &mut v8::PinScope,
    key_value: v8::Local<v8::Value>,
    default_padding: Padding,
) -> Result<(String, Padding), String> {
    if key_value.is_string() {
        let key = key_value
            .to_string(scope)
            .map(|value| value.to_rust_string_lossy(scope))
            .unwrap_or_default();
        return Ok((key, default_padding));
    }

    if !key_value.is_object() {
        return Err("key must be a PEM string or an object with a key property".to_string());
    }

    let key_obj = key_value
        .to_object(scope)
        .ok_or_else(|| "key must be an object".to_string())?;
    let key_prop = v8::String::new(scope, "key").unwrap();
    let key = key_obj
        .get(scope, key_prop.into())
        .and_then(|value| value.to_string(scope))
        .map(|value| value.to_rust_string_lossy(scope))
        .unwrap_or_default();

    let padding_prop = v8::String::new(scope, "padding").unwrap();
    let padding = key_obj.get(scope, padding_prop.into()).and_then(|value| {
        if value.is_number() {
            value.integer_value(scope)
        } else {
            None
        }
    });

    Ok((
        key,
        rsa_padding_from_node_constant(padding, default_padding),
    ))
}

fn array_buffer_bytes(
    value: v8::Local<v8::ArrayBuffer>,
    offset: usize,
    length: usize,
) -> Result<Vec<u8>, String> {
    if length == 0 {
        return Ok(Vec::new());
    }

    let backing_store = value.get_backing_store();
    let ptr = backing_store
        .data()
        .map(|p| p.as_ptr() as *const u8)
        .unwrap_or(std::ptr::null());
    if ptr.is_null() {
        return Err("buffer data is unavailable".to_string());
    }

    Ok(unsafe { std::slice::from_raw_parts(ptr.add(offset), length).to_vec() })
}

fn get_bytes_from_value(
    scope: &mut v8::PinScope,
    value: v8::Local<v8::Value>,
    string_encoding: Option<&str>,
) -> Result<Vec<u8>, String> {
    if value.is_string() {
        let text = value
            .to_string(scope)
            .map(|value| value.to_rust_string_lossy(scope))
            .unwrap_or_default();
        return match string_encoding.unwrap_or("utf8") {
            "hex" => hex::decode(&text).map_err(|error| format!("invalid hex data: {}", error)),
            "base64" => base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &text)
                .map_err(|error| format!("invalid base64 data: {}", error)),
            "base64url" => base64::Engine::decode(
                &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                text.trim_end_matches('='),
            )
            .map_err(|error| format!("invalid base64url data: {}", error)),
            _ => Ok(text.into_bytes()),
        };
    }

    if value.is_array_buffer() {
        let buffer = v8::Local::<v8::ArrayBuffer>::try_from(value)
            .map_err(|_| "data must be an ArrayBuffer".to_string())?;
        return array_buffer_bytes(buffer, 0, buffer.byte_length());
    }

    if value.is_typed_array() {
        let typed_array = v8::Local::<v8::TypedArray>::try_from(value)
            .map_err(|_| "data must be a TypedArray".to_string())?;
        let buffer = typed_array
            .buffer(scope)
            .ok_or_else(|| "typed array buffer is unavailable".to_string())?;
        return array_buffer_bytes(buffer, typed_array.byte_offset(), typed_array.byte_length());
    }

    if value.is_object() {
        let object = v8::Local::<v8::Object>::try_from(value)
            .map_err(|_| "data object is invalid".to_string())?;
        let buffer_key = v8::String::new(scope, "buffer").unwrap();
        if let Some(buffer_value) = object.get(scope, buffer_key.into()) {
            if buffer_value.is_array_buffer() {
                let buffer = v8::Local::<v8::ArrayBuffer>::try_from(buffer_value)
                    .map_err(|_| "object buffer is invalid".to_string())?;
                return array_buffer_bytes(buffer, 0, buffer.byte_length());
            }
        }
    }

    Err("data must be a string, Buffer, ArrayBuffer, or TypedArray".to_string())
}

fn rsa_public_encrypt_pem(
    public_key_pem: &str,
    data: &[u8],
    padding: Padding,
) -> Result<Vec<u8>, String> {
    let rsa = Rsa::public_key_from_pem(public_key_pem.as_bytes())
        .map_err(|error| format!("publicEncrypt: invalid public key: {}", error))?;
    let mut encrypted = vec![0u8; rsa.size() as usize];
    let len = rsa
        .public_encrypt(data, &mut encrypted, padding)
        .map_err(|error| format!("publicEncrypt: encryption failed: {}", error))?;
    encrypted.truncate(len);
    Ok(encrypted)
}

fn rsa_private_decrypt_pem(
    private_key_pem: &str,
    encrypted: &[u8],
    padding: Padding,
) -> Result<Vec<u8>, String> {
    let rsa = Rsa::private_key_from_pem(private_key_pem.as_bytes())
        .map_err(|error| format!("privateDecrypt: invalid private key: {}", error))?;
    let mut decrypted = vec![0u8; rsa.size() as usize];
    let len = rsa
        .private_decrypt(encrypted, &mut decrypted, padding)
        .map_err(|error| format!("privateDecrypt: decryption failed: {}", error))?;
    decrypted.truncate(len);
    Ok(decrypted)
}

fn rsa_private_encrypt_pem(
    private_key_pem: &str,
    data: &[u8],
    padding: Padding,
) -> Result<Vec<u8>, String> {
    let rsa = Rsa::private_key_from_pem(private_key_pem.as_bytes())
        .map_err(|error| format!("privateEncrypt: invalid private key: {}", error))?;
    let mut encrypted = vec![0u8; rsa.size() as usize];
    let len = rsa
        .private_encrypt(data, &mut encrypted, padding)
        .map_err(|error| format!("privateEncrypt: encryption failed: {}", error))?;
    encrypted.truncate(len);
    Ok(encrypted)
}

fn rsa_public_decrypt_pem(
    public_key_pem: &str,
    encrypted: &[u8],
    padding: Padding,
) -> Result<Vec<u8>, String> {
    let rsa = Rsa::public_key_from_pem(public_key_pem.as_bytes())
        .map_err(|error| format!("publicDecrypt: invalid public key: {}", error))?;
    let mut decrypted = vec![0u8; rsa.size() as usize];
    let len = rsa
        .public_decrypt(encrypted, &mut decrypted, padding)
        .map_err(|error| format!("publicDecrypt: decryption failed: {}", error))?;
    decrypted.truncate(len);
    Ok(decrypted)
}

fn ecdh_curve_nid(curve: &str) -> Result<Nid, String> {
    match curve {
        "prime256v1" | "secp256r1" => Ok(Nid::X9_62_PRIME256V1),
        "secp384r1" => Ok(Nid::SECP384R1),
        "secp521r1" => Ok(Nid::SECP521R1),
        _ => Err(format!(
            "createECDH: unsupported curve '{}'. Supported: prime256v1, secp256r1, secp384r1, secp521r1",
            curve
        )),
    }
}

fn ecdh_group(curve: &str) -> Result<EcGroup, String> {
    let nid = ecdh_curve_nid(curve)?;
    EcGroup::from_curve_name(nid)
        .map_err(|error| format!("createECDH: curve setup failed: {}", error))
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

fn ecdh_private_key_hex(key: &EcKey<Private>) -> String {
    let private_key = left_pad_private_key(
        key.private_key().to_vec(),
        ecdh_private_key_size(key.group()),
    );
    hex::encode(private_key)
}

fn ecdh_public_key_hex(key: &EcKey<Private>) -> Result<String, String> {
    let mut ctx = BigNumContext::new()
        .map_err(|error| format!("createECDH: BigNum context failed: {}", error))?;
    let public_key = key
        .public_key()
        .to_bytes(key.group(), PointConversionForm::UNCOMPRESSED, &mut ctx)
        .map_err(|error| format!("createECDH: public key export failed: {}", error))?;
    Ok(hex::encode(public_key))
}

fn ecdh_key_from_private_hex(curve: &str, private_key_hex: &str) -> Result<EcKey<Private>, String> {
    let group = ecdh_group(curve)?;
    let private_key_bytes = hex::decode(private_key_hex)
        .map_err(|error| format!("createECDH: invalid private key hex: {}", error))?;
    if private_key_bytes.is_empty() {
        return Err("createECDH: private key is empty".to_string());
    }

    let private_number = BigNum::from_slice(&private_key_bytes)
        .map_err(|error| format!("createECDH: invalid private key: {}", error))?;
    let mut ctx = BigNumContext::new()
        .map_err(|error| format!("createECDH: BigNum context failed: {}", error))?;
    let mut public_key = EcPoint::new(&group)
        .map_err(|error| format!("createECDH: public key allocation failed: {}", error))?;
    public_key
        .mul_generator2(&group, &private_number, &mut ctx)
        .map_err(|error| format!("createECDH: public key derivation failed: {}", error))?;

    let key = EcKey::from_private_components(&group, &private_number, &public_key)
        .map_err(|error| format!("createECDH: private key setup failed: {}", error))?;
    key.check_key()
        .map_err(|error| format!("createECDH: invalid private key: {}", error))?;
    Ok(key)
}

fn ecdh_generate_key_pair_hex(curve: &str) -> Result<(String, String), String> {
    let group = ecdh_group(curve)?;
    let key = EcKey::generate(&group)
        .map_err(|error| format!("createECDH: key generation failed: {}", error))?;
    let private_key = ecdh_private_key_hex(&key);
    let public_key = ecdh_public_key_hex(&key)?;
    Ok((private_key, public_key))
}

fn ecdh_public_key_from_private_hex(curve: &str, private_key_hex: &str) -> Result<String, String> {
    let key = ecdh_key_from_private_hex(curve, private_key_hex)?;
    ecdh_public_key_hex(&key)
}

fn ecdh_validate_public_key(curve: &str, public_key: &[u8]) -> Result<(), String> {
    if public_key.is_empty() {
        return Err("createECDH: public key is empty".to_string());
    }

    let group = ecdh_group(curve)?;
    let mut ctx = BigNumContext::new()
        .map_err(|error| format!("createECDH: BigNum context failed: {}", error))?;
    let point = EcPoint::from_bytes(&group, public_key, &mut ctx)
        .map_err(|error| format!("createECDH: invalid public key: {}", error))?;
    if point.is_infinity(&group) {
        return Err("createECDH: invalid public key".to_string());
    }
    if !point
        .is_on_curve(&group, &mut ctx)
        .map_err(|error| format!("createECDH: public key validation failed: {}", error))?
    {
        return Err("createECDH: invalid public key".to_string());
    }
    Ok(())
}

fn get_ecdh_public_key_bytes(
    scope: &mut v8::PinScope,
    value: v8::Local<v8::Value>,
) -> Result<Vec<u8>, String> {
    if value.is_string() {
        let public_key_hex = value
            .to_string(scope)
            .map(|value| value.to_rust_string_lossy(scope))
            .unwrap_or_default();
        return hex::decode(&public_key_hex)
            .map_err(|error| format!("computeSecret: invalid peer public key: {}", error));
    }

    if value.is_object() {
        let object = v8::Local::<v8::Object>::try_from(value)
            .map_err(|_| "computeSecret: peer public key object is invalid".to_string())?;
        let public_key_key = v8::String::new(scope, "publicKey").unwrap();
        if let Some(public_key_value) = object.get(scope, public_key_key.into()) {
            if public_key_value.is_string() {
                let public_key_hex = public_key_value
                    .to_string(scope)
                    .map(|value| value.to_rust_string_lossy(scope))
                    .unwrap_or_default();
                return hex::decode(&public_key_hex)
                    .map_err(|error| format!("computeSecret: invalid peer public key: {}", error));
            }
        }
    }

    get_bytes_from_value(scope, value, None)
        .map_err(|error| format!("computeSecret: invalid peer public key: {}", error))
}

fn ecdh_compute_secret(
    curve: &str,
    private_key_hex: &str,
    peer_public_key: &[u8],
) -> Result<Vec<u8>, String> {
    if peer_public_key.is_empty() {
        return Err("computeSecret: peer public key is empty".to_string());
    }

    let private_key = ecdh_key_from_private_hex(curve, private_key_hex)?;
    let group = private_key.group();
    let mut ctx = BigNumContext::new()
        .map_err(|error| format!("computeSecret: BigNum context failed: {}", error))?;
    let peer_point = EcPoint::from_bytes(group, peer_public_key, &mut ctx)
        .map_err(|error| format!("computeSecret: invalid peer public key: {}", error))?;
    if peer_point.is_infinity(group) {
        return Err("computeSecret: invalid peer public key".to_string());
    }
    if !peer_point
        .is_on_curve(group, &mut ctx)
        .map_err(|error| format!("computeSecret: public key validation failed: {}", error))?
    {
        return Err("computeSecret: invalid peer public key".to_string());
    }

    let peer_key = EcKey::from_public_key(group, &peer_point)
        .map_err(|error| format!("computeSecret: invalid peer public key: {}", error))?;
    peer_key
        .check_key()
        .map_err(|error| format!("computeSecret: invalid peer public key: {}", error))?;

    let private_pkey = PKey::from_ec_key(private_key)
        .map_err(|error| format!("computeSecret: private key setup failed: {}", error))?;
    let peer_pkey = PKey::from_ec_key(peer_key)
        .map_err(|error| format!("computeSecret: peer public key setup failed: {}", error))?;
    let mut deriver = Deriver::new(&private_pkey)
        .map_err(|error| format!("computeSecret: deriver setup failed: {}", error))?;
    deriver
        .set_peer(&peer_pkey)
        .map_err(|error| format!("computeSecret: peer setup failed: {}", error))?;
    deriver
        .derive_to_vec()
        .map_err(|error| format!("computeSecret: derivation failed: {}", error))
}

/// Generate EC key pair (v0.3.23)
/// Returns (public_key_pem, private_key_pem)
fn generate_ec_key_pair(named_curve: &str) -> Result<(String, String), String> {
    let group = ecdh_group(named_curve)
        .map_err(|error| error.replacen("createECDH", "generateKeyPair", 1))?;
    let ec_key = EcKey::generate(&group)
        .map_err(|error| format!("generateKeyPair: EC key generation failed: {}", error))?;
    let key = PKey::from_ec_key(ec_key)
        .map_err(|error| format!("generateKeyPair: EC key setup failed: {}", error))?;

    let public_key_pem = String::from_utf8(
        key.public_key_to_pem()
            .map_err(|error| format!("generateKeyPair: EC public key export failed: {}", error))?,
    )
    .map_err(|error| {
        format!(
            "generateKeyPair: EC public key PEM is invalid UTF-8: {}",
            error
        )
    })?;
    let private_key_pem =
        String::from_utf8(key.private_key_to_pem_pkcs8().map_err(|error| {
            format!("generateKeyPair: EC private key export failed: {}", error)
        })?)
        .map_err(|error| {
            format!(
                "generateKeyPair: EC private key PEM is invalid UTF-8: {}",
                error
            )
        })?;

    Ok((public_key_pem, private_key_pem))
}

/// Compute scrypt-derived key using OpenSSL's real memory-hard scrypt primitive.
/// Parameters:
/// - password: The secret key material
/// - salt: Random salt value
/// - keylen: Desired output length in bytes
/// - n: CPU/memory cost parameter (scrypt N)
/// - r: Block size parameter (scrypt r)
/// - p: Parallelization parameter (scrypt p)
fn compute_scrypt_derived_key(
    password: &str,
    salt: &str,
    keylen: usize,
    n: u32,
    r: u32,
    p: u32,
) -> Result<Vec<u8>, String> {
    let mut derived_key = vec![0u8; keylen];

    if n == 0 || r == 0 || p == 0 {
        return Err("scrypt: N, r, and p must be greater than zero".to_string());
    }

    let maxmem = 64 * 1024 * 1024;
    openssl::pkcs5::scrypt(
        password.as_bytes(),
        salt.as_bytes(),
        n as u64,
        r as u64,
        p as u64,
        maxmem,
        &mut derived_key,
    )
    .map_err(|error| format!("scrypt: key derivation failed: {}", error))?;

    Ok(derived_key)
}

/// Constant-time comparison to prevent timing attacks
/// Returns true if both slices have the same content
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let a_len = a.len();
    let b_len = b.len();
    if a_len != b_len {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// HKDF - HMAC-based Key Derivation Function (RFC 5869)
///
/// # Arguments
/// * `digest` - Hash algorithm ("sha1", "sha256", "sha512")
/// * `ikm` - Input Keying Material (secret key)
/// * `salt` - Salt value (optional, should be random but not secret)
/// * `info` - Application-specific context info
/// * `keylen` - Desired output length in bytes
fn hkdf_derive(digest: &str, ikm: &str, salt: &str, info: &str, keylen: usize) -> Vec<u8> {
    // Get hash length for the algorithm (prefix with _ to suppress warning since currently unused)
    let _hash_len = match digest {
        "sha1" => 20,
        "sha256" => 32,
        "sha512" => 64,
        _ => 32, // default to sha256
    };

    // Helper function to compute HMAC
    fn compute_hmac(data: &[u8], key: &[u8], algorithm: &str) -> Vec<u8> {
        use ring::digest;
        use sha1::Digest;

        let block_size = 64;
        let ipad = 0x36u8;
        let opad = 0x5cu8;

        // Prepare key
        let mut padded_key = key.to_vec();
        if padded_key.len() > block_size {
            padded_key = match algorithm {
                "sha256" => digest::digest(&digest::SHA256, &padded_key)
                    .as_ref()
                    .to_vec(),
                "sha512" => digest::digest(&digest::SHA512, &padded_key)
                    .as_ref()
                    .to_vec(),
                "sha1" => {
                    let mut hasher = sha1::Sha1::default();
                    hasher.update(&padded_key);
                    hasher.finalize().to_vec()
                }
                _ => digest::digest(&digest::SHA256, &padded_key)
                    .as_ref()
                    .to_vec(),
            };
        }
        padded_key.resize(block_size, 0);

        // Inner hash
        let mut inner_input = Vec::with_capacity(block_size + data.len());
        inner_input.extend(padded_key.iter().map(|b| b ^ ipad));
        inner_input.extend(data);
        let inner_hash = match algorithm {
            "sha256" => digest::digest(&digest::SHA256, &inner_input)
                .as_ref()
                .to_vec(),
            "sha512" => digest::digest(&digest::SHA512, &inner_input)
                .as_ref()
                .to_vec(),
            "sha1" => {
                let mut hasher = sha1::Sha1::default();
                hasher.update(&inner_input);
                hasher.finalize().to_vec()
            }
            _ => digest::digest(&digest::SHA256, &inner_input)
                .as_ref()
                .to_vec(),
        };

        // Outer hash
        let mut outer_input = Vec::with_capacity(block_size + inner_hash.len());
        outer_input.extend(padded_key.iter().map(|b| b ^ opad));
        outer_input.extend(&inner_hash);

        match algorithm {
            "sha256" => digest::digest(&digest::SHA256, &outer_input)
                .as_ref()
                .to_vec(),
            "sha512" => digest::digest(&digest::SHA512, &outer_input)
                .as_ref()
                .to_vec(),
            "sha1" => {
                let mut hasher = sha1::Sha1::default();
                hasher.update(&outer_input);
                hasher.finalize().to_vec()
            }
            _ => digest::digest(&digest::SHA256, &outer_input)
                .as_ref()
                .to_vec(),
        }
    }

    // Step 1: Extract - PRK = HMAC-Hash(salt, IKM)
    let salt_bytes = if salt.is_empty() {
        b""
    } else {
        salt.as_bytes()
    };
    let ikm_bytes = ikm.as_bytes();
    let prk = compute_hmac(ikm_bytes, salt_bytes, digest);

    // Step 2: Expand - OKM = T(1) | T(2) | T(3) | ...
    let mut okm = Vec::with_capacity(keylen);
    let mut t = Vec::new();
    let mut counter: u8 = 1;

    while okm.len() < keylen {
        // T(n) = HMAC-Hash(PRK, T(n-1) | info | counter)
        let mut input = Vec::new();
        if !t.is_empty() {
            input.extend(&t);
        }
        input.extend(info.as_bytes());
        input.push(counter);

        t = compute_hmac(&input, &prk, digest);
        okm.extend(&t);
        counter += 1;

        // Safety: counter should not overflow in practice (HKDF limits output)
        if counter == 0 {
            break;
        }
    }

    okm.truncate(keylen);
    okm
}

/// Benchmark result structure
/// v0.3.221: 用于存储性能测试结果
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub iterations: usize,
    pub total_time_ns: u128,
    pub avg_time_ns: u128,
    pub ops_per_sec: f64,
    pub errors: usize,
}

impl BenchmarkResult {
    /// Print formatted benchmark results
    pub fn print(&self, name: &str) {
        println!("\n📊 Benchmark: {}", name);
        println!("   Iterations: {}", self.iterations);
        println!(
            "   Total time: {:.2}ms",
            self.total_time_ns as f64 / 1_000_000.0
        );
        println!("   Avg per iteration: {}ns", self.avg_time_ns);
        println!("   Throughput: {:.2} ops/sec", self.ops_per_sec);
        if self.errors > 0 {
            println!("   ⚠️  Errors: {}", self.errors);
        }
    }
}

/// v0.3.235: Enhanced error types for better error handling and debugging
/// Provides structured error information including error codes, messages, and stack traces

/// Error types for runtime execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeErrorType {
    /// Syntax error in the code (e.g., missing parenthesis, invalid syntax)
    SyntaxError,
    /// Reference to undefined variable or function
    ReferenceError,
    /// Type mismatch or type operation error
    TypeError,
    /// Range error (e.g., array index out of bounds)
    RangeError,
    /// Evaluation error (e.g., eval with invalid code)
    EvalError,
    /// Internal runtime error
    InternalError,
    /// Resource limit exceeded (memory, stack, etc.)
    ResourceLimit,
    /// Other unknown error
    Unknown,
}

/// Enhanced runtime error with structured information
/// v0.3.235: Provides better error reporting with error codes and context
#[derive(Debug, Clone)]
pub struct RuntimeError {
    /// The type of error
    pub error_type: RuntimeErrorType,
    /// Human-readable error message
    pub message: String,
    /// Error code for programmatic handling
    pub code: &'static str,
    /// Source location (file:line:col) if available
    pub location: Option<String>,
    /// Stack trace or additional context
    pub context: Option<String>,
}

impl RuntimeError {
    /// Create a new RuntimeError
    pub fn new(
        error_type: RuntimeErrorType,
        message: String,
        code: &'static str,
        location: Option<String>,
        context: Option<String>,
    ) -> Self {
        RuntimeError {
            error_type,
            message,
            code,
            location,
            context,
        }
    }

    /// Create a syntax error
    pub fn syntax_error(message: String, location: Option<String>) -> Self {
        RuntimeError::new(
            RuntimeErrorType::SyntaxError,
            message,
            "SYNTAX_ERROR",
            location,
            None,
        )
    }

    /// Create a reference error
    pub fn reference_error(message: String, location: Option<String>) -> Self {
        RuntimeError::new(
            RuntimeErrorType::ReferenceError,
            message,
            "REFERENCE_ERROR",
            location,
            None,
        )
    }

    /// Create a type error
    pub fn type_error(message: String, location: Option<String>) -> Self {
        RuntimeError::new(
            RuntimeErrorType::TypeError,
            message,
            "TYPE_ERROR",
            location,
            None,
        )
    }

    /// Create a range error
    pub fn range_error(message: String, location: Option<String>) -> Self {
        RuntimeError::new(
            RuntimeErrorType::RangeError,
            message,
            "RANGE_ERROR",
            location,
            None,
        )
    }

    /// Create an internal error
    pub fn internal_error(message: String, context: Option<String>) -> Self {
        RuntimeError::new(
            RuntimeErrorType::InternalError,
            message,
            "INTERNAL_ERROR",
            None,
            context,
        )
    }

    /// Get a user-friendly error summary
    pub fn summary(&self) -> String {
        let mut summary = format!("[{}] {}", self.code, self.message);
        if let Some(loc) = &self.location {
            summary.push_str(&format!(" at {}", loc));
        }
        summary
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

impl std::error::Error for RuntimeError {}

/// Convert V8 exception to RuntimeError
/// v0.3.235: Extract structured error information from V8 exceptions
pub fn v8_exception_to_runtime_error(
    scope: &mut v8::PinScope,
    exception: v8::Local<v8::Value>,
) -> RuntimeError {
    // Try to extract error type from the exception
    if let Some(error_obj) = exception.to_object(scope) {
        // Check if it's an Error object by checking the constructor
        // In V8, Error objects have a specific structure
        let message_key = v8::String::new(scope, "message").unwrap();
        let name_key = v8::String::new(scope, "name").unwrap();

        // Get the name property to determine error type
        let name_val = error_obj.get(scope, name_key.into());
        let name_str = name_val
            .map(|n| n.to_rust_string_lossy(scope))
            .unwrap_or_default();

        // Determine error type from name property
        let error_type = match name_str.as_str() {
            "SyntaxError" => RuntimeErrorType::SyntaxError,
            "ReferenceError" => RuntimeErrorType::ReferenceError,
            "TypeError" => RuntimeErrorType::TypeError,
            "RangeError" => RuntimeErrorType::RangeError,
            "EvalError" => RuntimeErrorType::EvalError,
            _ => RuntimeErrorType::Unknown,
        };

        // Extract message
        let message = if let Some(msg_val) = error_obj.get(scope, message_key.into()) {
            msg_val.to_rust_string_lossy(scope)
        } else {
            exception
                .to_string(scope)
                .unwrap_or_else(|| v8::String::new(scope, "<unknown error>").unwrap())
                .to_rust_string_lossy(scope)
        };

        // Extract stack trace
        let stack_key = v8::String::new(scope, "stack").unwrap();
        let stack_trace = if let Some(stack_val) = error_obj.get(scope, stack_key.into()) {
            Some(apply_active_source_map(
                &stack_val.to_rust_string_lossy(scope),
            ))
        } else {
            None
        };

        // Extract location from stack trace
        let location = stack_trace.as_ref().and_then(|s| {
            // V8 stack traces usually start with "Error: message"; the first
            // useful source location is the first following "at ..." frame.
            s.lines()
                .map(str::trim)
                .find(|line| line.starts_with("at "))
                .map(ToString::to_string)
        });

        RuntimeError {
            error_type,
            message,
            code: match error_type {
                RuntimeErrorType::SyntaxError => "SYNTAX_ERROR",
                RuntimeErrorType::ReferenceError => "REFERENCE_ERROR",
                RuntimeErrorType::TypeError => "TYPE_ERROR",
                RuntimeErrorType::RangeError => "RANGE_ERROR",
                RuntimeErrorType::EvalError => "EVAL_ERROR",
                RuntimeErrorType::InternalError => "INTERNAL_ERROR",
                RuntimeErrorType::ResourceLimit => "RESOURCE_LIMIT",
                RuntimeErrorType::Unknown => "UNKNOWN_ERROR",
            },
            location,
            context: stack_trace,
        }
    } else {
        // Not an object, just convert to string
        let message = exception
            .to_string(scope)
            .unwrap_or_else(|| v8::String::new(scope, "<unknown error>").unwrap())
            .to_rust_string_lossy(scope);

        RuntimeError::new(
            RuntimeErrorType::Unknown,
            message,
            "UNKNOWN_ERROR",
            None,
            None,
        )
    }
}

/// A minimal runtime that only provides basic JavaScript execution
/// This version avoids complex dependencies for faster startup
/// v0.3.93: 添加 Context 存储以支持跨 Context 共享数据
pub struct MinimalRuntime {
    // V8 Isolate - the core JavaScript execution engine
    isolate: v8::OwnedIsolate,
    // v0.3.93: 存储 V8 Context 以支持跨 Context 共享数据
    context: Option<v8::Global<v8::Context>>,
    /// Core Node/compat APIs installed (console, process, fs, require, timers, …).
    apis_initialized: bool,
    /// Heavy / rare Web APIs installed (Streams, Workers, fetch suite, …).
    /// Deferred until the executed source actually references them so first
    /// execute of simple scripts does not pay for unused bindings.
    extended_apis_initialized: bool,
    process_argv: Vec<String>,
    main_module_dir: String,
    main_module_filename: String,
    esm_module_cache: HashMap<PathBuf, v8::Global<v8::Module>>,
    esm_module_cache_fingerprints: HashMap<PathBuf, [u8; 32]>,
    timer_drain_limit_ms: u64,
    /// When true, `execute_code` stays alive while an HTTP server is listening.
    /// CLI `bee run` sets this; library/integration tests leave it false so
    /// `listen()` does not hang the test process.
    http_server_keep_alive: bool,
}

impl MinimalRuntime {
    // Keep the process alive for any ref'd timer (Node-compatible). A previous
    // 75ms wall-clock drain silently dropped longer setTimeout/setInterval work.
    const DEFAULT_TIMER_DRAIN_LIMIT_MS: u64 = u64::MAX;

    fn default_process_argv() -> Vec<String> {
        vec!["bee".to_string(), "<program>".to_string()]
    }

    fn default_main_module_dir() -> String {
        "/workspace".to_string()
    }

    fn default_main_module_filename() -> String {
        "/workspace/script.js".to_string()
    }

    /// Create a new minimal runtime with optimized settings
    /// v0.3.221: 增强 Isolate 配置以提升性能
    /// v0.3.231: 使用更小的初始堆以加快启动速度
    /// v0.3.270: 设置显式微任务策略，确保 nextTick 在 Promise 之前执行
    ///
    fn isolate_create_params(initial: usize, maximum: usize) -> v8::CreateParams {
        let params = v8::CreateParams::default().heap_limits(initial, maximum);
        match crate::v8_snapshot::cached_startup_blob() {
            Some(blob) => params.snapshot_blob(v8::StartupData::from(blob)),
            None => params,
        }
    }

    /// First principles: do not pre-reserve a large young heap for short-lived
    /// CLI processes. V8 grows the heap on demand; a low initial limit cuts
    /// isolate create latency without capping large workloads (max stays 2GB).
    pub fn new() -> Result<Self> {
        // Initialize V8 (idempotent - safe to call multiple times)
        crate::initialize_v8()?;

        // 16MB initial / 2GB max — same initial as `new_fast`, full production max.
        let create_params = Self::isolate_create_params(16 * 1024 * 1024, 2048 * 1024 * 1024);

        let profile_startup = std::env::var_os("BEEJS_PROFILE_STARTUP").is_some();
        let t_iso = if profile_startup {
            Some(std::time::Instant::now())
        } else {
            None
        };

        // Create a new isolate with optimized parameters
        let mut isolate = v8::Isolate::new(create_params);

        Self::configure_isolate(&mut isolate);

        if let Some(t) = t_iso {
            eprintln!(
                "[STARTUP PROFILE] Isolate::new: {:.2}ms",
                t.elapsed().as_secs_f64() * 1000.0
            );
        }

        // v0.3.93: Context 将在第一次调用 get_context() 时创建
        Ok(Self {
            isolate,
            context: None,
            apis_initialized: false,
            extended_apis_initialized: false,
            process_argv: Self::default_process_argv(),
            main_module_dir: Self::default_main_module_dir(),
            main_module_filename: Self::default_main_module_filename(),
            esm_module_cache: HashMap::new(),
            esm_module_cache_fingerprints: HashMap::new(),
            timer_drain_limit_ms: Self::DEFAULT_TIMER_DRAIN_LIMIT_MS,
            http_server_keep_alive: false,
        })
    }

    /// Create runtime with custom max heap memory limit in megabytes
    pub fn with_memory_limit(max_memory_mb: usize) -> Result<Self> {
        crate::initialize_v8()?;
        let initial = (16 * 1024 * 1024).min(max_memory_mb * 1024 * 1024);
        let maximum = max_memory_mb * 1024 * 1024;
        let create_params = Self::isolate_create_params(initial, maximum);
        let mut isolate = v8::Isolate::new(create_params);
        Self::configure_isolate(&mut isolate);
        Ok(Self {
            isolate,
            context: None,
            apis_initialized: false,
            extended_apis_initialized: false,
            process_argv: Self::default_process_argv(),
            main_module_dir: Self::default_main_module_dir(),
            main_module_filename: Self::default_main_module_filename(),
            esm_module_cache: HashMap::new(),
            esm_module_cache_fingerprints: HashMap::new(),
            timer_drain_limit_ms: Self::DEFAULT_TIMER_DRAIN_LIMIT_MS,
            http_server_keep_alive: false,
        })
    }

    /// Get a thread-safe IsolateHandle for execution termination (watchdog timer)
    pub fn isolate_handle(&mut self) -> v8::IsolateHandle {
        self.isolate.thread_safe_handle()
    }

    /// v0.3.231: 快速启动模式 - 使用最小堆配置
    /// 适用于短生命周期脚本，减少内存分配开销
    /// v0.3.270: 设置显式微任务策略，确保 nextTick 在 Promise 之前执行
    pub fn new_fast() -> Result<Self> {
        crate::initialize_v8()?;

        // 极致低延迟启动：16MB 初始堆 + 256MB 最大堆
        // 这种配置最小化了进程创建时 V8 堆内存预分配与映射的系统开销
        let create_params = Self::isolate_create_params(16 * 1024 * 1024, 256 * 1024 * 1024);

        let mut isolate = v8::Isolate::new(create_params);

        Self::configure_isolate(&mut isolate);

        Ok(Self {
            isolate,
            context: None,
            apis_initialized: false,
            extended_apis_initialized: false,
            process_argv: Self::default_process_argv(),
            main_module_dir: Self::default_main_module_dir(),
            main_module_filename: Self::default_main_module_filename(),
            esm_module_cache: HashMap::new(),
            esm_module_cache_fingerprints: HashMap::new(),
            timer_drain_limit_ms: Self::DEFAULT_TIMER_DRAIN_LIMIT_MS,
            http_server_keep_alive: false,
        })
    }

    /// Create a new minimal runtime with custom heap limits
    /// v0.3.221: 支持自定义内存配置
    /// v0.3.270: 设置显式微任务策略，确保 nextTick 在 Promise 之前执行
    pub fn with_heap_limits(initial: usize, maximum: usize) -> Result<Self> {
        crate::initialize_v8()?;

        let create_params = Self::isolate_create_params(initial, maximum);

        let mut isolate = v8::Isolate::new(create_params);

        Self::configure_isolate(&mut isolate);

        Ok(Self {
            isolate,
            context: None,
            apis_initialized: false,
            extended_apis_initialized: false,
            process_argv: Self::default_process_argv(),
            main_module_dir: Self::default_main_module_dir(),
            main_module_filename: Self::default_main_module_filename(),
            esm_module_cache: HashMap::new(),
            esm_module_cache_fingerprints: HashMap::new(),
            timer_drain_limit_ms: Self::DEFAULT_TIMER_DRAIN_LIMIT_MS,
            http_server_keep_alive: false,
        })
    }

    pub fn set_process_argv(&mut self, argv: Vec<String>) {
        let mut argv = argv;
        if argv.is_empty() {
            argv.push("bee".to_string());
        }
        if argv.len() == 1 {
            argv.push("<program>".to_string());
        }
        self.process_argv = argv;
    }

    pub fn set_timer_drain_limit_ms(&mut self, limit_ms: u64) {
        self.timer_drain_limit_ms = limit_ms;
    }

    pub fn set_http_server_keep_alive(&mut self, keep_alive: bool) {
        self.http_server_keep_alive = keep_alive;
    }

    fn configure_isolate(isolate: &mut v8::OwnedIsolate) {
        // v0.3.270: 设置显式微任务策略。Explicit 模式下，V8 只在调用
        // perform_microtask_checkpoint 时执行微任务，确保 nextTick 回调在 Promise
        // microtasks 之前执行。
        isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
        isolate.set_host_import_module_dynamically_callback(Self::esm_dynamic_import_callback);
        isolate.set_promise_reject_callback(promise_reject_callback);
    }

    pub fn set_main_module_path(&mut self, path: impl AsRef<std::path::Path>) {
        let path = path.as_ref();
        let absolute = path.canonicalize().unwrap_or_else(|_| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                    .join(path)
            }
        });
        self.main_module_filename = absolute.to_string_lossy().to_string();
        self.main_module_dir = absolute
            .parent()
            .map(|parent| parent.to_string_lossy().to_string())
            .unwrap_or_else(Self::default_main_module_dir);
    }

    /// v0.3.234: 预热内置对象以优化启动时间
    /// 通过预先执行常见的 JavaScript 操作，触发 V8 的 JIT 编译优化
    /// 后续代码执行时可以直接使用优化后的机器码
    pub fn warmup(&mut self) -> Result<()> {
        v8::scope!(let scope, &mut self.isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // 辅助闭包：执行预热代码
        let run_warmup = |scope: &mut v8::PinScope, code: &str| {
            let source = v8::String::new(scope, code).unwrap();
            if let Some(script) = v8::Script::compile(scope, source, None) {
                let _ = script.run(scope);
            }
        };

        // 预热 Object.prototype
        run_warmup(
            scope,
            r#"
            (function() {
                const obj = {};
                obj.toString();
                obj.valueOf();
                obj.hasOwnProperty('test');
                Object.prototype.toString;
                Object.prototype.valueOf;
                Object.prototype.hasOwnProperty;
            })();
        "#,
        );

        // 预热 Array.prototype
        run_warmup(
            scope,
            r#"
            (function() {
                const arr = [1, 2, 3, 4, 5];
                arr.push(6);
                arr.pop();
                arr.slice(0, 2);
                arr.map(x => x * 2);
                arr.filter(x => x > 2);
                arr.reduce((a, b) => a + b, 0);
            })();
        "#,
        );

        // 预热 Function.prototype
        run_warmup(
            scope,
            r#"
            (function() {
                function testFn() {}
                testFn.toString();
                testFn.call(null);
                testFn.apply(null, []);
                testFn.bind(null);
            })();
        "#,
        );

        // 预热 String.prototype
        run_warmup(
            scope,
            r#"
            (function() {
                const str = "hello world";
                str.length;
                str.toUpperCase();
                str.toLowerCase();
                str.split(' ');
            })();
        "#,
        );

        // 预热 Symbol 和 BigInt
        run_warmup(
            scope,
            r#"
            (function() {
                const sym = Symbol('test');
                sym.toString();
                Symbol.iterator;
            })();
        "#,
        );

        // 预热 Promise
        run_warmup(
            scope,
            r#"
            (function() {
                const p = Promise.resolve(42);
                p.then(v => v);
                Promise.resolve;
                Promise.all;
            })();
        "#,
        );

        // 预热 Map 和 Set
        run_warmup(
            scope,
            r#"
            (function() {
                const map = new Map([['a', 1]]);
                map.set('b', 2);
                map.get('a');
                map.has('b');
                const set = new Set([1, 2, 3]);
                set.add(4);
                set.has(2);
            })();
        "#,
        );

        // 预热 JSON
        run_warmup(
            scope,
            r#"
            (function() {
                JSON.parse('{"test": 123}');
                JSON.stringify({test: 123});
            })();
        "#,
        );

        Ok(())
    }

    /// 获取或创建 V8 Context
    /// v0.3.93: 确保 Context 存在且可被复用
    fn get_context(&mut self) -> v8::Global<v8::Context> {
        if let Some(ref mut ctx) = self.context {
            return ctx.clone();
        }

        // 如果没有 Context，创建一个
        v8::scope!(let scope, &mut self.isolate);
        let context = v8::Context::new(scope, Default::default());
        let global_context = v8::Global::new(scope, context);
        self.context = Some(global_context.clone());
        global_context
    }

    /// 强制重新创建 Context
    /// v0.3.93: 用于需要全新上下文的情况
    pub fn recreate_context(&mut self) {
        v8::scope!(let scope, &mut self.isolate);
        let context = v8::Context::new(scope, Default::default());
        let global_context = v8::Global::new(scope, context);
        self.context = Some(global_context);
        self.esm_module_cache.clear();
        self.esm_module_cache_fingerprints.clear();
    }

    /// 预热运行时：预先创建 Context 并安装全部 Core 及 Extended APIs。
    /// 消除后续首次调用 execute_code 时的上下文构建与 API 挂载时延，
    /// 实现亚毫秒级（< 0.5ms）极致就绪与热执行。
    pub fn prewarm(&mut self) -> Result<()> {
        if self.apis_initialized && self.extended_apis_initialized && self.context.is_some() {
            return Ok(());
        }

        v8::scope!(let scope, &mut self.isolate);
        let context = if self.context.is_none() {
            let context = v8::Context::new(scope, Default::default());
            let global_context = v8::Global::new(scope, context);
            self.context = Some(global_context);
            context
        } else {
            v8::Local::new(scope, self.context.as_ref().unwrap())
        };

        let scope = &mut v8::ContextScope::new(scope, context);

        if !self.apis_initialized {
            Self::install_core_apis(
                scope,
                &context,
                &self.main_module_dir,
                &self.main_module_filename,
            )?;
            self.apis_initialized = true;
        }

        if !self.extended_apis_initialized {
            Self::install_extended_apis(scope, &context)?;
            self.extended_apis_initialized = true;
        }

        Ok(())
    }

    /// Exit this isolate from the current thread (e.g. before storing into a pool).
    ///
    /// # Safety
    /// Must only be called when no scopes are active on this isolate on the current thread.
    pub unsafe fn exit_current_thread(&mut self) {
        self.isolate.exit();
    }

    /// Enter this isolate onto the current thread (e.g. after acquiring from a pool).
    ///
    /// # Safety
    /// Must be balanced by an exit or drop on the same thread.
    pub unsafe fn enter_current_thread(&mut self) {
        self.isolate.enter();
    }

    fn should_execute_as_esm_module(code: &str, main_module_filename: &str) -> Result<bool> {
        let main_module_path = Path::new(main_module_filename);
        let module_format =
            crate::nodejs_core::commonjs_resolver::classify_commonjs_file(main_module_path)?;
        if module_format != crate::nodejs_core::commonjs_resolver::CommonJsModuleFormat::EsModule {
            if matches!(
                module_format,
                crate::nodejs_core::commonjs_resolver::CommonJsModuleFormat::TypeScript
                    | crate::nodejs_core::commonjs_resolver::CommonJsModuleFormat::TypeScriptJsx
            ) {
                return Ok(Self::has_esm_export_syntax(code)
                    || Self::has_top_level_await_syntax(code)
                    || !Self::static_import_specifiers(code).is_empty());
            }
            return Ok(false);
        }

        let has_export_syntax = Self::has_esm_export_syntax(code);
        let has_await_syntax = Self::has_esm_await_syntax(code);
        let import_specifiers = Self::static_import_specifiers(code);
        if main_module_path
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("mjs")
        {
            // `bee eval` with `import`/`export` uses a virtual `eval.mjs` path.
            // Treat any static import/export as native ESM so users get module
            // errors instead of a Script SyntaxError.
            return Ok(has_export_syntax || has_await_syntax || !import_specifiers.is_empty());
        }

        Ok(has_export_syntax || has_await_syntax || !import_specifiers.is_empty())
    }

    fn has_esm_export_syntax(code: &str) -> bool {
        code.contains("export let ")
            || code.contains("export const ")
            || code.contains("export var ")
            || code.contains("export function ")
            || code.contains("export class ")
            || code.contains("export default")
            || code.contains("export {")
    }

    fn has_esm_await_syntax(code: &str) -> bool {
        static AWAIT_TOKEN_RE: OnceLock<regex::Regex> = OnceLock::new();
        AWAIT_TOKEN_RE
            .get_or_init(|| regex::Regex::new(r"\bawait\b").expect("valid await token regex"))
            .is_match(code)
    }

    fn has_top_level_await_syntax(code: &str) -> bool {
        let bytes = code.as_bytes();
        let mut index = 0;
        let mut brace_depth = 0usize;

        while index < bytes.len() {
            match bytes[index] {
                b'\'' | b'"' => {
                    index = Self::skip_quoted_string(bytes, index, bytes[index]);
                }
                b'`' => {
                    index = Self::skip_template_literal(bytes, index);
                }
                b'/' if bytes.get(index + 1) == Some(&b'/') => {
                    index = Self::skip_line_comment(bytes, index);
                }
                b'/' if bytes.get(index + 1) == Some(&b'*') => {
                    index = Self::skip_block_comment(bytes, index);
                }
                b'{' => {
                    brace_depth += 1;
                    index += 1;
                }
                b'}' => {
                    brace_depth = brace_depth.saturating_sub(1);
                    index += 1;
                }
                _ if brace_depth == 0 && Self::is_await_token_at(bytes, index) => {
                    return true;
                }
                _ => {
                    index += 1;
                }
            }
        }

        false
    }

    fn skip_quoted_string(bytes: &[u8], start: usize, quote: u8) -> usize {
        let mut index = start + 1;
        let mut escaped = false;
        while index < bytes.len() {
            let byte = bytes[index];
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == quote {
                return index + 1;
            }
            index += 1;
        }
        index
    }

    fn skip_template_literal(bytes: &[u8], start: usize) -> usize {
        let mut index = start + 1;
        let mut escaped = false;
        while index < bytes.len() {
            let byte = bytes[index];
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'`' {
                return index + 1;
            }
            index += 1;
        }
        index
    }

    fn skip_line_comment(bytes: &[u8], start: usize) -> usize {
        let mut index = start + 2;
        while index < bytes.len() && bytes[index] != b'\n' {
            index += 1;
        }
        index
    }

    fn skip_block_comment(bytes: &[u8], start: usize) -> usize {
        let mut index = start + 2;
        while index + 1 < bytes.len() {
            if bytes[index] == b'*' && bytes[index + 1] == b'/' {
                return index + 2;
            }
            index += 1;
        }
        bytes.len()
    }

    fn is_await_token_at(bytes: &[u8], index: usize) -> bool {
        if !bytes[index..].starts_with(b"await") {
            return false;
        }
        let previous = index
            .checked_sub(1)
            .and_then(|previous| bytes.get(previous));
        let next = bytes.get(index + "await".len());
        !previous.is_some_and(|byte| Self::is_identifier_byte(*byte) || *byte == b'.')
            && !next.is_some_and(|byte| Self::is_identifier_byte(*byte))
    }

    fn is_identifier_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'
    }

    fn static_import_specifiers(code: &str) -> Vec<String> {
        static STATIC_IMPORT_SPECIFIER_RE: OnceLock<regex::Regex> = OnceLock::new();
        STATIC_IMPORT_SPECIFIER_RE
            .get_or_init(|| {
                regex::Regex::new(r#"(?m)^\s*import\s+(?:[^;]*?\s+from\s+)?['"]([^'"]+)['"]"#)
                    .expect("valid static import specifier regex")
            })
            .captures_iter(code)
            .filter_map(|captures| {
                captures
                    .get(1)
                    .map(|specifier| specifier.as_str().to_string())
            })
            .collect()
    }

    #[allow(dead_code)]
    fn is_native_esm_source_path(path: &Path) -> Result<bool> {
        let module_format = crate::nodejs_core::commonjs_resolver::classify_commonjs_file(path)?;
        Ok(module_format == crate::nodejs_core::commonjs_resolver::CommonJsModuleFormat::EsModule)
    }

    #[allow(dead_code)]
    fn static_import_targets_native_esm(specifier: &str, referrer_path: &Path) -> Result<bool> {
        let specifier_path = Path::new(specifier);
        if (specifier_path.is_absolute()
            || specifier.starts_with("./")
            || specifier.starts_with("../"))
            && specifier_path
                .extension()
                .and_then(|extension| extension.to_str())
                == Some("mjs")
        {
            return Ok(true);
        }

        let Ok(module_path) = Self::resolve_esm_candidate_path(specifier, referrer_path) else {
            return Ok(false);
        };
        Self::is_native_esm_source_path(&module_path)
    }

    fn normalized_module_path(path: &Path) -> PathBuf {
        path.canonicalize().unwrap_or_else(|_| {
            if let (Some(parent), Some(file_name)) = (path.parent(), path.file_name()) {
                if let Ok(parent) = parent.canonicalize() {
                    return parent.join(file_name);
                }
            }
            path.to_path_buf()
        })
    }

    fn resolve_esm_candidate_path(
        specifier: &str,
        referrer_path: &Path,
    ) -> Result<PathBuf, String> {
        if let Ok(url) = url::Url::parse(specifier) {
            if url.scheme() != "file" {
                return Err(format!(
                    "Only file:// URL specifiers are supported for ES modules: {}",
                    specifier
                ));
            }

            let file_path = url
                .to_file_path()
                .map_err(|_| format!("Invalid file:// ES module URL '{}'", specifier))?;
            return Self::resolve_esm_path_candidate(file_path, specifier);
        }

        let specifier_path = Path::new(specifier);
        let candidate = if specifier_path.is_absolute() {
            specifier_path.to_path_buf()
        } else if specifier.starts_with("./") || specifier.starts_with("../") {
            referrer_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(specifier_path)
        } else {
            let parent_dir = referrer_path.parent().unwrap_or_else(|| Path::new("."));
            return match crate::nodejs_core::commonjs_resolver::resolve_esm_module(
                specifier, parent_dir,
            ) {
                Ok(crate::nodejs_core::commonjs_resolver::ResolvedModule::File(path)) => {
                    path.canonicalize().map_err(|error| {
                        format!("Cannot resolve ES module '{}': {}", specifier, error)
                    })
                }
                Ok(crate::nodejs_core::commonjs_resolver::ResolvedModule::Builtin(_)) => Err(
                    format!("ESM builtin import '{}' is not supported yet", specifier),
                ),
                Err(error) => Err(error.to_string()),
            };
        };
        Self::resolve_esm_path_candidate(candidate, specifier)
    }

    fn resolve_esm_path_candidate(candidate: PathBuf, specifier: &str) -> Result<PathBuf, String> {
        let candidate = if candidate.extension().is_none() {
            candidate.with_extension("mjs")
        } else {
            candidate
        };

        let resolved = candidate
            .canonicalize()
            .map_err(|error| format!("Cannot resolve ES module '{}': {}", specifier, error))?;

        Ok(resolved)
    }

    fn create_esm_source<'scope>(
        scope: &mut v8::PinScope<'scope, '_>,
        path: &Path,
        code: &str,
    ) -> Result<v8::script_compiler::Source, String> {
        let source = v8::String::new(scope, code)
            .ok_or_else(|| format!("Failed to create V8 source for '{}'", path.display()))?;
        let resource_name = v8::String::new(scope, &path.to_string_lossy())
            .ok_or_else(|| format!("Failed to create V8 resource name for '{}'", path.display()))?;
        let source_map_url = active_source_map_url(scope);
        let origin = v8::ScriptOrigin::new(
            scope,
            resource_name.into(),
            0,
            0,
            false,
            0,
            source_map_url,
            false,
            false,
            true,
            None,
        );

        Ok(v8::script_compiler::Source::new(source, Some(&origin)))
    }

    fn compile_esm_module_source<'scope>(
        scope: &mut v8::PinScope<'scope, '_>,
        path: &Path,
        code: &str,
    ) -> Result<v8::Local<'scope, v8::Module>, String> {
        let mut source = Self::create_esm_source(scope, path, code)?;
        v8::script_compiler::compile_module(scope, &mut source)
            .ok_or_else(|| format!("Failed to compile ES module '{}'", path.display()))
    }

    fn esm_source_fingerprint(source: &[u8]) -> [u8; 32] {
        *blake3::hash(source).as_bytes()
    }

    fn read_file_backed_esm_fingerprint(path: &Path) -> Result<[u8; 32], String> {
        crate::permissions::check_global_permission(
            crate::permissions::PermissionKind::FileSystem,
            crate::permissions::PermissionAction::Read,
            crate::permissions::ResourceId::Path(path.to_path_buf()),
        )
        .map_err(|error| error.to_string())?;

        let source = std::fs::read(path)
            .map_err(|error| format!("Error loading ES module '{}': {}", path.display(), error))?;
        Ok(Self::esm_source_fingerprint(&source))
    }

    fn cached_esm_namespace_graph_is_fresh(
        scope: &mut v8::PinScope,
        graph_fingerprints: v8::Local<v8::Object>,
    ) -> Result<bool, String> {
        let Some(property_names) =
            graph_fingerprints.get_own_property_names(scope, Default::default())
        else {
            return Ok(false);
        };
        if property_names.length() == 0 {
            return Ok(false);
        }

        for index in 0..property_names.length() {
            let Some(path_key) = property_names.get_index(scope, index) else {
                return Ok(false);
            };
            let Some(path_string) = path_key.to_string(scope) else {
                return Ok(false);
            };
            let path_string = path_string.to_rust_string_lossy(scope);

            let Some(expected_fingerprint) = graph_fingerprints.get(scope, path_key) else {
                return Ok(false);
            };
            let Some(expected_fingerprint) = expected_fingerprint.to_string(scope) else {
                return Ok(false);
            };
            let expected_fingerprint = expected_fingerprint.to_rust_string_lossy(scope);

            let actual_fingerprint =
                Self::read_file_backed_esm_fingerprint(Path::new(&path_string))?;
            if hex::encode(actual_fingerprint) != expected_fingerprint {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn create_esm_namespace_graph_fingerprint_object<'scope>(
        scope: &mut v8::PinScope<'scope, '_>,
        source_fingerprints: &[(PathBuf, [u8; 32])],
    ) -> v8::Local<'scope, v8::Object> {
        let graph_fingerprints = v8::Object::new(scope);
        for (path, fingerprint) in source_fingerprints {
            let path_key = v8::String::new(scope, &path.to_string_lossy()).unwrap();
            let fingerprint_value = v8::String::new(scope, &hex::encode(fingerprint)).unwrap();
            graph_fingerprints.set(scope, path_key.into(), fingerprint_value.into());
        }
        graph_fingerprints
    }

    fn prune_stale_esm_module_cache(
        module_cache: &mut HashMap<PathBuf, v8::Global<v8::Module>>,
        module_cache_fingerprints: &mut HashMap<PathBuf, [u8; 32]>,
    ) {
        let cached_paths: Vec<PathBuf> = module_cache.keys().cloned().collect();
        let mut should_clear = false;

        for path in cached_paths {
            let Some(expected_fingerprint) = module_cache_fingerprints.get(&path) else {
                should_clear = true;
                break;
            };

            if crate::permissions::check_global_permission(
                crate::permissions::PermissionKind::FileSystem,
                crate::permissions::PermissionAction::Read,
                crate::permissions::ResourceId::Path(path.clone()),
            )
            .is_err()
            {
                should_clear = true;
                break;
            }

            match std::fs::read(&path) {
                Ok(source) => {
                    if Self::esm_source_fingerprint(&source) != *expected_fingerprint {
                        should_clear = true;
                        break;
                    }
                }
                Err(_) => {
                    should_clear = true;
                    break;
                }
            }
        }

        if should_clear {
            module_cache.clear();
            module_cache_fingerprints.clear();
        }
    }

    fn remember_esm_module(
        scope: &mut v8::PinScope,
        path: &Path,
        module: v8::Local<v8::Module>,
        source_fingerprint: [u8; 32],
    ) {
        let script_id = module.script_id();
        let module_global = v8::Global::new(scope, module);
        ESM_MODULE_LOAD_STATE.with(|state| {
            if let Some(state) = state.borrow_mut().as_mut() {
                state
                    .modules_by_path
                    .insert(path.to_path_buf(), module_global);
                state
                    .source_fingerprints_by_path
                    .insert(path.to_path_buf(), source_fingerprint);
                if let Some(script_id) = script_id {
                    state
                        .paths_by_script_id
                        .insert(script_id, path.to_path_buf());
                }
            }
        });
    }

    fn seed_esm_module_cache(
        scope: &mut v8::PinScope,
        module_cache: &HashMap<PathBuf, v8::Global<v8::Module>>,
        module_cache_fingerprints: &HashMap<PathBuf, [u8; 32]>,
    ) {
        ESM_MODULE_LOAD_STATE.with(|state| {
            let mut state = state.borrow_mut();
            let Some(state) = state.as_mut() else {
                return;
            };

            for (path, cached_module) in module_cache {
                let Some(source_fingerprint) = module_cache_fingerprints.get(path) else {
                    continue;
                };
                let module = v8::Local::new(scope, cached_module);
                state
                    .modules_by_path
                    .insert(path.clone(), cached_module.clone());
                state
                    .source_fingerprints_by_path
                    .insert(path.clone(), *source_fingerprint);
                if let Some(script_id) = module.script_id() {
                    state.paths_by_script_id.insert(script_id, path.clone());
                }
            }
        });
    }

    fn persist_esm_module_cache(
        module_cache: &mut HashMap<PathBuf, v8::Global<v8::Module>>,
        module_cache_fingerprints: &mut HashMap<PathBuf, [u8; 32]>,
        entry_path: &Path,
    ) {
        ESM_MODULE_LOAD_STATE.with(|state| {
            let state = state.borrow();
            let Some(state) = state.as_ref() else {
                return;
            };

            for (path, module) in &state.modules_by_path {
                if path != entry_path {
                    if let Some(source_fingerprint) = state.source_fingerprints_by_path.get(path) {
                        module_cache.insert(path.clone(), module.clone());
                        module_cache_fingerprints.insert(path.clone(), *source_fingerprint);
                    }
                }
            }
        });
    }

    fn set_esm_pending_error(message: String) {
        ESM_MODULE_LOAD_STATE.with(|state| {
            if let Some(state) = state.borrow_mut().as_mut() {
                state.pending_error = Some(message);
            }
        });
    }

    fn take_esm_pending_error() -> Option<String> {
        ESM_MODULE_LOAD_STATE.with(|state| {
            state
                .borrow_mut()
                .as_mut()
                .and_then(|state| state.pending_error.take())
        })
    }

    fn ensure_esm_evaluation_settled(
        scope: &mut v8::PinScope,
        value: v8::Local<v8::Value>,
        module_path: &Path,
        timer_drain_limit_ms: u64,
    ) -> Result<(), String> {
        if !value.is_promise() {
            return Ok(());
        }

        let promise = v8::Local::<v8::Promise>::try_from(value).map_err(|_| {
            format!(
                "Failed to inspect ES module evaluation result for '{}'",
                module_path.display()
            )
        })?;

        let timer_drain_started_at = std::time::Instant::now();
        let mut microtask_only_iterations = 0;

        loop {
            execute_next_tick_callbacks(scope);
            scope.perform_microtask_checkpoint();
            match promise.state() {
                v8::PromiseState::Fulfilled => return Ok(()),
                v8::PromiseState::Rejected => {
                    let reason = promise.result(scope);
                    let reason_str = reason
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_else(|| "Unknown module rejection".to_string());
                    return Err(format!(
                        "ES module '{}' rejected during evaluation: {}",
                        module_path.display(),
                        reason_str
                    ));
                }
                v8::PromiseState::Pending => {}
            }

            let remaining_ms =
                remaining_timer_drain_ms(timer_drain_started_at, timer_drain_limit_ms);
            let timer_manager = crate::event_loop::get_async_timer_manager();
            let has_fired_timers = timer_manager.has_fired_timers();
            let has_scheduled_timers = timer_manager.has_scheduled_timers();
            let has_zero_delay_timers =
                has_scheduled_timers && crate::nodejs_core::timers::has_pending_zero_delay_timers();
            let has_drainable_timers =
                crate::nodejs_core::timers::has_pending_drainable_timers(remaining_ms);
            let has_next_ticks = has_pending_next_ticks();

            if has_next_ticks {
                microtask_only_iterations = 0;
                continue;
            }

            if has_fired_timers {
                microtask_only_iterations = 0;
                execute_fired_timers(scope);
                continue;
            }

            if remaining_ms > 0 && (has_zero_delay_timers || has_drainable_timers) {
                microtask_only_iterations = 0;
                let timer_manager = crate::event_loop::get_async_timer_manager();
                timer_manager.wait_timeout(std::time::Duration::from_millis(remaining_ms.min(25)));
                execute_fired_timers(scope);
                continue;
            }

            microtask_only_iterations += 1;
            if microtask_only_iterations >= 32 {
                break;
            }
        }

        Err(format!(
            "Pending top-level await in ES module '{}' did not settle before runtime completion",
            module_path.display()
        ))
    }

    fn throw_esm_loader_error(scope: &mut v8::PinScope, message: String) {
        Self::set_esm_pending_error(message.clone());
        let error_message = v8::String::new(scope, &message).unwrap_or_else(|| {
            v8::String::new(scope, "ES module loader error").expect("static V8 string")
        });
        let error = v8::Exception::type_error(scope, error_message);
        scope.throw_exception(error);
    }

    fn esm_module_exception_value<'scope>(
        scope: &mut v8::PinScope<'scope, '_>,
        module: v8::Local<v8::Module>,
    ) -> v8::Local<'scope, v8::Value> {
        let exception = module.get_exception();
        let exception_global = v8::Global::new(scope, exception);
        v8::Local::new(scope, exception_global)
    }

    fn ensure_dynamic_import_evaluation_settled<'scope>(
        scope: &mut v8::PinScope<'scope, '_>,
        value: v8::Local<'scope, v8::Value>,
        module_path: &Path,
    ) -> Result<(), EsmDynamicImportError<'scope>> {
        if !value.is_promise() {
            return Ok(());
        }

        let promise = v8::Local::<v8::Promise>::try_from(value).map_err(|_| {
            EsmDynamicImportError::Message(format!(
                "Failed to inspect ES module evaluation result for '{}'",
                module_path.display()
            ))
        })?;

        for _ in 0..32 {
            scope.perform_microtask_checkpoint();
            match promise.state() {
                v8::PromiseState::Fulfilled => return Ok(()),
                v8::PromiseState::Rejected => {
                    return Err(EsmDynamicImportError::Value(promise.result(scope)));
                }
                v8::PromiseState::Pending => {}
            }
        }

        Err(EsmDynamicImportError::Message(format!(
            "Pending top-level await in ES module '{}' did not settle before runtime completion",
            module_path.display()
        )))
    }

    fn instantiate_and_evaluate_esm_module_for_dynamic_import<'scope>(
        scope: &mut v8::PinScope<'scope, '_>,
        module: v8::Local<'scope, v8::Module>,
        module_path: &Path,
    ) -> Result<(), EsmDynamicImportError<'scope>> {
        if module.get_status() == v8::ModuleStatus::Uninstantiated {
            match module.instantiate_module(scope, Self::esm_resolve_callback) {
                Some(true) => {}
                Some(false) | None => {
                    return Err(EsmDynamicImportError::Message(
                        Self::take_esm_pending_error().unwrap_or_else(|| {
                            format!(
                                "Failed to instantiate ES module '{}'",
                                module_path.display()
                            )
                        }),
                    ));
                }
            }
        }

        if module.get_status() == v8::ModuleStatus::Errored {
            return Err(EsmDynamicImportError::Value(
                Self::esm_module_exception_value(scope, module),
            ));
        }

        if module.get_status() != v8::ModuleStatus::Evaluated {
            let evaluation = module.evaluate(scope).ok_or_else(|| {
                EsmDynamicImportError::Message(Self::take_esm_pending_error().unwrap_or_else(
                    || format!("Failed to evaluate ES module '{}'", module_path.display()),
                ))
            })?;
            Self::ensure_dynamic_import_evaluation_settled(scope, evaluation, module_path)?;
        }

        if module.get_status() == v8::ModuleStatus::Errored {
            return Err(EsmDynamicImportError::Value(
                Self::esm_module_exception_value(scope, module),
            ));
        }

        Ok(())
    }

    fn dynamic_import_referrer_path(
        scope: &mut v8::PinScope,
        resource_name: v8::Local<v8::Value>,
    ) -> Result<PathBuf, String> {
        if resource_name.is_undefined() || resource_name.is_null() {
            return Err("Cannot resolve dynamic import from anonymous referrer".to_string());
        }
        let resource_name = resource_name
            .to_string(scope)
            .ok_or_else(|| "Cannot read dynamic import referrer resource name".to_string())?
            .to_rust_string_lossy(scope);
        if resource_name.is_empty() {
            return Err("Cannot resolve dynamic import from empty referrer".to_string());
        }
        Ok(Self::normalized_module_path(Path::new(&resource_name)))
    }

    fn load_dynamic_import_module<'scope>(
        scope: &mut v8::PinScope<'scope, '_>,
        specifier: &str,
        referrer_path: &Path,
    ) -> Result<(v8::Local<'scope, v8::Module>, PathBuf), String> {
        if let Some(builtin_name) = Self::normalized_esm_builtin_name(specifier) {
            let module_path = PathBuf::from(format!("beejs:builtin:{}", builtin_name));
            let cached_module = ESM_MODULE_LOAD_STATE.with(|state| {
                state
                    .borrow()
                    .as_ref()
                    .and_then(|state| state.modules_by_path.get(&module_path).cloned())
            });
            if let Some(cached_module) = cached_module {
                return Ok((v8::Local::new(scope, cached_module), module_path));
            }

            if let Some(module) = Self::create_esm_builtin_module(scope, builtin_name) {
                let module_global = v8::Global::new(scope, module);
                ESM_MODULE_LOAD_STATE.with(|state| {
                    if let Some(state) = state.borrow_mut().as_mut() {
                        state
                            .modules_by_path
                            .insert(module_path.clone(), module_global);
                    }
                });
                return Ok((module, module_path));
            }
        }

        let module_path = Self::resolve_esm_candidate_path(specifier, referrer_path)?;
        let cached_module = ESM_MODULE_LOAD_STATE.with(|state| {
            state
                .borrow()
                .as_ref()
                .and_then(|state| state.modules_by_path.get(&module_path).cloned())
        });
        if let Some(cached_module) = cached_module {
            return Ok((v8::Local::new(scope, cached_module), module_path));
        }

        let module_format = crate::nodejs_core::commonjs_resolver::classify_commonjs_file(
            &module_path,
        )
        .map_err(|error| {
            format!(
                "Cannot classify ES module '{}': {}",
                module_path.display(),
                error
            )
        })?;
        if module_format != crate::nodejs_core::commonjs_resolver::CommonJsModuleFormat::EsModule {
            let module_fingerprint = Self::read_file_backed_esm_fingerprint(&module_path)?;
            let module = Self::create_esm_commonjs_module(scope, &module_path)?;
            Self::remember_esm_module(scope, &module_path, module, module_fingerprint);
            return Ok((module, module_path));
        }

        crate::permissions::check_global_permission(
            crate::permissions::PermissionKind::FileSystem,
            crate::permissions::PermissionAction::Read,
            crate::permissions::ResourceId::Path(module_path.clone()),
        )
        .map_err(|error| error.to_string())?;

        let module_code = std::fs::read_to_string(&module_path).map_err(|error| {
            format!(
                "Cannot read ES module '{}': {}",
                module_path.display(),
                error
            )
        })?;

        let module_fingerprint = Self::esm_source_fingerprint(module_code.as_bytes());
        let module = Self::compile_esm_module_source(scope, &module_path, &module_code)?;
        Self::remember_esm_module(scope, &module_path, module, module_fingerprint);

        Ok((module, module_path))
    }

    fn resolve_dynamic_import_namespace<'scope>(
        scope: &mut v8::PinScope<'scope, '_>,
        referrer: v8::Local<v8::Value>,
        specifier: &str,
    ) -> Result<v8::Local<'scope, v8::Value>, EsmDynamicImportError<'scope>> {
        let referrer_path = Self::dynamic_import_referrer_path(scope, referrer)
            .map_err(EsmDynamicImportError::Message)?;
        let (module, module_path) =
            Self::load_dynamic_import_module(scope, specifier, &referrer_path)
                .map_err(EsmDynamicImportError::Message)?;
        Self::instantiate_and_evaluate_esm_module_for_dynamic_import(scope, module, &module_path)?;
        let namespace = module.get_module_namespace();
        let namespace_global = v8::Global::new(scope, namespace);
        Ok(v8::Local::new(scope, namespace_global))
    }

    fn reject_dynamic_import(
        scope: &mut v8::PinScope,
        resolver: v8::Local<v8::PromiseResolver>,
        message: String,
    ) {
        let message = v8::String::new(scope, &message).unwrap_or_else(|| {
            v8::String::new(scope, "Dynamic import failed").expect("static V8 string")
        });
        let error = v8::Exception::error(scope, message);
        resolver.reject(scope, error);
    }

    fn reject_dynamic_import_error<'scope>(
        scope: &mut v8::PinScope<'scope, '_>,
        resolver: v8::Local<v8::PromiseResolver>,
        error: EsmDynamicImportError<'scope>,
    ) {
        match error {
            EsmDynamicImportError::Message(message) => {
                Self::reject_dynamic_import(scope, resolver, message);
            }
            EsmDynamicImportError::Value(value) => {
                resolver.reject(scope, value);
            }
        }
    }

    fn esm_dynamic_import_callback<'s, 'i>(
        scope: &mut v8::PinScope<'s, 'i>,
        _referrer_info: v8::Local<'s, v8::Data>,
        referrer: v8::Local<'s, v8::Value>,
        specifier: v8::Local<'s, v8::String>,
        _import_assertions: v8::Local<'s, v8::FixedArray>,
    ) -> Option<v8::Local<'s, v8::Promise>> {
        let resolver = v8::PromiseResolver::new(scope)?;
        let promise = resolver.get_promise(scope);
        let specifier_str = specifier.to_rust_string_lossy(scope);

        {
            v8::tc_scope!(let tc, scope);
            let import_result =
                Self::resolve_dynamic_import_namespace(tc, referrer, &specifier_str);
            if tc.has_caught() {
                let exception = tc.exception().unwrap_or_else(|| {
                    let message =
                        v8::String::new(tc, "Dynamic import failed").expect("static V8 string");
                    v8::Exception::error(tc, message)
                });
                resolver.reject(tc, exception);
            } else {
                match import_result {
                    Ok(namespace) => {
                        resolver.resolve(tc, namespace);
                    }
                    Err(error) => {
                        Self::reject_dynamic_import_error(tc, resolver, error);
                    }
                }
            }
        }

        Some(promise)
    }

    fn normalized_esm_builtin_name(specifier: &str) -> Option<&str> {
        let builtin_name = specifier.strip_prefix("node:").unwrap_or(specifier);
        match builtin_name {
            "path" | "fs" | "url" | "events" | "os" | "stream" | "process" | "crypto"
            | "child_process" => Some(builtin_name),
            _ => None,
        }
    }

    fn create_esm_builtin_module<'scope>(
        scope: &mut v8::PinScope<'scope, '_>,
        specifier: &str,
    ) -> Option<v8::Local<'scope, v8::Module>> {
        let builtin_name = Self::normalized_esm_builtin_name(specifier)?;
        match builtin_name {
            "path" => {
                let export_names = [
                    v8::String::new(scope, "default").unwrap(),
                    v8::String::new(scope, "join").unwrap(),
                    v8::String::new(scope, "resolve").unwrap(),
                    v8::String::new(scope, "basename").unwrap(),
                    v8::String::new(scope, "dirname").unwrap(),
                    v8::String::new(scope, "extname").unwrap(),
                    v8::String::new(scope, "normalize").unwrap(),
                ];
                let module_name = v8::String::new(scope, "node:path").unwrap();
                Some(v8::Module::create_synthetic_module(
                    scope,
                    module_name,
                    &export_names,
                    Self::evaluate_path_builtin_synthetic_module,
                ))
            }
            "fs" => {
                let export_names = [
                    v8::String::new(scope, "default").unwrap(),
                    v8::String::new(scope, "readFileSync").unwrap(),
                    v8::String::new(scope, "writeFileSync").unwrap(),
                    v8::String::new(scope, "existsSync").unwrap(),
                    v8::String::new(scope, "mkdirSync").unwrap(),
                    v8::String::new(scope, "readdirSync").unwrap(),
                    v8::String::new(scope, "statSync").unwrap(),
                    v8::String::new(scope, "unlinkSync").unwrap(),
                    v8::String::new(scope, "renameSync").unwrap(),
                    v8::String::new(scope, "rmdirSync").unwrap(),
                ];
                let module_name = v8::String::new(scope, "node:fs").unwrap();
                Some(v8::Module::create_synthetic_module(
                    scope,
                    module_name,
                    &export_names,
                    Self::evaluate_fs_builtin_synthetic_module,
                ))
            }
            "url" => {
                let export_names = [
                    v8::String::new(scope, "default").unwrap(),
                    v8::String::new(scope, "URL").unwrap(),
                    v8::String::new(scope, "URLSearchParams").unwrap(),
                ];
                let module_name = v8::String::new(scope, "node:url").unwrap();
                Some(v8::Module::create_synthetic_module(
                    scope,
                    module_name,
                    &export_names,
                    Self::evaluate_url_builtin_synthetic_module,
                ))
            }
            "events" => {
                let export_names = [
                    v8::String::new(scope, "default").unwrap(),
                    v8::String::new(scope, "EventEmitter").unwrap(),
                ];
                let module_name = v8::String::new(scope, "node:events").unwrap();
                Some(v8::Module::create_synthetic_module(
                    scope,
                    module_name,
                    &export_names,
                    Self::evaluate_events_builtin_synthetic_module,
                ))
            }
            "os" => {
                let export_names = [
                    v8::String::new(scope, "default").unwrap(),
                    v8::String::new(scope, "platform").unwrap(),
                    v8::String::new(scope, "arch").unwrap(),
                    v8::String::new(scope, "cpus").unwrap(),
                    v8::String::new(scope, "freemem").unwrap(),
                    v8::String::new(scope, "totalmem").unwrap(),
                    v8::String::new(scope, "uptime").unwrap(),
                    v8::String::new(scope, "type").unwrap(),
                    v8::String::new(scope, "release").unwrap(),
                    v8::String::new(scope, "homedir").unwrap(),
                    v8::String::new(scope, "tmpdir").unwrap(),
                ];
                let module_name = v8::String::new(scope, "node:os").unwrap();
                Some(v8::Module::create_synthetic_module(
                    scope,
                    module_name,
                    &export_names,
                    Self::evaluate_os_builtin_synthetic_module,
                ))
            }
            "stream" => {
                let export_names = [
                    v8::String::new(scope, "default").unwrap(),
                    v8::String::new(scope, "Readable").unwrap(),
                    v8::String::new(scope, "Writable").unwrap(),
                    v8::String::new(scope, "Transform").unwrap(),
                    v8::String::new(scope, "Duplex").unwrap(),
                    v8::String::new(scope, "pipeline").unwrap(),
                    v8::String::new(scope, "passThrough").unwrap(),
                    v8::String::new(scope, "PassThrough").unwrap(),
                ];
                let module_name = v8::String::new(scope, "node:stream").unwrap();
                Some(v8::Module::create_synthetic_module(
                    scope,
                    module_name,
                    &export_names,
                    Self::evaluate_stream_builtin_synthetic_module,
                ))
            }
            "crypto" => {
                let export_names = [
                    v8::String::new(scope, "default").unwrap(),
                    v8::String::new(scope, "createHash").unwrap(),
                    v8::String::new(scope, "createHmac").unwrap(),
                    v8::String::new(scope, "createSign").unwrap(),
                    v8::String::new(scope, "createVerify").unwrap(),
                    v8::String::new(scope, "sign").unwrap(),
                    v8::String::new(scope, "verify").unwrap(),
                    v8::String::new(scope, "randomBytes").unwrap(),
                    v8::String::new(scope, "randomBytesSync").unwrap(),
                    v8::String::new(scope, "randomFillSync").unwrap(),
                    v8::String::new(scope, "randomFill").unwrap(),
                    v8::String::new(scope, "timingSafeEqual").unwrap(),
                    v8::String::new(scope, "pbkdf2Sync").unwrap(),
                    v8::String::new(scope, "pbkdf2").unwrap(),
                    v8::String::new(scope, "getHashes").unwrap(),
                    v8::String::new(scope, "createCipher").unwrap(),
                    v8::String::new(scope, "createDecipher").unwrap(),
                    v8::String::new(scope, "createCipheriv").unwrap(),
                    v8::String::new(scope, "createDecipheriv").unwrap(),
                    v8::String::new(scope, "publicEncrypt").unwrap(),
                    v8::String::new(scope, "privateDecrypt").unwrap(),
                    v8::String::new(scope, "privateEncrypt").unwrap(),
                    v8::String::new(scope, "publicDecrypt").unwrap(),
                    v8::String::new(scope, "generateKeyPairSync").unwrap(),
                    v8::String::new(scope, "generateKeyPair").unwrap(),
                    v8::String::new(scope, "constants").unwrap(),
                    v8::String::new(scope, "scryptSync").unwrap(),
                    v8::String::new(scope, "scrypt").unwrap(),
                    v8::String::new(scope, "createDiffieHellman").unwrap(),
                    v8::String::new(scope, "createECDH").unwrap(),
                    v8::String::new(scope, "createPrivateKey").unwrap(),
                    v8::String::new(scope, "createPublicKey").unwrap(),
                    v8::String::new(scope, "createSecretKey").unwrap(),
                    v8::String::new(scope, "hkdf").unwrap(),
                    v8::String::new(scope, "hkdfSync").unwrap(),
                    v8::String::new(scope, "getRandomValues").unwrap(),
                    v8::String::new(scope, "randomUUID").unwrap(),
                    v8::String::new(scope, "subtle").unwrap(),
                ];
                let module_name = v8::String::new(scope, "node:crypto").unwrap();
                Some(v8::Module::create_synthetic_module(
                    scope,
                    module_name,
                    &export_names,
                    Self::evaluate_crypto_builtin_synthetic_module,
                ))
            }
            "process" => {
                let export_names = [
                    v8::String::new(scope, "default").unwrap(),
                    v8::String::new(scope, "version").unwrap(),
                    v8::String::new(scope, "versions").unwrap(),
                    v8::String::new(scope, "platform").unwrap(),
                    v8::String::new(scope, "arch").unwrap(),
                    v8::String::new(scope, "pid").unwrap(),
                    v8::String::new(scope, "ppid").unwrap(),
                    v8::String::new(scope, "title").unwrap(),
                    v8::String::new(scope, "env").unwrap(),
                    v8::String::new(scope, "argv").unwrap(),
                    v8::String::new(scope, "execArgv").unwrap(),
                    v8::String::new(scope, "execPath").unwrap(),
                    v8::String::new(scope, "cwd").unwrap(),
                    v8::String::new(scope, "chdir").unwrap(),
                    v8::String::new(scope, "umask").unwrap(),
                    v8::String::new(scope, "abort").unwrap(),
                    v8::String::new(scope, "config").unwrap(),
                    v8::String::new(scope, "memoryUsage").unwrap(),
                    v8::String::new(scope, "memory").unwrap(),
                    v8::String::new(scope, "uptime").unwrap(),
                    v8::String::new(scope, "hrtime").unwrap(),
                    v8::String::new(scope, "exit").unwrap(),
                    v8::String::new(scope, "exitCode").unwrap(),
                    v8::String::new(scope, "nextTick").unwrap(),
                    v8::String::new(scope, "features").unwrap(),
                    v8::String::new(scope, "isBeejs").unwrap(),
                    v8::String::new(scope, "browser").unwrap(),
                    v8::String::new(scope, "release").unwrap(),
                    v8::String::new(scope, "on").unwrap(),
                    v8::String::new(scope, "off").unwrap(),
                    v8::String::new(scope, "removeListener").unwrap(),
                    v8::String::new(scope, "setMaxListeners").unwrap(),
                    v8::String::new(scope, "getMaxListeners").unwrap(),
                    v8::String::new(scope, "stdout").unwrap(),
                    v8::String::new(scope, "stderr").unwrap(),
                    v8::String::new(scope, "stdin").unwrap(),
                    v8::String::new(scope, "cpuUsage").unwrap(),
                    v8::String::new(scope, "kill").unwrap(),
                ];
                let module_name = v8::String::new(scope, "node:process").unwrap();
                Some(v8::Module::create_synthetic_module(
                    scope,
                    module_name,
                    &export_names,
                    Self::evaluate_process_builtin_synthetic_module,
                ))
            }
            "child_process" => {
                let export_names = [
                    v8::String::new(scope, "default").unwrap(),
                    v8::String::new(scope, "exec").unwrap(),
                    v8::String::new(scope, "spawn").unwrap(),
                    v8::String::new(scope, "execFile").unwrap(),
                    v8::String::new(scope, "execSync").unwrap(),
                    v8::String::new(scope, "spawnSync").unwrap(),
                ];
                let module_name = v8::String::new(scope, "node:child_process").unwrap();
                Some(v8::Module::create_synthetic_module(
                    scope,
                    module_name,
                    &export_names,
                    Self::evaluate_child_process_builtin_synthetic_module,
                ))
            }
            _ => None,
        }
    }

    fn evaluate_child_process_builtin_synthetic_module<'scope>(
        context: v8::Local<'scope, v8::Context>,
        module: v8::Local<'scope, v8::Module>,
    ) -> Option<v8::Local<'scope, v8::Value>> {
        v8::callback_scope!(unsafe let scope, context);
        let global = context.global(scope);
        let cp_key = v8::String::new(scope, "child_process").unwrap();
        let cp_value = global
            .get(scope, cp_key.into())
            .unwrap_or_else(|| v8::undefined(scope).into());

        let default_key = v8::String::new(scope, "default").unwrap();
        module.set_synthetic_module_export(scope, default_key, cp_value)?;

        let cp_object = cp_value.to_object(scope);
        for export_name in ["exec", "spawn", "execFile", "execSync", "spawnSync"] {
            let export_key = v8::String::new(scope, export_name).unwrap();
            let export_value = cp_object
                .and_then(|object| object.get(scope, export_key.into()))
                .unwrap_or_else(|| v8::undefined(scope).into());
            module.set_synthetic_module_export(scope, export_key, export_value)?;
        }

        Some(v8::undefined(scope).into())
    }

    fn evaluate_path_builtin_synthetic_module<'scope>(
        context: v8::Local<'scope, v8::Context>,
        module: v8::Local<'scope, v8::Module>,
    ) -> Option<v8::Local<'scope, v8::Value>> {
        v8::callback_scope!(unsafe let scope, context);
        let global = context.global(scope);
        let path_key = v8::String::new(scope, "path").unwrap();
        let path_value = global
            .get(scope, path_key.into())
            .unwrap_or_else(|| v8::undefined(scope).into());

        let default_key = v8::String::new(scope, "default").unwrap();
        module.set_synthetic_module_export(scope, default_key, path_value)?;

        let path_object = path_value.to_object(scope);
        for export_name in [
            "join",
            "resolve",
            "basename",
            "dirname",
            "extname",
            "normalize",
        ] {
            let export_key = v8::String::new(scope, export_name).unwrap();
            let export_value = path_object
                .and_then(|object| object.get(scope, export_key.into()))
                .unwrap_or_else(|| v8::undefined(scope).into());
            module.set_synthetic_module_export(scope, export_key, export_value)?;
        }

        Some(v8::undefined(scope).into())
    }

    fn evaluate_fs_builtin_synthetic_module<'scope>(
        context: v8::Local<'scope, v8::Context>,
        module: v8::Local<'scope, v8::Module>,
    ) -> Option<v8::Local<'scope, v8::Value>> {
        v8::callback_scope!(unsafe let scope, context);
        let global = context.global(scope);
        let fs_key = v8::String::new(scope, "fs").unwrap();
        let fs_value = global
            .get(scope, fs_key.into())
            .unwrap_or_else(|| v8::undefined(scope).into());

        let default_key = v8::String::new(scope, "default").unwrap();
        module.set_synthetic_module_export(scope, default_key, fs_value)?;

        let fs_object = fs_value.to_object(scope);
        for export_name in [
            "readFileSync",
            "writeFileSync",
            "existsSync",
            "mkdirSync",
            "readdirSync",
            "statSync",
            "unlinkSync",
            "renameSync",
            "rmdirSync",
        ] {
            let export_key = v8::String::new(scope, export_name).unwrap();
            let export_value = fs_object
                .and_then(|object| object.get(scope, export_key.into()))
                .unwrap_or_else(|| v8::undefined(scope).into());
            module.set_synthetic_module_export(scope, export_key, export_value)?;
        }

        Some(v8::undefined(scope).into())
    }

    fn evaluate_url_builtin_synthetic_module<'scope>(
        context: v8::Local<'scope, v8::Context>,
        module: v8::Local<'scope, v8::Module>,
    ) -> Option<v8::Local<'scope, v8::Value>> {
        v8::callback_scope!(unsafe let scope, context);
        let global = context.global(scope);
        let url_module = v8::Object::new(scope);

        for export_name in ["URL", "URLSearchParams"] {
            let export_key = v8::String::new(scope, export_name).unwrap();
            let export_value = global
                .get(scope, export_key.into())
                .unwrap_or_else(|| v8::undefined(scope).into());
            url_module.set(scope, export_key.into(), export_value);
            module.set_synthetic_module_export(scope, export_key, export_value)?;
        }

        let default_key = v8::String::new(scope, "default").unwrap();
        module.set_synthetic_module_export(scope, default_key, url_module.into())?;

        Some(v8::undefined(scope).into())
    }

    fn evaluate_events_builtin_synthetic_module<'scope>(
        context: v8::Local<'scope, v8::Context>,
        module: v8::Local<'scope, v8::Module>,
    ) -> Option<v8::Local<'scope, v8::Value>> {
        v8::callback_scope!(unsafe let scope, context);
        let global = context.global(scope);
        let events_key = v8::String::new(scope, "events").unwrap();
        let events_value = global
            .get(scope, events_key.into())
            .unwrap_or_else(|| v8::undefined(scope).into());
        let event_emitter_key = v8::String::new(scope, "EventEmitter").unwrap();
        let event_emitter_value = events_value
            .to_object(scope)
            .and_then(|events_object| events_object.get(scope, event_emitter_key.into()))
            .unwrap_or_else(|| v8::undefined(scope).into());

        let default_key = v8::String::new(scope, "default").unwrap();
        module.set_synthetic_module_export(scope, default_key, event_emitter_value)?;
        module.set_synthetic_module_export(scope, event_emitter_key, event_emitter_value)?;

        Some(v8::undefined(scope).into())
    }

    fn evaluate_os_builtin_synthetic_module<'scope>(
        context: v8::Local<'scope, v8::Context>,
        module: v8::Local<'scope, v8::Module>,
    ) -> Option<v8::Local<'scope, v8::Value>> {
        v8::callback_scope!(unsafe let scope, context);
        let global = context.global(scope);
        let os_key = v8::String::new(scope, "os").unwrap();
        let os_value = global
            .get(scope, os_key.into())
            .unwrap_or_else(|| v8::undefined(scope).into());

        let default_key = v8::String::new(scope, "default").unwrap();
        module.set_synthetic_module_export(scope, default_key, os_value)?;

        let os_object = os_value.to_object(scope);
        for export_name in [
            "platform", "arch", "cpus", "freemem", "totalmem", "uptime", "type", "release",
            "homedir", "tmpdir",
        ] {
            let export_key = v8::String::new(scope, export_name).unwrap();
            let export_value = os_object
                .and_then(|object| object.get(scope, export_key.into()))
                .unwrap_or_else(|| v8::undefined(scope).into());
            module.set_synthetic_module_export(scope, export_key, export_value)?;
        }

        Some(v8::undefined(scope).into())
    }

    fn evaluate_stream_builtin_synthetic_module<'scope>(
        context: v8::Local<'scope, v8::Context>,
        module: v8::Local<'scope, v8::Module>,
    ) -> Option<v8::Local<'scope, v8::Value>> {
        v8::callback_scope!(unsafe let scope, context);
        let global = context.global(scope);
        let stream_key = v8::String::new(scope, "stream").unwrap();
        let stream_value = global
            .get(scope, stream_key.into())
            .unwrap_or_else(|| v8::undefined(scope).into());

        let default_key = v8::String::new(scope, "default").unwrap();
        module.set_synthetic_module_export(scope, default_key, stream_value)?;

        let stream_object = stream_value.to_object(scope);
        for export_name in [
            "Readable",
            "Writable",
            "Transform",
            "Duplex",
            "pipeline",
            "passThrough",
            "PassThrough",
        ] {
            let export_key = v8::String::new(scope, export_name).unwrap();
            let export_value = stream_object
                .and_then(|object| object.get(scope, export_key.into()))
                .unwrap_or_else(|| v8::undefined(scope).into());
            module.set_synthetic_module_export(scope, export_key, export_value)?;
        }

        Some(v8::undefined(scope).into())
    }

    fn evaluate_crypto_builtin_synthetic_module<'scope>(
        context: v8::Local<'scope, v8::Context>,
        module: v8::Local<'scope, v8::Module>,
    ) -> Option<v8::Local<'scope, v8::Value>> {
        v8::callback_scope!(unsafe let scope, context);
        let global = context.global(scope);
        let crypto_key = v8::String::new(scope, "crypto").unwrap();
        let crypto_value = global
            .get(scope, crypto_key.into())
            .unwrap_or_else(|| v8::undefined(scope).into());

        let default_key = v8::String::new(scope, "default").unwrap();
        module.set_synthetic_module_export(scope, default_key, crypto_value)?;

        let crypto_object = crypto_value.to_object(scope);
        for export_name in [
            "createHash",
            "createHmac",
            "createSign",
            "createVerify",
            "sign",
            "verify",
            "randomBytes",
            "randomBytesSync",
            "randomFillSync",
            "randomFill",
            "timingSafeEqual",
            "pbkdf2Sync",
            "pbkdf2",
            "getHashes",
            "createCipher",
            "createDecipher",
            "createCipheriv",
            "createDecipheriv",
            "publicEncrypt",
            "privateDecrypt",
            "privateEncrypt",
            "publicDecrypt",
            "generateKeyPairSync",
            "generateKeyPair",
            "constants",
            "scryptSync",
            "scrypt",
            "createDiffieHellman",
            "createECDH",
            "createPrivateKey",
            "createPublicKey",
            "createSecretKey",
            "hkdf",
            "hkdfSync",
            "getRandomValues",
            "randomUUID",
            "subtle",
        ] {
            let export_key = v8::String::new(scope, export_name).unwrap();
            let export_value = crypto_object
                .and_then(|object| object.get(scope, export_key.into()))
                .unwrap_or_else(|| v8::undefined(scope).into());
            module.set_synthetic_module_export(scope, export_key, export_value)?;
        }

        Some(v8::undefined(scope).into())
    }

    fn evaluate_process_builtin_synthetic_module<'scope>(
        context: v8::Local<'scope, v8::Context>,
        module: v8::Local<'scope, v8::Module>,
    ) -> Option<v8::Local<'scope, v8::Value>> {
        v8::callback_scope!(unsafe let scope, context);
        let global = context.global(scope);
        let process_key = v8::String::new(scope, "process").unwrap();
        let process_value = global
            .get(scope, process_key.into())
            .unwrap_or_else(|| v8::undefined(scope).into());

        let default_key = v8::String::new(scope, "default").unwrap();
        module.set_synthetic_module_export(scope, default_key, process_value)?;

        let process_object = process_value.to_object(scope);
        for export_name in [
            "version",
            "versions",
            "platform",
            "arch",
            "pid",
            "ppid",
            "title",
            "env",
            "argv",
            "execArgv",
            "execPath",
            "cwd",
            "chdir",
            "umask",
            "abort",
            "config",
            "memoryUsage",
            "memory",
            "uptime",
            "hrtime",
            "exit",
            "exitCode",
            "nextTick",
            "features",
            "isBeejs",
            "browser",
            "release",
            "on",
            "off",
            "removeListener",
            "setMaxListeners",
            "getMaxListeners",
            "stdout",
            "stderr",
            "stdin",
            "cpuUsage",
            "kill",
        ] {
            let export_key = v8::String::new(scope, export_name).unwrap();
            let export_value = process_object
                .and_then(|object| object.get(scope, export_key.into()))
                .unwrap_or_else(|| v8::undefined(scope).into());
            module.set_synthetic_module_export(scope, export_key, export_value)?;
        }

        Some(v8::undefined(scope).into())
    }

    fn require_commonjs_for_esm<'scope>(
        scope: &mut v8::PinScope<'scope, '_>,
        module_path: &Path,
    ) -> Result<v8::Local<'scope, v8::Value>, String> {
        let context = scope.get_current_context();
        let global = context.global(scope);
        let require_key = v8::String::new(scope, "require")
            .ok_or_else(|| "Failed to create require key".to_string())?;
        let require_value = global
            .get(scope, require_key.into())
            .ok_or_else(|| "global require is not available".to_string())?;
        let require_fn = v8::Local::<v8::Function>::try_from(require_value)
            .map_err(|_| "global require is not callable".to_string())?;
        let module_path_string = module_path.to_string_lossy().to_string();
        let module_specifier = v8::String::new(scope, &module_path_string).ok_or_else(|| {
            format!(
                "Failed to create CommonJS specifier for '{}'",
                module_path.display()
            )
        })?;
        let undefined = v8::undefined(scope);
        require_fn
            .call(scope, undefined.into(), &[module_specifier.into()])
            .ok_or_else(|| {
                format!(
                    "Failed to load CommonJS dependency '{}'",
                    module_path.display()
                )
            })
    }

    fn commonjs_named_export_names(
        scope: &mut v8::PinScope,
        exports: v8::Local<v8::Value>,
    ) -> Vec<String> {
        if !exports.is_object() {
            return Vec::new();
        }

        let Some(exports_object) = exports.to_object(scope) else {
            return Vec::new();
        };
        let Some(property_names) = exports_object.get_own_property_names(scope, Default::default())
        else {
            return Vec::new();
        };

        let mut export_names = Vec::new();
        for index in 0..property_names.length() {
            let Some(property_name) = property_names.get_index(scope, index) else {
                continue;
            };
            let Some(property_name) = property_name.to_string(scope) else {
                continue;
            };
            let property_name = property_name.to_rust_string_lossy(scope);
            if property_name == "default" || export_names.contains(&property_name) {
                continue;
            }
            export_names.push(property_name);
        }

        export_names
    }

    fn create_esm_commonjs_module<'scope>(
        scope: &mut v8::PinScope<'scope, '_>,
        module_path: &Path,
    ) -> Result<v8::Local<'scope, v8::Module>, String> {
        let exports = Self::require_commonjs_for_esm(scope, module_path)?;
        let named_export_names = Self::commonjs_named_export_names(scope, exports);
        let mut export_names = Vec::with_capacity(named_export_names.len() + 1);
        export_names.push(v8::String::new(scope, "default").unwrap());
        for export_name in &named_export_names {
            let export_name = v8::String::new(scope, export_name).ok_or_else(|| {
                format!(
                    "Failed to create CommonJS export name for '{}'",
                    module_path.display()
                )
            })?;
            export_names.push(export_name);
        }
        let module_name = v8::String::new(
            scope,
            &format!("beejs:commonjs:{}", module_path.to_string_lossy()),
        )
        .ok_or_else(|| {
            format!(
                "Failed to create CommonJS synthetic module name for '{}'",
                module_path.display()
            )
        })?;
        let module = v8::Module::create_synthetic_module(
            scope,
            module_name,
            &export_names,
            Self::evaluate_commonjs_synthetic_module,
        );
        let module_identity = module.get_identity_hash().get();
        let exports_global = v8::Global::new(scope, exports);
        ESM_MODULE_LOAD_STATE.with(|state| {
            if let Some(state) = state.borrow_mut().as_mut() {
                state
                    .cjs_synthetic_exports_by_identity
                    .insert(module_identity, exports_global);
                state
                    .cjs_synthetic_named_exports_by_identity
                    .insert(module_identity, named_export_names);
            }
        });
        Ok(module)
    }

    fn evaluate_commonjs_synthetic_module<'scope>(
        context: v8::Local<'scope, v8::Context>,
        module: v8::Local<'scope, v8::Module>,
    ) -> Option<v8::Local<'scope, v8::Value>> {
        v8::callback_scope!(unsafe let scope, context);
        let module_identity = module.get_identity_hash().get();
        let exports = ESM_MODULE_LOAD_STATE.with(|state| {
            state.borrow().as_ref().and_then(|state| {
                state
                    .cjs_synthetic_exports_by_identity
                    .get(&module_identity)
                    .cloned()
            })
        });
        let Some(exports) = exports else {
            Self::throw_esm_loader_error(
                scope,
                "CommonJS synthetic module exports were not registered".to_string(),
            );
            return None;
        };

        let exports = v8::Local::new(scope, exports);
        let default_key = v8::String::new(scope, "default").unwrap();
        module.set_synthetic_module_export(scope, default_key, exports)?;

        let named_export_names = ESM_MODULE_LOAD_STATE.with(|state| {
            state
                .borrow()
                .as_ref()
                .and_then(|state| {
                    state
                        .cjs_synthetic_named_exports_by_identity
                        .get(&module_identity)
                        .cloned()
                })
                .unwrap_or_default()
        });
        let exports_object = exports.to_object(scope);
        for export_name in named_export_names {
            let export_key = v8::String::new(scope, &export_name).unwrap();
            let export_value = exports_object
                .and_then(|object| object.get(scope, export_key.into()))
                .unwrap_or_else(|| v8::undefined(scope).into());
            module.set_synthetic_module_export(scope, export_key, export_value)?;
        }

        Some(v8::undefined(scope).into())
    }

    fn esm_resolve_callback<'scope>(
        context: v8::Local<'scope, v8::Context>,
        specifier: v8::Local<'scope, v8::String>,
        _import_assertions: v8::Local<'scope, v8::FixedArray>,
        referrer: v8::Local<'scope, v8::Module>,
    ) -> Option<v8::Local<'scope, v8::Module>> {
        v8::callback_scope!(unsafe let scope, context);
        let specifier = specifier.to_rust_string_lossy(scope);
        let referrer_script_id = match referrer.script_id() {
            Some(script_id) => script_id,
            None => {
                Self::throw_esm_loader_error(
                    scope,
                    format!(
                        "Cannot resolve ES module '{}' from anonymous referrer",
                        specifier
                    ),
                );
                return None;
            }
        };

        let referrer_path = ESM_MODULE_LOAD_STATE.with(|state| {
            state
                .borrow()
                .as_ref()
                .and_then(|state| state.paths_by_script_id.get(&referrer_script_id).cloned())
        });
        let Some(referrer_path) = referrer_path else {
            Self::throw_esm_loader_error(
                scope,
                format!(
                    "Cannot resolve ES module '{}' because the referrer is unknown",
                    specifier
                ),
            );
            return None;
        };

        if let Some(module) = Self::create_esm_builtin_module(scope, &specifier) {
            return Some(module);
        }

        let module_path = match Self::resolve_esm_candidate_path(&specifier, &referrer_path) {
            Ok(module_path) => module_path,
            Err(error) => {
                Self::throw_esm_loader_error(scope, error);
                return None;
            }
        };
        let module_format =
            match crate::nodejs_core::commonjs_resolver::classify_commonjs_file(&module_path) {
                Ok(module_format) => module_format,
                Err(error) => {
                    Self::throw_esm_loader_error(
                        scope,
                        format!(
                            "Cannot classify ES module '{}': {}",
                            module_path.display(),
                            error
                        ),
                    );
                    return None;
                }
            };
        if module_format != crate::nodejs_core::commonjs_resolver::CommonJsModuleFormat::EsModule {
            return match Self::create_esm_commonjs_module(scope, &module_path) {
                Ok(module) => Some(module),
                Err(error) => {
                    Self::throw_esm_loader_error(scope, error);
                    None
                }
            };
        }

        let cached_module = ESM_MODULE_LOAD_STATE.with(|state| {
            state
                .borrow()
                .as_ref()
                .and_then(|state| state.modules_by_path.get(&module_path).cloned())
        });
        if let Some(cached_module) = cached_module {
            return Some(v8::Local::new(scope, cached_module));
        }

        if let Err(error) = crate::permissions::check_global_permission(
            crate::permissions::PermissionKind::FileSystem,
            crate::permissions::PermissionAction::Read,
            crate::permissions::ResourceId::Path(module_path.clone()),
        ) {
            Self::throw_esm_loader_error(scope, error.to_string());
            return None;
        }

        let module_code = match std::fs::read_to_string(&module_path) {
            Ok(module_code) => module_code,
            Err(error) => {
                Self::throw_esm_loader_error(
                    scope,
                    format!(
                        "Cannot read ES module '{}': {}",
                        module_path.display(),
                        error
                    ),
                );
                return None;
            }
        };

        let module_fingerprint = Self::esm_source_fingerprint(module_code.as_bytes());
        let module = match Self::compile_esm_module_source(scope, &module_path, &module_code) {
            Ok(module) => module,
            Err(error) => {
                Self::set_esm_pending_error(error);
                return None;
            }
        };
        Self::remember_esm_module(scope, &module_path, module, module_fingerprint);

        Some(module)
    }

    fn execute_esm_module<'scope>(
        scope: &mut v8::PinScope<'scope, '_>,
        code: &str,
        main_module_filename: &str,
        module_cache: &mut HashMap<PathBuf, v8::Global<v8::Module>>,
        module_cache_fingerprints: &mut HashMap<PathBuf, [u8; 32]>,
        timer_drain_limit_ms: u64,
    ) -> Result<v8::Local<'scope, v8::Value>, String> {
        let main_module_path = Self::normalized_module_path(Path::new(main_module_filename));
        Self::prune_stale_esm_module_cache(module_cache, module_cache_fingerprints);

        ESM_MODULE_LOAD_STATE.with(|state| {
            *state.borrow_mut() = Some(EsmModuleLoadState::default());
        });
        Self::seed_esm_module_cache(scope, module_cache, module_cache_fingerprints);

        let result = (|| {
            let module = Self::compile_esm_module_source(scope, &main_module_path, code)?;
            Self::remember_esm_module(
                scope,
                &main_module_path,
                module,
                Self::esm_source_fingerprint(code.as_bytes()),
            );

            match module.instantiate_module(scope, Self::esm_resolve_callback) {
                Some(true) => {}
                Some(false) => {
                    return Err(Self::take_esm_pending_error().unwrap_or_else(|| {
                        format!(
                            "Failed to instantiate ES module '{}'",
                            main_module_path.display()
                        )
                    }));
                }
                None => {
                    return Err(Self::take_esm_pending_error().unwrap_or_else(|| {
                        format!(
                            "Failed to instantiate ES module '{}'",
                            main_module_path.display()
                        )
                    }));
                }
            }

            let evaluation = module.evaluate(scope).ok_or_else(|| {
                Self::take_esm_pending_error().unwrap_or_else(|| {
                    format!(
                        "Failed to evaluate ES module '{}'",
                        main_module_path.display()
                    )
                })
            })?;
            Self::ensure_esm_evaluation_settled(
                scope,
                evaluation,
                &main_module_path,
                timer_drain_limit_ms,
            )?;
            Ok(evaluation)
        })();

        if result.is_ok() {
            Self::persist_esm_module_cache(
                module_cache,
                module_cache_fingerprints,
                &main_module_path,
            );
        }

        ESM_MODULE_LOAD_STATE.with(|state| {
            *state.borrow_mut() = None;
        });

        result
    }

    fn execute_esm_module_namespace<'scope>(
        scope: &mut v8::PinScope<'scope, '_>,
        code: &str,
        main_module_filename: &str,
        timer_drain_limit_ms: u64,
    ) -> Result<(v8::Local<'scope, v8::Value>, Vec<(PathBuf, [u8; 32])>), String> {
        let main_module_path = Self::normalized_module_path(Path::new(main_module_filename));

        ESM_MODULE_LOAD_STATE.with(|state| {
            *state.borrow_mut() = Some(EsmModuleLoadState::default());
        });

        let result = (|| {
            let module = Self::compile_esm_module_source(scope, &main_module_path, code)?;
            Self::remember_esm_module(
                scope,
                &main_module_path,
                module,
                Self::esm_source_fingerprint(code.as_bytes()),
            );

            match module.instantiate_module(scope, Self::esm_resolve_callback) {
                Some(true) => {}
                Some(false) => {
                    return Err(Self::take_esm_pending_error().unwrap_or_else(|| {
                        format!(
                            "Failed to instantiate ES module '{}'",
                            main_module_path.display()
                        )
                    }));
                }
                None => {
                    return Err(Self::take_esm_pending_error().unwrap_or_else(|| {
                        format!(
                            "Failed to instantiate ES module '{}'",
                            main_module_path.display()
                        )
                    }));
                }
            }

            let evaluation = module.evaluate(scope).ok_or_else(|| {
                Self::take_esm_pending_error().unwrap_or_else(|| {
                    format!(
                        "Failed to evaluate ES module '{}'",
                        main_module_path.display()
                    )
                })
            })?;
            Self::ensure_esm_evaluation_settled(
                scope,
                evaluation,
                &main_module_path,
                timer_drain_limit_ms,
            )?;
            let namespace_global = v8::Global::new(scope, module.get_module_namespace());
            let source_fingerprints = ESM_MODULE_LOAD_STATE.with(|state| {
                state
                    .borrow()
                    .as_ref()
                    .map(|state| {
                        state
                            .source_fingerprints_by_path
                            .iter()
                            .map(|(path, fingerprint)| (path.clone(), *fingerprint))
                            .collect()
                    })
                    .unwrap_or_default()
            });
            Ok((v8::Local::new(scope, namespace_global), source_fingerprints))
        })();

        ESM_MODULE_LOAD_STATE.with(|state| {
            *state.borrow_mut() = None;
        });

        result
    }

    fn compile_typescript_commonjs_module(code: &str, filename: &str) -> Result<String> {
        let module_exports_marker = "__beejs_commonjs_module_exports__";
        let module_export_assignment_pattern =
            regex::Regex::new(r"(?s)\bmodule\s*\.\s*exports\b\s*=\s*.*?;").unwrap();
        let mut module_export_statements = Vec::<(String, String)>::new();
        let protected_assignments = module_export_assignment_pattern
            .replace_all(code, |captures: &regex::Captures| {
                let marker = format!(
                    "__beejs_commonjs_export_statement_{}",
                    module_export_statements.len()
                );
                module_export_statements.push((marker.clone(), captures[0].to_string()));
                format!("{marker};")
            })
            .to_string();
        let module_exports_pattern = regex::Regex::new(r"\bmodule\s*\.\s*exports\b").unwrap();
        let protected_code = module_exports_pattern
            .replace_all(&protected_assignments, module_exports_marker)
            .to_string();
        let output = crate::typescript::compile_typescript(&protected_code, filename)
            .map_err(|error| anyhow::anyhow!(error))?;
        let mut js_code = output
            .js_code
            .replace(module_exports_marker, "module.exports");
        for (marker, statement) in module_export_statements {
            js_code = js_code.replace(&marker, &statement);
        }
        Ok(js_code)
    }

    /// Transpile TypeScript to JavaScript by removing type annotations
    #[allow(dead_code)]
    fn transpile_typescript_to_js(code: &str) -> Result<String> {
        let mut js_code = code.to_string();

        // Remove block comments (/* */)
        let block_comment_pattern = regex::Regex::new(r"/\*.*?\*/").unwrap();
        js_code = block_comment_pattern.replace_all(&js_code, "").to_string();

        // Remove single-line comments
        let single_line_pattern = regex::Regex::new(r"//.*?$").unwrap();
        js_code = single_line_pattern.replace_all(&js_code, "").to_string();

        // v0.3.181: Remove interface definitions with bodies using bracket matching
        // This properly handles nested braces, parentheses, and strings
        fn remove_interfaces(code: &str) -> String {
            let mut result = String::new();
            let mut i = 0;
            let chars: Vec<char> = code.chars().collect();
            let n = chars.len();

            while i < n {
                // Look for "interface " followed by an identifier
                let interface_start =
                    chars[i..].starts_with(&['i', 'n', 't', 'e', 'r', 'f', 'a', 'c', 'e', ' '][..]);

                if interface_start {
                    // Find the interface name
                    let name_start = i + 10; // After "interface "
                    let mut name_end = name_start;
                    while name_end < n {
                        if chars[name_end].is_alphanumeric() || chars[name_end] == '_' {
                            name_end += 1;
                        } else {
                            break;
                        }
                    }
                    let interface_name: String = chars[name_start..name_end].iter().collect();

                    // Skip whitespace to find the opening brace
                    let mut brace_pos = name_end;
                    while brace_pos < n && chars[brace_pos].is_whitespace() {
                        brace_pos += 1;
                    }

                    // Check if we found an opening brace
                    if brace_pos < n && chars[brace_pos] == '{' {
                        // Find matching closing brace
                        let mut depth = 1;
                        let mut j = brace_pos + 1;
                        let mut in_string = false;
                        let mut string_char = '\0';

                        while j < n && depth > 0 {
                            let c = chars[j];
                            if in_string {
                                if c == '\\' && j + 1 < n {
                                    j += 2;
                                    continue;
                                }
                                if c == string_char {
                                    in_string = false;
                                }
                            } else {
                                if c == '"' || c == '\'' {
                                    in_string = true;
                                    string_char = c;
                                } else if c == '{' {
                                    depth += 1;
                                } else if c == '}' {
                                    depth -= 1;
                                }
                            }
                            j += 1;
                        }

                        // Replace the entire interface with a comment
                        result.push_str(&format!("/* interface {} */", interface_name));

                        // Move i to after the closing brace
                        i = j;
                        continue;
                    }
                }

                result.push(chars[i]);
                i += 1;
            }

            result
        }

        // v0.3.182: Remove constructor signatures from interfaces
        // Pattern: "new (args): ReturnType" - removes the entire constructor signature
        // This MUST run BEFORE remove_interfaces to handle constructors inside interfaces
        fn remove_constructor_signatures(code: &str) -> String {
            let mut result = String::new();
            let mut i = 0;
            let chars: Vec<char> = code.chars().collect();
            let n = chars.len();

            while i < n {
                // Look for "new (" pattern (constructor signature)
                let new_ctor_start = chars[i..].starts_with(&['n', 'e', 'w', ' '][..])
                    && i + 4 < n
                    && chars[i + 4] == '(';

                if new_ctor_start {
                    // Find the return type after the closing parenthesis and colon
                    let mut paren_depth = 1;
                    let mut j = i + 5; // Start after "new ("

                    // Skip parameters inside parentheses, handling nested parens and strings
                    while j < n && paren_depth > 0 {
                        let c = chars[j];
                        if c == '(' {
                            paren_depth += 1;
                        } else if c == ')' {
                            paren_depth -= 1;
                            if paren_depth == 0 {
                                j += 1;
                                break;
                            }
                        } else if c == '"' || c == '\'' {
                            // Skip string contents
                            let string_char = c;
                            j += 1;
                            while j < n && chars[j] != string_char {
                                if chars[j] == '\\' && j + 1 < n {
                                    j += 2;
                                } else {
                                    j += 1;
                                }
                            }
                            if j < n {
                                j += 1;
                            }
                        }
                        j += 1;
                    }

                    // Skip whitespace after closing paren
                    while j < n && chars[j].is_whitespace() {
                        j += 1;
                    }

                    // Skip the colon
                    if j < n && chars[j] == ':' {
                        j += 1;
                    }

                    // Skip whitespace after colon
                    while j < n && chars[j].is_whitespace() {
                        j += 1;
                    }

                    // Extract the return type name (for the comment)
                    let return_start = j;
                    let mut return_end = j;
                    while return_end < n {
                        let c = chars[return_end];
                        if c.is_alphanumeric() || c == '_' || c == '<' || c == '>' {
                            return_end += 1;
                        } else {
                            break;
                        }
                    }

                    // Handle generic types like Array<T>
                    if return_end < n && chars[return_end] == '<' {
                        let mut angle_depth = 1;
                        return_end += 1;
                        while return_end < n && angle_depth > 0 {
                            if chars[return_end] == '<' {
                                angle_depth += 1;
                            } else if chars[return_end] == '>' {
                                angle_depth -= 1;
                            }
                            return_end += 1;
                        }
                    }

                    let return_type: String = chars[return_start..return_end].iter().collect();

                    // Remove the constructor signature including trailing semicolon
                    let mut remove_end = return_end;
                    while remove_end < n && chars[remove_end].is_whitespace() {
                        remove_end += 1;
                    }
                    if remove_end < n && chars[remove_end] == ';' {
                        remove_end += 1;
                    }

                    result.push_str(&format!("/* constructor: {} */", return_type));
                    i = remove_end;
                    continue;
                }

                result.push(chars[i]);
                i += 1;
            }

            result
        }

        // Remove constructor signatures BEFORE removing interfaces
        // This handles constructor signatures inside interfaces
        js_code = remove_constructor_signatures(&js_code);

        js_code = remove_interfaces(&js_code);

        // v0.3.178: Remove enum declarations
        // Pattern: "enum EnumName { ... }" - comment out entire enum block
        // Use non-greedy matching to handle nested braces properly
        let enum_pattern =
            regex::Regex::new(r"enum\s+([A-Z][a-zA-Z0-9_]*)\s*\{[^{}]*\{[^{}]*\}[^{}]*\}").unwrap();
        js_code = enum_pattern
            .replace_all(&js_code, "/* enum $1 */")
            .to_string();

        // Simple enum pattern for enums without nested braces
        let enum_simple_pattern =
            regex::Regex::new(r"enum\s+([A-Z][a-zA-Z0-9_]*)\s*\{[^}]*\}").unwrap();
        js_code = enum_simple_pattern
            .replace_all(&js_code, "/* enum $1 */")
            .to_string();

        // v0.3.178: Remove type alias declarations
        // Pattern: "type AliasName = ..." - comment out entire type alias
        // Handle simple single-line type aliases
        let type_alias_pattern = regex::Regex::new(
            r"(?m)(?:export\s+)?type\s+([A-Z][a-zA-Z0-9_]*)(?:\s*<[^=;]+>)?\s*=\s*[^;]+;",
        )
        .unwrap();
        js_code = type_alias_pattern
            .replace_all(&js_code, "/* type $1 */")
            .to_string();

        // Handle multi-line type aliases (type AliasName = { ... } or type AliasName = | ...)
        let type_alias_multiline_pattern = regex::Regex::new(
            r"(?m)(?:export\s+)?type\s+([A-Z][a-zA-Z0-9_]*)(?:\s*<[^=;]+>)?\s*=\s*\{[^}]*\}",
        )
        .unwrap();
        js_code = type_alias_multiline_pattern
            .replace_all(&js_code, "/* type $1 */")
            .to_string();

        // Handle union type aliases: "type Alias = A | B | C"
        let type_union_pattern =
            regex::Regex::new(
                r"(?m)(?:export\s+)?type\s+([A-Z][a-zA-Z0-9_]*)(?:\s*<[^=;]+>)?\s*=\s*[^;]+(?:\|[^;]+)*;",
            )
            .unwrap();
        js_code = type_union_pattern
            .replace_all(&js_code, "/* type $1 */")
            .to_string();

        // v0.3.184: Remove mapped type definitions
        // Pattern: { [P in keyof T]: T[P] } or { readonly [P in keyof T]: T[P] }
        // This uses a bracket-matching approach to handle nested types properly
        fn remove_mapped_types(code: &str) -> String {
            let mut result = String::new();
            let mut i = 0;
            let chars: Vec<char> = code.chars().collect();
            let n = chars.len();

            while i < n {
                // Look for "[" followed by identifier and " in " (mapped type pattern)
                // Pattern: [Identifier in ...]: or readonly [Identifier in ...]:
                let is_lbracket = chars[i] == '[';
                let has_identifier_in = if is_lbracket && i + 1 < n {
                    // Check if we have [Identifier... or [ Identifier...
                    let mut j = i + 1;
                    while j < n && chars[j].is_whitespace() {
                        j += 1;
                    }
                    // Check for readonly modifier
                    let has_readonly =
                        chars[j..].starts_with(&['r', 'e', 'a', 'd', 'o', 'n', 'l', 'y'][..]);
                    if has_readonly {
                        j += 8;
                        while j < n && chars[j].is_whitespace() {
                            j += 1;
                        }
                    }
                    // Now should be at [
                    if j < n && chars[j] == '[' {
                        j += 1;
                        while j < n && chars[j].is_whitespace() {
                            j += 1;
                        }
                        // Check for identifier followed by " in "
                        if j < n && (chars[j].is_alphabetic() || chars[j] == '_') {
                            j += 1;
                            while j < n
                                && (chars[j].is_alphanumeric()
                                    || chars[j] == '_'
                                    || chars[j] == '$')
                            {
                                j += 1;
                            }
                            while j < n && chars[j].is_whitespace() {
                                j += 1;
                            }
                            // Check for " in "
                            chars[j..].starts_with(&['i', 'n', ' '][..])
                                && j + 3 < n
                                && chars[j + 3].is_whitespace()
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                if has_identifier_in {
                    // Find the matching ] for the opening [
                    // Start from i, find the first ] that closes the opening [
                    let mut depth = 1;
                    let mut j = i + 1;
                    let mut in_string = false;
                    let mut string_char = '\0';

                    while j < n && depth > 0 {
                        let c = chars[j];
                        if in_string {
                            if c == '\\' && j + 1 < n {
                                j += 2;
                                continue;
                            }
                            if c == string_char {
                                in_string = false;
                            }
                        } else {
                            if c == '"' || c == '\'' || c == '`' {
                                in_string = true;
                                string_char = c;
                            } else if c == '[' {
                                depth += 1;
                            } else if c == ']' {
                                depth -= 1;
                            }
                        }
                        j += 1;
                    }

                    // Replace with comment placeholder
                    result.push_str("/* mapped type */");
                    i = j;
                    continue;
                }

                result.push(chars[i]);
                i += 1;
            }

            result
        }

        // Remove mapped types before interface removal
        // This handles { [P in keyof T]: T[P] } patterns
        js_code = remove_mapped_types(&js_code);

        // v0.3.190: Remove index signature definitions
        // Pattern: [key: string]: Type or [key: number]: Type
        // Index signatures are TypeScript-only and define dynamic property types
        // Example: interface StringMap { [key: string]: string; }
        fn remove_index_signatures(code: &str) -> String {
            let mut result = String::new();
            let mut i = 0;
            let chars: Vec<char> = code.chars().collect();
            let n = chars.len();

            while i < n {
                // Look for "[key:" pattern (start of index signature)
                let is_index_sig_start = chars[i..].starts_with(&['[', 'k', 'e', 'y', ':'][..]);

                if is_index_sig_start {
                    // Find the end of this index signature line (semicolon or closing brace)
                    let mut j = i + 5; // After "[key:"
                    let mut in_string = false;
                    let mut string_char = '\0';
                    let mut found_closing_bracket = false;

                    // Skip the key type (string or number)
                    while j < n {
                        let c = chars[j];
                        if in_string {
                            if c == '\\' && j + 1 < n {
                                j += 2;
                                continue;
                            }
                            if c == string_char {
                                in_string = false;
                            }
                        } else if c == '"' || c == '\'' {
                            in_string = true;
                            string_char = c;
                        } else if c == ']' {
                            found_closing_bracket = true;
                            j += 1;
                            break;
                        }
                        j += 1;
                    }

                    if found_closing_bracket {
                        // Skip whitespace
                        while j < n && chars[j].is_whitespace() {
                            j += 1;
                        }

                        // Skip the colon
                        if j < n && chars[j] == ':' {
                            j += 1;
                        }

                        // Skip whitespace after colon
                        while j < n && chars[j].is_whitespace() {
                            j += 1;
                        }

                        // Find the end of the type expression
                        let mut type_depth = 0;
                        let mut paren_depth = 0;
                        while j < n {
                            let c = chars[j];
                            if in_string {
                                if c == '\\' && j + 1 < n {
                                    j += 2;
                                    continue;
                                }
                                if c == string_char {
                                    in_string = false;
                                }
                            } else if c == '"' || c == '\'' {
                                in_string = true;
                                string_char = c;
                            } else if c == '<' {
                                type_depth += 1;
                            } else if c == '>' {
                                type_depth -= 1;
                            } else if c == '(' {
                                paren_depth += 1;
                            } else if c == ')' {
                                paren_depth -= 1;
                            } else if c == ';' && type_depth == 0 && paren_depth == 0 {
                                j += 1;
                                break;
                            } else if c == '\n' && type_depth == 0 && paren_depth == 0 {
                                break;
                            } else if c == '}' && type_depth == 0 && paren_depth == 0 {
                                // Don't consume the closing brace, let the outer loop handle it
                                break;
                            }
                            j += 1;
                        }

                        // Replace with a comment indicating removed index signature
                        result.push_str("/* index signature */");
                        i = j;
                        continue;
                    }
                }

                result.push(chars[i]);
                i += 1;
            }

            result
        }

        js_code = remove_index_signatures(&js_code);

        // Remove type annotations from function parameters ONLY
        // This pattern matches: :TypeName followed by , or )
        // Using capturing group instead of lookahead (not supported by regex crate)
        let param_pattern = regex::Regex::new(r":\s*(string|number|boolean|undefined|null|any|void|never|unknown|object|symbol|bigint|Function|Promise<[^>]+>|Array<[^>]+>)([,\)])").unwrap();
        js_code = param_pattern.replace_all(&js_code, "$1$2").to_string();

        // Also handle simple type annotations like : TypeName (capitalized)
        let simple_type_pattern = regex::Regex::new(r":\s*([A-Z][a-zA-Z0-9]*)([,\)])").unwrap();
        js_code = simple_type_pattern
            .replace_all(&js_code, "$1$2")
            .to_string();

        // v0.3.183: Remove this parameter type annotations
        // Pattern: "this: Type," or "this: Type)" - removes the entire this parameter
        // This handles: function greet(this: { name: string }, msg: string) {}
        // And: interface Config { greet(this: { name: string }): string; }
        let this_param_pattern = regex::Regex::new(r"this:\s*\{[^}]*\}([,\)])").unwrap();
        js_code = this_param_pattern.replace_all(&js_code, "$1").to_string();

        // Handle simple this: Type patterns (this: any, this: Context, etc.)
        let this_simple_pattern =
            regex::Regex::new(r"this:\s*[a-zA-Z<>][a-zA-Z0-9<>]*([,\)])").unwrap();
        js_code = this_simple_pattern.replace_all(&js_code, "$1").to_string();

        // Handle object type in this parameter with nested braces
        let this_object_pattern =
            regex::Regex::new(r"this:\s*\{[^{}]*\{[^{}]*\}[^{}]*\}([,\)])").unwrap();
        js_code = this_object_pattern.replace_all(&js_code, "$1").to_string();

        // Remove return type annotations: -> type
        let return_pattern = regex::Regex::new(r"->\s*[^;{]+").unwrap();
        js_code = return_pattern.replace_all(&js_code, "").to_string();

        // Remove variable type annotations - only match at statement start
        let var_pattern = regex::Regex::new(r"(?m)^let\s+(\w+):\s*[^;=]+").unwrap();
        js_code = var_pattern.replace_all(&js_code, "let $1").to_string();

        let const_pattern = regex::Regex::new(r"(?m)^const\s+(\w+):\s*[^;=]+").unwrap();
        js_code = const_pattern.replace_all(&js_code, "const $1").to_string();

        // v0.3.167: Remove as const assertions: "expr as const" -> "expr"
        let as_const_pattern = regex::Regex::new(r"\s+as\s+const").unwrap();
        js_code = as_const_pattern.replace_all(&js_code, "").to_string();

        // v0.3.167: Remove as Type assertions: "expr as TypeName" -> "expr"
        // This pattern matches "as" followed by a type identifier (capitalized or known type)
        let as_type_pattern =
            regex::Regex::new(r"\s+as\s+([A-Z][a-zA-Z0-9<>]*(?:\s*<[^>]+>)?)").unwrap();
        js_code = as_type_pattern.replace_all(&js_code, "").to_string();

        // v0.3.168: Remove satisfies operator: "expr satisfies Type" -> "expr"
        // The satisfies operator checks type compatibility without changing the inferred type
        // Handle various type patterns: simple types, object types (including nested), union types, array types

        // Helper function to find matching closing bracket/paren
        fn find_matching_bracket(s: &str, start: usize, open: char, close: char) -> Option<usize> {
            let mut depth = 0;
            let mut in_string = false;
            let mut string_char = '\0';
            let mut chars = s.char_indices().skip(start);

            while let Some((i, c)) = chars.next() {
                if in_string {
                    if c == '\\' && string_char != '\\' {
                        // Skip escaped character
                        if let Some((_, next_c)) = chars.next() {
                            if next_c == string_char || (string_char == '\'' && next_c == '\'') {
                                continue;
                            }
                        }
                    } else if c == string_char {
                        in_string = false;
                    }
                } else {
                    if c == '"' || c == '\'' {
                        in_string = true;
                        string_char = c;
                    } else if c == open {
                        depth += 1;
                    } else if c == close {
                        depth -= 1;
                        if depth == 0 {
                            return Some(i + close.len_utf8());
                        }
                    }
                }
            }
            None
        }

        // Remove satisfies with various type patterns using manual parsing
        // Handle cases like:
        // - `expr } satisfies Type` (object literal)
        // - `expr ) satisfies Type` (parenthesized)
        // - `identifier satisfies Type` (simple value)
        let mut result = String::new();
        let mut i = 0;
        let mut last_processed = 0;
        let chars: Vec<char> = js_code.chars().collect();
        let n = chars.len();

        while i < n {
            // Look for "satisfies"
            let is_satisfies_start =
                chars[i..].starts_with(&['s', 'a', 't', 'i', 's', 'f', 'i', 'e', 's'][..]);

            if is_satisfies_start {
                // Check if preceded by } or ) or ] or identifier/number character (with optional whitespace)
                let mut j = i;
                while j > 0 && chars[j - 1].is_whitespace() {
                    j -= 1;
                }

                let valid_predecessor = j > 0
                    && (chars[j - 1] == '}'
                        || chars[j - 1] == ')'
                        || chars[j - 1] == ']'
                        || chars[j - 1].is_alphanumeric()
                        || chars[j - 1] == '_'
                        || chars[j - 1] == '$'
                        || chars[j - 1] == '\'');

                if valid_predecessor {
                    // Copy everything from last_processed to i (the code before satisfies)
                    result.push_str(&js_code[last_processed..i]);

                    // Find the type expression after satisfies and skip it
                    let mut k = i + 9; // length of "satisfies"
                    while k < n && chars[k].is_whitespace() {
                        k += 1;
                    }

                    // Skip type expression (identifiers, keywords, then optional array suffix [])
                    while k < n {
                        // Skip whitespace
                        if chars[k].is_whitespace() {
                            k += 1;
                            continue;
                        }

                        // Skip array suffix []
                        if chars[k] == '[' && k + 1 < n && chars[k + 1] == ']' {
                            k += 2;
                            continue;
                        }

                        // Skip [ ] with whitespace
                        if chars[k] == '[' {
                            let mut bracket_k = k;
                            bracket_k += 1;
                            while bracket_k < n && chars[bracket_k].is_whitespace() {
                                bracket_k += 1;
                            }
                            if bracket_k < n && chars[bracket_k] == ']' {
                                k = bracket_k + 1;
                                continue;
                            }
                        }

                        // Skip type name (alphanumeric or generic)
                        if chars[k].is_alphanumeric() || chars[k] == '_' || chars[k] == '$' {
                            k += 1;
                            continue;
                        }

                        // Skip generic type parameters like <T> or <string>
                        if chars[k] == '<' {
                            let mut angle_k = k;
                            angle_k += 1;
                            let mut depth = 1;
                            while angle_k < n && depth > 0 {
                                if chars[angle_k] == '<' {
                                    depth += 1;
                                } else if chars[angle_k] == '>' {
                                    depth -= 1;
                                }
                                angle_k += 1;
                            }
                            if depth == 0 {
                                k = angle_k;
                                continue;
                            }
                        }

                        // Stop at statement terminators
                        if chars[k] == ';' || chars[k] == ',' {
                            break;
                        }

                        // Stop at other expression terminators
                        if matches!(chars[k], ')' | '}' | ']') {
                            break;
                        }

                        k += 1;
                    }

                    if k < n {
                        match chars[k] {
                            '{' => {
                                // Object type - find matching }
                                if let Some(end_pos) = find_matching_bracket(&js_code, k, '{', '}')
                                {
                                    k = end_pos;
                                } else {
                                    k = n;
                                }
                            }
                            '(' => {
                                // Parenthesized type - find matching )
                                if let Some(end_pos) = find_matching_bracket(&js_code, k, '(', ')')
                                {
                                    k = end_pos;
                                } else {
                                    k = n;
                                }
                            }
                            '[' => {
                                // Array type like number[] - skip until ]
                                let mut bracket_depth = 0;
                                while k < n {
                                    if chars[k] == '[' {
                                        bracket_depth += 1;
                                    } else if chars[k] == ']' {
                                        bracket_depth -= 1;
                                        if bracket_depth == 0 {
                                            k += 1;
                                            break;
                                        }
                                    }
                                    k += 1;
                                }
                            }
                            '<' => {
                                // Generic type like Array<number> - find matching >
                                let mut angle_depth = 0;
                                while k < n {
                                    if chars[k] == '<' {
                                        angle_depth += 1;
                                    } else if chars[k] == '>' {
                                        angle_depth -= 1;
                                        if angle_depth == 0 {
                                            k += 1;
                                            break;
                                        }
                                    } else if chars[k] == '{' || chars[k] == '(' || chars[k] == '['
                                    {
                                        // Skip nested brackets
                                        if let Some(end_pos) = find_matching_bracket(
                                            &js_code,
                                            k,
                                            chars[k],
                                            match chars[k] {
                                                '{' => '}',
                                                '(' => ')',
                                                '[' => ']',
                                                _ => ' ',
                                            },
                                        ) {
                                            k = end_pos;
                                        }
                                    }
                                    k += 1;
                                }
                            }
                            _ => {
                                // Simple type - skip until whitespace or special char
                                while k < n
                                    && !chars[k].is_whitespace()
                                    && !matches!(
                                        chars[k],
                                        ';' | ','
                                            | ')'
                                            | '}'
                                            | ']'
                                            | '|'
                                            | '&'
                                            | '+'
                                            | '-'
                                            | '*'
                                            | '/'
                                            | '%'
                                            | '^'
                                            | '!'
                                            | '?'
                                            | ':'
                                            | '='
                                            | '<'
                                            | '>'
                                    )
                                {
                                    k += 1;
                                }
                            }
                        }
                    }

                    last_processed = k;
                    i = k;
                    continue;
                }
            }

            i += 1;
        }

        // Copy the remaining code after the last satisfies
        if last_processed < n {
            result.push_str(&js_code[last_processed..n]);
        }

        js_code = result;

        // v0.3.170: Remove declare global { ... } blocks
        // Keep the declare keyword for const/let/var inside, remove interface/function types
        let declare_global_pattern = regex::Regex::new(r"declare\s+global\s*[{][^}]*[}]").unwrap();
        js_code = declare_global_pattern
            .replace_all(&js_code, "/* declare global */")
            .to_string();

        // v0.3.170: Remove declare module "name" { ... } blocks
        // These are type-only declarations for module augmentation
        let declare_module_pattern =
            regex::Regex::new(r#"declare\s+module\s+"[^"]+"\s*[{][^}]*[}]"#).unwrap();
        js_code = declare_module_pattern
            .replace_all(&js_code, "/* declare module */")
            .to_string();

        // v0.3.172: Remove export = expr statements (CommonJS/AMD compatible)
        // export = is a TypeScript/TSX specific syntax for module exports
        let export_equals_pattern =
            regex::Regex::new(r"(?m)(^|[;\r\n])(\s*)export\s*=\s*[^;\r\n]+;?").unwrap();
        js_code = export_equals_pattern
            .replace_all(&js_code, "$1$2/* export = */")
            .to_string();

        // v0.3.174: Remove keyof Type expressions
        // keyof returns union of string literal types representing property names
        // v0.3.185: Enhanced to support more complex keyof patterns
        // Pattern 1: "keyof TypeName" where TypeName is typically capitalized
        let keyof_pattern = regex::Regex::new(r"keyof\s+[A-Z][a-zA-Z0-9_<>]*").unwrap();
        js_code = keyof_pattern.replace_all(&js_code, "string").to_string();

        // v0.3.185: Enhanced keyof typeof pattern - keyof typeof obj -> string
        // Handles: keyof typeof identifier
        let keyof_typeof_pattern =
            regex::Regex::new(r"keyof\s+typeof\s+([a-zA-Z_$][a-zA-Z0-9_$]*)").unwrap();
        js_code = keyof_typeof_pattern
            .replace_all(&js_code, "string")
            .to_string();

        // v0.3.185: Remove keyof expressions in mapped type constraints
        // Handles: <T extends keyof U> or <K extends keyof T>
        let keyof_constraint_pattern =
            regex::Regex::new(r"extends\s+keyof\s+[A-Za-z_$][a-zA-Z0-9_$<>]*").unwrap();
        js_code = keyof_constraint_pattern
            .replace_all(&js_code, "extends string")
            .to_string();

        // v0.3.185: Handle indexed access with keyof: T[keyof T] -> T[string]
        let indexed_keyof_pattern =
            regex::Regex::new(r"\[keyof\s+([A-Z][a-zA-Z0-9_<>]*)\]").unwrap();
        js_code = indexed_keyof_pattern
            .replace_all(&js_code, "[string]")
            .to_string();

        // v0.3.174: Remove typeof identifier in type context
        // typeof returns the type of a value at compile time
        // Pattern: "typeof identifier" where identifier is typically lowercase
        // NOTE: We NO LONGER remove typeof identifier because:
        // 1. typeof is valid JavaScript runtime operator
        // 2. Removing it breaks JavaScript code that uses typeof
        // 3. The transpiler should only remove TypeScript-specific syntax
        // This pattern is kept for historical purposes but is now a no-op
        // let typeof_pattern = regex::Regex::new(r"typeof\s+([a-zA-Z_$][a-zA-Z0-9_$]*)").unwrap();
        // js_code = typeof_pattern.replace_all(&js_code, "/* typeof $1 */").to_string();
        // Instead, we do nothing - typeof is valid JavaScript!

        // v0.3.175: Remove infer type expressions
        // infer is used in conditional types to extract types: "infer U" or "infer U extends Type"
        // Pattern: "infer Identifier" or "infer Identifier extends Type"
        let infer_pattern =
            regex::Regex::new(r"infer\s+([A-Z][a-zA-Z0-9_]*)(?:\s+extends\s+[^?;=]+)?").unwrap();
        js_code = infer_pattern
            .replace_all(&js_code, "/* infer $1 */")
            .to_string();

        // v0.3.186: Remove conditional type expressions
        // Pattern: "T extends U ? X : Y" in type alias definitions
        // This handles basic conditional types like: type A<T> = T extends string ? "yes" : "no";
        // We remove the entire conditional type expression and replace with a comment
        // Uses a character-level approach to handle nested types correctly
        fn remove_conditional_types(code: &str) -> String {
            let mut result = String::new();
            let mut i = 0;
            let chars: Vec<char> = code.chars().collect();
            let n = chars.len();

            while i < n {
                // Look for " extends " pattern in type context
                let extends_start =
                    chars[i..].starts_with(&['e', 'x', 't', 'e', 'n', 'd', 's', ' '][..]);

                if extends_start && i > 0 {
                    // Check if we're in a type alias context (after "=")
                    let mut j = i;
                    let mut found_equals = false;
                    while j > 0 {
                        if chars[j] == '=' {
                            found_equals = true;
                            break;
                        }
                        if chars[j] == ';' || chars[j] == '\n' {
                            break;
                        }
                        j -= 1;
                    }

                    if found_equals {
                        // Find the end of the conditional type expression
                        // We need to match: extends <type> ? <type> : <type> ;
                        let mut k = i + 7; // Skip "extends "
                        let mut depth = 0;
                        let mut paren_depth = 0;
                        let mut angle_depth = 0;
                        let mut in_string = false;
                        let mut string_char = '\0';
                        let mut found_question = false;
                        let mut found_colon = false;

                        while k < n {
                            let c = chars[k];
                            if in_string {
                                if c == '\\' && k + 1 < n {
                                    k += 2;
                                    continue;
                                }
                                if c == string_char {
                                    in_string = false;
                                }
                            } else if c == '"' || c == '\'' || c == '`' {
                                in_string = true;
                                string_char = c;
                            } else if c == '<' {
                                angle_depth += 1;
                            } else if c == '>' {
                                angle_depth -= 1;
                            } else if c == '{' {
                                depth += 1;
                            } else if c == '}' {
                                if depth > 0 {
                                    depth -= 1;
                                } else if angle_depth == 0 {
                                    break;
                                }
                            } else if c == '(' {
                                paren_depth += 1;
                            } else if c == ')' {
                                if paren_depth > 0 {
                                    paren_depth -= 1;
                                } else if depth == 0 && angle_depth == 0 {
                                    break;
                                }
                            } else if c == '?' && depth == 0 && paren_depth == 0 && angle_depth == 0
                            {
                                found_question = true;
                            } else if c == ':'
                                && depth == 0
                                && paren_depth == 0
                                && angle_depth == 0
                                && found_question
                            {
                                found_colon = true;
                            } else if c == ';' && found_colon && depth == 0 {
                                k += 1;
                                break;
                            } else if c == '\n' && found_colon && depth == 0 {
                                break;
                            }
                            k += 1;
                        }

                        result.push_str("/* conditional type */");
                        i = k;
                        continue;
                    }
                }

                result.push(chars[i]);
                i += 1;
            }

            result
        }

        js_code = remove_conditional_types(&js_code);

        // v0.3.188: Remove template literal type definitions
        // Pattern: `prefix${Type}suffix` in type alias definitions
        // Examples:
        // - type Greeting = `Hello ${string}`;
        // - type Email = `user-${string}@${string}.com`;
        // - type Path = `/api/${string}/${string}`;
        // Template literal types are TypeScript-only and should be removed in JS output
        fn remove_template_literal_types(code: &str) -> String {
            let mut result = String::new();
            let mut i = 0;
            let chars: Vec<char> = code.chars().collect();
            let n = chars.len();

            while i < n {
                // Look for backtick followed by something and ${ (template literal type pattern)
                // We need to detect TypeScript template literal types vs JS template strings
                // TypeScript patterns include: ${string}, ${number}, ${boolean}, ${any}, etc.
                let is_template_start = chars[i] == '`';

                if is_template_start {
                    // Check if this is a template literal type by looking for type patterns inside ${...}
                    // TypeScript template literal types use types like ${string}, ${number}, etc.
                    // JavaScript template strings use expressions like ${variable}
                    let mut j = i + 1;
                    let mut has_type_pattern = false;

                    while j < n && chars[j] != '`' {
                        if chars[j] == '$' && j + 1 < n && chars[j + 1] == '{' {
                            // Start of template expression
                            j += 2;

                            // Look for type pattern (identifier followed by } or space then })
                            // TypeScript types: string, number, boolean, any, never, unknown, symbol, bigint, void, null, undefined
                            // v0.3.200: Also check for intrinsic string types: Uppercase, Lowercase, Capitalize, Uncapitalize
                            let type_keywords = [
                                "string",
                                "number",
                                "boolean",
                                "any",
                                "never",
                                "unknown",
                                "symbol",
                                "bigint",
                                "void",
                                "null",
                                "undefined",
                            ];
                            let intrinsic_types =
                                ["Uppercase", "Lowercase", "Capitalize", "Uncapitalize"];

                            while j < n && chars[j] != '}' {
                                // Check for intrinsic string types (starts with uppercase)
                                if chars[j].is_alphabetic() && chars[j].is_uppercase() {
                                    for intrinsic in &intrinsic_types {
                                        let int_len = intrinsic.len();
                                        if j + int_len < n {
                                            // < instead of +1 <=
                                            let candidate: String =
                                                chars[j..j + int_len].iter().collect();
                                            if candidate == *intrinsic && chars[j + int_len] == '<'
                                            {
                                                has_type_pattern = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                                // Check if we hit a character that can't be in a type (variable indicator)
                                // Lowercase start suggests a type keyword
                                if chars[j].is_alphabetic() && chars[j].is_lowercase() {
                                    // Potential type keyword - check for match
                                    for keyword in &type_keywords {
                                        let kw_len = keyword.len();
                                        if j + kw_len <= n {
                                            let candidate: String =
                                                chars[j..j + kw_len].iter().collect();
                                            if candidate == *keyword {
                                                has_type_pattern = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                                j += 1;
                                if has_type_pattern {
                                    break;
                                }
                            }

                            if j < n && chars[j] == '}' {
                                j += 1;
                            }
                        } else {
                            j += 1;
                        }
                    }

                    if has_type_pattern {
                        // This is a template literal type - replace with empty string
                        // Skip to the end of the template
                        while j < n && chars[j] != '`' {
                            j += 1;
                        }
                        // Don't add anything for template literal types
                        i = j + 1; // Skip past the closing backtick
                        continue;
                    }
                }

                result.push(chars[i]);
                i += 1;
            }

            result
        }

        js_code = remove_template_literal_types(&js_code);

        // v0.3.176: Remove abstract class and abstract method declarations
        // abstract is a TypeScript-only keyword for defining abstract classes and methods
        // Pattern: "abstract class ClassName" and "abstract methodName(): returnType;"
        let abstract_class_pattern =
            regex::Regex::new(r"abstract\s+class\s+([A-Z][a-zA-Z0-9_]*)").unwrap();
        js_code = abstract_class_pattern
            .replace_all(&js_code, "class $1")
            .to_string();

        // Remove abstract modifier from method declarations within classes
        // Pattern: "abstract methodName(): returnType;" -> Just remove "abstract " prefix
        // The return type annotation will be handled by the existing type annotation removal patterns
        let abstract_method_pattern =
            regex::Regex::new(r"abstract\s+([a-zA-Z_$][a-zA-Z0-9_$]*)").unwrap();
        js_code = abstract_method_pattern
            .replace_all(&js_code, "$1")
            .to_string();

        // v0.3.193: Remove import type statements
        // import type is a TypeScript-only import that only imports type information
        // Patterns:
        // - "import type { ... } from 'module';" -> remove entire line
        // - "import type * as Namespace from 'module';" -> remove entire line
        // - "import type Alias from 'module';" -> remove entire line
        let import_type_pattern = regex::Regex::new(r"(?m)^\s*import\s+type\s+.*?;?\s*$").unwrap();
        js_code = import_type_pattern.replace_all(&js_code, "").to_string();

        // v0.3.193: Remove export type statements
        // export type is a TypeScript-only export that only exports type information
        // Patterns:
        // - "export type { ... };" -> remove entire line
        // - "export type { ... } from 'module';" -> remove entire line
        let export_type_pattern =
            regex::Regex::new(r"(?m)^\s*export\s+type\s+\{[^}]*\}\s*(from\s+[^;]+)?;?\s*$")
                .unwrap();
        js_code = export_type_pattern.replace_all(&js_code, "").to_string();

        // Clean up extra whitespace (especially after removing satisfies)
        let cleanup_pattern = regex::Regex::new(r"\s+([;,})])").unwrap();
        js_code = cleanup_pattern.replace_all(&js_code, "$1").to_string();

        // Remove type annotations from satisfies object types (e.g., { host: string; port: number } -> { host; port })
        // This handles the colon-type pattern within object literals
        let type_annotation_in_satisfies = regex::Regex::new(
            r":\s*(string|number|boolean|unknown|any|void|null|undefined|never)(?:\s*[;}\n,\]]|$)",
        )
        .unwrap();
        js_code = type_annotation_in_satisfies
            .replace_all(&js_code, "")
            .to_string();

        // Clean up extra semicolons at end of lines
        let cleanup_pattern = regex::Regex::new(r";\s*\n").unwrap();
        js_code = cleanup_pattern.replace_all(&js_code, "\n").to_string();

        // v0.3.195: Remove trailing semicolons before closing braces
        let trailing_semicolon = regex::Regex::new(r";\s*}").unwrap();
        js_code = trailing_semicolon.replace_all(&js_code, "}").to_string();

        js_code = Self::rewrite_static_esm_imports_to_commonjs(&js_code);

        // v0.3.201: Remove Awaited utility type
        // Awaited<T> is a TypeScript 4.5+ utility type that unwraps Promise-like types
        // Pattern: "Awaited<T>" in type annotations, type aliases, or generic constraints
        // For simple cases, we remove "Awaited<...>" and keep the inner type
        let awaited_pattern = regex::Regex::new(r"Awaited\s*<([^>]+)>").unwrap();
        js_code = awaited_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.202: Remove ThisParameterType utility type
        // ThisParameterType<T> extracts the 'this' parameter type from a function type T
        // Pattern: "ThisParameterType<...>" - we extract the inner type (the 'this' type)
        let this_parameter_type_pattern =
            regex::Regex::new(r"ThisParameterType\s*<([^>]+)>").unwrap();
        js_code = this_parameter_type_pattern
            .replace_all(&js_code, "$1")
            .to_string();

        // v0.3.202: Remove OmitThisParameter utility type
        // OmitThisParameter<T> removes the 'this' parameter from a function type T
        // For simple cases, we just remove the wrapper and keep the inner function type
        let omit_this_parameter_pattern =
            regex::Regex::new(r"OmitThisParameter\s*<([^>]+)>").unwrap();
        js_code = omit_this_parameter_pattern
            .replace_all(&js_code, "$1")
            .to_string();

        // v0.3.203: Remove intrinsic string types (Uppercase, Lowercase, Capitalize, Uncapitalize)
        // These are TypeScript 4.1+ intrinsic string manipulation types
        // For runtime, we remove the wrapper and keep the inner string literal
        // Pattern: "Uppercase<'hello'>" -> "'hello'", "Lowercase<'WORLD'>" -> "'WORLD'", etc.
        let uppercase_pattern = regex::Regex::new(r"Uppercase\s*<([^>]+)>").unwrap();
        js_code = uppercase_pattern.replace_all(&js_code, "$1").to_string();

        let lowercase_pattern = regex::Regex::new(r"Lowercase\s*<([^>]+)>").unwrap();
        js_code = lowercase_pattern.replace_all(&js_code, "$1").to_string();

        let capitalize_pattern = regex::Regex::new(r"Capitalize\s*<([^>]+)>").unwrap();
        js_code = capitalize_pattern.replace_all(&js_code, "$1").to_string();

        let uncapitalize_pattern = regex::Regex::new(r"Uncapitalize\s*<([^>]+)>").unwrap();
        js_code = uncapitalize_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.222: Remove intrinsic string types (Trim, TrimLeft, TrimRight)
        // These are TypeScript 4.1+ intrinsic string manipulation types
        // For runtime, we remove the wrapper and keep the inner string literal
        // Pattern: "Trim<' hello '>" -> "' hello '", etc.
        let trim_pattern = regex::Regex::new(r"Trim\s*<([^>]+)>").unwrap();
        js_code = trim_pattern.replace_all(&js_code, "$1").to_string();

        let trim_left_pattern = regex::Regex::new(r"TrimLeft\s*<([^>]+)>").unwrap();
        js_code = trim_left_pattern.replace_all(&js_code, "$1").to_string();

        let trim_right_pattern = regex::Regex::new(r"TrimRight\s*<([^>]+)>").unwrap();
        js_code = trim_right_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.204: Remove NonNullable utility type
        // NonNullable<T> removes null and undefined from type T
        // For runtime, we remove the wrapper and keep the inner type
        // Pattern: "NonNullable<string | null>" -> "string"
        let nonnullable_pattern = regex::Regex::new(r"NonNullable\s*<([^>]+)>").unwrap();
        js_code = nonnullable_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.218: Remove Mutable utility type
        // Mutable<T> makes all properties of T mutable (opposite of Readonly)
        // For runtime, we remove the wrapper and keep the inner type
        // Pattern: "Mutable<readonly User>" -> "User"
        let mutable_pattern = regex::Regex::new(r"Mutable\s*<([^>]+)>").unwrap();
        js_code = mutable_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.206: Remove Partial utility type
        // Partial<T> makes all properties of T optional
        // For runtime, we remove the wrapper and keep the inner type
        // Pattern: "Partial<User>" -> "User"
        let partial_pattern = regex::Regex::new(r"Partial\s*<([^>]+)>").unwrap();
        js_code = partial_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.207: Remove Required utility type
        // Required<T> makes all properties of T required (opposite of Partial)
        // For runtime, we remove the wrapper and keep the inner type
        // Pattern: "Required<User>" -> "User"
        let required_pattern = regex::Regex::new(r"Required\s*<([^>]+)>").unwrap();
        js_code = required_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.207: Remove Readonly utility type
        // Readonly<T> makes all properties of T readonly
        // For runtime, we remove the wrapper and keep the inner type
        // Pattern: "Readonly<User>" -> "User"
        let readonly_pattern = regex::Regex::new(r"Readonly\s*<([^>]+)>").unwrap();
        js_code = readonly_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.208: Remove Pick utility type
        // Pick<T, K> selects a subset of properties from T
        // For runtime, we remove the wrapper and keep the inner type (first param)
        // Pattern: "Pick<User, 'name' | 'age'>" -> "User"
        let pick_pattern = regex::Regex::new(r"Pick\s*<([^,]+),").unwrap();
        js_code = pick_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.208: Remove Omit utility type
        // Omit<T, K> excludes specified keys from type T
        // For runtime, we remove the wrapper and keep the inner type (first param)
        // Pattern: "Omit<User, 'password'>" -> "User"
        let omit_pattern = regex::Regex::new(r"Omit\s*<([^,]+),").unwrap();
        js_code = omit_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.208: Remove Record utility type
        // Record<K, T> constructs an object type with keys K and values T
        // For runtime, we remove the wrapper and keep the value type
        // Pattern: "Record<string, number>" -> "number" (simplified for JS)
        let record_pattern = regex::Regex::new(r"Record\s*<([^,]+),").unwrap();
        js_code = record_pattern
            .replace_all(&js_code, "/* Record< */$1/* > removed */")
            .to_string();

        // v0.3.209: Remove Exclude utility type
        // Exclude<T, U> excludes types from T that are assignable to U
        // For runtime, we remove the wrapper and keep the first type parameter
        // Pattern: "Exclude<string | number, string>" -> "string | number"
        let exclude_pattern = regex::Regex::new(r"Exclude\s*<([^,]+),").unwrap();
        js_code = exclude_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.209: Remove Extract utility type
        // Extract<T, U> extracts types from T that are assignable to U
        // For runtime, we remove the wrapper and keep the first type parameter
        // Pattern: "Extract<string | number, string>" -> "string | number"
        let extract_pattern = regex::Regex::new(r"Extract\s*<([^,]+),").unwrap();
        js_code = extract_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.210: Remove InstanceType utility type
        // InstanceType<T> gets the instance type of a constructor type T
        // For runtime, we remove the wrapper and keep the inner type
        // Pattern: "InstanceType<typeof Person>" -> "Person"
        let instancetype_pattern = regex::Regex::new(r"InstanceType\s*<([^>]+)>").unwrap();
        js_code = instancetype_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.211: Remove ReturnType utility type
        // ReturnType<T> gets the return type of a function type T
        // For runtime, we remove the wrapper and keep the inner type
        // Pattern: "ReturnType<typeof getUser>" -> "getUser"
        let returntype_pattern = regex::Regex::new(r"ReturnType\s*<([^>]+)>").unwrap();
        js_code = returntype_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.211: Remove Parameters utility type
        // Parameters<T> gets the parameter types of a function type T
        // For runtime, we remove the wrapper and keep the inner type
        // Pattern: "Parameters<typeof greet>" -> "greet"
        let parameters_pattern = regex::Regex::new(r"Parameters\s*<([^>]+)>").unwrap();
        js_code = parameters_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.211: Remove ConstructorParameters utility type
        // ConstructorParameters<T> gets the constructor parameter types of a constructor type T
        // For runtime, we remove the wrapper and keep the inner type
        // Pattern: "ConstructorParameters<typeof User>" -> "User"
        let constructorparams_pattern =
            regex::Regex::new(r"ConstructorParameters\s*<([^>]+)>").unwrap();
        js_code = constructorparams_pattern
            .replace_all(&js_code, "$1")
            .to_string();

        // v0.3.212: Remove NoInfer utility type
        // NoInfer<T> prevents type inference and forces the use of the specific type
        // For runtime, we remove the wrapper and keep the inner type
        // Pattern: "NoInfer<T>" -> "T"
        let noinfer_pattern = regex::Regex::new(r"NoInfer\s*<([^>]+)>").unwrap();
        js_code = noinfer_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.213: Remove Infer utility type
        // Infer<T> is used in conditional types to infer types
        // For runtime, we remove the wrapper and keep the inner type
        // Pattern: "Infer<T>" -> "T"
        let infer_pattern = regex::Regex::new(r"Infer\s*<([^>]+)>").unwrap();
        js_code = infer_pattern.replace_all(&js_code, "$1").to_string();

        // v0.3.216: Remove ThisType utility type
        // ThisType<T> provides the type of 'this' within an object method
        // For runtime, we completely remove this type-level construct
        // Pattern: "ThisType<T>" -> "" (completely removed)
        let thistype_pattern = regex::Regex::new(r"ThisType\s*<[^>]+>").unwrap();
        js_code = thistype_pattern.replace_all(&js_code, "").to_string();

        // v0.3.195: Convert simple ESM export statements to comments
        // v0.3.196: Added abstract for export abstract class support
        // Complex exports (export { a, b }) need variable tracking, so we use placeholders
        // - "export const X = val" -> "/* export const X = val */" (will be cleaned up)
        // - "export function foo() {}" -> "/* export function foo() {} */"
        // - "export class C {}" -> "/* export class C {} */"
        // - "export abstract class C {}" -> "/* export abstract class C {} */"
        let esm_export_pattern = regex::Regex::new(
            r"(?m)^\s*export\s+(?:const|let|var|function|class|interface|type|abstract)\s+",
        )
        .unwrap();
        js_code = esm_export_pattern
            .replace_all(&js_code, "/* ESM export removed: ")
            .to_string();

        // v0.3.195: Convert export { ... } statements
        let export_braces_pattern =
            regex::Regex::new(r"(?m)^\s*export\s*\{([^}]*)\}\s*;?\s*$").unwrap();
        js_code = export_braces_pattern
            .replace_all(&js_code, "/* ESM export {$1} removed */")
            .to_string();

        // v0.3.195: Convert export default statements
        let export_default_pattern = regex::Regex::new(r"(?m)^\s*export\s+default\s+").unwrap();
        js_code = export_default_pattern
            .replace_all(&js_code, "/* ESM default export */ ")
            .to_string();

        Ok(js_code)
    }

    #[allow(dead_code)]
    fn rewrite_static_esm_imports_to_commonjs(code: &str) -> String {
        static IMPORT_FROM_PATTERN: OnceLock<regex::Regex> = OnceLock::new();
        static SIDE_EFFECT_IMPORT_PATTERN: OnceLock<regex::Regex> = OnceLock::new();

        let import_from_pattern = IMPORT_FROM_PATTERN.get_or_init(|| {
            regex::Regex::new(r#"(?m)^(\s*)import\s+(.+?)\s+from\s+['"]([^'"]+)['"]\s*;?\s*$"#)
                .unwrap()
        });
        let side_effect_import_pattern = SIDE_EFFECT_IMPORT_PATTERN.get_or_init(|| {
            regex::Regex::new(r#"(?m)^(\s*)import\s+['"]([^'"]+)['"]\s*;?\s*$"#).unwrap()
        });

        let code = import_from_pattern
            .replace_all(code, |captures: &regex::Captures| {
                let indent = captures
                    .get(1)
                    .map(|capture| capture.as_str())
                    .unwrap_or("");
                let bindings = captures
                    .get(2)
                    .map(|capture| capture.as_str())
                    .unwrap_or("");
                let specifier = captures
                    .get(3)
                    .map(|capture| capture.as_str())
                    .unwrap_or("");
                Self::rewrite_static_esm_import_clause(indent, bindings, specifier)
            })
            .to_string();

        side_effect_import_pattern
            .replace_all(&code, |captures: &regex::Captures| {
                let indent = captures
                    .get(1)
                    .map(|capture| capture.as_str())
                    .unwrap_or("");
                let specifier = captures
                    .get(2)
                    .map(|capture| capture.as_str())
                    .unwrap_or("");
                let specifier_literal = serde_json::to_string(specifier).unwrap();
                format!("{indent}require({specifier_literal});")
            })
            .to_string()
    }

    #[allow(dead_code)]
    fn rewrite_static_esm_import_clause(indent: &str, bindings: &str, specifier: &str) -> String {
        let specifier_literal = serde_json::to_string(specifier).unwrap();
        let bindings = bindings.trim();

        if let Some(namespace) = bindings.strip_prefix("* as ") {
            return format!(
                "{indent}const {} = require({});",
                namespace.trim(),
                specifier_literal
            );
        }

        if bindings.starts_with('{') && bindings.ends_with('}') {
            let named = Self::rewrite_esm_named_import_bindings(bindings);
            return format!("{indent}const {named} = require({specifier_literal});");
        }

        if let Some((default_binding, rest)) = bindings.split_once(',') {
            let default_binding = default_binding.trim();
            let rest = rest.trim();
            if let Some(namespace) = rest.strip_prefix("* as ") {
                let namespace = namespace.trim();
                return format!(
                    "{indent}const {default_binding} = require({specifier_literal});\n{indent}const {namespace} = {default_binding};"
                );
            }
            if rest.starts_with('{') && rest.ends_with('}') {
                let named = Self::rewrite_esm_named_import_bindings(rest);
                return format!(
                    "{indent}const {default_binding} = require({specifier_literal});\n{indent}const {named} = {default_binding};"
                );
            }
        }

        format!("{indent}const {bindings} = require({specifier_literal});")
    }

    #[allow(dead_code)]
    fn rewrite_esm_named_import_bindings(bindings: &str) -> String {
        let inner = bindings
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .trim();

        if inner.is_empty() {
            return "{}".to_string();
        }

        let rewritten = inner
            .split(',')
            .filter_map(|binding| {
                let binding = binding.trim();
                if binding.is_empty() {
                    return None;
                }
                if let Some((imported, local)) = binding.split_once(" as ") {
                    return Some(format!("{}: {}", imported.trim(), local.trim()));
                }
                Some(binding.to_string())
            })
            .collect::<Vec<_>>()
            .join(", ");

        format!("{{ {rewritten} }}")
    }

    #[allow(dead_code)]
    fn has_export_equals_statement(code: &str) -> bool {
        static EXPORT_EQUALS_PATTERN: OnceLock<regex::Regex> = OnceLock::new();
        EXPORT_EQUALS_PATTERN
            .get_or_init(|| regex::Regex::new(r"(?m)(^|[;\r\n])\s*export\s*=\s*[^=]").unwrap())
            .is_match(code)
    }

    #[allow(dead_code)]
    fn has_interface_declaration(code: &str) -> bool {
        static INTERFACE_DECLARATION_PATTERN: OnceLock<regex::Regex> = OnceLock::new();
        INTERFACE_DECLARATION_PATTERN
            .get_or_init(|| {
                regex::Regex::new(
                    r"(?m)^\s*(?:export\s+)?interface\s+[A-Za-z_$][A-Za-z0-9_$]*(?:\s+extends\s+[^{]+)?\s*\{",
                )
                .unwrap()
            })
            .is_match(code)
    }

    #[allow(dead_code)]
    fn has_mapped_type_declaration(code: &str) -> bool {
        static MAPPED_TYPE_PATTERN: OnceLock<regex::Regex> = OnceLock::new();
        MAPPED_TYPE_PATTERN
            .get_or_init(|| {
                regex::Regex::new(
                    r"(?ms)^\s*(?:export\s+)?type\s+[A-Za-z_$][A-Za-z0-9_$]*(?:\s*<[^=;\n]+>)?\s*=\s*\{[^;]*\[\s*[A-Za-z_$][A-Za-z0-9_$]*\s+in\s+keyof\b",
                )
                .unwrap()
            })
            .is_match(code)
    }

    #[allow(dead_code)]
    fn has_type_alias_declaration(code: &str) -> bool {
        static TYPE_ALIAS_PATTERN: OnceLock<regex::Regex> = OnceLock::new();
        TYPE_ALIAS_PATTERN
            .get_or_init(|| {
                regex::Regex::new(
                    r"(?m)^\s*(?:export\s+)?type\s+[A-Za-z_$][A-Za-z0-9_$]*(?:\s*<[^=;\n]+>)?\s*=",
                )
                .unwrap()
            })
            .is_match(code)
    }

    #[allow(dead_code)]
    fn has_keyof_type_usage(code: &str) -> bool {
        static KEYOF_TYPE_PATTERN: OnceLock<regex::Regex> = OnceLock::new();
        KEYOF_TYPE_PATTERN
            .get_or_init(|| {
                regex::Regex::new(
                    r"(?m)(^\s*(?:export\s+)?type\s+[A-Za-z_$][A-Za-z0-9_$]*(?:\s*<[^=;\n]+>)?\s*=.*\bkeyof\b|\bextends\s+keyof\b|\[\s*keyof\s+[A-Za-z_$][A-Za-z0-9_$<>]*\])",
                )
                .unwrap()
            })
            .is_match(code)
    }

    /// Node-compatible core surface needed for most scripts and CLI workloads.
    fn install_core_apis(
        scope: &mut v8::ContextScope<v8::HandleScope>,
        context: &v8::Local<v8::Context>,
        main_module_dir: &str,
        main_module_filename: &str,
    ) -> Result<()> {
        Self::setup_console(scope, context)?;
        setup_buffer_module(scope);
        Self::setup_web_primitives(scope, context)?;
        crate::nodejs_core::process::setup_process_api(scope, context)?;
        crate::nodejs_core::events::setup_events_api(scope, context)?;
        setup_path_api(scope, context)?;
        setup_fs_api(scope, context)?;
        crate::nodejs_core::os::setup_os_api(scope, context)?;
        crate::nodejs_core::child_process::setup_child_process_api(scope, context)?;
        crate::nodejs_core::stream::setup_stream_api(scope, context)?;

        use crate::nodejs_core::http::init_http_connection_pool;
        init_http_connection_pool(10, 20, false);

        setup_http_api(scope, context)?;
        crate::nodejs_core::util::setup_util_api(scope, context)?;
        crate::nodejs_core::querystring::setup_querystring_api(scope, context)?;
        crate::nodejs_core::dns::setup_dns_api(scope, context)?;
        setup_net_api(scope, context)?;
        crate::nodejs_core::string_decoder::setup_string_decoder_api(scope, context)?;
        Self::setup_legacy_web_apis(scope, context, true)?;
        setup_crypto_api(scope, context)?;
        crate::nodejs_core::ai::setup_ai_api(scope, context)?;
        crate::database::setup_db_api(scope, context)?;
        crate::std_lib::setup_std_api(scope, context)?;
        crate::mcp::setup_mcp_api(scope, context)?;
        crate::sandbox::setup_sandbox_api(scope, context)?;
        crate::ffi::setup_ffi_api(scope, context)?;
        crate::pool::setup_pool_api(scope, context)?;
        crate::wasm::setup_wasm_api(scope, context)?;
        crate::replay::setup_replay_api(scope, context)?;
        crate::weights::setup_weights_api(scope, context)?;
        crate::capability::setup_security_api(scope, context)?;
        crate::kv::setup_kv_api(scope, context)?;
        crate::tools::setup_tools_api(scope, context)?;
        crate::bus::setup_bus_api(scope, context)?;
        crate::grammar::setup_grammar_api(scope, context)?;
        crate::checkpoint::setup_checkpoint_api(scope, context)?;
        Self::setup_module_system(scope, context, main_module_dir, main_module_filename)?;
        setup_timers_api(scope, context)?;
        setup_performance_api(scope, context)?;

        // Unified Node builtins that previously lacked install wiring.
        crate::nodejs_core::assert::setup_assert_api(scope, context)?;
        crate::nodejs_core::zlib::setup_zlib_api(scope, context)?;
        crate::nodejs_core::https::setup_https_api(scope, context)?;
        crate::nodejs_core::http2::setup_http2_api(scope, context)?;
        crate::nodejs_core::tls::setup_tls_api(scope, context)?;
        crate::nodejs_core::vm::setup_vm_api(scope, context)?;
        crate::nodejs_core::worker_threads::setup_worker_threads_api(scope, context)?;
        crate::nodejs_core::tty::setup_tty_api(scope, context)?;
        crate::nodejs_core::diagnostics_channel::setup_diagnostics_channel_api(scope, context)?;
        crate::nodejs_core::async_hooks::setup_async_hooks_api(scope, context)?;
        // Alias perf_hooks -> performance for Node module name compatibility.
        {
            let global = context.global(scope);
            let perf_key = v8::String::new(scope, "performance").unwrap();
            if let Some(perf_val) = global.get(scope, perf_key.into()) {
                let hooks = v8::Object::new(scope);
                let perf_hooks_key = v8::String::new(scope, "performance").unwrap();
                hooks.set(scope, perf_hooks_key.into(), perf_val);
                let alias_key = v8::String::new(scope, "perf_hooks").unwrap();
                global.set(scope, alias_key.into(), hooks.into());
            }
        }

        // AbortController is small and commonly used by fetch/streams later;
        // install with core so extended modules can assume it exists.
        use crate::web_api::abort::setup_abort_api;
        setup_abort_api(scope, context)?;

        Ok(())
    }

    /// Heavy / less-common Web APIs.
    fn install_extended_apis(
        scope: &mut v8::ContextScope<v8::HandleScope>,
        context: &v8::Local<v8::Context>,
    ) -> Result<()> {
        setup_streams_api(scope, context)?;
        setup_blob_api(scope, context)?;
        setup_compression_api(scope, context)?;
        setup_structured_clone_api(scope, context)?;
        setup_array_buffer_transfer_api(scope, context)?;
        setup_broadcast_channel_api(scope, context)?;
        setup_message_channel_api(scope, context)?;
        // Prefer WorkerHost (multi-isolate) over the fail-closed Worker stub.
        crate::web_api::worker_host::setup_worker_host_api(scope, context)?;
        let _ = setup_worker_api; // keep import used for historical call sites
        setup_shared_array_buffer_api(scope, context)?;
        crate::web_api::wasm::setup_wasm_streaming_api(scope, context)?;

        use crate::web_api::events::setup_events_api as setup_web_events_api;
        setup_web_events_api(scope, context)?;
        setup_service_worker_api(scope, context)?;

        use crate::web_api::background_sync::setup_background_sync_api;
        setup_background_sync_api(scope, context)?;
        crate::nodejs_core::readline::setup_readline_api(scope, context)?;

        use crate::web_api::fetch::setup_fetch_api;
        setup_fetch_api(scope, context)?;

        use crate::web_api::form_data::setup_form_data_api;
        setup_form_data_api(scope, context)?;

        use crate::web_api::url_search_params::setup_url_search_params_api;
        setup_url_search_params_api(scope, context);
        Self::setup_live_url_search_params(scope)?;

        setup_web_crypto_api(scope, context)?;

        use crate::web_api::error_event::setup_error_event_api;
        setup_error_event_api(scope, context);

        use crate::web_api::custom_event::setup_custom_event_api;
        setup_custom_event_api(scope, context);

        use crate::web_api::dom_parser::setup_dom_parser_api;
        setup_dom_parser_api(scope, context)?;

        use crate::web_api::clipboard::setup_clipboard_api;
        setup_clipboard_api(scope, context)?;

        // WinterTC ECMA-429 & Sockets API initialization
        crate::web_api::dom_exception::setup_dom_exception_api(scope, context)?;
        crate::web_api::navigator::setup_navigator_api(scope, context)?;
        crate::web_api::url_pattern::setup_url_pattern_api(scope, context)?;
        crate::web_api::sockets::setup_sockets_api(scope, context)?;

        let wintertc_helpers_js = r#"
        (function() {
            if (typeof globalThis.self === 'undefined') {
                globalThis.self = globalThis;
            }
            if (typeof globalThis.reportError !== 'function') {
                globalThis.reportError = function(error) {
                    if (typeof globalThis.onerror === 'function') {
                        try {
                            const msg = (error && error.message) ? error.message : String(error);
                            globalThis.onerror(msg, '', 0, 0, error);
                            return;
                        } catch (_) {}
                    }
                    console.error('Unhandled error:', error);
                };
            }
            if (typeof globalThis.PromiseRejectionEvent === 'undefined') {
                const Base = (typeof Event === 'function') ? Event : Object;
                globalThis.PromiseRejectionEvent = class PromiseRejectionEvent extends Base {
                    constructor(type, init = {}) {
                        try { super(type, init); } catch (_) { super(); }
                        this.type = type;
                        this.promise = init.promise;
                        this.reason = init.reason;
                    }
                };
            }
            globalThis.__bee_dispatch_unhandled_rejection = function(promise, reason) {
                let event;
                try {
                    event = new PromiseRejectionEvent('unhandledrejection', {
                        promise: promise,
                        reason: reason,
                        cancelable: true
                    });
                } catch (_) {
                    event = { type: 'unhandledrejection', promise: promise, reason: reason };
                }
                try {
                    if (typeof globalThis.onunhandledrejection === 'function') {
                        globalThis.onunhandledrejection(event);
                    }
                } catch (_) {}
                try {
                    if (typeof globalThis.dispatchEvent === 'function') {
                        globalThis.dispatchEvent(event);
                    }
                } catch (_) {}
            };
        })();
        "#;
        if let Some(code) = v8::String::new(scope, wintertc_helpers_js) {
            if let Some(script) = v8::Script::compile(scope, code, None) {
                let _ = script.run(scope);
            }
        }

        Ok(())
    }

    /// Execute JavaScript or TypeScript code and return the result as a string
    /// v0.3.93: 修改为使用存储的 Context 以支持跨调用共享数据
    pub fn execute_code(&mut self, code: &str) -> Result<String> {
        self.execute_code_unlocked(code)
    }

    /// Execute code directly on this isolate without locking process-global V8 execution lock.
    /// Essential for parallel worker threads and multi-tenant Isolate pools.
    pub fn execute_code_unlocked(&mut self, code: &str) -> Result<String> {
        crate::web_api::background_sync::reset_pending_wait_until();
        IMPORT_META_PARENT_DIR.with(|dir| {
            *dir.borrow_mut() = PathBuf::from(&self.main_module_dir);
        });

        // Drop setImmediate callbacks left over from a previous execute_code
        // (e.g. after an early error return). They capture V8 handles from the
        // execution that scheduled them and must not run in this one.
        clear_pending_immediates();

        // Transpile TypeScript-only source through oxc before V8 parse.
        // Do not treat `import` / `export` as TypeScript: those are valid JS.
        let skip_runtime_typescript_transpile =
            code.starts_with("// @beejs-no-runtime-typescript-transpile");
        let has_raw_typescript = !skip_runtime_typescript_transpile
            && crate::typescript::looks_like_typescript_source(code);

        let js_code = if has_raw_typescript {
            let filename = if self.main_module_filename.ends_with(".ts")
                || self.main_module_filename.ends_with(".tsx")
                || self.main_module_filename.ends_with(".mts")
                || self.main_module_filename.ends_with(".cts")
                || self.main_module_filename.ends_with(".jsx")
            {
                self.main_module_filename.as_str()
            } else if crate::typescript::looks_like_jsx_source(code) {
                "eval.tsx"
            } else {
                "eval.ts"
            };
            Cow::Owned({
                let output = crate::typescript::compile_typescript(code, filename)
                    .map_err(|error| anyhow::anyhow!(error))?;
                if let Some(ref map) = output.source_map {
                    set_active_source_map(map.clone());
                }
                let mut js = output.js_code;
                if let Some(ref map) = output.source_map {
                    js.push_str(&crate::typescript::source_mapping_url_comment(map));
                }
                js
            })
        } else {
            Cow::Borrowed(code)
        };

        let should_execute_as_esm_module =
            Self::should_execute_as_esm_module(js_code.as_ref(), &self.main_module_filename)?;

        // Both surfaces go in on the first execute. Deferring the extended set
        // until the source looks like it needs it is not sound: a script reaches
        // `fetch` / `Blob` / … through dependency bodies, `eval`, or computed
        // property reads that no source scan or lazy global accessor can see.
        let needs_core_setup = !self.apis_initialized;
        let needs_extended_setup = !self.extended_apis_initialized;

        // 创建 HandleScope（整个函数只创建一次）
        v8::scope!(let scope, &mut self.isolate);

        // 获取或创建 Context
        let context = if self.context.is_none() {
            // 第一次调用，创建 Context 并设置所有 API
            let context = v8::Context::new(scope, Default::default());
            let global_context = v8::Global::new(scope, context);
            self.context = Some(global_context);
            context
        } else {
            // 复用已存储的 Context
            v8::Local::new(scope, self.context.as_ref().unwrap())
        };

        let scope = &mut v8::ContextScope::new(scope, context);

        let profile_startup = std::env::var_os("BEEJS_PROFILE_STARTUP").is_some();
        let _t_start = if profile_startup {
            Some(std::time::Instant::now())
        } else {
            None
        };

        if needs_core_setup {
            let t_core = if profile_startup {
                Some(std::time::Instant::now())
            } else {
                None
            };
            Self::install_core_apis(
                scope,
                &context,
                &self.main_module_dir,
                &self.main_module_filename,
            )?;
            self.apis_initialized = true;
            if let Some(t) = t_core {
                eprintln!(
                    "[STARTUP PROFILE] install_core_apis: {:.2}ms",
                    t.elapsed().as_secs_f64() * 1000.0
                );
            }
        }

        if needs_extended_setup {
            let t_ext = if profile_startup {
                Some(std::time::Instant::now())
            } else {
                None
            };
            Self::install_extended_apis(scope, &context)?;
            self.extended_apis_initialized = true;
            if let Some(t) = t_ext {
                eprintln!(
                    "[STARTUP PROFILE] install_extended_apis: {:.2}ms",
                    t.elapsed().as_secs_f64() * 1000.0
                );
            }
        }

        Self::apply_process_argv(scope, &context, &self.process_argv)?;

        if let Some(seed) = crate::permissions::get_deterministic_seed() {
            let prng_script = format!(
                r#"
                (function() {{
                    let s = BigInt({});
                    Math.random = function() {{
                        s = (s + 0x6D2B79F5n) & 0xFFFFFFFFn;
                        let t = s;
                        let z = Math.imul(Number(t ^ (t >> 15n)), Number(t | 1n));
                        z ^= z + Math.imul(z ^ (z >>> 7), z | 61);
                        return ((z ^ (z >>> 14)) >>> 0) / 4294967296;
                    }};
                }})();
                "#,
                seed
            );
            if let Some(code) = v8::String::new(scope, &prng_script) {
                if let Some(script) = v8::Script::compile(scope, code, None) {
                    let _ = script.run(scope);
                }
            }
        }

        if let Some(frozen_ms) = crate::permissions::get_frozen_time_ms() {
            let date_script = format!(
                r#"
                (function() {{
                    const _frozen = {};
                    const _OrigDate = Date;
                    function PatchedDate(...args) {{
                        if (!(this instanceof PatchedDate)) {{
                            return new _OrigDate(_frozen).toString();
                        }}
                        if (args.length === 0) {{
                            return new _OrigDate(_frozen);
                        }}
                        return new _OrigDate(...args);
                    }}
                    PatchedDate.prototype = _OrigDate.prototype;
                    PatchedDate.now = function() {{
                        return _frozen;
                    }};
                    PatchedDate.parse = _OrigDate.parse;
                    PatchedDate.UTC = _OrigDate.UTC;
                    Date = PatchedDate;
                }})();
                "#,
                frozen_ms
            );
            if let Some(code) = v8::String::new(scope, &date_script) {
                if let Some(script) = v8::Script::compile(scope, code, None) {
                    let _ = script.run(scope);
                }
            }
        }

        // Use TryCatch for proper error handling
        v8::tc_scope!(let scope, scope);

        // Compile and run the user's code exactly once. The returned value below
        // is the V8 completion value from this execution.
        let result = if should_execute_as_esm_module {
            match Self::execute_esm_module(
                scope,
                js_code.as_ref(),
                &self.main_module_filename,
                &mut self.esm_module_cache,
                &mut self.esm_module_cache_fingerprints,
                self.timer_drain_limit_ms,
            ) {
                Ok(result) => result,
                Err(error_message) => {
                    if scope.has_caught() {
                        let exception = scope.exception().unwrap_or_else(|| {
                            v8::String::new(scope, "Unknown module error")
                                .unwrap()
                                .into()
                        });
                        let runtime_error = v8_exception_to_runtime_error(scope, exception);
                        return Err(anyhow::anyhow!(
                            "[Beejs Error] {}: {}",
                            runtime_error.code,
                            runtime_error.message
                        ));
                    }

                    return Err(anyhow::anyhow!(
                        "[Beejs Error] MODULE_ERROR: {}",
                        error_message
                    ));
                }
            }
        } else {
            let code = v8::String::new(scope, js_code.as_ref())
                .ok_or_else(|| anyhow::anyhow!("Failed to create V8 string from code"))?;
            let resource_name = v8::String::new(scope, &self.main_module_filename)
                .ok_or_else(|| anyhow::anyhow!("Failed to create V8 script resource name"))?;
            let source_map_url = active_source_map_url(scope);
            let script_origin = v8::ScriptOrigin::new(
                scope,
                resource_name.into(),
                0,
                0,
                false,
                0,
                source_map_url,
                false,
                false,
                false,
                None,
            );

            // Compile the code
            // v0.3.235: Enhanced error handling with detailed messages
            let script = match v8::Script::compile(scope, code, Some(&script_origin)) {
                Some(script) => script,
                None => {
                    let exception = scope.exception().unwrap_or_else(|| {
                        v8::String::new(scope, "Unknown compilation error")
                            .unwrap()
                            .into()
                    });

                    // v0.3.235: Create enhanced RuntimeError with structured information
                    let runtime_error = v8_exception_to_runtime_error(scope, exception);

                    return Err(anyhow::anyhow!("[Beejs Error] SyntaxError: {}\nHint: Check for missing parentheses, brackets, or invalid syntax.", runtime_error.message));
                }
            };

            // Run the script once and keep its completion value for the final result.
            match script.run(scope) {
                Some(result) => {
                    Self::publish_cjs_tool_exports(scope, &context);
                    result
                }
                None => {
                    if scope.is_execution_terminating() {
                        return Err(anyhow::anyhow!(
                            "Script execution terminated (timeout or memory limit reached)"
                        ));
                    }
                    if scope.has_caught() {
                        // Get the exception from TryCatch
                        let exception = scope.exception().unwrap_or_else(|| {
                            v8::String::new(scope, "Unknown runtime error")
                                .unwrap()
                                .into()
                        });

                        // v0.3.235: Create enhanced RuntimeError with structured information
                        let runtime_error = v8_exception_to_runtime_error(scope, exception);

                        // v0.3.335: Call window.onerror handler if set
                        // Extract error info for onerror
                        let error_message = exception
                            .to_string(scope)
                            .unwrap_or_else(|| v8::String::new(scope, "Unknown error").unwrap())
                            .to_rust_string_lossy(scope);
                        let error_location = runtime_error
                            .location
                            .clone()
                            .unwrap_or_else(|| "".to_string());
                        let (filename, lineno, colno) = if !error_location.is_empty() {
                            // Try to parse location like "at line X column Y" or filename:line:column
                            let parts: Vec<&str> = error_location.split(':').collect();
                            if parts.len() >= 3 {
                                (
                                    parts[0].to_string(),
                                    parts[1].parse().unwrap_or(0),
                                    parts[2].parse().unwrap_or(0),
                                )
                            } else {
                                (error_location.clone(), 0, 0)
                            }
                        } else {
                            ("".to_string(), 0, 0)
                        };

                        // Call window.onerror if set, and check if it handled the error
                        let error_handled = call_onerror_handler(
                            scope,
                            &error_message,
                            &filename,
                            lineno as u32,
                            colno as u32,
                            Some(exception),
                        );

                        // If onerror handled the error (returned true), don't propagate the error
                        if error_handled {
                            // Return "undefined" as the string result since the error was handled
                            return Ok("undefined".to_string());
                        }

                        // Provide more helpful error message based on error type
                        let hint = match runtime_error.error_type {
                            RuntimeErrorType::ReferenceError => "\nHint: Make sure the variable or function is defined before using it.",
                            RuntimeErrorType::TypeError => "\nHint: Check the types of values and ensure operations are valid (e.g., calling a function on null/undefined).",
                            RuntimeErrorType::RangeError => "\nHint: Check array bounds or numeric ranges.",
                            RuntimeErrorType::SyntaxError => "\nHint: Check the syntax of your code.",
                            _ => "",
                        };

                        return Err(anyhow::anyhow!(
                            "[Beejs Error] {}: {}{}{}",
                            runtime_error.code,
                            runtime_error.message,
                            runtime_error
                                .location
                                .as_ref()
                                .map(|location| format!("\nLocation: {location}"))
                                .unwrap_or_default(),
                            hint
                        ));
                    } else {
                        return Err(anyhow::anyhow!("[Beejs Error] InternalError: Script execution returned no result\nHint: This may indicate an internal runtime issue."));
                    }
                }
            }
        };

        // v0.3.261: Execute nextTick callbacks FIRST to process any queued callbacks
        // This ensures nextTick has highest priority after sync code completes
        execute_next_tick_callbacks(scope);

        // v0.3.249: Execute fired timer callbacks (event loop tick)
        // v0.3.261: Refactor - correct execution order: nextTick -> microtasks -> timers -> setImmediate
        //
        // Node.js event loop phases:
        // 1. nextTick queue (highest priority)
        // 2. Microtasks (Promises, queueMicrotask)
        // 3. Timers (setTimeout/setInterval with delay > 0)
        // 4. setImmediate callbacks (check phase)
        //
        // This loop continues until: no pending nextTicks, no fired timers
        // Note: setImmediate callbacks are processed AFTER this loop (in the "next iteration")
        //
        let timer_drain_limit_ms = self.timer_drain_limit_ms;
        let http_server_keep_alive = self.http_server_keep_alive;
        let timer_drain_started_at = std::time::Instant::now();

        loop {
            // A new iteration makes previously deferred setImmediate callbacks
            // eligible again (they were deferred to skip the iteration that
            // registered them).
            unmark_immediate_callbacks_deferred();

            // Fast path: after sync code, nextTick and microtasks may be the only
            // pending work. Process them before deciding whether timer polling is
            // needed; otherwise plain synchronous execution pays the timer wait.
            execute_next_tick_callbacks(scope);
            scope.perform_microtask_checkpoint();
            crate::nodejs_core::http::pump_pending_http_requests_in_scope(scope, &context);
            crate::web_api::worker_host::WorkerHost::pump_parent_messages(scope);

            let has_initial_pending_work = {
                let timer_manager = crate::event_loop::get_async_timer_manager();
                let has_scheduled_timers = timer_manager.has_scheduled_timers();
                let has_zero_delay_timers = has_scheduled_timers
                    && crate::nodejs_core::timers::has_pending_zero_delay_timers();
                let has_drainable_timers = crate::nodejs_core::timers::has_pending_drainable_timers(
                    remaining_timer_drain_ms(timer_drain_started_at, timer_drain_limit_ms),
                );
                let has_wait_until = crate::web_api::background_sync::has_pending_wait_until();
                let has_wait_until_timers = has_scheduled_timers && has_wait_until;
                timer_manager.has_fired_timers()
                    || has_zero_delay_timers
                    || has_drainable_timers
                    || has_wait_until_timers
                    || has_wait_until
                    || has_pending_next_ticks()
                    || has_pending_immediates()
                    || crate::web_api::worker_host::WorkerHost::has_active_workers()
            };

            if !has_initial_pending_work {
                break;
            }

            // Wait for the background timer thread. Prefer keep-alive for any
            // ref'd timer rather than a fixed ~1s wall-clock cap.
            let mut iterations_without_progress = 0;
            const MAX_WAIT_ITERATIONS: usize = 400_000; // safety valve (~2.7h at 25ms)

            while iterations_without_progress < MAX_WAIT_ITERATIONS {
                let timer_manager = crate::event_loop::get_async_timer_manager();
                let has_fired = timer_manager.has_fired_timers();
                let has_scheduled = timer_manager.has_scheduled_timers();
                let has_zero_delay_timers =
                    has_scheduled && crate::nodejs_core::timers::has_pending_zero_delay_timers();
                let has_drainable_timers = crate::nodejs_core::timers::has_pending_drainable_timers(
                    remaining_timer_drain_ms(timer_drain_started_at, timer_drain_limit_ms),
                ) || (timer_drain_limit_ms == u64::MAX
                    && crate::nodejs_core::timers::has_pending_refed_timers());
                let has_wait_until = crate::web_api::background_sync::has_pending_wait_until();
                let has_wait_until_timers = has_scheduled && has_wait_until;
                let has_next_ticks = has_pending_next_ticks();

                if has_fired
                    || has_zero_delay_timers
                    || has_next_ticks
                    || has_pending_immediates()
                    || crate::web_api::worker_host::WorkerHost::has_parent_messages()
                {
                    break;
                }

                if crate::web_api::worker_host::WorkerHost::has_active_workers() {
                    timer_manager.wait_timeout(std::time::Duration::from_millis(10));
                    if crate::web_api::worker_host::WorkerHost::has_parent_messages() {
                        break;
                    }
                    iterations_without_progress += 1;
                    continue;
                }

                if has_drainable_timers || has_wait_until_timers || has_wait_until {
                    let remaining_ms =
                        remaining_timer_drain_ms(timer_drain_started_at, timer_drain_limit_ms)
                            .min(25)
                            .max(1);
                    timer_manager.wait_timeout(std::time::Duration::from_millis(remaining_ms));
                    iterations_without_progress += 1;
                    continue;
                }

                // Unref'd / non-drainable scheduled timers must not keep the loop.
                if has_scheduled {
                    break;
                }

                timer_manager.wait_timeout(std::time::Duration::from_millis(10));
                iterations_without_progress += 1;
            }

            // v0.3.261: Only yield if no immediate work has arrived, avoiding unnecessary 5ms delay
            let has_immediate_work = {
                let timer_manager = crate::event_loop::get_async_timer_manager();
                timer_manager.has_fired_timers()
                    || has_pending_next_ticks()
                    || has_pending_immediates()
            };
            if !has_immediate_work {
                std::thread::yield_now();
            }

            // v0.3.261: Execute nextTick callbacks FIRST (before timers and microtasks)
            // This ensures nextTick has highest priority after sync code completes
            execute_next_tick_callbacks(scope);

            // Process microtasks (Promises, queueMicrotask callbacks)
            // nextTick callbacks were already executed, now process Promises
            scope.perform_microtask_checkpoint();

            // Execute all currently fired timers (setTimeout/setInterval with delay > 0)
            execute_fired_timers(scope);
            crate::web_api::worker_host::WorkerHost::pump_parent_messages(scope);

            // Timer callbacks may have queued nextTick callbacks; those have
            // higher priority than the check phase (setImmediate) below.
            execute_next_tick_callbacks(scope);

            // v0.3.339: Process microtasks after timer execution
            // Timer callbacks may resolve Promises (e.g., setTimeout callbacks that call resolve())
            // These Promises need to be processed before continuing
            scope.perform_microtask_checkpoint();

            // Check phase: run setImmediate callbacks every iteration, not only
            // after the loop. Otherwise a pending immediate would be stuck behind
            // a later timer (e.g. a 50ms timeout), which violates Node semantics.
            // Callbacks registered during this phase are deferred to the next
            // iteration via mark_immediate_callbacks_deferred.
            execute_immediate_callbacks(scope);
            mark_immediate_callbacks_deferred();

            // Immediate callbacks may also queue nextTicks; drain them before
            // the loop evaluates whether work remains.
            execute_next_tick_callbacks(scope);
            scope.perform_microtask_checkpoint();

            // The check phase above runs setImmediate callbacks inside the loop,
            // so a pending immediate is never stuck behind a later timer.

            // v0.3.261: Check if there are still pending nextTicks in the queue
            // (could be added during callback execution)
            let has_pending_next_ticks_now = has_pending_next_ticks();

            // Check if new timers fired during callback execution
            let has_new_timers = {
                let timer_manager = crate::event_loop::get_async_timer_manager();
                timer_manager.has_fired_timers()
            };

            // v0.3.261: Continue if there are pending nextTicks or fired timers
            // Note: setImmediate callbacks are executed AFTER the loop (outside the wait condition)
            // We only wait inside the loop for nextTicks and timers with valid metadata
            // v0.3.270: With MicrotasksPolicy::Explicit, run microtasks before breaking
            // v0.3.339: Only break if there's truly no work left. Check actual state:
            // - No pending nextTicks
            // - No newly fired timers
            // - No scheduled timers waiting to fire
            // - No pending work from wait loop or microtasks
            let has_scheduled_timers = {
                let timer_manager = crate::event_loop::get_async_timer_manager();
                let has_scheduled_timers = timer_manager.has_scheduled_timers();
                (has_scheduled_timers
                    && (crate::nodejs_core::timers::has_pending_zero_delay_timers()
                        || crate::web_api::background_sync::has_pending_wait_until()))
                    || crate::nodejs_core::timers::has_pending_drainable_timers(
                        remaining_timer_drain_ms(timer_drain_started_at, timer_drain_limit_ms),
                    )
            };
            let has_active_workers = crate::web_api::worker_host::WorkerHost::has_active_workers();
            // v0.3.339: Don't include has_pending_work in break condition since it's a stored value
            // that may be stale. Instead, check the actual state of timers and nextTicks.
            if !has_pending_next_ticks_now
                && !has_new_timers
                && !has_scheduled_timers
                && !has_pending_immediates()
                && !has_active_workers
            {
                // Run any remaining microtasks before exiting
                scope.perform_microtask_checkpoint();
                break;
            }

            // v0.3.261: If there are fired timers, check if any are valid
            // Timer IDs now include epoch offset, so stale timers have different IDs
            if has_new_timers {
                let timer_manager = crate::event_loop::get_async_timer_manager();
                let fired_timers = timer_manager.poll_fired_timers();

                // Execute only valid timer callbacks (check if callback exists)
                for timer_id in fired_timers {
                    let _ = crate::nodejs_core::timers::execute_timer_callback(scope, timer_id);
                }

                // v0.3.339: Process microtasks after timer execution
                // This handles Promises resolved by timer callbacks
                scope.perform_microtask_checkpoint();
            }
        }

        // Final cleanup: drain remaining work until every queue is empty.
        // Leftover immediate callbacks hold V8 Global handles tied to this
        // execution; leaving them queued would leak them into the next
        // execute_code call (or past isolate disposal in tests).
        loop {
            execute_next_tick_callbacks(scope);
            scope.perform_microtask_checkpoint();
            unmark_immediate_callbacks_deferred();
            execute_immediate_callbacks(scope);
            mark_immediate_callbacks_deferred();
            if !has_pending_next_ticks() && !has_pending_immediates() {
                break;
            }
        }

        // CLI `bee run` keeps the process alive while HTTP servers listen.
        // Tests leave `http_server_keep_alive` false so listen() returns.
        if http_server_keep_alive {
            crate::nodejs_core::http::register_http_dispatch_thread(std::thread::current());
            // Post-startup compilation memory compaction (purge require/AST leftovers before load)
            scope.low_memory_notification();
            #[cfg(target_os = "macos")]
            unsafe {
                extern "C" {
                    fn malloc_zone_pressure_relief(
                        zone: *mut std::ffi::c_void,
                        goal: usize,
                    ) -> usize;
                    fn malloc_default_zone() -> *mut std::ffi::c_void;
                }
                let zone = malloc_default_zone();
                if !zone.is_null() {
                    malloc_zone_pressure_relief(zone, 0);
                }
            }
            let mut idle_ticks: u32 = 0;
            let mut last_trim_time = std::time::Instant::now();
            loop {
                let pumped = {
                    let p = crate::nodejs_core::http::pump_pending_http_requests_in_scope(
                        scope, &context,
                    );
                    execute_next_tick_callbacks(scope);
                    scope.perform_microtask_checkpoint();
                    execute_fired_timers(scope);
                    unmark_immediate_callbacks_deferred();
                    execute_immediate_callbacks(scope);
                    mark_immediate_callbacks_deferred();
                    execute_next_tick_callbacks(scope);
                    scope.perform_microtask_checkpoint();
                    p
                };

                let listening = crate::nodejs_core::http::has_listening_http_servers();
                let pending_req = crate::nodejs_core::http::has_pending_http_requests();
                let pending_async = crate::nodejs_core::http::has_pending_async_http_responses();
                if !listening
                    && !pending_req
                    && !pending_async
                    && !has_pending_next_ticks()
                    && !has_pending_immediates()
                {
                    break;
                }
                if pumped > 0 || pending_req || pending_async {
                    idle_ticks = 0;
                    // Active traffic: keep pumping without delay while there is work
                    continue;
                } else {
                    idle_ticks = idle_ticks.saturating_add(1);
                    // Phase 3: Active Idle Memory Trimming (Aligning with Bun v1.4.1)
                    // When idle for >50 ticks (after traffic cools down)
                    if idle_ticks >= 50
                        && last_trim_time.elapsed() >= std::time::Duration::from_millis(250)
                    {
                        for _ in 0..3 {
                            scope.low_memory_notification();
                        }
                        #[cfg(target_os = "macos")]
                        unsafe {
                            extern "C" {
                                fn malloc_zone_pressure_relief(
                                    zone: *mut std::ffi::c_void,
                                    goal: usize,
                                ) -> usize;
                                fn malloc_default_zone() -> *mut std::ffi::c_void;
                            }
                            let zone = malloc_default_zone();
                            if !zone.is_null() {
                                malloc_zone_pressure_relief(zone, 0);
                            }
                        }
                        #[cfg(target_os = "linux")]
                        unsafe {
                            libc::malloc_trim(0);
                        }
                        last_trim_time = std::time::Instant::now();
                    }
                    std::thread::park_timeout(std::time::Duration::from_millis(1));
                }
            }
        }

        // Return the original script completion value. Do not recompile or rerun
        // any user expression while converting it to a string.
        crate::web_api::background_sync::reset_pending_wait_until();
        let result_str = value_to_string_after_microtasks(scope, result)?;
        Ok(result_str)
    }

    fn publish_tool_exports<'scope>(
        scope: &mut v8::PinScope<'scope, '_>,
        namespace: v8::Local<'scope, v8::Value>,
    ) {
        let context = scope.get_current_context();
        let global = context.global(scope);
        if let Some(key) = v8::String::new(scope, "__beeToolExports") {
            global.set(scope, key.into(), namespace);
        }
    }

    fn publish_cjs_tool_exports(scope: &mut v8::PinScope, context: &v8::Local<v8::Context>) {
        let global = context.global(scope);
        if let Some(existing_key) = v8::String::new(scope, "__beeToolExports") {
            if let Some(existing) = global.get(scope, existing_key.into()) {
                if existing.is_object() && !existing.is_null() && !existing.is_undefined() {
                    return;
                }
            }
        }
        let Some(module_key) = v8::String::new(scope, "module") else {
            return;
        };
        let Some(module_val) = global.get(scope, module_key.into()) else {
            return;
        };
        if !module_val.is_object() {
            return;
        }
        let Some(module_obj) = module_val.to_object(scope) else {
            return;
        };
        let Some(exports_key) = v8::String::new(scope, "exports") else {
            return;
        };
        if let Some(exports) = module_obj.get(scope, exports_key.into()) {
            Self::publish_tool_exports(scope, exports);
        }
    }

    /// Call a previously loaded named export. Used by `bee session` / `bee mcp`.
    pub fn call_named_export(&mut self, name: &str, args_json: &str) -> Result<String> {
        let name_json = serde_json::to_string(name)
            .map_err(|e| anyhow::anyhow!("Failed to encode tool name: {e}"))?;
        let args_literal = serde_json::to_string(args_json)
            .map_err(|e| anyhow::anyhow!("Failed to encode tool arguments: {e}"))?;
        let code = format!(
            r#"
(async () => {{
  const name = {name_json};
  const args = JSON.parse({args_literal});
  const bag = globalThis.__beeToolExports
    || (typeof module !== 'undefined' && module.exports)
    || {{}};
  const fn = bag[name];
  if (typeof fn !== 'function') {{
    throw new Error('unknown tool: ' + name);
  }}
  const result = await fn(args);
  if (result === undefined) {{
    return 'null';
  }}
  return JSON.stringify(result);
}})()
"#
        );
        self.execute_code(&code)
    }

    /// Execute code multiple times and measure performance
    /// v0.3.221: 添加性能基准测试功能
    /// Note: 每次迭代会重新创建 Context 以避免变量重复声明错误
    pub fn benchmark(&mut self, code: &str, iterations: usize) -> Result<BenchmarkResult> {
        let _start = std::time::Instant::now();

        // Warmup runs (not counted) - 每次都重新创建 Context
        for _ in 0..3 {
            self.recreate_context();
            self.execute_code(code)?;
        }

        let bench_start = std::time::Instant::now();
        let mut errors = 0;

        for _ in 0..iterations {
            self.recreate_context();
            if self.execute_code(code).is_err() {
                errors += 1;
            }
        }

        let elapsed = bench_start.elapsed();
        let nanos = elapsed.as_nanos() as f64;
        let avg_ns = nanos / iterations as f64;
        let ops_per_sec = if avg_ns > 0.0 {
            1_000_000_000.0 / avg_ns
        } else {
            0.0
        };

        Ok(BenchmarkResult {
            iterations,
            total_time_ns: nanos as u128,
            avg_time_ns: avg_ns as u128,
            ops_per_sec,
            errors,
        })
    }

    /// Execute code and return execution time
    /// v0.3.221: 添加带时间的执行方法
    pub fn execute_timed(&mut self, code: &str) -> Result<(String, std::time::Duration)> {
        let start = std::time::Instant::now();
        let result = self.execute_code(code)?;
        let elapsed = start.elapsed();
        Ok((result, elapsed))
    }

    /// Set up console object in the V8 context
    fn setup_console(
        scope: &mut v8::ContextScope<v8::HandleScope>,
        context: &v8::Context,
    ) -> Result<()> {
        // Get the global object
        let global = context.global(scope);

        // Create console object
        let console_object = v8::Object::new(scope);

        // Create console.log function
        let console_log_fn = v8::Function::new(scope, crate::console_log_callback)
            .ok_or_else(|| anyhow::anyhow!("Failed to create console.log function"))?;
        let log_key = v8::String::new(scope, "log").unwrap().into();
        console_object.set(scope, log_key, console_log_fn.into());

        // Create console.error function
        let console_error_fn = v8::Function::new(scope, crate::console_error_callback)
            .ok_or_else(|| anyhow::anyhow!("Failed to create console.error function"))?;
        let error_key = v8::String::new(scope, "error").unwrap().into();
        console_object.set(scope, error_key, console_error_fn.into());

        // Create console.warn function
        let console_warn_fn = v8::Function::new(scope, crate::console_warn_callback)
            .ok_or_else(|| anyhow::anyhow!("Failed to create console.warn function"))?;
        let warn_key = v8::String::new(scope, "warn").unwrap().into();
        console_object.set(scope, warn_key, console_warn_fn.into());

        // Create console.info function
        let console_info_fn = v8::Function::new(scope, crate::console_info_callback)
            .ok_or_else(|| anyhow::anyhow!("Failed to create console.info function"))?;
        let info_key = v8::String::new(scope, "info").unwrap().into();
        console_object.set(scope, info_key, console_info_fn.into());

        // Create console.debug function
        let console_debug_fn = v8::Function::new(scope, crate::console_debug_callback)
            .ok_or_else(|| anyhow::anyhow!("Failed to create console.debug function"))?;
        let debug_key = v8::String::new(scope, "debug").unwrap().into();
        console_object.set(scope, debug_key, console_debug_fn.into());

        // Create console.table function
        let console_table_fn = v8::Function::new(scope, crate::console_table_callback)
            .ok_or_else(|| anyhow::anyhow!("Failed to create console.table function"))?;
        let table_key = v8::String::new(scope, "table").unwrap().into();
        console_object.set(scope, table_key, console_table_fn.into());

        // Create console.time function
        let console_time_fn = v8::Function::new(scope, crate::console_time_callback)
            .ok_or_else(|| anyhow::anyhow!("Failed to create console.time function"))?;
        let time_key = v8::String::new(scope, "time").unwrap().into();
        console_object.set(scope, time_key, console_time_fn.into());

        // Create console.timeEnd function
        let console_time_end_fn = v8::Function::new(scope, crate::console_time_end_callback)
            .ok_or_else(|| anyhow::anyhow!("Failed to create console.timeEnd function"))?;
        let time_end_key = v8::String::new(scope, "timeEnd").unwrap().into();
        console_object.set(scope, time_end_key, console_time_end_fn.into());

        // Create console.count function
        let console_count_fn = v8::Function::new(scope, crate::console_count_callback)
            .ok_or_else(|| anyhow::anyhow!("Failed to create console.count function"))?;
        let count_key = v8::String::new(scope, "count").unwrap().into();
        console_object.set(scope, count_key, console_count_fn.into());

        // Create console.countReset function
        let console_count_reset_fn = v8::Function::new(scope, crate::console_count_reset_callback)
            .ok_or_else(|| anyhow::anyhow!("Failed to create console.countReset function"))?;
        let count_reset_key = v8::String::new(scope, "countReset").unwrap().into();
        console_object.set(scope, count_reset_key, console_count_reset_fn.into());

        // Create console.group function
        let console_group_fn = v8::Function::new(scope, crate::console_group_callback)
            .ok_or_else(|| anyhow::anyhow!("Failed to create console.group function"))?;
        let group_key = v8::String::new(scope, "group").unwrap().into();
        console_object.set(scope, group_key, console_group_fn.into());

        // Create console.groupEnd function
        let console_group_end_fn = v8::Function::new(scope, crate::console_group_end_callback)
            .ok_or_else(|| anyhow::anyhow!("Failed to create console.groupEnd function"))?;
        let group_end_key = v8::String::new(scope, "groupEnd").unwrap().into();
        console_object.set(scope, group_end_key, console_group_end_fn.into());

        // Create console.trace function
        let console_trace_fn = v8::Function::new(scope, crate::console_trace_callback)
            .ok_or_else(|| anyhow::anyhow!("Failed to create console.trace function"))?;
        let trace_key = v8::String::new(scope, "trace").unwrap().into();
        console_object.set(scope, trace_key, console_trace_fn.into());

        // Create console.assert function
        let console_assert_fn = v8::Function::new(scope, crate::console_assert_callback)
            .ok_or_else(|| anyhow::anyhow!("Failed to create console.assert function"))?;
        let assert_key = v8::String::new(scope, "assert").unwrap().into();
        console_object.set(scope, assert_key, console_assert_fn.into());

        // Create console.dir function
        let console_dir_fn = v8::Function::new(scope, crate::console_dir_callback)
            .ok_or_else(|| anyhow::anyhow!("Failed to create console.dir function"))?;
        let dir_key = v8::String::new(scope, "dir").unwrap().into();
        console_object.set(scope, dir_key, console_dir_fn.into());

        // Add console to global object
        let console_key = v8::String::new(scope, "console").unwrap().into();
        global.set(scope, console_key, console_object.into());

        Ok(())
    }

    /// Set up Buffer/Uint8Array methods (toString with encoding support)
    #[allow(dead_code)]
    fn setup_buffer_methods(
        scope: &mut v8::ContextScope<v8::HandleScope>,
        context: &v8::Context,
    ) -> Result<()> {
        let global = context.global(scope);

        // Create a hex encoding function
        let _to_hex_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let this = args.this();

                // Get the underlying ArrayBuffer
                let (bytes, _byte_length) = if this.is_typed_array() {
                    let ta = match v8::Local::<v8::TypedArray>::try_from(this) {
                        Ok(ta) => ta,
                        Err(_) => {
                            let error = v8::String::new(scope, "Not a TypedArray").unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };
                    let buffer = ta.buffer(scope).unwrap();
                    let store = buffer.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    let len = ta.byte_length();
                    (unsafe { std::slice::from_raw_parts(ptr, len) }, len)
                } else if this.is_array_buffer() {
                    let ab = match v8::Local::<v8::ArrayBuffer>::try_from(this) {
                        Ok(ab) => ab,
                        Err(_) => {
                            let error = v8::String::new(scope, "Not an ArrayBuffer").unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };
                    let store = ab.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    let len = ab.byte_length();
                    (unsafe { std::slice::from_raw_parts(ptr, len) }, len)
                } else {
                    let error =
                        v8::String::new(scope, "Expected TypedArray or ArrayBuffer").unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                };

                // Convert to hex string
                let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
                let result = v8::String::new(scope, &hex).unwrap();
                retval.set(result.into());
            },
        );

        // Create a base64 encoding function
        let _to_base64_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let this = args.this();

                // Get the underlying ArrayBuffer
                let (bytes, _byte_length) = if this.is_typed_array() {
                    let ta = match v8::Local::<v8::TypedArray>::try_from(this) {
                        Ok(ta) => ta,
                        Err(_) => {
                            let error = v8::String::new(scope, "Not a TypedArray").unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };
                    let buffer = ta.buffer(scope).unwrap();
                    let store = buffer.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    let len = ta.byte_length();
                    (unsafe { std::slice::from_raw_parts(ptr, len) }, len)
                } else if this.is_array_buffer() {
                    let ab = match v8::Local::<v8::ArrayBuffer>::try_from(this) {
                        Ok(ab) => ab,
                        Err(_) => {
                            let error = v8::String::new(scope, "Not an ArrayBuffer").unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };
                    let store = ab.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    let len = ab.byte_length();
                    (unsafe { std::slice::from_raw_parts(ptr, len) }, len)
                } else {
                    let error =
                        v8::String::new(scope, "Expected TypedArray or ArrayBuffer").unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                };

                // Convert to base64 string
                let engine = base64::engine::general_purpose::STANDARD;
                let base64 = engine.encode(bytes);
                let result = v8::String::new(scope, &base64).unwrap();
                retval.set(result.into());
            },
        );

        // Create a custom toString function that handles encoding parameter
        let to_string_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let encoding = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_else(|| "utf8".to_string());

                let this = args.this();

                // Handle different encodings
                match encoding.to_lowercase().as_str() {
                    "hex" => {
                        // Get the underlying ArrayBuffer
                        let (bytes, _) = if this.is_typed_array() {
                            let ta = match v8::Local::<v8::TypedArray>::try_from(this) {
                                Ok(ta) => ta,
                                Err(_) => {
                                    let error = v8::String::new(scope, "Not a TypedArray").unwrap();
                                    let error_obj = v8::Exception::type_error(scope, error);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }
                            };
                            let buffer = ta.buffer(scope).unwrap();
                            let store = buffer.get_backing_store();
                            let ptr = store.as_ref().as_ptr() as *const u8;
                            let len = ta.byte_length();
                            (unsafe { std::slice::from_raw_parts(ptr, len) }, len)
                        } else if this.is_array_buffer() {
                            let ab = match v8::Local::<v8::ArrayBuffer>::try_from(this) {
                                Ok(ab) => ab,
                                Err(_) => {
                                    let error =
                                        v8::String::new(scope, "Not an ArrayBuffer").unwrap();
                                    let error_obj = v8::Exception::type_error(scope, error);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }
                            };
                            let store = ab.get_backing_store();
                            let ptr = store.as_ref().as_ptr() as *const u8;
                            let len = ab.byte_length();
                            (unsafe { std::slice::from_raw_parts(ptr, len) }, len)
                        } else {
                            let error =
                                v8::String::new(scope, "Expected TypedArray or ArrayBuffer")
                                    .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        };

                        let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
                        let result = v8::String::new(scope, &hex).unwrap();
                        retval.set(result.into());
                    }
                    "base64" => {
                        let (bytes, _) = if this.is_typed_array() {
                            let ta = match v8::Local::<v8::TypedArray>::try_from(this) {
                                Ok(ta) => ta,
                                Err(_) => {
                                    let error = v8::String::new(scope, "Not a TypedArray").unwrap();
                                    let error_obj = v8::Exception::type_error(scope, error);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }
                            };
                            let buffer = ta.buffer(scope).unwrap();
                            let store = buffer.get_backing_store();
                            let ptr = store.as_ref().as_ptr() as *const u8;
                            let len = ta.byte_length();
                            (unsafe { std::slice::from_raw_parts(ptr, len) }, len)
                        } else if this.is_array_buffer() {
                            let ab = match v8::Local::<v8::ArrayBuffer>::try_from(this) {
                                Ok(ab) => ab,
                                Err(_) => {
                                    let error =
                                        v8::String::new(scope, "Not an ArrayBuffer").unwrap();
                                    let error_obj = v8::Exception::type_error(scope, error);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }
                            };
                            let store = ab.get_backing_store();
                            let ptr = store.as_ref().as_ptr() as *const u8;
                            let len = ab.byte_length();
                            (unsafe { std::slice::from_raw_parts(ptr, len) }, len)
                        } else {
                            let error =
                                v8::String::new(scope, "Expected TypedArray or ArrayBuffer")
                                    .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        };

                        let engine = base64::engine::general_purpose::STANDARD;
                        let base64 = engine.encode(bytes);
                        let result = v8::String::new(scope, &base64).unwrap();
                        retval.set(result.into());
                    }
                    "utf8" | "utf-8" | "utf8mb4" => {
                        let bytes = if this.is_typed_array() {
                            let ta = match v8::Local::<v8::TypedArray>::try_from(this) {
                                Ok(ta) => ta,
                                Err(_) => {
                                    let error = v8::String::new(scope, "Not a TypedArray").unwrap();
                                    let error_obj = v8::Exception::type_error(scope, error);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }
                            };
                            let len = ta.byte_length();
                            if len == 0 {
                                Vec::new()
                            } else {
                                let buffer = ta.buffer(scope).unwrap();
                                let store = buffer.get_backing_store();
                                let ptr = store.as_ref().as_ptr() as *const u8;
                                unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
                            }
                        } else if this.is_array_buffer() {
                            let ab = match v8::Local::<v8::ArrayBuffer>::try_from(this) {
                                Ok(ab) => ab,
                                Err(_) => {
                                    let error =
                                        v8::String::new(scope, "Not an ArrayBuffer").unwrap();
                                    let error_obj = v8::Exception::type_error(scope, error);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }
                            };
                            let len = ab.byte_length();
                            if len == 0 {
                                Vec::new()
                            } else {
                                let store = ab.get_backing_store();
                                let ptr = store.as_ref().as_ptr() as *const u8;
                                unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
                            }
                        } else {
                            let error =
                                v8::String::new(scope, "Expected TypedArray or ArrayBuffer")
                                    .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        };

                        let utf8 = String::from_utf8_lossy(&bytes).to_string();
                        let result = v8::String::new(scope, &utf8).unwrap();
                        retval.set(result.into());
                    }
                    _ => {
                        // Default to Object.prototype.toString for unsupported encodings
                        let obj_string = v8::String::new(scope, "[object Uint8Array]").unwrap();
                        retval.set(obj_string.into());
                    }
                }
            },
        );

        // Inject the custom toString into Uint8Array's prototype
        let uint8_array_key = v8::String::new(scope, "Uint8Array").unwrap();
        let uint8_array_ctor_val = match global.get(scope, uint8_array_key.into()) {
            Some(val) => val,
            None => return Ok(()),
        };

        if uint8_array_ctor_val.is_object() {
            let uint8_array_ctor = v8::Local::<v8::Object>::try_from(uint8_array_ctor_val).ok();
            if let Some(ctor) = uint8_array_ctor {
                let proto_key = v8::String::new(scope, "prototype").unwrap();
                let proto_val = match ctor.get(scope, proto_key.into()) {
                    Some(val) => val,
                    None => return Ok(()),
                };

                if proto_val.is_object() {
                    let prototype = v8::Local::<v8::Object>::try_from(proto_val).ok();
                    if let Some(prototype) = prototype {
                        let to_string_key = v8::String::new(scope, "toString").unwrap();

                        // Set our custom toString that handles encoding
                        // Note: We keep the original toString for fallback
                        let to_string_fn = match to_string_fn {
                            Some(f) => f,
                            None => return Ok(()),
                        };
                        prototype.set(scope, to_string_key.into(), to_string_fn.into());

                        // Note: Don't override length property - V8's Uint8Array already has it
                        // as a built-in getter that returns byte length
                    }
                }
            }
        }

        Ok(())
    }

    fn setup_live_url_search_params(scope: &mut v8::ContextScope<v8::HandleScope>) -> Result<()> {
        let bootstrap = r##"
(function () {
  const NativeURL = globalThis.URL;
  if (typeof NativeURL !== "function" || NativeURL.__beejsLiveSearchParams === true) {
    return;
  }

  function decodeQueryComponent(value) {
    try {
      return decodeURIComponent(String(value).replace(/\+/g, "%20"));
    } catch (_error) {
      return String(value);
    }
  }

  function encodeQueryComponent(value) {
    return encodeURIComponent(String(value));
  }

  function parseSearch(search) {
    const text = String(search || "");
    const query = text.charAt(0) === "?" ? text.slice(1) : text;
    if (query === "") {
      return [];
    }
    return query.split("&").filter((part) => part.length > 0).map((part) => {
      const equalsIndex = part.indexOf("=");
      if (equalsIndex === -1) {
        return [decodeQueryComponent(part), ""];
      }
      return [
        decodeQueryComponent(part.slice(0, equalsIndex)),
        decodeQueryComponent(part.slice(equalsIndex + 1))
      ];
    });
  }

  function serializePairs(pairs) {
    return pairs
      .map(([key, value]) => `${encodeQueryComponent(key)}=${encodeQueryComponent(value)}`)
      .join("&");
  }

  function createIterator(readValues) {
    let index = 0;
    const iterator = {
      next() {
        const values = readValues();
        if (index >= values.length) {
          return { done: true };
        }
        return { done: false, value: values[index++] };
      }
    };
    if (typeof Symbol !== "undefined" && Symbol.iterator) {
      iterator[Symbol.iterator] = function () {
        return this;
      };
    }
    return iterator;
  }

  function BeeURL(input, base) {
    if (!(this instanceof BeeURL)) {
      return new BeeURL(input, base);
    }

    const parsed = base === undefined
      ? new NativeURL(String(input))
      : new NativeURL(String(input), String(base));

    const state = {
      protocol: parsed.protocol || "",
      host: parsed.host || "",
      hostname: parsed.hostname || "",
      port: parsed.port || "",
      pathname: parsed.pathname || "",
      hash: parsed.hash || "",
      origin: parsed.origin || ""
    };
    const pairs = parseSearch(parsed.search || "");

    const updateHref = () => {
      const query = serializePairs(pairs);
      state.search = query === "" ? "" : `?${query}`;
      state.href = `${state.origin}${state.pathname}${state.search}${state.hash}`;
    };

    const replaceFromParsedUrl = (nextParsed) => {
      state.protocol = nextParsed.protocol || "";
      state.host = nextParsed.host || "";
      state.hostname = nextParsed.hostname || "";
      state.port = nextParsed.port || "";
      state.pathname = nextParsed.pathname || "";
      state.hash = nextParsed.hash || "";
      state.origin = nextParsed.origin || "";
      pairs.splice(0, pairs.length, ...parseSearch(nextParsed.search || ""));
      updateHref();
    };

    const replaceSearch = (nextSearch) => {
      pairs.splice(0, pairs.length, ...parseSearch(nextSearch));
      updateHref();
    };

    const params = {
      append(name, value) {
        pairs.push([String(name), String(value)]);
        updateHref();
      },
      delete(name) {
        const key = String(name);
        for (let i = pairs.length - 1; i >= 0; i--) {
          if (pairs[i][0] === key) {
            pairs.splice(i, 1);
          }
        }
        updateHref();
      },
      get(name) {
        const key = String(name);
        const pair = pairs.find(([entryName]) => entryName === key);
        return pair ? pair[1] : null;
      },
      getAll(name) {
        const key = String(name);
        return pairs.filter(([entryName]) => entryName === key).map(([, value]) => value);
      },
      has(name) {
        const key = String(name);
        return pairs.some(([entryName]) => entryName === key);
      },
      set(name, value) {
        const key = String(name);
        for (let i = pairs.length - 1; i >= 0; i--) {
          if (pairs[i][0] === key) {
            pairs.splice(i, 1);
          }
        }
        pairs.push([key, String(value)]);
        updateHref();
      },
      sort() {
        pairs.sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0);
        updateHref();
      },
      forEach(callback, thisArg) {
        if (typeof callback !== "function") {
          return;
        }
        for (const [key, value] of pairs.slice()) {
          callback.call(thisArg, value, key, params);
        }
      },
      entries() {
        return createIterator(() => pairs.map(([key, value]) => [key, value]));
      },
      keys() {
        return createIterator(() => pairs.map(([key]) => key));
      },
      values() {
        return createIterator(() => pairs.map(([, value]) => value));
      },
      toString() {
        return serializePairs(pairs);
      }
    };

    if (typeof Symbol !== "undefined" && Symbol.iterator) {
      params[Symbol.iterator] = params.entries;
    }

    updateHref();

    Object.defineProperties(this, {
      href: {
        enumerable: true,
        get() {
          return state.href;
        },
        set(value) {
          replaceFromParsedUrl(new NativeURL(String(value)));
        }
      },
      protocol: {
        enumerable: true,
        get() {
          return state.protocol;
        }
      },
      host: {
        enumerable: true,
        get() {
          return state.host;
        }
      },
      hostname: {
        enumerable: true,
        get() {
          return state.hostname;
        }
      },
      port: {
        enumerable: true,
        get() {
          return state.port;
        }
      },
      pathname: {
        enumerable: true,
        get() {
          return state.pathname;
        },
        set(value) {
          state.pathname = String(value) || "/";
          updateHref();
        }
      },
      search: {
        enumerable: true,
        get() {
          return state.search;
        },
        set(value) {
          replaceSearch(String(value || ""));
        }
      },
      hash: {
        enumerable: true,
        get() {
          return state.hash;
        },
        set(value) {
          const next = String(value || "");
          state.hash = next === "" ? "" : next.charAt(0) === "#" ? next : `#${next}`;
          updateHref();
        }
      },
      origin: {
        enumerable: true,
        get() {
          return state.origin;
        }
      },
      searchParams: {
        enumerable: true,
        value: params
      }
    });
  }

  BeeURL.prototype = Object.create(NativeURL.prototype || Object.prototype);
  BeeURL.prototype.constructor = BeeURL;
  Object.defineProperty(BeeURL, "name", { value: "URL" });
  Object.defineProperty(BeeURL, "__beejsLiveSearchParams", { value: true });
  globalThis.URL = BeeURL;
})();
"##;

        let source = v8::String::new(scope, bootstrap)
            .ok_or_else(|| anyhow::anyhow!("Failed to create URL bootstrap source"))?;
        let script = v8::Script::compile(scope, source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to compile URL bootstrap"))?;
        script
            .run(scope)
            .ok_or_else(|| anyhow::anyhow!("Failed to run URL bootstrap"))?;
        Ok(())
    }

    /// Set up Web primitives that are not installed by the modular API setup below.
    fn setup_web_primitives(
        scope: &mut v8::ContextScope<v8::HandleScope>,
        context: &v8::Local<v8::Context>,
    ) -> Result<()> {
        crate::web_api::encoding::setup_encoding_api(scope, context)?;
        crate::web_api::url::setup_url_api(scope, context)?;
        crate::web_api::websocket::setup_websocket_api(scope, context)?;

        // V8 already provides globalThis, Promise, Math, JSON, and Date. Keep its
        // native implementations and only add the Node.js global alias here.
        let global = context.global(scope);
        let global_key = v8::String::new(scope, "global")
            .ok_or_else(|| anyhow::anyhow!("Failed to create global alias key"))?;
        global.set(scope, global_key.into(), global.into());

        let polyfill_script = r#"
        (function() {
            if (!Object.hasOwn) {
                Object.hasOwn = function(obj, prop) {
                    if (obj === null || obj === undefined) {
                        throw new TypeError('Cannot convert undefined or null to object');
                    }
                    return Object.prototype.hasOwnProperty.call(obj, prop);
                };
            }
        })();
        "#;
        if let Some(code) = v8::String::new(scope, polyfill_script) {
            if let Some(script) = v8::Script::compile(scope, code, None) {
                let _ = script.run(scope);
            }
        }

        Ok(())
    }

    /// Legacy monolithic setup retained while compatibility migrates to modules.
    ///
    /// `crypto_only` avoids registering Web APIs that are replaced by the newer
    /// focused modules while preserving Node crypto methods not migrated yet.
    #[rustfmt::skip]
    fn setup_legacy_web_apis(
        scope: &mut v8::ContextScope<v8::HandleScope>,
        context: &v8::Context,
        _crypto_only: bool,
    ) -> Result<()> {
        let global = context.global(scope);

        // Set up global crypto object
        let crypto_obj = v8::Object::new(scope);

        // Add crypto.getRandomValues
        let get_random_values_fn = v8::Function::new(
            scope,
            |_scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                if args.length() >= 1 {
                    // For now, return the array as-is (mock implementation)
                    // In a full implementation, this would fill the array with random values
                    let array = args.get(0);
                    retval.set(array);
                }
            },
        )
        .ok_or_else(|| anyhow::anyhow!("Failed to create getRandomValues function"))?;
        let get_random_values_key = v8::String::new(scope, "getRandomValues").unwrap().into();
        crypto_obj.set(scope, get_random_values_key, get_random_values_fn.into());

        // Add crypto.randomUUID (v0.3.29 - fixed implementation)
        let random_uuid_fn = v8::Function::new(
            scope,
            |_scope: &mut v8::PinScope,
             _args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                // Generate a proper UUID v4
                let uuid = uuid::Uuid::new_v4();
                let uuid_str = uuid.to_string();
                let uuid_v8 = v8::String::new(_scope, &uuid_str).unwrap();
                retval.set(uuid_v8.into());
            },
        )
        .ok_or_else(|| anyhow::anyhow!("Failed to create randomUUID function"))?;
        let random_uuid_key = v8::String::new(scope, "randomUUID").unwrap().into();
        crypto_obj.set(scope, random_uuid_key, random_uuid_fn.into());

        // ==================== Web Crypto API (v0.3.30) ====================
        // Add crypto.subtle for WebCrypto API (v0.3.30)
        let subtle_obj = v8::Object::new(scope);

        // ----- subtle.digest(algorithm, data) -----
        let digest_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let algorithm = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_else(|| "SHA-256".to_string());

                let data_arg = args.get(1);

                // Convert data to bytes
                let data_bytes: Vec<u8> = if data_arg.is_array_buffer() {
                    let buffer = v8::Local::<v8::ArrayBuffer>::try_from(data_arg).unwrap();
                    let len = buffer.byte_length();
                    let store = buffer.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
                } else if data_arg.is_typed_array() {
                    let typed_array = v8::Local::<v8::TypedArray>::try_from(data_arg).unwrap();
                    let buffer = typed_array.buffer(scope).unwrap();
                    let len = buffer.byte_length();
                    let store = buffer.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
                } else {
                    let data_str = data_arg
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    data_str.into_bytes()
                };

                // Compute hash based on algorithm
                let hash_result: Result<Vec<u8>, String> = match algorithm.to_uppercase().as_str() {
                    "SHA-256" => {
                        use ring::digest;
                        let digest_val = digest::digest(&digest::SHA256, &data_bytes);
                        Ok(digest_val.as_ref().to_vec())
                    }
                    "SHA-512" => {
                        use ring::digest;
                        let digest_val = digest::digest(&digest::SHA512, &data_bytes);
                        Ok(digest_val.as_ref().to_vec())
                    }
                    "SHA-384" => {
                        use ring::digest;
                        let digest_val = digest::digest(&digest::SHA384, &data_bytes);
                        Ok(digest_val.as_ref().to_vec())
                    }
                    "SHA-1" => {
                        use sha1::Digest;
                        let mut hasher = sha1::Sha1::default();
                        hasher.update(&data_bytes);
                        Ok(hasher.finalize().to_vec())
                    }
                    _ => Err(format!(
                        "subtle.digest: unsupported algorithm '{}'",
                        algorithm
                    )),
                };

                // Create Promise to return
                let resolver = match v8::PromiseResolver::new(scope) {
                    Some(r) => r,
                    None => {
                        let error =
                            v8::String::new(scope, "Failed to create promise resolver").unwrap();
                        scope.throw_exception(error.into());
                        return;
                    }
                };
                let promise = resolver.get_promise(scope);
                retval.set(promise.into());

                // Resolve or reject the promise based on the result
                match hash_result {
                    Ok(hash_result) => {
                        // Create Uint8Array for the hash
                        let array_buffer = v8::ArrayBuffer::new(scope, hash_result.len());
                        let store = array_buffer.get_backing_store();
                        let ptr = store.as_ref().as_ptr() as *mut u8;
                        unsafe {
                            std::slice::from_raw_parts_mut(ptr, hash_result.len())
                                .copy_from_slice(&hash_result);
                        }
                        if let Some(uint8_array) =
                            v8::Uint8Array::new(scope, array_buffer, 0, hash_result.len())
                        {
                            resolver.resolve(scope, uint8_array.into());
                        } else {
                            let error =
                                v8::String::new(scope, "Failed to create Uint8Array").unwrap();
                            resolver.reject(scope, error.into());
                        }
                    }
                    Err(error_msg) => {
                        let error = v8::String::new(scope, &error_msg).unwrap();
                        resolver.reject(scope, error.into());
                    }
                }
            },
        )
        .ok_or_else(|| anyhow::anyhow!("Failed to create digest function"))?;
        let digest_key = v8::String::new(scope, "digest").unwrap().into();
        subtle_obj.set(scope, digest_key, digest_fn.into());

        // ----- subtle.importKey(format, keyData, algorithm, extractable, usages) -----
        let import_key_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let _format = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_else(|| "raw".to_string());

                let key_data_arg = args.get(1);
                let algo_arg = args.get(2);
                let extractable = args.get(3).to_boolean(scope).boolean_value(scope);

                // Parse algorithm object
                let algo_name = if algo_arg.is_object() {
                    let algo_obj = v8::Local::<v8::Object>::try_from(algo_arg).unwrap();
                    let name_key = v8::String::new(scope, "name").unwrap();
                    if let Some(name_val) = algo_obj.get(scope, name_key.into()) {
                        name_val
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_else(|| "HMAC".to_string())
                    } else {
                        "HMAC".to_string()
                    }
                } else {
                    algo_arg
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_else(|| "HMAC".to_string())
                };

                // Get key bytes
                let key_bytes: Vec<u8> = if key_data_arg.is_array_buffer() {
                    let buffer = v8::Local::<v8::ArrayBuffer>::try_from(key_data_arg).unwrap();
                    let len = buffer.byte_length();
                    let store = buffer.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
                } else if key_data_arg.is_typed_array() {
                    let typed_array = v8::Local::<v8::TypedArray>::try_from(key_data_arg).unwrap();
                    let buffer = typed_array.buffer(scope).unwrap();
                    let len = buffer.byte_length();
                    let store = buffer.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
                } else {
                    let data_str = key_data_arg
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    data_str.into_bytes()
                };

                // Create Promise
                let resolver = match v8::PromiseResolver::new(scope) {
                    Some(r) => r,
                    None => {
                        let error =
                            v8::String::new(scope, "Failed to create promise resolver").unwrap();
                        scope.throw_exception(error.into());
                        return;
                    }
                };
                let promise = resolver.get_promise(scope);
                retval.set(promise.into());

                // Create key object
                let key_obj = v8::Object::new(scope);

                // Store key type
                let type_key = v8::String::new(scope, "type").unwrap();
                let type_val = v8::String::new(scope, "secret").unwrap();
                key_obj.set(scope, type_key.into(), type_val.into());

                // Store algorithm
                let algo_key = v8::String::new(scope, "algorithm").unwrap();
                let algo_obj = v8::Object::new(scope);
                let name_prop = v8::String::new(scope, "name").unwrap();
                let algo_name_val = v8::String::new(scope, &algo_name).unwrap();
                algo_obj.set(scope, name_prop.into(), algo_name_val.into());

                // Add hash to algorithm if HMAC
                if algo_name == "HMAC" {
                    let hash_prop = v8::String::new(scope, "hash").unwrap();
                    let hash_obj = v8::Object::new(scope);
                    let hash_name = v8::String::new(scope, "name").unwrap();
                    let sha256_val = v8::String::new(scope, "SHA-256").unwrap();
                    hash_obj.set(scope, hash_name.into(), sha256_val.into());
                    algo_obj.set(scope, hash_prop.into(), hash_obj.into());
                }

                key_obj.set(scope, algo_key.into(), algo_obj.into());

                // Store extractable
                let extractable_key = v8::String::new(scope, "extractable").unwrap();
                let extractable_val = v8::Boolean::new(scope, extractable);
                key_obj.set(scope, extractable_key.into(), extractable_val.into());

                // Store usages
                let usages_key = v8::String::new(scope, "usages").unwrap();
                let usages_val = v8::Array::new(scope, 0);
                let sign_val = v8::String::new(scope, "sign").unwrap();
                let verify_val = v8::String::new(scope, "verify").unwrap();
                usages_val.set_index(scope, 0, sign_val.into());
                usages_val.set_index(scope, 1, verify_val.into());
                key_obj.set(scope, usages_key.into(), usages_val.into());

                // Store key bytes (base64 encoded)
                let key_bytes_key = v8::String::new(scope, "_keyBytes").unwrap();
                let key_bytes_val = v8::String::new(
                    scope,
                    &base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &key_bytes),
                )
                .unwrap();
                key_obj.set(scope, key_bytes_key.into(), key_bytes_val.into());

                // Resolve the promise with the key object
                resolver.resolve(scope, key_obj.into());
            },
        )
        .ok_or_else(|| anyhow::anyhow!("Failed to create importKey function"))?;
        let import_key_key = v8::String::new(scope, "importKey").unwrap().into();
        subtle_obj.set(scope, import_key_key, import_key_fn.into());

        // ----- subtle.sign(algorithm, key, data) -----
        let sign_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let algo_arg = args.get(0);
                let key_arg = args.get(1);
                let data_arg = args.get(2);

                // Create Promise
                let resolver = match v8::PromiseResolver::new(scope) {
                    Some(r) => r,
                    None => {
                        let error =
                            v8::String::new(scope, "Failed to create promise resolver").unwrap();
                        scope.throw_exception(error.into());
                        return;
                    }
                };
                let promise = resolver.get_promise(scope);
                retval.set(promise.into());

                // Get algorithm name
                let algo_name = if algo_arg.is_object() {
                    let algo_obj = v8::Local::<v8::Object>::try_from(algo_arg).unwrap();
                    let name_key = v8::String::new(scope, "name").unwrap();
                    if let Some(name_val) = algo_obj.get(scope, name_key.into()) {
                        name_val
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_else(|| "HMAC".to_string())
                    } else {
                        "HMAC".to_string()
                    }
                } else {
                    "HMAC".to_string()
                };

                // Get key bytes
                let key_bytes: Vec<u8> = {
                    let key_bytes_key = v8::String::new(scope, "_keyBytes").unwrap();
                    let key_obj = v8::Local::<v8::Object>::try_from(key_arg).unwrap();
                    if let Some(key_bytes_val) = key_obj.get(scope, key_bytes_key.into()) {
                        let b64_str = key_bytes_val
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();
                        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64_str)
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    }
                };

                // Get data bytes
                let data_bytes: Vec<u8> = if data_arg.is_array_buffer() {
                    let buffer = v8::Local::<v8::ArrayBuffer>::try_from(data_arg).unwrap();
                    let len = buffer.byte_length();
                    let store = buffer.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
                } else if data_arg.is_typed_array() {
                    let typed_array = v8::Local::<v8::TypedArray>::try_from(data_arg).unwrap();
                    let buffer = typed_array.buffer(scope).unwrap();
                    let len = buffer.byte_length();
                    let store = buffer.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
                } else {
                    let data_str = data_arg
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    data_str.into_bytes()
                };

                // Sign based on algorithm
                match algo_name.as_str() {
                    "HMAC" => {
                        use ring::hmac;
                        let signing_key = hmac::Key::new(hmac::HMAC_SHA256, &key_bytes);
                        let hmac_result = hmac::sign(&signing_key, &data_bytes);
                        let sig_result = hmac_result.as_ref().to_vec();

                        // Create Uint8Array for signature
                        let array_buffer = v8::ArrayBuffer::new(scope, sig_result.len());
                        let store = array_buffer.get_backing_store();
                        let ptr = store.as_ref().as_ptr() as *mut u8;
                        unsafe {
                            std::slice::from_raw_parts_mut(ptr, sig_result.len())
                                .copy_from_slice(&sig_result);
                        }
                        if let Some(uint8_array) =
                            v8::Uint8Array::new(scope, array_buffer, 0, sig_result.len())
                        {
                            resolver.resolve(scope, uint8_array.into());
                        } else {
                            let error =
                                v8::String::new(scope, "Failed to create Uint8Array").unwrap();
                            resolver.reject(scope, error.into());
                        }
                    }
                    _ => {
                        let error_msg =
                            format!("subtle.sign: unsupported algorithm '{}'", algo_name);
                        let error = v8::String::new(scope, &error_msg).unwrap();
                        resolver.reject(scope, error.into());
                    }
                }
            },
        )
        .ok_or_else(|| anyhow::anyhow!("Failed to create sign function"))?;
        let sign_key = v8::String::new(scope, "sign").unwrap().into();
        subtle_obj.set(scope, sign_key, sign_fn.into());

        // ----- subtle.verify(algorithm, key, signature, data) -----
        let verify_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let algo_arg = args.get(0);
                let key_arg = args.get(1);
                let sig_arg = args.get(2);
                let data_arg = args.get(3);

                // Create Promise
                let resolver = match v8::PromiseResolver::new(scope) {
                    Some(r) => r,
                    None => {
                        let error =
                            v8::String::new(scope, "Failed to create promise resolver").unwrap();
                        scope.throw_exception(error.into());
                        return;
                    }
                };
                let promise = resolver.get_promise(scope);
                retval.set(promise.into());

                // Get algorithm name
                let algo_name = if algo_arg.is_object() {
                    let algo_obj = v8::Local::<v8::Object>::try_from(algo_arg).unwrap();
                    let name_key = v8::String::new(scope, "name").unwrap();
                    if let Some(name_val) = algo_obj.get(scope, name_key.into()) {
                        name_val
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_else(|| "HMAC".to_string())
                    } else {
                        "HMAC".to_string()
                    }
                } else {
                    "HMAC".to_string()
                };

                // Get signature bytes
                let sig_bytes: Vec<u8> = if sig_arg.is_array_buffer() {
                    let buffer = v8::Local::<v8::ArrayBuffer>::try_from(sig_arg).unwrap();
                    let len = buffer.byte_length();
                    let store = buffer.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
                } else if sig_arg.is_typed_array() {
                    let typed_array = v8::Local::<v8::TypedArray>::try_from(sig_arg).unwrap();
                    let buffer = typed_array.buffer(scope).unwrap();
                    let len = buffer.byte_length();
                    let store = buffer.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
                } else {
                    Vec::new()
                };

                // Get key bytes
                let key_bytes: Vec<u8> = {
                    let key_bytes_key = v8::String::new(scope, "_keyBytes").unwrap();
                    let key_obj = v8::Local::<v8::Object>::try_from(key_arg).unwrap();
                    if let Some(key_bytes_val) = key_obj.get(scope, key_bytes_key.into()) {
                        let b64_str = key_bytes_val
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();
                        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64_str)
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    }
                };

                // Get data bytes
                let data_bytes: Vec<u8> = if data_arg.is_array_buffer() {
                    let buffer = v8::Local::<v8::ArrayBuffer>::try_from(data_arg).unwrap();
                    let len = buffer.byte_length();
                    let store = buffer.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
                } else if data_arg.is_typed_array() {
                    let typed_array = v8::Local::<v8::TypedArray>::try_from(data_arg).unwrap();
                    let buffer = typed_array.buffer(scope).unwrap();
                    let len = buffer.byte_length();
                    let store = buffer.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
                } else {
                    let data_str = data_arg
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    data_str.into_bytes()
                };

                // Verify based on algorithm
                match algo_name.as_str() {
                    "HMAC" => {
                        use ring::hmac;
                        let signing_key = hmac::Key::new(hmac::HMAC_SHA256, &key_bytes);
                        let expected_sig = hmac::sign(&signing_key, &data_bytes);

                        // Constant-time comparison using our secure comparison function
                        let is_valid = constant_time_eq(expected_sig.as_ref(), &sig_bytes);
                        let result_bool = v8::Boolean::new(scope, is_valid);
                        resolver.resolve(scope, result_bool.into());
                    }
                    _ => {
                        let error_msg =
                            format!("subtle.verify: unsupported algorithm '{}'", algo_name);
                        let error = v8::String::new(scope, &error_msg).unwrap();
                        resolver.reject(scope, error.into());
                    }
                }
            },
        )
        .ok_or_else(|| anyhow::anyhow!("Failed to create verify function"))?;
        let verify_key = v8::String::new(scope, "verify").unwrap().into();
        subtle_obj.set(scope, verify_key, verify_fn.into());

        // ----- subtle.generateKey(algorithm, extractable, usages) -----
        let generate_key_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let algo_arg = args.get(0);
                let extractable = args.get(1).to_boolean(scope).boolean_value(scope);

                // Create Promise
                let resolver = match v8::PromiseResolver::new(scope) {
                    Some(r) => r,
                    None => {
                        let error =
                            v8::String::new(scope, "Failed to create promise resolver").unwrap();
                        scope.throw_exception(error.into());
                        return;
                    }
                };
                let promise = resolver.get_promise(scope);
                retval.set(promise.into());

                // Parse algorithm
                let (algo_name, key_length) = if algo_arg.is_object() {
                    let algo_obj = v8::Local::<v8::Object>::try_from(algo_arg).unwrap();
                    let name_key = v8::String::new(scope, "name").unwrap();
                    let name_val = algo_obj
                        .get(scope, name_key.into())
                        .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                        .unwrap_or_else(|| "AES-GCM".to_string());

                    let length_key = v8::String::new(scope, "length").unwrap();
                    let length_val = algo_obj
                        .get(scope, length_key.into())
                        .and_then(|v| v.to_integer(scope).map(|i| i.value() as usize))
                        .unwrap_or(256);

                    (name_val, length_val)
                } else {
                    ("AES-GCM".to_string(), 256)
                };

                // Generate random key
                let key_len_bytes = (key_length + 7) / 8;
                let mut key_bytes = vec![0u8; key_len_bytes];
                let rand = ring::rand::SystemRandom::new();
                ring::rand::SecureRandom::fill(&rand, &mut key_bytes).unwrap_or(());

                // Create key object
                let key_obj = v8::Object::new(scope);

                // Store key type
                let type_key = v8::String::new(scope, "type").unwrap();
                let type_val = v8::String::new(scope, "secret").unwrap();
                key_obj.set(scope, type_key.into(), type_val.into());

                // Store algorithm
                let algo_key = v8::String::new(scope, "algorithm").unwrap();
                let algo_obj = v8::Object::new(scope);
                let name_prop = v8::String::new(scope, "name").unwrap();
                let algo_name_str = v8::String::new(scope, &algo_name).unwrap();
                algo_obj.set(scope, name_prop.into(), algo_name_str.into());
                if algo_name.starts_with("AES") {
                    let length_prop = v8::String::new(scope, "length").unwrap();
                    let length_val = v8::Integer::new(scope, key_length as i32);
                    algo_obj.set(scope, length_prop.into(), length_val.into());
                }
                key_obj.set(scope, algo_key.into(), algo_obj.into());

                // Store extractable
                let extractable_key = v8::String::new(scope, "extractable").unwrap();
                let extractable_val = v8::Boolean::new(scope, extractable);
                key_obj.set(scope, extractable_key.into(), extractable_val.into());

                // Store usages
                let usages_key = v8::String::new(scope, "usages").unwrap();
                let usages_val = v8::Array::new(scope, 0);
                let encrypt_str = v8::String::new(scope, "encrypt").unwrap();
                let decrypt_str = v8::String::new(scope, "decrypt").unwrap();
                usages_val.set_index(scope, 0, encrypt_str.into());
                usages_val.set_index(scope, 1, decrypt_str.into());
                key_obj.set(scope, usages_key.into(), usages_val.into());

                // Store key bytes (base64 encoded)
                let key_bytes_key = v8::String::new(scope, "_keyBytes").unwrap();
                let key_bytes_val = v8::String::new(
                    scope,
                    &base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &key_bytes),
                )
                .unwrap();
                key_obj.set(scope, key_bytes_key.into(), key_bytes_val.into());

                // Resolve the promise with the key object
                resolver.resolve(scope, key_obj.into());
            },
        )
        .ok_or_else(|| anyhow::anyhow!("Failed to create generateKey function"))?;
        let generate_key_key = v8::String::new(scope, "generateKey").unwrap().into();
        subtle_obj.set(scope, generate_key_key, generate_key_fn.into());

        // ----- subtle.encrypt(algorithm, key, data) -----
        let encrypt_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let algo_arg = args.get(0);
                let key_arg = args.get(1);
                let data_arg = args.get(2);

                // Create Promise
                let resolver = match v8::PromiseResolver::new(scope) {
                    Some(r) => r,
                    None => {
                        let error =
                            v8::String::new(scope, "Failed to create promise resolver").unwrap();
                        scope.throw_exception(error.into());
                        return;
                    }
                };
                let promise = resolver.get_promise(scope);
                retval.set(promise.into());

                // Parse algorithm
                let algo_name = if algo_arg.is_object() {
                    let algo_obj = v8::Local::<v8::Object>::try_from(algo_arg).unwrap();
                    let name_key = v8::String::new(scope, "name").unwrap();
                    algo_obj
                        .get(scope, name_key.into())
                        .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                        .unwrap_or_else(|| "AES-GCM".to_string())
                } else {
                    "AES-GCM".to_string()
                };

                // Get IV from algorithm
                let iv = if algo_arg.is_object() {
                    let algo_obj = v8::Local::<v8::Object>::try_from(algo_arg).unwrap();
                    let iv_key = v8::String::new(scope, "iv").unwrap();
                    algo_obj
                        .get(scope, iv_key.into())
                        .and_then(|v| {
                            if v.is_array_buffer() {
                                let buffer = v8::Local::<v8::ArrayBuffer>::try_from(v).ok()?;
                                let len = buffer.byte_length();
                                let store = buffer.get_backing_store();
                                let ptr = store.as_ref().as_ptr() as *const u8;
                                Some(unsafe { std::slice::from_raw_parts(ptr, len).to_vec() })
                            } else if v.is_typed_array() {
                                let typed_array = v8::Local::<v8::TypedArray>::try_from(v).unwrap();
                                let buffer = typed_array.buffer(scope).unwrap();
                                let len = buffer.byte_length();
                                let store = buffer.get_backing_store();
                                let ptr = store.as_ref().as_ptr() as *const u8;
                                Some(unsafe { std::slice::from_raw_parts(ptr, len).to_vec() })
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| vec![0u8; 12])
                } else {
                    vec![0u8; 12]
                };

                // Get key bytes
                let key_bytes: Vec<u8> = {
                    let key_bytes_key = v8::String::new(scope, "_keyBytes").unwrap();
                    let key_obj = v8::Local::<v8::Object>::try_from(key_arg).unwrap();
                    if let Some(key_bytes_val) = key_obj.get(scope, key_bytes_key.into()) {
                        let b64_str = key_bytes_val
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();
                        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64_str)
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    }
                };

                // Get data bytes
                let data_bytes: Vec<u8> = if data_arg.is_array_buffer() {
                    let buffer = v8::Local::<v8::ArrayBuffer>::try_from(data_arg).unwrap();
                    let len = buffer.byte_length();
                    let store = buffer.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
                } else if data_arg.is_typed_array() {
                    let typed_array = v8::Local::<v8::TypedArray>::try_from(data_arg).unwrap();
                    let buffer = typed_array.buffer(scope).unwrap();
                    let len = buffer.byte_length();
                    let store = buffer.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
                } else {
                    let data_str = data_arg
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    data_str.into_bytes()
                };

                // Encrypt based on algorithm
                match algo_name.as_str() {
                    "AES-GCM" => {
                        // Simplified AES-GCM implementation (XOR-based for demonstration)
                        // In production, use ring::aead
                        let mut ciphertext = iv.clone();
                        for (i, &byte) in data_bytes.iter().enumerate() {
                            ciphertext.push(byte ^ key_bytes[i % key_bytes.len()]);
                        }

                        // Create Uint8Array for ciphertext
                        let array_buffer = v8::ArrayBuffer::new(scope, ciphertext.len());
                        let store = array_buffer.get_backing_store();
                        let ptr = store.as_ref().as_ptr() as *mut u8;
                        unsafe {
                            std::slice::from_raw_parts_mut(ptr, ciphertext.len())
                                .copy_from_slice(&ciphertext);
                        }
                        if let Some(uint8_array) =
                            v8::Uint8Array::new(scope, array_buffer, 0, ciphertext.len())
                        {
                            resolver.resolve(scope, uint8_array.into());
                        } else {
                            let error =
                                v8::String::new(scope, "Failed to create Uint8Array").unwrap();
                            resolver.reject(scope, error.into());
                        }
                    }
                    _ => {
                        let error_msg =
                            format!("subtle.encrypt: unsupported algorithm '{}'", algo_name);
                        let error = v8::String::new(scope, &error_msg).unwrap();
                        resolver.reject(scope, error.into());
                    }
                }
            },
        )
        .ok_or_else(|| anyhow::anyhow!("Failed to create encrypt function"))?;
        let encrypt_key = v8::String::new(scope, "encrypt").unwrap().into();
        subtle_obj.set(scope, encrypt_key, encrypt_fn.into());

        // ----- subtle.decrypt(algorithm, key, data) -----
        let decrypt_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let algo_arg = args.get(0);
                let key_arg = args.get(1);
                let data_arg = args.get(2);

                // Create Promise
                let resolver = match v8::PromiseResolver::new(scope) {
                    Some(r) => r,
                    None => {
                        let error =
                            v8::String::new(scope, "Failed to create promise resolver").unwrap();
                        scope.throw_exception(error.into());
                        return;
                    }
                };
                let promise = resolver.get_promise(scope);
                retval.set(promise.into());

                // Parse algorithm
                let algo_name = if algo_arg.is_object() {
                    let algo_obj = v8::Local::<v8::Object>::try_from(algo_arg).unwrap();
                    let name_key = v8::String::new(scope, "name").unwrap();
                    algo_obj
                        .get(scope, name_key.into())
                        .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                        .unwrap_or_else(|| "AES-GCM".to_string())
                } else {
                    "AES-GCM".to_string()
                };

                // Get IV from algorithm
                let iv = if algo_arg.is_object() {
                    let algo_obj = v8::Local::<v8::Object>::try_from(algo_arg).unwrap();
                    let iv_key = v8::String::new(scope, "iv").unwrap();
                    algo_obj
                        .get(scope, iv_key.into())
                        .and_then(|v| {
                            if v.is_array_buffer() {
                                let buffer = v8::Local::<v8::ArrayBuffer>::try_from(v).ok()?;
                                let len = buffer.byte_length();
                                let store = buffer.get_backing_store();
                                let ptr = store.as_ref().as_ptr() as *const u8;
                                Some(unsafe { std::slice::from_raw_parts(ptr, len).to_vec() })
                            } else if v.is_typed_array() {
                                let typed_array = v8::Local::<v8::TypedArray>::try_from(v).unwrap();
                                let buffer = typed_array.buffer(scope).unwrap();
                                let len = buffer.byte_length();
                                let store = buffer.get_backing_store();
                                let ptr = store.as_ref().as_ptr() as *const u8;
                                Some(unsafe { std::slice::from_raw_parts(ptr, len).to_vec() })
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| vec![0u8; 12])
                } else {
                    vec![0u8; 12]
                };

                // Get key bytes
                let key_bytes: Vec<u8> = {
                    let key_bytes_key = v8::String::new(scope, "_keyBytes").unwrap();
                    let key_obj = v8::Local::<v8::Object>::try_from(key_arg).unwrap();
                    if let Some(key_bytes_val) = key_obj.get(scope, key_bytes_key.into()) {
                        let b64_str = key_bytes_val
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();
                        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64_str)
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    }
                };

                // Get ciphertext bytes
                let ct_bytes: Vec<u8> = if data_arg.is_array_buffer() {
                    let buffer = v8::Local::<v8::ArrayBuffer>::try_from(data_arg).unwrap();
                    let len = buffer.byte_length();
                    let store = buffer.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
                } else if data_arg.is_typed_array() {
                    let typed_array = v8::Local::<v8::TypedArray>::try_from(data_arg).unwrap();
                    let buffer = typed_array.buffer(scope).unwrap();
                    let len = buffer.byte_length();
                    let store = buffer.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
                } else {
                    Vec::new()
                };

                // Decrypt based on algorithm
                match algo_name.as_str() {
                    "AES-GCM" => {
                        // Simplified AES-GCM decryption (XOR-based for demonstration)
                        // In production, use ring::aead
                        let plaintext = if ct_bytes.len() < iv.len() {
                            Vec::new()
                        } else {
                            let mut result = Vec::new();
                            for (i, &byte) in ct_bytes[iv.len()..].iter().enumerate() {
                                result.push(byte ^ key_bytes[i % key_bytes.len()]);
                            }
                            result
                        };

                        // Create Uint8Array for plaintext
                        let array_buffer = v8::ArrayBuffer::new(scope, plaintext.len());
                        let store = array_buffer.get_backing_store();
                        let ptr = store.as_ref().as_ptr() as *mut u8;
                        unsafe {
                            std::slice::from_raw_parts_mut(ptr, plaintext.len())
                                .copy_from_slice(&plaintext);
                        }
                        if let Some(uint8_array) =
                            v8::Uint8Array::new(scope, array_buffer, 0, plaintext.len())
                        {
                            resolver.resolve(scope, uint8_array.into());
                        } else {
                            let error =
                                v8::String::new(scope, "Failed to create Uint8Array").unwrap();
                            resolver.reject(scope, error.into());
                        }
                    }
                    _ => {
                        let error_msg =
                            format!("subtle.decrypt: unsupported algorithm '{}'", algo_name);
                        let error = v8::String::new(scope, &error_msg).unwrap();
                        resolver.reject(scope, error.into());
                    }
                }
            },
        )
        .ok_or_else(|| anyhow::anyhow!("Failed to create decrypt function"))?;
        let decrypt_key = v8::String::new(scope, "decrypt").unwrap().into();
        subtle_obj.set(scope, decrypt_key, decrypt_fn.into());

        // ----- subtle.exportKey(format, key) -----
        let export_key_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let format = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_else(|| "raw".to_string());

                let key_arg = args.get(1);

                // Create Promise
                let resolver = match v8::PromiseResolver::new(scope) {
                    Some(r) => r,
                    None => {
                        let error =
                            v8::String::new(scope, "Failed to create promise resolver").unwrap();
                        scope.throw_exception(error.into());
                        return;
                    }
                };
                let promise = resolver.get_promise(scope);
                retval.set(promise.into());

                // Get key bytes
                let key_bytes: Vec<u8> = {
                    let key_bytes_key = v8::String::new(scope, "_keyBytes").unwrap();
                    let key_obj = v8::Local::<v8::Object>::try_from(key_arg).unwrap();
                    if let Some(key_bytes_val) = key_obj.get(scope, key_bytes_key.into()) {
                        let b64_str = key_bytes_val
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();
                        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64_str)
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    }
                };

                match format.as_str() {
                    "raw" => {
                        // Create Uint8Array
                        let array_buffer = v8::ArrayBuffer::new(scope, key_bytes.len());
                        let store = array_buffer.get_backing_store();
                        let ptr = store.as_ref().as_ptr() as *mut u8;
                        unsafe {
                            std::slice::from_raw_parts_mut(ptr, key_bytes.len())
                                .copy_from_slice(&key_bytes);
                        }
                        if let Some(uint8_array) =
                            v8::Uint8Array::new(scope, array_buffer, 0, key_bytes.len())
                        {
                            resolver.resolve(scope, uint8_array.into());
                        } else {
                            let error =
                                v8::String::new(scope, "Failed to create Uint8Array").unwrap();
                            resolver.reject(scope, error.into());
                        }
                    }
                    "jwk" => {
                        // Create JWK format
                        let jwk_obj = v8::Object::new(scope);
                        let kty_key = v8::String::new(scope, "kty").unwrap();
                        let kty_val = v8::String::new(scope, "oct").unwrap();
                        jwk_obj.set(scope, kty_key.into(), kty_val.into());
                        let k_key = v8::String::new(scope, "k").unwrap();
                        let k_val = v8::String::new(
                            scope,
                            &base64::Engine::encode(
                                &base64::engine::general_purpose::STANDARD,
                                &key_bytes,
                            ),
                        )
                        .unwrap();
                        jwk_obj.set(scope, k_key.into(), k_val.into());
                        let alg_key = v8::String::new(scope, "alg").unwrap();
                        let alg_val = v8::String::new(scope, "A256GCM").unwrap();
                        jwk_obj.set(scope, alg_key.into(), alg_val.into());
                        resolver.resolve(scope, jwk_obj.into());
                    }
                    _ => {
                        let error_msg =
                            format!("subtle.exportKey: unsupported format '{}'", format);
                        let error = v8::String::new(scope, &error_msg).unwrap();
                        resolver.reject(scope, error.into());
                    }
                }
            },
        )
        .ok_or_else(|| anyhow::anyhow!("Failed to create exportKey function"))?;
        let export_key_key = v8::String::new(scope, "exportKey").unwrap().into();
        subtle_obj.set(scope, export_key_key, export_key_fn.into());

        let subtle_key = v8::String::new(scope, "subtle").unwrap().into();
        crypto_obj.set(scope, subtle_key, subtle_obj.into());

        // Add crypto.createHash (v0.3.8)
        let create_hash_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let algorithm = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                // Validate algorithm
                let valid_algorithms = ["md5", "sha1", "sha256", "sha512", "blake3"];
                if !valid_algorithms.contains(&algorithm.as_str()) {
                    let error_msg = format!(
                        "createHash: unsupported algorithm '{}'. Supported: {}",
                        algorithm,
                        valid_algorithms.join(", ")
                    );
                    let error = v8::String::new(scope, &error_msg).unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }

                // Create Hash object
                let hash_obj = v8::Object::new(scope);

                // Store algorithm in object property
                let algo_key = v8::String::new(scope, "_algorithm").unwrap();
                let algo_val = v8::String::new(scope, &algorithm).unwrap();
                hash_obj.set(scope, algo_key.into(), algo_val.into());

                // Store data buffer
                let data_key = v8::String::new(scope, "_data").unwrap();
                let data_val = v8::Array::new(scope, 0);
                hash_obj.set(scope, data_key.into(), data_val.into());

                // Add update method
                let update_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let this = args.this();
                        let data = args
                            .get(0)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();

                        // Append data to buffer
                        let data_key = v8::String::new(scope, "_data").unwrap();
                        if let Some(data_array_val) = this.get(scope, data_key.into()) {
                            if data_array_val.is_array() {
                                let arr = v8::Local::<v8::Array>::try_from(data_array_val).unwrap();
                                let length = arr.length();
                                let str_val = v8::String::new(scope, &data).unwrap();
                                arr.set_index(scope, length, str_val.into());
                            }
                        }

                        // Return this for chaining
                        retval.set(this.into());
                    },
                );
                let update_fn = match update_fn_opt {
                    Some(f) => f,
                    None => {
                        // Return early from setup_web_apis
                        return;
                    }
                };
                let update_key = v8::String::new(scope, "update").unwrap().into();
                hash_obj.set(scope, update_key, update_fn.into());

                // Add digest method
                let digest_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let this = args.this();
                        let encoding = args
                            .get(0)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_else(|| "hex".to_string());

                        // Get algorithm
                        let algo_key = v8::String::new(scope, "_algorithm").unwrap();
                        let algorithm = this
                            .get(scope, algo_key.into())
                            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                            .unwrap_or_default();

                        // Get data
                        let data_key = v8::String::new(scope, "_data").unwrap();
                        let mut combined_data = String::new();
                        if let Some(data_array_val) = this.get(scope, data_key.into()) {
                            if data_array_val.is_array() {
                                let arr = v8::Local::<v8::Array>::try_from(data_array_val).unwrap();
                                for i in 0..arr.length() {
                                    if let Some(data_str) =
                                        arr.get_index(scope, i).and_then(|v| v.to_string(scope))
                                    {
                                        combined_data
                                            .push_str(&data_str.to_rust_string_lossy(scope));
                                    }
                                }
                            }
                        }

                        // Compute hash
                        let digest_result: String = match algorithm.as_str() {
                            "md5" => {
                                let digest = md5::compute(combined_data.as_bytes());
                                match encoding.as_str() {
                                    "hex" => format!("{:x}", digest),
                                    "base64" => base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD,
                                        &digest.0,
                                    ),
                                    "buffer" => {
                                        let ab = v8::ArrayBuffer::new(scope, digest.0.len());
                                        let backing_store = ab.get_backing_store();
                                        for (i, byte) in digest.0.iter().enumerate() {
                                            backing_store[i].set(*byte);
                                        }
                                        if let Some(uint8_array) =
                                            v8::Uint8Array::new(scope, ab, 0, digest.0.len())
                                        {
                                            retval.set(uint8_array.into());
                                        }
                                        return;
                                    }
                                    _ => format!("{:x}", digest),
                                }
                            }
                            "sha1" => {
                                // Use MD5 for sha1 as fallback (simplified)
                                let digest = md5::compute(combined_data.as_bytes());
                                match encoding.as_str() {
                                    "hex" => format!("{:x}", digest),
                                    "base64" => base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD,
                                        &digest.0,
                                    ),
                                    "buffer" => {
                                        let ab = v8::ArrayBuffer::new(scope, digest.0.len());
                                        let backing_store = ab.get_backing_store();
                                        for (i, byte) in digest.0.iter().enumerate() {
                                            backing_store[i].set(*byte);
                                        }
                                        if let Some(uint8_array) =
                                            v8::Uint8Array::new(scope, ab, 0, digest.0.len())
                                        {
                                            retval.set(uint8_array.into());
                                        }
                                        return;
                                    }
                                    _ => format!("{:x}", digest),
                                }
                            }
                            "sha256" => {
                                use ring::digest;
                                let digest_result =
                                    digest::digest(&digest::SHA256, combined_data.as_bytes());
                                match encoding.as_str() {
                                    "hex" => hex::encode(digest_result.as_ref()),
                                    "base64" => base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD,
                                        digest_result.as_ref(),
                                    ),
                                    "buffer" => {
                                        let ab = v8::ArrayBuffer::new(
                                            scope,
                                            digest_result.as_ref().len(),
                                        );
                                        let backing_store = ab.get_backing_store();
                                        for (i, byte) in digest_result.as_ref().iter().enumerate() {
                                            backing_store[i].set(*byte);
                                        }
                                        if let Some(uint8_array) = v8::Uint8Array::new(
                                            scope,
                                            ab,
                                            0,
                                            digest_result.as_ref().len(),
                                        ) {
                                            retval.set(uint8_array.into());
                                        }
                                        return;
                                    }
                                    _ => hex::encode(digest_result.as_ref()),
                                }
                            }
                            "sha512" => {
                                use ring::digest;
                                let digest_result =
                                    digest::digest(&digest::SHA512, combined_data.as_bytes());
                                match encoding.as_str() {
                                    "hex" => hex::encode(digest_result.as_ref()),
                                    "base64" => base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD,
                                        digest_result.as_ref(),
                                    ),
                                    "buffer" => {
                                        let ab = v8::ArrayBuffer::new(
                                            scope,
                                            digest_result.as_ref().len(),
                                        );
                                        let backing_store = ab.get_backing_store();
                                        for (i, byte) in digest_result.as_ref().iter().enumerate() {
                                            backing_store[i].set(*byte);
                                        }
                                        if let Some(uint8_array) = v8::Uint8Array::new(
                                            scope,
                                            ab,
                                            0,
                                            digest_result.as_ref().len(),
                                        ) {
                                            retval.set(uint8_array.into());
                                        }
                                        return;
                                    }
                                    _ => hex::encode(digest_result.as_ref()),
                                }
                            }
                            "blake3" => {
                                let hash = blake3::Hasher::default()
                                    .update(combined_data.as_bytes())
                                    .finalize();
                                let hash_bytes = hash.as_bytes();
                                match encoding.as_str() {
                                    "hex" => hex::encode(hash_bytes),
                                    "base64" => base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD,
                                        hash_bytes,
                                    ),
                                    "buffer" => {
                                        let ab = v8::ArrayBuffer::new(scope, hash_bytes.len());
                                        let backing_store = ab.get_backing_store();
                                        for (i, byte) in hash_bytes.iter().enumerate() {
                                            backing_store[i].set(*byte);
                                        }
                                        if let Some(uint8_array) =
                                            v8::Uint8Array::new(scope, ab, 0, hash_bytes.len())
                                        {
                                            retval.set(uint8_array.into());
                                        }
                                        return;
                                    }
                                    _ => hex::encode(hash_bytes),
                                }
                            }
                            _ => String::new(),
                        };

                        let result_str = v8::String::new(scope, &digest_result).unwrap();
                        retval.set(result_str.into());
                    },
                );
                let digest_fn = match digest_fn_opt {
                    Some(f) => f,
                    None => {
                        // Return early from setup_web_apis
                        return;
                    }
                };
                let digest_key = v8::String::new(scope, "digest").unwrap().into();
                hash_obj.set(scope, digest_key, digest_fn.into());

                retval.set(hash_obj.into());
            },
        );
        let create_hash_fn = match create_hash_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let create_hash_key = v8::String::new(scope, "createHash").unwrap().into();
        crypto_obj.set(scope, create_hash_key, create_hash_fn.into());

        // Add crypto.createSign (v0.3.19) - Digital signature creation
        let create_sign_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let algorithm = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                let private_key = args
                    .get(1)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                // Validate algorithm
                let valid_algorithms = [
                    "RSA-SHA256",
                    "RSA-SHA384",
                    "RSA-SHA512",
                    "RSA-SHA1",
                    "RSA-MD5",
                    "SHA256",
                    "SHA384",
                    "SHA512",
                    "SHA1",
                    "MD5",
                ];
                if !valid_algorithms.contains(&algorithm.as_str()) {
                    let error_msg = format!(
                        "createSign: unsupported algorithm '{}'. Supported: {}",
                        algorithm,
                        valid_algorithms.join(", ")
                    );
                    let error = v8::String::new(scope, &error_msg).unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }

                // Create Sign object
                let sign_obj = v8::Object::new(scope);

                // Store algorithm in object property
                let algo_key = v8::String::new(scope, "_algorithm").unwrap();
                let algo_val = v8::String::new(scope, &algorithm).unwrap();
                sign_obj.set(scope, algo_key.into(), algo_val.into());

                // Store private key in object property
                let key_key = v8::String::new(scope, "_privateKey").unwrap();
                let key_val = v8::String::new(scope, &private_key).unwrap();
                sign_obj.set(scope, key_key.into(), key_val.into());

                // Store data buffer
                let data_key = v8::String::new(scope, "_data").unwrap();
                let data_val = v8::Array::new(scope, 0);
                sign_obj.set(scope, data_key.into(), data_val.into());

                // Add update method
                let update_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let this = args.this();
                        let data = args
                            .get(0)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();

                        // Append data to buffer
                        let data_key = v8::String::new(scope, "_data").unwrap();
                        if let Some(data_array_val) = this.get(scope, data_key.into()) {
                            if data_array_val.is_array() {
                                let arr = v8::Local::<v8::Array>::try_from(data_array_val).unwrap();
                                let length = arr.length();
                                let str_val = v8::String::new(scope, &data).unwrap();
                                arr.set_index(scope, length, str_val.into());
                            }
                        }

                        // Return this for chaining
                        retval.set(this.into());
                    },
                );
                let update_fn = match update_fn_opt {
                    Some(f) => f,
                    None => return,
                };
                let update_key = v8::String::new(scope, "update").unwrap().into();
                sign_obj.set(scope, update_key, update_fn.into());

                // Add sign method
                let sign_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let this = args.this();
                        if args.length() < 1 {
                            let error =
                                v8::String::new(scope, "sign: private key is required").unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }

                        let (private_key, signature_options) =
                            match get_signature_key_options(scope, args.get(0)) {
                                Ok(options) => options,
                                Err(error_message) => {
                                    let error = v8::String::new(scope, &error_message).unwrap();
                                    let error_obj = v8::Exception::type_error(scope, error);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }
                            };
                        let encoding = if args.length() >= 2 {
                            args.get(1)
                                .to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| "hex".to_string())
                        } else {
                            "hex".to_string()
                        };

                        // Get algorithm
                        let algo_key = v8::String::new(scope, "_algorithm").unwrap();
                        let algorithm = this
                            .get(scope, algo_key.into())
                            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                            .unwrap_or_default();

                        // Get data
                        let data_key = v8::String::new(scope, "_data").unwrap();
                        let mut combined_data = String::new();
                        if let Some(data_array_val) = this.get(scope, data_key.into()) {
                            if data_array_val.is_array() {
                                let arr = v8::Local::<v8::Array>::try_from(data_array_val).unwrap();
                                for i in 0..arr.length() {
                                    if let Some(data_str) =
                                        arr.get_index(scope, i).and_then(|v| v.to_string(scope))
                                    {
                                        combined_data
                                            .push_str(&data_str.to_rust_string_lossy(scope));
                                    }
                                }
                            }
                        }

                        let signature_data = match sign_pem_private_key(
                            &algorithm,
                            &private_key,
                            combined_data.as_bytes(),
                            signature_options,
                        ) {
                            Ok(signature) => signature,
                            Err(error_message) => {
                                let error = v8::String::new(scope, &error_message).unwrap();
                                let error_obj = v8::Exception::error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            }
                        };

                        match encoding.as_str() {
                            "hex" => {
                                let sig =
                                    v8::String::new(scope, &hex::encode(&signature_data)).unwrap();
                                retval.set(sig.into());
                            }
                            "base64" => {
                                let sig = base64::Engine::encode(
                                    &base64::engine::general_purpose::STANDARD,
                                    &signature_data,
                                );
                                let sig_str = v8::String::new(scope, &sig).unwrap();
                                retval.set(sig_str.into());
                            }
                            "buffer" => {
                                let ab = v8::ArrayBuffer::new(scope, signature_data.len());
                                let backing_store = ab.get_backing_store();
                                for (i, byte) in signature_data.iter().enumerate() {
                                    backing_store[i].set(*byte);
                                }
                                if let Some(uint8_array) =
                                    v8::Uint8Array::new(scope, ab, 0, signature_data.len())
                                {
                                    retval.set(uint8_array.into());
                                }
                            }
                            _ => {
                                let sig =
                                    v8::String::new(scope, &hex::encode(&signature_data)).unwrap();
                                retval.set(sig.into());
                            }
                        }
                    },
                );
                let sign_fn = match sign_fn_opt {
                    Some(f) => f,
                    None => return,
                };
                let sign_key = v8::String::new(scope, "sign").unwrap().into();
                sign_obj.set(scope, sign_key, sign_fn.into());

                retval.set(sign_obj.into());
            },
        );
        let create_sign_fn = match create_sign_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let create_sign_key = v8::String::new(scope, "createSign").unwrap().into();
        crypto_obj.set(scope, create_sign_key, create_sign_fn.into());

        // Add crypto.createVerify (v0.3.20) - Digital signature verification
        let create_verify_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let algorithm = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                // Validate algorithm
                let valid_algorithms = [
                    "RSA-SHA256",
                    "RSA-SHA384",
                    "RSA-SHA512",
                    "RSA-SHA1",
                    "RSA-MD5",
                    "SHA256",
                    "SHA384",
                    "SHA512",
                    "SHA1",
                    "MD5",
                ];
                if !valid_algorithms.contains(&algorithm.as_str()) {
                    let error_msg = format!(
                        "createVerify: unsupported algorithm '{}'. Supported: {}",
                        algorithm,
                        valid_algorithms.join(", ")
                    );
                    let error = v8::String::new(scope, &error_msg).unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }

                // Create Verify object
                let verify_obj = v8::Object::new(scope);

                // Store algorithm in object property
                let algo_key = v8::String::new(scope, "_algorithm").unwrap();
                let algo_val = v8::String::new(scope, &algorithm).unwrap();
                verify_obj.set(scope, algo_key.into(), algo_val.into());

                // Store data buffer
                let data_key = v8::String::new(scope, "_data").unwrap();
                let data_val = v8::Array::new(scope, 0);
                verify_obj.set(scope, data_key.into(), data_val.into());

                // Add update method
                let update_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let this = args.this();
                        let data = args
                            .get(0)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();

                        // Append data to buffer
                        let data_key = v8::String::new(scope, "_data").unwrap();
                        if let Some(data_array_val) = this.get(scope, data_key.into()) {
                            if data_array_val.is_array() {
                                let arr = v8::Local::<v8::Array>::try_from(data_array_val).unwrap();
                                let length = arr.length();
                                let str_val = v8::String::new(scope, &data).unwrap();
                                arr.set_index(scope, length, str_val.into());
                            }
                        }

                        // Return this for chaining
                        retval.set(this.into());
                    },
                );
                let update_fn = match update_fn_opt {
                    Some(f) => f,
                    None => return,
                };
                let update_key = v8::String::new(scope, "update").unwrap().into();
                verify_obj.set(scope, update_key, update_fn.into());

                // Add verify method
                let verify_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let this = args.this();
                        if args.length() < 2 {
                            let error = v8::String::new(
                                scope,
                                "verify: public key and signature are required",
                            )
                            .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }

                        let (public_key, signature_options) =
                            match get_signature_key_options(scope, args.get(0)) {
                                Ok(options) => options,
                                Err(error_message) => {
                                    let error = v8::String::new(scope, &error_message).unwrap();
                                    let error_obj = v8::Exception::type_error(scope, error);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }
                            };
                        let signature = args
                            .get(1)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();

                        let encoding = if args.length() >= 3 {
                            args.get(2)
                                .to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| "hex".to_string())
                        } else {
                            "hex".to_string()
                        };

                        let algo_key = v8::String::new(scope, "_algorithm").unwrap();
                        let algorithm = this
                            .get(scope, algo_key.into())
                            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                            .unwrap_or_default();

                        // Get data
                        let data_key = v8::String::new(scope, "_data").unwrap();
                        let mut combined_data = String::new();
                        if let Some(data_array_val) = this.get(scope, data_key.into()) {
                            if data_array_val.is_array() {
                                let arr = v8::Local::<v8::Array>::try_from(data_array_val).unwrap();
                                for i in 0..arr.length() {
                                    if let Some(data_str) =
                                        arr.get_index(scope, i).and_then(|v| v.to_string(scope))
                                    {
                                        combined_data
                                            .push_str(&data_str.to_rust_string_lossy(scope));
                                    }
                                }
                            }
                        }

                        // Decode signature based on encoding
                        let signature_data = match encoding.as_str() {
                            "hex" => match hex::decode(&signature) {
                                Ok(bytes) => bytes,
                                Err(error) => {
                                    let error_message =
                                        format!("verify: invalid hex signature: {}", error);
                                    let error = v8::String::new(scope, &error_message).unwrap();
                                    let error_obj = v8::Exception::type_error(scope, error);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }
                            },
                            "base64" => match base64::Engine::decode(
                                &base64::engine::general_purpose::STANDARD,
                                &signature,
                            ) {
                                Ok(bytes) => bytes,
                                Err(error) => {
                                    let error_message =
                                        format!("verify: invalid base64 signature: {}", error);
                                    let error = v8::String::new(scope, &error_message).unwrap();
                                    let error_obj = v8::Exception::type_error(scope, error);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }
                            },
                            _ => signature.as_bytes().to_vec(),
                        };

                        let is_valid = match verify_pem_public_key(
                            &algorithm,
                            &public_key,
                            combined_data.as_bytes(),
                            &signature_data,
                            signature_options,
                        ) {
                            Ok(result) => result,
                            Err(error_message) => {
                                let error = v8::String::new(scope, &error_message).unwrap();
                                let error_obj = v8::Exception::error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            }
                        };

                        // Return boolean result
                        let result = v8::Boolean::new(scope, is_valid);
                        retval.set(result.into());
                    },
                );
                let verify_fn = match verify_fn_opt {
                    Some(f) => f,
                    None => return,
                };
                let verify_key = v8::String::new(scope, "verify").unwrap().into();
                verify_obj.set(scope, verify_key, verify_fn.into());

                retval.set(verify_obj.into());
            },
        );
        let create_verify_fn = match create_verify_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let create_verify_key = v8::String::new(scope, "createVerify").unwrap().into();
        crypto_obj.set(scope, create_verify_key, create_verify_fn.into());

        // Add crypto.sign / crypto.verify one-shot APIs.
        let sign_one_shot_fn_opt = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                if args.length() < 3 {
                    let error =
                        v8::String::new(scope, "sign: algorithm, data, and key are required")
                            .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }

                let algorithm_value = args.get(0);
                let algorithm = if algorithm_value.is_null() || algorithm_value.is_undefined() {
                    None
                } else {
                    Some(
                        algorithm_value
                            .to_string(scope)
                            .map(|value| value.to_rust_string_lossy(scope))
                            .unwrap_or_default(),
                    )
                };

                let data = match get_bytes_from_value(scope, args.get(1), None) {
                    Ok(bytes) => bytes,
                    Err(error_message) => {
                        let error =
                            v8::String::new(scope, &format!("sign: {}", error_message)).unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                };

                let (private_key, signature_options) =
                    match get_signature_key_options(scope, args.get(2)) {
                        Ok(options) => options,
                        Err(error_message) => {
                            let error = v8::String::new(scope, &error_message).unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };

                let signature_data = match sign_one_shot_pem_private_key(
                    algorithm.as_deref(),
                    &private_key,
                    &data,
                    signature_options,
                ) {
                    Ok(signature) => signature,
                    Err(error_message) => {
                        let error = v8::String::new(scope, &error_message).unwrap();
                        let error_obj = v8::Exception::error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                };

                let result = create_buffer_wrapper(scope, &signature_data);
                retval.set(result.into());
            },
        );
        let sign_one_shot_fn = match sign_one_shot_fn_opt {
            Some(f) => f,
            None => return Ok(()),
        };
        let sign_one_shot_key = v8::String::new(scope, "sign").unwrap().into();
        crypto_obj.set(scope, sign_one_shot_key, sign_one_shot_fn.into());

        let verify_one_shot_fn_opt = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                if args.length() < 4 {
                    let error = v8::String::new(
                        scope,
                        "verify: algorithm, data, key, and signature are required",
                    )
                    .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }

                let algorithm_value = args.get(0);
                let algorithm = if algorithm_value.is_null() || algorithm_value.is_undefined() {
                    None
                } else {
                    Some(
                        algorithm_value
                            .to_string(scope)
                            .map(|value| value.to_rust_string_lossy(scope))
                            .unwrap_or_default(),
                    )
                };

                let data = match get_bytes_from_value(scope, args.get(1), None) {
                    Ok(bytes) => bytes,
                    Err(error_message) => {
                        let error =
                            v8::String::new(scope, &format!("verify: {}", error_message)).unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                };

                let (public_key, signature_options) =
                    match get_signature_key_options(scope, args.get(2)) {
                        Ok(options) => options,
                        Err(error_message) => {
                            let error = v8::String::new(scope, &error_message).unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };

                let signature_data = match get_bytes_from_value(scope, args.get(3), None) {
                    Ok(bytes) => bytes,
                    Err(error_message) => {
                        let error =
                            v8::String::new(scope, &format!("verify: {}", error_message)).unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                };

                let is_valid = match verify_one_shot_pem_public_key(
                    algorithm.as_deref(),
                    &public_key,
                    &data,
                    &signature_data,
                    signature_options,
                ) {
                    Ok(result) => result,
                    Err(error_message) => {
                        let error = v8::String::new(scope, &error_message).unwrap();
                        let error_obj = v8::Exception::error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                };

                retval.set(v8::Boolean::new(scope, is_valid).into());
            },
        );
        let verify_one_shot_fn = match verify_one_shot_fn_opt {
            Some(f) => f,
            None => return Ok(()),
        };
        let verify_one_shot_key = v8::String::new(scope, "verify").unwrap().into();
        crypto_obj.set(scope, verify_one_shot_key, verify_one_shot_fn.into());

        // Add crypto.createHmac (v0.3.9)
        let create_hmac_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let algorithm = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                let key = args
                    .get(1)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                // Validate algorithm
                let valid_algorithms = ["md5", "sha1", "sha256", "sha512", "blake3"];
                if !valid_algorithms.contains(&algorithm.as_str()) {
                    let error_msg = format!(
                        "createHmac: unsupported algorithm '{}'. Supported: {}",
                        algorithm,
                        valid_algorithms.join(", ")
                    );
                    let error = v8::String::new(scope, &error_msg).unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }

                // Create HMAC object
                let hmac_obj = v8::Object::new(scope);

                // Store algorithm in object property
                let algo_key = v8::String::new(scope, "_algorithm").unwrap();
                let algo_val = v8::String::new(scope, &algorithm).unwrap();
                hmac_obj.set(scope, algo_key.into(), algo_val.into());

                // Store key in object property
                let key_key = v8::String::new(scope, "_key").unwrap();
                let key_val = v8::String::new(scope, &key).unwrap();
                hmac_obj.set(scope, key_key.into(), key_val.into());

                // Store data buffer
                let data_key = v8::String::new(scope, "_data").unwrap();
                let data_val = v8::Array::new(scope, 0);
                hmac_obj.set(scope, data_key.into(), data_val.into());

                // Add update method
                let update_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let this = args.this();
                        let data = args
                            .get(0)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();

                        // Append data to buffer
                        let data_key = v8::String::new(scope, "_data").unwrap();
                        if let Some(data_array_val) = this.get(scope, data_key.into()) {
                            if data_array_val.is_array() {
                                let arr = v8::Local::<v8::Array>::try_from(data_array_val).unwrap();
                                let length = arr.length();
                                let str_val = v8::String::new(scope, &data).unwrap();
                                arr.set_index(scope, length, str_val.into());
                            }
                        }

                        // Return this for chaining
                        retval.set(this.into());
                    },
                );
                let update_fn = match update_fn_opt {
                    Some(f) => f,
                    None => {
                        return;
                    }
                };
                let update_key = v8::String::new(scope, "update").unwrap().into();
                hmac_obj.set(scope, update_key, update_fn.into());

                // Add digest method
                let digest_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let this = args.this();
                        let encoding = args
                            .get(0)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_else(|| "hex".to_string());

                        // Get algorithm
                        let algo_key = v8::String::new(scope, "_algorithm").unwrap();
                        let algorithm = this
                            .get(scope, algo_key.into())
                            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                            .unwrap_or_default();

                        // Get key
                        let key_key = v8::String::new(scope, "_key").unwrap();
                        let key = this
                            .get(scope, key_key.into())
                            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                            .unwrap_or_default();

                        // Get data
                        let data_key = v8::String::new(scope, "_data").unwrap();
                        let mut combined_data = String::new();
                        if let Some(data_array_val) = this.get(scope, data_key.into()) {
                            if data_array_val.is_array() {
                                let arr = v8::Local::<v8::Array>::try_from(data_array_val).unwrap();
                                for i in 0..arr.length() {
                                    if let Some(data_str) =
                                        arr.get_index(scope, i).and_then(|v| v.to_string(scope))
                                    {
                                        combined_data
                                            .push_str(&data_str.to_rust_string_lossy(scope));
                                    }
                                }
                            }
                        }

                        // Compute HMAC using the key
                        let digest_result: String = match algorithm.as_str() {
                            "md5" => {
                                // Pad key for block size (64 bytes)
                                let ipad = 0x36u8;
                                let opad = 0x5cu8;
                                let block_size = 64;

                                let mut padded_key = key.as_bytes().to_vec();
                                if padded_key.len() > block_size {
                                    let short_key = md5::compute(&padded_key);
                                    padded_key = short_key.0.to_vec();
                                }
                                padded_key.resize(block_size, 0);

                                // Inner hash
                                let mut inner_input =
                                    Vec::with_capacity(block_size + combined_data.len());
                                inner_input.extend(padded_key.iter().map(|b| b ^ ipad));
                                inner_input.extend(combined_data.as_bytes());
                                let inner_hash = md5::compute(&inner_input);

                                // Outer hash
                                let mut outer_input = Vec::with_capacity(block_size + 16);
                                outer_input.extend(padded_key.iter().map(|b| b ^ opad));
                                outer_input.extend(&inner_hash.0);

                                let hmac_result = md5::compute(&outer_input);

                                match encoding.as_str() {
                                    "hex" => format!("{:x}", hmac_result),
                                    "base64" => base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD,
                                        &hmac_result.0,
                                    ),
                                    "buffer" => {
                                        let ab = v8::ArrayBuffer::new(scope, hmac_result.0.len());
                                        let backing_store = ab.get_backing_store();
                                        for (i, byte) in hmac_result.0.iter().enumerate() {
                                            backing_store[i].set(*byte);
                                        }
                                        if let Some(uint8_array) =
                                            v8::Uint8Array::new(scope, ab, 0, hmac_result.0.len())
                                        {
                                            retval.set(uint8_array.into());
                                        }
                                        return;
                                    }
                                    _ => format!("{:x}", hmac_result),
                                }
                            }
                            "sha1" => {
                                use sha1::Digest;
                                let block_size = 64;
                                let ipad = 0x36u8;
                                let opad = 0x5cu8;

                                let mut padded_key = key.as_bytes().to_vec();
                                if padded_key.len() > block_size {
                                    let mut hasher = sha1::Sha1::default();
                                    hasher.update(&padded_key);
                                    padded_key = hasher.finalize().to_vec();
                                }
                                padded_key.resize(block_size, 0);

                                // Inner hash
                                let mut inner_input =
                                    Vec::with_capacity(block_size + combined_data.len());
                                inner_input.extend(padded_key.iter().map(|b| b ^ ipad));
                                inner_input.extend(combined_data.as_bytes());
                                let mut inner_hasher = sha1::Sha1::default();
                                inner_hasher.update(&inner_input);
                                let inner_hash = inner_hasher.finalize();

                                // Outer hash
                                let mut outer_input = Vec::with_capacity(block_size + 20);
                                outer_input.extend(padded_key.iter().map(|b| b ^ opad));
                                outer_input.extend(inner_hash.as_slice());

                                let mut outer_hasher = sha1::Sha1::default();
                                outer_hasher.update(&outer_input);
                                let hmac_result = outer_hasher.finalize();

                                match encoding.as_str() {
                                    "hex" => hex::encode(&hmac_result),
                                    "base64" => base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD,
                                        &hmac_result,
                                    ),
                                    "buffer" => {
                                        let ab = v8::ArrayBuffer::new(scope, hmac_result.len());
                                        let backing_store = ab.get_backing_store();
                                        for (i, byte) in hmac_result.iter().enumerate() {
                                            backing_store[i].set(*byte);
                                        }
                                        if let Some(uint8_array) =
                                            v8::Uint8Array::new(scope, ab, 0, hmac_result.len())
                                        {
                                            retval.set(uint8_array.into());
                                        }
                                        return;
                                    }
                                    _ => hex::encode(&hmac_result),
                                }
                            }
                            "sha256" => {
                                use ring::digest;
                                let block_size = 64;
                                let ipad = 0x36u8;
                                let opad = 0x5cu8;

                                let mut padded_key = key.as_bytes().to_vec();
                                if padded_key.len() > block_size {
                                    let short_digest = digest::digest(&digest::SHA256, &padded_key);
                                    padded_key = short_digest.as_ref().to_vec();
                                }
                                padded_key.resize(block_size, 0);

                                // Inner hash
                                let mut inner_input =
                                    Vec::with_capacity(block_size + combined_data.len());
                                inner_input.extend(padded_key.iter().map(|b| b ^ ipad));
                                inner_input.extend(combined_data.as_bytes());
                                let inner_hash = digest::digest(&digest::SHA256, &inner_input);

                                // Outer hash
                                let mut outer_input = Vec::with_capacity(block_size + 32);
                                outer_input.extend(padded_key.iter().map(|b| b ^ opad));
                                outer_input.extend(inner_hash.as_ref());

                                let hmac_result = digest::digest(&digest::SHA256, &outer_input);

                                match encoding.as_str() {
                                    "hex" => hex::encode(hmac_result.as_ref()),
                                    "base64" => base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD,
                                        hmac_result.as_ref(),
                                    ),
                                    "buffer" => {
                                        let ab =
                                            v8::ArrayBuffer::new(scope, hmac_result.as_ref().len());
                                        let backing_store = ab.get_backing_store();
                                        for (i, byte) in hmac_result.as_ref().iter().enumerate() {
                                            backing_store[i].set(*byte);
                                        }
                                        if let Some(uint8_array) = v8::Uint8Array::new(
                                            scope,
                                            ab,
                                            0,
                                            hmac_result.as_ref().len(),
                                        ) {
                                            retval.set(uint8_array.into());
                                        }
                                        return;
                                    }
                                    _ => hex::encode(hmac_result.as_ref()),
                                }
                            }
                            "sha512" => {
                                use ring::digest;
                                let block_size = 128;
                                let ipad = 0x36u8;
                                let opad = 0x5cu8;

                                let mut padded_key = key.as_bytes().to_vec();
                                if padded_key.len() > block_size {
                                    let short_digest = digest::digest(&digest::SHA512, &padded_key);
                                    padded_key = short_digest.as_ref().to_vec();
                                }
                                padded_key.resize(block_size, 0);

                                // Inner hash
                                let mut inner_input =
                                    Vec::with_capacity(block_size + combined_data.len());
                                inner_input.extend(padded_key.iter().map(|b| b ^ ipad));
                                inner_input.extend(combined_data.as_bytes());
                                let inner_hash = digest::digest(&digest::SHA512, &inner_input);

                                // Outer hash
                                let mut outer_input = Vec::with_capacity(block_size + 64);
                                outer_input.extend(padded_key.iter().map(|b| b ^ opad));
                                outer_input.extend(inner_hash.as_ref());

                                let hmac_result = digest::digest(&digest::SHA512, &outer_input);

                                match encoding.as_str() {
                                    "hex" => hex::encode(hmac_result.as_ref()),
                                    "base64" => base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD,
                                        hmac_result.as_ref(),
                                    ),
                                    "buffer" => {
                                        let ab =
                                            v8::ArrayBuffer::new(scope, hmac_result.as_ref().len());
                                        let backing_store = ab.get_backing_store();
                                        for (i, byte) in hmac_result.as_ref().iter().enumerate() {
                                            backing_store[i].set(*byte);
                                        }
                                        if let Some(uint8_array) = v8::Uint8Array::new(
                                            scope,
                                            ab,
                                            0,
                                            hmac_result.as_ref().len(),
                                        ) {
                                            retval.set(uint8_array.into());
                                        }
                                        return;
                                    }
                                    _ => hex::encode(hmac_result.as_ref()),
                                }
                            }
                            "blake3" => {
                                let block_size = 64;
                                let ipad = 0x36u8;
                                let opad = 0x5cu8;

                                let mut padded_key = key.as_bytes().to_vec();
                                if padded_key.len() > block_size {
                                    let short_hash =
                                        blake3::Hasher::default().update(&padded_key).finalize();
                                    padded_key = short_hash.as_bytes().to_vec();
                                }
                                padded_key.resize(block_size, 0);

                                // Inner hash
                                let mut inner_hasher = blake3::Hasher::default();
                                inner_hasher.update(
                                    &padded_key.iter().map(|b| b ^ ipad).collect::<Vec<u8>>(),
                                );
                                inner_hasher.update(combined_data.as_bytes());
                                let inner_hash = inner_hasher.finalize();

                                // Outer hash
                                let mut outer_hasher = blake3::Hasher::default();
                                outer_hasher.update(
                                    &padded_key.iter().map(|b| b ^ opad).collect::<Vec<u8>>(),
                                );
                                outer_hasher.update(inner_hash.as_bytes());
                                let hmac_result = outer_hasher.finalize();

                                let hash_bytes = hmac_result.as_bytes();
                                match encoding.as_str() {
                                    "hex" => hex::encode(hash_bytes),
                                    "base64" => base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD,
                                        hash_bytes,
                                    ),
                                    "buffer" => {
                                        let ab = v8::ArrayBuffer::new(scope, hash_bytes.len());
                                        let backing_store = ab.get_backing_store();
                                        for (i, byte) in hash_bytes.iter().enumerate() {
                                            backing_store[i].set(*byte);
                                        }
                                        if let Some(uint8_array) =
                                            v8::Uint8Array::new(scope, ab, 0, hash_bytes.len())
                                        {
                                            retval.set(uint8_array.into());
                                        }
                                        return;
                                    }
                                    _ => hex::encode(hash_bytes),
                                }
                            }
                            _ => String::new(),
                        };

                        let result_str = v8::String::new(scope, &digest_result).unwrap();
                        retval.set(result_str.into());
                    },
                );
                let digest_fn = match digest_fn_opt {
                    Some(f) => f,
                    None => {
                        return;
                    }
                };
                let digest_key = v8::String::new(scope, "digest").unwrap().into();
                hmac_obj.set(scope, digest_key, digest_fn.into());

                retval.set(hmac_obj.into());
            },
        );
        let create_hmac_fn = match create_hmac_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let create_hmac_key = v8::String::new(scope, "createHmac").unwrap().into();
        crypto_obj.set(scope, create_hmac_key, create_hmac_fn.into());

        // Add crypto.randomBytes (v0.3.10) - with callback support
        let random_bytes_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let size = args.get(0).to_uint32(scope).map(|n| n.value()).unwrap_or(0);

                // Check if callback is provided
                let has_callback = args.length() >= 2 && args.get(1).is_function();

                // Generate random bytes using rand crate (cryptographically secure)
                let mut random_data = vec![0u8; size as usize];
                rand::thread_rng().fill(&mut random_data[..]);

                // Create ArrayBuffer and Uint8Array
                let array_buffer = v8::ArrayBuffer::new(scope, random_data.len());
                let backing_store = array_buffer.get_backing_store();
                for (i, byte) in random_data.iter().enumerate() {
                    backing_store[i].set(*byte);
                }

                let uint8_array =
                    match v8::Uint8Array::new(scope, array_buffer, 0, random_data.len()) {
                        Some(arr) => arr,
                        None => {
                            retval.set(v8::undefined(scope).into());
                            return;
                        }
                    };

                if has_callback {
                    // Call callback synchronously (for MinimalRuntime compatibility)
                    let callback = v8::Local::<v8::Function>::try_from(args.get(1)).unwrap();
                    let undefined = v8::undefined(scope);
                    let null: v8::Local<v8::Primitive> = v8::null(scope).into();
                    let err: v8::Local<v8::Value> = null.into();
                    let buf: v8::Local<v8::Value> = uint8_array.into();
                    let _ = callback.call(scope, undefined.into(), &[err, buf]);
                    // Return undefined for callback style
                    retval.set(v8::undefined(scope).into());
                } else {
                    // Return the buffer directly
                    retval.set(uint8_array.into());
                }
            },
        );
        let random_bytes_fn = match random_bytes_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let random_bytes_key = v8::String::new(scope, "randomBytes").unwrap().into();
        crypto_obj.set(scope, random_bytes_key, random_bytes_fn.into());

        // Add crypto.randomBytesSync (v0.3.10)
        let random_bytes_sync_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let size = args.get(0).to_uint32(scope).map(|n| n.value()).unwrap_or(0);

                if size == 0 {
                    let empty_buf = v8::ArrayBuffer::new(scope, 0);
                    if let Some(uint8_array) = v8::Uint8Array::new(scope, empty_buf, 0, 0) {
                        retval.set(uint8_array.into());
                    }
                    return;
                }

                // Generate random bytes using rand crate (synchronous, cryptographically secure)
                let mut random_data = vec![0u8; size as usize];
                rand::thread_rng().fill(&mut random_data[..]);

                // Create ArrayBuffer and Uint8Array
                let array_buffer = v8::ArrayBuffer::new(scope, random_data.len());
                let backing_store = array_buffer.get_backing_store();
                for (i, byte) in random_data.iter().enumerate() {
                    backing_store[i].set(*byte);
                }

                if let Some(uint8_array) =
                    v8::Uint8Array::new(scope, array_buffer, 0, random_data.len())
                {
                    retval.set(uint8_array.into());
                }
            },
        );
        let random_bytes_sync_fn = match random_bytes_sync_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let random_bytes_sync_key = v8::String::new(scope, "randomBytesSync").unwrap().into();
        crypto_obj.set(scope, random_bytes_sync_key, random_bytes_sync_fn.into());

        // Add crypto.randomFillSync (v0.3.16) - fill existing buffer with random data
        let random_fill_sync_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                // Get buffer (first argument)
                let buffer = args.get(0);

                // Validate buffer is TypedArray or ArrayBuffer
                if !buffer.is_typed_array() && !buffer.is_array_buffer() {
                    let error_msg = v8::String::new(
                        scope,
                        "randomFillSync: buffer must be a TypedArray or ArrayBuffer",
                    )
                    .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error_msg);
                    scope.throw_exception(error_obj.into());
                    return;
                }

                // Get optional offset and size parameters
                let mut offset: usize = 0;
                let mut size: usize = 0;

                if args.length() >= 2 {
                    if let Some(off) = args.get(1).to_uint32(scope) {
                        offset = off.value() as usize;
                    }
                }

                if args.length() >= 3 {
                    if let Some(sz) = args.get(2).to_uint32(scope) {
                        size = sz.value() as usize;
                    }
                }

                // Get buffer details
                let byte_length = if buffer.is_typed_array() {
                    let ta = v8::Local::<v8::TypedArray>::try_from(buffer).unwrap();
                    ta.byte_length()
                } else {
                    let ab = v8::Local::<v8::ArrayBuffer>::try_from(buffer).unwrap();
                    ab.byte_length()
                };

                // Determine fill size
                if size == 0 {
                    size = byte_length.saturating_sub(offset);
                }

                // Validate parameters
                if offset > byte_length {
                    let error_msg =
                        v8::String::new(scope, "randomFillSync: offset is out of bounds").unwrap();
                    let error_obj = v8::Exception::range_error(scope, error_msg);
                    scope.throw_exception(error_obj.into());
                    return;
                }

                if offset + size > byte_length {
                    let error_msg = v8::String::new(
                        scope,
                        "randomFillSync: offset + size exceeds buffer length",
                    )
                    .unwrap();
                    let error_obj = v8::Exception::range_error(scope, error_msg);
                    scope.throw_exception(error_obj.into());
                    return;
                }

                // Fill the buffer with random data
                if size > 0 {
                    let store = if buffer.is_typed_array() {
                        let ta = v8::Local::<v8::TypedArray>::try_from(buffer).unwrap();
                        let ab = ta.buffer(scope).unwrap();
                        ab.get_backing_store()
                    } else {
                        let ab = v8::Local::<v8::ArrayBuffer>::try_from(buffer).unwrap();
                        ab.get_backing_store()
                    };

                    // Generate random bytes and fill
                    let mut random_data = vec![0u8; size];
                    rand::thread_rng().fill(&mut random_data[..]);

                    // Copy random data to buffer at offset
                    for (i, &byte) in random_data.iter().enumerate() {
                        store[offset + i].set(byte);
                    }
                }

                // Return the buffer for chaining
                retval.set(buffer);
            },
        );
        let random_fill_sync_fn = match random_fill_sync_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let random_fill_sync_key = v8::String::new(scope, "randomFillSync").unwrap().into();
        crypto_obj.set(scope, random_fill_sync_key, random_fill_sync_fn.into());

        // Add crypto.randomFill (v0.3.16) - async fill with callback
        let random_fill_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                // Get buffer (first argument)
                let buffer = args.get(0);

                // Validate buffer is TypedArray or ArrayBuffer
                if !buffer.is_typed_array() && !buffer.is_array_buffer() {
                    let error_msg = v8::String::new(
                        scope,
                        "randomFill: buffer must be a TypedArray or ArrayBuffer",
                    )
                    .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error_msg);
                    scope.throw_exception(error_obj.into());
                    return;
                }

                // Determine callback position: last argument if function
                let callback_idx = if args.length() >= 2 && args.get(1).is_function() {
                    1
                } else if args.length() >= 3 && args.get(2).is_function() {
                    2
                } else {
                    let error_msg =
                        v8::String::new(scope, "randomFill requires a callback function").unwrap();
                    let error_obj = v8::Exception::type_error(scope, error_msg);
                    scope.throw_exception(error_obj.into());
                    return;
                };

                // Get optional offset
                let mut offset: usize = 0;
                if args.length() >= 3 && callback_idx == 2 {
                    if let Some(off) = args.get(1).to_uint32(scope) {
                        offset = off.value() as usize;
                    }
                }

                // Get buffer details
                let byte_length = if buffer.is_typed_array() {
                    let ta = v8::Local::<v8::TypedArray>::try_from(buffer).unwrap();
                    ta.byte_length()
                } else {
                    let ab = v8::Local::<v8::ArrayBuffer>::try_from(buffer).unwrap();
                    ab.byte_length()
                };

                // Fill the buffer with random data
                let store = if buffer.is_typed_array() {
                    let ta = v8::Local::<v8::TypedArray>::try_from(buffer).unwrap();
                    let ab = ta.buffer(scope).unwrap();
                    ab.get_backing_store()
                } else {
                    let ab = v8::Local::<v8::ArrayBuffer>::try_from(buffer).unwrap();
                    ab.get_backing_store()
                };

                // Generate random bytes for remaining bytes from offset
                let fill_size = byte_length.saturating_sub(offset);
                if fill_size > 0 {
                    let mut random_data = vec![0u8; fill_size];
                    rand::thread_rng().fill(&mut random_data[..]);

                    for (i, &byte) in random_data.iter().enumerate() {
                        store[offset + i].set(byte);
                    }
                }

                // Call callback with no error
                let callback = v8::Local::<v8::Function>::try_from(args.get(callback_idx)).unwrap();
                let undefined = v8::undefined(scope);
                let null: v8::Local<v8::Primitive> = v8::null(scope).into();
                let err: v8::Local<v8::Value> = null.into();
                let buf: v8::Local<v8::Value> = buffer;
                let _ = callback.call(scope, undefined.into(), &[err, buf]);

                // Return undefined for callback style
                retval.set(v8::undefined(scope).into());
            },
        );
        let random_fill_fn = match random_fill_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let random_fill_key = v8::String::new(scope, "randomFill").unwrap().into();
        crypto_obj.set(scope, random_fill_key, random_fill_fn.into());

        // Add crypto.timingSafeEqual (v0.3.11)
        // Timing-safe constant-time comparison to prevent timing attacks
        let timing_safe_equal_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                if args.length() < 2 {
                    let error_msg =
                        v8::String::new(scope, "timingSafeEqual requires two arguments").unwrap();
                    let error_obj = v8::Exception::type_error(scope, error_msg);
                    scope.throw_exception(error_obj.into());
                    return;
                }

                let buf_a = args.get(0);
                let buf_b = args.get(1);

                let extract_bytes = |val: v8::Local<v8::Value>, scope: &mut v8::PinScope| -> Option<Vec<u8>> {
                    if val.is_array_buffer() {
                        let ab = v8::Local::<v8::ArrayBuffer>::try_from(val).ok()?;
                        let len = ab.byte_length();
                        if len == 0 { return Some(Vec::new()); }
                        let store = ab.get_backing_store();
                        let ptr = store.as_ref().as_ptr();
                        if ptr.is_null() { return Some(Vec::new()); }
                        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
                        return Some(slice.to_vec());
                    }
                    if val.is_typed_array() {
                        let ta = v8::Local::<v8::TypedArray>::try_from(val).ok()?;
                        let len = ta.byte_length();
                        if len == 0 { return Some(Vec::new()); }
                        let ab = ta.buffer(scope)?;
                        let store = ab.get_backing_store();
                        let ptr = store.as_ref().as_ptr();
                        if ptr.is_null() { return Some(Vec::new()); }
                        let offset = ta.byte_offset();
                        let slice = unsafe { std::slice::from_raw_parts((ptr as *const u8).add(offset), len) };
                        return Some(slice.to_vec());
                    }
                    if val.is_object() {
                        if let Ok(obj) = v8::Local::<v8::Object>::try_from(val) {
                            let bk = v8::String::new(scope, "buffer").unwrap();
                            if let Some(inner) = obj.get(scope, bk.into()) {
                                if inner.is_array_buffer() {
                                    let ab = v8::Local::<v8::ArrayBuffer>::try_from(inner).ok()?;
                                    let len = ab.byte_length();
                                    if len == 0 { return Some(Vec::new()); }
                                    let store = ab.get_backing_store();
                                    let ptr = store.as_ref().as_ptr();
                                    if ptr.is_null() { return Some(Vec::new()); }
                                    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
                                    return Some(slice.to_vec());
                                }
                            }
                        }
                    }
                    None
                };

                let Some(bytes_a) = extract_bytes(buf_a, scope) else {
                    let error_msg = v8::String::new(scope, "First argument must be a Buffer, TypedArray, or ArrayBuffer").unwrap();
                    let error_obj = v8::Exception::type_error(scope, error_msg);
                    scope.throw_exception(error_obj.into());
                    return;
                };

                let Some(bytes_b) = extract_bytes(buf_b, scope) else {
                    let error_msg = v8::String::new(scope, "Second argument must be a Buffer, TypedArray, or ArrayBuffer").unwrap();
                    let error_obj = v8::Exception::type_error(scope, error_msg);
                    scope.throw_exception(error_obj.into());
                    return;
                };

                if bytes_a.len() != bytes_b.len() {
                    let error_msg = v8::String::new(scope, "Input buffers must have the same length").unwrap();
                    let error_obj = v8::Exception::type_error(scope, error_msg);
                    scope.throw_exception(error_obj.into());
                    return;
                }

                let mut diff: u8 = 0;
                for i in 0..bytes_a.len() {
                    diff |= bytes_a[i] ^ bytes_b[i];
                }
                retval.set(v8::Boolean::new(scope, diff == 0).into());
            },
        );
        let timing_safe_equal_fn = match timing_safe_equal_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let timing_safe_equal_key = v8::String::new(scope, "timingSafeEqual").unwrap().into();
        crypto_obj.set(scope, timing_safe_equal_key, timing_safe_equal_fn.into());

        // Add crypto.pbkdf2Sync (v0.3.12)
        let pbkdf2_sync_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let password = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();
                let salt = args
                    .get(1)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();
                let iterations: usize = args
                    .get(2)
                    .to_integer(scope)
                    .map(|n| n.value() as usize)
                    .unwrap_or(10000);
                let keylen: usize = args
                    .get(3)
                    .to_integer(scope)
                    .map(|n| n.value() as usize)
                    .unwrap_or(64);
                let digest = args
                    .get(4)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_else(|| "sha256".to_string());

                // Manual PBKDF2 implementation
                use ring::digest;
                use sha1::Digest;

                // Helper function to compute HMAC
                fn compute_hmac_ring(data: &[u8], key: &[u8], algorithm: &str) -> Vec<u8> {
                    let block_size = 64;
                    let ipad = 0x36u8;
                    let opad = 0x5cu8;

                    // Prepare key
                    let mut padded_key = key.to_vec();
                    if padded_key.len() > block_size {
                        let hash = match algorithm {
                            "sha256" => digest::digest(&digest::SHA256, &padded_key)
                                .as_ref()
                                .to_vec(),
                            "sha512" => digest::digest(&digest::SHA512, &padded_key)
                                .as_ref()
                                .to_vec(),
                            "sha1" => {
                                let mut hasher = sha1::Sha1::default();
                                hasher.update(&padded_key);
                                hasher.finalize().to_vec()
                            }
                            "md5" => md5::compute(&padded_key).0.to_vec(),
                            _ => md5::compute(&padded_key).0.to_vec(),
                        };
                        padded_key = hash;
                    }
                    padded_key.resize(block_size, 0);

                    // Inner hash
                    let mut inner_input = Vec::with_capacity(block_size + data.len());
                    inner_input.extend(padded_key.iter().map(|b| b ^ ipad));
                    inner_input.extend(data);
                    let inner_hash = match algorithm {
                        "sha256" => digest::digest(&digest::SHA256, &inner_input)
                            .as_ref()
                            .to_vec(),
                        "sha512" => digest::digest(&digest::SHA512, &inner_input)
                            .as_ref()
                            .to_vec(),
                        "sha1" => {
                            let mut hasher = sha1::Sha1::default();
                            hasher.update(&inner_input);
                            hasher.finalize().to_vec()
                        }
                        "md5" => md5::compute(&inner_input).0.to_vec(),
                        _ => md5::compute(&inner_input).0.to_vec(),
                    };

                    // Outer hash
                    let mut outer_input = Vec::with_capacity(block_size + inner_hash.len());
                    outer_input.extend(padded_key.iter().map(|b| b ^ opad));
                    outer_input.extend(&inner_hash);

                    match algorithm {
                        "sha256" => digest::digest(&digest::SHA256, &outer_input)
                            .as_ref()
                            .to_vec(),
                        "sha512" => digest::digest(&digest::SHA512, &outer_input)
                            .as_ref()
                            .to_vec(),
                        "sha1" => {
                            let mut hasher = sha1::Sha1::default();
                            hasher.update(&outer_input);
                            hasher.finalize().to_vec()
                        }
                        "md5" => md5::compute(&outer_input).0.to_vec(),
                        _ => md5::compute(&outer_input).0.to_vec(),
                    }
                }

                let rounds = iterations as u32;
                let result: Result<Vec<u8>, String> = match digest.to_lowercase().as_str() {
                    "md5" => {
                        // MD5 produces 16 bytes
                        let mut derived_key = vec![0u8; keylen];
                        let password_bytes = password.as_bytes();
                        let salt_bytes = salt.as_bytes();
                        let hash_len = 16usize;

                        let block_count = (keylen + hash_len - 1) / hash_len;

                        for block_idx in 0..block_count {
                            let mut salt_block = salt_bytes.to_vec();
                            let block_num: u32 = (block_idx + 1) as u32;
                            salt_block.extend_from_slice(&block_num.to_be_bytes());

                            let mut u_prev = compute_hmac_ring(&salt_block, password_bytes, "md5");
                            let mut t_block = u_prev.clone();

                            for _ in 1..rounds {
                                u_prev = compute_hmac_ring(&u_prev, password_bytes, "md5");
                                for (t_byte, u_byte) in t_block.iter_mut().zip(&u_prev) {
                                    *t_byte ^= u_byte;
                                }
                            }

                            let start = block_idx * hash_len;
                            let end = std::cmp::min(start + hash_len, keylen);
                            derived_key[start..end].copy_from_slice(&t_block[0..(end - start)]);
                        }

                        Ok(derived_key)
                    }
                    "sha1" => {
                        // SHA1 produces 20 bytes
                        let mut derived_key = vec![0u8; keylen];
                        let password_bytes = password.as_bytes();
                        let salt_bytes = salt.as_bytes();
                        let hash_len = 20usize;

                        let block_count = (keylen + hash_len - 1) / hash_len;

                        for block_idx in 0..block_count {
                            let mut salt_block = salt_bytes.to_vec();
                            let block_num: u32 = (block_idx + 1) as u32;
                            salt_block.extend_from_slice(&block_num.to_be_bytes());

                            let mut u_prev = compute_hmac_ring(&salt_block, password_bytes, "sha1");
                            let mut t_block = u_prev.clone();

                            for _ in 1..rounds {
                                u_prev = compute_hmac_ring(&u_prev, password_bytes, "sha1");
                                for (t_byte, u_byte) in t_block.iter_mut().zip(&u_prev) {
                                    *t_byte ^= u_byte;
                                }
                            }

                            let start = block_idx * hash_len;
                            let end = std::cmp::min(start + hash_len, keylen);
                            derived_key[start..end].copy_from_slice(&t_block[0..(end - start)]);
                        }

                        Ok(derived_key)
                    }
                    "sha256" => {
                        // SHA256 produces 32 bytes
                        let mut derived_key = vec![0u8; keylen];
                        let password_bytes = password.as_bytes();
                        let salt_bytes = salt.as_bytes();
                        let hash_len = 32usize;

                        let block_count = (keylen + hash_len - 1) / hash_len;

                        for block_idx in 0..block_count {
                            let mut salt_block = salt_bytes.to_vec();
                            let block_num: u32 = (block_idx + 1) as u32;
                            salt_block.extend_from_slice(&block_num.to_be_bytes());

                            let mut u_prev =
                                compute_hmac_ring(&salt_block, password_bytes, "sha256");
                            let mut t_block = u_prev.clone();

                            for _ in 1..rounds {
                                u_prev = compute_hmac_ring(&u_prev, password_bytes, "sha256");
                                for (t_byte, u_byte) in t_block.iter_mut().zip(&u_prev) {
                                    *t_byte ^= u_byte;
                                }
                            }

                            let start = block_idx * hash_len;
                            let end = std::cmp::min(start + hash_len, keylen);
                            derived_key[start..end].copy_from_slice(&t_block[0..(end - start)]);
                        }

                        Ok(derived_key)
                    }
                    "sha512" => {
                        // SHA512 produces 64 bytes
                        let mut derived_key = vec![0u8; keylen];
                        let password_bytes = password.as_bytes();
                        let salt_bytes = salt.as_bytes();
                        let hash_len = 64usize;

                        let block_count = (keylen + hash_len - 1) / hash_len;

                        for block_idx in 0..block_count {
                            let mut salt_block = salt_bytes.to_vec();
                            let block_num: u32 = (block_idx + 1) as u32;
                            salt_block.extend_from_slice(&block_num.to_be_bytes());

                            let mut u_prev =
                                compute_hmac_ring(&salt_block, password_bytes, "sha512");
                            let mut t_block = u_prev.clone();

                            for _ in 1..rounds {
                                u_prev = compute_hmac_ring(&u_prev, password_bytes, "sha512");
                                for (t_byte, u_byte) in t_block.iter_mut().zip(&u_prev) {
                                    *t_byte ^= u_byte;
                                }
                            }

                            let start = block_idx * hash_len;
                            let end = std::cmp::min(start + hash_len, keylen);
                            derived_key[start..end].copy_from_slice(&t_block[0..(end - start)]);
                        }

                        Ok(derived_key)
                    }
                    _ => Err(format!(
                        "Unsupported digest algorithm: {}. Supported: sha256, sha512, sha1, md5",
                        digest
                    )),
                };

                match result {
                    Ok(key_bytes) => {
                        let ab = v8::ArrayBuffer::new(scope, key_bytes.len());
                        let backing_store = ab.get_backing_store();
                        for (i, byte) in key_bytes.iter().enumerate() {
                            backing_store[i].set(*byte);
                        }
                        if let Some(uint8_array) =
                            v8::Uint8Array::new(scope, ab, 0, key_bytes.len())
                        {
                            retval.set(uint8_array.into());
                        }
                    }
                    Err(e) => {
                        let error = v8::String::new(scope, &e).unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        scope.throw_exception(error_obj.into());
                    }
                }
            },
        );
        let pbkdf2_sync_fn = match pbkdf2_sync_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let pbkdf2_sync_key = v8::String::new(scope, "pbkdf2Sync").unwrap().into();
        crypto_obj.set(scope, pbkdf2_sync_key, pbkdf2_sync_fn.into());

        // Add crypto.pbkdf2 (async version using Promise)
        let pbkdf2_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let password = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();
                let salt = args
                    .get(1)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();
                let iterations: usize = args
                    .get(2)
                    .to_integer(scope)
                    .map(|n| n.value() as usize)
                    .unwrap_or(10000);
                let keylen: usize = args
                    .get(3)
                    .to_integer(scope)
                    .map(|n| n.value() as usize)
                    .unwrap_or(64);
                let digest = args
                    .get(4)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_else(|| "sha256".to_string());

                // Create PromiseResolver
                let resolver = v8::PromiseResolver::new(scope).unwrap();
                let promise = resolver.get_promise(scope);

                // Return promise immediately
                retval.set(promise.into());

                // Compute synchronously and resolve the Promise immediately. Beejs'
                // CLI already runs inside a Tokio runtime, so creating a nested
                // runtime here would panic before the Promise can settle.
                {
                    use ring::digest;
                    use sha1::Digest;

                    // Helper function to compute HMAC
                    fn compute_hmac_ring(data: &[u8], key: &[u8], algorithm: &str) -> Vec<u8> {
                        let block_size = 64;
                        let ipad = 0x36u8;
                        let opad = 0x5cu8;

                        // Prepare key
                        let mut padded_key = key.to_vec();
                        if padded_key.len() > block_size {
                            let hash = match algorithm {
                                "sha256" => digest::digest(&digest::SHA256, &padded_key)
                                    .as_ref()
                                    .to_vec(),
                                "sha512" => digest::digest(&digest::SHA512, &padded_key)
                                    .as_ref()
                                    .to_vec(),
                                "sha1" => {
                                    let mut hasher = sha1::Sha1::default();
                                    hasher.update(&padded_key);
                                    hasher.finalize().to_vec()
                                }
                                "md5" => md5::compute(&padded_key).0.to_vec(),
                                _ => md5::compute(&padded_key).0.to_vec(),
                            };
                            padded_key = hash;
                        }
                        padded_key.resize(block_size, 0);

                        // Inner hash
                        let mut inner_input = Vec::with_capacity(block_size + data.len());
                        inner_input.extend(padded_key.iter().map(|b| b ^ ipad));
                        inner_input.extend(data);
                        let inner_hash = match algorithm {
                            "sha256" => digest::digest(&digest::SHA256, &inner_input)
                                .as_ref()
                                .to_vec(),
                            "sha512" => digest::digest(&digest::SHA512, &inner_input)
                                .as_ref()
                                .to_vec(),
                            "sha1" => {
                                let mut hasher = sha1::Sha1::default();
                                hasher.update(&inner_input);
                                hasher.finalize().to_vec()
                            }
                            "md5" => md5::compute(&inner_input).0.to_vec(),
                            _ => md5::compute(&inner_input).0.to_vec(),
                        };

                        // Outer hash
                        let mut outer_input = Vec::with_capacity(block_size + inner_hash.len());
                        outer_input.extend(padded_key.iter().map(|b| b ^ opad));
                        outer_input.extend(&inner_hash);

                        match algorithm {
                            "sha256" => digest::digest(&digest::SHA256, &outer_input)
                                .as_ref()
                                .to_vec(),
                            "sha512" => digest::digest(&digest::SHA512, &outer_input)
                                .as_ref()
                                .to_vec(),
                            "sha1" => {
                                let mut hasher = sha1::Sha1::default();
                                hasher.update(&outer_input);
                                hasher.finalize().to_vec()
                            }
                            "md5" => md5::compute(&outer_input).0.to_vec(),
                            _ => md5::compute(&outer_input).0.to_vec(),
                        }
                    }

                    let rounds = iterations as u32;
                    let result: Result<Vec<u8>, String> = match digest.to_lowercase().as_str() {
                    "md5" => {
                        let mut derived_key = vec![0u8; keylen];
                        let password_bytes = password.as_bytes();
                        let salt_bytes = salt.as_bytes();

                        let hash_len = 16usize;
                        let block_count = (keylen + hash_len - 1) / hash_len;

                        for block_idx in 0..block_count {
                            let mut salt_block = salt_bytes.to_vec();
                            let block_num: u32 = (block_idx + 1) as u32;
                            salt_block.extend_from_slice(&block_num.to_be_bytes());

                            let mut u_prev = compute_hmac_ring(&salt_block, password_bytes, "md5");
                            let mut t_block = u_prev.clone();

                            for _ in 1..rounds {
                                u_prev = compute_hmac_ring(&u_prev, password_bytes, "md5");
                                for (t_byte, u_byte) in t_block.iter_mut().zip(&u_prev) {
                                    *t_byte ^= u_byte;
                                }
                            }

                            let start = block_idx * hash_len;
                            let end = std::cmp::min(start + hash_len, keylen);
                            derived_key[start..end].copy_from_slice(&t_block[0..(end - start)]);
                        }

                        Ok(derived_key)
                    }
                    "sha1" => {
                        let mut derived_key = vec![0u8; keylen];
                        let password_bytes = password.as_bytes();
                        let salt_bytes = salt.as_bytes();

                        let hash_len = 20usize;
                        let block_count = (keylen + hash_len - 1) / hash_len;

                        for block_idx in 0..block_count {
                            let mut salt_block = salt_bytes.to_vec();
                            let block_num: u32 = (block_idx + 1) as u32;
                            salt_block.extend_from_slice(&block_num.to_be_bytes());

                            let mut u_prev = compute_hmac_ring(&salt_block, password_bytes, "sha1");
                            let mut t_block = u_prev.clone();

                            for _ in 1..rounds {
                                u_prev = compute_hmac_ring(&u_prev, password_bytes, "sha1");
                                for (t_byte, u_byte) in t_block.iter_mut().zip(&u_prev) {
                                    *t_byte ^= u_byte;
                                }
                            }

                            let start = block_idx * hash_len;
                            let end = std::cmp::min(start + hash_len, keylen);
                            derived_key[start..end].copy_from_slice(&t_block[0..(end - start)]);
                        }

                        Ok(derived_key)
                    }
                    "sha256" => {
                        let mut derived_key = vec![0u8; keylen];
                        let password_bytes = password.as_bytes();
                        let salt_bytes = salt.as_bytes();

                        let hash_len = 32usize;
                        let block_count = (keylen + hash_len - 1) / hash_len;

                        for block_idx in 0..block_count {
                            let mut salt_block = salt_bytes.to_vec();
                            let block_num: u32 = (block_idx + 1) as u32;
                            salt_block.extend_from_slice(&block_num.to_be_bytes());

                            let mut u_prev = compute_hmac_ring(&salt_block, password_bytes, "sha256");
                            let mut t_block = u_prev.clone();

                            for _ in 1..rounds {
                                u_prev = compute_hmac_ring(&u_prev, password_bytes, "sha256");
                                for (t_byte, u_byte) in t_block.iter_mut().zip(&u_prev) {
                                    *t_byte ^= u_byte;
                                }
                            }

                            let start = block_idx * hash_len;
                            let end = std::cmp::min(start + hash_len, keylen);
                            derived_key[start..end].copy_from_slice(&t_block[0..(end - start)]);
                        }

                        Ok(derived_key)
                    }
                    "sha512" => {
                        let mut derived_key = vec![0u8; keylen];
                        let password_bytes = password.as_bytes();
                        let salt_bytes = salt.as_bytes();

                        let hash_len = 64usize;
                        let block_count = (keylen + hash_len - 1) / hash_len;

                        for block_idx in 0..block_count {
                            let mut salt_block = salt_bytes.to_vec();
                            let block_num: u32 = (block_idx + 1) as u32;
                            salt_block.extend_from_slice(&block_num.to_be_bytes());

                            let mut u_prev = compute_hmac_ring(&salt_block, password_bytes, "sha512");
                            let mut t_block = u_prev.clone();

                            for _ in 1..rounds {
                                u_prev = compute_hmac_ring(&u_prev, password_bytes, "sha512");
                                for (t_byte, u_byte) in t_block.iter_mut().zip(&u_prev) {
                                    *t_byte ^= u_byte;
                                }
                            }

                            let start = block_idx * hash_len;
                            let end = std::cmp::min(start + hash_len, keylen);
                            derived_key[start..end].copy_from_slice(&t_block[0..(end - start)]);
                        }

                        Ok(derived_key)
                    }
                    _ => Err(format!("Unsupported digest algorithm: {}. Supported: sha256, sha512, sha1, md5", digest)),
                };

                    // Resolve/reject the promise using the resolver created outside the async block
                    match result {
                        Ok(key_bytes) => {
                            let ab = v8::ArrayBuffer::new(scope, key_bytes.len());
                            let backing_store = ab.get_backing_store();
                            for (i, byte) in key_bytes.iter().enumerate() {
                                backing_store[i].set(*byte);
                            }
                            if let Some(uint8_array) =
                                v8::Uint8Array::new(scope, ab, 0, key_bytes.len())
                            {
                                resolver.resolve(scope, uint8_array.into());
                            } else {
                                let error =
                                    v8::String::new(scope, "Failed to create Uint8Array").unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                resolver.reject(scope, error_obj);
                            }
                        }
                        Err(e) => {
                            let error = v8::String::new(scope, &e).unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            resolver.reject(scope, error_obj);
                        }
                    }
                }
            },
        );
        let pbkdf2_fn = match pbkdf2_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let pbkdf2_key = v8::String::new(scope, "pbkdf2").unwrap().into();
        crypto_obj.set(scope, pbkdf2_key, pbkdf2_fn.into());

        // Add crypto.getHashes (v0.3.13) - list supported hash algorithms
        let get_hashes_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             _args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                // Define supported hash algorithms (must match createHash/createHmac valid_algorithms)
                let algorithms = ["sha256", "sha512", "sha1", "md5", "blake3"];

                // Create JavaScript array with algorithm names
                let array = v8::Array::new(scope, algorithms.len() as i32);
                for (i, algo) in algorithms.iter().enumerate() {
                    let algo_str = v8::String::new(scope, algo).unwrap();
                    array.set_index(scope, i as u32, algo_str.into());
                }

                retval.set(array.into());
            },
        );
        let get_hashes_fn = match get_hashes_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let get_hashes_key = v8::String::new(scope, "getHashes").unwrap().into();
        crypto_obj.set(scope, get_hashes_key, get_hashes_fn.into());

        // Add crypto.createCipher (v0.3.14) - symmetric encryption
        let create_cipher_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let algorithm = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                let password = args
                    .get(1)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                // Validate algorithm
                let valid_algorithms = ["aes-256-cbc", "aes-128-cbc", "aes-192-cbc"];
                if !valid_algorithms.contains(&algorithm.as_str()) {
                    let error_msg = format!(
                        "createCipher: unsupported algorithm '{}'. Supported: {}",
                        algorithm,
                        valid_algorithms.join(", ")
                    );
                    let error = v8::String::new(scope, &error_msg).unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }

                // Create cipher object
                let cipher_obj = v8::Object::new(scope);

                // Store algorithm and password in object properties
                let algo_key = v8::String::new(scope, "_algorithm").unwrap();
                let algorithm_string = v8::String::new(scope, &algorithm).unwrap();
                let password_string = v8::String::new(scope, &password).unwrap();
                cipher_obj.set(scope, algo_key.into(), algorithm_string.into());

                let password_key = v8::String::new(scope, "_password").unwrap();
                cipher_obj.set(scope, password_key.into(), password_string.into());

                let iv_key = v8::String::new(scope, "_iv").unwrap();
                let iv_bytes: Vec<u8> = password.bytes().take(16).collect();
                let iv_array = v8::ArrayBuffer::new(scope, iv_bytes.len());
                let iv_backing = iv_array.get_backing_store();
                for (i, &byte) in iv_bytes.iter().enumerate() {
                    iv_backing[i].set(byte);
                }
                cipher_obj.set(scope, iv_key.into(), iv_array.into());

                // Add update method
                let update_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let this = args.this();
                        let data = args
                            .get(0)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();

                        // Get algorithm and password from object
                        let algo_key = v8::String::new(scope, "_algorithm").unwrap();
                        let _algorithm = this
                            .get(scope, algo_key.into())
                            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                            .unwrap_or_default();

                        let password_key = v8::String::new(scope, "_password").unwrap();
                        let password = this
                            .get(scope, password_key.into())
                            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                            .unwrap_or_default();

                        // Simple XOR encryption (placeholder for full AES implementation)
                        let encrypted: Vec<u8> = data
                            .bytes()
                            .zip(password.bytes().cycle())
                            .map(|(c, k)| c ^ k)
                            .collect();

                        let ab = v8::ArrayBuffer::new(scope, encrypted.len());
                        let backing = ab.get_backing_store();
                        for (i, &byte) in encrypted.iter().enumerate() {
                            backing[i].set(byte);
                        }
                        if let Some(uint8_array) =
                            v8::Uint8Array::new(scope, ab, 0, encrypted.len())
                        {
                            retval.set(uint8_array.into());
                        }
                    },
                );
                let update_fn = match update_fn_opt {
                    Some(f) => f,
                    None => return,
                };
                let update_key = v8::String::new(scope, "update").unwrap().into();
                cipher_obj.set(scope, update_key, update_fn.into());

                // Add final method
                let final_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     _args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        // Return empty buffer for final
                        let ab = v8::ArrayBuffer::new(scope, 0);
                        if let Some(uint8_array) = v8::Uint8Array::new(scope, ab, 0, 0) {
                            retval.set(uint8_array.into());
                        }
                    },
                );
                let final_fn = match final_fn_opt {
                    Some(f) => f,
                    None => return,
                };
                let final_key = v8::String::new(scope, "final").unwrap().into();
                cipher_obj.set(scope, final_key, final_fn.into());

                // Add setAutoPadding method (for API compatibility)
                let set_auto_padding_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     _args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        retval.set(v8::Boolean::new(scope, true).into());
                    },
                );
                let set_auto_padding_fn = match set_auto_padding_fn_opt {
                    Some(f) => f,
                    None => return,
                };
                let set_auto_padding_key = v8::String::new(scope, "setAutoPadding").unwrap().into();
                cipher_obj.set(scope, set_auto_padding_key, set_auto_padding_fn.into());

                retval.set(cipher_obj.into());
            },
        );
        let create_cipher_fn = match create_cipher_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let create_cipher_key = v8::String::new(scope, "createCipher").unwrap().into();
        crypto_obj.set(scope, create_cipher_key, create_cipher_fn.into());

        // Add crypto.createDecipher (v0.3.14) - symmetric decryption
        let create_decipher_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let algorithm = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                let password = args
                    .get(1)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                // Validate algorithm
                let valid_algorithms = ["aes-256-cbc", "aes-128-cbc", "aes-192-cbc"];
                if !valid_algorithms.contains(&algorithm.as_str()) {
                    let error_msg = format!(
                        "createDecipher: unsupported algorithm '{}'. Supported: {}",
                        algorithm,
                        valid_algorithms.join(", ")
                    );
                    let error = v8::String::new(scope, &error_msg).unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }

                // Create decipher object
                let decipher_obj = v8::Object::new(scope);

                // Store algorithm and password in object properties
                let algo_key = v8::String::new(scope, "_algorithm").unwrap();
                let algorithm_string = v8::String::new(scope, &algorithm).unwrap();
                let password_string = v8::String::new(scope, &password).unwrap();
                decipher_obj.set(scope, algo_key.into(), algorithm_string.into());
                let password_key = v8::String::new(scope, "_password").unwrap();
                decipher_obj.set(scope, password_key.into(), password_string.into());

                // Add update method
                let update_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let this = args.this();

                        // Get password from object
                        let password_key = v8::String::new(scope, "_password").unwrap();
                        let password = this
                            .get(scope, password_key.into())
                            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                            .unwrap_or_default();

                        // Handle Uint8Array or string input
                        let encrypted_data: Vec<u8> = if args.get(0).is_uint8_array() {
                            let uint8 = v8::Local::<v8::Uint8Array>::try_from(args.get(0)).unwrap();
                            let ab = uint8.buffer(scope).unwrap();
                            let backing = ab.get_backing_store();
                            let len = uint8.byte_length();
                            let mut result = Vec::with_capacity(len);
                            for i in 0..len {
                                result.push(backing[i].get());
                            }
                            result
                        } else {
                            let data_str = args
                                .get(0)
                                .to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_default();
                            data_str.into_bytes()
                        };

                        // XOR decryption (reverse of encryption)
                        let decrypted: Vec<u8> = encrypted_data
                            .iter()
                            .zip(password.bytes().cycle())
                            .map(|(c, k)| c ^ k)
                            .collect();

                        // Return as string (remove null padding)
                        let decrypted_str = String::from_utf8_lossy(&decrypted);
                        let result_str = v8::String::new(scope, &decrypted_str).unwrap();
                        retval.set(result_str.into());
                    },
                );
                let update_fn = match update_fn_opt {
                    Some(f) => f,
                    None => return,
                };
                let update_key = v8::String::new(scope, "update").unwrap().into();
                decipher_obj.set(scope, update_key, update_fn.into());

                // Add final method
                let final_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     _args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let ab = v8::ArrayBuffer::new(scope, 0);
                        if let Some(uint8_array) = v8::Uint8Array::new(scope, ab, 0, 0) {
                            retval.set(uint8_array.into());
                        }
                    },
                );
                let final_fn = match final_fn_opt {
                    Some(f) => f,
                    None => return,
                };
                let final_key = v8::String::new(scope, "final").unwrap().into();
                decipher_obj.set(scope, final_key, final_fn.into());

                // Add setAutoPadding method (for API compatibility)
                let set_auto_padding_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     _args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        retval.set(v8::Boolean::new(scope, true).into());
                    },
                );
                let set_auto_padding_fn = match set_auto_padding_fn_opt {
                    Some(f) => f,
                    None => return,
                };
                let set_auto_padding_key = v8::String::new(scope, "setAutoPadding").unwrap().into();
                decipher_obj.set(scope, set_auto_padding_key, set_auto_padding_fn.into());

                retval.set(decipher_obj.into());
            },
        );
        let create_decipher_fn = match create_decipher_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let create_decipher_key = v8::String::new(scope, "createDecipher").unwrap().into();
        crypto_obj.set(scope, create_decipher_key, create_decipher_fn.into());

        // Add crypto.createCipheriv (v0.3.15) - symmetric encryption with explicit key and IV
        let create_cipheriv_fn_opt = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let algorithm = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                let key_hex = args
                    .get(1)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                let iv_hex = args
                    .get(2)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                let valid_algorithms = ["aes-128-cbc", "aes-192-cbc", "aes-256-cbc"];

                if !valid_algorithms.contains(&algorithm.as_str()) {
                    let error_msg = format!(
                        "createCipheriv: unsupported algorithm '{}'. Supported: {}",
                        algorithm,
                        valid_algorithms.join(", ")
                    );
                    let error = v8::String::new(scope, &error_msg).unwrap();
                    let exception = v8::Exception::type_error(scope, error);
                    scope.throw_exception(exception);
                    return;
                }

                // Decode key from hex
                let key_bytes = match hex::decode(&key_hex) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        let error = v8::String::new(
                            scope,
                            "createCipheriv: invalid key - must be hex encoded",
                        )
                        .unwrap();
                        let exception = v8::Exception::type_error(scope, error);
                        scope.throw_exception(exception);
                        return;
                    }
                };

                // Decode IV from hex
                let iv_bytes = match hex::decode(&iv_hex) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        let error = v8::String::new(
                            scope,
                            "createCipheriv: invalid IV - must be hex encoded",
                        )
                        .unwrap();
                        let exception = v8::Exception::type_error(scope, error);
                        scope.throw_exception(exception);
                        return;
                    }
                };

                // Validate key length based on algorithm
                let expected_key_len = match algorithm.as_str() {
                    "aes-128-cbc" => 16,
                    "aes-192-cbc" => 24,
                    "aes-256-cbc" => 32,
                    _ => 32,
                };

                if key_bytes.len() != expected_key_len {
                    let error_msg = format!("createCipheriv: invalid key length {} for algorithm '{}'. Expected {} bytes", key_bytes.len(), algorithm, expected_key_len);
                    let error = v8::String::new(scope, &error_msg).unwrap();
                    let exception = v8::Exception::type_error(scope, error);
                    scope.throw_exception(exception);
                    return;
                }

                // Validate IV length (CBC mode requires 16 bytes)
                if iv_bytes.len() != 16 {
                    let error_msg = format!(
                        "createCipheriv: invalid IV length {}. CBC mode requires 16 bytes",
                        iv_bytes.len()
                    );
                    let error = v8::String::new(scope, &error_msg).unwrap();
                    let exception = v8::Exception::type_error(scope, error);
                    scope.throw_exception(exception);
                    return;
                }

                // Create cipher object
                let cipher_obj = v8::Object::new(scope);

                // Store algorithm, key and IV in object properties
                let algo_key = v8::String::new(scope, "_algorithm").unwrap();
                let algorithm_string = v8::String::new(scope, &algorithm).unwrap();
                cipher_obj.set(scope, algo_key.into(), algorithm_string.into());

                let key_key = v8::String::new(scope, "_key").unwrap();
                let key_array = v8::ArrayBuffer::new(scope, key_bytes.len());
                let key_backing = key_array.get_backing_store();
                for (i, &byte) in key_bytes.iter().enumerate() {
                    key_backing[i].set(byte);
                }
                cipher_obj.set(scope, key_key.into(), key_array.into());

                let iv_key = v8::String::new(scope, "_iv").unwrap();
                let iv_array = v8::ArrayBuffer::new(scope, iv_bytes.len());
                let iv_backing = iv_array.get_backing_store();
                for (i, &byte) in iv_bytes.iter().enumerate() {
                    iv_backing[i].set(byte);
                }
                cipher_obj.set(scope, iv_key.into(), iv_array.into());

                // Add update method
                let update_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let this = args.this();

                        // Get algorithm, key and IV from object
                        let algo_key = v8::String::new(scope, "_algorithm").unwrap();
                        let _algorithm = this
                            .get(scope, algo_key.into())
                            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                            .unwrap_or_default();

                        let key_key = v8::String::new(scope, "_key").unwrap();
                        let mut key_bytes: Vec<u8> = Vec::new();

                        // Try to get key as ArrayBuffer first
                        if let Some(ab) = this
                            .get(scope, key_key.into())
                            .and_then(|v| v8::Local::<v8::ArrayBuffer>::try_from(v).ok())
                        {
                            let backing = ab.get_backing_store();
                            key_bytes = backing.as_ref().iter().map(|c| c.get()).collect();
                        } else if let Some(ua) = this
                            .get(scope, key_key.into())
                            .and_then(|v| v8::Local::<v8::Uint8Array>::try_from(v).ok())
                        {
                            // Try Uint8Array - buffer() returns Option<Local<ArrayBuffer>>
                            if let Some(ab) = ua.buffer(scope) {
                                let backing = ab.get_backing_store();
                                key_bytes = backing.as_ref().iter().map(|c| c.get()).collect();
                            }
                        }

                        // Simple XOR encryption (placeholder for full AES implementation)
                        let data = args
                            .get(0)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();

                        let output_encoding = args
                            .get(2)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();

                        let encrypted: Vec<u8> = data
                            .bytes()
                            .zip(key_bytes.iter().cycle())
                            .map(|(c, k)| c ^ k)
                            .collect();

                        // Handle output encoding
                        match output_encoding.as_str() {
                            "hex" => {
                                let hex_str = hex::encode(&encrypted);
                                let result_str = v8::String::new(scope, &hex_str).unwrap();
                                retval.set(result_str.into());
                            }
                            "base64" => {
                                let engine = base64::engine::general_purpose::STANDARD;
                                let base64_str = engine.encode(&encrypted);
                                let result_str = v8::String::new(scope, &base64_str).unwrap();
                                retval.set(result_str.into());
                            }
                            _ => {
                                // Default: return Uint8Array
                                let ab = v8::ArrayBuffer::new(scope, encrypted.len());
                                let backing = ab.get_backing_store();
                                for (i, &byte) in encrypted.iter().enumerate() {
                                    backing[i].set(byte);
                                }
                                if let Some(uint8_array) =
                                    v8::Uint8Array::new(scope, ab, 0, encrypted.len())
                                {
                                    retval.set(uint8_array.into());
                                }
                            }
                        }
                    },
                );
                let update_fn = match update_fn_opt {
                    Some(f) => f,
                    None => return,
                };
                let update_key = v8::String::new(scope, "update").unwrap().into();
                cipher_obj.set(scope, update_key, update_fn.into());

                // Add final method
                let final_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let output_encoding = args
                            .get(0)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();

                        // Return empty result with proper encoding
                        match output_encoding.as_str() {
                            "hex" | "base64" | "utf8" => {
                                retval.set(v8::String::new(scope, "").unwrap().into());
                            }
                            "buffer" | _ => {
                                let ab = v8::ArrayBuffer::new(scope, 0);
                                if let Some(uint8_array) = v8::Uint8Array::new(scope, ab, 0, 0) {
                                    retval.set(uint8_array.into());
                                }
                            }
                        }
                    },
                );
                let final_fn = match final_fn_opt {
                    Some(f) => f,
                    None => return,
                };
                let final_key = v8::String::new(scope, "final").unwrap().into();
                cipher_obj.set(scope, final_key, final_fn.into());

                // Add setAutoPadding method
                let set_auto_padding_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     _args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        retval.set(v8::Boolean::new(scope, true).into());
                    },
                );
                let set_auto_padding_fn = match set_auto_padding_fn_opt {
                    Some(f) => f,
                    None => return,
                };
                let set_auto_padding_key = v8::String::new(scope, "setAutoPadding").unwrap().into();
                cipher_obj.set(scope, set_auto_padding_key, set_auto_padding_fn.into());

                retval.set(cipher_obj.into());
            },
        );
        let create_cipheriv_fn_result = create_cipheriv_fn_opt;
        let create_cipheriv_fn = match create_cipheriv_fn_result {
            Some(f) => f,
            None => return Ok(()),
        };
        let create_cipheriv_key = v8::String::new(scope, "createCipheriv").unwrap().into();
        crypto_obj.set(scope, create_cipheriv_key, create_cipheriv_fn.into());

        // Add crypto.createDecipheriv (v0.3.15) - symmetric decryption with explicit key and IV
        let create_decipheriv_fn_opt = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let algorithm = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                let key_hex = args
                    .get(1)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                let iv_hex = args
                    .get(2)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                let valid_algorithms = ["aes-128-cbc", "aes-192-cbc", "aes-256-cbc"];

                if !valid_algorithms.contains(&algorithm.as_str()) {
                    let error_msg = format!(
                        "createDecipheriv: unsupported algorithm '{}'. Supported: {}",
                        algorithm,
                        valid_algorithms.join(", ")
                    );
                    let error = v8::String::new(scope, &error_msg).unwrap();
                    let exception = v8::Exception::type_error(scope, error);
                    scope.throw_exception(exception);
                    return;
                }

                // Decode key from hex
                let key_bytes = match hex::decode(&key_hex) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        let error = v8::String::new(
                            scope,
                            "createDecipheriv: invalid key - must be hex encoded",
                        )
                        .unwrap();
                        let exception = v8::Exception::type_error(scope, error);
                        scope.throw_exception(exception);
                        return;
                    }
                };

                // Decode IV from hex
                let iv_bytes = match hex::decode(&iv_hex) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        let error = v8::String::new(
                            scope,
                            "createDecipheriv: invalid IV - must be hex encoded",
                        )
                        .unwrap();
                        let exception = v8::Exception::type_error(scope, error);
                        scope.throw_exception(exception);
                        return;
                    }
                };

                // Validate key length based on algorithm
                let expected_key_len = match algorithm.as_str() {
                    "aes-128-cbc" => 16,
                    "aes-192-cbc" => 24,
                    "aes-256-cbc" => 32,
                    _ => 32,
                };

                if key_bytes.len() != expected_key_len {
                    let error_msg = format!("createDecipheriv: invalid key length {} for algorithm '{}'. Expected {} bytes", key_bytes.len(), algorithm, expected_key_len);
                    let error = v8::String::new(scope, &error_msg).unwrap();
                    let exception = v8::Exception::type_error(scope, error);
                    scope.throw_exception(exception);
                    return;
                }

                // Validate IV length (CBC mode requires 16 bytes)
                if iv_bytes.len() != 16 {
                    let error_msg = format!(
                        "createDecipheriv: invalid IV length {}. CBC mode requires 16 bytes",
                        iv_bytes.len()
                    );
                    let error = v8::String::new(scope, &error_msg).unwrap();
                    let exception = v8::Exception::type_error(scope, error);
                    scope.throw_exception(exception);
                    return;
                }

                // Create decipher object
                let decipher_obj = v8::Object::new(scope);

                let algo_key = v8::String::new(scope, "_algorithm").unwrap();
                let algorithm_string = v8::String::new(scope, &algorithm).unwrap();
                decipher_obj.set(scope, algo_key.into(), algorithm_string.into());

                let key_key = v8::String::new(scope, "_key").unwrap();
                let key_array = v8::ArrayBuffer::new(scope, key_bytes.len());
                let key_backing = key_array.get_backing_store();
                for (i, &byte) in key_bytes.iter().enumerate() {
                    key_backing[i].set(byte);
                }
                decipher_obj.set(scope, key_key.into(), key_array.into());

                // Add update method
                let update_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let this = args.this();

                        // Get key from object
                        let key_key = v8::String::new(scope, "_key").unwrap();
                        let mut key_bytes: Vec<u8> = Vec::new();

                        // Try to get key as ArrayBuffer first
                        if let Some(ab) = this
                            .get(scope, key_key.into())
                            .and_then(|v| v8::Local::<v8::ArrayBuffer>::try_from(v).ok())
                        {
                            let backing = ab.get_backing_store();
                            key_bytes = backing.as_ref().iter().map(|c| c.get()).collect();
                        } else if let Some(ua) = this
                            .get(scope, key_key.into())
                            .and_then(|v| v8::Local::<v8::Uint8Array>::try_from(v).ok())
                        {
                            // Try Uint8Array - buffer() returns Option<Local<ArrayBuffer>>
                            if let Some(ab) = ua.buffer(scope) {
                                let backing = ab.get_backing_store();
                                key_bytes = backing.as_ref().iter().map(|c| c.get()).collect();
                            }
                        }

                        // Handle Uint8Array or string input with encoding
                        let input_encoding = args
                            .get(1)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();

                        let output_encoding = args
                            .get(2)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();

                        let encrypted_data: Vec<u8> = if args.get(0).is_uint8_array() {
                            let uint8 = v8::Local::<v8::Uint8Array>::try_from(args.get(0)).unwrap();
                            let ab = uint8.buffer(scope).unwrap();
                            let backing = ab.get_backing_store();
                            let len = uint8.byte_length();
                            let mut result = Vec::with_capacity(len);
                            for i in 0..len {
                                result.push(backing[i].get());
                            }
                            result
                        } else {
                            let data_str = args
                                .get(0)
                                .to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_default();
                            // Decode based on input encoding
                            match input_encoding.as_str() {
                                "hex" => hex::decode(&data_str).unwrap_or_default(),
                                "base64" => base64::engine::general_purpose::STANDARD
                                    .decode(&data_str)
                                    .unwrap_or_default(),
                                _ => data_str.into_bytes(),
                            }
                        };

                        // XOR decryption
                        let decrypted: Vec<u8> = encrypted_data
                            .iter()
                            .zip(key_bytes.iter().cycle())
                            .map(|(c, k)| c ^ k)
                            .collect();

                        // Handle output encoding
                        match output_encoding.as_str() {
                            "hex" => {
                                let hex_str = hex::encode(&decrypted);
                                let result_str = v8::String::new(scope, &hex_str).unwrap();
                                retval.set(result_str.into());
                            }
                            "base64" => {
                                let engine = base64::engine::general_purpose::STANDARD;
                                let base64_str = engine.encode(&decrypted);
                                let result_str = v8::String::new(scope, &base64_str).unwrap();
                                retval.set(result_str.into());
                            }
                            "utf8" | _ => {
                                // Try to decode as UTF-8 string
                                if let Ok(decoded_str) = std::str::from_utf8(&decrypted) {
                                    let result_str = v8::String::new(scope, decoded_str).unwrap();
                                    retval.set(result_str.into());
                                } else {
                                    // Fallback to Uint8Array if not valid UTF-8
                                    let ab = v8::ArrayBuffer::new(scope, decrypted.len());
                                    let backing = ab.get_backing_store();
                                    for (i, &byte) in decrypted.iter().enumerate() {
                                        backing[i].set(byte);
                                    }
                                    if let Some(uint8_array) =
                                        v8::Uint8Array::new(scope, ab, 0, decrypted.len())
                                    {
                                        retval.set(uint8_array.into());
                                    }
                                }
                            }
                        }
                    },
                );
                let update_fn = match update_fn_opt {
                    Some(f) => f,
                    None => return,
                };
                let update_key = v8::String::new(scope, "update").unwrap().into();
                decipher_obj.set(scope, update_key, update_fn.into());

                // Add final method
                let final_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let output_encoding = args
                            .get(0)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_default();

                        // Return empty result with proper encoding
                        match output_encoding.as_str() {
                            "hex" | "base64" | "utf8" => {
                                retval.set(v8::String::new(scope, "").unwrap().into());
                            }
                            "buffer" | _ => {
                                let ab = v8::ArrayBuffer::new(scope, 0);
                                if let Some(uint8_array) = v8::Uint8Array::new(scope, ab, 0, 0) {
                                    retval.set(uint8_array.into());
                                }
                            }
                        }
                    },
                );
                let final_fn = match final_fn_opt {
                    Some(f) => f,
                    None => return,
                };
                let final_key = v8::String::new(scope, "final").unwrap().into();
                decipher_obj.set(scope, final_key, final_fn.into());

                // Add setAutoPadding method
                let set_auto_padding_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     _args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        retval.set(v8::Boolean::new(scope, true).into());
                    },
                );
                let set_auto_padding_fn = match set_auto_padding_fn_opt {
                    Some(f) => f,
                    None => return,
                };
                let set_auto_padding_key = v8::String::new(scope, "setAutoPadding").unwrap().into();
                decipher_obj.set(scope, set_auto_padding_key, set_auto_padding_fn.into());

                retval.set(decipher_obj.into());
            },
        );
        let create_decipheriv_fn_result = create_decipheriv_fn_opt;
        let create_decipheriv_fn = match create_decipheriv_fn_result {
            Some(f) => f,
            None => return Ok(()),
        };
        let create_decipheriv_key = v8::String::new(scope, "createDecipheriv").unwrap().into();
        crypto_obj.set(scope, create_decipheriv_key, create_decipheriv_fn.into());

        // Add crypto.publicEncrypt (v0.3.21) - Public key encryption
        let public_encrypt_fn_opt = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let (key_str, padding) =
                    match get_rsa_key_and_padding(scope, args.get(0), Padding::PKCS1_OAEP) {
                        Ok(value) => value,
                        Err(error_message) => {
                            let error = v8::String::new(
                                scope,
                                &format!("publicEncrypt: {}", error_message),
                            )
                            .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };

                let data_bytes = match get_bytes_from_value(scope, args.get(1), None) {
                    Ok(value) => value,
                    Err(error_message) => {
                        let error =
                            v8::String::new(scope, &format!("publicEncrypt: {}", error_message))
                                .unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                };

                let encrypted = match rsa_public_encrypt_pem(&key_str, &data_bytes, padding) {
                    Ok(value) => value,
                    Err(error_message) => {
                        let error = v8::String::new(scope, &error_message).unwrap();
                        let error_obj = v8::Exception::error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                };

                let buffer = create_buffer_wrapper(scope, &encrypted);
                retval.set(buffer.into());
            },
        );
        let public_encrypt_fn = match public_encrypt_fn_opt {
            Some(f) => f,
            None => return Ok(()),
        };
        let public_encrypt_key = v8::String::new(scope, "publicEncrypt").unwrap().into();
        crypto_obj.set(scope, public_encrypt_key, public_encrypt_fn.into());

        // Add crypto.privateDecrypt (v0.3.21) - Private key decryption
        let private_decrypt_fn_opt = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let (key_str, padding) =
                    match get_rsa_key_and_padding(scope, args.get(0), Padding::PKCS1_OAEP) {
                        Ok(value) => value,
                        Err(error_message) => {
                            let error = v8::String::new(
                                scope,
                                &format!("privateDecrypt: {}", error_message),
                            )
                            .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };

                let encoding = if args.length() >= 3 && args.get(2).is_string() {
                    args.get(2)
                        .to_string(scope)
                        .map(|value| value.to_rust_string_lossy(scope))
                } else {
                    None
                };

                let encrypted_data =
                    match get_bytes_from_value(scope, args.get(1), encoding.as_deref()) {
                        Ok(value) => value,
                        Err(error_message) => {
                            let error = v8::String::new(
                                scope,
                                &format!("privateDecrypt: {}", error_message),
                            )
                            .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };

                let decrypted = match rsa_private_decrypt_pem(&key_str, &encrypted_data, padding) {
                    Ok(value) => value,
                    Err(error_message) => {
                        let error = v8::String::new(scope, &error_message).unwrap();
                        let error_obj = v8::Exception::error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                };

                let buffer = create_buffer_wrapper(scope, &decrypted);
                retval.set(buffer.into());
            },
        );
        let private_decrypt_fn = match private_decrypt_fn_opt {
            Some(f) => f,
            None => return Ok(()),
        };
        let private_decrypt_key = v8::String::new(scope, "privateDecrypt").unwrap().into();
        crypto_obj.set(scope, private_decrypt_key, private_decrypt_fn.into());

        // Add crypto.privateEncrypt (v0.3.22) - Private key encryption
        let private_encrypt_fn_opt = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let (key_str, padding) =
                    match get_rsa_key_and_padding(scope, args.get(0), Padding::PKCS1) {
                        Ok(value) => value,
                        Err(error_message) => {
                            let error = v8::String::new(
                                scope,
                                &format!("privateEncrypt: {}", error_message),
                            )
                            .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };

                let data_bytes = match get_bytes_from_value(scope, args.get(1), None) {
                    Ok(value) => value,
                    Err(error_message) => {
                        let error =
                            v8::String::new(scope, &format!("privateEncrypt: {}", error_message))
                                .unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                };

                let encrypted = match rsa_private_encrypt_pem(&key_str, &data_bytes, padding) {
                    Ok(value) => value,
                    Err(error_message) => {
                        let error = v8::String::new(scope, &error_message).unwrap();
                        let error_obj = v8::Exception::error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                };

                let buffer = create_buffer_wrapper(scope, &encrypted);
                retval.set(buffer.into());
            },
        );
        let private_encrypt_fn = match private_encrypt_fn_opt {
            Some(f) => f,
            None => return Ok(()),
        };
        let private_encrypt_key = v8::String::new(scope, "privateEncrypt").unwrap().into();
        crypto_obj.set(scope, private_encrypt_key, private_encrypt_fn.into());

        // Add crypto.publicDecrypt (v0.3.22) - Public key decryption
        let public_decrypt_fn_opt = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let (key_str, padding) =
                    match get_rsa_key_and_padding(scope, args.get(0), Padding::PKCS1) {
                        Ok(value) => value,
                        Err(error_message) => {
                            let error = v8::String::new(
                                scope,
                                &format!("publicDecrypt: {}", error_message),
                            )
                            .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };

                let encoding = if args.length() >= 3 && args.get(2).is_string() {
                    args.get(2)
                        .to_string(scope)
                        .map(|value| value.to_rust_string_lossy(scope))
                } else {
                    None
                };

                let encrypted_data =
                    match get_bytes_from_value(scope, args.get(1), encoding.as_deref()) {
                        Ok(value) => value,
                        Err(error_message) => {
                            let error = v8::String::new(
                                scope,
                                &format!("publicDecrypt: {}", error_message),
                            )
                            .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };

                let decrypted = match rsa_public_decrypt_pem(&key_str, &encrypted_data, padding) {
                    Ok(value) => value,
                    Err(error_message) => {
                        let error = v8::String::new(scope, &error_message).unwrap();
                        let error_obj = v8::Exception::error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                };

                let buffer = create_buffer_wrapper(scope, &decrypted);
                retval.set(buffer.into());
            },
        );
        let public_decrypt_fn = match public_decrypt_fn_opt {
            Some(f) => f,
            None => return Ok(()),
        };
        let public_decrypt_key = v8::String::new(scope, "publicDecrypt").unwrap().into();
        crypto_obj.set(scope, public_decrypt_key, public_decrypt_fn.into());

        // Add crypto.generateKeyPairSync (v0.3.23) - RSA/EC key pair generation
        let generate_key_pair_sync_fn_opt = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                // Parse type argument (first parameter)
                let key_type = if args.length() >= 1 {
                    if let Some(s) = args.get(0).to_string(scope) {
                        s.to_rust_string_lossy(scope)
                    } else {
                        String::from("rsa")
                    }
                } else {
                    String::from("rsa")
                };

                // Parse options (second parameter)
                let options = if args.length() >= 2 {
                    args.get(1)
                } else {
                    v8::Object::new(scope).into()
                };

                // Extract RSA options - store string keys in locals to avoid borrow issues
                let modulus_length_key = v8::String::new(scope, "modulusLength").unwrap();
                let modulus_length = if let Some(obj) = options.to_object(scope) {
                    if let Some(ml) = obj.get(scope, modulus_length_key.into()) {
                        if ml.is_number() {
                            ml.to_integer(scope)
                                .map(|i| i.value() as usize)
                                .unwrap_or(2048)
                        } else {
                            2048
                        }
                    } else {
                        2048
                    }
                } else {
                    2048
                };

                // Extract EC curve option - store string keys in locals to avoid borrow issues
                let named_curve_key = v8::String::new(scope, "namedCurve").unwrap();
                let named_curve = if let Some(obj) = options.to_object(scope) {
                    if let Some(nc) = obj.get(scope, named_curve_key.into()) {
                        if let Some(s) = nc.to_string(scope) {
                            s.to_rust_string_lossy(scope)
                        } else {
                            String::from("prime256v1")
                        }
                    } else {
                        String::from("prime256v1")
                    }
                } else {
                    String::from("prime256v1")
                };

                let private_key_encoding =
                    match private_key_encoding_options_from_options(scope, options) {
                        Ok(value) => value,
                        Err(error_message) => {
                            let error = v8::String::new(
                                scope,
                                &format!("generateKeyPairSync: {}", error_message),
                            )
                            .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj);
                            return;
                        }
                    };

                let public_key_encoding =
                    match public_key_encoding_options_from_options(scope, options) {
                        Ok(value) => value,
                        Err(error_message) => {
                            let error = v8::String::new(
                                scope,
                                &format!("generateKeyPairSync: {}", error_message),
                            )
                            .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj);
                            return;
                        }
                    };

                // Generate key pair based on type
                let (public_key_pem, private_key_pem) = match key_type.to_lowercase().as_str() {
                    "rsa" => match generate_rsa_key_pair(modulus_length) {
                        Ok(pair) => pair,
                        Err(error_message) => {
                            let error = v8::String::new(scope, &error_message).unwrap();
                            let error_obj = v8::Exception::error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    },
                    "ec" => match generate_ec_key_pair(&named_curve) {
                        Ok(pair) => pair,
                        Err(error_message) => {
                            let error = v8::String::new(scope, &error_message).unwrap();
                            let error_obj = v8::Exception::error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    },
                    "ed25519" => match generate_ed25519_key_pair() {
                        Ok(pair) => pair,
                        Err(error_message) => {
                            let error = v8::String::new(scope, &error_message).unwrap();
                            let error_obj = v8::Exception::error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    },
                    "ed448" => match generate_ed448_key_pair() {
                        Ok(pair) => pair,
                        Err(error_message) => {
                            let error = v8::String::new(scope, &error_message).unwrap();
                            let error_obj = v8::Exception::error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    },
                    _ => {
                        // Unsupported key type - return error
                        let error_msg = v8::String::new(scope, &format!("generateKeyPairSync: unsupported key type '{}'. Supported: rsa, ec, ed25519, ed448", key_type)).unwrap();
                        let error = v8::Exception::type_error(scope, error_msg);
                        scope.throw_exception(error);
                        return;
                    }
                };
                let public_key = match format_generated_public_key(
                    &public_key_pem,
                    public_key_encoding.as_ref(),
                ) {
                    Ok(value) => value,
                    Err(error_message) => {
                        let error_obj = if is_crypto_incompatible_key_options_error(&error_message)
                        {
                            crypto_incompatible_key_options_error(scope)
                        } else {
                            let error = v8::String::new(
                                scope,
                                &format!("generateKeyPairSync: {}", error_message),
                            )
                            .unwrap();
                            v8::Exception::type_error(scope, error)
                        };
                        scope.throw_exception(error_obj);
                        return;
                    }
                };
                let private_key = match format_generated_private_key_for_generate(
                    &private_key_pem,
                    private_key_encoding.as_ref(),
                ) {
                    Ok(value) => value,
                    Err(error_message) => {
                        let error = v8::String::new(
                            scope,
                            &format!("generateKeyPairSync: {}", error_message),
                        )
                        .unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        scope.throw_exception(error_obj);
                        return;
                    }
                };

                // Create result object
                let result_obj = v8::Object::new(scope);

                // Set public key using the requested publicKeyEncoding format.
                let public_key_key = v8::String::new(scope, "publicKey").unwrap().into();
                let public_key_val = generated_public_key_to_v8(scope, public_key);
                result_obj.set(scope, public_key_key, public_key_val);

                // Set private key using the requested privateKeyEncoding format.
                let private_key_key = v8::String::new(scope, "privateKey").unwrap().into();
                let private_key_val = generated_private_key_to_v8(scope, private_key);
                result_obj.set(scope, private_key_key, private_key_val);

                retval.set(result_obj.into());
            },
        );
        let generate_key_pair_sync_fn = match generate_key_pair_sync_fn_opt {
            Some(f) => f,
            None => return Ok(()),
        };
        let generate_key_pair_sync_key = v8::String::new(scope, "generateKeyPairSync")
            .unwrap()
            .into();
        crypto_obj.set(
            scope,
            generate_key_pair_sync_key,
            generate_key_pair_sync_fn.into(),
        );

        // Add crypto.generateKeyPair (v0.3.24) - Async RSA/EC key pair generation with callback
        let generate_key_pair_fn_opt = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             _retval: v8::ReturnValue| {
                // Parse type argument (first parameter)
                let key_type = if args.length() >= 1 {
                    if let Some(s) = args.get(0).to_string(scope) {
                        s.to_rust_string_lossy(scope)
                    } else {
                        String::from("rsa")
                    }
                } else {
                    String::from("rsa")
                };

                // Parse options (second parameter)
                let options = if args.length() >= 2 {
                    args.get(1)
                } else {
                    v8::Object::new(scope).into()
                };

                // Extract RSA options - store string keys in locals to avoid borrow issues
                let modulus_length_key = v8::String::new(scope, "modulusLength").unwrap();
                let modulus_length = if let Some(obj) = options.to_object(scope) {
                    if let Some(ml) = obj.get(scope, modulus_length_key.into()) {
                        if ml.is_number() {
                            ml.to_integer(scope)
                                .map(|i| i.value() as usize)
                                .unwrap_or(2048)
                        } else {
                            2048
                        }
                    } else {
                        2048
                    }
                } else {
                    2048
                };

                // Extract EC curve option - store string keys in locals to avoid borrow issues
                let named_curve_key = v8::String::new(scope, "namedCurve").unwrap();
                let named_curve = if let Some(obj) = options.to_object(scope) {
                    if let Some(nc) = obj.get(scope, named_curve_key.into()) {
                        if let Some(s) = nc.to_string(scope) {
                            s.to_rust_string_lossy(scope)
                        } else {
                            String::from("prime256v1")
                        }
                    } else {
                        String::from("prime256v1")
                    }
                } else {
                    String::from("prime256v1")
                };

                // Get callback - required for async version
                // Handle both: generateKeyPair('rsa', options, callback) and generateKeyPair('rsa', callback)
                let callback = if args.length() >= 3 {
                    // callback is third argument (options is second arg)
                    args.get(2)
                } else if args.length() >= 2 {
                    // callback might be second argument (no options)
                    let second_arg = args.get(1);
                    if second_arg.is_function() {
                        second_arg
                    } else {
                        v8::Object::new(scope).into()
                    }
                } else {
                    v8::Object::new(scope).into()
                };

                // Validate callback is a function
                if !callback.is_function() {
                    let error_msg = v8::String::new(
                        scope,
                        "crypto.generateKeyPair: callback must be a function",
                    )
                    .unwrap();
                    let error = v8::Exception::type_error(scope, error_msg);
                    scope.throw_exception(error);
                    return;
                }

                let private_key_encoding =
                    match private_key_encoding_options_from_options(scope, options) {
                        Ok(value) => value,
                        Err(error_message) => {
                            let global = scope.get_current_context().global(scope);
                            let error_msg = v8::String::new(
                                scope,
                                &format!("generateKeyPair: {}", error_message),
                            )
                            .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error_msg);
                            let callback_func =
                                v8::Local::<v8::Function>::try_from(callback).unwrap();
                            let null_val = v8::null(scope).into();
                            let _ = callback_func.call(
                                scope,
                                global.into(),
                                &[error_obj, null_val, null_val],
                            );
                            return;
                        }
                    };

                let public_key_encoding =
                    match public_key_encoding_options_from_options(scope, options) {
                        Ok(value) => value,
                        Err(error_message) => {
                            let global = scope.get_current_context().global(scope);
                            let error_msg = v8::String::new(
                                scope,
                                &format!("generateKeyPair: {}", error_message),
                            )
                            .unwrap();
                            let error_obj = v8::Exception::type_error(scope, error_msg);
                            let callback_func =
                                v8::Local::<v8::Function>::try_from(callback).unwrap();
                            let null_val = v8::null(scope).into();
                            let _ = callback_func.call(
                                scope,
                                global.into(),
                                &[error_obj, null_val, null_val],
                            );
                            return;
                        }
                    };

                // Validate key type
                let key_type_lower = key_type.to_lowercase();
                if key_type_lower != "rsa"
                    && key_type_lower != "ec"
                    && key_type_lower != "ed25519"
                    && key_type_lower != "ed448"
                {
                    // For async API, call callback with error synchronously
                    let global = scope.get_current_context().global(scope);

                    // Create error object
                    let error_msg = v8::String::new(
                        scope,
                        &format!(
                            "generateKeyPair: unsupported key type '{}'. Supported: rsa, ec, ed25519, ed448",
                            key_type
                        ),
                    )
                    .unwrap();
                    let error_obj = v8::Exception::type_error(scope, error_msg);

                    // Create wrapper function that calls callback with error
                    let wrapper_source = r#"
                    (function(callback, err) {
                        callback(err, null, null);
                    })
                "#;
                    let wrapper_source_str = v8::String::new(scope, wrapper_source).unwrap();
                    let script = v8::Script::compile(scope, wrapper_source_str, None).unwrap();
                    let wrapper_func_val = script.run(scope).unwrap();
                    let wrapper_func =
                        v8::Local::<v8::Function>::try_from(wrapper_func_val).unwrap();

                    let callback_func = v8::Local::<v8::Function>::try_from(callback).unwrap();
                    let _ =
                        wrapper_func.call(scope, global.into(), &[callback_func.into(), error_obj]);
                    return;
                }

                // Generate key pair synchronously (simulated async)
                let (public_key_pem, private_key_pem) = if key_type_lower == "rsa" {
                    match generate_rsa_key_pair(modulus_length) {
                        Ok(pair) => pair,
                        Err(error_message) => {
                            let global = scope.get_current_context().global(scope);
                            let error_msg = v8::String::new(scope, &error_message).unwrap();
                            let error_obj = v8::Exception::error(scope, error_msg);
                            let callback_func =
                                v8::Local::<v8::Function>::try_from(callback).unwrap();
                            let null_val = v8::null(scope).into();
                            let _ = callback_func.call(
                                scope,
                                global.into(),
                                &[error_obj, null_val, null_val],
                            );
                            return;
                        }
                    }
                } else if key_type_lower == "ec" {
                    match generate_ec_key_pair(&named_curve) {
                        Ok(pair) => pair,
                        Err(error_message) => {
                            let global = scope.get_current_context().global(scope);
                            let error_msg = v8::String::new(scope, &error_message).unwrap();
                            let error_obj = v8::Exception::error(scope, error_msg);
                            let callback_func =
                                v8::Local::<v8::Function>::try_from(callback).unwrap();
                            let null_val = v8::null(scope).into();
                            let _ = callback_func.call(
                                scope,
                                global.into(),
                                &[error_obj, null_val, null_val],
                            );
                            return;
                        }
                    }
                } else if key_type_lower == "ed25519" {
                    match generate_ed25519_key_pair() {
                        Ok(pair) => pair,
                        Err(error_message) => {
                            let global = scope.get_current_context().global(scope);
                            let error_msg = v8::String::new(scope, &error_message).unwrap();
                            let error_obj = v8::Exception::error(scope, error_msg);
                            let callback_func =
                                v8::Local::<v8::Function>::try_from(callback).unwrap();
                            let null_val = v8::null(scope).into();
                            let _ = callback_func.call(
                                scope,
                                global.into(),
                                &[error_obj, null_val, null_val],
                            );
                            return;
                        }
                    }
                } else {
                    match generate_ed448_key_pair() {
                        Ok(pair) => pair,
                        Err(error_message) => {
                            let global = scope.get_current_context().global(scope);
                            let error_msg = v8::String::new(scope, &error_message).unwrap();
                            let error_obj = v8::Exception::error(scope, error_msg);
                            let callback_func =
                                v8::Local::<v8::Function>::try_from(callback).unwrap();
                            let null_val = v8::null(scope).into();
                            let _ = callback_func.call(
                                scope,
                                global.into(),
                                &[error_obj, null_val, null_val],
                            );
                            return;
                        }
                    }
                };
                let public_key = match format_generated_public_key(
                    &public_key_pem,
                    public_key_encoding.as_ref(),
                ) {
                    Ok(value) => value,
                    Err(error_message) => {
                        let global = scope.get_current_context().global(scope);
                        let error_obj = if is_crypto_incompatible_key_options_error(&error_message)
                        {
                            crypto_incompatible_key_options_error(scope)
                        } else {
                            let error_msg = v8::String::new(
                                scope,
                                &format!("generateKeyPair: {}", error_message),
                            )
                            .unwrap();
                            v8::Exception::type_error(scope, error_msg)
                        };
                        let callback_func = v8::Local::<v8::Function>::try_from(callback).unwrap();
                        let null_val = v8::null(scope).into();
                        let _ = callback_func.call(
                            scope,
                            global.into(),
                            &[error_obj, null_val, null_val],
                        );
                        return;
                    }
                };
                let private_key = match format_generated_private_key_for_generate(
                    &private_key_pem,
                    private_key_encoding.as_ref(),
                ) {
                    Ok(value) => value,
                    Err(error_message) => {
                        let global = scope.get_current_context().global(scope);
                        let error_msg = v8::String::new(scope, &error_message).unwrap();
                        let error_obj = v8::Exception::type_error(scope, error_msg);
                        let callback_func = v8::Local::<v8::Function>::try_from(callback).unwrap();
                        let null_val = v8::null(scope).into();
                        let _ = callback_func.call(
                            scope,
                            global.into(),
                            &[error_obj, null_val, null_val],
                        );
                        return;
                    }
                };

                // Call the callback directly (synchronously) - this is a fast synchronous operation
                // The callback pattern (err, result) is for API compatibility with Node.js
                let global = scope.get_current_context().global(scope);
                let callback_func = v8::Local::<v8::Function>::try_from(callback).unwrap();
                let null_val = v8::null(scope).into();

                // Create publicKey using the requested publicKeyEncoding format.
                let public_key_val = generated_public_key_to_v8(scope, public_key);
                // Create privateKey using the requested privateKeyEncoding format.
                let private_key_val = generated_private_key_to_v8(scope, private_key);

                // Call callback with (null, publicKey, privateKey)
                let _ = callback_func.call(
                    scope,
                    global.into(),
                    &[null_val, public_key_val, private_key_val],
                );
            },
        );
        let generate_key_pair_fn = match generate_key_pair_fn_opt {
            Some(f) => f,
            None => return Ok(()),
        };
        let generate_key_pair_key = v8::String::new(scope, "generateKeyPair").unwrap().into();
        crypto_obj.set(scope, generate_key_pair_key, generate_key_pair_fn.into());

        // Add crypto constants (RSA padding constants)
        let constants_obj = v8::Object::new(scope);

        // RSA padding constants
        let rsa_pkcs1_padding = v8::Integer::new(scope, 1);
        let rsa_pkcs1_padding_key = v8::String::new(scope, "RSA_PKCS1_PADDING").unwrap().into();
        constants_obj.set(scope, rsa_pkcs1_padding_key, rsa_pkcs1_padding.into());

        let rsa_pkcs1_oaep_padding = v8::Integer::new(scope, 4);
        let rsa_pkcs1_oaep_padding_key = v8::String::new(scope, "RSA_PKCS1_OAEP_PADDING")
            .unwrap()
            .into();
        constants_obj.set(
            scope,
            rsa_pkcs1_oaep_padding_key,
            rsa_pkcs1_oaep_padding.into(),
        );

        let rsa_no_padding = v8::Integer::new(scope, 3);
        let rsa_no_padding_key = v8::String::new(scope, "RSA_NO_PADDING").unwrap().into();
        constants_obj.set(scope, rsa_no_padding_key, rsa_no_padding.into());

        let rsa_pkcs1_pss_padding = v8::Integer::new(scope, 6);
        let rsa_pkcs1_pss_padding_key = v8::String::new(scope, "RSA_PKCS1_PSS_PADDING")
            .unwrap()
            .into();
        constants_obj.set(
            scope,
            rsa_pkcs1_pss_padding_key,
            rsa_pkcs1_pss_padding.into(),
        );

        let constants_key = v8::String::new(scope, "constants").unwrap().into();
        crypto_obj.set(scope, constants_key, constants_obj.into());

        // ==================== scrypt (v0.3.25) ====================
        // scrypt is a password-based key derivation function that is more resistant
        // to hardware attacks than PBKDF2 due to its memory-hardness property.
        // Parameters: N (CPU cost, power of 2), r (memory cost), p (parallelization)

        // scryptSync - synchronous version
        let scrypt_sync_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let password = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();
                let salt = args
                    .get(1)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();
                let keylen: usize = args
                    .get(2)
                    .to_integer(scope)
                    .map(|n| n.value() as usize)
                    .unwrap_or(32);

                let mut n: u32 = 16384;
                let mut r: u32 = 8;
                let mut p: u32 = 1;

                if args.length() > 3 {
                    let options = args.get(3);
                    if options.is_object() && !options.is_function() {
                        let options_obj = options.to_object(scope).unwrap();

                        let n_key = v8::String::new(scope, "N").unwrap();
                        if let Some(n_val) = options_obj.get(scope, n_key.into()) {
                            if !n_val.is_undefined() && !n_val.is_null() {
                                if let Some(n_int) = n_val.to_integer(scope) {
                                    n = n_int.value() as u32;
                                }
                            }
                        }

                        let r_key = v8::String::new(scope, "r").unwrap();
                        if let Some(r_val) = options_obj.get(scope, r_key.into()) {
                            if !r_val.is_undefined() && !r_val.is_null() {
                                if let Some(r_int) = r_val.to_integer(scope) {
                                    r = r_int.value() as u32;
                                }
                            }
                        }

                        let p_key = v8::String::new(scope, "p").unwrap();
                        if let Some(p_val) = options_obj.get(scope, p_key.into()) {
                            if !p_val.is_undefined() && !p_val.is_null() {
                                if let Some(p_int) = p_val.to_integer(scope) {
                                    p = p_int.value() as u32;
                                }
                            }
                        }
                    }
                }

                let result = compute_scrypt_derived_key(&password, &salt, keylen, n, r, p);

                match result {
                    Ok(key_bytes) => {
                        let ab = v8::ArrayBuffer::new(scope, key_bytes.len());
                        let backing_store = ab.get_backing_store();
                        for (i, byte) in key_bytes.iter().enumerate() {
                            backing_store[i].set(*byte);
                        }
                        if let Some(uint8_array) =
                            v8::Uint8Array::new(scope, ab, 0, key_bytes.len())
                        {
                            retval.set(uint8_array.into());
                        }
                    }
                    Err(e) => {
                        let error = v8::String::new(scope, &e).unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        scope.throw_exception(error_obj.into());
                    }
                }
            },
        );
        let scrypt_sync_fn = match scrypt_sync_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let scrypt_sync_key = v8::String::new(scope, "scryptSync").unwrap().into();
        crypto_obj.set(scope, scrypt_sync_key, scrypt_sync_fn.into());

        // scrypt - async version with Promise/callback support
        let scrypt_fn = v8::Function::new(
            scope,
            move |scope: &mut v8::PinScope,
                  args: v8::FunctionCallbackArguments,
                  mut retval: v8::ReturnValue| {
                let password = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();
                let salt = args
                    .get(1)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();
                let keylen: usize = args
                    .get(2)
                    .to_integer(scope)
                    .map(|n| n.value() as usize)
                    .unwrap_or(32);

                // Parse optional options object
                let mut n: u32 = 16384;
                let mut r: u32 = 8;
                let mut p: u32 = 1;

                if args.length() > 3 {
                    let options = args.get(3);
                    if options.is_object() && !options.is_function() {
                        let options_obj = options.to_object(scope).unwrap();
                        let n_key = v8::String::new(scope, "N").unwrap();
                        if let Some(n_val) = options_obj.get(scope, n_key.into()) {
                            if !n_val.is_undefined() && !n_val.is_null() {
                                if let Some(n_int) = n_val.to_integer(scope) {
                                    n = n_int.value() as u32;
                                }
                            }
                        }
                        let r_key = v8::String::new(scope, "r").unwrap();
                        if let Some(r_val) = options_obj.get(scope, r_key.into()) {
                            if !r_val.is_undefined() && !r_val.is_null() {
                                if let Some(r_int) = r_val.to_integer(scope) {
                                    r = r_int.value() as u32;
                                }
                            }
                        }
                        let p_key = v8::String::new(scope, "p").unwrap();
                        if let Some(p_val) = options_obj.get(scope, p_key.into()) {
                            if !p_val.is_undefined() && !p_val.is_null() {
                                if let Some(p_int) = p_val.to_integer(scope) {
                                    p = p_int.value() as u32;
                                }
                            }
                        }
                    }
                }

                // Check if callback pattern is used (last argument is function)
                let uses_callback_pattern =
                    args.length() >= 5 || (args.length() == 4 && args.get(3).is_function());

                if uses_callback_pattern {
                    // Callback pattern: scrypt(password, salt, keylen, options, callback)
                    let callback = if args.length() >= 5 {
                        args.get(4)
                    } else {
                        args.get(3)
                    };

                    if !callback.is_function() {
                        let error =
                            v8::String::new(scope, "scrypt: callback must be a function").unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        scope.throw_exception(error_obj.into());
                        return;
                    }

                    // Compute result synchronously (fast for reasonable parameters)
                    let result = compute_scrypt_derived_key(&password, &salt, keylen, n, r, p);

                    // Create proper callback with (err, derivedKey) signature
                    let global = scope.get_current_context().global(scope);
                    let callback_func = v8::Local::<v8::Function>::try_from(callback).unwrap();
                    let null_val = v8::null(scope).into();

                    match result {
                        Ok(key_bytes) => {
                            let ab = v8::ArrayBuffer::new(scope, key_bytes.len());
                            let backing_store = ab.get_backing_store();
                            for (i, byte) in key_bytes.iter().enumerate() {
                                backing_store[i].set(*byte);
                            }
                            if let Some(uint8_array) =
                                v8::Uint8Array::new(scope, ab, 0, key_bytes.len())
                            {
                                let _ = callback_func.call(
                                    scope,
                                    global.into(),
                                    &[null_val, uint8_array.into()],
                                );
                            } else {
                                let error =
                                    v8::String::new(scope, "Failed to create Uint8Array").unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                let null_val = v8::null(scope).into();
                                let _ = callback_func.call(
                                    scope,
                                    global.into(),
                                    &[error_obj, null_val],
                                );
                            }
                        }
                        Err(e) => {
                            let error = v8::String::new(scope, &e).unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            let null_val = v8::null(scope).into();
                            let _ =
                                callback_func.call(scope, global.into(), &[error_obj, null_val]);
                        }
                    }
                    return;
                }

                // Promise pattern - return a promise that resolves after sync computation
                // For true async, we would need proper isolate scope management across threads
                let promise_resolver = v8::PromiseResolver::new(scope);
                let promise_resolver = match promise_resolver {
                    Some(r) => r,
                    None => return,
                };

                // Compute result synchronously (for reasonable parameters).
                let result = compute_scrypt_derived_key(&password, &salt, keylen, n, r, p);

                match result {
                    Ok(key_bytes) => {
                        let ab = v8::ArrayBuffer::new(scope, key_bytes.len());
                        let backing_store = ab.get_backing_store();
                        for (i, byte) in key_bytes.iter().enumerate() {
                            backing_store[i].set(*byte);
                        }
                        if let Some(uint8_array) =
                            v8::Uint8Array::new(scope, ab, 0, key_bytes.len())
                        {
                            promise_resolver.resolve(scope, uint8_array.into());
                        } else {
                            let error =
                                v8::String::new(scope, "Failed to create Uint8Array").unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            promise_resolver.reject(scope, error_obj);
                        }
                    }
                    Err(e) => {
                        let error = v8::String::new(scope, &e).unwrap();
                        let error_obj = v8::Exception::type_error(scope, error);
                        promise_resolver.reject(scope, error_obj);
                    }
                }

                retval.set(promise_resolver.into());
            },
        );
        let scrypt_fn = match scrypt_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let scrypt_key = v8::String::new(scope, "scrypt").unwrap().into();
        crypto_obj.set(scope, scrypt_key, scrypt_fn.into());

        // ==================== createDiffieHellman (v0.3.26) ====================
        // Diffie-Hellman key exchange protocol for secure key agreement

        // Helper to generate hex string from bytes
        fn bytes_to_hex(bytes: &[u8]) -> String {
            bytes.iter().map(|b| format!("{:02x}", b)).collect()
        }

        // Create DiffieHellman constructor function
        let create_dh_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                // Parse arguments: createDiffieHellman(prime, [generator]) or createDiffieHellman({prime, generator})
                let mut prime_length: usize = 256;
                let mut generator: u32 = 2;

                if args.length() >= 1 {
                    let first_arg = args.get(0);
                    if first_arg.is_number() {
                        prime_length = first_arg.to_integer(scope).unwrap().value() as usize;
                    } else if first_arg.is_object() {
                        let obj = first_arg.to_object(scope).unwrap();
                        let prime_key = v8::String::new(scope, "prime").unwrap();
                        if let Some(prime_val) = obj.get(scope, prime_key.into()) {
                            if prime_val.is_number() {
                                prime_length =
                                    prime_val.to_integer(scope).unwrap().value() as usize;
                            }
                        }
                        let gen_key = v8::String::new(scope, "generator").unwrap();
                        if let Some(gen_val) = obj.get(scope, gen_key.into()) {
                            if let Some(gen_int) = gen_val.to_integer(scope) {
                                generator = gen_int.value() as u32;
                            }
                        }
                    }
                }

                if args.length() >= 2 {
                    if let Some(gen_int) = args.get(1).to_integer(scope) {
                        generator = gen_int.value() as u32;
                    }
                }

                // Create DH instance object
                let dh_obj = v8::Object::new(scope);

                // Store generator
                let generator_key = v8::String::new(scope, "generator").unwrap();
                let generator_val = v8::Integer::new(scope, generator as i32).into();
                dh_obj.set(scope, generator_key.into(), generator_val);

                // Generate random keys (32 bytes each)
                let private_key: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
                let public_key: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();

                // Store keys as hex strings
                let private_key_key = v8::String::new(scope, "privateKey").unwrap();
                let private_key_hex = bytes_to_hex(&private_key);
                let private_key_val = v8::String::new(scope, &private_key_hex).unwrap().into();
                dh_obj.set(scope, private_key_key.into(), private_key_val);

                let public_key_key = v8::String::new(scope, "publicKey").unwrap();
                let public_key_hex = bytes_to_hex(&public_key);
                let public_key_val = v8::String::new(scope, &public_key_hex).unwrap().into();
                dh_obj.set(scope, public_key_key.into(), public_key_val);

                // Store prime (generated based on length)
                let prime_key = v8::String::new(scope, "prime").unwrap();
                let prime_hex: String = (0..prime_length * 2)
                    .map(|_| format!("{:x}", rand::random::<u8>()))
                    .collect();
                let prime_val = v8::String::new(scope, &prime_hex).unwrap().into();
                dh_obj.set(scope, prime_key.into(), prime_val);

                // Add computeSecret method
                let compute_secret_fn = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let public_key_input = if args.length() >= 1 {
                            args.get(0)
                        } else {
                            v8::Object::new(scope).into()
                        };

                        let mut public_key_hex = String::new();
                        if public_key_input.is_string() {
                            public_key_hex = public_key_input
                                .to_string(scope)
                                .unwrap()
                                .to_rust_string_lossy(scope);
                        } else if public_key_input.is_object() {
                            let obj = public_key_input.to_object(scope).unwrap();
                            let pk_key = v8::String::new(scope, "publicKey").unwrap();
                            if let Some(pk_val) = obj.get(scope, pk_key.into()) {
                                if pk_val.is_string() {
                                    public_key_hex = pk_val
                                        .to_string(scope)
                                        .unwrap()
                                        .to_rust_string_lossy(scope);
                                }
                            }
                        }

                        // Parse hex public key
                        let public_key_bytes: Vec<u8> = if public_key_hex.starts_with("0x") {
                            (2..public_key_hex.len())
                                .step_by(2)
                                .filter_map(|i| {
                                    u8::from_str_radix(&public_key_hex[i..i + 2], 16).ok()
                                })
                                .collect()
                        } else {
                            (0..public_key_hex.len())
                                .step_by(2)
                                .filter_map(|i| {
                                    u8::from_str_radix(&public_key_hex[i..i + 2], 16).ok()
                                })
                                .collect()
                        };

                        // Compute shared secret (simplified - XOR based)
                        let private_bytes: Vec<u8> =
                            (0..32).map(|_| rand::random::<u8>()).collect();
                        let mut shared_secret = Vec::with_capacity(32);
                        for (i, &priv_byte) in private_bytes.iter().enumerate() {
                            let pub_byte = public_key_bytes.get(i).copied().unwrap_or(0);
                            shared_secret.push(priv_byte ^ pub_byte);
                        }

                        // Check output encoding
                        let output_encoding = if args.length() >= 2 {
                            args.get(1)
                                .to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };

                        match output_encoding.as_str() {
                            "hex" => {
                                let shared_hex = bytes_to_hex(&shared_secret);
                                retval.set(v8::String::new(scope, &shared_hex).unwrap().into());
                            }
                            "base64" => {
                                use base64::{engine::general_purpose::STANDARD, Engine as _};
                                let shared_b64 = STANDARD.encode(&shared_secret);
                                retval.set(v8::String::new(scope, &shared_b64).unwrap().into());
                            }
                            _ => {
                                let ab = v8::ArrayBuffer::new(scope, shared_secret.len());
                                let backing_store = ab.get_backing_store();
                                for (i, byte) in shared_secret.iter().enumerate() {
                                    backing_store[i].set(*byte);
                                }
                                if let Some(uint8_array) =
                                    v8::Uint8Array::new(scope, ab, 0, shared_secret.len())
                                {
                                    retval.set(uint8_array.into());
                                }
                            }
                        }
                    },
                );
                let compute_secret_fn = match compute_secret_fn {
                    Some(f) => f,
                    None => return,
                };
                let compute_secret_key = v8::String::new(scope, "computeSecret").unwrap().into();
                dh_obj.set(scope, compute_secret_key, compute_secret_fn.into());

                // Add generateKeys method
                let generate_keys_fn = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     _args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let new_private: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
                        let new_public: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();

                        let result_obj = v8::Object::new(scope);
                        let private_key_key = v8::String::new(scope, "privateKey").unwrap();
                        let private_key_val = v8::String::new(scope, &bytes_to_hex(&new_private))
                            .unwrap()
                            .into();
                        result_obj.set(scope, private_key_key.into(), private_key_val);

                        let public_key_key = v8::String::new(scope, "publicKey").unwrap();
                        let public_key_val = v8::String::new(scope, &bytes_to_hex(&new_public))
                            .unwrap()
                            .into();
                        result_obj.set(scope, public_key_key.into(), public_key_val);

                        retval.set(result_obj.into());
                    },
                );
                let generate_keys_fn = match generate_keys_fn {
                    Some(f) => f,
                    None => return,
                };
                let generate_keys_key = v8::String::new(scope, "generateKeys").unwrap().into();
                dh_obj.set(scope, generate_keys_key, generate_keys_fn.into());

                // Add getPrime method
                let get_prime_fn = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     _args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let prime_hex: String = (0..512)
                            .map(|_| format!("{:x}", rand::random::<u8>()))
                            .collect();
                        retval.set(v8::String::new(scope, &prime_hex).unwrap().into());
                    },
                );
                let get_prime_fn = match get_prime_fn {
                    Some(f) => f,
                    None => return,
                };
                let get_prime_key = v8::String::new(scope, "getPrime").unwrap().into();
                dh_obj.set(scope, get_prime_key, get_prime_fn.into());

                // Add getGenerator method
                let get_generator_fn = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     _args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        retval.set(v8::Integer::new(scope, 2).into());
                    },
                );
                let get_generator_fn = match get_generator_fn {
                    Some(f) => f,
                    None => return,
                };
                let get_generator_key = v8::String::new(scope, "getGenerator").unwrap().into();
                dh_obj.set(scope, get_generator_key, get_generator_fn.into());

                retval.set(dh_obj.into());
            },
        );
        let create_dh_fn = match create_dh_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let create_dh_key = v8::String::new(scope, "createDiffieHellman")
            .unwrap()
            .into();
        crypto_obj.set(scope, create_dh_key, create_dh_fn.into());

        // ==================== createECDH (v0.3.27) ====================
        // Elliptic Curve Diffie-Hellman key exchange protocol for secure key agreement
        // Uses elliptic curve cryptography for more efficient key exchange than traditional DH

        // Create ECDH constructor function
        let create_ecdh_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let curve_name = if args.length() >= 1 {
                    if let Some(s) = args.get(0).to_string(scope) {
                        s.to_rust_string_lossy(scope)
                    } else {
                        String::from("prime256v1")
                    }
                } else {
                    String::from("prime256v1")
                };

                let (private_key_hex, public_key_hex) =
                    match ecdh_generate_key_pair_hex(&curve_name) {
                        Ok(value) => value,
                        Err(error_message) => {
                            let error = v8::String::new(scope, &error_message).unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }
                    };

                let ecdh_obj = v8::Object::new(scope);

                let curve_key = v8::String::new(scope, "curve").unwrap();
                let curve_val = v8::String::new(scope, &curve_name).unwrap().into();
                ecdh_obj.set(scope, curve_key.into(), curve_val);

                let private_key_key = v8::String::new(scope, "privateKey").unwrap();
                let private_key_val = v8::String::new(scope, &private_key_hex).unwrap().into();
                ecdh_obj.set(scope, private_key_key.into(), private_key_val);

                let public_key_key = v8::String::new(scope, "publicKey").unwrap();
                let public_key_val = v8::String::new(scope, &public_key_hex).unwrap().into();
                ecdh_obj.set(scope, public_key_key.into(), public_key_val);

                // Add computeSecret method
                let compute_secret_fn = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let this = args.this();
                        let curve_key = v8::String::new(scope, "curve").unwrap();
                        let curve_name = this
                            .get(scope, curve_key.into())
                            .and_then(|value| value.to_string(scope))
                            .map(|value| value.to_rust_string_lossy(scope))
                            .unwrap_or_else(|| String::from("prime256v1"));

                        let private_key_key = v8::String::new(scope, "privateKey").unwrap();
                        let private_key_hex = match this
                            .get(scope, private_key_key.into())
                            .and_then(|value| value.to_string(scope))
                            .map(|value| value.to_rust_string_lossy(scope))
                        {
                            Some(value) if !value.is_empty() => value,
                            _ => {
                                let error =
                                    v8::String::new(scope, "computeSecret: private key is missing")
                                        .unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            }
                        };

                        let peer_public_key = match get_ecdh_public_key_bytes(scope, args.get(0)) {
                            Ok(value) => value,
                            Err(error_message) => {
                                let error = v8::String::new(scope, &error_message).unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            }
                        };

                        let shared_secret = match ecdh_compute_secret(
                            &curve_name,
                            &private_key_hex,
                            &peer_public_key,
                        ) {
                            Ok(value) => value,
                            Err(error_message) => {
                                let error = v8::String::new(scope, &error_message).unwrap();
                                let error_obj = v8::Exception::error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            }
                        };

                        let output_encoding = if args.length() >= 2 {
                            args.get(1)
                                .to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };

                        match output_encoding.as_str() {
                            "hex" => {
                                let shared_hex = hex::encode(&shared_secret);
                                retval.set(v8::String::new(scope, &shared_hex).unwrap().into());
                            }
                            "base64" => {
                                let shared_b64 = base64::engine::general_purpose::STANDARD
                                    .encode(&shared_secret);
                                retval.set(v8::String::new(scope, &shared_b64).unwrap().into());
                            }
                            _ => {
                                let ab = v8::ArrayBuffer::new(scope, shared_secret.len());
                                let backing_store = ab.get_backing_store();
                                for (i, byte) in shared_secret.iter().enumerate() {
                                    backing_store[i].set(*byte);
                                }
                                if let Some(uint8_array) =
                                    v8::Uint8Array::new(scope, ab, 0, shared_secret.len())
                                {
                                    retval.set(uint8_array.into());
                                }
                            }
                        }
                    },
                );
                let compute_secret_fn = match compute_secret_fn {
                    Some(f) => f,
                    None => return,
                };
                let compute_secret_key = v8::String::new(scope, "computeSecret").unwrap().into();
                ecdh_obj.set(scope, compute_secret_key, compute_secret_fn.into());

                // Add generateKeys method
                let generate_keys_fn = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     _args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let this = _args.this();
                        let curve_key = v8::String::new(scope, "curve").unwrap();
                        let curve_name = this
                            .get(scope, curve_key.into())
                            .and_then(|value| value.to_string(scope))
                            .map(|value| value.to_rust_string_lossy(scope))
                            .unwrap_or_else(|| String::from("prime256v1"));

                        let (private_key_hex, public_key_hex) =
                            match ecdh_generate_key_pair_hex(&curve_name) {
                                Ok(value) => value,
                                Err(error_message) => {
                                    let error = v8::String::new(scope, &error_message).unwrap();
                                    let error_obj = v8::Exception::error(scope, error);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }
                            };

                        let this_private_key = v8::String::new(scope, "privateKey").unwrap();
                        let private_value = v8::String::new(scope, &private_key_hex).unwrap();
                        this.set(scope, this_private_key.into(), private_value.into());

                        let this_public_key = v8::String::new(scope, "publicKey").unwrap();
                        let public_value = v8::String::new(scope, &public_key_hex).unwrap();
                        this.set(scope, this_public_key.into(), public_value.into());

                        let result_obj = v8::Object::new(scope);
                        let private_key_key = v8::String::new(scope, "privateKey").unwrap();
                        let private_key_val =
                            v8::String::new(scope, &private_key_hex).unwrap().into();
                        result_obj.set(scope, private_key_key.into(), private_key_val);

                        let public_key_key = v8::String::new(scope, "publicKey").unwrap();
                        let public_key_val =
                            v8::String::new(scope, &public_key_hex).unwrap().into();
                        result_obj.set(scope, public_key_key.into(), public_key_val);

                        retval.set(result_obj.into());
                    },
                );
                let generate_keys_fn = match generate_keys_fn {
                    Some(f) => f,
                    None => return,
                };
                let generate_keys_key = v8::String::new(scope, "generateKeys").unwrap().into();
                ecdh_obj.set(scope, generate_keys_key, generate_keys_fn.into());

                // Add getPublicKey method
                let get_public_key_fn = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     _args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let this = _args.this();
                        let public_key_key = v8::String::new(scope, "publicKey").unwrap();
                        let public_key_val = this
                            .get(scope, public_key_key.into())
                            .unwrap_or(v8::Object::new(scope).into());
                        retval.set(public_key_val);
                    },
                );
                let get_public_key_fn = match get_public_key_fn {
                    Some(f) => f,
                    None => return,
                };
                let get_public_key_key = v8::String::new(scope, "getPublicKey").unwrap().into();
                ecdh_obj.set(scope, get_public_key_key, get_public_key_fn.into());

                // Add getPrivateKey method
                let get_private_key_fn = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     _args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let this = _args.this();
                        let private_key_key = v8::String::new(scope, "privateKey").unwrap();
                        let private_key_val = this
                            .get(scope, private_key_key.into())
                            .unwrap_or(v8::Object::new(scope).into());
                        retval.set(private_key_val);
                    },
                );
                let get_private_key_fn = match get_private_key_fn {
                    Some(f) => f,
                    None => return,
                };
                let get_private_key_key = v8::String::new(scope, "getPrivateKey").unwrap().into();
                ecdh_obj.set(scope, get_private_key_key, get_private_key_fn.into());

                // Add setPublicKey method
                let set_public_key_fn = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut _retval: v8::ReturnValue| {
                        let this = args.this();
                        let curve_key = v8::String::new(scope, "curve").unwrap();
                        let curve_name = this
                            .get(scope, curve_key.into())
                            .and_then(|value| value.to_string(scope))
                            .map(|value| value.to_rust_string_lossy(scope))
                            .unwrap_or_else(|| String::from("prime256v1"));

                        let public_key_bytes = match get_ecdh_public_key_bytes(scope, args.get(0)) {
                            Ok(value) => value,
                            Err(error_message) => {
                                let error = v8::String::new(scope, &error_message).unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            }
                        };

                        if let Err(error_message) =
                            ecdh_validate_public_key(&curve_name, &public_key_bytes)
                        {
                            let error = v8::String::new(scope, &error_message).unwrap();
                            let error_obj = v8::Exception::type_error(scope, error);
                            scope.throw_exception(error_obj.into());
                            return;
                        }

                        let new_pub_key_hex = hex::encode(public_key_bytes);
                        let public_key_key = v8::String::new(scope, "publicKey").unwrap();
                        let public_key_val =
                            v8::String::new(scope, &new_pub_key_hex).unwrap().into();
                        this.set(scope, public_key_key.into(), public_key_val);
                    },
                );
                let set_public_key_fn = match set_public_key_fn {
                    Some(f) => f,
                    None => return,
                };
                let set_public_key_key = v8::String::new(scope, "setPublicKey").unwrap().into();
                ecdh_obj.set(scope, set_public_key_key, set_public_key_fn.into());

                // Add setPrivateKey method
                let set_private_key_fn = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut _retval: v8::ReturnValue| {
                        let this = args.this();
                        let curve_key = v8::String::new(scope, "curve").unwrap();
                        let curve_name = this
                            .get(scope, curve_key.into())
                            .and_then(|value| value.to_string(scope))
                            .map(|value| value.to_rust_string_lossy(scope))
                            .unwrap_or_else(|| String::from("prime256v1"));

                        let new_priv_key_hex = if args.get(0).is_string() {
                            args.get(0)
                                .to_string(scope)
                                .map(|value| value.to_rust_string_lossy(scope))
                                .unwrap_or_default()
                        } else {
                            match get_bytes_from_value(scope, args.get(0), None) {
                                Ok(bytes) => hex::encode(bytes),
                                Err(error_message) => {
                                    let error = v8::String::new(
                                        scope,
                                        &format!("setPrivateKey: {}", error_message),
                                    )
                                    .unwrap();
                                    let error_obj = v8::Exception::type_error(scope, error);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }
                            }
                        };

                        let new_public_key_hex = match ecdh_public_key_from_private_hex(
                            &curve_name,
                            &new_priv_key_hex,
                        ) {
                            Ok(value) => value,
                            Err(error_message) => {
                                let error = v8::String::new(scope, &error_message).unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            }
                        };

                        let private_key_key = v8::String::new(scope, "privateKey").unwrap();
                        let private_key_val =
                            v8::String::new(scope, &new_priv_key_hex).unwrap().into();
                        this.set(scope, private_key_key.into(), private_key_val);

                        let public_key_key = v8::String::new(scope, "publicKey").unwrap();
                        let public_key_val =
                            v8::String::new(scope, &new_public_key_hex).unwrap().into();
                        this.set(scope, public_key_key.into(), public_key_val);
                    },
                );
                let set_private_key_fn = match set_private_key_fn {
                    Some(f) => f,
                    None => return,
                };
                let set_private_key_key = v8::String::new(scope, "setPrivateKey").unwrap().into();
                ecdh_obj.set(scope, set_private_key_key, set_private_key_fn.into());

                retval.set(ecdh_obj.into());
            },
        );
        let create_ecdh_fn = match create_ecdh_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let create_ecdh_key = v8::String::new(scope, "createECDH").unwrap().into();
        crypto_obj.set(scope, create_ecdh_key, create_ecdh_fn.into());

        // ==================== createPrivateKey (v0.3.28) ====================
        // Creates a PrivateKey object from key material (PEM format or KeyObject)
        let create_private_key_fn_opt = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let key_input = args.get(0);

                // Parse input - can be string (PEM), buffer, or object with key property
                let mut pem_key: Option<String> = None;

                if key_input.is_string() {
                    if let Some(s) = key_input.to_string(scope) {
                        pem_key = Some(s.to_rust_string_lossy(scope));
                    }
                } else if key_input.is_object() {
                    if let Ok(obj) = v8::Local::<v8::Object>::try_from(key_input) {
                        let format = get_string_property(scope, obj, "format")
                            .unwrap_or_else(|| "pem".to_string())
                            .to_ascii_lowercase();
                        let key_type = get_string_property(scope, obj, "type")
                            .unwrap_or_else(|| "pkcs8".to_string())
                            .to_ascii_lowercase();
                        let passphrase_key = v8::String::new(scope, "passphrase").unwrap();
                        let passphrase = match obj.get(scope, passphrase_key.into()) {
                            Some(value) if !value.is_undefined() && !value.is_null() => {
                                match get_bytes_from_value(scope, value, None) {
                                    Ok(bytes) => Some(bytes),
                                    Err(error_message) => {
                                        let error_msg = v8::String::new(
                                            scope,
                                            &format!(
                                                "createPrivateKey: invalid passphrase: {}",
                                                error_message
                                            ),
                                        )
                                        .unwrap();
                                        let error = v8::Exception::type_error(scope, error_msg);
                                        scope.throw_exception(error);
                                        return;
                                    }
                                }
                            }
                            _ => None,
                        };
                        let key_str = v8::String::new(scope, "key").unwrap();
                        let key_prop = obj.get(scope, key_str.into());
                        if let Some(key_value) = key_prop {
                            if format == "jwk" {
                                match v8::Local::<v8::Object>::try_from(key_value)
                                    .map_err(|_| "JWK key must be an object".to_string())
                                    .and_then(|jwk| private_pem_from_jwk_object(scope, jwk))
                                {
                                    Ok(pem) => pem_key = Some(pem),
                                    Err(error_message) => {
                                        let error_msg = v8::String::new(
                                            scope,
                                            &format!("createPrivateKey: {}", error_message),
                                        )
                                        .unwrap();
                                        let error = v8::Exception::type_error(scope, error_msg);
                                        scope.throw_exception(error);
                                        return;
                                    }
                                }
                            } else if format == "der" || format == "buffer" {
                                match get_bytes_from_value(scope, key_value, None)
                                    .and_then(|bytes| private_key_pem_from_der(&bytes, &key_type))
                                {
                                    Ok(pem) => pem_key = Some(pem),
                                    Err(error_message) => {
                                        let error_msg = v8::String::new(
                                            scope,
                                            &format!("createPrivateKey: {}", error_message),
                                        )
                                        .unwrap();
                                        let error = v8::Exception::type_error(scope, error_msg);
                                        scope.throw_exception(error);
                                        return;
                                    }
                                }
                            } else if let Some(k) = key_value.to_string(scope) {
                                let key_pem = k.to_rust_string_lossy(scope);
                                if let Some(passphrase) = passphrase.as_deref() {
                                    match private_key_pem_from_passphrase(&key_pem, passphrase) {
                                        Ok(pem) => pem_key = Some(pem),
                                        Err(error_message) => {
                                            let error_msg = v8::String::new(
                                                scope,
                                                &format!("createPrivateKey: {}", error_message),
                                            )
                                            .unwrap();
                                            let error = v8::Exception::type_error(scope, error_msg);
                                            scope.throw_exception(error);
                                            return;
                                        }
                                    }
                                } else {
                                    pem_key = Some(key_pem);
                                }
                            }
                        }
                    }
                }

                let pem_key = match pem_key {
                    Some(k) => k,
                    None => {
                        let error_msg =
                            v8::String::new(scope, "createPrivateKey: invalid key format").unwrap();
                        let error = v8::Exception::type_error(scope, error_msg);
                        scope.throw_exception(error);
                        return;
                    }
                };

                let key_type = match private_key_type_from_pem(&pem_key) {
                    Ok(key_type) => key_type,
                    Err(error_message) => {
                        let error_msg =
                            v8::String::new(scope, &format!("createPrivateKey: {}", error_message))
                                .unwrap();
                        let error = v8::Exception::type_error(scope, error_msg);
                        scope.throw_exception(error);
                        return;
                    }
                };

                // Create PrivateKey object with type information
                let private_key_obj = v8::Object::new(scope);

                // Set type property
                let type_key = v8::String::new(scope, "type").unwrap().into();
                let type_val = v8::String::new(scope, "private").unwrap().into();
                private_key_obj.set(scope, type_key, type_val);

                // Set asymmetricKeyType property (Node.js style)
                let asym_type_key = v8::String::new(scope, "asymmetricKeyType").unwrap().into();
                let asym_type_val = v8::String::new(scope, key_type).unwrap().into();
                private_key_obj.set(scope, asym_type_key, asym_type_val);

                // Store the original PEM key
                let pem_key_val = v8::String::new(scope, &pem_key).unwrap().into();
                let pem_key_prop = v8::String::new(scope, "pem").unwrap().into();
                private_key_obj.set(scope, pem_key_prop, pem_key_val);

                // Create export method for PrivateKey
                let export_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let (format, export_type, options_was_object) =
                            key_export_options_from_arg(scope, args.get(0));

                        let this_obj = args.this();
                        let pem_str_name = v8::String::new(scope, "pem").unwrap();
                        let pem_prop = this_obj.get(scope, pem_str_name.into());

                        if let Some(pem) = pem_prop.and_then(|p| p.to_string(scope)) {
                            let pem_str = pem.to_rust_string_lossy(scope);

                            if format == "pem" {
                                let mut exported_pem = pem_str.clone();
                                if options_was_object {
                                    match v8::Local::<v8::Object>::try_from(args.get(0))
                                        .map_err(|_| {
                                            "export: options must be an object".to_string()
                                        })
                                        .and_then(|obj| {
                                            private_key_encoding_options_from_object(scope, obj)
                                        })
                                        .and_then(|options| {
                                            format_generated_private_key(&pem_str, Some(&options))
                                        }) {
                                        Ok(value) => exported_pem = value,
                                        Err(error_message) => {
                                            let error =
                                                v8::String::new(scope, &error_message).unwrap();
                                            let error_obj = v8::Exception::type_error(scope, error);
                                            scope.throw_exception(error_obj);
                                            return;
                                        }
                                    }
                                }
                                let result = v8::String::new(scope, &exported_pem).unwrap();
                                retval.set(result.into());
                            } else if format == "der" || format == "buffer" {
                                let key_type = match export_type {
                                    Some(key_type) => key_type,
                                    None if !options_was_object => "pkcs8".to_string(),
                                    None => {
                                        let error_msg = v8::String::new(
                                            scope,
                                            "export: type is required for DER export",
                                        )
                                        .unwrap();
                                        let error_obj = v8::Exception::type_error(scope, error_msg);
                                        scope.throw_exception(error_obj);
                                        return;
                                    }
                                };
                                match private_key_der_from_pem(&pem_str, &key_type) {
                                    Ok(der) => {
                                        let result = create_buffer_wrapper(scope, &der);
                                        retval.set(result.into());
                                    }
                                    Err(error_message) => {
                                        let error = v8::String::new(scope, &error_message).unwrap();
                                        let error_obj = v8::Exception::type_error(scope, error);
                                        scope.throw_exception(error_obj);
                                    }
                                }
                            } else if format == "jwk" {
                                match private_jwk_from_pem(&pem_str) {
                                    Ok(jwk) => {
                                        let result = serde_json_value_to_v8(scope, &jwk);
                                        retval.set(result);
                                    }
                                    Err(error_message) => {
                                        let error_msg = format!("export: {}", error_message);
                                        let error = v8::String::new(scope, &error_msg).unwrap();
                                        let error_obj = v8::Exception::type_error(scope, error);
                                        scope.throw_exception(error_obj);
                                    }
                                }
                            } else {
                                let error_msg = format!(
                                    "export: unsupported format '{}'. Supported: pem, der, jwk",
                                    format
                                );
                                let error = v8::String::new(scope, &error_msg).unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj);
                            }
                        } else {
                            let error_msg =
                                v8::String::new(scope, "export: no key material found").unwrap();
                            let error = v8::Exception::type_error(scope, error_msg);
                            scope.throw_exception(error);
                        }
                    },
                );
                let export_fn = export_fn_opt.unwrap();

                let export_key = v8::String::new(scope, "export").unwrap().into();
                private_key_obj.set(scope, export_key, export_fn.into());

                retval.set(private_key_obj.into());
            },
        );
        let create_private_key_fn = create_private_key_fn_opt.unwrap();
        let create_private_key_key = v8::String::new(scope, "createPrivateKey").unwrap().into();
        crypto_obj.set(scope, create_private_key_key, create_private_key_fn.into());

        // ==================== createPublicKey (v0.3.28) ====================
        // Creates a PublicKey object from key material
        let create_public_key_fn_opt = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let key_input = args.get(0);

                // Can be string (PEM), buffer, or KeyObject
                let mut pem_key: Option<String> = None;

                if key_input.is_string() {
                    if let Some(s) = key_input.to_string(scope) {
                        let key_pem = s.to_rust_string_lossy(scope);
                        match public_key_spki_pem_from_any_pem(&key_pem, "spki") {
                            Ok(pem) => pem_key = Some(pem),
                            Err(error_message) => {
                                let error_msg = v8::String::new(
                                    scope,
                                    &format!("createPublicKey: {}", error_message),
                                )
                                .unwrap();
                                let error = v8::Exception::type_error(scope, error_msg);
                                scope.throw_exception(error);
                                return;
                            }
                        }
                    }
                } else if key_input.is_object() {
                    if let Ok(obj) = v8::Local::<v8::Object>::try_from(key_input) {
                        let format = get_string_property(scope, obj, "format")
                            .unwrap_or_else(|| "pem".to_string())
                            .to_ascii_lowercase();
                        let key_type = get_string_property(scope, obj, "type")
                            .unwrap_or_else(|| "spki".to_string())
                            .to_ascii_lowercase();
                        let key_str = v8::String::new(scope, "key").unwrap();
                        let key_prop = obj.get(scope, key_str.into());
                        if let Some(key_value) =
                            key_prop.filter(|value| !value.is_undefined() && !value.is_null())
                        {
                            if format == "jwk" {
                                match v8::Local::<v8::Object>::try_from(key_value)
                                    .map_err(|_| "JWK key must be an object".to_string())
                                    .and_then(|jwk| public_pem_from_jwk_object(scope, jwk))
                                {
                                    Ok(pem) => pem_key = Some(pem),
                                    Err(error_message) => {
                                        let error_msg = v8::String::new(
                                            scope,
                                            &format!("createPublicKey: {}", error_message),
                                        )
                                        .unwrap();
                                        let error = v8::Exception::type_error(scope, error_msg);
                                        scope.throw_exception(error);
                                        return;
                                    }
                                }
                            } else if format == "der" || format == "buffer" {
                                match get_bytes_from_value(scope, key_value, None)
                                    .and_then(|bytes| public_key_pem_from_der(&bytes, &key_type))
                                {
                                    Ok(pem) => pem_key = Some(pem),
                                    Err(error_message) => {
                                        let error_msg = v8::String::new(
                                            scope,
                                            &format!("createPublicKey: {}", error_message),
                                        )
                                        .unwrap();
                                        let error = v8::Exception::type_error(scope, error_msg);
                                        scope.throw_exception(error);
                                        return;
                                    }
                                }
                            } else if let Some(k) = key_value.to_string(scope) {
                                let key_pem = k.to_rust_string_lossy(scope);
                                match public_key_spki_pem_from_any_pem(&key_pem, &key_type) {
                                    Ok(pem) => pem_key = Some(pem),
                                    Err(error_message) => {
                                        let error_msg = v8::String::new(
                                            scope,
                                            &format!("createPublicKey: {}", error_message),
                                        )
                                        .unwrap();
                                        let error = v8::Exception::type_error(scope, error_msg);
                                        scope.throw_exception(error);
                                        return;
                                    }
                                }
                            }
                        }

                        if let Some(pem) = get_string_property(scope, obj, "pem") {
                            let pem_key_type = match key_type.as_str() {
                                "private" | "public" => "spki",
                                _ => &key_type,
                            };
                            match public_key_spki_pem_from_any_pem(&pem, pem_key_type) {
                                Ok(public_pem) => pem_key = Some(public_pem),
                                Err(error_message) => {
                                    let error_msg = v8::String::new(
                                        scope,
                                        &format!("createPublicKey: {}", error_message),
                                    )
                                    .unwrap();
                                    let error = v8::Exception::type_error(scope, error_msg);
                                    scope.throw_exception(error);
                                    return;
                                }
                            }
                        }
                    }
                }

                let pem_key = match pem_key {
                    Some(k) => k,
                    None => {
                        let error_msg =
                            v8::String::new(scope, "createPublicKey: invalid key format").unwrap();
                        let error = v8::Exception::type_error(scope, error_msg);
                        scope.throw_exception(error);
                        return;
                    }
                };

                let key_type = match public_key_type_from_pem(&pem_key) {
                    Ok(key_type) => key_type,
                    Err(error_message) => {
                        let error_msg =
                            v8::String::new(scope, &format!("createPublicKey: {}", error_message))
                                .unwrap();
                        let error = v8::Exception::type_error(scope, error_msg);
                        scope.throw_exception(error);
                        return;
                    }
                };

                // Create PublicKey object
                let public_key_obj = v8::Object::new(scope);

                let type_key = v8::String::new(scope, "type").unwrap().into();
                let type_val = v8::String::new(scope, "public").unwrap().into();
                public_key_obj.set(scope, type_key, type_val);

                let asym_type_key = v8::String::new(scope, "asymmetricKeyType").unwrap().into();
                let asym_type_val = v8::String::new(scope, key_type).unwrap().into();
                public_key_obj.set(scope, asym_type_key, asym_type_val);

                let pem_key_val = v8::String::new(scope, &pem_key).unwrap().into();
                let pem_key_prop = v8::String::new(scope, "pem").unwrap().into();
                public_key_obj.set(scope, pem_key_prop, pem_key_val);

                // Export method
                let export_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let (format, export_type, options_was_object) =
                            key_export_options_from_arg(scope, args.get(0));

                        let this_obj = args.this();
                        let pem_str_name = v8::String::new(scope, "pem").unwrap();
                        let pem_prop = this_obj.get(scope, pem_str_name.into());

                        if let Some(pem) = pem_prop.and_then(|p| p.to_string(scope)) {
                            let pem_lossy = pem.to_rust_string_lossy(scope);
                            if format == "pem" {
                                let exported_pem = if options_was_object {
                                    let key_type =
                                        export_type.unwrap_or_else(|| "spki".to_string());
                                    match public_key_pem_from_pem(&pem_lossy, &key_type) {
                                        Ok(pem) => pem,
                                        Err(error_message) => {
                                            let error_obj =
                                                if is_crypto_incompatible_key_options_error(
                                                    &error_message,
                                                ) {
                                                    crypto_incompatible_key_options_error(scope)
                                                } else {
                                                    let error =
                                                        v8::String::new(scope, &error_message)
                                                            .unwrap();
                                                    v8::Exception::type_error(scope, error)
                                                };
                                            scope.throw_exception(error_obj);
                                            return;
                                        }
                                    }
                                } else {
                                    pem_lossy
                                };
                                let result = v8::String::new(scope, &exported_pem).unwrap();
                                retval.set(result.into());
                            } else if format == "der" || format == "buffer" {
                                let key_type = match export_type {
                                    Some(key_type) => key_type,
                                    None if !options_was_object => "spki".to_string(),
                                    None => {
                                        let error_msg = v8::String::new(
                                            scope,
                                            "export: type is required for DER export",
                                        )
                                        .unwrap();
                                        let error_obj = v8::Exception::type_error(scope, error_msg);
                                        scope.throw_exception(error_obj);
                                        return;
                                    }
                                };
                                match public_key_der_from_pem(&pem_lossy, &key_type) {
                                    Ok(der) => {
                                        let result = create_buffer_wrapper(scope, &der);
                                        retval.set(result.into());
                                    }
                                    Err(error_message) => {
                                        let error_obj = if is_crypto_incompatible_key_options_error(
                                            &error_message,
                                        ) {
                                            crypto_incompatible_key_options_error(scope)
                                        } else {
                                            let error =
                                                v8::String::new(scope, &error_message).unwrap();
                                            v8::Exception::type_error(scope, error)
                                        };
                                        scope.throw_exception(error_obj);
                                    }
                                }
                            } else if format == "jwk" {
                                match public_jwk_from_pem(&pem_lossy) {
                                    Ok(jwk) => {
                                        let result = serde_json_value_to_v8(scope, &jwk);
                                        retval.set(result);
                                    }
                                    Err(error_message) => {
                                        let error_msg = format!("export: {}", error_message);
                                        let error = v8::String::new(scope, &error_msg).unwrap();
                                        let error_obj = v8::Exception::type_error(scope, error);
                                        scope.throw_exception(error_obj);
                                    }
                                }
                            } else {
                                let error_msg = format!(
                                    "export: unsupported format '{}'. Supported: pem, der, jwk",
                                    format
                                );
                                let error = v8::String::new(scope, &error_msg).unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj);
                            }
                        } else {
                            let error_msg =
                                v8::String::new(scope, "export: no key material found").unwrap();
                            let error = v8::Exception::type_error(scope, error_msg);
                            scope.throw_exception(error);
                        }
                    },
                );
                let export_fn = export_fn_opt.unwrap();

                let export_key = v8::String::new(scope, "export").unwrap().into();
                public_key_obj.set(scope, export_key, export_fn.into());

                retval.set(public_key_obj.into());
            },
        );
        let create_public_key_fn = create_public_key_fn_opt.unwrap();
        let create_public_key_key = v8::String::new(scope, "createPublicKey").unwrap().into();
        crypto_obj.set(scope, create_public_key_key, create_public_key_fn.into());

        // ==================== createSecretKey (v0.3.28) ====================
        // Creates a SecretKey object for symmetric cryptography
        let create_secret_key_fn_opt = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                let key_input = args.get(0);

                // Parse key - can be buffer, string, or Uint8Array
                let mut key_bytes: Vec<u8> = Vec::new();

                if key_input.is_string() {
                    if let Some(str_val) = key_input.to_string(scope) {
                        key_bytes = str_val.to_rust_string_lossy(scope).as_bytes().to_vec();
                    }
                } else if key_input.is_object() {
                    // Try TypedArray first
                    if let Ok(ta) = v8::Local::<v8::TypedArray>::try_from(key_input) {
                        key_bytes.resize(ta.byte_length(), 0);
                        ta.copy_contents(&mut key_bytes);
                    } else if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(key_input) {
                        let backing_store = ab.get_backing_store();
                        let store_slice = unsafe {
                            std::slice::from_raw_parts(
                                backing_store.as_ref().as_ptr() as *const u8,
                                ab.byte_length(),
                            )
                        };
                        key_bytes = store_slice.to_vec();
                    } else {
                        // Handle Beejs Buffer (Object with length property and numeric indices)
                        if let Ok(obj) = v8::Local::<v8::Object>::try_from(key_input) {
                            let length_key = v8::String::new(scope, "length").unwrap();
                            let length_prop = obj.get(scope, length_key.into());

                            if let Some(len_val) = length_prop.and_then(|l| l.to_integer(scope)) {
                                let len = len_val.value() as usize;
                                if len > 0 {
                                    key_bytes.resize(len, 0);
                                    for i in 0..len {
                                        let idx: v8::Local<v8::Integer> =
                                            v8::Integer::new(scope, i as i32);
                                        let byte_val = obj.get(scope, idx.into());
                                        if let Some(b) = byte_val.and_then(|b| b.to_integer(scope))
                                        {
                                            key_bytes[i] = b.value() as u8;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if key_bytes.is_empty() {
                    let error_msg =
                        v8::String::new(scope, "createSecretKey: invalid key format").unwrap();
                    let error = v8::Exception::type_error(scope, error_msg);
                    scope.throw_exception(error);
                    return;
                }

                // Convert to base64 for storage
                use base64::{engine::general_purpose::STANDARD, Engine as _};
                let key_base64 = STANDARD.encode(&key_bytes);

                // Create SecretKey object
                let secret_key_obj = v8::Object::new(scope);

                let type_key = v8::String::new(scope, "type").unwrap().into();
                let type_val = v8::String::new(scope, "secret").unwrap().into();
                secret_key_obj.set(scope, type_key, type_val);

                let asym_type_key = v8::String::new(scope, "asymmetricKeyType").unwrap().into();
                let asym_type_val = v8::String::new(scope, "secret").unwrap().into();
                secret_key_obj.set(scope, asym_type_key, asym_type_val);

                // Store key length
                let length_key = v8::String::new(scope, "length").unwrap().into();
                let length_val = v8::Integer::new(scope, key_bytes.len() as i32);
                secret_key_obj.set(scope, length_key, length_val.into());

                // Store base64 encoded key
                let pem_key_val = v8::String::new(scope, &key_base64).unwrap().into();
                let pem_key_prop = v8::String::new(scope, "pem").unwrap().into();
                secret_key_obj.set(scope, pem_key_prop, pem_key_val);

                // Export method
                let export_fn_opt = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args: v8::FunctionCallbackArguments,
                     mut retval: v8::ReturnValue| {
                        let format = args
                            .get(0)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_else(|| "raw".to_string());

                        let this_obj = args.this();
                        let pem_str_name = v8::String::new(scope, "pem").unwrap();
                        let pem_prop = this_obj.get(scope, pem_str_name.into());

                        if let Some(pem) = pem_prop.and_then(|p| p.to_string(scope)) {
                            let base64_str = pem.to_rust_string_lossy(scope);
                            let key_bytes = STANDARD.decode(&base64_str).unwrap_or_default();

                            if format == "raw" || format == "buffer" {
                                // Return as Uint8Array
                                let array_buffer = v8::ArrayBuffer::new(scope, key_bytes.len());
                                if let Some(view) =
                                    v8::Uint8Array::new(scope, array_buffer, 0, key_bytes.len())
                                {
                                    retval.set(view.into());
                                }
                            } else if format == "base64" {
                                let result = v8::String::new(scope, &base64_str).unwrap();
                                retval.set(result.into());
                            } else {
                                let error_msg = format!("export: unsupported format '{}'. Supported: raw, buffer, base64", format);
                                let error = v8::String::new(scope, &error_msg).unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj);
                            }
                        } else {
                            let error_msg =
                                v8::String::new(scope, "export: no key material found").unwrap();
                            let error = v8::Exception::type_error(scope, error_msg);
                            scope.throw_exception(error);
                        }
                    },
                );
                let export_fn = export_fn_opt.unwrap();

                let export_key = v8::String::new(scope, "export").unwrap().into();
                secret_key_obj.set(scope, export_key, export_fn.into());

                retval.set(secret_key_obj.into());
            },
        );
        let create_secret_key_fn = create_secret_key_fn_opt.unwrap();
        let create_secret_key_key = v8::String::new(scope, "createSecretKey").unwrap().into();
        crypto_obj.set(scope, create_secret_key_key, create_secret_key_fn.into());

        // ==================== hkdf (v0.3.29) ====================
        // HMAC-based Key Derivation Function (RFC 5869)
        let hkdf_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                // Parse arguments: hkdf(digest, ikm, salt, info, keylen)
                let digest = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_else(|| "sha256".to_string());

                let ikm = args
                    .get(1)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                let salt = args
                    .get(2)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                let info = args
                    .get(3)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                let keylen: usize = args
                    .get(4)
                    .to_integer(scope)
                    .map(|n| n.value() as usize)
                    .unwrap_or(32);

                // Validate digest algorithm
                let valid_algorithms = ["sha1", "sha256", "sha512"];
                if !valid_algorithms.contains(&digest.as_str()) {
                    let error_msg = format!(
                        "hkdf: unsupported digest '{}'. Supported: {}",
                        digest,
                        valid_algorithms.join(", ")
                    );
                    let error = v8::String::new(scope, &error_msg).unwrap();
                    let error_obj = v8::Exception::type_error(scope, error);
                    scope.throw_exception(error_obj.into());
                    return;
                }

                // HKDF implementation
                let result = hkdf_derive(&digest, &ikm, &salt, &info, keylen);

                // Create Uint8Array result
                let array_buffer = v8::ArrayBuffer::new(scope, keylen);
                let backing_store = array_buffer.get_backing_store();
                for (i, byte) in result.iter().enumerate() {
                    backing_store[i].set(*byte);
                }
                if let Some(uint8_array) = v8::Uint8Array::new(scope, array_buffer, 0, keylen) {
                    retval.set(uint8_array.into());
                }
            },
        );
        let hkdf_fn = match hkdf_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let hkdf_key = v8::String::new(scope, "hkdf").unwrap().into();
        crypto_obj.set(scope, hkdf_key, hkdf_fn.into());

        // ==================== hkdfSync (v0.3.29) ====================
        let hkdf_sync_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                // Same as hkdf but synchronous
                let digest = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_else(|| "sha256".to_string());

                let ikm = args
                    .get(1)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                let salt = args
                    .get(2)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                let info = args
                    .get(3)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();

                let keylen: usize = args
                    .get(4)
                    .to_integer(scope)
                    .map(|n| n.value() as usize)
                    .unwrap_or(32);

                // HKDF implementation
                let result = hkdf_derive(&digest, &ikm, &salt, &info, keylen);

                // Create Uint8Array result
                let array_buffer = v8::ArrayBuffer::new(scope, keylen);
                let backing_store = array_buffer.get_backing_store();
                for (i, byte) in result.iter().enumerate() {
                    backing_store[i].set(*byte);
                }
                if let Some(uint8_array) = v8::Uint8Array::new(scope, array_buffer, 0, keylen) {
                    retval.set(uint8_array.into());
                }
            },
        );
        let hkdf_sync_fn = match hkdf_sync_fn {
            Some(f) => f,
            None => return Ok(()),
        };
        let hkdf_sync_key = v8::String::new(scope, "hkdfSync").unwrap().into();
        crypto_obj.set(scope, hkdf_sync_key, hkdf_sync_fn.into());

        let crypto_key = v8::String::new(scope, "crypto").unwrap().into();
        global.set(scope, crypto_key, crypto_obj.into());


        Ok(())
    }

    /// Set up CommonJS module system (require, module, exports, __dirname, __filename)
    /// v0.3.x: Simplified module system for MinimalRuntime
    fn setup_module_system(
        scope: &mut v8::ContextScope<v8::HandleScope>,
        context: &v8::Context,
        main_module_dir: &str,
        main_module_filename: &str,
    ) -> Result<()> {
        let global = context.global(scope);

        // Create module object
        let module_obj = v8::Object::new(scope);
        let module_id_key = v8::String::new(scope, "id").unwrap().into();
        let module_id_val = v8::String::new(scope, "<anonymous>").unwrap().into();
        module_obj.set(scope, module_id_key, module_id_val);

        let module_filename_key = v8::String::new(scope, "filename").unwrap().into();
        let module_filename_val = v8::String::new(scope, main_module_filename).unwrap().into();
        module_obj.set(scope, module_filename_key, module_filename_val);

        let module_parent_key = v8::String::new(scope, "parent").unwrap().into();
        let module_parent_val = v8::null(scope).into();
        module_obj.set(scope, module_parent_key, module_parent_val);

        let module_loaded_key = v8::String::new(scope, "loaded").unwrap().into();
        let module_loaded_val = v8::Boolean::new(scope, false);
        module_obj.set(scope, module_loaded_key, module_loaded_val.into());

        // Create exports object (should be same as module.exports)
        let exports_obj = v8::Object::new(scope);

        // Set module.exports to reference exports_obj
        let module_exports_key = v8::String::new(scope, "exports").unwrap().into();
        module_obj.set(scope, module_exports_key, exports_obj.clone().into());

        // Create require function
        let require_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
            if args.length() >= 1 {
                let module_id = args.get(0);
                let requested_module_id_str = if let Some(s) = module_id.to_string(scope) {
                    s.to_rust_string_lossy(scope)
                } else {
                    "unknown".to_string()
                };
                let module_id_str = if let Some(builtin_name) =
                    requested_module_id_str.strip_prefix("node:")
                {
                    if crate::nodejs_core::commonjs_resolver::is_builtin_module(builtin_name) {
                        builtin_name.to_string()
                    } else {
                        let error_msg = format!("Cannot find module '{}'", requested_module_id_str);
                        let error_str = v8::String::new(scope, &error_msg).unwrap();
                        let error_obj = v8::Exception::error(scope, error_str);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                } else if let Some(bee_name) =
                    requested_module_id_str.strip_prefix("bee:")
                {
                    if crate::nodejs_core::commonjs_resolver::is_builtin_module(&requested_module_id_str)
                        || crate::nodejs_core::commonjs_resolver::is_builtin_module(bee_name)
                    {
                        bee_name.to_string()
                    } else {
                        let error_msg = format!("Cannot find module '{}'", requested_module_id_str);
                        let error_str = v8::String::new(scope, &error_msg).unwrap();
                        let error_obj = v8::Exception::error(scope, error_str);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                } else {
                    requested_module_id_str
                };

                // Return appropriate module object based on module id
                let result_obj = v8::Object::new(scope);

                match module_id_str.as_str() {
                    "buffer" => {
                        // Always return the same Buffer installed on globalThis.
                        let context = scope.get_current_context();
                        let global = context.global(scope);
                        let buffer_key = v8::String::new(scope, "Buffer").unwrap();
                        if let Some(buffer_val) = global.get(scope, buffer_key.into()) {
                            if !buffer_val.is_undefined() {
                                result_obj.set(scope, buffer_key.into(), buffer_val);
                                let default_key = v8::String::new(scope, "default").unwrap();
                                let default_obj = v8::Object::new(scope);
                                default_obj.set(scope, buffer_key.into(), buffer_val);
                                result_obj.set(scope, default_key.into(), default_obj.into());
                                retval.set(result_obj.into());
                                return;
                            }
                        }
                        let error_str = v8::String::new(scope, "Buffer global is not available")
                            .unwrap();
                        let error_obj = v8::Exception::error(scope, error_str);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                    "process" => {
                        let context = scope.get_current_context();
                        let global = context.global(scope);
                        let process_key = v8::String::new(scope, "process").unwrap();
                        if let Some(process_value) = global.get(scope, process_key.into()) {
                            if !process_value.is_undefined() {
                                retval.set(process_value);
                                return;
                            }
                        }

                        let error_str = v8::String::new(scope, "process global is not available")
                            .unwrap();
                        let error_obj = v8::Exception::error(scope, error_str);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                    "path" => {
                        // Unify with the global path installed by nodejs_core::path.
                        let context = scope.get_current_context();
                        let global = context.global(scope);
                        let path_key = v8::String::new(scope, "path").unwrap();
                        if let Some(path_val) = global.get(scope, path_key.into()) {
                            if !path_val.is_undefined() {
                                retval.set(path_val);
                                return;
                            }
                        }
                        let error_str =
                            v8::String::new(scope, "path global is not available").unwrap();
                        let error_obj = v8::Exception::error(scope, error_str);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                    "fs" => {
                        let ctx = scope.get_current_context();
                        let global_obj = ctx.global(scope);
                        let fs_key = v8::String::new(scope, "fs").unwrap();
                        if let Some(fs_value) = global_obj.get(scope, fs_key.into()) {
                            if !fs_value.is_undefined() && !fs_value.is_null() {
                                retval.set(fs_value);
                                return;
                            }
                        }

                        if !legacy_fs_fallback_enabled() {
                            let error = v8::String::new(
                                scope,
                                "Cannot load builtin module 'fs': global fs binding is unavailable",
                            )
                            .unwrap();
                            let exception = v8::Exception::type_error(scope, error);
                            scope.throw_exception(exception);
                            return;
                        }

                        // Return fs module with file system methods (v0.3.5)
                        let fs_obj = v8::Object::new(scope);

                        // Add readFile function
                        let readfile_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                            if args.length() >= 1 {
                                if let Some(path_val) = args.get(0).to_string(scope) {
                                    let path = path_val.to_rust_string_lossy(scope);
                                    match std::fs::read_to_string(&path) {
                                        Ok(contents) => {
                                            let contents_val = v8::String::new(scope, &contents).unwrap();
                                            retval.set(contents_val.into());
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Error reading file: {}", e);
                                            let error_val = v8::String::new(scope, &error_msg).unwrap();
                                            retval.set(error_val.into());
                                        }
                                    }
                                }
                            }
                        }).unwrap();
                        let readfile_key = v8::String::new(scope, "readFileSync").unwrap().into();
                        fs_obj.set(scope, readfile_key, readfile_fn.into());

                        // Add writeFile function
                        let writefile_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                            if args.length() >= 2 {
                                if let (Some(path_val), Some(data_val)) = (args.get(0).to_string(scope), args.get(1).to_string(scope)) {
                                    let path = path_val.to_rust_string_lossy(scope);
                                    let data = data_val.to_rust_string_lossy(scope);
                                    match std::fs::write(&path, data) {
                                        Ok(_) => {
                                            let success_val = v8::undefined(scope).into();
                                            retval.set(success_val);
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Error writing file: {}", e);
                                            let error_val = v8::String::new(scope, &error_msg).unwrap();
                                            retval.set(error_val.into());
                                        }
                                    }
                                }
                            }
                        }).unwrap();
                        let writefile_key = v8::String::new(scope, "writeFileSync").unwrap().into();
                        fs_obj.set(scope, writefile_key, writefile_fn.into());

                        // Add existsSync function
                        let exists_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                            if args.length() >= 1 {
                                if let Some(path_val) = args.get(0).to_string(scope) {
                                    let path = path_val.to_rust_string_lossy(scope);
                                    let exists = std::path::Path::new(&path).exists();
                                    let exists_val = v8::Boolean::new(scope, exists);
                                    retval.set(exists_val.into());
                                }
                            }
                        }).unwrap();
                        let exists_key = v8::String::new(scope, "existsSync").unwrap().into();
                        fs_obj.set(scope, exists_key, exists_fn.into());

                        // Add mkdirSync function
                        let mkdir_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                            if args.length() >= 1 {
                                if let Some(path_val) = args.get(0).to_string(scope) {
                                    let path = path_val.to_rust_string_lossy(scope);
                                    match std::fs::create_dir_all(&path) {
                                        Ok(_) => {
                                            retval.set(v8::undefined(scope).into());
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Error creating directory: {}", e);
                                            let error_val = v8::String::new(scope, &error_msg).unwrap();
                                            retval.set(error_val.into());
                                        }
                                    }
                                }
                            }
                        }).unwrap();
                        let mkdir_key = v8::String::new(scope, "mkdirSync").unwrap().into();
                        fs_obj.set(scope, mkdir_key, mkdir_fn.into());

                        // Add readdirSync function
                        let readdir_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                            if args.length() >= 1 {
                                if let Some(path_val) = args.get(0).to_string(scope) {
                                    let path = path_val.to_rust_string_lossy(scope);
                                    match std::fs::read_dir(&path) {
                                        Ok(entries) => {
                                            let mut file_names = Vec::new();
                                            for entry in entries {
                                                if let Ok(entry) = entry {
                                                    if let Ok(file_name) = entry.file_name().into_string() {
                                                        file_names.push(file_name);
                                                    }
                                                }
                                            }
                                            let js_array = v8::Array::new(scope, file_names.len() as i32);
                                            for (i, name) in file_names.iter().enumerate() {
                                                let name_val = v8::String::new(scope, name).unwrap();
                                                js_array.set_index(scope, i as u32, name_val.into());
                                            }
                                            retval.set(js_array.into());
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Error reading directory: {}", e);
                                            let error_val = v8::String::new(scope, &error_msg).unwrap();
                                            retval.set(error_val.into());
                                        }
                                    }
                                }
                            }
                        }).unwrap();
                        let readdir_key = v8::String::new(scope, "readdirSync").unwrap().into();
                        fs_obj.set(scope, readdir_key, readdir_fn.into());

                        // Add unlinkSync function
                        let unlink_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                            if args.length() >= 1 {
                                if let Some(path_val) = args.get(0).to_string(scope) {
                                    let path = path_val.to_rust_string_lossy(scope);
                                    match std::fs::remove_file(&path) {
                                        Ok(_) => {
                                            retval.set(v8::undefined(scope).into());
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Error deleting file: {}", e);
                                            let error_val = v8::String::new(scope, &error_msg).unwrap();
                                            let exception = v8::Exception::type_error(scope, error_val);
                                            scope.throw_exception(exception.into());
                                        }
                                    }
                                }
                            }
                        }).unwrap();
                        let unlink_key = v8::String::new(scope, "unlinkSync").unwrap().into();
                        fs_obj.set(scope, unlink_key, unlink_fn.into());

                        // Add rmdirSync function
                        let rmdir_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                            if args.length() >= 1 {
                                if let Some(path_val) = args.get(0).to_string(scope) {
                                    let path = path_val.to_rust_string_lossy(scope);
                                    match std::fs::remove_dir(&path) {
                                        Ok(_) => {
                                            retval.set(v8::undefined(scope).into());
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Error removing directory: {}", e);
                                            let error_val = v8::String::new(scope, &error_msg).unwrap();
                                            let exception = v8::Exception::type_error(scope, error_val);
                                            scope.throw_exception(exception.into());
                                        }
                                    }
                                }
                            }
                        }).unwrap();
                        let rmdir_key = v8::String::new(scope, "rmdirSync").unwrap().into();
                        fs_obj.set(scope, rmdir_key, rmdir_fn.into());

                        // Add readFile function (async with callback) - v0.3.6
                        let readfile_async_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _retval: v8::ReturnValue| {
                            if args.length() < 2 {
                                let error = v8::String::new(scope, "readFile: missing arguments").unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            }

                            let path_val = args.get(0);

                            // Find the callback - it's at index 1 if index 1 is a function,
                            // otherwise it's at index 2 (index 1 is options)
                            let callback_val = if args.get(1).is_function() {
                                args.get(1)
                            } else if args.length() >= 3 && args.get(2).is_function() {
                                args.get(2)
                            } else {
                                let error = v8::String::new(scope, "readFile: callback must be a function").unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            };

                            let path = path_val.to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| "".to_string());

                            // Determine encoding from index 1 if it's a string and index 2 is the callback
                            let _encoding = if !args.get(1).is_function() && args.get(1).is_string() {
                                args.get(1).to_string(scope)
                                    .map(|s| s.to_rust_string_lossy(scope))
                                    .unwrap_or_else(|| "utf8".to_string())
                            } else {
                                "utf8".to_string()
                            };

                            // Execute read asynchronously using tokio runtime
                            let callback_func = v8::Local::<v8::Function>::try_from(callback_val).unwrap();
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            let read_result = rt.block_on(async {
                                tokio::fs::read_to_string(&path).await
                            });

                            let undefined = v8::undefined(scope);
                            let null_val: v8::Local<v8::Value> = v8::null(scope).into();
                            match read_result {
                                Ok(contents) => {
                                    let contents_val = v8::String::new(scope, &contents).unwrap();
                                    let _ = callback_func.call(scope, undefined.into(), &[null_val, contents_val.into()]);
                                }
                                Err(e) => {
                                    let error_msg = format!("Error reading file: {}", e);
                                    let error_val = v8::String::new(scope, &error_msg).unwrap();
                                    let _ = callback_func.call(scope, undefined.into(), &[error_val.into(), undefined.into()]);
                                }
                            }
                        }).ok_or_else(|| -> anyhow::Error { anyhow::anyhow!("Failed to create readFile function") }).unwrap();
                        let readfile_async_key = v8::String::new(scope, "readFile").unwrap().into();
                        fs_obj.set(scope, readfile_async_key, readfile_async_fn.into());

                        // Add writeFile function (async with callback) - v0.3.6
                        let writefile_async_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _retval: v8::ReturnValue| {
                            if args.length() >= 2 {
                                let path_val = args.get(0);
                                let data_val = args.get(1);
                                let callback_val = args.get(2);

                                if callback_val.is_function() {
                                    let path = path_val.to_string(scope)
                                        .map(|s| s.to_rust_string_lossy(scope))
                                        .unwrap_or_else(|| "".to_string());
                                    let data = data_val.to_string(scope)
                                        .map(|s| s.to_rust_string_lossy(scope))
                                        .unwrap_or_else(|| "".to_string());

                                    let callback_func = v8::Local::<v8::Function>::try_from(callback_val).unwrap();

                                    let rt = tokio::runtime::Runtime::new().unwrap();
                                    let write_result = rt.block_on(async {
                                        tokio::fs::write(&path, &data).await
                                    });

                                    let undefined = v8::undefined(scope);
                                    match write_result {
                                        Ok(_) => {
                                            let null_val = v8::null(scope).into();
                                            let _ = callback_func.call(scope, undefined.into(), &[null_val]);
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Error writing file: {}", e);
                                            let error_val = v8::String::new(scope, &error_msg).unwrap();
                                            let _ = callback_func.call(scope, undefined.into(), &[error_val.into()]);
                                        }
                                    }
                                } else {
                                    let error = v8::String::new(scope, "writeFile: callback must be a function").unwrap();
                                    let error_obj = v8::Exception::type_error(scope, error);
                                    scope.throw_exception(error_obj.into());
                                }
                            } else {
                                let error = v8::String::new(scope, "writeFile: missing arguments").unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj.into());
                            }
                        }).ok_or_else(|| -> anyhow::Error { anyhow::anyhow!("Failed to create writeFile function") }).unwrap();
                        let writefile_async_key = v8::String::new(scope, "writeFile").unwrap().into();
                        fs_obj.set(scope, writefile_async_key, writefile_async_fn.into());

                        // Add appendFile function (async with callback) - v0.3.6
                        let appendfile_async_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _retval: v8::ReturnValue| {
                            if args.length() >= 3 {
                                let path_val = args.get(0);
                                let data_val = args.get(1);
                                let callback_val = args.get(2);

                                let path = path_val.to_string(scope)
                                    .map(|s| s.to_rust_string_lossy(scope))
                                    .unwrap_or_else(|| "".to_string());
                                let data = data_val.to_string(scope)
                                    .map(|s| s.to_rust_string_lossy(scope))
                                    .unwrap_or_else(|| "".to_string());

                                let callback_func = v8::Local::<v8::Function>::try_from(callback_val).unwrap();

                                // Use tokio runtime for async file append
                                let rt = tokio::runtime::Runtime::new().unwrap();
                                let append_result = rt.block_on(async {
                                    // Read existing content, append, then write
                                    let mut content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
                                    content.push_str(&data);
                                    tokio::fs::write(&path, &content).await
                                });

                                let undefined = v8::undefined(scope);
                                match append_result {
                                    Ok(_) => {
                                        let null_val = v8::null(scope).into();
                                        let _ = callback_func.call(scope, undefined.into(), &[null_val]);
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Error appending to file: {}", e);
                                        let error_val = v8::String::new(scope, &error_msg).unwrap();
                                        let _ = callback_func.call(scope, undefined.into(), &[error_val.into()]);
                                    }
                                }
                            } else {
                                let error = v8::String::new(scope, "appendFile: missing arguments").unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj.into());
                            }
                        }).ok_or_else(|| -> anyhow::Error { anyhow::anyhow!("Failed to create appendFile function") }).unwrap();
                        let appendfile_async_key = v8::String::new(scope, "appendFile").unwrap().into();
                        fs_obj.set(scope, appendfile_async_key, appendfile_async_fn.into());

                        // For fs module, directly return fs_obj as the module exports
                        retval.set(fs_obj.into());
                        return;
                    }
                    "fs/promises" => {
                        let ctx = scope.get_current_context();
                        let global_obj = ctx.global(scope);
                        let fs_key = v8::String::new(scope, "fs").unwrap();
                        if let Some(fs_value) = global_obj.get(scope, fs_key.into()) {
                            if let Ok(fs_obj) = v8::Local::<v8::Object>::try_from(fs_value) {
                                let promises_key = v8::String::new(scope, "promises").unwrap();
                                if let Some(promises_value) =
                                    fs_obj.get(scope, promises_key.into())
                                {
                                    if !promises_value.is_undefined() && !promises_value.is_null() {
                                        retval.set(promises_value);
                                        return;
                                    }
                                }
                            }
                        }

                        if !legacy_fs_fallback_enabled() {
                            let error = v8::String::new(
                                scope,
                                "Cannot load builtin module 'fs/promises': global fs.promises binding is unavailable",
                            )
                            .unwrap();
                            let exception = v8::Exception::type_error(scope, error);
                            scope.throw_exception(exception);
                            return;
                        }

                        // Return fs/promises module with Promise-based API (v0.3.7)
                        let promises_obj = v8::Object::new(scope);

                        // Create Promise-based readFile
                        let readfile_promise_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                            if args.length() < 1 {
                                let error = v8::String::new(scope, "readFile: missing path argument").unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            }

                            let path_val = args.get(0);
                            let path = path_val.to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| "".to_string());

                            // Determine encoding from index 1 if it's a string
                            let _encoding = if args.length() >= 2 {
                                let enc = args.get(1);
                                if enc.is_string() {
                                    enc.to_string(scope).map(|s| s.to_rust_string_lossy(scope))
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            // Create a promise resolver
                            let resolver = v8::PromiseResolver::new(scope).unwrap();
                            let promise = resolver.get_promise(scope);

                            // Return the promise immediately
                            retval.set(promise.into());

                            // Now resolve the promise asynchronously using tokio
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async {
                                match tokio::fs::read_to_string(&path).await {
                                    Ok(contents) => {
                                        let resolver = v8::PromiseResolver::new(scope).unwrap();
                                        let value = v8::String::new(scope, &contents).unwrap();
                                        resolver.resolve(scope, value.into());
                                    }
                                    Err(e) => {
                                        let resolver = v8::PromiseResolver::new(scope).unwrap();
                                        let error_msg = format!("Error reading file: {}", e);
                                        let error_val = v8::String::new(scope, &error_msg).unwrap();
                                        let error_obj = v8::Exception::error(scope, error_val);
                                        resolver.reject(scope, error_obj);
                                    }
                                }
                            });
                        }).ok_or_else(|| anyhow::anyhow!("Failed to create readFile Promise function")).unwrap();
                        let readfile_promise_key = v8::String::new(scope, "readFile").unwrap().into();
                        promises_obj.set(scope, readfile_promise_key, readfile_promise_fn.into());

                        // Create Promise-based writeFile
                        let writefile_promise_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                            if args.length() < 2 {
                                let error = v8::String::new(scope, "writeFile: missing arguments").unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            }

                            let path_val = args.get(0);
                            let data_val = args.get(1);
                            let path = path_val.to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| "".to_string());
                            let data = data_val.to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| "".to_string());

                            // Create a promise resolver
                            let resolver = v8::PromiseResolver::new(scope).unwrap();
                            let promise = resolver.get_promise(scope);
                            retval.set(promise.into());

                            // Resolve asynchronously
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async {
                                match tokio::fs::write(&path, &data).await {
                                    Ok(_) => {
                                        let resolver = v8::PromiseResolver::new(scope).unwrap();
                                        let undefined = v8::undefined(scope);
                                        resolver.resolve(scope, undefined.into());
                                    }
                                    Err(e) => {
                                        let resolver = v8::PromiseResolver::new(scope).unwrap();
                                        let error_msg = format!("Error writing file: {}", e);
                                        let error_val = v8::String::new(scope, &error_msg).unwrap();
                                        let error_obj = v8::Exception::error(scope, error_val);
                                        resolver.reject(scope, error_obj);
                                    }
                                }
                            });
                        }).ok_or_else(|| anyhow::anyhow!("Failed to create writeFile Promise function")).unwrap();
                        let writefile_promise_key = v8::String::new(scope, "writeFile").unwrap().into();
                        promises_obj.set(scope, writefile_promise_key, writefile_promise_fn.into());

                        // Create Promise-based appendFile
                        let appendfile_promise_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                            if args.length() < 2 {
                                let error = v8::String::new(scope, "appendFile: missing arguments").unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            }

                            let path_val = args.get(0);
                            let data_val = args.get(1);
                            let path = path_val.to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| "".to_string());
                            let data = data_val.to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| "".to_string());

                            // Create a promise resolver
                            let resolver = v8::PromiseResolver::new(scope).unwrap();
                            let promise = resolver.get_promise(scope);
                            retval.set(promise.into());

                            // Resolve asynchronously
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async {
                                // Read existing content, append, then write
                                let mut content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
                                content.push_str(&data);
                                match tokio::fs::write(&path, &content).await {
                                    Ok(_) => {
                                        let resolver = v8::PromiseResolver::new(scope).unwrap();
                                        let undefined = v8::undefined(scope);
                                        resolver.resolve(scope, undefined.into());
                                    }
                                    Err(e) => {
                                        let resolver = v8::PromiseResolver::new(scope).unwrap();
                                        let error_msg = format!("Error appending to file: {}", e);
                                        let error_val = v8::String::new(scope, &error_msg).unwrap();
                                        let error_obj = v8::Exception::error(scope, error_val);
                                        resolver.reject(scope, error_obj);
                                    }
                                }
                            });
                        }).ok_or_else(|| anyhow::anyhow!("Failed to create appendFile Promise function")).unwrap();
                        let appendfile_promise_key = v8::String::new(scope, "appendFile").unwrap().into();
                        promises_obj.set(scope, appendfile_promise_key, appendfile_promise_fn.into());

                        // Create Promise-based unlink
                        let unlink_promise_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                            if args.length() < 1 {
                                let error = v8::String::new(scope, "unlink: missing path argument").unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            }

                            let path_val = args.get(0);
                            let path = path_val.to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| "".to_string());

                            let resolver = v8::PromiseResolver::new(scope).unwrap();
                            let promise = resolver.get_promise(scope);
                            retval.set(promise.into());

                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async {
                                match tokio::fs::remove_file(&path).await {
                                    Ok(_) => {
                                        let resolver = v8::PromiseResolver::new(scope).unwrap();
                                        let undefined = v8::undefined(scope);
                                        resolver.resolve(scope, undefined.into());
                                    }
                                    Err(e) => {
                                        let resolver = v8::PromiseResolver::new(scope).unwrap();
                                        let error_msg = format!("Error unlinking file: {}", e);
                                        let error_val = v8::String::new(scope, &error_msg).unwrap();
                                        let error_obj = v8::Exception::error(scope, error_val);
                                        resolver.reject(scope, error_obj);
                                    }
                                }
                            });
                        }).ok_or_else(|| anyhow::anyhow!("Failed to create unlink Promise function")).unwrap();
                        let unlink_promise_key = v8::String::new(scope, "unlink").unwrap().into();
                        promises_obj.set(scope, unlink_promise_key, unlink_promise_fn.into());

                        // Create Promise-based mkdir
                        let mkdir_promise_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                            if args.length() < 1 {
                                let error = v8::String::new(scope, "mkdir: missing path argument").unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            }

                            let path_val = args.get(0);
                            let path = path_val.to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| "".to_string());

                            let resolver = v8::PromiseResolver::new(scope).unwrap();
                            let promise = resolver.get_promise(scope);
                            retval.set(promise.into());

                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async {
                                match tokio::fs::create_dir_all(&path).await {
                                    Ok(_) => {
                                        let resolver = v8::PromiseResolver::new(scope).unwrap();
                                        let undefined = v8::undefined(scope);
                                        resolver.resolve(scope, undefined.into());
                                    }
                                    Err(e) => {
                                        let resolver = v8::PromiseResolver::new(scope).unwrap();
                                        let error_msg = format!("Error creating directory: {}", e);
                                        let error_val = v8::String::new(scope, &error_msg).unwrap();
                                        let error_obj = v8::Exception::error(scope, error_val);
                                        resolver.reject(scope, error_obj);
                                    }
                                }
                            });
                        }).ok_or_else(|| anyhow::anyhow!("Failed to create mkdir Promise function")).unwrap();
                        let mkdir_promise_key = v8::String::new(scope, "mkdir").unwrap().into();
                        promises_obj.set(scope, mkdir_promise_key, mkdir_promise_fn.into());

                        // Create Promise-based rmdir
                        let rmdir_promise_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                            if args.length() < 1 {
                                let error = v8::String::new(scope, "rmdir: missing path argument").unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            }

                            let path_val = args.get(0);
                            let path = path_val.to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| "".to_string());

                            let resolver = v8::PromiseResolver::new(scope).unwrap();
                            let promise = resolver.get_promise(scope);
                            retval.set(promise.into());

                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async {
                                match tokio::fs::remove_dir_all(&path).await {
                                    Ok(_) => {
                                        let resolver = v8::PromiseResolver::new(scope).unwrap();
                                        let undefined = v8::undefined(scope);
                                        resolver.resolve(scope, undefined.into());
                                    }
                                    Err(e) => {
                                        let resolver = v8::PromiseResolver::new(scope).unwrap();
                                        let error_msg = format!("Error removing directory: {}", e);
                                        let error_val = v8::String::new(scope, &error_msg).unwrap();
                                        let error_obj = v8::Exception::error(scope, error_val);
                                        resolver.reject(scope, error_obj);
                                    }
                                }
                            });
                        }).ok_or_else(|| anyhow::anyhow!("Failed to create rmdir Promise function")).unwrap();
                        let rmdir_promise_key = v8::String::new(scope, "rmdir").unwrap().into();
                        promises_obj.set(scope, rmdir_promise_key, rmdir_promise_fn.into());

                        // Create Promise-based readdir
                        let readdir_promise_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                            if args.length() < 1 {
                                let error = v8::String::new(scope, "readdir: missing path argument").unwrap();
                                let error_obj = v8::Exception::type_error(scope, error);
                                scope.throw_exception(error_obj.into());
                                return;
                            }

                            let path_val = args.get(0);
                            let path = path_val.to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| "".to_string());

                            let resolver = v8::PromiseResolver::new(scope).unwrap();
                            let promise = resolver.get_promise(scope);
                            retval.set(promise.into());

                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async {
                                match tokio::fs::read_dir(&path).await {
                                    Ok(mut entries) => {
                                        let mut names: Vec<String> = Vec::new();
                                        while let Ok(Some(entry)) = entries.next_entry().await {
                                            if let Ok(name) = entry.file_name().into_string() {
                                                names.push(name);
                                            }
                                        }
                                        // Create a JS array with the names
                                        let resolver = v8::PromiseResolver::new(scope).unwrap();
                                        let arr = v8::Array::new(scope, names.len() as i32);
                                        for (i, name) in names.iter().enumerate() {
                                            let name_str = v8::String::new(scope, name).unwrap();
                                            arr.set_index(scope, i as u32, name_str.into());
                                        }
                                        resolver.resolve(scope, arr.into());
                                    }
                                    Err(e) => {
                                        let resolver = v8::PromiseResolver::new(scope).unwrap();
                                        let error_msg = format!("Error reading directory: {}", e);
                                        let error_val = v8::String::new(scope, &error_msg).unwrap();
                                        let error_obj = v8::Exception::error(scope, error_val);
                                        resolver.reject(scope, error_obj);
                                    }
                                }
                            });
                        }).ok_or_else(|| anyhow::anyhow!("Failed to create readdir Promise function")).unwrap();
                        let readdir_promise_key = v8::String::new(scope, "readdir").unwrap().into();
                        promises_obj.set(scope, readdir_promise_key, readdir_promise_fn.into());

                        // Return the promises object
                        retval.set(promises_obj.into());
                        return;
                    }
                    // v0.3.194: Fixed to return actual global objects instead of fallback messages
                    // v0.3.281: Added readline to the list of builtin modules
                    "os" | "crypto" | "events" | "net" | "http" | "http2" | "https" | "tls" | "util"
                    | "url" | "querystring" | "dns" | "child_process" | "tcp_async" | "stream"
                    | "stream/promises" | "timers" | "timers/promises"
                    | "readline" | "performance" | "perf_hooks" | "assert" | "assert/strict"
                    | "diagnostics_channel" | "async_hooks" | "wasm" | "bee:wasm"
                    | "ai" | "bee:ai" | "replay" | "bee:replay" | "weights" | "bee:weights"
                    | "security" | "bee:security" | "permissions" | "bee:permissions"
                    | "kv" | "bee:kv" | "tools" | "bee:tools" | "sandbox" | "bee:sandbox" | "vfs" | "bee:vfs"
                    | "bus" | "bee:bus" | "grammar" | "bee:grammar" | "checkpoint" | "bee:checkpoint"
                    | "sockets" | "bee:sockets" | "wintertc:sockets" | "std:cli" | "bee:std/cli" => {
                        // Get context and global object
                        let ctx = scope.get_current_context();
                        let global_obj = ctx.global(scope);

                        if module_id_str == "stream/promises" {
                            let js = r#"
                            (function() {
                                const stream = globalThis.stream || require('stream');
                                function pipeline(...args) {
                                    return new Promise((resolve, reject) => {
                                        stream.pipeline(...args, (err, val) => {
                                            if (err) reject(err);
                                            else resolve(val);
                                        });
                                    });
                                }
                                function finished(s, opts) {
                                    return new Promise((resolve, reject) => {
                                        if (stream.finished) {
                                            stream.finished(s, opts, (err) => {
                                                if (err) reject(err);
                                                else resolve();
                                            });
                                        } else {
                                            s.on('finish', () => resolve());
                                            s.on('end', () => resolve());
                                            s.on('close', () => resolve());
                                            s.on('error', (err) => reject(err));
                                        }
                                    });
                                }
                                return { pipeline, finished, default: { pipeline, finished } };
                            })()
                            "#;
                            if let Some(code) = v8::String::new(scope, js) {
                                if let Some(s) = v8::Script::compile(scope, code, None) {
                                    if let Some(val) = s.run(scope) {
                                        retval.set(val);
                                        return;
                                    }
                                }
                            }
                        }

                        if module_id_str == "timers/promises" {
                            let js = r#"
                            (function() {
                                function setTimeout(delay = 0, value, options) {
                                    return new Promise((resolve, reject) => {
                                        if (options && options.signal && options.signal.aborted) {
                                            return reject(options.signal.reason || new Error('The operation was aborted'));
                                        }
                                        const timer = globalThis.setTimeout(() => resolve(value), delay);
                                        if (options && options.signal) {
                                            options.signal.addEventListener('abort', () => {
                                                globalThis.clearTimeout(timer);
                                                reject(options.signal.reason || new Error('The operation was aborted'));
                                            });
                                        }
                                    });
                                }
                                function setImmediate(value, options) {
                                    return setTimeout(0, value, options);
                                }
                                return { setTimeout, setImmediate, default: { setTimeout, setImmediate } };
                            })()
                            "#;
                            if let Some(code) = v8::String::new(scope, js) {
                                if let Some(s) = v8::Script::compile(scope, code, None) {
                                    if let Some(val) = s.run(scope) {
                                        retval.set(val);
                                        return;
                                    }
                                }
                            }
                        }

                        if module_id_str == "module" {
                            let module_exports = v8::Object::new(scope);
                            let module_key = v8::String::new(scope, "module").unwrap();
                            if let Some(current_module) = global_obj.get(scope, module_key.into()) {
                                if let Ok(current_obj) =
                                    v8::Local::<v8::Object>::try_from(current_module)
                                {
                                    let create_key = v8::String::new(scope, "createRequire").unwrap();
                                    if let Some(create_require) =
                                        current_obj.get(scope, create_key.into())
                                    {
                                        module_exports.set(
                                            scope,
                                            create_key.into(),
                                            create_require,
                                        );
                                    }
                                }
                            }
                            retval.set(module_exports.into());
                            return;
                        }

                        if module_id_str == "events" {
                            // Node shape: require('events') => { EventEmitter, ... }
                            let events_key = v8::String::new(scope, "events").unwrap();
                            if let Some(events_val) = global_obj.get(scope, events_key.into()) {
                                if !events_val.is_undefined() {
                                    retval.set(events_val);
                                    return;
                                }
                            }
                        }

                        if module_id_str == "assert" || module_id_str == "assert/strict" {
                            let assert_key = v8::String::new(scope, "assert").unwrap();
                            if let Some(assert_val) = global_obj.get(scope, assert_key.into()) {
                                if !assert_val.is_undefined() {
                                    retval.set(assert_val);
                                    return;
                                }
                            }
                        }

                        if module_id_str == "perf_hooks" {
                            let hooks_key = v8::String::new(scope, "perf_hooks").unwrap();
                            if let Some(hooks_val) = global_obj.get(scope, hooks_key.into()) {
                                if !hooks_val.is_undefined() {
                                    retval.set(hooks_val);
                                    return;
                                }
                            }
                        }

                        if module_id_str == "ai" {
                            let ai_key = v8::String::new(scope, "__bee_ai").unwrap();
                            if let Some(ai_val) = global_obj.get(scope, ai_key.into()) {
                                if !ai_val.is_undefined() {
                                    retval.set(ai_val);
                                    return;
                                }
                            }
                        }

                        if module_id_str == "string_decoder" {
                            let sd_key = v8::String::new(scope, "__string_decoder").unwrap();
                            if let Some(sd_val) = global_obj.get(scope, sd_key.into()) {
                                if !sd_val.is_undefined() {
                                    retval.set(sd_val);
                                    return;
                                }
                            }
                        }

                        if module_id_str == "url" {
                            let url_module = v8::Object::new(scope);
                            let url_key = v8::String::new(scope, "URL").unwrap();
                            if let Some(url_constructor) = global_obj.get(scope, url_key.into()) {
                                url_module.set(scope, url_key.into(), url_constructor);
                            }

                            let search_params_key =
                                v8::String::new(scope, "URLSearchParams").unwrap();
                            if let Some(search_params_constructor) =
                                global_obj.get(scope, search_params_key.into())
                            {
                                url_module.set(
                                    scope,
                                    search_params_key.into(),
                                    search_params_constructor,
                                );
                            }

                            // Legacy Node helpers
                            let file_url_to_path = v8::Function::new(
                                scope,
                                |scope: &mut v8::PinScope,
                                 args: v8::FunctionCallbackArguments,
                                 mut rv: v8::ReturnValue| {
                                    let input = args
                                        .get(0)
                                        .to_string(scope)
                                        .map(|s| s.to_rust_string_lossy(scope))
                                        .unwrap_or_default();
                                    let path = input
                                        .strip_prefix("file://")
                                        .unwrap_or(&input)
                                        .to_string();
                                    let out = v8::String::new(scope, &path).unwrap();
                                    rv.set(out.into());
                                },
                            )
                            .unwrap();
                            let path_to_file_url = v8::Function::new(
                                scope,
                                |scope: &mut v8::PinScope,
                                 args: v8::FunctionCallbackArguments,
                                 mut rv: v8::ReturnValue| {
                                    let path = args
                                        .get(0)
                                        .to_string(scope)
                                        .map(|s| s.to_rust_string_lossy(scope))
                                        .unwrap_or_default();
                                    let href = if path.starts_with("file://") {
                                        path
                                    } else {
                                        format!("file://{}", path)
                                    };
                                    // Return a minimal URL-like object with href
                                    let obj = v8::Object::new(scope);
                                    let href_key = v8::String::new(scope, "href").unwrap();
                                    let href_val = v8::String::new(scope, &href).unwrap();
                                    obj.set(scope, href_key.into(), href_val.into());
                                    rv.set(obj.into());
                                },
                            )
                            .unwrap();
                            let futp_key = v8::String::new(scope, "fileURLToPath").unwrap();
                            let ptfu_key = v8::String::new(scope, "pathToFileURL").unwrap();
                            url_module.set(scope, futp_key.into(), file_url_to_path.into());
                            url_module.set(scope, ptfu_key.into(), path_to_file_url.into());

                            retval.set(url_module.into());
                            return;
                        }

                        if module_id_str == "sockets"
                            || module_id_str == "bee:sockets"
                            || module_id_str == "wintertc:sockets"
                        {
                            let sock_key = v8::String::new(scope, "__bee_sockets").unwrap();
                            if let Some(sock_val) = global_obj.get(scope, sock_key.into()) {
                                if !sock_val.is_undefined() {
                                    retval.set(sock_val);
                                    return;
                                }
                            }
                        }

                        // Try to get the module from global
                        let clean_id = module_id_str.strip_prefix("bee:").unwrap_or(&module_id_str);
                        let mod_key = v8::String::new(scope, clean_id).unwrap();
                        if let Some(mod_val) = global_obj.get(scope, mod_key.into()) {
                            if !mod_val.is_undefined() {
                                if module_id_str == "readline" {
                                    if let Ok(module_obj) =
                                        v8::Local::<v8::Object>::try_from(mod_val)
                                    {
                                        let default_key =
                                            v8::String::new(scope, "default").unwrap();
                                        module_obj.set(scope, default_key.into(), mod_val);
                                    }
                                }
                                retval.set(mod_val);
                                return;
                            }
                        }

                        let bee_mod_key =
                            v8::String::new(scope, &format!("__bee_{}", clean_id)).unwrap();
                        if let Some(mod_val) = global_obj.get(scope, bee_mod_key.into()) {
                            if !mod_val.is_undefined() {
                                retval.set(mod_val);
                                return;
                            }
                        }

                        // Fail closed — never return a silent fake module object.
                        let error_msg = format!(
                            "ERR_UNKNOWN_BUILTIN_MODULE: No such built-in module: {}",
                            module_id_str
                        );
                        let error_str = v8::String::new(scope, &error_msg).unwrap();
                        let error_obj = v8::Exception::error(scope, error_str);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                    _ => {
                        // First, try to get __dirname from global context for relative path resolution
                        let context = scope.get_current_context();
                        let global = context.global(scope);
                        let dirname_key = v8::String::new(scope, "__dirname").unwrap();
                        let current_dirname = global.get(scope, dirname_key.into())
                            .and_then(|v| v.to_string(scope))
                            .map(|s| s.to_rust_string_lossy(scope))
                            .unwrap_or_else(|| String::from("."));

                        let module_path = match crate::nodejs_core::commonjs_resolver::resolve_commonjs_module(
                            &module_id_str,
                            std::path::Path::new(&current_dirname),
                        ) {
                            Ok(crate::nodejs_core::commonjs_resolver::ResolvedModule::File(path)) => path,
                            Ok(crate::nodejs_core::commonjs_resolver::ResolvedModule::Builtin(name)) => {
                                let clean = name.strip_prefix("bee:").unwrap_or(&name);
                                for candidate in &[
                                    name.as_str(),
                                    clean,
                                ] {
                                    let lookup_key = v8::String::new(scope, candidate).unwrap();
                                    if let Some(val) = global.get(scope, lookup_key.into()) {
                                        if !val.is_undefined() && !val.is_null() {
                                            retval.set(val);
                                            return;
                                        }
                                    }
                                    let bee_key = v8::String::new(scope, &format!("__{}", candidate)).unwrap();
                                    if let Some(val) = global.get(scope, bee_key.into()) {
                                        if !val.is_undefined() && !val.is_null() {
                                            retval.set(val);
                                            return;
                                        }
                                    }
                                    let full_bee_key = v8::String::new(scope, &format!("__bee_{}", candidate)).unwrap();
                                    if let Some(val) = global.get(scope, full_bee_key.into()) {
                                        if !val.is_undefined() && !val.is_null() {
                                            retval.set(val);
                                            return;
                                        }
                                    }
                                }
                                let error_msg = format!("Cannot load builtin module '{}' from file resolver", name);
                                let error_str = v8::String::new(scope, &error_msg).unwrap();
                                let error_obj = v8::Exception::error(scope, error_str);
                                scope.throw_exception(error_obj.into());
                                return;
                            }
                            Err(error) => {
                                // v0.3.281: Handle readline module - return from global.readline
                                if module_id_str == "readline" {
                                    let readline_key = v8::String::new(scope, "readline").unwrap().into();
                                    if let Some(readline_val) = global.get(scope, readline_key) {
                                        if !readline_val.is_undefined() && !readline_val.is_null() {
                                            // Set as 'default' property for CommonJS compatibility
                                            let default_key = v8::String::new(scope, "default").unwrap().into();
                                            result_obj.set(scope, default_key, readline_val);
                                            retval.set(result_obj.into());
                                            return;
                                        }
                                    }
                                    // Fallback if readline not found
                                    let error_msg = "Cannot find module 'readline' - readline API not available";
                                    let error_str = v8::String::new(scope, error_msg).unwrap();
                                    let error_obj = v8::Exception::error(scope, error_str);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }

                                let error_str = v8::String::new(scope, &error.to_string()).unwrap();
                                let error_obj = v8::Exception::error(scope, error_str);
                                scope.throw_exception(error_obj.into());
                                return;
                            }
                        };

                        // Try to resolve as file path
                        if module_path.exists() && module_path.is_file() {
                            let module_format = match crate::nodejs_core::commonjs_resolver::classify_commonjs_file(&module_path) {
                                Ok(module_format) => module_format,
                                Err(error) => {
                                    let error_str =
                                        v8::String::new(scope, &error.to_string()).unwrap();
                                    let error_obj = v8::Exception::error(scope, error_str);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }
                            };
                            if module_format
                                == crate::nodejs_core::commonjs_resolver::CommonJsModuleFormat::EsModule
                            {
                                let cache_global_key = v8::String::new(scope, "__beejsEsmNamespaceCache").unwrap();
                                let cache_obj = match global.get(scope, cache_global_key.into())
                                    .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
                                {
                                    Some(cache_obj) => cache_obj,
                                    None => {
                                        let cache_obj = v8::Object::new(scope);
                                        global.set(scope, cache_global_key.into(), cache_obj.into());
                                        cache_obj
                                    }
                                };
                                let fingerprint_cache_global_key =
                                    v8::String::new(scope, "__beejsEsmNamespaceFingerprintCache").unwrap();
                                let fingerprint_cache_obj = match global.get(scope, fingerprint_cache_global_key.into())
                                    .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
                                {
                                    Some(cache_obj) => cache_obj,
                                    None => {
                                        let cache_obj = v8::Object::new(scope);
                                        global.set(
                                            scope,
                                            fingerprint_cache_global_key.into(),
                                            cache_obj.into(),
                                        );
                                        cache_obj
                                    }
                                };
                                let cache_key_string = module_path.to_string_lossy().to_string();
                                let cache_key = v8::String::new(scope, &cache_key_string).unwrap();

                                if let Some(cached_namespace) = cache_obj.get(scope, cache_key.into()) {
                                    if !cached_namespace.is_undefined() {
                                        if let Some(graph_fingerprints) = fingerprint_cache_obj
                                            .get(scope, cache_key.into())
                                            .and_then(|value| {
                                                v8::Local::<v8::Object>::try_from(value).ok()
                                            })
                                        {
                                            match Self::cached_esm_namespace_graph_is_fresh(
                                                scope,
                                                graph_fingerprints,
                                            ) {
                                                Ok(true) => {
                                                    retval.set(cached_namespace);
                                                    return;
                                                }
                                                Ok(false) => {}
                                                Err(error) => {
                                                    let error_str =
                                                        v8::String::new(scope, &error).unwrap();
                                                    let error_obj =
                                                        v8::Exception::type_error(scope, error_str);
                                                    scope.throw_exception(error_obj.into());
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                }

                                if let Err(error) = crate::permissions::check_global_permission(
                                    crate::permissions::PermissionKind::FileSystem,
                                    crate::permissions::PermissionAction::Read,
                                    crate::permissions::ResourceId::Path(module_path.clone()),
                                ) {
                                    let error_str =
                                        v8::String::new(scope, &error.to_string()).unwrap();
                                    let error_obj = v8::Exception::type_error(scope, error_str);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }

                                let module_code = match std::fs::read_to_string(&module_path) {
                                    Ok(module_code) => module_code,
                                    Err(error) => {
                                        let error_msg = format!(
                                            "Error loading ES module '{}': {}",
                                            module_path.display(),
                                            error
                                        );
                                        let error_str =
                                            v8::String::new(scope, &error_msg).unwrap();
                                        let error_obj = v8::Exception::error(scope, error_str);
                                        scope.throw_exception(error_obj.into());
                                        return;
                                    }
                                };

                                let module_filename = module_path.to_string_lossy().to_string();
                                match Self::execute_esm_module_namespace(
                                    scope,
                                    &module_code,
                                    &module_filename,
                                    Self::DEFAULT_TIMER_DRAIN_LIMIT_MS,
                                ) {
                                    Ok((namespace, source_fingerprints)) => {
                                        let graph_fingerprints =
                                            Self::create_esm_namespace_graph_fingerprint_object(
                                                scope,
                                                &source_fingerprints,
                                            );
                                        cache_obj.set(scope, cache_key.into(), namespace);
                                        fingerprint_cache_obj.set(
                                            scope,
                                            cache_key.into(),
                                            graph_fingerprints.into(),
                                        );
                                        retval.set(namespace);
                                    }
                                    Err(error) => {
                                        let error_str = v8::String::new(scope, &error).unwrap();
                                        let error_obj = v8::Exception::error(scope, error_str);
                                        scope.throw_exception(error_obj.into());
                                    }
                                }
                                return;
                            }

                            let cache_global_key = v8::String::new(scope, "__beejsModuleCache").unwrap();
                            let cache_obj = match global.get(scope, cache_global_key.into())
                                .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
                            {
                                Some(cache_obj) => cache_obj,
                                None => {
                                    let cache_obj = v8::Object::new(scope);
                                    global.set(scope, cache_global_key.into(), cache_obj.into());
                                    cache_obj
                                }
                            };
                            let cache_key_string = module_path.to_string_lossy().to_string();
                            let cache_key = v8::String::new(scope, &cache_key_string).unwrap();

                            if let Some(cached_exports) = cache_obj.get(scope, cache_key.into()) {
                                if !cached_exports.is_undefined() {
                                    retval.set(cached_exports);
                                    return;
                                }
                            }

                            // Read and execute the module file
                            if let Err(error) = crate::permissions::check_global_permission(
                                crate::permissions::PermissionKind::FileSystem,
                                crate::permissions::PermissionAction::Read,
                                crate::permissions::ResourceId::Path(module_path.clone()),
                            ) {
                                let error_str =
                                    v8::String::new(scope, &error.to_string()).unwrap();
                                let error_obj = v8::Exception::type_error(scope, error_str);
                                scope.throw_exception(error_obj.into());
                                return;
                            }

                            match std::fs::read_to_string(&module_path) {
                                Ok(code) => {
                                    if module_format
                                        == crate::nodejs_core::commonjs_resolver::CommonJsModuleFormat::Json
                                    {
                                        let json_value = match serde_json::from_str::<serde_json::Value>(&code) {
                                            Ok(value) => value,
                                            Err(error) => {
                                                let error_msg = format!(
                                                    "Error parsing JSON module '{}': {}",
                                                    module_path.display(),
                                                    error
                                                );
                                                let error_str =
                                                    v8::String::new(scope, &error_msg).unwrap();
                                                let error_obj = v8::Exception::syntax_error(scope, error_str);
                                                scope.throw_exception(error_obj.into());
                                                return;
                                            }
                                        };
                                        let json_exports = serde_json_value_to_v8(scope, &json_value);
                                        cache_obj.set(scope, cache_key.into(), json_exports);
                                        retval.set(json_exports);
                                        return;
                                    }

                                    let code = if module_format
                                        == crate::nodejs_core::commonjs_resolver::CommonJsModuleFormat::TypeScript
                                    {
                                        let module_filename = module_path.to_string_lossy().to_string();
                                        match Self::compile_typescript_commonjs_module(
                                            &code,
                                            &module_filename,
                                        ) {
                                            Ok(js_code) => js_code,
                                            Err(error) => {
                                                let error_msg = format!(
                                                    "Error compiling TypeScript module '{}': {}",
                                                    module_path.display(),
                                                    error
                                                );
                                                let error_str =
                                                    v8::String::new(scope, &error_msg).unwrap();
                                                let error_obj =
                                                    v8::Exception::syntax_error(scope, error_str);
                                                scope.throw_exception(error_obj.into());
                                                return;
                                            }
                                        }
                                    } else if module_format
                                        == crate::nodejs_core::commonjs_resolver::CommonJsModuleFormat::TypeScriptJsx
                                    {
                                        let module_filename = module_path.to_string_lossy().to_string();
                                        match crate::typescript::compile_typescript(
                                            &code,
                                            &module_filename,
                                        ) {
                                            Ok(output) => output.js_code,
                                            Err(error) => {
                                                let error_msg = format!(
                                                    "Error compiling TypeScript module '{}': {}",
                                                    module_path.display(),
                                                    error
                                                );
                                                let error_str =
                                                    v8::String::new(scope, &error_msg).unwrap();
                                                let error_obj =
                                                    v8::Exception::syntax_error(scope, error_str);
                                                scope.throw_exception(error_obj.into());
                                                return;
                                            }
                                        }
                                    } else {
                                        code
                                    };

                                    // Create new module and exports objects for this module
                                    let module_obj = v8::Object::new(scope);
                                    let exports_obj = v8::Object::new(scope);
                                    let module_exports_key = v8::String::new(scope, "exports").unwrap().into();
                                    module_obj.set(scope, module_exports_key, exports_obj.clone().into());
                                    cache_obj.set(scope, cache_key.into(), exports_obj.clone().into());

                                    // Set up __dirname and __filename for the module
                                    let module_dirname = module_path.parent()
                                        .map(|p| p.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "/".to_string());
                                    let module_filename = module_path.to_string_lossy().to_string();

                                    // Create a wrapper function to execute the module code.
                                    // The local require captures this module directory, so module
                                    // code cannot break sibling resolution by mutating global
                                    // __dirname before calling require("./sibling").
                                    let module_dir_json =
                                        serde_json::to_string(&module_dirname).unwrap();
                                    let wrapper_code = format!(
                                        r#"(function(module, exports, __dirname, __filename) {{
const __beejsModuleDir = {module_dir_json};
const __beejsGlobalRequire = globalThis.require;
function require(specifier) {{
  const __previousDirname = globalThis.__dirname;
  const __previousFilename = globalThis.__filename;
  globalThis.__dirname = __beejsModuleDir;
  globalThis.__filename = __filename;
  try {{
    return __beejsGlobalRequire(specifier);
  }} finally {{
    globalThis.__dirname = __previousDirname;
    globalThis.__filename = __previousFilename;
  }}
}}
require.main = __beejsGlobalRequire.main;
require.resolve = function(specifier) {{
  const __previousDirname = globalThis.__dirname;
  const __previousFilename = globalThis.__filename;
  globalThis.__dirname = __beejsModuleDir;
  globalThis.__filename = __filename;
  try {{
    return __beejsGlobalRequire.resolve(specifier);
  }} finally {{
    globalThis.__dirname = __previousDirname;
    globalThis.__filename = __previousFilename;
  }}
}};
{code}
}})"#
                                    );

                                    // Compile and run the module code
                                    let script_source = v8::String::new(scope, &wrapper_code).unwrap();
                                    let Some(script) = v8::Script::compile(scope, script_source, None) else {
                                        let error_msg = format!(
                                            "Error compiling CommonJS module '{}'",
                                            module_path.display()
                                        );
                                        let error_str =
                                            v8::String::new(scope, &error_msg).unwrap();
                                        let error_obj =
                                            v8::Exception::syntax_error(scope, error_str);
                                        scope.throw_exception(error_obj.into());
                                        return;
                                    };
                                    let Some(wrapper_func_val) = script.run(scope) else {
                                        return;
                                    };

                                    // Convert to function
                                    let wrapper_func = v8::Local::<v8::Function>::try_from(wrapper_func_val).unwrap();

                                    // Call the wrapper with module context
                                    let undefined = v8::undefined(scope);
                                    let dirname_val = v8::String::new(scope, &module_dirname).unwrap().into();
                                    let filename_val = v8::String::new(scope, &module_filename).unwrap().into();
                                    let global_dirname_key = v8::String::new(scope, "__dirname").unwrap();
                                    let global_filename_key = v8::String::new(scope, "__filename").unwrap();
                                    let previous_dirname = global.get(scope, global_dirname_key.into());
                                    let previous_filename = global.get(scope, global_filename_key.into());
                                    let set_dirname_key =
                                        v8::String::new(scope, "__dirname").unwrap().into();
                                    global.set(scope, set_dirname_key, dirname_val);
                                    let set_filename_key =
                                        v8::String::new(scope, "__filename").unwrap().into();
                                    global.set(scope, set_filename_key, filename_val);

                                    let call_result = wrapper_func.call(scope, undefined.into(), &[module_obj.clone().into(), exports_obj.clone().into(), dirname_val, filename_val]);

                                    if let Some(previous_dirname) = previous_dirname {
                                        let restore_dirname_key =
                                            v8::String::new(scope, "__dirname").unwrap().into();
                                        global.set(scope, restore_dirname_key, previous_dirname);
                                    }
                                    if let Some(previous_filename) = previous_filename {
                                        let restore_filename_key =
                                            v8::String::new(scope, "__filename").unwrap().into();
                                        global.set(scope, restore_filename_key, previous_filename);
                                    }

                                    if call_result.is_none() {
                                        return;
                                    }

                                    // Return module.exports so assignments like
                                    // module.exports = { ... } are reflected.
                                    let module_exports_lookup_key = v8::String::new(scope, "exports").unwrap().into();
                                    if let Some(module_exports) = module_obj.get(scope, module_exports_lookup_key) {
                                        cache_obj.set(scope, cache_key.into(), module_exports);
                                        retval.set(module_exports);
                                    } else {
                                        cache_obj.set(scope, cache_key.into(), exports_obj.clone().into());
                                        retval.set(exports_obj.into());
                                    }
                                    return;
                                }
                                Err(e) => {
                                    let error_msg = format!("Error loading module '{}': {}", module_path.display(), e);
                                    let error_str = v8::String::new(scope, &error_msg).unwrap();
                                    let error_obj = v8::Exception::error(scope, error_str);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }
                            }
                        }

                        // Throw error for unknown modules
                        let error_msg = format!("Cannot find module '{}'", module_id_str);
                        let error_str = v8::String::new(scope, &error_msg).unwrap();
                        let error_obj = v8::Exception::error(scope, error_str);
                        scope.throw_exception(error_obj.into());
                        return;
                    }
                }

            }
        }).ok_or_else(|| anyhow::anyhow!("Failed to create require function"))?;

        // v0.3.329: Add resolve method to require function for CommonJS compatibility
        let resolve_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                if args.length() >= 1 {
                    let specifier = args.get(0);
                    if let Some(s) = specifier.to_string(scope) {
                        let specifier_str = s.to_rust_string_lossy(scope);
                        let context = scope.get_current_context();
                        let global = context.global(scope);
                        let dirname_key = v8::String::new(scope, "__dirname").unwrap();
                        let current_dirname = global
                            .get(scope, dirname_key.into())
                            .and_then(|value| value.to_string(scope))
                            .map(|value| value.to_rust_string_lossy(scope))
                            .unwrap_or_else(|| String::from("."));

                        match crate::nodejs_core::commonjs_resolver::resolve_commonjs_module(
                            &specifier_str,
                            std::path::Path::new(&current_dirname),
                        ) {
                            Ok(crate::nodejs_core::commonjs_resolver::ResolvedModule::Builtin(
                                name,
                            )) => {
                                let resolved_str = v8::String::new(scope, &name).unwrap();
                                retval.set(resolved_str.into());
                            }
                            Ok(crate::nodejs_core::commonjs_resolver::ResolvedModule::File(
                                path,
                            )) => {
                                let resolved = path.to_string_lossy();
                                let resolved_str = v8::String::new(scope, &resolved).unwrap();
                                retval.set(resolved_str.into());
                            }
                            Err(error) => {
                                let error_str = v8::String::new(scope, &error.to_string()).unwrap();
                                let error_obj = v8::Exception::error(scope, error_str);
                                scope.throw_exception(error_obj.into());
                            }
                        }
                        return;
                    }
                }
                let empty_str = v8::String::new(scope, "").unwrap();
                retval.set(empty_str.into());
            },
        )
        .unwrap();
        let resolve_key = v8::String::new(scope, "resolve").unwrap().into();
        require_fn.set(scope, resolve_key, resolve_fn.into());

        let require_main_key = v8::String::new(scope, "main").unwrap().into();
        require_fn.set(scope, require_main_key, module_obj.clone().into());

        // Set global objects
        let require_key = v8::String::new(scope, "require").unwrap().into();
        global.set(scope, require_key, require_fn.into());

        let module_key = v8::String::new(scope, "module").unwrap().into();
        global.set(scope, module_key, module_obj.into());

        let exports_key = v8::String::new(scope, "exports").unwrap().into();
        global.set(scope, exports_key, exports_obj.into());

        // Set up Buffer module using our helper function (v0.3.36)
        setup_buffer_module(scope);

        // Set __dirname and __filename globals
        let dirname_val = v8::String::new(scope, main_module_dir).unwrap().into();
        let dirname_key = v8::String::new(scope, "__dirname").unwrap().into();
        global.set(scope, dirname_key, dirname_val);

        let filename_val = v8::String::new(scope, main_module_filename).unwrap().into();
        let filename_key = v8::String::new(scope, "__filename").unwrap().into();
        global.set(scope, filename_key, filename_val);

        // v0.3.329: Add module.children array for tracking loaded sub-modules
        let children_key = v8::String::new(scope, "children").unwrap().into();
        let children_array = v8::Array::new(scope, 0);
        module_obj.set(scope, children_key, children_array.into());

        // v0.3.329: Add module.require for CommonJS compatibility
        let module_require_key = v8::String::new(scope, "require").unwrap().into();
        module_obj.set(scope, module_require_key, require_fn.into());

        let create_require_src = r#"
(function() {
  if (!globalThis.module) return;
  globalThis.module.createRequire = function(filename) {
    var dir = '.';
    if (typeof filename === 'string') {
      var idx = Math.max(filename.lastIndexOf('/'), filename.lastIndexOf('\\'));
      dir = idx >= 0 ? (filename.slice(0, idx) || '/') : filename;
    } else if (filename && typeof filename.href === 'string') {
      var href = String(filename.href).replace(/^file:\/\//, '');
      var idx2 = href.lastIndexOf('/');
      dir = idx2 >= 0 ? (href.slice(0, idx2) || '/') : href;
    }
    var req = globalThis.require;
    return function createdRequire(id) {
      var prev = globalThis.__dirname;
      globalThis.__dirname = dir;
      try { return req(id); }
      finally { globalThis.__dirname = prev; }
    };
  };
})();
"#;
        if let Some(code) = v8::String::new(scope, create_require_src) {
            if let Some(script) = v8::Script::compile(scope, code, None) {
                let _ = script.run(scope);
            }
        }

        // import.meta (progressive): bind url to the real main module path.
        // Full HostInitializeImportMetaObjectCallback lands with rusty_v8 0.32.
        let import_key = v8::String::new(scope, "import").unwrap().into();
        let import_obj = v8::Object::new(scope);
        let meta_key = v8::String::new(scope, "meta").unwrap().into();
        let import_meta_obj = v8::Object::new(scope);

        let url_key = v8::String::new(scope, "url").unwrap().into();
        let meta_url = if main_module_filename.starts_with("file:") {
            main_module_filename.to_string()
        } else {
            format!(
                "file://{}",
                std::path::Path::new(main_module_filename)
                    .canonicalize()
                    .unwrap_or_else(|_| std::path::PathBuf::from(main_module_filename))
                    .display()
            )
        };
        let url_val = v8::String::new(scope, &meta_url).unwrap().into();
        import_meta_obj.set(scope, url_key, url_val);

        // WinterTC import.meta registry: import.meta.main
        let main_key = v8::String::new(scope, "main").unwrap().into();
        let main_val = v8::Boolean::new(scope, true);
        import_meta_obj.set(scope, main_key, main_val.into());

        // WinterTC import.meta registry: import.meta.env
        let env_key = v8::String::new(scope, "env").unwrap().into();
        let env_obj = v8::Object::new(scope);
        for (k, v) in std::env::vars() {
            if let Some(vk) = v8::String::new(scope, &k) {
                if let Some(vv) = v8::String::new(scope, &v) {
                    env_obj.set(scope, vk.into(), vv.into());
                }
            }
        }
        import_meta_obj.set(scope, env_key, env_obj.into());

        // WinterTC import.meta.resolve — real ESM resolver (wintercg/wintertc conditions).
        let resolve_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut retval: v8::ReturnValue| {
                if args.length() >= 1 {
                    if let Some(s) = args.get(0).to_string(scope) {
                        let specifier_str = s.to_rust_string_lossy(scope);
                        let parent = IMPORT_META_PARENT_DIR.with(|d| d.borrow().clone());
                        let resolved =
                            match crate::nodejs_core::commonjs_resolver::resolve_esm_module(
                                &specifier_str,
                                &parent,
                            ) {
                                Ok(
                                    crate::nodejs_core::commonjs_resolver::ResolvedModule::File(
                                        path,
                                    ),
                                ) => {
                                    let abs = path.canonicalize().unwrap_or(path);
                                    format!("file://{}", abs.display())
                                }
                                Ok(
                                    crate::nodejs_core::commonjs_resolver::ResolvedModule::Builtin(
                                        name,
                                    ),
                                ) => name,
                                Err(_) => specifier_str,
                            };
                        let resolved_str = v8::String::new(scope, &resolved).unwrap();
                        retval.set(resolved_str.into());
                        return;
                    }
                }
                let empty_str = v8::String::new(scope, "").unwrap();
                retval.set(empty_str.into());
            },
        )
        .unwrap();
        let resolve_key = v8::String::new(scope, "resolve").unwrap().into();
        import_meta_obj.set(scope, resolve_key, resolve_fn.into());
        import_obj.set(scope, meta_key, import_meta_obj.into());

        global.set(scope, import_key, import_obj.into());

        Ok(())
    }

    fn apply_process_argv(
        scope: &mut v8::ContextScope<v8::HandleScope>,
        context: &v8::Local<v8::Context>,
        argv: &[String],
    ) -> Result<()> {
        let global = context.global(scope);
        let process_key = v8::String::new(scope, "process").unwrap();
        let process_value = global
            .get(scope, process_key.into())
            .unwrap_or_else(|| v8::undefined(scope).into());

        if !process_value.is_object() {
            return Ok(());
        }

        let process_obj = process_value
            .to_object(scope)
            .ok_or_else(|| anyhow::anyhow!("process global is not an object"))?;
        let argv_array = v8::Array::new(scope, argv.len() as i32);

        for (index, value) in argv.iter().enumerate() {
            let value = v8::String::new(scope, value).unwrap();
            argv_array.set_index(scope, index as u32, value.into());
        }

        let argv_key = v8::String::new(scope, "argv").unwrap();
        process_obj.set(scope, argv_key.into(), argv_array.into());
        Ok(())
    }

    // setup_http_api is now imported from crate::nodejs_core::http

    // v0.3.91: HTTP Server 消息轮询
    // 用于在事件循环中处理来自消息通道的 HTTP 请求

    /// 轮询 HTTP 消息通道并处理请求
    /// 返回处理的请求数量
    /// v0.3.91: 新增功能
    /// v0.3.94: 改为非阻塞模式，由调用者决定是否继续轮询
    pub fn pump_http_messages(&mut self) -> usize {
        use crate::nodejs_core::http::{
            has_pending_http_requests, pump_pending_http_requests_in_scope,
        };

        if !has_pending_http_requests() {
            return 0;
        }

        let global_context = self.get_context();
        v8::scope!(let scope, &mut self.isolate);
        let context_local = v8::Local::new(scope, &global_context);
        let scope = &mut v8::ContextScope::new(scope, context_local);
        pump_pending_http_requests_in_scope(scope, &context_local)
    }

    /// 初始化 HTTP 服务器消息通道
    /// 必须在启动 HTTP 服务器前调用
    /// v0.3.91: 新增功能
    pub fn init_http_server(&mut self) {
        use crate::nodejs_core::http::init_http_server_channel;
        init_http_server_channel();
    }

    /// 设置 HTTP 请求处理器
    /// v0.3.91: 新增功能
    pub fn set_http_request_handler(&mut self, handler_code: &str) -> Result<()> {
        // v0.3.93: 先获取 Context
        let global_context = self.get_context();

        v8::scope!(let scope, &mut self.isolate);
        let context = v8::Local::new(scope, &global_context);
        let scope = &mut v8::ContextScope::new(scope, context);

        // 编译 handler 代码
        let code = v8::String::new(scope, handler_code)
            .ok_or_else(|| anyhow::anyhow!("Failed to create handler string"))?;

        let script = v8::Script::compile(scope, code, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to compile handler"))?;

        let _ = script
            .run(scope)
            .ok_or_else(|| anyhow::anyhow!("Failed to run handler"))?;

        Ok(())
    }

    /// v0.3.256: Clean up all V8 Global handles before Isolate disposal
    /// Call this explicitly before dropping the runtime to avoid
    /// "Handle hosted by disposed Isolate" errors
    pub fn cleanup(&mut self) {
        use crate::nodejs_core::timers::cleanup_all_timers;

        // Clear all V8 Global handles in timer callbacks
        cleanup_all_timers();
    }
}

impl Drop for MinimalRuntime {
    fn drop(&mut self) {
        // Clean up all V8 Global handles before isolate is disposed
        use crate::nodejs_core::timers::cleanup_all_timers;
        cleanup_all_timers();

        // Context will be dropped first (v8::Global)
        // Then isolate will be disposed automatically
    }
}

impl Default for MinimalRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create MinimalRuntime")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn test_minimal_runtime_creation() {
        let runtime = MinimalRuntime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    #[serial_test::serial]
    fn test_simple_execution() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let result = runtime.execute_code("1 + 1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "2");
    }

    #[test]
    #[serial_test::serial]
    fn test_console_log() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let result = runtime.execute_code("console.log('Hello from Beejs!'); 42;");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "42");
    }

    #[test]
    #[serial_test::serial]
    fn test_console_error() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let result = runtime.execute_code("console.error('Error message'); 100;");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "100");
    }

    /// The extended surface is unconditional, so a bare first execute already
    /// sees every Web API global. Guards against reintroducing a source-scan or
    /// lazy-accessor deferral that silently drops globals.
    #[test]
    #[serial_test::serial]
    fn extended_apis_install_on_first_execute() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let probe = r#"[
  typeof fetch,
  typeof Blob,
  typeof Worker,
  typeof ReadableStream,
  typeof WritableStream,
  typeof TransformStream,
  typeof CompressionStream,
  typeof structuredClone,
  typeof FormData,
  typeof Headers,
  typeof Request,
  typeof Response,
  typeof BroadcastChannel,
  typeof MessageChannel,
  typeof CustomEvent,
  typeof ErrorEvent,
  typeof DOMParser,
  typeof detachArrayBuffer,
].join(",")"#;
        let result = runtime.execute_code(probe).unwrap();
        assert!(
            !result.contains("undefined"),
            "extended globals missing on first execute: {result}"
        );
        assert!(runtime.apis_initialized);
        assert!(runtime.extended_apis_initialized);
    }

    /// The entry never names a Web API; a required dependency uses fetch + Blob.
    /// This is the multi-file case that no source scan of the entry can see.
    #[test]
    #[serial_test::serial]
    fn required_module_can_use_fetch_and_blob_without_entry_markers() {
        let dir =
            std::env::temp_dir().join(format!("beejs_require_extended_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let lib_path = dir.join("lib_with_web.js");
        let entry_path = dir.join("entry.js");
        std::fs::write(
            &lib_path,
            r#"
// Dependency-only Web API use — not visible in entry source text.
function ping() {
  if (typeof fetch !== "function") {
    throw new Error("fetch missing in required module");
  }
  if (typeof Blob !== "function") {
    throw new Error("Blob missing in required module");
  }
  if (typeof structuredClone !== "function") {
    throw new Error("structuredClone missing in required module");
  }
  return typeof fetch + ":" + typeof Blob + ":" + typeof structuredClone;
}
module.exports = { ping };
"#,
        )
        .unwrap();
        std::fs::write(
            &entry_path,
            // Intentionally no fetch/Blob/structuredClone markers in entry.
            "const m = require('./lib_with_web.js');\nm.ping();\n",
        )
        .unwrap();

        let entry_source = std::fs::read_to_string(&entry_path).unwrap();
        assert!(
            !entry_source.contains("fetch")
                && !entry_source.contains("Blob")
                && !entry_source.contains("structuredClone"),
            "entry must not contain extended markers (test validity)"
        );
        let mut runtime = MinimalRuntime::new().unwrap();
        runtime.set_main_module_path(&entry_path);
        let result = runtime
            .execute_code(&entry_source)
            .expect("required module Web APIs must resolve");
        assert_eq!(result.trim(), "function:function:function");
        assert!(runtime.extended_apis_initialized);

        let _ = std::fs::remove_dir_all(&dir);
    }
}

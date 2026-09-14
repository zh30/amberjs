// TextEncoder/TextDecoder API implementation
/// Provides text encoding and decoding functionality per Web standards
use anyhow::Result;
use base64::Engine;
use rusty_v8 as v8;
/// Setup TextEncoder and TextDecoder APIs in V8 context
pub fn setup_encoding_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global: _ = context.global(scope);
    // Setup TextEncoder constructor
    let encoder_template: _ = v8::FunctionTemplate::new(scope, text_encoder_constructor);
    let encoder_constructor: _ = encoder_template.get_function(scope).unwrap();
    let encoder_key: _ = v8::String::new(scope, "TextEncoder").unwrap();
    global.set(scope, encoder_key.into(), encoder_constructor.into());
    // Setup TextDecoder constructor
    let decoder_template: _ = v8::FunctionTemplate::new(scope, text_decoder_constructor);
    let decoder_constructor: _ = decoder_template.get_function(scope).unwrap();
    let decoder_key: _ = v8::String::new(scope, "TextDecoder").unwrap();
    global.set(scope, decoder_key.into(), decoder_constructor.into());
    // Setup atob (base64 decode)
    let atob_template: _ = v8::FunctionTemplate::new(scope, atob_callback);
    let atob_func: _ = atob_template.get_function(scope).unwrap();
    let atob_key: _ = v8::String::new(scope, "atob").unwrap();
    global.set(scope, atob_key.into(), atob_func.into());
    // Setup btoa (base64 encode)
    let btoa_template: _ = v8::FunctionTemplate::new(scope, btoa_callback);
    let btoa_func: _ = btoa_template.get_function(scope).unwrap();
    let btoa_key: _ = v8::String::new(scope, "btoa").unwrap();
    global.set(scope, btoa_key.into(), btoa_func.into());
    Ok(())
}
/// TextEncoder constructor callback
fn text_encoder_constructor(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let encoder_obj: _ = v8::Object::new(scope);
    // Set encoding property (always "utf-8")
    let encoding_key: _ = v8::String::new(scope, "encoding").unwrap();
    let encoding_val: _ = v8::String::new(scope, "utf-8").unwrap();
    encoder_obj.set(scope, encoding_key.into(), encoding_val.into());
    // Add encode method
    let encode_key: _ = v8::String::new(scope, "encode").unwrap();
    let encode_template: _ = v8::FunctionTemplate::new(scope, text_encoder_encode);
    let encode_func: _ = encode_template.get_function(scope).unwrap();
    encoder_obj.set(scope, encode_key.into(), encode_func.into());
    // Add encodeInto method
    let encode_into_key: _ = v8::String::new(scope, "encodeInto").unwrap();
    let encode_into_template: _ = v8::FunctionTemplate::new(scope, text_encoder_encode_into);
    let encode_into_func: _ = encode_into_template.get_function(scope).unwrap();
    encoder_obj.set(scope, encode_into_key.into(), encode_into_func.into());
    retval.set(encoder_obj.into());
}
/// TextEncoder.encode() method
fn text_encoder_encode(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let input: _ = args.get(0);
    let input_str: _ = if input.is_undefined() || input.is_null() {
        String::new()
    } else {
        input
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_default()
    };
    // Convert string to UTF-8 bytes
    let bytes: _ = input_str.as_bytes();
    // Create Uint8Array
    let array_buffer: _ = v8::ArrayBuffer::new(scope, bytes.len());
    let backing_store: _ = array_buffer.get_backing_store();
    // Copy bytes to backing store
    if let Some(data) = backing_store.data() {
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.as_ptr() as *mut u8, bytes.len());
        }
    }
    let uint8_array: _ = v8::Uint8Array::new(scope, array_buffer, 0, bytes.len()).unwrap();
    retval.set(uint8_array.into());
}
/// TextEncoder.encodeInto() method
fn text_encoder_encode_into(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let input: _ = args.get(0);
    let destination: _ = args.get(1);
    // Validate destination is Uint8Array
    if !destination.is_uint8_array() {
        let error: _ =
            v8::String::new(scope, "encodeInto: destination must be Uint8Array").unwrap();
        let error_obj: _ = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }
    let input_str: _ = input
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let dest_array: _ = v8::Local::<v8::Uint8Array>::try_from(destination).unwrap();
    let dest_len: _ = dest_array.byte_length();
    let mut encoded_bytes = Vec::new();
    let mut read = 0usize;
    let mut written = 0usize;

    for ch in input_str.chars() {
        let mut buffer = [0u8; 4];
        let chunk = ch.encode_utf8(&mut buffer).as_bytes();
        if written + chunk.len() > dest_len {
            break;
        }
        encoded_bytes.extend_from_slice(chunk);
        written += chunk.len();
        read += ch.len_utf16();
    }

    if written > 0 {
        let dest_buffer = dest_array.buffer(scope).unwrap();
        let byte_offset = dest_array.byte_offset();
        let backing_store = dest_buffer.get_backing_store();
        for (i, byte) in encoded_bytes.iter().enumerate() {
            backing_store[byte_offset + i].set(*byte);
        }
    }

    let result: _ = v8::Object::new(scope);
    let read_key: _ = v8::String::new(scope, "read").unwrap();
    let read_val: _ = v8::Number::new(scope, read as f64);
    result.set(scope, read_key.into(), read_val.into());
    let written_key: _ = v8::String::new(scope, "written").unwrap();
    let written_val: _ = v8::Number::new(scope, written as f64);
    result.set(scope, written_key.into(), written_val.into());
    retval.set(result.into());
}
/// TextDecoder constructor callback
fn text_decoder_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Get encoding (default "utf-8")
    let encoding: _ = args.get(0);
    let encoding_str: _ = if encoding.is_undefined() || encoding.is_null() {
        "utf-8".to_string()
    } else {
        encoding
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope).to_lowercase())
            .unwrap_or_else(|| "utf-8".to_string())
    };
    // Validate encoding
    let valid_encodings: _ = ["utf-8", "utf8", "unicode-1-1-utf-8"];
    let normalized_encoding: _ = if valid_encodings.contains(&encoding_str.as_str()) {
        "utf-8"
    } else {
        // For now, only support UTF-8
        let error: _ = v8::String::new(
            scope,
            &format!("TextDecoder: unsupported encoding '{}'", encoding_str),
        )
        .unwrap();
        let error_obj: _ = v8::Exception::range_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    };
    let decoder_obj: _ = v8::Object::new(scope);
    // Set encoding property
    let encoding_key: _ = v8::String::new(scope, "encoding").unwrap();
    let encoding_val: _ = v8::String::new(scope, normalized_encoding).unwrap();
    decoder_obj.set(scope, encoding_key.into(), encoding_val.into());
    let mut fatal = false;
    let mut ignore_bom = false;
    if args.length() >= 2 {
        let options = args.get(1);
        if let Ok(options_obj) = v8::Local::<v8::Object>::try_from(options) {
            let fatal_key: _ = v8::String::new(scope, "fatal").unwrap();
            fatal = options_obj
                .get(scope, fatal_key.into())
                .map(|value| value.to_boolean(scope).is_true())
                .unwrap_or(false);
            let ignore_bom_key: _ = v8::String::new(scope, "ignoreBOM").unwrap();
            ignore_bom = options_obj
                .get(scope, ignore_bom_key.into())
                .map(|value| value.to_boolean(scope).is_true())
                .unwrap_or(false);
        }
    }
    // Set fatal property (from options)
    let fatal_key: _ = v8::String::new(scope, "fatal").unwrap();
    let fatal_val: _ = v8::Boolean::new(scope, fatal);
    decoder_obj.set(scope, fatal_key.into(), fatal_val.into());
    // Set ignoreBOM property
    let ignore_bom_key: _ = v8::String::new(scope, "ignoreBOM").unwrap();
    let ignore_bom_val: _ = v8::Boolean::new(scope, ignore_bom);
    decoder_obj.set(scope, ignore_bom_key.into(), ignore_bom_val.into());
    // Add decode method
    let decode_key: _ = v8::String::new(scope, "decode").unwrap();
    let decode_template: _ = v8::FunctionTemplate::new(scope, text_decoder_decode);
    let decode_func: _ = decode_template.get_function(scope).unwrap();
    decoder_obj.set(scope, decode_key.into(), decode_func.into());
    retval.set(decoder_obj.into());
}
/// TextDecoder.decode() method
fn text_decoder_decode(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let input: _ = args.get(0);
    let this_obj = args.this();
    let fatal_key: _ = v8::String::new(scope, "fatal").unwrap();
    let fatal = this_obj
        .get(scope, fatal_key.into())
        .map(|value| value.to_boolean(scope).is_true())
        .unwrap_or(false);
    let ignore_bom_key: _ = v8::String::new(scope, "ignoreBOM").unwrap();
    let ignore_bom = this_obj
        .get(scope, ignore_bom_key.into())
        .map(|value| value.to_boolean(scope).is_true())
        .unwrap_or(false);
    // Handle undefined/null input
    if input.is_undefined() || input.is_null() {
        let empty: _ = v8::String::new(scope, "").unwrap();
        retval.set(empty.into());
        return;
    }
    // Get bytes from input (Uint8Array, ArrayBuffer, etc.)
    let bytes: Vec<u8> = if input.is_uint8_array() {
        let array: v8::Local<v8::Uint8Array> =
            v8::Local::<v8::Uint8Array>::try_from(input).unwrap();
        let len = array.byte_length();
        let byte_offset = array.byte_offset();
        let array_buffer = array.buffer(scope).unwrap();
        let backing_store = array_buffer.get_backing_store();
        let mut buffer = vec![0u8; len];
        if let Some(data) = backing_store.data() {
            unsafe {
                let src_ptr = (data.as_ptr() as *const u8).add(byte_offset);
                std::ptr::copy_nonoverlapping(src_ptr, buffer.as_mut_ptr(), len);
            }
        }
        buffer
    } else if input.is_array_buffer() {
        let array_buffer: _ = v8::Local::<v8::ArrayBuffer>::try_from(input).unwrap();
        let backing_store: _ = array_buffer.get_backing_store();
        let len: _ = backing_store.byte_length();
        let mut buffer = vec![0u8; len];
        if let Some(ptr) = backing_store.data() {
            unsafe {
                std::ptr::copy_nonoverlapping(ptr.as_ptr() as *const u8, buffer.as_mut_ptr(), len);
            }
        }
        buffer
    } else if input.is_array_buffer_view() {
        let view: _ = v8::Local::<v8::ArrayBufferView>::try_from(input).unwrap();
        let len: _ = view.byte_length();
        let mut buffer = vec![0u8; len];
        view.copy_contents(&mut buffer);
        buffer
    } else {
        let error: _ =
            v8::String::new(scope, "decode: input must be ArrayBuffer or TypedArray").unwrap();
        let error_obj: _ = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    };
    // Decode UTF-8 bytes to string
    let encoding_rs_encoding = encoding_rs::Encoding::for_label(b"utf-8").unwrap();
    let decoded = if fatal {
        match encoding_rs_encoding.decode_without_bom_handling_and_without_replacement(&bytes) {
            Some(decoded) => {
                let mut decoded = decoded.into_owned();
                if !ignore_bom && decoded.starts_with('\u{feff}') {
                    decoded.remove(0);
                }
                decoded
            }
            None => {
                let error: _ =
                    v8::String::new(scope, "The encoded data was not valid UTF-8").unwrap();
                let error_obj: _ = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
        }
    } else if ignore_bom {
        encoding_rs_encoding
            .decode_without_bom_handling(&bytes)
            .0
            .into_owned()
    } else {
        encoding_rs_encoding.decode(&bytes).0.into_owned()
    };
    let result: _ = v8::String::new(scope, &decoded).unwrap();
    retval.set(result.into());
}
/// atob - decode base64 string
fn atob_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let input: _ = args.get(0);
    if input.is_undefined() {
        let error: _ = v8::String::new(scope, "atob: input is required").unwrap();
        let error_obj: _ = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }
    let encoded: _ = input
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    // Use base64 decoding
    use base64::{engine::general_purpose::STANDARD, Engine};
    match STANDARD.decode(&encoded) {
        Ok(bytes) => {
            // Convert bytes to string (treating as Latin-1)
            let decoded: String = bytes.iter().map(|&b| b as char).collect();
            let result: _ = v8::String::new(scope, &decoded).unwrap();
            retval.set(result.into());
        }
        Err(_) => {
            let error: _ = v8::String::new(scope, "atob: invalid base64 string").unwrap();
            let error_obj: _ = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
        }
    }
}
/// btoa - encode to base64 string
fn btoa_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let input: _ = args.get(0);
    if input.is_undefined() {
        let error: _ = v8::String::new(scope, "btoa: input is required").unwrap();
        let error_obj: _ = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }
    let to_encode: _ = input
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    // Check for non-Latin1 characters
    for c in to_encode.chars() {
        if c as u32 > 255 {
            let error: _ = v8::String::new(
                scope,
                "btoa: string contains characters outside Latin-1 range",
            )
            .unwrap();
            let error_obj: _ = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
    }
    // Convert to bytes (Latin-1 encoding)
    let bytes: Vec<u8> = to_encode.chars().map(|c| c as u8).collect();
    // Encode to base64
    let encoded: _ = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let result: _ = v8::String::new(scope, &encoded).unwrap();
    retval.set(result.into());
}
#[cfg(test)]
mod tests {
    use base64::Engine;

    #[test]
    fn test_base64_encode_decode() {
        let original: _ = "Hello, World!";
        let encoded: _ = base64::engine::general_purpose::STANDARD.encode(original);
        let decoded_bytes: _ = base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .unwrap();
        let decoded: _ = String::from_utf8(decoded_bytes).unwrap();
        assert_eq!(original, decoded);
    }
}

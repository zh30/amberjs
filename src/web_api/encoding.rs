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
fn throw_type_error(scope: &mut v8::PinScope, message: &str) {
    let Some(message) = v8::String::new(scope, message) else {
        return;
    };
    scope.throw_exception(v8::Exception::type_error(scope, message));
}

fn require_construct_call(
    scope: &mut v8::PinScope,
    args: &v8::FunctionCallbackArguments,
    name: &str,
) -> bool {
    if args.is_construct_call() {
        return true;
    }
    throw_type_error(
        scope,
        &format!("{name} constructor must be called with new"),
    );
    false
}

/// `encode()` omits the argument as the empty string. An explicit `null` is the string `"null"`.
fn usv_string_argument(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> String {
    if value.is_undefined() {
        return String::new();
    }
    value
        .to_string(scope)
        .map(|text| text.to_rust_string_lossy(scope))
        .unwrap_or_default()
}

fn decoder_private<'a>(scope: &mut v8::PinScope<'a, '_>, name: &str) -> v8::Local<'a, v8::Private> {
    let name = v8::String::new(scope, name).unwrap();
    v8::Private::for_api(scope, Some(name))
}

fn read_pending(scope: &mut v8::PinScope, decoder: v8::Local<v8::Object>) -> Vec<u8> {
    let key = decoder_private(scope, "amber.textdecoder.pending");
    let Some(value) = decoder.get_private(scope, key) else {
        return Vec::new();
    };
    if !value.is_uint8_array() {
        return Vec::new();
    }
    let Ok(array) = v8::Local::<v8::Uint8Array>::try_from(value) else {
        return Vec::new();
    };
    let len = array.byte_length();
    let mut buffer = vec![0u8; len];
    if len > 0 {
        let copied = array.copy_contents(&mut buffer);
        buffer.truncate(copied);
    }
    buffer
}

fn write_pending(scope: &mut v8::PinScope, decoder: v8::Local<v8::Object>, bytes: &[u8]) {
    let key = decoder_private(scope, "amber.textdecoder.pending");
    if bytes.is_empty() {
        let _ = decoder.set_private(scope, key, v8::undefined(scope).into());
        return;
    }
    let array_buffer = v8::ArrayBuffer::new(scope, bytes.len());
    let backing_store = array_buffer.get_backing_store();
    if let Some(data) = backing_store.data() {
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.as_ptr() as *mut u8, bytes.len());
        }
    }
    let Some(array) = v8::Uint8Array::new(scope, array_buffer, 0, bytes.len()) else {
        return;
    };
    let _ = decoder.set_private(scope, key, array.into());
}

fn bom_seen(scope: &mut v8::PinScope, decoder: v8::Local<v8::Object>) -> bool {
    let key = decoder_private(scope, "amber.textdecoder.bom");
    decoder
        .get_private(scope, key)
        .map(|value| value.is_true())
        .unwrap_or(false)
}

fn set_bom_seen(scope: &mut v8::PinScope, decoder: v8::Local<v8::Object>, seen: bool) {
    let key = decoder_private(scope, "amber.textdecoder.bom");
    let value = v8::Boolean::new(scope, seen);
    let _ = decoder.set_private(scope, key, value.into());
}

/// Bytes of an incomplete UTF-8 sequence at the end of `bytes`, if any.
fn incomplete_utf8_tail(bytes: &[u8]) -> usize {
    if bytes.is_empty() {
        return 0;
    }
    let mut index = bytes.len() - 1;
    let mut continuations = 0usize;
    while continuations < 3 && bytes[index] & 0b1100_0000 == 0b1000_0000 {
        if index == 0 {
            return 0;
        }
        index -= 1;
        continuations += 1;
    }
    let lead = bytes[index];
    let needed = if lead & 0b1000_0000 == 0 {
        1
    } else if lead & 0b1110_0000 == 0b1100_0000 {
        2
    } else if lead & 0b1111_0000 == 0b1110_0000 {
        3
    } else if lead & 0b1111_1000 == 0b1111_0000 {
        4
    } else {
        return 0;
    };
    let have = bytes.len() - index;
    if have < needed {
        have
    } else {
        0
    }
}

fn read_bytes(scope: &mut v8::PinScope, input: v8::Local<v8::Value>) -> Result<Vec<u8>, ()> {
    if input.is_uint8_array() {
        let array = v8::Local::<v8::Uint8Array>::try_from(input).map_err(|_| ())?;
        let len = array.byte_length();
        let byte_offset = array.byte_offset();
        let array_buffer = array.buffer(scope).ok_or(())?;
        let backing_store = array_buffer.get_backing_store();
        let mut buffer = vec![0u8; len];
        if let Some(data) = backing_store.data() {
            unsafe {
                let src_ptr = (data.as_ptr() as *const u8).add(byte_offset);
                std::ptr::copy_nonoverlapping(src_ptr, buffer.as_mut_ptr(), len);
            }
        }
        return Ok(buffer);
    }
    if input.is_array_buffer() {
        let array_buffer = v8::Local::<v8::ArrayBuffer>::try_from(input).map_err(|_| ())?;
        let backing_store = array_buffer.get_backing_store();
        let len = backing_store.byte_length();
        let mut buffer = vec![0u8; len];
        if let Some(ptr) = backing_store.data() {
            unsafe {
                std::ptr::copy_nonoverlapping(ptr.as_ptr() as *const u8, buffer.as_mut_ptr(), len);
            }
        }
        return Ok(buffer);
    }
    if input.is_array_buffer_view() {
        let view = v8::Local::<v8::ArrayBufferView>::try_from(input).map_err(|_| ())?;
        let len = view.byte_length();
        let mut buffer = vec![0u8; len];
        view.copy_contents(&mut buffer);
        return Ok(buffer);
    }
    Err(())
}

/// TextEncoder constructor callback
fn text_encoder_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if !require_construct_call(scope, &args, "TextEncoder") {
        return;
    }
    let encoder_obj = args.this();
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
    let input_str = usv_string_argument(scope, args.get(0));
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
    let input_str = usv_string_argument(scope, input);
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
    if !require_construct_call(scope, &args, "TextDecoder") {
        return;
    }
    let decoder_obj = args.this();
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
    let input = args.get(0);
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
    let mut stream = false;
    if args.length() >= 2 {
        if let Ok(options) = v8::Local::<v8::Object>::try_from(args.get(1)) {
            let stream_key = v8::String::new(scope, "stream").unwrap();
            stream = options
                .get(scope, stream_key.into())
                .map(|value| value.to_boolean(scope).is_true())
                .unwrap_or(false);
        }
    }
    let mut pending = read_pending(scope, this_obj);
    let incoming = if input.is_undefined() || input.is_null() {
        Vec::new()
    } else {
        match read_bytes(scope, input) {
            Ok(bytes) => bytes,
            Err(()) => {
                throw_type_error(scope, "decode: input must be ArrayBuffer or TypedArray");
                return;
            }
        }
    };
    pending.extend(incoming);
    let tail = if stream {
        incomplete_utf8_tail(&pending)
    } else {
        0
    };
    let split = pending.len() - tail;
    let body = pending[..split].to_vec();
    let kept = pending[split..].to_vec();
    let already_saw_bom = bom_seen(scope, this_obj);
    let mut decode_from = body.as_slice();
    let mut mark_bom = already_saw_bom;
    if !ignore_bom && !already_saw_bom && decode_from.starts_with(&[0xEF, 0xBB, 0xBF]) {
        decode_from = &decode_from[3..];
        mark_bom = true;
    } else if !already_saw_bom && (!decode_from.is_empty() || !stream) {
        mark_bom = true;
    }
    let utf8 = encoding_rs::Encoding::for_label(b"utf-8").unwrap();
    let decoded = if fatal {
        match utf8.decode_without_bom_handling_and_without_replacement(decode_from) {
            Some(decoded) => decoded.into_owned(),
            None => {
                throw_type_error(scope, "The encoded data was not valid UTF-8");
                return;
            }
        }
    } else {
        utf8.decode_without_bom_handling(decode_from).0.into_owned()
    };
    write_pending(scope, this_obj, &kept);
    set_bom_seen(scope, this_obj, mark_bom);
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

// Blob API implementation per Web standard
// Provides binary data container with File API support

use anyhow::Result;
use rusty_v8 as v8;

fn append_blob_part(scope: &mut v8::PinScope, part: v8::Local<v8::Value>, data: &mut Vec<u8>) {
    if part.is_string() {
        let part_str = part.to_string(scope).unwrap().to_rust_string_lossy(scope);
        data.extend_from_slice(part_str.as_bytes());
    } else if let Ok(array_buffer) = v8::Local::<v8::ArrayBuffer>::try_from(part) {
        let byte_len = array_buffer.byte_length();
        if byte_len > 0 {
            let backing_store = array_buffer.get_backing_store();
            let src_ptr = backing_store
                .data()
                .map(|p| p.as_ptr() as *const u8)
                .unwrap_or(std::ptr::null());
            if !src_ptr.is_null() {
                let bytes = unsafe { std::slice::from_raw_parts(src_ptr, byte_len) };
                data.extend_from_slice(bytes);
            } else {
                data.resize(data.len() + byte_len, 0);
            }
        }
    } else if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(part) {
        let byte_len = view.byte_length();
        if byte_len > 0 {
            let mut bytes = vec![0u8; byte_len];
            view.copy_contents(&mut bytes);
            data.extend_from_slice(&bytes);
        }
    }
}

fn array_buffer_from_bytes<'scope>(
    scope: &mut v8::PinScope<'scope, '_>,
    bytes: &[u8],
) -> v8::Local<'scope, v8::ArrayBuffer> {
    let array_buffer = v8::ArrayBuffer::new(scope, bytes.len());
    if !bytes.is_empty() {
        let backing_store = array_buffer.get_backing_store();
        let ptr = backing_store
            .data()
            .map(|p| p.as_ptr() as *mut u8)
            .unwrap_or(std::ptr::null_mut());
        if !ptr.is_null() {
            let slice = unsafe { std::slice::from_raw_parts_mut(ptr, bytes.len()) };
            slice.copy_from_slice(bytes);
        }
    }
    array_buffer
}

fn bytes_from_array_buffer_value(value: v8::Local<v8::Value>) -> Option<Vec<u8>> {
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
    Some(unsafe { std::slice::from_raw_parts(ptr, len).to_vec() })
}

fn blob_object_bytes(scope: &mut v8::PinScope, object: v8::Local<v8::Object>) -> Option<Vec<u8>> {
    let bytes_key = v8::String::new(scope, "blobBytes")?;
    object
        .get(scope, bytes_key.into())
        .and_then(bytes_from_array_buffer_value)
}

fn set_blob_data_properties(scope: &mut v8::PinScope, object: v8::Local<v8::Object>, data: &[u8]) {
    let bytes_key = v8::String::new(scope, "blobBytes").unwrap();
    let bytes_buffer = array_buffer_from_bytes(scope, data);
    object.set(scope, bytes_key.into(), bytes_buffer.into());

    let data_str = String::from_utf8_lossy(data);
    let data_key = v8::String::new(scope, "blobData").unwrap();
    let data_v8 = v8::String::new(scope, &data_str).unwrap();
    object.set(scope, data_key.into(), data_v8.into());
}

/// Setup Blob and File APIs in V8 context
pub fn setup_blob_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global: _ = context.global(scope);
    // Setup Blob constructor
    let blob_template: _ = v8::FunctionTemplate::new(scope, blob_constructor);
    let blob_constructor: _ = blob_template.get_function(scope).unwrap();
    let blob_key: _ = v8::String::new(scope, "Blob").unwrap();
    global.set(scope, blob_key.into(), blob_constructor.into());
    // Setup File constructor
    let file_template: _ = v8::FunctionTemplate::new(scope, file_constructor);
    let file_constructor: _ = file_template.get_function(scope).unwrap();
    let file_key: _ = v8::String::new(scope, "File").unwrap();
    global.set(scope, file_key.into(), file_constructor.into());
    let file_proto_key = v8::String::new(scope, "prototype").unwrap();
    if let (Some(file_proto), Some(blob_proto)) = (
        file_constructor.get(scope, file_proto_key.into()),
        blob_constructor.get(scope, file_proto_key.into()),
    ) {
        if let Ok(file_proto_obj) = v8::Local::<v8::Object>::try_from(file_proto) {
            let _ = file_proto_obj.set_prototype(scope, blob_proto);
        }
    }
    Ok(())
}
/// Blob constructor callback
fn blob_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _retval: v8::ReturnValue,
) {
    // Parse blob parts array
    let mut data = Vec::new();
    let mut mime_type = String::new();
    if args.length() > 0 {
        let blob_parts: _ = args.get(0);
        if !blob_parts.is_undefined() && !blob_parts.is_null() {
            if blob_parts.is_array() {
                let parts_array: _ = v8::Local::<v8::Array>::try_from(blob_parts).unwrap();
                let len: _ = parts_array.length();
                for i in 0..len {
                    let part: _ = parts_array.get_index(scope, i).unwrap();
                    append_blob_part(scope, part, &mut data);
                }
            } else if blob_parts.is_string() {
                // Direct string argument
                let part_str: _ = blob_parts
                    .to_string(scope)
                    .unwrap()
                    .to_rust_string_lossy(scope);
                data.extend_from_slice(part_str.as_bytes());
            }
        }
    }
    // Parse options for MIME type
    if args.length() > 1 {
        let options: _ = args.get(1);
        if !options.is_undefined() && !options.is_null() && options.is_object() {
            let options_obj: _ = v8::Local::<v8::Object>::try_from(options).unwrap();
            let type_key: _ = v8::String::new(scope, "type").unwrap();
            if let Some(type_val) = options_obj.get(scope, type_key.into()) {
                if type_val.is_string() {
                    mime_type = type_val
                        .to_string(scope)
                        .unwrap()
                        .to_rust_string_lossy(scope);
                }
            }
        }
    }
    // Get the default instance created by V8
    let this: v8::Local<v8::Object> = args.this();
    // Set size property
    let size_key: _ = v8::String::new(scope, "size").unwrap();
    let size_val: _ = v8::Number::new(scope, data.len() as f64);
    this.set(scope, size_key.into(), size_val.into());
    // Set type property
    let type_key: _ = v8::String::new(scope, "type").unwrap();
    let type_val: _ = v8::String::new(scope, &mime_type).unwrap();
    this.set(scope, type_key.into(), type_val.into());
    // Add methods to Blob FIRST
    let text_key: _ = v8::String::new(scope, "text").unwrap();
    let text_template: _ = v8::FunctionTemplate::new(scope, blob_text);
    let text_func: _ = text_template.get_function(scope).unwrap();
    this.set(scope, text_key.into(), text_func.into());
    let slice_key: _ = v8::String::new(scope, "slice").unwrap();
    let slice_template: _ = v8::FunctionTemplate::new(scope, blob_slice);
    let slice_func: _ = slice_template.get_function(scope).unwrap();
    this.set(scope, slice_key.into(), slice_func.into());
    let array_buffer_key: _ = v8::String::new(scope, "arrayBuffer").unwrap();
    let array_buffer_template: _ = v8::FunctionTemplate::new(scope, blob_array_buffer);
    let array_buffer_func: _ = array_buffer_template.get_function(scope).unwrap();
    this.set(scope, array_buffer_key.into(), array_buffer_func.into());
    let stream_key: _ = v8::String::new(scope, "stream").unwrap();
    let stream_template: _ = v8::FunctionTemplate::new(scope, blob_stream);
    let stream_func: _ = stream_template.get_function(scope).unwrap();
    this.set(scope, stream_key.into(), stream_func.into());
    // Store bytes internally AFTER adding methods.
    set_blob_data_properties(scope, this, &data);
    // Don't return anything - use the default instance
}
/// File constructor callback (extends Blob)
fn file_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _retval: v8::ReturnValue,
) {
    // Parse file parts (same as Blob)
    let mut data = Vec::new();
    let mut mime_type = String::new();
    let mut file_name = String::new();
    if args.length() > 0 {
        let blob_parts: _ = args.get(0);
        if !blob_parts.is_undefined() && !blob_parts.is_null() {
            if blob_parts.is_array() {
                let parts_array: _ = v8::Local::<v8::Array>::try_from(blob_parts).unwrap();
                let len: _ = parts_array.length();
                for i in 0..len {
                    let part: _ = parts_array.get_index(scope, i).unwrap();
                    append_blob_part(scope, part, &mut data);
                }
            }
        }
    }
    // Parse file name (first argument after parts)
    if args.length() > 1 {
        let name_arg: _ = args.get(1);
        if name_arg.is_string() {
            file_name = name_arg
                .to_string(scope)
                .unwrap()
                .to_rust_string_lossy(scope);
        }
    }
    // Parse options for MIME type and other metadata
    if args.length() > 2 {
        let options: _ = args.get(2);
        if !options.is_undefined() && !options.is_null() && options.is_object() {
            let options_obj: _ = v8::Local::<v8::Object>::try_from(options).unwrap();
            let type_key: _ = v8::String::new(scope, "type").unwrap();
            if let Some(type_val) = options_obj.get(scope, type_key.into()) {
                if type_val.is_string() {
                    mime_type = type_val
                        .to_string(scope)
                        .unwrap()
                        .to_rust_string_lossy(scope);
                }
            }
        }
    }
    // Get the default instance created by V8
    let this: v8::Local<v8::Object> = args.this();
    // Set Blob properties
    let size_key: _ = v8::String::new(scope, "size").unwrap();
    let size_val: _ = v8::Number::new(scope, data.len() as f64);
    this.set(scope, size_key.into(), size_val.into());
    let type_key: _ = v8::String::new(scope, "type").unwrap();
    let type_val: _ = v8::String::new(scope, &mime_type).unwrap();
    this.set(scope, type_key.into(), type_val.into());
    // Set File-specific properties
    let name_key: _ = v8::String::new(scope, "name").unwrap();
    let name_val: _ = v8::String::new(scope, &file_name).unwrap();
    this.set(scope, name_key.into(), name_val.into());
    let is_file_key: _ = v8::String::new(scope, "_isFile").unwrap();
    let true_value = v8::Boolean::new(scope, true);
    this.set(scope, is_file_key.into(), true_value.into());
    // Add lastModified property (current timestamp)
    let last_modified_key: _ = v8::String::new(scope, "lastModified").unwrap();
    let mut last_modified = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as f64;
    if args.length() > 2 {
        let options: _ = args.get(2);
        if !options.is_undefined() && !options.is_null() && options.is_object() {
            let options_obj: _ = v8::Local::<v8::Object>::try_from(options).unwrap();
            let last_modified_option_key: _ = v8::String::new(scope, "lastModified").unwrap();
            if let Some(last_modified_val) = options_obj.get(scope, last_modified_option_key.into())
            {
                if last_modified_val.is_number() {
                    last_modified = last_modified_val.to_number(scope).unwrap().value();
                }
            }
        }
    }
    let last_modified_val: _ = v8::Number::new(scope, last_modified);
    this.set(scope, last_modified_key.into(), last_modified_val.into());
    // Store bytes internally.
    set_blob_data_properties(scope, this, &data);
    // Add methods to File (inherits from Blob)
    let text_key: _ = v8::String::new(scope, "text").unwrap();
    let text_template: _ = v8::FunctionTemplate::new(scope, blob_text);
    let text_func: _ = text_template.get_function(scope).unwrap();
    this.set(scope, text_key.into(), text_func.into());
    let slice_key: _ = v8::String::new(scope, "slice").unwrap();
    let slice_template: _ = v8::FunctionTemplate::new(scope, blob_slice);
    let slice_func: _ = slice_template.get_function(scope).unwrap();
    this.set(scope, slice_key.into(), slice_func.into());
    let array_buffer_key: _ = v8::String::new(scope, "arrayBuffer").unwrap();
    let array_buffer_template: _ = v8::FunctionTemplate::new(scope, blob_array_buffer);
    let array_buffer_func: _ = array_buffer_template.get_function(scope).unwrap();
    this.set(scope, array_buffer_key.into(), array_buffer_func.into());
    let stream_key: _ = v8::String::new(scope, "stream").unwrap();
    let stream_template: _ = v8::FunctionTemplate::new(scope, blob_stream);
    let stream_func: _ = stream_template.get_function(scope).unwrap();
    this.set(scope, stream_key.into(), stream_func.into());
    // Don't return anything - use the default instance
}
/// Blob.arrayBuffer() method
fn blob_array_buffer(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: v8::Local<v8::Object> = args.this();

    if let Some(bytes) = blob_object_bytes(scope, this) {
        let array_buffer = array_buffer_from_bytes(scope, &bytes);
        retval.set(array_buffer.into());
        return;
    }

    // Fallback for older Blob objects created before blobBytes existed.
    let data_key: _ = v8::String::new(scope, "blobData").unwrap();
    let data_val: _ = this.get(scope, data_key.into());
    if let Some(data) = data_val {
        if data.is_string() {
            let data_string = data.to_string(scope).unwrap().to_rust_string_lossy(scope);
            let bytes = data_string.as_bytes();
            let array_buffer = v8::ArrayBuffer::new(scope, bytes.len());
            if !bytes.is_empty() {
                let backing_store = array_buffer.get_backing_store();
                for (i, byte) in bytes.iter().enumerate() {
                    backing_store[i].set(*byte);
                }
            }
            retval.set(array_buffer.into());
            return;
        }
    }
    // If no data stored, return empty ArrayBuffer
    let array_buffer: _ = v8::ArrayBuffer::new(scope, 0);
    retval.set(array_buffer.into());
}
/// Blob.text() method
fn blob_text(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Get the Blob object (this)
    let this: v8::Local<v8::Object> = args.this();
    // Get the internal data stored in the Blob
    let data_key: _ = v8::String::new(scope, "blobData").unwrap();
    let data_val: _ = this.get(scope, data_key.into());
    if let Some(data) = data_val {
        if data.is_string() {
            // Return the stored string directly
            let text_str: _ = data.to_string(scope).unwrap();
            retval.set(text_str.into());
            return;
        }
    }
    // Return empty string if no valid data
    let text: _ = v8::String::new(scope, "").unwrap();
    retval.set(text.into());
}
/// Blob.slice() method
fn blob_slice(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Get the source Blob object (this)
    let this: v8::Local<v8::Object> = args.this();
    // Parse arguments with defaults
    let start: _ = if args.length() > 0 && !args.get(0).is_undefined() {
        args.get(0).to_number(scope).unwrap().value() as i64
    } else {
        0
    };
    let end: _ = if args.length() > 1 && !args.get(1).is_undefined() {
        args.get(1).to_number(scope).unwrap().value() as i64
    } else {
        i64::MAX
    };
    let content_type: _ = if args.length() > 2 && !args.get(2).is_undefined() {
        let type_val: _ = args.get(2);
        if type_val.is_string() {
            type_val
                .to_string(scope)
                .unwrap()
                .to_rust_string_lossy(scope)
        } else {
            String::new()
        }
    } else {
        // Get the original type from this Blob
        let type_key: _ = v8::String::new(scope, "type").unwrap();
        if let Some(type_val) = this.get(scope, type_key.into()) {
            if type_val.is_string() {
                type_val
                    .to_string(scope)
                    .unwrap()
                    .to_rust_string_lossy(scope)
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    };
    // Get and slice source bytes.
    let data = blob_object_bytes(scope, this).unwrap_or_else(|| {
        let data_key = v8::String::new(scope, "blobData").unwrap();
        this.get(scope, data_key.into())
            .and_then(|data| {
                data.to_string(scope)
                    .map(|value| value.to_rust_string_lossy(scope).into_bytes())
            })
            .unwrap_or_default()
    });
    let len = data.len() as i64;
    let start_usize = if start < 0 {
        (len + start).clamp(0, len) as usize
    } else {
        start.min(len) as usize
    };
    let end_usize = if end == i64::MAX {
        len as usize
    } else if end < 0 {
        (len + end).clamp(0, len) as usize
    } else {
        end.min(len) as usize
    };
    let sliced_bytes = if end_usize > start_usize {
        data[start_usize..end_usize].to_vec()
    } else {
        Vec::new()
    };
    // Create new Blob object with sliced data
    let blob_obj: _ = v8::Object::new(scope);
    let is_file_key = v8::String::new(scope, "_isFile").unwrap();
    let is_file = this
        .get(scope, is_file_key.into())
        .map(|value| value.is_boolean() && value.boolean_value(scope))
        .unwrap_or(false);
    if is_file {
        let true_value = v8::Boolean::new(scope, true);
        blob_obj.set(scope, is_file_key.into(), true_value.into());
    }
    let size_key: _ = v8::String::new(scope, "size").unwrap();
    let size_val: _ = v8::Number::new(scope, sliced_bytes.len() as f64);
    blob_obj.set(scope, size_key.into(), size_val.into());
    let type_key: _ = v8::String::new(scope, "type").unwrap();
    let type_val: _ = v8::String::new(scope, &content_type).unwrap();
    blob_obj.set(scope, type_key.into(), type_val.into());
    set_blob_data_properties(scope, blob_obj, &sliced_bytes);
    // Add methods to sliced Blob
    let text_key: _ = v8::String::new(scope, "text").unwrap();
    let text_template: _ = v8::FunctionTemplate::new(scope, blob_text);
    let text_func: _ = text_template.get_function(scope).unwrap();
    blob_obj.set(scope, text_key.into(), text_func.into());
    let slice_key: _ = v8::String::new(scope, "slice").unwrap();
    let slice_template: _ = v8::FunctionTemplate::new(scope, blob_slice);
    let slice_func: _ = slice_template.get_function(scope).unwrap();
    blob_obj.set(scope, slice_key.into(), slice_func.into());
    let array_buffer_key: _ = v8::String::new(scope, "arrayBuffer").unwrap();
    let array_buffer_template: _ = v8::FunctionTemplate::new(scope, blob_array_buffer);
    let array_buffer_func: _ = array_buffer_template.get_function(scope).unwrap();
    blob_obj.set(scope, array_buffer_key.into(), array_buffer_func.into());
    let stream_key: _ = v8::String::new(scope, "stream").unwrap();
    let stream_template: _ = v8::FunctionTemplate::new(scope, blob_stream);
    let stream_func: _ = stream_template.get_function(scope).unwrap();
    blob_obj.set(scope, stream_key.into(), stream_func.into());
    let context = scope.get_current_context();
    let global = context.global(scope);
    let ctor_key = v8::String::new(scope, if is_file { "File" } else { "Blob" }).unwrap();
    if let Some(ctor_val) = global.get(scope, ctor_key.into()) {
        if let Ok(ctor_obj) = v8::Local::<v8::Object>::try_from(ctor_val) {
            let prototype_key = v8::String::new(scope, "prototype").unwrap();
            if let Some(proto) = ctor_obj.get(scope, prototype_key.into()) {
                let _ = blob_obj.set_prototype(scope, proto);
            }
        }
    }
    retval.set(blob_obj.into());
}
/// Blob.stream() method
fn blob_stream(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let stream = v8::Object::new(scope);
    let get_reader = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let reader = v8::Object::new(scope);
            let read = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let result = v8::Object::new(scope);
                    let done_key = v8::String::new(scope, "done").unwrap();
                    let true_value = v8::Boolean::new(scope, true);
                    result.set(scope, done_key.into(), true_value.into());
                    let value_key = v8::String::new(scope, "value").unwrap();
                    let undefined = v8::undefined(scope);
                    result.set(scope, value_key.into(), undefined.into());
                    retval.set(result.into());
                },
            )
            .unwrap();
            let read_key = v8::String::new(scope, "read").unwrap();
            reader.set(scope, read_key.into(), read.into());
            retval.set(reader.into());
        },
    )
    .unwrap();
    let get_reader_key = v8::String::new(scope, "getReader").unwrap();
    stream.set(scope, get_reader_key.into(), get_reader.into());
    retval.set(stream.into());
}

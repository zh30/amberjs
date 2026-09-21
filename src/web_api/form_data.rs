// FormData API implementation per Web standard
//
// The FormData interface provides a way to construct a set of key/value pairs
/// representing form fields and their values, which can be sent using fetch().
use anyhow::Result;
use rusty_v8 as v8;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Thread-safe FormData storage
static FORMDATA_CACHE: OnceLock<Mutex<HashMap<usize, Vec<FormDataEntry>>>> = OnceLock::new();

/// Get the FormData cache mutex
fn get_formdata_cache() -> &'static Mutex<HashMap<usize, Vec<FormDataEntry>>> {
    FORMDATA_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// FormData entry - can be simple value or file
#[derive(Debug, Clone)]
pub struct FormDataEntry {
    pub name: String,
    pub value: String,
    pub body: Vec<u8>,
    pub filename: Option<String>,
    pub content_type: String,
}

fn value_to_optional_string(
    scope: &mut v8::PinScope,
    value: v8::Local<v8::Value>,
) -> Option<String> {
    if value.is_undefined() || value.is_null() {
        return None;
    }

    value
        .to_string(scope)
        .map(|value| value.to_rust_string_lossy(scope))
}

fn object_string_property(
    scope: &mut v8::PinScope,
    object: v8::Local<v8::Object>,
    key: &str,
) -> Option<String> {
    let key = v8::String::new(scope, key)?.into();
    object
        .get(scope, key)
        .and_then(|value| value_to_optional_string(scope, value))
}

fn object_bool_property(
    scope: &mut v8::PinScope,
    object: v8::Local<v8::Object>,
    key: &str,
) -> bool {
    let Some(key) = v8::String::new(scope, key) else {
        return false;
    };
    object
        .get(scope, key.into())
        .is_some_and(|value| value.is_boolean() && value.boolean_value(scope))
}

fn object_has_property(scope: &mut v8::PinScope, object: v8::Local<v8::Object>, key: &str) -> bool {
    let Some(key) = v8::String::new(scope, key) else {
        return false;
    };
    object.has(scope, key.into()).unwrap_or(false)
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

fn object_blob_bytes(scope: &mut v8::PinScope, object: v8::Local<v8::Object>) -> Option<Vec<u8>> {
    let key = v8::String::new(scope, "blobBytes")?;
    object
        .get(scope, key.into())
        .and_then(bytes_from_array_buffer_value)
}

fn form_data_value_entry(
    scope: &mut v8::PinScope,
    value: v8::Local<v8::Value>,
    explicit_filename: Option<String>,
) -> Option<(String, Vec<u8>, Option<String>, String)> {
    if value.is_object() {
        let object = value.to_object(scope)?;
        let is_blob_like = object_has_property(scope, object, "arrayBuffer")
            || object_has_property(scope, object, "blobData");

        if is_blob_like {
            let body = object_blob_bytes(scope, object).unwrap_or_else(|| {
                object_string_property(scope, object, "blobData")
                    .unwrap_or_default()
                    .into_bytes()
            });
            let body_text = object_string_property(scope, object, "blobData")
                .unwrap_or_else(|| String::from_utf8_lossy(&body).into_owned());
            let content_type = object_string_property(scope, object, "type")
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "application/octet-stream".to_string());
            let filename = explicit_filename.or_else(|| {
                if object_bool_property(scope, object, "_isFile") {
                    object_string_property(scope, object, "name").filter(|name| !name.is_empty())
                } else {
                    Some("blob".to_string())
                }
            });

            return Some((body_text, body, filename, content_type));
        }
    }

    let text = value_to_optional_string(scope, value)?;
    let body = text.as_bytes().to_vec();
    Some((text, body, explicit_filename, "text/plain".to_string()))
}

fn form_data_index_from_object(
    scope: &mut v8::PinScope,
    form_data_obj: v8::Local<v8::Object>,
) -> Option<usize> {
    form_data_obj
        .get_internal_field(scope, 0)
        .and_then(|data| v8::Local::<v8::Value>::try_from(data).ok())
        .and_then(|value| value.to_integer(scope))
        .map(|index| index.value() as usize)
}

fn form_data_entries_for_object(
    scope: &mut v8::PinScope,
    form_data_obj: v8::Local<v8::Object>,
) -> Vec<FormDataEntry> {
    form_data_index_from_object(scope, form_data_obj)
        .and_then(|index| get_formdata_cache().lock().unwrap().get(&index).cloned())
        .unwrap_or_default()
}

fn form_data_entries_array<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    entries: &[FormDataEntry],
) -> v8::Local<'a, v8::Array> {
    let array = v8::Array::new(scope, entries.len() as i32);
    for (index, entry) in entries.iter().enumerate() {
        let pair = v8::Array::new(scope, 2);
        let name = v8::String::new(scope, &entry.name).unwrap().into();
        let value = v8::String::new(scope, &entry.value).unwrap().into();
        pair.set_index(scope, 0, name);
        pair.set_index(scope, 1, value);
        array.set_index(scope, index as u32, pair.into());
    }
    array
}

fn form_data_keys_array<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    entries: &[FormDataEntry],
) -> v8::Local<'a, v8::Array> {
    let array = v8::Array::new(scope, entries.len() as i32);
    for (index, entry) in entries.iter().enumerate() {
        let name = v8::String::new(scope, &entry.name).unwrap().into();
        array.set_index(scope, index as u32, name);
    }
    array
}

fn form_data_values_array<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    entries: &[FormDataEntry],
) -> v8::Local<'a, v8::Array> {
    let array = v8::Array::new(scope, entries.len() as i32);
    for (index, entry) in entries.iter().enumerate() {
        let value = v8::String::new(scope, &entry.value).unwrap().into();
        array.set_index(scope, index as u32, value);
    }
    array
}

fn symbol_iterator_value<'a>(scope: &mut v8::PinScope<'a, '_>) -> Option<v8::Local<'a, v8::Value>> {
    let context = scope.get_current_context();
    let global = context.global(scope);
    let symbol_key: v8::Local<v8::Value> = v8::String::new(scope, "Symbol")?.into();
    let symbol_value = global.get(scope, symbol_key)?;
    let symbol_object = symbol_value.to_object(scope)?;
    let iterator_key: v8::Local<v8::Value> = v8::String::new(scope, "iterator")?.into();
    symbol_object.get(scope, iterator_key)
}

fn iterator_from_array<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    array: v8::Local<'a, v8::Array>,
) -> v8::Local<'a, v8::Value> {
    let Some(iterator_key) = symbol_iterator_value(scope) else {
        return array.into();
    };
    let Some(array_iterator) = array.get(scope, iterator_key) else {
        return array.into();
    };
    let Ok(array_iterator_func) = v8::Local::<v8::Function>::try_from(array_iterator) else {
        return array.into();
    };

    array_iterator_func
        .call(scope, array.into(), &[])
        .unwrap_or_else(|| array.into())
}

/// Setup FormData API in V8 context
pub fn setup_form_data_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    // FormData constructor
    let form_data_template: _ = v8::FunctionTemplate::new(scope, form_data_constructor);
    let form_data_constructor: _ = form_data_template.get_function(scope).unwrap();
    // Register FormData constructor
    let global: _ = context.global(scope);
    let form_data_key: _ = v8::String::new(scope, "FormData").unwrap();
    global.set(scope, form_data_key.into(), form_data_constructor.into());
    Ok(())
}
/// FormData constructor callback
fn form_data_constructor(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Create ObjectTemplate with internal field for storing FormData index
    let form_data_template = v8::ObjectTemplate::new(scope);
    form_data_template.set_internal_field_count(1);

    let form_data_obj: v8::Local<v8::Object> = match form_data_template.new_instance(scope) {
        Some(obj) => obj,
        None => {
            retval.set(v8::null(scope).into());
            return;
        }
    };

    // Get next available index for this FormData instance
    static FORMDATA_INDEX_COUNTER: OnceLock<Mutex<usize>> = OnceLock::new();
    let index_counter = FORMDATA_INDEX_COUNTER.get_or_init(|| Mutex::new(0));
    let mut counter = index_counter.lock().unwrap();
    let index = *counter;
    *counter += 1;
    drop(counter);

    // Store index in internal field 0
    let index_val: v8::Local<v8::Value> = v8::Integer::new(scope, index as i32).into();
    form_data_obj.set_internal_field(0, index_val.into());

    // Initialize FormData entries for this index
    let mut cache = get_formdata_cache().lock().unwrap();
    cache.insert(index, Vec::new());
    drop(cache);

    // Add append method
    let append_key = v8::String::new(scope, "append").unwrap().into();
    let append_template = v8::FunctionTemplate::new(scope, form_data_append);
    let append_func = append_template.get_function(scope).unwrap();
    form_data_obj.set(scope, append_key, append_func.into());

    // Add delete method
    let delete_key = v8::String::new(scope, "delete").unwrap().into();
    let delete_template = v8::FunctionTemplate::new(scope, form_data_delete);
    let delete_func = delete_template.get_function(scope).unwrap();
    form_data_obj.set(scope, delete_key, delete_func.into());

    // Add get method
    let get_key = v8::String::new(scope, "get").unwrap().into();
    let get_template = v8::FunctionTemplate::new(scope, form_data_get);
    let get_func = get_template.get_function(scope).unwrap();
    form_data_obj.set(scope, get_key, get_func.into());

    // Add getAll method
    let get_all_key = v8::String::new(scope, "getAll").unwrap().into();
    let get_all_template = v8::FunctionTemplate::new(scope, form_data_get_all);
    let get_all_func = get_all_template.get_function(scope).unwrap();
    form_data_obj.set(scope, get_all_key, get_all_func.into());

    // Add has method
    let has_key = v8::String::new(scope, "has").unwrap().into();
    let has_template = v8::FunctionTemplate::new(scope, form_data_has);
    let has_func = has_template.get_function(scope).unwrap();
    form_data_obj.set(scope, has_key, has_func.into());

    // Add set method
    let set_key = v8::String::new(scope, "set").unwrap().into();
    let set_template = v8::FunctionTemplate::new(scope, form_data_set);
    let set_func = set_template.get_function(scope).unwrap();
    form_data_obj.set(scope, set_key, set_func.into());

    // Add entries method
    let entries_key = v8::String::new(scope, "entries").unwrap().into();
    let entries_template = v8::FunctionTemplate::new(scope, form_data_entries);
    let entries_func = entries_template.get_function(scope).unwrap();
    form_data_obj.set(scope, entries_key, entries_func.into());
    if let Some(iterator_key) = symbol_iterator_value(scope) {
        form_data_obj.set(scope, iterator_key, entries_func.into());
    }

    // Add keys method
    let keys_key = v8::String::new(scope, "keys").unwrap().into();
    let keys_template = v8::FunctionTemplate::new(scope, form_data_keys);
    let keys_func = keys_template.get_function(scope).unwrap();
    form_data_obj.set(scope, keys_key, keys_func.into());

    // Add values method
    let values_key = v8::String::new(scope, "values").unwrap().into();
    let values_template = v8::FunctionTemplate::new(scope, form_data_values);
    let values_func = values_template.get_function(scope).unwrap();
    form_data_obj.set(scope, values_key, values_func.into());

    // Add forEach method
    let for_each_key = v8::String::new(scope, "forEach").unwrap().into();
    let for_each_template = v8::FunctionTemplate::new(scope, form_data_for_each);
    let for_each_func = for_each_template.get_function(scope).unwrap();
    form_data_obj.set(scope, for_each_key, for_each_func.into());

    retval.set(form_data_obj.into());
}

/// FormData.append() method - adds a new value to an existing key
/// or adds the key if it doesn't exist
fn form_data_append(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _retval: v8::ReturnValue,
) {
    let this_obj: v8::Local<v8::Object> = args.this();

    // Get index from internal field
    let index = this_obj
        .get_internal_field(scope, 0)
        .and_then(|data| v8::Local::<v8::Value>::try_from(data).ok())
        .and_then(|v| v.to_integer(scope))
        .map(|i| i.value() as usize)
        .unwrap_or(usize::MAX);

    // Get name
    let name = if let Some(name_val) = args.get(0).to_string(scope) {
        name_val.to_rust_string_lossy(scope)
    } else {
        return;
    };

    // Get optional filename (third argument)
    let explicit_filename = value_to_optional_string(scope, args.get(2));

    // Get value (can be string or Blob/File)
    let Some((value, body, filename, content_type)) =
        form_data_value_entry(scope, args.get(1), explicit_filename)
    else {
        return;
    };

    // Store in cache
    let mut cache = get_formdata_cache().lock().unwrap();
    if let Some(entries) = cache.get_mut(&index) {
        entries.push(FormDataEntry {
            name,
            value,
            body,
            filename,
            content_type,
        });
    }
}

/// FormData.delete() method - removes all values associated with a key
fn form_data_delete(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _retval: v8::ReturnValue,
) {
    let this_obj: v8::Local<v8::Object> = args.this();

    let index = this_obj
        .get_internal_field(scope, 0)
        .and_then(|data| v8::Local::<v8::Value>::try_from(data).ok())
        .and_then(|v| v.to_integer(scope))
        .map(|i| i.value() as usize)
        .unwrap_or(usize::MAX);

    let name = if let Some(name_val) = args.get(0).to_string(scope) {
        name_val.to_rust_string_lossy(scope)
    } else {
        return;
    };

    let mut cache = get_formdata_cache().lock().unwrap();
    if let Some(entries) = cache.get_mut(&index) {
        entries.retain(|entry| entry.name != name);
    }
}

/// FormData.get() method - returns the first value associated with a key
fn form_data_get(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this_obj: v8::Local<v8::Object> = args.this();

    let index = this_obj
        .get_internal_field(scope, 0)
        .and_then(|data| v8::Local::<v8::Value>::try_from(data).ok())
        .and_then(|v| v.to_integer(scope))
        .map(|i| i.value() as usize)
        .unwrap_or(usize::MAX);

    let name = if let Some(name_val) = args.get(0).to_string(scope) {
        name_val.to_rust_string_lossy(scope)
    } else {
        retval.set(v8::null(scope).into());
        return;
    };

    let cache = get_formdata_cache().lock().unwrap();
    if let Some(entries) = cache.get(&index) {
        for entry in entries {
            if entry.name == name {
                retval.set(v8::String::new(scope, &entry.value).unwrap().into());
                return;
            }
        }
    }
    retval.set(v8::null(scope).into());
}

/// FormData.getAll() method - returns all values associated with a key
fn form_data_get_all(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this_obj: v8::Local<v8::Object> = args.this();

    let index = this_obj
        .get_internal_field(scope, 0)
        .and_then(|data| v8::Local::<v8::Value>::try_from(data).ok())
        .and_then(|v| v.to_integer(scope))
        .map(|i| i.value() as usize)
        .unwrap_or(usize::MAX);

    let name = if let Some(name_val) = args.get(0).to_string(scope) {
        name_val.to_rust_string_lossy(scope)
    } else {
        retval.set(v8::Array::new(scope, 0).into());
        return;
    };

    let cache = get_formdata_cache().lock().unwrap();
    if let Some(entries) = cache.get(&index) {
        let values: Vec<_> = entries
            .iter()
            .filter(|entry| entry.name == name)
            .map(|entry| v8::String::new(scope, &entry.value).unwrap().into())
            .collect();
        let array = v8::Array::new_with_elements(scope, &values);
        retval.set(array.into());
    } else {
        retval.set(v8::Array::new(scope, 0).into());
    }
}

/// FormData.has() method - returns whether a key exists
fn form_data_has(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this_obj: v8::Local<v8::Object> = args.this();

    let index = this_obj
        .get_internal_field(scope, 0)
        .and_then(|data| v8::Local::<v8::Value>::try_from(data).ok())
        .and_then(|v| v.to_integer(scope))
        .map(|i| i.value() as usize)
        .unwrap_or(usize::MAX);

    let name = if let Some(name_val) = args.get(0).to_string(scope) {
        name_val.to_rust_string_lossy(scope)
    } else {
        retval.set(v8::Boolean::new(scope, false).into());
        return;
    };

    let cache = get_formdata_cache().lock().unwrap();
    let has_key = cache
        .get(&index)
        .map(|entries| entries.iter().any(|e| e.name == name))
        .unwrap_or(false);

    retval.set(v8::Boolean::new(scope, has_key).into());
}

/// FormData.set() method - sets a new value for a key, replacing existing values
fn form_data_set(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _retval: v8::ReturnValue,
) {
    let this_obj: v8::Local<v8::Object> = args.this();

    let index = this_obj
        .get_internal_field(scope, 0)
        .and_then(|data| v8::Local::<v8::Value>::try_from(data).ok())
        .and_then(|v| v.to_integer(scope))
        .map(|i| i.value() as usize)
        .unwrap_or(usize::MAX);

    let name = if let Some(name_val) = args.get(0).to_string(scope) {
        name_val.to_rust_string_lossy(scope)
    } else {
        return;
    };

    let explicit_filename = value_to_optional_string(scope, args.get(2));
    let Some((value, body, filename, content_type)) =
        form_data_value_entry(scope, args.get(1), explicit_filename)
    else {
        return;
    };

    let mut cache = get_formdata_cache().lock().unwrap();
    if let Some(entries) = cache.get_mut(&index) {
        // Remove all existing entries with this name
        entries.retain(|entry| entry.name != name);
        // Add new entry
        entries.push(FormDataEntry {
            name,
            value,
            body,
            filename,
            content_type,
        });
    }
}

/// FormData.entries() method - returns an iterator of key/value pairs
fn form_data_entries(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let entries = form_data_entries_for_object(scope, args.this());
    let array = form_data_entries_array(scope, &entries);
    retval.set(iterator_from_array(scope, array));
}

/// FormData.keys() method - returns an iterator of keys
fn form_data_keys(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let entries = form_data_entries_for_object(scope, args.this());
    let array = form_data_keys_array(scope, &entries);
    retval.set(iterator_from_array(scope, array));
}

/// FormData.values() method - returns an iterator of values
fn form_data_values(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let entries = form_data_entries_for_object(scope, args.this());
    let array = form_data_values_array(scope, &entries);
    retval.set(iterator_from_array(scope, array));
}

/// FormData.forEach() method - iterates over all key/value pairs
fn form_data_for_each(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _retval: v8::ReturnValue,
) {
    let callback_val = args.get(0);
    let Ok(callback) = v8::Local::<v8::Function>::try_from(callback_val) else {
        return;
    };

    let this_obj = args.this();
    let entries = form_data_entries_for_object(scope, this_obj);
    let this_arg = args.get(1);
    let receiver = if this_arg.is_undefined() {
        v8::undefined(scope).into()
    } else {
        this_arg
    };

    for entry in entries {
        let value_arg: v8::Local<v8::Value> = v8::String::new(scope, &entry.value).unwrap().into();
        let name_arg: v8::Local<v8::Value> = v8::String::new(scope, &entry.name).unwrap().into();
        let owner_arg: v8::Local<v8::Value> = this_obj.into();
        let _ = callback.call(scope, receiver, &[value_arg, name_arg, owner_arg]);
    }
}

/// Export FormData entries for use with fetch
/// Returns the entries as a Vec for serialization
pub fn get_formdata_entries(index: usize) -> Option<Vec<FormDataEntry>> {
    let cache = FORMDATA_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let guard = cache.lock().unwrap();
    guard.get(&index).cloned()
}

/// Check if a V8 value is a FormData object and return its internal index
pub fn get_formdata_index(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> Option<usize> {
    if !value.is_object() {
        return None;
    }

    let obj = value.to_object(scope)?;

    // Check if object has internal fields (our FormData uses ObjectTemplate with internal field count)
    if obj.internal_field_count() < 1 {
        return None;
    }

    // Check if the object has FormData methods
    let append_key = v8::String::new(scope, "append").unwrap().into();
    if !obj.has(scope, append_key).unwrap_or(false) {
        return None;
    }

    // Get the internal field which stores the FormData index
    obj.get_internal_field(scope, 0)
        .and_then(|data| v8::Local::<v8::Value>::try_from(data).ok())
        .and_then(|v| v.to_integer(scope))
        .map(|i| i.value() as usize)
}

/// Serialize FormData entries to multipart/form-data format
pub fn serialize_formdata_multipart(entries: &[FormDataEntry], boundary: &str) -> Vec<u8> {
    let mut result = Vec::new();

    for entry in entries {
        // Write boundary
        result.extend_from_slice(b"--");
        result.extend_from_slice(boundary.as_bytes());
        result.extend_from_slice(b"\r\n");

        // Write Content-Disposition header
        result.extend_from_slice(b"Content-Disposition: form-data; name=\"");
        result.extend_from_slice(entry.name.as_bytes());
        result.extend_from_slice(b"\"");

        // Add filename if present
        if let Some(filename) = &entry.filename {
            result.extend_from_slice(b"; filename=\"");
            result.extend_from_slice(filename.as_bytes());
            result.extend_from_slice(b"\"");
        }
        result.extend_from_slice(b"\r\n");

        // Write Content-Type header
        if !entry.content_type.is_empty() {
            result.extend_from_slice(b"Content-Type: ");
            result.extend_from_slice(entry.content_type.as_bytes());
            result.extend_from_slice(b"\r\n");
        }

        // Empty line before body
        result.extend_from_slice(b"\r\n");

        // Write body
        result.extend_from_slice(&entry.body);
        result.extend_from_slice(b"\r\n");
    }

    // Write final boundary
    result.extend_from_slice(b"--");
    result.extend_from_slice(boundary.as_bytes());
    result.extend_from_slice(b"--\r\n");

    result
}

/// Generate a random boundary string for multipart/form-data
pub fn generate_boundary() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random: u128 = rng.gen();
    format!("----AmberFormBoundary{}", random)
}

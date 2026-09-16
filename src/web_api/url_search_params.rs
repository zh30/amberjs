// URLSearchParams API implementation for Web standard
// Provides URLSearchParams constructor and methods for query string manipulation

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rusty_v8 as v8;
use std::os::raw::c_void;
use std::sync::{Arc, Mutex};

/// URL-encode a string for use in query strings
fn url_encode(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC)
        .collect::<String>()
        .replace("%20", "+")
}

/// URL-decode a string from query string
fn url_decode(value: &str) -> String {
    let plus_normalized = value.replace('+', " ");
    percent_encoding::percent_decode_str(&plus_normalized)
        .decode_utf8_lossy()
        .to_string()
}

fn throw_type_error(scope: &mut v8::PinScope, message: &str) {
    let Some(message) = v8::String::new(scope, message) else {
        return;
    };
    let error = v8::Exception::type_error(scope, message);
    scope.throw_exception(error);
}

/// Parse a query string into key-value pairs
fn parse_query_string(query: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    if query.is_empty() {
        return pairs;
    }

    let parts: Vec<&str> = query.split('&').collect();
    for part in parts {
        if let Some(idx) = part.find('=') {
            let key = url_decode(&part[..idx]);
            let value = url_decode(&part[idx + 1..]);
            pairs.push((key, value));
        } else if !part.is_empty() {
            pairs.push((url_decode(part), String::new()));
        }
    }
    pairs
}

fn parse_sequence_pairs(
    scope: &mut v8::PinScope,
    sequence: v8::Local<v8::Array>,
) -> Result<Vec<(String, String)>, ()> {
    let mut pairs = Vec::new();

    for i in 0..sequence.length() {
        let Some(pair_val) = sequence.get_index(scope, i) else {
            throw_type_error(scope, "URLSearchParams sequence pair is missing");
            return Err(());
        };
        if !pair_val.is_array() {
            throw_type_error(scope, "URLSearchParams sequence pair must be an array");
            return Err(());
        }
        let Ok(pair_array) = v8::Local::<v8::Array>::try_from(pair_val) else {
            throw_type_error(scope, "URLSearchParams sequence pair must be an array");
            return Err(());
        };
        if pair_array.length() != 2 {
            throw_type_error(
                scope,
                "URLSearchParams sequence pair must contain exactly two items",
            );
            return Err(());
        }
        let key_val = match pair_array.get_index(scope, 0) {
            Some(value) => value,
            None => v8::undefined(scope).into(),
        };
        let value_val = match pair_array.get_index(scope, 1) {
            Some(value) => value,
            None => v8::undefined(scope).into(),
        };
        let Some(key) = key_val.to_string(scope) else {
            return Err(());
        };
        let Some(value) = value_val.to_string(scope) else {
            return Err(());
        };

        pairs.push((
            key.to_rust_string_lossy(scope),
            value.to_rust_string_lossy(scope),
        ));
    }

    Ok(pairs)
}

/// Convert key-value pairs to query string
fn to_query_string(pairs: &[(String, String)]) -> String {
    let mut result = String::new();
    for (i, (key, value)) in pairs.iter().enumerate() {
        if i > 0 {
            result.push('&');
        }
        result.push_str(&url_encode(key));
        result.push('=');
        result.push_str(&url_encode(value));
    }
    result
}

/// Get the params data from a V8 object using External
fn get_params_data(
    scope: &mut v8::PinScope,
    this_obj: &v8::Object,
) -> Option<Arc<Mutex<Vec<(String, String)>>>> {
    let data_key = v8::String::new(scope, "_paramsData").unwrap();
    if let Some(external_val) = this_obj.get(scope, data_key.into()) {
        if external_val.is_external() {
            if let Ok(ptr) = v8::Local::<v8::External>::try_from(external_val) {
                // Clone the Arc to keep it alive, but don't call from_raw
                // since we didn't transfer ownership
                let raw = ptr.value() as *mut Mutex<Vec<(String, String)>>;
                return Some(unsafe {
                    Arc::increment_strong_count(raw);
                    Arc::from_raw(raw)
                });
            }
        }
    }
    None
}

/// Set the params data on a V8 object using External
fn set_params_data(
    scope: &mut v8::PinScope,
    this_obj: &v8::Object,
    data: &Arc<Mutex<Vec<(String, String)>>>,
) {
    // Clone the Arc and leak it - we'll manage it manually
    let cloned = Arc::clone(data);
    let data_ptr = Arc::into_raw(cloned) as *mut c_void;
    let external = v8::External::new(scope, data_ptr);
    let data_key = v8::String::new(scope, "_paramsData").unwrap();
    this_obj.set(scope, data_key.into(), external.into());
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

fn object_has_iterator(scope: &mut v8::PinScope, object: v8::Local<v8::Object>) -> bool {
    let Some(iterator_key) = symbol_iterator_value(scope) else {
        return false;
    };
    object
        .get(scope, iterator_key)
        .map(|iterator| iterator.is_function())
        .unwrap_or(false)
}

fn array_from_iterable<'scope, 'value>(
    scope: &mut v8::PinScope<'scope, '_>,
    iterable: v8::Local<'value, v8::Value>,
) -> Option<v8::Local<'scope, v8::Array>> {
    let context = scope.get_current_context();
    let global = context.global(scope);
    let array_key: v8::Local<v8::Value> = v8::String::new(scope, "Array")?.into();
    let array_value = global.get(scope, array_key)?;
    let array_object = array_value.to_object(scope)?;
    let from_key: v8::Local<v8::Value> = v8::String::new(scope, "from")?.into();
    let from_value = array_object.get(scope, from_key)?;
    let Ok(from_fn) = v8::Local::<v8::Function>::try_from(from_value) else {
        throw_type_error(scope, "Array.from is not available");
        return None;
    };
    let result = from_fn.call(scope, array_object.into(), &[iterable])?;
    if !result.is_array() {
        throw_type_error(
            scope,
            "URLSearchParams iterable init did not produce an array",
        );
        return None;
    }
    v8::Local::<v8::Array>::try_from(result).ok()
}

fn set_iterator_returns_self(scope: &mut v8::PinScope, iterator: v8::Local<v8::Object>) {
    let Some(iterator_key) = symbol_iterator_value(scope) else {
        return;
    };
    let Some(iterator_fn) = v8::Function::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(args.this().into());
        },
    ) else {
        return;
    };
    iterator.set(scope, iterator_key, iterator_fn.into());
}

/// URLSearchParams constructor callback
fn url_search_params_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Create ObjectTemplate for URLSearchParams instance
    let params_template = v8::ObjectTemplate::new(scope);
    params_template.set_internal_field_count(0); // We use External instead

    let params_obj: v8::Local<v8::Object> = match params_template.new_instance(scope) {
        Some(obj) => obj,
        None => {
            retval.set(v8::null(scope).into());
            return;
        }
    };

    // Initialize params data
    let mut pairs = Vec::new();

    // Parse input and populate params
    let init_arg = args.get(0);

    if !init_arg.is_undefined() && !init_arg.is_null() {
        if init_arg.is_string() {
            // Parse from query string
            let init_str = if let Some(s) = init_arg.to_string(scope) {
                s.to_rust_string_lossy(scope)
            } else {
                String::new()
            };
            // Remove leading '?' if present
            let query = if init_str.starts_with('?') {
                &init_str[1..]
            } else {
                &init_str
            };
            pairs = parse_query_string(query);
        } else if init_arg.is_array() {
            if let Ok(sequence) = v8::Local::<v8::Array>::try_from(init_arg) {
                let Ok(parsed_pairs) = parse_sequence_pairs(scope, sequence) else {
                    return;
                };
                pairs = parsed_pairs;
            }
        } else if init_arg.is_object() {
            if let Some(obj) = init_arg.to_object(scope) {
                if let Some(data) = get_params_data(scope, &obj) {
                    pairs = data.lock().unwrap().clone();
                } else if object_has_iterator(scope, obj) {
                    let Some(sequence) = array_from_iterable(scope, init_arg) else {
                        return;
                    };
                    let Ok(parsed_pairs) = parse_sequence_pairs(scope, sequence) else {
                        return;
                    };
                    pairs = parsed_pairs;
                } else {
                    // Parse from record object - use simple property enumeration
                    let prop_names = obj.get_own_property_names(scope, Default::default());
                    let prop_names = match prop_names {
                        Some(names) => names,
                        None => v8::Array::new(scope, 0),
                    };

                    for i in 0..prop_names.length() {
                        let key = match prop_names.get_index(scope, i) {
                            Some(k) => k,
                            None => continue,
                        };
                        let key_str = if let Some(s) = key.to_string(scope) {
                            s.to_rust_string_lossy(scope)
                        } else {
                            continue;
                        };

                        let value = match obj.get(scope, key) {
                            Some(v) => v,
                            None => continue,
                        };
                        let value_str = if let Some(s) = value.to_string(scope) {
                            s.to_rust_string_lossy(scope)
                        } else {
                            continue;
                        };

                        pairs.push((key_str, value_str));
                    }
                }
            }
        }
    }

    // Store pairs in the object using External
    let data = Arc::new(Mutex::new(pairs));
    set_params_data(scope, &params_obj, &data);

    // Add toString method
    let to_string_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut _retval: v8::ReturnValue| {
            let this_obj = args.this();
            if let Some(data) = get_params_data(scope, &this_obj) {
                let pairs = data.lock().unwrap();
                let result = to_query_string(&pairs);
                _retval.set(v8::String::new(scope, &result).unwrap().into());
            }
        },
    )
    .unwrap();
    let to_string_key = v8::String::new(scope, "toString").unwrap().into();
    params_obj.set(scope, to_string_key, to_string_fn.into());

    // Add append method
    let append_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {
            let this_obj = args.this();
            if let Some(data) = get_params_data(scope, &this_obj) {
                let name = if let Some(s) = args.get(0).to_string(scope) {
                    s.to_rust_string_lossy(scope)
                } else {
                    return;
                };
                let value = if let Some(s) = args.get(1).to_string(scope) {
                    s.to_rust_string_lossy(scope)
                } else {
                    return;
                };
                let mut pairs = data.lock().unwrap();
                pairs.push((name, value));
            }
        },
    )
    .unwrap();
    let append_key = v8::String::new(scope, "append").unwrap().into();
    params_obj.set(scope, append_key, append_fn.into());

    // Add delete method
    let delete_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {
            let this_obj = args.this();
            if let Some(data) = get_params_data(scope, &this_obj) {
                let name = if let Some(s) = args.get(0).to_string(scope) {
                    s.to_rust_string_lossy(scope)
                } else {
                    return;
                };
                let mut pairs = data.lock().unwrap();
                pairs.retain(|(n, _)| *n != name);
            }
        },
    )
    .unwrap();
    let delete_key = v8::String::new(scope, "delete").unwrap().into();
    params_obj.set(scope, delete_key, delete_fn.into());

    // Add get method
    let get_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this_obj = args.this();
            if let Some(data) = get_params_data(scope, &this_obj) {
                let name = if let Some(s) = args.get(0).to_string(scope) {
                    s.to_rust_string_lossy(scope)
                } else {
                    retval.set(v8::null(scope).into());
                    return;
                };
                let pairs = data.lock().unwrap();
                for (n, v) in pairs.iter() {
                    if n == &name {
                        retval.set(v8::String::new(scope, v).unwrap().into());
                        return;
                    }
                }
                retval.set(v8::null(scope).into());
            } else {
                retval.set(v8::null(scope).into());
            }
        },
    )
    .unwrap();
    let get_key = v8::String::new(scope, "get").unwrap().into();
    params_obj.set(scope, get_key, get_fn.into());

    // Add getAll method
    let get_all_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this_obj = args.this();
            if let Some(data) = get_params_data(scope, &this_obj) {
                let name = if let Some(s) = args.get(0).to_string(scope) {
                    s.to_rust_string_lossy(scope)
                } else {
                    retval.set(v8::null(scope).into());
                    return;
                };
                let pairs = data.lock().unwrap();
                let values: Vec<v8::Local<v8::Value>> = pairs
                    .iter()
                    .filter(|(n, _)| *n == name)
                    .map(|(_, v)| v8::String::new(scope, v).unwrap().into())
                    .collect();
                let arr = v8::Array::new_with_elements(scope, &values);
                retval.set(arr.into());
            } else {
                retval.set(v8::null(scope).into());
            }
        },
    )
    .unwrap();
    let get_all_key = v8::String::new(scope, "getAll").unwrap().into();
    params_obj.set(scope, get_all_key, get_all_fn.into());

    // Add has method
    let has_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this_obj = args.this();
            if let Some(data) = get_params_data(scope, &this_obj) {
                let name = if let Some(s) = args.get(0).to_string(scope) {
                    s.to_rust_string_lossy(scope)
                } else {
                    retval.set(v8::Boolean::new(scope, false).into());
                    return;
                };
                let pairs = data.lock().unwrap();
                let found = pairs.iter().any(|(n, _)| *n == name);
                retval.set(v8::Boolean::new(scope, found).into());
            } else {
                retval.set(v8::Boolean::new(scope, false).into());
            }
        },
    )
    .unwrap();
    let has_key = v8::String::new(scope, "has").unwrap().into();
    params_obj.set(scope, has_key, has_fn.into());

    // Add set method
    let set_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {
            let this_obj = args.this();
            if let Some(data) = get_params_data(scope, &this_obj) {
                let name = if let Some(s) = args.get(0).to_string(scope) {
                    s.to_rust_string_lossy(scope)
                } else {
                    return;
                };
                let value = if let Some(s) = args.get(1).to_string(scope) {
                    s.to_rust_string_lossy(scope)
                } else {
                    return;
                };
                let mut pairs = data.lock().unwrap();
                // Remove existing entries with this name
                pairs.retain(|(n, _)| *n != name);
                // Add the new entry
                pairs.push((name, value));
            }
        },
    )
    .unwrap();
    let set_key = v8::String::new(scope, "set").unwrap().into();
    params_obj.set(scope, set_key, set_fn.into());

    // Add sort method
    let sort_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {
            let this_obj = args.this();
            if let Some(data) = get_params_data(scope, &this_obj) {
                let mut pairs = data.lock().unwrap();
                pairs.sort_by(|(a, _), (b, _)| a.cmp(b));
            }
        },
    )
    .unwrap();
    let sort_key = v8::String::new(scope, "sort").unwrap().into();
    params_obj.set(scope, sort_key, sort_fn.into());

    // Add forEach method
    let for_each_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {
            let this_obj = args.this();
            if let Some(data) = get_params_data(scope, &this_obj) {
                let callback = args.get(0);
                if !callback.is_function() {
                    return;
                }
                let callback_fn = v8::Local::<v8::Function>::try_from(callback).unwrap();
                let pairs = data.lock().unwrap();
                let len = pairs.len();
                let this_arg = args.get(1);
                let receiver = if this_arg.is_undefined() {
                    v8::undefined(scope).into()
                } else {
                    this_arg
                };
                for i in 0..len {
                    if let Some((ref key, ref value)) = pairs.get(i) {
                        let key_val = v8::String::new(scope, key).unwrap().into();
                        let value_val = v8::String::new(scope, value).unwrap().into();
                        let _ = callback_fn.call(
                            scope,
                            receiver,
                            &[value_val, key_val, this_obj.into()],
                        );
                    }
                }
            }
        },
    )
    .unwrap();
    let for_each_key = v8::String::new(scope, "forEach").unwrap().into();
    params_obj.set(scope, for_each_key, for_each_fn.into());

    // Add keys method
    let keys_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this_obj = args.this();
            if let Some(data) = get_params_data(scope, &this_obj) {
                let iterator_template = v8::ObjectTemplate::new(scope);
                iterator_template.set_internal_field_count(1);
                let iterator: v8::Local<v8::Object> = match iterator_template.new_instance(scope) {
                    Some(obj) => obj,
                    None => {
                        retval.set(v8::null(scope).into());
                        return;
                    }
                };

                // Store iterator state: index in internal field 0
                let index_val: v8::Local<v8::Value> = v8::Integer::new(scope, 0).into();
                iterator.set_internal_field(0, index_val.into());

                // Store data reference using External - clone Arc and leak it
                let data_ptr = Arc::into_raw(data) as *mut c_void;
                let external = v8::External::new(scope, data_ptr);
                let data_key = v8::String::new(scope, "_iteratorData").unwrap();
                iterator.set(scope, data_key.into(), external.into());

                // Create next function
                let next_fn = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args_inner: v8::FunctionCallbackArguments,
                     mut retval_inner: v8::ReturnValue| {
                        let iterator_obj = args_inner.this();

                        // Get index - get_internal_field returns Option<Local<Value>>
                        if let Some(index_val) = iterator_obj
                            .get_internal_field(scope, 0)
                            .and_then(|d| v8::Local::<v8::Value>::try_from(d).ok())
                        {
                            let index = index_val
                                .to_integer(scope)
                                .map(|i| i.value() as usize)
                                .unwrap_or(0);

                            // Get data
                            let data_key = v8::String::new(scope, "_iteratorData").unwrap();
                            if let Some(external_val) = iterator_obj.get(scope, data_key.into()) {
                                if external_val.is_external() {
                                    if let Ok(ext_ptr) =
                                        v8::Local::<v8::External>::try_from(external_val)
                                    {
                                        // Increment strong count and create Arc (don't call from_raw)
                                        let raw =
                                            ext_ptr.value() as *mut Mutex<Vec<(String, String)>>;
                                        let data = unsafe {
                                            Arc::increment_strong_count(raw);
                                            Arc::from_raw(raw)
                                        };
                                        let pairs = data.lock().unwrap();

                                        if index >= pairs.len() {
                                            // Done - return {done: true}
                                            let done_obj = v8::Object::new(scope);
                                            let done_key = v8::String::new(scope, "done").unwrap();
                                            let done_val = v8::Boolean::new(scope, true);
                                            done_obj.set(scope, done_key.into(), done_val.into());
                                            retval_inner.set(done_obj.into());
                                            return;
                                        }

                                        // Return {done: false, value: key}
                                        let (ref key, _) = pairs[index];
                                        let result_obj = v8::Object::new(scope);

                                        let done_key = v8::String::new(scope, "done").unwrap();
                                        let done_val = v8::Boolean::new(scope, false);
                                        result_obj.set(scope, done_key.into(), done_val.into());

                                        let value_key = v8::String::new(scope, "value").unwrap();
                                        let key_str = v8::String::new(scope, key).unwrap();
                                        result_obj.set(scope, value_key.into(), key_str.into());

                                        // Increment index - set_internal_field does NOT take scope
                                        let next_index = (index + 1) as i32;
                                        let next_index_val: v8::Local<v8::Value> =
                                            v8::Integer::new(scope, next_index).into();
                                        iterator_obj.set_internal_field(0, next_index_val.into());

                                        retval_inner.set(result_obj.into());
                                        return;
                                    }
                                }
                            }
                        }
                        retval_inner.set(v8::null(scope).into());
                    },
                )
                .unwrap();

                let next_key = v8::String::new(scope, "next").unwrap();
                iterator.set(scope, next_key.into(), next_fn.into());
                set_iterator_returns_self(scope, iterator);

                retval.set(iterator.into());
            } else {
                retval.set(v8::null(scope).into());
            }
        },
    )
    .unwrap();
    let keys_key = v8::String::new(scope, "keys").unwrap().into();
    params_obj.set(scope, keys_key, keys_fn.into());

    // Add values method
    let values_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this_obj = args.this();
            if let Some(data) = get_params_data(scope, &this_obj) {
                let iterator_template = v8::ObjectTemplate::new(scope);
                iterator_template.set_internal_field_count(1);
                let iterator: v8::Local<v8::Object> = match iterator_template.new_instance(scope) {
                    Some(obj) => obj,
                    None => {
                        retval.set(v8::null(scope).into());
                        return;
                    }
                };

                // Store iterator state: index in internal field 0
                let index_val: v8::Local<v8::Value> = v8::Integer::new(scope, 0).into();
                iterator.set_internal_field(0, index_val.into());

                // Store data reference using External - clone Arc and leak it
                let data_ptr = Arc::into_raw(data) as *mut c_void;
                let external = v8::External::new(scope, data_ptr);
                let data_key = v8::String::new(scope, "_iteratorData").unwrap();
                iterator.set(scope, data_key.into(), external.into());

                // Create next function
                let next_fn = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args_inner: v8::FunctionCallbackArguments,
                     mut retval_inner: v8::ReturnValue| {
                        let iterator_obj = args_inner.this();

                        // Get index - get_internal_field returns Option<Local<Value>>
                        if let Some(index_val) = iterator_obj
                            .get_internal_field(scope, 0)
                            .and_then(|d| v8::Local::<v8::Value>::try_from(d).ok())
                        {
                            let index = index_val
                                .to_integer(scope)
                                .map(|i| i.value() as usize)
                                .unwrap_or(0);

                            // Get data
                            let data_key = v8::String::new(scope, "_iteratorData").unwrap();
                            if let Some(external_val) = iterator_obj.get(scope, data_key.into()) {
                                if external_val.is_external() {
                                    if let Ok(ext_ptr) =
                                        v8::Local::<v8::External>::try_from(external_val)
                                    {
                                        // Increment strong count and create Arc (don't call from_raw)
                                        let raw =
                                            ext_ptr.value() as *mut Mutex<Vec<(String, String)>>;
                                        let data = unsafe {
                                            Arc::increment_strong_count(raw);
                                            Arc::from_raw(raw)
                                        };
                                        let pairs = data.lock().unwrap();

                                        if index >= pairs.len() {
                                            // Done - return {done: true}
                                            let done_obj = v8::Object::new(scope);
                                            let done_key = v8::String::new(scope, "done").unwrap();
                                            let done_val = v8::Boolean::new(scope, true);
                                            done_obj.set(scope, done_key.into(), done_val.into());
                                            retval_inner.set(done_obj.into());
                                            return;
                                        }

                                        // Return {done: false, value: value}
                                        let (_, ref value) = pairs[index];
                                        let result_obj = v8::Object::new(scope);

                                        let done_key = v8::String::new(scope, "done").unwrap();
                                        let done_val = v8::Boolean::new(scope, false);
                                        result_obj.set(scope, done_key.into(), done_val.into());

                                        let value_key = v8::String::new(scope, "value").unwrap();
                                        let value_str = v8::String::new(scope, value).unwrap();
                                        result_obj.set(scope, value_key.into(), value_str.into());

                                        // Increment index
                                        let next_index = (index + 1) as i32;
                                        let next_index_val: v8::Local<v8::Value> =
                                            v8::Integer::new(scope, next_index).into();
                                        iterator_obj.set_internal_field(0, next_index_val.into());

                                        retval_inner.set(result_obj.into());
                                        return;
                                    }
                                }
                            }
                        }
                        retval_inner.set(v8::null(scope).into());
                    },
                )
                .unwrap();

                let next_key = v8::String::new(scope, "next").unwrap();
                iterator.set(scope, next_key.into(), next_fn.into());
                set_iterator_returns_self(scope, iterator);

                retval.set(iterator.into());
            } else {
                retval.set(v8::null(scope).into());
            }
        },
    )
    .unwrap();
    let values_key = v8::String::new(scope, "values").unwrap().into();
    params_obj.set(scope, values_key, values_fn.into());

    // Add entries method
    let entries_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this_obj = args.this();
            if let Some(data) = get_params_data(scope, &this_obj) {
                let iterator_template = v8::ObjectTemplate::new(scope);
                iterator_template.set_internal_field_count(1);
                let iterator: v8::Local<v8::Object> = match iterator_template.new_instance(scope) {
                    Some(obj) => obj,
                    None => {
                        retval.set(v8::null(scope).into());
                        return;
                    }
                };

                // Store iterator state: index in internal field 0
                let index_val: v8::Local<v8::Value> = v8::Integer::new(scope, 0).into();
                iterator.set_internal_field(0, index_val.into());

                // Store data reference using External - clone Arc and leak it
                let data_ptr = Arc::into_raw(data) as *mut c_void;
                let external = v8::External::new(scope, data_ptr);
                let data_key = v8::String::new(scope, "_iteratorData").unwrap();
                iterator.set(scope, data_key.into(), external.into());

                // Create next function
                let next_fn = v8::Function::new(
                    scope,
                    |scope: &mut v8::PinScope,
                     args_inner: v8::FunctionCallbackArguments,
                     mut retval_inner: v8::ReturnValue| {
                        let iterator_obj = args_inner.this();

                        // Get index - get_internal_field returns Option<Local<Value>>
                        if let Some(index_val) = iterator_obj
                            .get_internal_field(scope, 0)
                            .and_then(|d| v8::Local::<v8::Value>::try_from(d).ok())
                        {
                            let index = index_val
                                .to_integer(scope)
                                .map(|i| i.value() as usize)
                                .unwrap_or(0);

                            // Get data
                            let data_key = v8::String::new(scope, "_iteratorData").unwrap();
                            if let Some(external_val) = iterator_obj.get(scope, data_key.into()) {
                                if external_val.is_external() {
                                    if let Ok(ext_ptr) =
                                        v8::Local::<v8::External>::try_from(external_val)
                                    {
                                        // Increment strong count and create Arc (don't call from_raw)
                                        let raw =
                                            ext_ptr.value() as *mut Mutex<Vec<(String, String)>>;
                                        let data = unsafe {
                                            Arc::increment_strong_count(raw);
                                            Arc::from_raw(raw)
                                        };
                                        let pairs = data.lock().unwrap();

                                        if index >= pairs.len() {
                                            // Done - return {done: true}
                                            let done_obj = v8::Object::new(scope);
                                            let done_key = v8::String::new(scope, "done").unwrap();
                                            let done_val = v8::Boolean::new(scope, true);
                                            done_obj.set(scope, done_key.into(), done_val.into());
                                            retval_inner.set(done_obj.into());
                                            return;
                                        }

                                        // Return {done: false, value: [key, value]}
                                        let (ref key, ref value) = pairs[index];
                                        let result_obj = v8::Object::new(scope);

                                        let done_key = v8::String::new(scope, "done").unwrap();
                                        let done_val = v8::Boolean::new(scope, false);
                                        result_obj.set(scope, done_key.into(), done_val.into());

                                        let value_key = v8::String::new(scope, "value").unwrap();
                                        // Pre-create strings to avoid multiple mutable borrows of scope
                                        let key_str = v8::String::new(scope, key).unwrap();
                                        let value_str = v8::String::new(scope, value).unwrap();
                                        let arr = v8::Array::new_with_elements(
                                            scope,
                                            &[key_str.into(), value_str.into()],
                                        );
                                        result_obj.set(scope, value_key.into(), arr.into());

                                        // Increment index
                                        let next_index = (index + 1) as i32;
                                        let next_index_val: v8::Local<v8::Value> =
                                            v8::Integer::new(scope, next_index).into();
                                        iterator_obj.set_internal_field(0, next_index_val.into());

                                        retval_inner.set(result_obj.into());
                                        return;
                                    }
                                }
                            }
                        }
                        retval_inner.set(v8::null(scope).into());
                    },
                )
                .unwrap();

                let next_key = v8::String::new(scope, "next").unwrap();
                iterator.set(scope, next_key.into(), next_fn.into());
                set_iterator_returns_self(scope, iterator);

                retval.set(iterator.into());
            } else {
                retval.set(v8::null(scope).into());
            }
        },
    )
    .unwrap();
    let entries_key = v8::String::new(scope, "entries").unwrap().into();
    params_obj.set(scope, entries_key, entries_fn.into());
    if let Some(iterator_key) = symbol_iterator_value(scope) {
        params_obj.set(scope, iterator_key, entries_fn.into());
    }

    // Set the object as the return value
    retval.set(params_obj.into());
}

/// Set up URLSearchParams API
pub fn setup_url_search_params_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Context,
) {
    let global = context.global(scope);
    let existing_key = v8::String::new(scope, "URLSearchParams").unwrap();
    if let Some(existing) = global.get(scope, existing_key.into()) {
        if existing.is_function() {
            return;
        }
    }

    // Create constructor template
    let constructor_template = v8::FunctionTemplate::new(scope, url_search_params_constructor);
    constructor_template.set_class_name(v8::String::new(scope, "URLSearchParams").unwrap());

    // Get the constructor function
    let constructor: v8::Local<v8::Function> = match constructor_template.get_function(scope) {
        Some(c) => c,
        None => return,
    };

    // Set on global object
    let key = v8::String::new(scope, "URLSearchParams").unwrap();
    global.set(scope, key.into(), constructor.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_string() {
        let result = parse_query_string("foo=bar&baz=qux");
        assert_eq!(
            result,
            vec![
                ("foo".to_string(), "bar".to_string()),
                ("baz".to_string(), "qux".to_string())
            ]
        );
    }

    #[test]
    fn test_parse_query_string_with_empty_value() {
        let result = parse_query_string("foo=&bar=baz");
        assert_eq!(
            result,
            vec![
                ("foo".to_string(), "".to_string()),
                ("bar".to_string(), "baz".to_string())
            ]
        );
    }

    #[test]
    fn test_to_query_string() {
        let pairs = vec![
            ("foo".to_string(), "bar".to_string()),
            ("baz".to_string(), "qux".to_string()),
        ];
        let result = to_query_string(&pairs);
        assert!(result.contains("foo=bar"));
        assert!(result.contains("baz=qux"));
    }

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode("foo bar"), "foo+bar");
        assert_eq!(url_encode("a=b&c=d"), "a%3Db%26c%3Dd");
    }

    #[test]
    fn test_url_decode() {
        assert_eq!(url_decode("foo%20bar"), "foo bar");
        assert_eq!(url_decode("a%3Db%26c%3Dd"), "a=b&c=d");
    }
}

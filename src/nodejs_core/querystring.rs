// Node.js querystring模块实现
/// 查询字符串处理
use anyhow::Result;
use rusty_v8 as v8;

/// 设置querystring API
pub fn setup_querystring_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let qs_obj: _ = v8::Object::new(scope);
    // parse
    let parse_func: _ = v8::FunctionTemplate::new(scope, qs_parse_callback);
    let parse_instance: _ = parse_func.get_function(scope).unwrap();
    let parse_key: _ = v8::String::new(scope, "parse").unwrap();
    qs_obj.set(scope, parse_key.into(), parse_instance.into());
    // stringify
    let stringify_func: _ = v8::FunctionTemplate::new(scope, qs_stringify_callback);
    let stringify_instance: _ = stringify_func.get_function(scope).unwrap();
    let stringify_key: _ = v8::String::new(scope, "stringify").unwrap();
    qs_obj.set(scope, stringify_key.into(), stringify_instance.into());
    // escape
    let escape_func: _ = v8::FunctionTemplate::new(scope, qs_escape_callback);
    let escape_instance: _ = escape_func.get_function(scope).unwrap();
    let escape_key: _ = v8::String::new(scope, "escape").unwrap();
    qs_obj.set(scope, escape_key.into(), escape_instance.into());
    // unescape
    let unescape_func: _ = v8::FunctionTemplate::new(scope, qs_unescape_callback);
    let unescape_instance: _ = unescape_func.get_function(scope).unwrap();
    let unescape_key: _ = v8::String::new(scope, "unescape").unwrap();
    qs_obj.set(scope, unescape_key.into(), unescape_instance.into());
    // 设置到全局
    let global: _ = context.global(scope);
    let qs_key: _ = v8::String::new(scope, "querystring").unwrap();
    global.set(scope, qs_key.into(), qs_obj.into());
    Ok(())
}

fn argument_string_or_default(
    scope: &mut v8::PinScope,
    args: &v8::FunctionCallbackArguments,
    index: i32,
    default: &str,
) -> String {
    let value = args.get(index);
    if value.is_undefined() || value.is_null() {
        return default.to_string();
    }
    value
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_else(|| default.to_string())
}

fn encode_component(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}

fn decode_component(value: &str) -> String {
    let plus_normalized = value.replace('+', " ");
    percent_encoding::percent_decode_str(&plus_normalized)
        .decode_utf8_lossy()
        .to_string()
}

fn value_to_string(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> String {
    if value.is_undefined() || value.is_null() {
        return String::new();
    }
    value
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default()
}

fn set_or_append_parse_value(
    scope: &mut v8::PinScope,
    result_obj: v8::Local<v8::Object>,
    key: &str,
    value: &str,
) {
    let key_string = v8::String::new(scope, key).unwrap();
    let value_string: v8::Local<v8::Value> = v8::String::new(scope, value).unwrap().into();
    let existing = result_obj
        .get(scope, key_string.into())
        .unwrap_or_else(|| v8::undefined(scope).into());

    if existing.is_undefined() {
        result_obj.set(scope, key_string.into(), value_string);
    } else if existing.is_array() {
        if let Ok(existing_array) = v8::Local::<v8::Array>::try_from(existing) {
            existing_array.set_index(scope, existing_array.length(), value_string);
        }
    } else {
        let values = v8::Array::new(scope, 2);
        values.set_index(scope, 0, existing);
        values.set_index(scope, 1, value_string);
        result_obj.set(scope, key_string.into(), values.into());
    }
}

fn push_stringified_pair(output: &mut Vec<String>, key: &str, value: &str, eq: &str) {
    output.push(format!(
        "{}{}{}",
        encode_component(key),
        eq,
        encode_component(value)
    ));
}

fn qs_parse_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let str = argument_string_or_default(scope, &args, 0, "");
    let sep = argument_string_or_default(scope, &args, 1, "&");
    let eq = argument_string_or_default(scope, &args, 2, "=");
    let result_obj: _ = v8::Object::new(scope);

    if !str.is_empty() {
        for part in str.split(&sep) {
            if part.is_empty() {
                continue;
            }
            let (key, value) = if let Some(eq_index) = part.find(&eq) {
                (&part[..eq_index], &part[eq_index + eq.len()..])
            } else {
                (part, "")
            };
            let decoded_key = decode_component(key);
            let decoded_value = decode_component(value);
            set_or_append_parse_value(scope, result_obj, &decoded_key, &decoded_value);
        }
    }

    retval.set(result_obj.into());
}

fn qs_stringify_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let value = args.get(0);
    let sep = argument_string_or_default(scope, &args, 1, "&");
    let eq = argument_string_or_default(scope, &args, 2, "=");
    let mut pairs = Vec::new();

    if value.is_object() && !value.is_null() {
        if let Some(obj) = value.to_object(scope) {
            let prop_names = obj
                .get_own_property_names(scope, Default::default())
                .unwrap_or_else(|| v8::Array::new(scope, 0));
            for i in 0..prop_names.length() {
                let Some(key_val) = prop_names.get_index(scope, i) else {
                    continue;
                };
                let Some(key_string) = key_val.to_string(scope) else {
                    continue;
                };
                let key = key_string.to_rust_string_lossy(scope);
                let Some(prop_value) = obj.get(scope, key_val) else {
                    continue;
                };

                if prop_value.is_array() {
                    if let Ok(values) = v8::Local::<v8::Array>::try_from(prop_value) {
                        for value_index in 0..values.length() {
                            let item = values
                                .get_index(scope, value_index)
                                .unwrap_or_else(|| v8::undefined(scope).into());
                            let item_string = value_to_string(scope, item);
                            push_stringified_pair(&mut pairs, &key, &item_string, &eq);
                        }
                    }
                } else {
                    let value_string = value_to_string(scope, prop_value);
                    push_stringified_pair(&mut pairs, &key, &value_string, &eq);
                }
            }
        }
    }

    let result = pairs.join(&sep);
    retval.set(v8::String::new(scope, &result).unwrap().into());
}

fn qs_escape_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let str: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let result: _ = encode_component(&str);
    retval.set(v8::String::new(scope, &result).unwrap().into());
}

fn qs_unescape_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let str: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let result: _ = percent_encoding::percent_decode_str(&str)
        .decode_utf8_lossy()
        .to_string();
    retval.set(v8::String::new(scope, &result).unwrap().into());
}

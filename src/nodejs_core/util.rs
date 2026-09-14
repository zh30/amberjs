// Node.js Util模块实现
/// 实用工具函数
use anyhow::Result;
use rusty_v8 as v8;
/// 设置Util API
pub fn setup_util_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let util_obj: _ = v8::Object::new(scope);
    // inspect
    let inspect_func: _ = v8::FunctionTemplate::new(scope, util_inspect_callback);
    let inspect_instance: _ = inspect_func.get_function(scope).unwrap();
    let inspect_key: _ = v8::String::new(scope, "inspect").unwrap();
    util_obj.set(scope, inspect_key.into(), inspect_instance.into());
    // format
    let format_func: _ = v8::FunctionTemplate::new(scope, util_format_callback);
    let format_instance: _ = format_func.get_function(scope).unwrap();
    let format_key: _ = v8::String::new(scope, "format").unwrap();
    util_obj.set(scope, format_key.into(), format_instance.into());
    // types
    let types_func: _ = v8::FunctionTemplate::new(scope, util_types_callback);
    let types_instance: _ = types_func.get_function(scope).unwrap();
    let types_key: _ = v8::String::new(scope, "types").unwrap();
    util_obj.set(scope, types_key.into(), types_instance.into());
    // isArray
    let is_array_func: _ = v8::FunctionTemplate::new(scope, util_is_array_callback);
    let is_array_instance: _ = is_array_func.get_function(scope).unwrap();
    let is_array_key: _ = v8::String::new(scope, "isArray").unwrap();
    util_obj.set(scope, is_array_key.into(), is_array_instance.into());
    // isBoolean
    let is_bool_func: _ = v8::FunctionTemplate::new(scope, util_is_boolean_callback);
    let is_bool_instance: _ = is_bool_func.get_function(scope).unwrap();
    let is_bool_key: _ = v8::String::new(scope, "isBoolean").unwrap();
    util_obj.set(scope, is_bool_key.into(), is_bool_instance.into());
    // isNull
    let is_null_func: _ = v8::FunctionTemplate::new(scope, util_is_null_callback);
    let is_null_instance: _ = is_null_func.get_function(scope).unwrap();
    let is_null_key: _ = v8::String::new(scope, "isNull").unwrap();
    util_obj.set(scope, is_null_key.into(), is_null_instance.into());
    // isNullOrUndefined
    let is_null_undefined_func: _ =
        v8::FunctionTemplate::new(scope, util_is_null_undefined_callback);
    let is_null_undefined_instance: _ = is_null_undefined_func.get_function(scope).unwrap();
    let is_null_undefined_key: _ = v8::String::new(scope, "isNullOrUndefined").unwrap();
    util_obj.set(
        scope,
        is_null_undefined_key.into(),
        is_null_undefined_instance.into(),
    );
    // isNumber
    let is_number_func: _ = v8::FunctionTemplate::new(scope, util_is_number_callback);
    let is_number_instance: _ = is_number_func.get_function(scope).unwrap();
    let is_number_key: _ = v8::String::new(scope, "isNumber").unwrap();
    util_obj.set(scope, is_number_key.into(), is_number_instance.into());
    // isString
    let is_string_func: _ = v8::FunctionTemplate::new(scope, util_is_string_callback);
    let is_string_instance: _ = is_string_func.get_function(scope).unwrap();
    let is_string_key: _ = v8::String::new(scope, "isString").unwrap();
    util_obj.set(scope, is_string_key.into(), is_string_instance.into());
    // isUndefined
    let is_undefined_func: _ = v8::FunctionTemplate::new(scope, util_is_undefined_callback);
    let is_undefined_instance: _ = is_undefined_func.get_function(scope).unwrap();
    let is_undefined_key: _ = v8::String::new(scope, "isUndefined").unwrap();
    util_obj.set(scope, is_undefined_key.into(), is_undefined_instance.into());
    // isObject
    let is_object_func: _ = v8::FunctionTemplate::new(scope, util_is_object_callback);
    let is_object_instance: _ = is_object_func.get_function(scope).unwrap();
    let is_object_key: _ = v8::String::new(scope, "isObject").unwrap();
    util_obj.set(scope, is_object_key.into(), is_object_instance.into());
    // isFunction
    let is_function_func: _ = v8::FunctionTemplate::new(scope, util_is_function_callback);
    let is_function_instance: _ = is_function_func.get_function(scope).unwrap();
    let is_function_key: _ = v8::String::new(scope, "isFunction").unwrap();
    util_obj.set(scope, is_function_key.into(), is_function_instance.into());
    // promisify
    let promisify_func: _ = v8::FunctionTemplate::new(scope, util_promisify_callback);
    let promisify_instance: _ = promisify_func.get_function(scope).unwrap();
    let promisify_key: _ = v8::String::new(scope, "promisify").unwrap();
    util_obj.set(scope, promisify_key.into(), promisify_instance.into());
    // debuglog
    let debuglog_func: _ = v8::FunctionTemplate::new(scope, util_debuglog_callback);
    let debuglog_instance: _ = debuglog_func.get_function(scope).unwrap();
    let debuglog_key: _ = v8::String::new(scope, "debuglog").unwrap();
    util_obj.set(scope, debuglog_key.into(), debuglog_instance.into());
    // 设置到全局
    let global: _ = context.global(scope);
    let util_key: _ = v8::String::new(scope, "util").unwrap();
    global.set(scope, util_key.into(), util_obj.into());
    Ok(())
}
fn util_inspect_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let object: _ = args.get(0);
    let options: _ = args.get(1);
    let _show_hidden: _ = if !options.is_undefined() {
        let show_hidden_key: _ = v8::String::new(scope, "showHidden").unwrap();
        options
            .to_object(scope)
            .and_then(|obj| obj.get(scope, show_hidden_key.into()))
            .map(|v| v.to_boolean(scope).is_true())
            .unwrap_or(false)
    } else {
        false
    };
    let _depth: _ = if !options.is_undefined() {
        let depth_key: _ = v8::String::new(scope, "depth").unwrap();
        options
            .to_object(scope)
            .and_then(|obj| obj.get(scope, depth_key.into()))
            .unwrap_or(v8::Integer::new(scope, 2).into())
            .to_integer(scope)
            .unwrap()
            .value() as i32
    } else {
        2
    };
    // 简化的inspect实现
    let result: _ = if object.is_string() {
        format!(
            "'{}'",
            object.to_string(scope).unwrap().to_rust_string_lossy(scope)
        )
    } else if object.is_number() {
        object
            .to_number(scope)
            .unwrap()
            .to_string(scope)
            .unwrap()
            .to_rust_string_lossy(scope)
    } else if object.is_boolean() {
        if object.to_boolean(scope).is_true() {
            "true".to_string()
        } else {
            "false".to_string()
        }
    } else if object.is_null() {
        "null".to_string()
    } else if object.is_undefined() {
        "undefined".to_string()
    } else if object.is_array() {
        let arr: _ = v8::Local::<v8::Array>::try_from(object).unwrap();
        let length: _ = arr.length();
        format!("Array({}) [{} items]", length, length)
    } else if object.is_object() {
        let obj: _ = object.to_object(scope).unwrap();
        let key_count: _ = get_object_key_count(obj, scope);
        format!("{{}} {} keys", key_count)
    } else {
        "[Unknown]".to_string()
    };
    retval.set(v8::String::new(scope, &result).unwrap().into());
}
fn util_format_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let format_str: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let mut result = String::new();
    let mut arg_index = 1;
    let mut i = 0;
    while i < format_str.len() {
        if format_str.chars().nth(i) == Some('%') && i + 1 < format_str.len() {
            let format_char: _ = format_str.chars().nth(i + 1).unwrap();
            match format_char {
                's' => {
                    if arg_index < args.length() {
                        let arg: _ = args.get(arg_index);
                        let arg_str: _ = if arg.is_string() {
                            arg.to_string(scope).unwrap().to_rust_string_lossy(scope)
                        } else if arg.is_number() {
                            arg.to_number(scope)
                                .unwrap()
                                .to_string(scope)
                                .unwrap()
                                .to_rust_string_lossy(scope)
                        } else if arg.is_boolean() {
                            if arg.to_boolean(scope).is_true() {
                                "true".to_string()
                            } else {
                                "false".to_string()
                            }
                        } else if arg.is_null() {
                            "null".to_string()
                        } else if arg.is_undefined() {
                            "undefined".to_string()
                        } else {
                            "[Object]".to_string()
                        };
                        result.push_str(&arg_str);
                        arg_index += 1;
                    }
                    i += 2;
                }
                'd' | 'i' => {
                    if arg_index < args.length() {
                        let arg: _ = args.get(arg_index);
                        if arg.is_number() {
                            result.push_str(
                                &arg.to_number(scope)
                                    .unwrap()
                                    .to_string(scope)
                                    .unwrap()
                                    .to_rust_string_lossy(scope),
                            );
                        } else {
                            result.push_str("NaN");
                        }
                        arg_index += 1;
                    }
                    i += 2;
                }
                'f' => {
                    if arg_index < args.length() {
                        let arg: _ = args.get(arg_index);
                        if arg.is_number() {
                            result.push_str(
                                &arg.to_number(scope)
                                    .unwrap()
                                    .to_string(scope)
                                    .unwrap()
                                    .to_rust_string_lossy(scope),
                            );
                        } else {
                            result.push_str("NaN");
                        }
                        arg_index += 1;
                    }
                    i += 2;
                }
                'j' => {
                    if arg_index < args.length() {
                        let arg: _ = args.get(arg_index);
                        if arg.is_object() {
                            result.push_str("{}");
                        } else {
                            result.push_str("[Circular]");
                        }
                        arg_index += 1;
                    }
                    i += 2;
                }
                '%' => {
                    result.push('%');
                    i += 2;
                }
                _ => {
                    result.push('%');
                    result.push(format_char);
                    i += 2;
                }
            }
        } else {
            result.push(format_str.chars().nth(i).unwrap());
            i += 1;
        }
    }
    // 添加剩余参数
    while arg_index < args.length() {
        if !result.is_empty() {
            result.push(' ');
        }
        let arg: _ = args.get(arg_index);
        if arg.is_string() {
            result.push_str(&arg.to_string(scope).unwrap().to_rust_string_lossy(scope));
        } else if arg.is_number() {
            result.push_str(
                &arg.to_number(scope)
                    .unwrap()
                    .to_string(scope)
                    .unwrap()
                    .to_rust_string_lossy(scope),
            );
        } else if arg.is_boolean() {
            result.push_str(if arg.to_boolean(scope).is_true() {
                "true"
            } else {
                "false"
            });
        } else {
            result.push_str("[Object]");
        }
        arg_index += 1;
    }
    retval.set(v8::String::new(scope, &result).unwrap().into());
}
fn util_types_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let types_obj: _ = v8::Object::new(scope);
    // isDate
    let is_date_func: _ = v8::FunctionTemplate::new(scope, util_is_date_callback);
    let is_date_instance: _ = is_date_func.get_function(scope).unwrap();
    let is_date_key: _ = v8::String::new(scope, "isDate").unwrap();
    types_obj.set(scope, is_date_key.into(), is_date_instance.into());
    // isRegExp
    let is_regex_func: _ = v8::FunctionTemplate::new(scope, util_is_regex_callback);
    let is_regex_instance: _ = is_regex_func.get_function(scope).unwrap();
    let is_regex_key: _ = v8::String::new(scope, "isRegExp").unwrap();
    types_obj.set(scope, is_regex_key.into(), is_regex_instance.into());
    // isError
    let is_error_func: _ = v8::FunctionTemplate::new(scope, util_is_error_callback);
    let is_error_instance: _ = is_error_func.get_function(scope).unwrap();
    let is_error_key: _ = v8::String::new(scope, "isError").unwrap();
    types_obj.set(scope, is_error_key.into(), is_error_instance.into());
    // isNativeError
    let is_native_error_func: _ = v8::FunctionTemplate::new(scope, util_is_native_error_callback);
    let is_native_error_instance: _ = is_native_error_func.get_function(scope).unwrap();
    let is_native_error_key: _ = v8::String::new(scope, "isNativeError").unwrap();
    types_obj.set(
        scope,
        is_native_error_key.into(),
        is_native_error_instance.into(),
    );
    retval.set(types_obj.into());
}
fn util_is_array_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let value: _ = args.get(0);
    let is_array: _ = value.is_array();
    retval.set(v8::Boolean::new(scope, is_array).into());
}
fn util_is_boolean_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let value: _ = args.get(0);
    let is_boolean: _ = value.is_boolean();
    retval.set(v8::Boolean::new(scope, is_boolean).into());
}
fn util_is_null_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let value: _ = args.get(0);
    let is_null: _ = value.is_null();
    retval.set(v8::Boolean::new(scope, is_null).into());
}
fn util_is_null_undefined_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let value: _ = args.get(0);
    let is_null_or_undefined: _ = value.is_null() || value.is_undefined();
    retval.set(v8::Boolean::new(scope, is_null_or_undefined).into());
}
fn util_is_number_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let value: _ = args.get(0);
    let is_number: _ = value.is_number();
    retval.set(v8::Boolean::new(scope, is_number).into());
}
fn util_is_string_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let value: _ = args.get(0);
    let is_string: _ = value.is_string();
    retval.set(v8::Boolean::new(scope, is_string).into());
}
fn util_is_undefined_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let value: _ = args.get(0);
    let is_undefined: _ = value.is_undefined();
    retval.set(v8::Boolean::new(scope, is_undefined).into());
}
fn util_is_object_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let value: _ = args.get(0);
    let is_object: _ = value.is_object() && !value.is_null() && !value.is_array();
    retval.set(v8::Boolean::new(scope, is_object).into());
}
fn util_is_function_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let value: _ = args.get(0);
    let is_function: _ = value.is_function();
    retval.set(v8::Boolean::new(scope, is_function).into());
}
fn util_promisify_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let original: _ = args.get(0);
    if !original.is_function() {
        retval.set(v8::null(scope).into());
        return;
    }
    // 创建promisified函数
    let promisified_func: _ = v8::FunctionTemplate::new(scope, util_promisified_callback);
    let promisified_instance: _ = promisified_func.get_function(scope).unwrap();
    // 保存原始函数
    let original_key: _ = v8::String::new(scope, "_original").unwrap();
    promisified_instance.set(scope, original_key.into(), original);
    retval.set(promisified_instance.into());
}
fn util_promisified_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // 简化的promisify实现 - 返回Promise对象
    let this: _ = args.this();
    let original_key: _ = v8::String::new(scope, "_original").unwrap();
    let _original_func: _ = this.get(scope, original_key.into());
    // 返回一个模拟的Promise
    let promise_obj: _ = v8::Object::new(scope);
    let then_func: _ = v8::FunctionTemplate::new(scope, util_promise_then_callback);
    let then_instance: _ = then_func.get_function(scope).unwrap();
    let then_key: _ = v8::String::new(scope, "then").unwrap();
    promise_obj.set(scope, then_key.into(), then_instance.into());
    retval.set(promise_obj.into());
}
fn util_promise_then_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let on_fulfilled: _ = args.get(0);
    if on_fulfilled.is_function() {
        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_fulfilled) {
            let undefined: _ = v8::undefined(scope);
            func.call(scope, undefined.into(), &[]);
        }
    }
    retval.set(v8::undefined(scope).into());
}
fn util_debuglog_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let section: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    // 创建debuglog函数
    let debuglog_func: _ = v8::FunctionTemplate::new(scope, util_debuglog_func_callback);
    let debuglog_instance: _ = debuglog_func.get_function(scope).unwrap();
    // 保存section
    let section_key: _ = v8::String::new(scope, "_section").unwrap();
    let section_val: _ = v8::String::new(scope, &section).unwrap();
    debuglog_instance.set(scope, section_key.into(), section_val.into());
    retval.set(debuglog_instance.into());
}
fn util_debuglog_func_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let section_key: _ = v8::String::new(scope, "_section").unwrap();
    let section: _ = this
        .get(scope, section_key.into())
        .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
        .unwrap_or_default();
    let mut message = String::new();
    for i in 0..args.length() {
        if i > 0 {
            message.push(' ');
        }
        let arg: _ = args.get(i);
        if arg.is_string() {
            message.push_str(&arg.to_string(scope).unwrap().to_rust_string_lossy(scope));
        } else if arg.is_number() {
            message.push_str(
                &arg.to_number(scope)
                    .unwrap()
                    .to_string(scope)
                    .unwrap()
                    .to_rust_string_lossy(scope),
            );
        } else {
            message.push_str("[Object]");
        }
    }
    eprintln!("[DEBUG:{}] {}", section, message);
    retval.set(v8::undefined(scope).into());
}
// 辅助函数
fn get_object_key_count(_obj: v8::Local<v8::Object>, _scope: &mut v8::PinScope) -> usize {
    // 简化的实现，返回固定值
    3
}
// 类型检查辅助函数
fn util_is_date_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let _value: _ = _args.get(0);
    let is_date: _ = false; // 简化实现
    retval.set(v8::Boolean::new(scope, is_date).into());
}
fn util_is_regex_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let _value: _ = args.get(0);
    let is_regex: _ = false; // 简化实现
    retval.set(v8::Boolean::new(scope, is_regex).into());
}
fn util_is_error_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let _value: _ = args.get(0);
    let is_error: _ = false; // 简化实现
    retval.set(v8::Boolean::new(scope, is_error).into());
}
fn util_is_native_error_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let _value: _ = args.get(0);
    let is_native_error: _ = false; // 简化实现
    retval.set(v8::Boolean::new(scope, is_native_error).into());
}

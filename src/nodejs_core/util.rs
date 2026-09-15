// Node.js Util模块实现
/// 实用工具函数
use anyhow::Result;
use rusty_v8 as v8;

pub fn setup_util_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Create util object
    let util_obj = v8::Object::new(scope);

    // inspect function
    let inspect_func = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let object = args.get(0);
            let result = if object.is_null() {
                "null".to_string()
            } else if object.is_undefined() {
                "undefined".to_string()
            } else if object.is_string() {
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
                object.to_boolean(scope).is_true().to_string()
            } else if object.is_array() {
                let arr = v8::Local::<v8::Array>::try_from(object).unwrap();
                format!("Array({})", arr.length())
            } else if object.is_object() {
                let obj = object.to_object(scope).unwrap();
                if let Some(keys) = obj.get_own_property_names(scope, Default::default()) {
                    let mut entries = Vec::new();
                    for i in 0..keys.length() {
                        if let Some(key) = keys.get_index(scope, i) {
                            let key_str = key
                                .to_string(scope)
                                .map(|s| s.to_rust_string_lossy(scope))
                                .unwrap_or_default();
                            let value = obj
                                .get(scope, key.into())
                                .unwrap_or_else(|| v8::undefined(scope).into());
                            let value_str = if value.is_string() {
                                format!(
                                    "'{}'",
                                    value.to_string(scope).unwrap().to_rust_string_lossy(scope)
                                )
                            } else {
                                value
                                    .to_string(scope)
                                    .map(|s| s.to_rust_string_lossy(scope))
                                    .unwrap_or_else(|| "[Object]".to_string())
                            };
                            entries.push(format!("{}: {}", key_str, value_str));
                        }
                    }
                    format!("{{ {} }}", entries.join(", "))
                } else {
                    "Object {}".to_string()
                }
            } else {
                "[Unknown]".to_string()
            };
            retval.set(v8::String::new(scope, &result).unwrap().into());
        },
    );
    let inspect_instance = inspect_func.get_function(scope).unwrap();
    let inspect_key = v8::String::new(scope, "inspect").unwrap();
    util_obj.set(scope, inspect_key.into(), inspect_instance.into());

    // format function
    let format_func = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let format_str = args
                .get(0)
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();
            let mut result = String::new();
            let mut arg_index = 1;
            let mut i = 0;
            while i < format_str.len() {
                if format_str.chars().nth(i) == Some('%') && i + 1 < format_str.len() {
                    let format_char = format_str.chars().nth(i + 1).unwrap();
                    match format_char {
                        's' | 'd' | 'i' | 'f' => {
                            if arg_index < args.length() {
                                let arg = args.get(arg_index);
                                let arg_str = if arg.is_string() {
                                    arg.to_string(scope).unwrap().to_rust_string_lossy(scope)
                                } else if arg.is_number() {
                                    arg.to_number(scope)
                                        .unwrap()
                                        .to_string(scope)
                                        .unwrap()
                                        .to_rust_string_lossy(scope)
                                } else if arg.is_boolean() {
                                    arg.to_boolean(scope).is_true().to_string()
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
                        'j' => {
                            result.push_str("[Object]");
                            arg_index += 1;
                            i += 2;
                        }
                        '%' => {
                            result.push('%');
                            i += 2;
                        }
                        _ => {
                            result.push(format_char);
                            i += 1;
                        }
                    }
                } else {
                    result.push(format_str.chars().nth(i).unwrap());
                    i += 1;
                }
            }
            // Add remaining arguments
            while arg_index < args.length() {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(
                    &args
                        .get(arg_index)
                        .to_string(scope)
                        .unwrap()
                        .to_rust_string_lossy(scope),
                );
                arg_index += 1;
            }
            retval.set(v8::String::new(scope, &result).unwrap().into());
        },
    );
    let format_instance = format_func.get_function(scope).unwrap();
    let format_key = v8::String::new(scope, "format").unwrap();
    util_obj.set(scope, format_key.into(), format_instance.into());

    // types object
    let types_obj = v8::Object::new(scope);
    let is_array_buffer_key = v8::String::new(scope, "isArrayBuffer").unwrap();
    let is_array_buffer_value = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut _retval: v8::ReturnValue| {
            _retval.set(v8::Boolean::new(_scope, false).into());
        },
    )
    .get_function(scope)
    .unwrap();
    types_obj.set(
        scope,
        is_array_buffer_key.into(),
        is_array_buffer_value.into(),
    );

    let is_date_key = v8::String::new(scope, "isDate").unwrap();
    let is_date_value = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(v8::Boolean::new(_scope, args.get(0).is_date()).into());
        },
    )
    .get_function(scope)
    .unwrap();
    types_obj.set(scope, is_date_key.into(), is_date_value.into());

    let is_regexp_key = v8::String::new(scope, "isRegExp").unwrap();
    let is_regexp_value = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut _retval: v8::ReturnValue| {
            _retval.set(v8::Boolean::new(_scope, false).into());
        },
    )
    .get_function(scope)
    .unwrap();
    types_obj.set(scope, is_regexp_key.into(), is_regexp_value.into());

    let types_key = v8::String::new(scope, "types").unwrap();
    util_obj.set(scope, types_key.into(), types_obj.into());

    // isArray function
    let is_array_func = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(v8::Boolean::new(_scope, args.get(0).is_array()).into());
        },
    )
    .get_function(scope)
    .unwrap();
    let is_array_key = v8::String::new(scope, "isArray").unwrap();
    util_obj.set(scope, is_array_key.into(), is_array_func.into());

    // isBoolean function
    let is_bool_func = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(v8::Boolean::new(_scope, args.get(0).is_boolean()).into());
        },
    )
    .get_function(scope)
    .unwrap();
    let is_bool_key = v8::String::new(scope, "isBoolean").unwrap();
    util_obj.set(scope, is_bool_key.into(), is_bool_func.into());

    // isNull function
    let is_null_func = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(v8::Boolean::new(_scope, args.get(0).is_null()).into());
        },
    )
    .get_function(scope)
    .unwrap();
    let is_null_key = v8::String::new(scope, "isNull").unwrap();
    util_obj.set(scope, is_null_key.into(), is_null_func.into());

    // isNumber function
    let is_number_func = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(v8::Boolean::new(_scope, args.get(0).is_number()).into());
        },
    )
    .get_function(scope)
    .unwrap();
    let is_number_key = v8::String::new(scope, "isNumber").unwrap();
    util_obj.set(scope, is_number_key.into(), is_number_func.into());

    // isString function
    let is_string_func = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(v8::Boolean::new(_scope, args.get(0).is_string()).into());
        },
    )
    .get_function(scope)
    .unwrap();
    let is_string_key = v8::String::new(scope, "isString").unwrap();
    util_obj.set(scope, is_string_key.into(), is_string_func.into());

    // is_undefined function
    let is_undefined_func = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(v8::Boolean::new(_scope, args.get(0).is_undefined()).into());
        },
    )
    .get_function(scope)
    .unwrap();
    let is_undefined_key = v8::String::new(scope, "is_undefined").unwrap();
    util_obj.set(scope, is_undefined_key.into(), is_undefined_func.into());

    // isObject function
    let is_object_func = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let val = args.get(0);
            retval.set(v8::Boolean::new(_scope, val.is_object() && !val.is_null()).into());
        },
    )
    .get_function(scope)
    .unwrap();
    let is_object_key = v8::String::new(scope, "isObject").unwrap();
    util_obj.set(scope, is_object_key.into(), is_object_func.into());

    // isFunction function
    let is_function_func = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(v8::Boolean::new(_scope, args.get(0).is_function()).into());
        },
    )
    .get_function(scope)
    .unwrap();
    let is_function_key = v8::String::new(scope, "isFunction").unwrap();
    util_obj.set(scope, is_function_key.into(), is_function_func.into());

    // promisify function
    let promisify_func = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(v8::undefined(_scope).into());
        },
    )
    .get_function(scope)
    .unwrap();
    let promisify_key = v8::String::new(scope, "promisify").unwrap();
    util_obj.set(scope, promisify_key.into(), promisify_func.into());

    // debuglog function
    let debuglog_func = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(v8::undefined(_scope).into());
        },
    )
    .get_function(scope)
    .unwrap();
    let debuglog_key = v8::String::new(scope, "debuglog").unwrap();
    util_obj.set(scope, debuglog_key.into(), debuglog_func.into());

    // Set util as global
    let util_key = v8::String::new(scope, "util").unwrap();
    global.set(scope, util_key.into(), util_obj.into());

    let util_script = r#"
    (function() {
        if (!globalThis.util) {
            globalThis.util = {};
        }
        const util = globalThis.util;

        util.debuglog = function(section, callback) {
            function logger(...args) {}
            logger.enabled = false;
            return logger;
        };

        util.deprecate = function(fn, msg, code) {
            if (typeof fn !== 'function') {
                throw new TypeError('The "fn" argument must be of type function');
            }
            let warned = false;
            function deprecated(...args) {
                if (!warned) {
                    warned = true;
                    if (typeof process !== 'undefined' && typeof process.emitWarning === 'function') {
                        process.emitWarning(msg, 'DeprecationWarning', code);
                    }
                }
                if (new.target) {
                    return Reflect.construct(fn, args, new.target);
                }
                return fn.apply(this, args);
            }
            Object.setPrototypeOf(deprecated, fn);
            if (fn.prototype) {
                deprecated.prototype = fn.prototype;
            }
            return deprecated;
        };

        util.inherits = function(ctor, superCtor) {
            if (ctor === undefined || ctor === null)
                throw new TypeError('The constructor to "inherits" must not be empty');
            if (superCtor === undefined || superCtor === null)
                throw new TypeError('The super constructor to "inherits" must not be empty');
            if (superCtor.prototype === undefined)
                throw new TypeError('The super constructor to "inherits" must have a prototype');
            ctor.super_ = superCtor;
            if (!ctor.prototype) ctor.prototype = Object.create(superCtor.prototype);
            else Object.setPrototypeOf(ctor.prototype, superCtor.prototype);
        };

        util.callbackify = function(original) {
            if (typeof original !== 'function') {
                throw new TypeError('The "original" argument must be of type function');
            }
            return function(...args) {
                const cb = args.pop();
                if (typeof cb !== 'function') {
                    throw new TypeError('The last argument must be of type function');
                }
                Reflect.apply(original, this, args).then(
                    ret => { cb(null, ret); },
                    rej => { cb(rej); }
                );
            };
        };

        const kCustomPromisify = Symbol.for('nodejs.util.promisify.custom');
        util.promisify = function(original) {
            if (typeof original !== 'function') {
                throw new TypeError('The "original" argument must be of type function');
            }
            if (original[kCustomPromisify]) {
                return original[kCustomPromisify];
            }
            function fn(...args) {
                return new Promise((resolve, reject) => {
                    try {
                        original.call(this, ...args, (err, ...values) => {
                            if (err) {
                                return reject(err);
                            }
                            if (values.length <= 1) {
                                resolve(values[0]);
                            } else {
                                resolve(values);
                            }
                        });
                    } catch (err) {
                        reject(err);
                    }
                });
            }
            Object.setPrototypeOf(fn, Object.getPrototypeOf(original));
            Object.defineProperties(fn, Object.getOwnPropertyDescriptors(original));
            return fn;
        };
        util.promisify.custom = kCustomPromisify;

        util.isUndefined = function(v) { return v === undefined; };

        if (!util.types) {
            util.types = {};
        }
        const types = util.types;
        types.isAnyArrayBuffer = types.isAnyArrayBuffer || (v => v instanceof ArrayBuffer || (typeof SharedArrayBuffer !== 'undefined' && v instanceof SharedArrayBuffer));
        types.isArrayBuffer = types.isArrayBuffer || (v => v instanceof ArrayBuffer);
        types.isArgumentsObject = types.isArgumentsObject || (v => Object.prototype.toString.call(v) === '[object Arguments]');
        types.isAsyncFunction = types.isAsyncFunction || (v => typeof v === 'function' && v.constructor && v.constructor.name === 'AsyncFunction');
        types.isBooleanObject = types.isBooleanObject || (v => typeof v === 'object' && v !== null && Object.prototype.toString.call(v) === '[object Boolean]');
        types.isBoxedPrimitive = types.isBoxedPrimitive || (v => typeof v === 'object' && v !== null && (v instanceof Number || v instanceof String || v instanceof Boolean || v instanceof Symbol || (typeof BigInt !== 'undefined' && v instanceof BigInt)));
        types.isDataView = types.isDataView || (v => v instanceof DataView);
        types.isDate = types.isDate || (v => v instanceof Date);
        types.isGeneratorFunction = types.isGeneratorFunction || (v => typeof v === 'function' && v.constructor && v.constructor.name === 'GeneratorFunction');
        types.isMap = types.isMap || (v => v instanceof Map);
        types.isNativeError = types.isNativeError || (v => v instanceof Error);
        types.isNumberObject = types.isNumberObject || (v => typeof v === 'object' && v !== null && Object.prototype.toString.call(v) === '[object Number]');
        types.isPromise = types.isPromise || (v => v instanceof Promise || (v && typeof v.then === 'function'));
        types.isRegExp = types.isRegExp || (v => v instanceof RegExp);
        types.isSet = types.isSet || (v => v instanceof Set);
        types.isStringObject = types.isStringObject || (v => typeof v === 'object' && v !== null && Object.prototype.toString.call(v) === '[object String]');
        types.isSymbolObject = types.isSymbolObject || (v => typeof v === 'object' && v !== null && Object.prototype.toString.call(v) === '[object Symbol]');
        types.isTypedArray = types.isTypedArray || (v => ArrayBuffer.isView(v) && !(v instanceof DataView));
        types.isWeakMap = types.isWeakMap || (v => v instanceof WeakMap);
        types.isWeakSet = types.isWeakSet || (v => v instanceof WeakSet);
    })();
    "#;
    if let Some(code) = v8::String::new(scope, util_script) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }

    Ok(())
}

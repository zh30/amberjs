// Node.js Events模块实现
// 事件驱动编程的核心模块

use std::sync::Mutex;

use anyhow::Result;
use rusty_v8 as v8;
use std::collections::HashMap;
thread_local! {
    static EVENT_LISTENERS: Mutex<HashMap<String, Vec<v8::Global<v8::Function>>>> = Mutex::new(HashMap::new());
    static ONCE_LISTENERS: Mutex<HashMap<String, Vec<v8::Global<v8::Function>>>> = Mutex::new(HashMap::new());
}
/// 设置Events API
pub fn setup_events_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    // EventEmitter构造函数
    let event_emitter_constructor: _ =
        v8::FunctionTemplate::new(scope, event_emitter_constructor_callback);
    // 添加静态方法
    // EventEmitter.listenerCount()
    let listener_count_func: _ =
        v8::FunctionTemplate::new(scope, event_emitter_listener_count_callback);
    let listener_count_instance: _ = listener_count_func.get_function(scope).unwrap();
    // set_on_instance has been removed, use instance template instead
    let listener_count_key: _ = v8::String::new(scope, "listenerCount").unwrap();
    event_emitter_constructor.set(listener_count_key.into(), listener_count_instance.into());
    // 创建构造函数实例
    let event_emitter_func: _ = event_emitter_constructor.get_function(scope).unwrap();
    // 设置到全局
    //
    // Node exports the class itself, with a self-reference so that both
    // `const E = require('events')` and `const { EventEmitter } =
    // require('events')` yield a constructor.
    let global: _ = context.global(scope);
    let events_key: _ = v8::String::new(scope, "events").unwrap();
    let event_emitter_key: _ = v8::String::new(scope, "EventEmitter").unwrap();
    event_emitter_func.set(scope, event_emitter_key.into(), event_emitter_func.into());
    global.set(scope, events_key.into(), event_emitter_func.into());
    Ok(())
}
fn event_emitter_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Use the 'this' object that V8 creates for the new instance
    let emitter_obj = args.this();

    // on(eventName, listener)
    let on_func = v8::FunctionTemplate::new(scope, event_emitter_on_callback);
    let on_instance = on_func.get_function(scope).unwrap();
    let on_key = v8::String::new(scope, "on").unwrap();
    emitter_obj.set(scope, on_key.into(), on_instance.into());

    // v0.3.257: prependListener(eventName, listener) - 添加到队列开头
    let prepend_func = v8::FunctionTemplate::new(scope, event_emitter_prepend_listener_callback);
    let prepend_instance = prepend_func.get_function(scope).unwrap();
    let prepend_key = v8::String::new(scope, "prependListener").unwrap();
    emitter_obj.set(scope, prepend_key.into(), prepend_instance.into());

    // once(eventName, listener)
    let once_func = v8::FunctionTemplate::new(scope, event_emitter_once_callback);
    let once_instance = once_func.get_function(scope).unwrap();
    let once_key = v8::String::new(scope, "once").unwrap();
    emitter_obj.set(scope, once_key.into(), once_instance.into());

    // v0.3.258: prependOnceListener(eventName, listener) - 一次性监听器，添加到队列开头
    let prepend_once_func =
        v8::FunctionTemplate::new(scope, event_emitter_prepend_once_listener_callback);
    let prepend_once_instance = prepend_once_func.get_function(scope).unwrap();
    let prepend_once_key = v8::String::new(scope, "prependOnceListener").unwrap();
    emitter_obj.set(scope, prepend_once_key.into(), prepend_once_instance.into());

    // emit(eventName, ...args)
    let emit_func = v8::FunctionTemplate::new(scope, event_emitter_emit_callback);
    let emit_instance = emit_func.get_function(scope).unwrap();
    let emit_key = v8::String::new(scope, "emit").unwrap();
    emitter_obj.set(scope, emit_key.into(), emit_instance.into());
    // removeListener(eventName, listener)
    let remove_listener_func =
        v8::FunctionTemplate::new(scope, event_emitter_remove_listener_callback);
    let remove_listener_instance = remove_listener_func.get_function(scope).unwrap();
    let remove_listener_key = v8::String::new(scope, "removeListener").unwrap();
    emitter_obj.set(
        scope,
        remove_listener_key.into(),
        remove_listener_instance.into(),
    );
    // removeAllListeners(eventName)
    let remove_all_func = v8::FunctionTemplate::new(scope, event_emitter_remove_all_callback);
    let remove_all_instance = remove_all_func.get_function(scope).unwrap();
    let remove_all_key = v8::String::new(scope, "removeAllListeners").unwrap();
    emitter_obj.set(scope, remove_all_key.into(), remove_all_instance.into());
    // listeners(eventName)
    let listeners_func = v8::FunctionTemplate::new(scope, event_emitter_listeners_callback);
    let listeners_instance = listeners_func.get_function(scope).unwrap();
    let listeners_key = v8::String::new(scope, "listeners").unwrap();
    emitter_obj.set(scope, listeners_key.into(), listeners_instance.into());
    // eventNames()
    let event_names_func = v8::FunctionTemplate::new(scope, event_emitter_event_names_callback);
    let event_names_instance = event_names_func.get_function(scope).unwrap();
    let event_names_key = v8::String::new(scope, "eventNames").unwrap();
    emitter_obj.set(scope, event_names_key.into(), event_names_instance.into());
    // getMaxListeners()
    let get_max_func = v8::FunctionTemplate::new(scope, event_emitter_get_max_callback);
    let get_max_instance = get_max_func.get_function(scope).unwrap();
    let get_max_key = v8::String::new(scope, "getMaxListeners").unwrap();
    emitter_obj.set(scope, get_max_key.into(), get_max_instance.into());
    // setMaxListeners(n)
    let set_max_func = v8::FunctionTemplate::new(scope, event_emitter_set_max_callback);
    let set_max_instance = set_max_func.get_function(scope).unwrap();
    let set_max_key = v8::String::new(scope, "setMaxListeners").unwrap();
    emitter_obj.set(scope, set_max_key.into(), set_max_instance.into());
    // 添加_maxListeners属性
    let max_listeners_key = v8::String::new(scope, "_maxListeners").unwrap();
    let max_val = v8::Integer::new(scope, 10);
    emitter_obj.set(scope, max_listeners_key.into(), max_val.into());
    // Return the constructed object (V8 handles this for constructors, but we set it explicitly)
    retval.set(emitter_obj.into());
}
fn event_emitter_on_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let event_name: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let listener: _ = args.get(1);
    if !listener.is_function() {
        retval.set(v8::null(scope).into());
        return;
    }
    // 创建持久化函数引用
    let listener_func: _ = v8::Local::<v8::Function>::try_from(listener).unwrap();
    let function_global: _ = v8::Global::new(scope, listener_func);

    // v0.3.243: 检查是否超过 maxListeners 并发出警告
    let current_count = EVENT_LISTENERS.with(|map| {
        let map_ref = map.lock().unwrap();
        map_ref.get(&event_name).map(|v| v.len()).unwrap_or(0)
    });

    // 获取 maxListeners
    let max_key: _ = v8::String::new(scope, "_maxListeners").unwrap();
    let max_listeners = this
        .get(scope, max_key.into())
        .and_then(|v| v.to_integer(scope))
        .map(|v| v.value() as usize)
        .unwrap_or(10);

    // 如果超过限制，发出警告
    if max_listeners > 0 && current_count >= max_listeners {
        emit_max_listeners_warning(scope, &this, &event_name, current_count + 1, max_listeners);
    }

    // 获取当前的监听器数组
    let listeners_key: _ = v8::String::new(scope, "_listeners").unwrap();
    let existing_listeners: _ = this.get(scope, listeners_key.into());
    let mut listeners_map: HashMap<String, Vec<v8::Global<v8::Function>>> = HashMap::new();
    if let Some(arr) = existing_listeners.and_then(|v| v.to_object(scope)) {
        // 转换现有的监听器
        let listener_names: _ = EVENT_LISTENERS.with(|map| {
            let map_ref: _ = map.lock().unwrap();
            map_ref.keys().cloned().collect::<Vec<_>>()
        });
        for name in listener_names {
            let name_key: _ = v8::String::new(scope, &name).unwrap();
            // 检查属性是否存在（简化实现）
            let prop: _ = arr.get(scope, name_key.into());
            if prop.is_some() && !prop.unwrap().is_undefined() {
                listeners_map.insert(name, vec![]); // 简化实现
            }
        }
    }
    // 添加新监听器
    EVENT_LISTENERS.with(|map| {
        let mut map_ref = map.lock().unwrap();
        map_ref
            .entry(event_name.clone())
            .or_insert_with(Vec::new)
            .push(function_global);
    });
    // 在对象上设置属性标记
    let prop_key: _ = v8::String::new(scope, &event_name).unwrap();
    let val: _ = v8::Boolean::new(scope, true).into();
    this.set(scope, prop_key.into(), val);
    retval.set(this.into());
}

/// v0.3.257: prependListener 回调
/// 与 on 类似，但将监听器添加到事件队列的开头（优先执行）
fn event_emitter_prepend_listener_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let event_name: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let listener: _ = args.get(1);
    if !listener.is_function() {
        retval.set(v8::null(scope).into());
        return;
    }
    // 创建持久化函数引用
    let listener_func: _ = v8::Local::<v8::Function>::try_from(listener).unwrap();
    let function_global: _ = v8::Global::new(scope, listener_func);

    // 检查 maxListeners 警告
    let current_count = EVENT_LISTENERS.with(|map| {
        let map_ref = map.lock().unwrap();
        map_ref.get(&event_name).map(|v| v.len()).unwrap_or(0)
    });

    let max_key: _ = v8::String::new(scope, "_maxListeners").unwrap();
    let max_listeners = this
        .get(scope, max_key.into())
        .and_then(|v| v.to_integer(scope))
        .map(|v| v.value() as usize)
        .unwrap_or(10);

    if max_listeners > 0 && current_count >= max_listeners {
        emit_max_listeners_warning(scope, &this, &event_name, current_count + 1, max_listeners);
    }

    // 添加监听器到队列开头（与 on 的区别）
    EVENT_LISTENERS.with(|map| {
        let mut map_ref = map.lock().unwrap();
        let listeners = map_ref.entry(event_name.clone()).or_insert_with(Vec::new);
        listeners.insert(0, function_global); // 插入到开头
    });

    let prop_key: _ = v8::String::new(scope, &event_name).unwrap();
    let val: _ = v8::Boolean::new(scope, true).into();
    this.set(scope, prop_key.into(), val);
    retval.set(this.into());
}

fn event_emitter_once_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let event_name: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let listener: _ = args.get(1);
    if !listener.is_function() {
        retval.set(v8::null(scope).into());
        return;
    }
    let listener_func: _ = v8::Local::<v8::Function>::try_from(listener).unwrap();
    let function_global: _ = v8::Global::new(scope, listener_func);

    // v0.3.243: 检查是否超过 maxListeners 并发出警告
    let current_count = ONCE_LISTENERS.with(|map| {
        let map_ref = map.lock().unwrap();
        map_ref.get(&event_name).map(|v| v.len()).unwrap_or(0)
    });

    // 获取 maxListeners
    let max_key: _ = v8::String::new(scope, "_maxListeners").unwrap();
    let max_listeners = this
        .get(scope, max_key.into())
        .and_then(|v| v.to_integer(scope))
        .map(|v| v.value() as usize)
        .unwrap_or(10);

    // 如果超过限制，发出警告
    if max_listeners > 0 && current_count >= max_listeners {
        emit_max_listeners_warning(scope, &this, &event_name, current_count + 1, max_listeners);
    }

    // 添加一次性监听器
    ONCE_LISTENERS.with(|map| {
        let mut map_ref = map.lock().unwrap();
        map_ref
            .entry(event_name.clone())
            .or_insert_with(Vec::new)
            .push(function_global);
    });
    let prop_key: _ = v8::String::new(scope, &event_name).unwrap();
    let prop_val: _ = v8::Boolean::new(scope, true);
    this.set(scope, prop_key.into(), prop_val.into());
    retval.set(this.into());
}

/// v0.3.258: prependOnceListener 回调
/// 与 once 类似，但将一次性监听器添加到队列的开头（优先执行）
fn event_emitter_prepend_once_listener_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let event_name: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let listener: _ = args.get(1);
    if !listener.is_function() {
        retval.set(v8::null(scope).into());
        return;
    }
    let listener_func: _ = v8::Local::<v8::Function>::try_from(listener).unwrap();
    let function_global: _ = v8::Global::new(scope, listener_func);

    // 检查 maxListeners 警告
    let current_count = ONCE_LISTENERS.with(|map| {
        let map_ref = map.lock().unwrap();
        map_ref.get(&event_name).map(|v| v.len()).unwrap_or(0)
    });

    let max_key: _ = v8::String::new(scope, "_maxListeners").unwrap();
    let max_listeners = this
        .get(scope, max_key.into())
        .and_then(|v| v.to_integer(scope))
        .map(|v| v.value() as usize)
        .unwrap_or(10);

    if max_listeners > 0 && current_count >= max_listeners {
        emit_max_listeners_warning(scope, &this, &event_name, current_count + 1, max_listeners);
    }

    // 添加一次性监听器到队列开头（与 once 的区别）
    ONCE_LISTENERS.with(|map| {
        let mut map_ref = map.lock().unwrap();
        let listeners = map_ref.entry(event_name.clone()).or_insert_with(Vec::new);
        listeners.insert(0, function_global); // 插入到开头
    });

    let prop_key: _ = v8::String::new(scope, &event_name).unwrap();
    let prop_val: _ = v8::Boolean::new(scope, true);
    this.set(scope, prop_key.into(), prop_val.into());
    retval.set(this.into());
}

fn event_emitter_emit_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let event_name: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let mut event_args: Vec<v8::Local<v8::Value>> = Vec::new();
    for i in 1..args.length() {
        event_args.push(args.get(i));
    }
    let mut emitted = false;
    // 调用普通监听器
    EVENT_LISTENERS.with(|map| {
        let map_ref: _ = map.lock().unwrap();
        if let Some(listeners) = map_ref.get(&event_name) {
            for listener in listeners {
                let listener_func: _ = v8::Local::new(scope, listener);
                listener_func.call(scope, this.into(), &event_args);
                emitted = true;
            }
        }
    });
    // 调用一次性监听器并移除
    let mut executed_once = Vec::new();
    ONCE_LISTENERS.with(|map| {
        let mut map_ref = map.lock().unwrap();
        if let Some(listeners) = map_ref.get_mut(&event_name) {
            for listener in listeners.iter() {
                let listener_func: _ = v8::Local::new(scope, listener);
                listener_func.call(scope, this.into(), &event_args);
                executed_once.push(listener.clone());
                emitted = true;
            }
            // 移除已执行的一次性监听器
            listeners.retain(|l| !executed_once.contains(l));
        }
    });
    retval.set(v8::Boolean::new(scope, emitted).into());
}
fn event_emitter_remove_listener_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let event_name: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let listener: _ = args.get(1);
    if !listener.is_function() {
        retval.set(v8::null(scope).into());
        return;
    }
    // 简化实现：移除事件标记
    let prop_key: _ = v8::String::new(scope, &event_name).unwrap();
    this.delete(scope, prop_key.into());
    retval.set(this.into());
}
fn event_emitter_remove_all_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let event_name: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    if event_name.is_empty() {
        // 移除所有事件
        EVENT_LISTENERS.with(|map| {
            let mut map_ref = map.lock().unwrap();
            map_ref.clear();
        });
        ONCE_LISTENERS.with(|map| {
            let mut map_ref = map.lock().unwrap();
            map_ref.clear();
        });
    } else {
        // 移除特定事件
        EVENT_LISTENERS.with(|map| {
            let mut map_ref = map.lock().unwrap();
            map_ref.remove(&event_name);
        });
        ONCE_LISTENERS.with(|map| {
            let mut map_ref = map.lock().unwrap();
            map_ref.remove(&event_name);
        });
    }
    retval.set(this.into());
}
fn event_emitter_listeners_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let event_name: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let listeners_array: _ = v8::Array::new(scope, 0);
    EVENT_LISTENERS.with(|map| {
        let map_ref: _ = map.lock().unwrap();
        if let Some(listeners) = map_ref.get(&event_name) {
            for (i, listener) in listeners.iter().enumerate() {
                let listener_func: _ = v8::Local::new(scope, listener);
                listeners_array.set_index(scope, i as u32, listener_func.into());
            }
        }
    });
    retval.set(listeners_array.into());
}
fn event_emitter_event_names_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let names_array: _ = v8::Array::new(scope, 0);
    EVENT_LISTENERS.with(|map| {
        let map_ref: _ = map.lock().unwrap();
        for (i, (name, _)) in map_ref.iter().enumerate() {
            let name_str: _ = v8::String::new(scope, name).unwrap();
            names_array.set_index(scope, i as u32, name_str.into());
        }
    });
    retval.set(names_array.into());
}
fn event_emitter_listener_count_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let _emitter: _ = args.get(0);
    let event_name: _ = args
        .get(1)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let mut count = 0;
    EVENT_LISTENERS.with(|map| {
        let map_ref: _ = map.lock().unwrap();
        if let Some(listeners) = map_ref.get(&event_name) {
            count = listeners.len();
        }
    });
    retval.set(v8::Integer::new(scope, count as i32).into());
}
fn event_emitter_get_max_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let max_key: _ = v8::String::new(scope, "_maxListeners").unwrap();
    let max: _ = this
        .get(scope, max_key.into())
        .unwrap_or(v8::Integer::new(scope, 10).into());
    retval.set(max);
}
fn event_emitter_set_max_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let n: _ = args
        .get(0)
        .to_integer(scope)
        .unwrap_or(v8::Integer::new(scope, 10))
        .value() as i32;
    let max_key: _ = v8::String::new(scope, "_maxListeners").unwrap();
    let max_key_val: _ = v8::Integer::new(scope, n).into();
    this.set(scope, max_key.into(), max_key_val);
    retval.set(this.into());
}

/// v0.3.243: 发出 MaxListenersExceededWarning 警告
/// 当添加的监听器数量超过 maxListeners 时调用 console.warn
fn emit_max_listeners_warning(
    scope: &mut v8::PinScope,
    _emitter: &v8::Local<v8::Object>,
    event_name: &str,
    current_count: usize,
    _max_listeners: usize,
) {
    // 构建警告消息
    let warning_msg = format!(
        "MaxListenersExceededWarning: Possible EventEmitter memory leak detected. {} listeners added to {} event. Use emitter.setMaxListeners() to increase limit",
        current_count,
        event_name
    );

    // 调用 console.warn
    let console_key = v8::String::new(scope, "console").unwrap();
    let console = scope
        .get_current_context()
        .global(scope)
        .get(scope, console_key.into());

    if let Some(console_obj) = console {
        if console_obj.is_object() {
            let console_obj = console_obj.to_object(scope).unwrap();
            let warn_key = v8::String::new(scope, "warn").unwrap();
            let warn_method = console_obj.get(scope, warn_key.into());

            if let Some(warn_func) = warn_method {
                if warn_func.is_function() {
                    let warn_func = v8::Local::<v8::Function>::try_from(warn_func).unwrap();
                    let msg_str = v8::String::new(scope, &warning_msg).unwrap();
                    let _console = console_obj;
                    let _ = warn_func.call(scope, _console.into(), &[msg_str.into()]);
                }
            }
        }
    }
}

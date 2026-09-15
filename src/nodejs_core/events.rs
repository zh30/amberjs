// Node.js Events模块实现
// 事件驱动编程的核心模块
use anyhow::Result;
use rusty_v8 as v8;
use std::collections::HashMap;
use std::sync::Mutex;

thread_local! {
    pub static EVENT_LISTENERS: Mutex<HashMap<String, Vec<v8::Global<v8::Function>>>> = Mutex::new(HashMap::new());
    pub static ONCE_LISTENERS: Mutex<HashMap<String, Vec<v8::Global<v8::Function>>>> = Mutex::new(HashMap::new());
    pub static PREPEND_ONCE_LISTENERS: Mutex<HashMap<String, Vec<v8::Global<v8::Function>>>> = Mutex::new(HashMap::new());
}

pub fn setup_events_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // EventEmitter constructor
    let event_emitter_constructor = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let _ = args; // args not used in constructor
            let emitter_obj = v8::Object::new(scope);

            // Note: Full instanceof support requires prototype chain setup after constructor is created
            // This is handled in setup_events_api after getting the function

            // on(eventName, listener)
            let on_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let event_name = args
                        .get(0)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    let listener = args.get(1);

                    if !listener.is_function() {
                        retval.set(v8::null(scope).into());
                        return;
                    }

                    let listener_func = v8::Local::<v8::Function>::try_from(listener).unwrap();
                    let function_global = v8::Global::new(scope, listener_func);

                    // v0.3.243: Check if adding listener exceeds maxListeners and emit warning
                    let current_count = EVENT_LISTENERS.with(|map| {
                        let map_ref = map.lock().unwrap();
                        map_ref.get(&event_name).map(|v| v.len()).unwrap_or(0)
                    });

                    // Get maxListeners from the emitter
                    let max_key = v8::String::new(scope, "_maxListeners").unwrap();
                    let max_listeners = this
                        .get(scope, max_key.into())
                        .and_then(|v| v.to_integer(scope))
                        .map(|v| v.value() as usize)
                        .unwrap_or(10);

                    // Emit warning if exceeding maxListeners
                    if max_listeners > 0 && current_count >= max_listeners {
                        let warning_msg = format!(
                    "MaxListenersExceededWarning: Possible EventEmitter memory leak detected. {} listeners added to {} event. Use emitter.setMaxListeners() to increase limit",
                    current_count + 1,
                    event_name
                );
                        // Call console.warn
                        let console_key = v8::String::new(scope, "console").unwrap();
                        let console = scope
                            .get_current_context()
                            .global(scope)
                            .get(scope, console_key.into());
                        if let Some(console_obj) = console.and_then(|c| c.to_object(scope)) {
                            let warn_key = v8::String::new(scope, "warn").unwrap();
                            if let Some(warn_func) = console_obj
                                .get(scope, warn_key.into())
                                .and_then(|f| v8::Local::<v8::Function>::try_from(f).ok())
                            {
                                let msg_str = v8::String::new(scope, &warning_msg).unwrap();
                                let _ =
                                    warn_func.call(scope, console_obj.into(), &[msg_str.into()]);
                            }
                        }
                    }

                    EVENT_LISTENERS.with(|map| {
                        let mut map_ref = map.lock().unwrap();
                        map_ref
                            .entry(event_name.clone())
                            .or_insert_with(Vec::new)
                            .push(function_global);
                    });

                    // Set event flag on object
                    let prop_key = v8::String::new(scope, &event_name).unwrap();
                    let val = v8::Boolean::new(scope, true);
                    this.set(scope, prop_key.into(), val.into());
                    retval.set(this.into());
                },
            );
            let on_instance = on_func.get_function(scope).unwrap();
            let on_key = v8::String::new(scope, "on").unwrap();
            emitter_obj.set(scope, on_key.into(), on_instance.into());

            // v0.3.257: prependListener(eventName, listener) - adds listener to the beginning of the listener array
            let prepend_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let event_name = args
                        .get(0)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    let listener = args.get(1);

                    if !listener.is_function() {
                        retval.set(v8::null(scope).into());
                        return;
                    }

                    let listener_func = v8::Local::<v8::Function>::try_from(listener).unwrap();
                    let function_global = v8::Global::new(scope, listener_func);

                    // v0.3.257: Check if adding listener exceeds maxListeners and emit warning
                    let current_count = EVENT_LISTENERS.with(|map| {
                        let map_ref = map.lock().unwrap();
                        map_ref.get(&event_name).map(|v| v.len()).unwrap_or(0)
                    });

                    // Get maxListeners from the emitter
                    let max_key = v8::String::new(scope, "_maxListeners").unwrap();
                    let max_listeners = this
                        .get(scope, max_key.into())
                        .and_then(|v| v.to_integer(scope))
                        .map(|v| v.value() as usize)
                        .unwrap_or(10);

                    // Emit warning if exceeding maxListeners
                    if max_listeners > 0 && current_count >= max_listeners {
                        let warning_msg = format!(
                    "MaxListenersExceededWarning: Possible EventEmitter memory leak detected. {} listeners added to {} event. Use emitter.setMaxListeners() to increase limit",
                    current_count + 1,
                    event_name
                );
                        // Call console.warn
                        let console_key = v8::String::new(scope, "console").unwrap();
                        let console = scope
                            .get_current_context()
                            .global(scope)
                            .get(scope, console_key.into());
                        if let Some(console_obj) = console.and_then(|c| c.to_object(scope)) {
                            let warn_key = v8::String::new(scope, "warn").unwrap();
                            if let Some(warn_func) = console_obj
                                .get(scope, warn_key.into())
                                .and_then(|f| v8::Local::<v8::Function>::try_from(f).ok())
                            {
                                let msg_str = v8::String::new(scope, &warning_msg).unwrap();
                                let _ =
                                    warn_func.call(scope, console_obj.into(), &[msg_str.into()]);
                            }
                        }
                    }

                    // v0.3.257: Add listener to the BEGINNING of the listener array (unlike on which adds to the end)
                    EVENT_LISTENERS.with(|map| {
                        let mut map_ref = map.lock().unwrap();
                        let listeners = map_ref.entry(event_name.clone()).or_insert_with(Vec::new);
                        listeners.insert(0, function_global); // Insert at beginning
                    });

                    // Set event flag on object
                    let prop_key = v8::String::new(scope, &event_name).unwrap();
                    let val = v8::Boolean::new(scope, true);
                    this.set(scope, prop_key.into(), val.into());
                    retval.set(this.into());
                },
            );
            let prepend_instance = prepend_func.get_function(scope).unwrap();
            let prepend_key = v8::String::new(scope, "prependListener").unwrap();
            emitter_obj.set(scope, prepend_key.into(), prepend_instance.into());

            // once(eventName, listener)
            let once_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let event_name = args
                        .get(0)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    let listener = args.get(1);

                    if !listener.is_function() {
                        retval.set(v8::null(scope).into());
                        return;
                    }

                    let listener_func = v8::Local::<v8::Function>::try_from(listener).unwrap();
                    let function_global = v8::Global::new(scope, listener_func);

                    // v0.3.243: Check if adding listener exceeds maxListeners and emit warning
                    // Count both ONCE_LISTENERS and PREPEND_ONCE_LISTENERS for maxListeners
                    let current_count = ONCE_LISTENERS.with(|map| {
                        let map_ref = map.lock().unwrap();
                        map_ref.get(&event_name).map(|v| v.len()).unwrap_or(0)
                    }) + PREPEND_ONCE_LISTENERS.with(|map| {
                        let map_ref = map.lock().unwrap();
                        map_ref.get(&event_name).map(|v| v.len()).unwrap_or(0)
                    });

                    // Get maxListeners from the emitter
                    let max_key = v8::String::new(scope, "_maxListeners").unwrap();
                    let max_listeners = this
                        .get(scope, max_key.into())
                        .and_then(|v| v.to_integer(scope))
                        .map(|v| v.value() as usize)
                        .unwrap_or(10);

                    // Emit warning if exceeding maxListeners
                    if max_listeners > 0 && current_count >= max_listeners {
                        let warning_msg = format!(
                    "MaxListenersExceededWarning: Possible EventEmitter memory leak detected. {} once listeners added to {} event. Use emitter.setMaxListeners() to increase limit",
                    current_count + 1,
                    event_name
                );
                        // Call console.warn
                        let console_key = v8::String::new(scope, "console").unwrap();
                        let console = scope
                            .get_current_context()
                            .global(scope)
                            .get(scope, console_key.into());
                        if let Some(console_obj) = console.and_then(|c| c.to_object(scope)) {
                            let warn_key = v8::String::new(scope, "warn").unwrap();
                            if let Some(warn_func) = console_obj
                                .get(scope, warn_key.into())
                                .and_then(|f| v8::Local::<v8::Function>::try_from(f).ok())
                            {
                                let msg_str = v8::String::new(scope, &warning_msg).unwrap();
                                let _ =
                                    warn_func.call(scope, console_obj.into(), &[msg_str.into()]);
                            }
                        }
                    }

                    // Add to ONCE_LISTENERS (append at end)
                    ONCE_LISTENERS.with(|map| {
                        let mut map_ref = map.lock().unwrap();
                        map_ref
                            .entry(event_name.clone())
                            .or_insert_with(Vec::new)
                            .push(function_global);
                    });

                    let prop_key = v8::String::new(scope, &event_name).unwrap();
                    let prop_val = v8::Boolean::new(scope, true);
                    this.set(scope, prop_key.into(), prop_val.into());
                    retval.set(this.into());
                },
            );
            let once_instance = once_func.get_function(scope).unwrap();
            let once_key = v8::String::new(scope, "once").unwrap();
            emitter_obj.set(scope, once_key.into(), once_instance.into());

            // emit(eventName, ...args)
            let emit_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let event_name = args
                        .get(0)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();

                    let mut event_args: Vec<v8::Local<v8::Value>> = Vec::new();
                    for i in 1..args.length() {
                        event_args.push(args.get(i));
                    }

                    let mut emitted = false;

                    // v0.3.258: Correct execution order: prependOnce -> prepend -> once -> on
                    // PREPEND_ONCE_LISTENERS -> EVENT_LISTENERS -> ONCE_LISTENERS

                    // Execute PREPEND_ONCE_LISTENERS (prependOnceListener) first
                    let mut prepend_once_to_remove: Vec<v8::Global<v8::Function>> = Vec::new();
                    PREPEND_ONCE_LISTENERS.with(|map| {
                        let mut map_ref = map.lock().unwrap();
                        if let Some(listeners) = map_ref.get_mut(&event_name) {
                            for listener in listeners.iter() {
                                let listener_func = v8::Local::new(scope, listener);
                                listener_func.call(scope, this.into(), &event_args);
                                prepend_once_to_remove.push(listener.clone());
                                emitted = true;
                            }
                            listeners.retain(|l| !prepend_once_to_remove.contains(l));
                        }
                    });

                    // Execute EVENT_LISTENERS (prependListener and on) second
                    let mut regular_to_execute: Vec<v8::Global<v8::Function>> = Vec::new();
                    EVENT_LISTENERS.with(|map| {
                        let map_ref = map.lock().unwrap();
                        if let Some(listeners) = map_ref.get(&event_name) {
                            for listener in listeners.iter() {
                                regular_to_execute.push(listener.clone());
                            }
                        }
                    });
                    for listener in regular_to_execute.iter() {
                        let listener_func = v8::Local::new(scope, listener);
                        listener_func.call(scope, this.into(), &event_args);
                        emitted = true;
                    }

                    // Execute ONCE_LISTENERS (once) last
                    let mut once_to_remove: Vec<v8::Global<v8::Function>> = Vec::new();
                    ONCE_LISTENERS.with(|map| {
                        let mut map_ref = map.lock().unwrap();
                        if let Some(listeners) = map_ref.get_mut(&event_name) {
                            for listener in listeners.iter() {
                                let listener_func = v8::Local::new(scope, listener);
                                listener_func.call(scope, this.into(), &event_args);
                                once_to_remove.push(listener.clone());
                                emitted = true;
                            }
                            listeners.retain(|l| !once_to_remove.contains(l));
                        }
                    });

                    retval.set(v8::Boolean::new(scope, emitted).into());
                },
            );
            // once_instance and once_key are already set above at lines 14353-14355

            // v0.3.258: prependOnceListener - one-time listener added to the front of the queue
            let _prepend_once_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let event_name = args
                        .get(0)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    let listener = args.get(1);

                    if !listener.is_function() {
                        retval.set(v8::null(scope).into());
                        return;
                    }

                    let listener_func = v8::Local::<v8::Function>::try_from(listener).unwrap();
                    let function_global = v8::Global::new(scope, listener_func);

                    // Check maxListeners (count both ONCE_LISTENERS and PREPEND_ONCE_LISTENERS)
                    let max_key = v8::String::new(scope, "_maxListeners").unwrap();
                    let max_listeners = this
                        .get(scope, max_key.into())
                        .and_then(|v| v.to_integer(scope))
                        .map(|v| v.value() as usize)
                        .unwrap_or(10);

                    let current_count = ONCE_LISTENERS.with(|map| {
                        let map_ref = map.lock().unwrap();
                        map_ref.get(&event_name).map(|v| v.len()).unwrap_or(0)
                    }) + PREPEND_ONCE_LISTENERS.with(|map| {
                        let map_ref = map.lock().unwrap();
                        map_ref.get(&event_name).map(|v| v.len()).unwrap_or(0)
                    });

                    // Emit warning if exceeding maxListeners
                    if max_listeners > 0 && current_count >= max_listeners {
                        let warning_msg = format!(
                    "MaxListenersExceededWarning: Possible EventEmitter memory leak detected. {} once listeners added to {} event. Use emitter.setMaxListeners() to increase limit",
                    current_count + 1,
                    event_name
                );
                        // Call console.warn
                        let console_key = v8::String::new(scope, "console").unwrap();
                        let console = scope
                            .get_current_context()
                            .global(scope)
                            .get(scope, console_key.into());
                        if let Some(console_obj) = console.and_then(|c| c.to_object(scope)) {
                            let warn_key = v8::String::new(scope, "warn").unwrap();
                            if let Some(warn_func) = console_obj
                                .get(scope, warn_key.into())
                                .and_then(|f| v8::Local::<v8::Function>::try_from(f).ok())
                            {
                                let msg_str = v8::String::new(scope, &warning_msg).unwrap();
                                let _ =
                                    warn_func.call(scope, console_obj.into(), &[msg_str.into()]);
                            }
                        }
                    }

                    // Add to PREPEND_ONCE_LISTENERS (append at end - will be executed before EVENT_LISTENERS)
                    PREPEND_ONCE_LISTENERS.with(|map| {
                        let mut map_ref = map.lock().unwrap();
                        map_ref
                            .entry(event_name.clone())
                            .or_insert_with(Vec::new)
                            .push(function_global);
                    });

                    let prop_key = v8::String::new(scope, &event_name).unwrap();
                    let prop_val = v8::Boolean::new(scope, true);
                    this.set(scope, prop_key.into(), prop_val.into());
                    retval.set(this.into());
                },
            );
            let prepend_once_instance = _prepend_once_func.get_function(scope).unwrap();
            let prepend_once_key = v8::String::new(scope, "prependOnceListener").unwrap();
            emitter_obj.set(scope, prepend_once_key.into(), prepend_once_instance.into());

            let emit_instance = emit_func.get_function(scope).unwrap();
            let emit_key = v8::String::new(scope, "emit").unwrap();
            emitter_obj.set(scope, emit_key.into(), emit_instance.into());

            // removeListener(eventName, listener)
            let remove_listener_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let event_name = args
                        .get(0)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    let listener = args.get(1);

                    // Remove from EVENT_LISTENERS
                    if listener.is_function() {
                        EVENT_LISTENERS.with(|map| {
                            let mut map_ref = map.lock().unwrap();
                            if let Some(listeners) = map_ref.get_mut(&event_name) {
                                listeners.retain(|global_func| {
                                    let local_func = v8::Local::new(scope, global_func);
                                    !local_func.strict_equals(listener)
                                });
                            }
                        });
                        // Also check ONCE_LISTENERS
                        ONCE_LISTENERS.with(|map| {
                            let mut map_ref = map.lock().unwrap();
                            if let Some(listeners) = map_ref.get_mut(&event_name) {
                                listeners.retain(|global_func| {
                                    let local_func = v8::Local::new(scope, global_func);
                                    !local_func.strict_equals(listener)
                                });
                            }
                        });
                    }

                    // Remove event flag
                    let prop_key = v8::String::new(scope, &event_name).unwrap();
                    this.delete(scope, prop_key.into());
                    retval.set(this.into());
                },
            );
            let remove_listener_instance = remove_listener_func.get_function(scope).unwrap();
            let remove_listener_key = v8::String::new(scope, "removeListener").unwrap();
            emitter_obj.set(
                scope,
                remove_listener_key.into(),
                remove_listener_instance.into(),
            );

            // removeAllListeners([eventName])
            let remove_all_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let event_name = args
                        .get(0)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();

                    if event_name.is_empty() {
                        EVENT_LISTENERS.with(|map| {
                            let mut map_ref = map.lock().unwrap();
                            map_ref.clear();
                        });
                        ONCE_LISTENERS.with(|map| {
                            let mut map_ref = map.lock().unwrap();
                            map_ref.clear();
                        });
                    } else {
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
                },
            );
            let remove_all_instance = remove_all_func.get_function(scope).unwrap();
            let remove_all_key = v8::String::new(scope, "removeAllListeners").unwrap();
            emitter_obj.set(scope, remove_all_key.into(), remove_all_instance.into());

            // listeners(eventName)
            let listeners_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let event_name = args
                        .get(0)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    let listeners_array = v8::Array::new(scope, 0);

                    EVENT_LISTENERS.with(|map| {
                        let map_ref = map.lock().unwrap();
                        if let Some(listeners) = map_ref.get(&event_name) {
                            for (i, listener) in listeners.iter().enumerate() {
                                let listener_func = v8::Local::new(scope, listener);
                                listeners_array.set_index(scope, i as u32, listener_func.into());
                            }
                        }
                    });
                    retval.set(listeners_array.into());
                },
            );
            let listeners_instance = listeners_func.get_function(scope).unwrap();
            let listeners_key = v8::String::new(scope, "listeners").unwrap();
            emitter_obj.set(scope, listeners_key.into(), listeners_instance.into());

            // eventNames()
            let event_names_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let names_array = v8::Array::new(scope, 0);
                    EVENT_LISTENERS.with(|map| {
                        let map_ref = map.lock().unwrap();
                        for (i, (name, _)) in map_ref.iter().enumerate() {
                            let name_str = v8::String::new(scope, name).unwrap();
                            names_array.set_index(scope, i as u32, name_str.into());
                        }
                    });
                    retval.set(names_array.into());
                },
            );
            let event_names_instance = event_names_func.get_function(scope).unwrap();
            let event_names_key = v8::String::new(scope, "eventNames").unwrap();
            emitter_obj.set(scope, event_names_key.into(), event_names_instance.into());

            // getMaxListeners()
            let get_max_func = v8::FunctionTemplate::new(
                scope,
                |_scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let max_key = v8::String::new(_scope, "_maxListeners").unwrap();
                    let max = this
                        .get(_scope, max_key.into())
                        .unwrap_or(v8::Integer::new(_scope, 10).into());
                    retval.set(max);
                },
            );
            let get_max_instance = get_max_func.get_function(scope).unwrap();
            let get_max_key = v8::String::new(scope, "getMaxListeners").unwrap();
            emitter_obj.set(scope, get_max_key.into(), get_max_instance.into());

            // setMaxListeners(n)
            let set_max_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let n = args
                        .get(0)
                        .to_integer(scope)
                        .unwrap_or(v8::Integer::new(scope, 10))
                        .value() as i32;
                    let max_key = v8::String::new(scope, "_maxListeners").unwrap();
                    let max_key_val = v8::Integer::new(scope, n).into();
                    this.set(scope, max_key.into(), max_key_val);
                    retval.set(this.into());
                },
            );
            let set_max_instance = set_max_func.get_function(scope).unwrap();
            let set_max_key = v8::String::new(scope, "setMaxListeners").unwrap();
            emitter_obj.set(scope, set_max_key.into(), set_max_instance.into());

            // _maxListeners property (default 10)
            let max_listeners_key = v8::String::new(scope, "_maxListeners").unwrap();
            let max_val = v8::Integer::new(scope, 10);
            emitter_obj.set(scope, max_listeners_key.into(), max_val.into());

            retval.set(emitter_obj.into());
        },
    );

    // Get the EventEmitter function
    let event_emitter_func = event_emitter_constructor.get_function(scope).unwrap();

    // Node exports the class itself, with a self-reference so that both
    // `const E = require('events')` and `const { EventEmitter } =
    // require('events')` yield a constructor.
    let event_emitter_key = v8::String::new(scope, "EventEmitter").unwrap();
    event_emitter_func.set(scope, event_emitter_key.into(), event_emitter_func.into());
    event_emitter_func.set(scope, event_emitter_key.into(), event_emitter_func.into());

    // Add static method listenerCount(emitter, eventName)
    let listener_count_func = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let _emitter = args.get(0);
            let event_name = args
                .get(1)
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();
            let mut count = 0;

            // Count regular listeners
            EVENT_LISTENERS.with(|map| {
                let map_ref = map.lock().unwrap();
                if let Some(listeners) = map_ref.get(&event_name) {
                    count += listeners.len();
                }
            });

            // Count prependOnce listeners
            PREPEND_ONCE_LISTENERS.with(|map| {
                let map_ref = map.lock().unwrap();
                if let Some(listeners) = map_ref.get(&event_name) {
                    count += listeners.len();
                }
            });

            // Count once listeners
            ONCE_LISTENERS.with(|map| {
                let map_ref = map.lock().unwrap();
                if let Some(listeners) = map_ref.get(&event_name) {
                    count += listeners.len();
                }
            });

            retval.set(v8::Integer::new(scope, count as i32).into());
        },
    );
    let listener_count_instance = listener_count_func.get_function(scope).unwrap();
    let listener_count_key = v8::String::new(scope, "listenerCount").unwrap();
    event_emitter_func.set(
        scope,
        listener_count_key.into(),
        listener_count_instance.into(),
    );

    // Set events as global
    let events_key = v8::String::new(scope, "events").unwrap();
    global.set(scope, events_key.into(), event_emitter_func.into());

    let events_bootstrap = r#"
    (function() {
        function EventEmitter() {
            this._events = Object.create(null);
            this._eventsCount = 0;
            this._maxListeners = undefined;
        }
        EventEmitter.defaultMaxListeners = 10;

        EventEmitter.prototype.setMaxListeners = function(n) {
            if (typeof n !== 'number' || n < 0 || Number.isNaN(n)) {
                throw new RangeError('The value of "n" is out of range');
            }
            this._maxListeners = n;
            return this;
        };

        EventEmitter.prototype.getMaxListeners = function() {
            return this._maxListeners === undefined ? EventEmitter.defaultMaxListeners : this._maxListeners;
        };

        EventEmitter.prototype.emit = function(type, ...args) {
            const events = this._events;
            if (events === undefined) return false;
            const handler = events[type];
            if (handler === undefined) {
                if (type === 'error') {
                    const er = args[0];
                    if (er instanceof Error) throw er;
                    const err = new Error('Unhandled error.' + (er ? ' (' + er + ')' : ''));
                    err.context = er;
                    throw err;
                }
                return false;
            }
            if (typeof handler === 'function') {
                const len = args.length;
                if (len === 1) handler.call(this, args[0]);
                else if (len === 0) handler.call(this);
                else if (len === 2) handler.call(this, args[0], args[1]);
                else handler.apply(this, args);
                return true;
            }
            const listeners = handler.slice();
            const lCount = listeners.length;
            const len = args.length;
            if (len === 1) {
                for (let i = 0; i < lCount; ++i) listeners[i].call(this, args[0]);
            } else if (len === 0) {
                for (let i = 0; i < lCount; ++i) listeners[i].call(this);
            } else {
                for (let i = 0; i < lCount; ++i) listeners[i].apply(this, args);
            }
            return true;
        };

        EventEmitter.prototype.addListener = function(type, listener, prepend) {
            if (typeof listener !== 'function') {
                throw new TypeError('The "listener" argument must be of type Function');
            }
            if (!this._events) {
                this._events = Object.create(null);
                this._eventsCount = 0;
            }
            if (this._events.newListener) {
                this.emit('newListener', type, listener.listener || listener);
            }
            const existing = this._events[type];
            if (!existing) {
                this._events[type] = listener;
                this._eventsCount++;
            } else if (typeof existing === 'function') {
                this._events[type] = prepend ? [listener, existing] : [existing, listener];
            } else if (prepend) {
                existing.unshift(listener);
            } else {
                existing.push(listener);
            }

            const max = this.getMaxListeners();
            if (max > 0) {
                const len = Array.isArray(this._events[type]) ? this._events[type].length : 1;
                if (len > max) {
                    const targetName = this.constructor ? this.constructor.name : 'EventEmitter';
                    const msg = `MaxListenersExceededWarning: Possible EventEmitter memory leak detected. ${len} ${type} listeners added to [${targetName}]. Use emitter.setMaxListeners() to increase limit`;
                    if (typeof console !== 'undefined' && typeof console.warn === 'function') {
                        console.warn(msg);
                    }
                }
            }

            return this;
        };

        EventEmitter.prototype.on = EventEmitter.prototype.addListener;

        EventEmitter.prototype.prependListener = function(type, listener) {
            return this.addListener(type, listener, true);
        };

        function onceWrapper(...args) {
            if (!this.fired) {
                this.target.removeListener(this.type, this.wrapFn);
                this.fired = true;
                return Reflect.apply(this.listener, this.target, args);
            }
        }

        EventEmitter.prototype.once = function(type, listener) {
            if (typeof listener !== 'function') {
                throw new TypeError('The "listener" argument must be of type Function');
            }
            const state = { fired: false, wrapFn: undefined, target: this, type, listener };
            const wrapped = onceWrapper.bind(state);
            wrapped.listener = listener;
            state.wrapFn = wrapped;
            return this.addListener(type, wrapped, false);
        };

        EventEmitter.prototype.prependOnceListener = function(type, listener) {
            if (typeof listener !== 'function') {
                throw new TypeError('The "listener" argument must be of type Function');
            }
            const state = { fired: false, wrapFn: undefined, target: this, type, listener };
            const wrapped = onceWrapper.bind(state);
            wrapped.listener = listener;
            state.wrapFn = wrapped;
            return this.addListener(type, wrapped, true);
        };

        EventEmitter.prototype.removeListener = function(type, listener) {
            if (typeof listener !== 'function') {
                throw new TypeError('The "listener" argument must be of type Function');
            }
            if (!this._events) return this;
            const list = this._events[type];
            if (!list) return this;
            if (list === listener || list.listener === listener) {
                if (--this._eventsCount === 0) {
                    this._events = Object.create(null);
                } else {
                    delete this._events[type];
                    if (this._events.removeListener) {
                        this.emit('removeListener', type, list.listener || list);
                    }
                }
            } else if (Array.isArray(list)) {
                let position = -1;
                for (let i = list.length - 1; i >= 0; i--) {
                    if (list[i] === listener || list[i].listener === listener) {
                        position = i;
                        break;
                    }
                }
                if (position < 0) return this;
                if (position === 0) list.shift();
                else list.splice(position, 1);
                if (list.length === 1) this._events[type] = list[0];
                if (this._events.removeListener) {
                    this.emit('removeListener', type, listener);
                }
            }
            return this;
        };

        EventEmitter.prototype.off = EventEmitter.prototype.removeListener;

        EventEmitter.prototype.removeAllListeners = function(type) {
            if (!this._events) return this;
            if (!type) {
                this._events = Object.create(null);
                this._eventsCount = 0;
                return this;
            }
            if (this._events[type]) {
                delete this._events[type];
                this._eventsCount = Reflect.ownKeys(this._events).length;
            }
            return this;
        };

        EventEmitter.prototype.listeners = function(type) {
            if (!this._events) return [];
            const ev = this._events[type];
            if (!ev) return [];
            if (typeof ev === 'function') return [ev.listener || ev];
            return ev.map(fn => fn.listener || fn);
        };

        EventEmitter.prototype.rawListeners = function(type) {
            if (!this._events) return [];
            const ev = this._events[type];
            if (!ev) return [];
            if (typeof ev === 'function') return [ev];
            return ev.slice();
        };

        EventEmitter.prototype.listenerCount = function(type) {
            if (!this._events) return 0;
            const ev = this._events[type];
            if (!ev) return 0;
            if (typeof ev === 'function') return 1;
            return ev.length;
        };

        EventEmitter.prototype.eventNames = function() {
            return this._eventsCount > 0 ? Reflect.ownKeys(this._events) : [];
        };

        EventEmitter.listenerCount = function(emitter, type) {
            return typeof emitter.listenerCount === 'function' ? emitter.listenerCount(type) : 0;
        };

        EventEmitter.once = function(emitter, type, options) {
            const signal = options && options.signal;
            return new Promise((resolve, reject) => {
                if (signal && signal.aborted) {
                    return reject(new Error('This operation was aborted'));
                }
                function onEvent(...args) {
                    cleanup();
                    resolve(args);
                }
                function onError(err) {
                    cleanup();
                    reject(err);
                }
                function onAbort() {
                    cleanup();
                    reject(new Error('This operation was aborted'));
                }
                function cleanup() {
                    if (emitter && typeof emitter.removeListener === 'function') {
                        emitter.removeListener(type, onEvent);
                        if (type !== 'error') {
                            emitter.removeListener('error', onError);
                        }
                    }
                    if (signal && typeof signal.removeEventListener === 'function') {
                        signal.removeEventListener('abort', onAbort);
                    }
                }
                if (emitter && typeof emitter.once === 'function') {
                    emitter.once(type, onEvent);
                    if (type !== 'error') {
                        emitter.once('error', onError);
                    }
                }
                if (signal && typeof signal.addEventListener === 'function') {
                    signal.addEventListener('abort', onAbort, { once: true });
                }
            });
        };

        EventEmitter.EventEmitter = EventEmitter;
        globalThis.EventEmitter = EventEmitter;
        globalThis.events = EventEmitter;
    })();
    "#;
    if let Some(code) = v8::String::new(scope, events_bootstrap) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }

    Ok(())
}

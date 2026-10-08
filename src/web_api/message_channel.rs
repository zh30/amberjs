// MessageChannel API implementation for Web standard
// v0.3.315: Enables port-based message communication between contexts
// Provides two connected MessagePorts for structured message passing

use anyhow::Result;
use rusty_v8 as v8;

fn is_port_closed(scope: &mut v8::PinScope, port: v8::Local<v8::Object>) -> bool {
    let closed_key = v8::String::new(scope, "_closed").unwrap();
    port.get(scope, closed_key.into())
        .is_some_and(|value| value.is_true())
        || port
            .get(
                scope,
                v8::String::new(scope, "_transferred").unwrap().into(),
            )
            .is_some_and(|value| value.is_true())
}

fn throw_dom(scope: &mut v8::PinScope, name: &str, message: &str) {
    let global = scope.get_current_context().global(scope);
    let text = v8::String::new(scope, message).unwrap();
    let name_value = v8::String::new(scope, name).unwrap();
    if let Some(constructor) = global
        .get(
            scope,
            v8::String::new(scope, "DOMException").unwrap().into(),
        )
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        if let Some(error) = constructor.new_instance(scope, &[text.into(), name_value.into()]) {
            scope.throw_exception(error.into());
            return;
        }
    }
    let error = v8::Exception::error(scope, text);
    if let Ok(object) = v8::Local::<v8::Object>::try_from(error) {
        object.set(
            scope,
            v8::String::new(scope, "name").unwrap().into(),
            name_value.into(),
        );
    }
    scope.throw_exception(error);
}

fn is_message_port(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> bool {
    v8::Local::<v8::Object>::try_from(value)
        .ok()
        .and_then(|object| object.get(scope, v8::String::new(scope, "_amberPort").unwrap().into()))
        .is_some_and(|flag| flag.is_true())
}

fn message_port_prototype<'a>(
    scope: &mut v8::PinScope<'a, '_>,
) -> Option<v8::Local<'a, v8::Object>> {
    let global = scope.get_current_context().global(scope);
    let constructor = global.get(scope, v8::String::new(scope, "MessagePort").unwrap().into())?;
    let constructor = v8::Local::<v8::Object>::try_from(constructor).ok()?;
    let prototype = constructor.get(scope, v8::String::new(scope, "prototype").unwrap().into())?;
    v8::Local::<v8::Object>::try_from(prototype).ok()
}

/// Setup MessageChannel API in V8 context
pub fn setup_message_channel_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    // Get global object
    let global = context.global(scope);

    let message_port_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, _args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            let message = v8::String::new(scope, "Illegal constructor").unwrap();
            scope.throw_exception(v8::Exception::type_error(scope, message));
        },
    );
    message_port_template.set_class_name(v8::String::new(scope, "MessagePort").unwrap());
    let message_port_constructor = message_port_template.get_function(scope).unwrap();
    global.set(
        scope,
        v8::String::new(scope, "MessagePort").unwrap().into(),
        message_port_constructor.into(),
    );

    // Create MessageChannel constructor function
    let message_channel_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            if !args.is_construct_call() {
                let message = v8::String::new(
                    scope,
                    "Failed to construct 'MessageChannel': Please use the 'new' operator.",
                )
                .unwrap();
                scope.throw_exception(v8::Exception::type_error(scope, message));
                return;
            }
            let channel_obj = args.this();

            // Create port1
            let port1 = v8::Object::new(scope);
            setup_message_port_properties(scope, port1);
            let port1_key = v8::String::new(scope, "port1").unwrap();
            channel_obj.set(scope, port1_key.into(), port1.into());

            // Create port2
            let port2 = v8::Object::new(scope);
            setup_message_port_properties(scope, port2);
            let port2_key = v8::String::new(scope, "port2").unwrap();
            channel_obj.set(scope, port2_key.into(), port2.into());

            // Store reference to other port on each port for message passing
            let other_port_key = v8::String::new(scope, "_otherPort").unwrap();
            port1.set(scope, other_port_key.into(), port2.into());
            port2.set(scope, other_port_key.into(), port1.into());

            // Initialize message queue on each port
            let queue1: v8::Local<v8::Array> = v8::Array::new(scope, 0);
            let queue_key = v8::String::new(scope, "_messageQueue").unwrap();
            port1.set(scope, queue_key.into(), queue1.into());

            let queue2: v8::Local<v8::Array> = v8::Array::new(scope, 0);
            port2.set(scope, queue_key.into(), queue2.into());

            // Initialize pending count
            let pending_key = v8::String::new(scope, "_pendingMessages").unwrap();
            let zero_int = v8::Integer::new(scope, 0);
            port1.set(scope, pending_key.into(), zero_int.into());
            port2.set(scope, pending_key.into(), zero_int.into());

            // Set closed flag
            let false_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            let closed_key = v8::String::new(scope, "_closed").unwrap();
            port1.set(scope, closed_key.into(), false_val.into());
            port2.set(scope, closed_key.into(), false_val.into());

            // Set started flag (message events are queued until start() is called)
            let started_false: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            let started_key = v8::String::new(scope, "_started").unwrap();
            port1.set(scope, started_key.into(), started_false.into());
            port2.set(scope, started_key.into(), started_false.into());

            // Set up closed property using undefined for now
            let undefined_val = v8::undefined(scope);
            let closed_prop_key = v8::String::new(scope, "closed").unwrap();
            port1.set(scope, closed_prop_key.into(), undefined_val.into());
            port2.set(scope, closed_prop_key.into(), undefined_val.into());
            brand_message_port(scope, port1);
            brand_message_port(scope, port2);

            retval.set(channel_obj.into());
        },
    );
    message_channel_fn.set_class_name(v8::String::new(scope, "MessageChannel").unwrap());

    // Set MessageChannel on global
    let message_channel_key = v8::String::new(scope, "MessageChannel").unwrap();
    let message_channel_val = message_channel_fn.get_function(scope).unwrap();
    global.set(
        scope,
        message_channel_key.into(),
        message_channel_val.into(),
    );

    Ok(())
}

/// Setup MessagePort properties (postMessage, onmessage, start, close, etc.)
fn setup_message_port_properties(scope: &mut v8::PinScope, port: v8::Local<v8::Object>) {
    // Create postMessage function
    let post_message_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            port_post_message(scope, args);
        },
    );

    let post_message_key = v8::String::new(scope, "postMessage").unwrap();
    let post_message_func = post_message_fn.get_function(scope).unwrap();
    port.set(scope, post_message_key.into(), post_message_func.into());

    // Create start() function
    let start_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            let this_obj = args.this();

            // Set started flag
            let started_key = v8::String::new(scope, "_started").unwrap();
            let true_val: v8::Local<v8::Value> = v8::Boolean::new(scope, true).into();
            this_obj.set(scope, started_key.into(), true_val.into());

            // Process queued messages
            let queue_key = v8::String::new(scope, "_messageQueue").unwrap();
            let pending_key = v8::String::new(scope, "_pendingMessages").unwrap();

            if let Some(queue_val) = this_obj.get(scope, queue_key.into()) {
                if let Ok(queue) = v8::Local::<v8::Array>::try_from(queue_val) {
                    let queue_len = queue.length();
                    for i in 0..queue_len {
                        if let Some(msg) = queue.get_index(scope, i) {
                            // Decrement pending as we process
                            let pending_val = this_obj.get(scope, pending_key.into()).unwrap();
                            let pending_int = pending_val.to_int32(scope).unwrap().value() as u32;
                            let new_pending = pending_int.saturating_sub(1);
                            let new_pending_val = v8::Integer::new(scope, new_pending as i32);
                            this_obj.set(scope, pending_key.into(), new_pending_val.into());

                            deliver_queued_message(scope, this_obj, msg);
                        }
                    }
                    // Clear queue
                    let empty_queue: v8::Local<v8::Array> = v8::Array::new(scope, 0);
                    this_obj.set(scope, queue_key.into(), empty_queue.into());
                }
            }
        },
    );

    let start_key = v8::String::new(scope, "start").unwrap();
    let start_func = start_fn.get_function(scope).unwrap();
    port.set(scope, start_key.into(), start_func.into());

    // Create close() function
    let close_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            let this_obj = args.this();

            disentangle_port(scope, this_obj);
        },
    );

    let close_key = v8::String::new(scope, "close").unwrap();
    let close_func = close_fn.get_function(scope).unwrap();
    port.set(scope, close_key.into(), close_func.into());

    port.set(
        scope,
        v8::String::new(scope, "_listeners").unwrap().into(),
        v8::Array::new(scope, 0).into(),
    );
    install_port_event_methods(scope, port);
    define_port_handlers(scope, port);
}

fn deliver_queued_message(
    scope: &mut v8::PinScope,
    port: v8::Local<v8::Object>,
    message: v8::Local<v8::Value>,
) {
    if let Ok(wrapper) = v8::Local::<v8::Object>::try_from(message) {
        if wrapper
            .get(
                scope,
                v8::String::new(scope, "_queuedMessage").unwrap().into(),
            )
            .is_some_and(|value| value.is_true())
        {
            let data = wrapper
                .get(scope, v8::String::new(scope, "data").unwrap().into())
                .unwrap_or_else(|| v8::undefined(scope).into());
            let ports = wrapper
                .get(scope, v8::String::new(scope, "ports").unwrap().into())
                .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
                .unwrap_or_else(|| v8::Array::new(scope, 0));
            dispatch_message_event(scope, port, data, ports);
            return;
        }
    }
    dispatch_message_event(scope, port, message, v8::Array::new(scope, 0));
}

/// Dispatch a message event to the port's onmessage handler and listeners.
fn dispatch_message_event(
    scope: &mut v8::PinScope,
    port: v8::Local<v8::Object>,
    data: v8::Local<v8::Value>,
    ports: v8::Local<v8::Array>,
) {
    let event_obj = v8::Object::new(scope);
    event_obj.set(
        scope,
        v8::String::new(scope, "type").unwrap().into(),
        v8::String::new(scope, "message").unwrap().into(),
    );
    event_obj.set(scope, v8::String::new(scope, "data").unwrap().into(), data);
    let empty = v8::String::new(scope, "").unwrap();
    event_obj.set(
        scope,
        v8::String::new(scope, "origin").unwrap().into(),
        empty.into(),
    );
    event_obj.set(
        scope,
        v8::String::new(scope, "lastEventId").unwrap().into(),
        empty.into(),
    );
    event_obj.set(
        scope,
        v8::String::new(scope, "ports").unwrap().into(),
        ports.into(),
    );

    if let Some(onmessage_val) =
        port.get(scope, v8::String::new(scope, "onmessage").unwrap().into())
    {
        if let Ok(onmessage_fn) = v8::Local::<v8::Function>::try_from(onmessage_val) {
            let _ = onmessage_fn.call(scope, port.into(), &[event_obj.into()]);
        }
    }

    if let Some(listeners_val) =
        port.get(scope, v8::String::new(scope, "_listeners").unwrap().into())
    {
        if let Ok(listeners) = v8::Local::<v8::Array>::try_from(listeners_val) {
            for index in 0..listeners.length() {
                if let Some(listener) = listeners.get_index(scope, index) {
                    if let Ok(listener_fn) = v8::Local::<v8::Function>::try_from(listener) {
                        let _ = listener_fn.call(scope, port.into(), &[event_obj.into()]);
                    }
                }
            }
        }
    }
}

fn brand_message_port(scope: &mut v8::PinScope, port: v8::Local<v8::Object>) {
    port.set(
        scope,
        v8::String::new(scope, "_amberPort").unwrap().into(),
        v8::Boolean::new(scope, true).into(),
    );
    port.set(
        scope,
        v8::String::new(scope, "_transferred").unwrap().into(),
        v8::Boolean::new(scope, false).into(),
    );
    if let Some(prototype) = message_port_prototype(scope) {
        let _ = port.set_prototype(scope, prototype.into());
    }
}

fn define_port_handlers(scope: &mut v8::PinScope, port: v8::Local<v8::Object>) {
    let source = r#"
        (function(port) {
            let handler = undefined;
            let errorHandler = undefined;
            Object.defineProperty(port, 'onmessage', {
                configurable: true,
                enumerable: true,
                get() { return handler; },
                set(value) {
                    handler = value;
                    port.start();
                }
            });
            Object.defineProperty(port, 'onmessageerror', {
                configurable: true,
                enumerable: true,
                get() { return errorHandler; },
                set(value) { errorHandler = value; }
            });
        })
    "#;
    let Some(code) = v8::String::new(scope, source) else {
        return;
    };
    let Some(script) = v8::Script::compile(scope, code, None) else {
        return;
    };
    let Some(factory) = script.run(scope) else {
        return;
    };
    let Ok(factory) = v8::Local::<v8::Function>::try_from(factory) else {
        return;
    };
    let _ = factory.call(scope, v8::undefined(scope).into(), &[port.into()]);
}

fn install_port_event_methods(scope: &mut v8::PinScope, port: v8::Local<v8::Object>) {
    let add = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            if args.length() < 2 || !args.get(1).is_function() {
                return;
            }
            let event_type = args
                .get(0)
                .to_string(scope)
                .map(|value| value.to_rust_string_lossy(scope))
                .unwrap_or_default();
            if event_type != "message" && event_type != "messageerror" {
                return;
            }
            let port = args.this();
            let key = if event_type == "message" {
                "_listeners"
            } else {
                "_errorListeners"
            };
            let listeners = port
                .get(scope, v8::String::new(scope, key).unwrap().into())
                .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
                .unwrap_or_else(|| {
                    let created = v8::Array::new(scope, 0);
                    port.set(
                        scope,
                        v8::String::new(scope, key).unwrap().into(),
                        created.into(),
                    );
                    created
                });
            let listener = args.get(1);
            for index in 0..listeners.length() {
                if listeners
                    .get_index(scope, index)
                    .is_some_and(|existing| existing.strict_equals(listener))
                {
                    return;
                }
            }
            listeners.set_index(scope, listeners.length(), listener);
        },
    )
    .unwrap();
    let remove = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            if args.length() < 2 || !args.get(1).is_function() {
                return;
            }
            let event_type = args
                .get(0)
                .to_string(scope)
                .map(|value| value.to_rust_string_lossy(scope))
                .unwrap_or_default();
            let key = if event_type == "message" {
                "_listeners"
            } else if event_type == "messageerror" {
                "_errorListeners"
            } else {
                return;
            };
            let port = args.this();
            let Some(listeners) = port
                .get(scope, v8::String::new(scope, key).unwrap().into())
                .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
            else {
                return;
            };
            let listener = args.get(1);
            let filtered = v8::Array::new(scope, 0);
            let mut next = 0u32;
            for index in 0..listeners.length() {
                if let Some(existing) = listeners.get_index(scope, index) {
                    if existing.strict_equals(listener) {
                        continue;
                    }
                    filtered.set_index(scope, next, existing);
                    next += 1;
                }
            }
            port.set(
                scope,
                v8::String::new(scope, key).unwrap().into(),
                filtered.into(),
            );
        },
    )
    .unwrap();
    port.set(
        scope,
        v8::String::new(scope, "addEventListener").unwrap().into(),
        add.into(),
    );
    port.set(
        scope,
        v8::String::new(scope, "removeEventListener")
            .unwrap()
            .into(),
        remove.into(),
    );
}

fn disentangle_port(scope: &mut v8::PinScope, port: v8::Local<v8::Object>) {
    let closed = v8::Boolean::new(scope, true);
    port.set(
        scope,
        v8::String::new(scope, "_closed").unwrap().into(),
        closed.into(),
    );
    if let Some(other) = port
        .get(scope, v8::String::new(scope, "_otherPort").unwrap().into())
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    {
        other.set(
            scope,
            v8::String::new(scope, "_closed").unwrap().into(),
            closed.into(),
        );
        let _ = other.delete(scope, v8::String::new(scope, "_otherPort").unwrap().into());
    }
    let _ = port.delete(scope, v8::String::new(scope, "_otherPort").unwrap().into());
    port.set(
        scope,
        v8::String::new(scope, "_messageQueue").unwrap().into(),
        v8::Array::new(scope, 0).into(),
    );
    port.set(
        scope,
        v8::String::new(scope, "_pendingMessages").unwrap().into(),
        v8::Integer::new(scope, 0).into(),
    );
}

fn transfer_buffer<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    value: v8::Local<'a, v8::Value>,
) -> Result<v8::Local<'a, v8::ArrayBuffer>, &'static str> {
    if value.is_array_buffer() {
        return v8::Local::<v8::ArrayBuffer>::try_from(value).map_err(|_| "ArrayBuffer");
    }
    let object = v8::Local::<v8::Object>::try_from(value).map_err(|_| "not a buffer")?;
    let buffer = object
        .get(scope, v8::String::new(scope, "buffer").unwrap().into())
        .ok_or("no buffer")?;
    if buffer.is_array_buffer() {
        return v8::Local::<v8::ArrayBuffer>::try_from(buffer).map_err(|_| "ArrayBuffer");
    }
    Err("not a buffer")
}

fn port_post_message(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments) {
    if args.length() == 0 {
        let message = v8::String::new(
            scope,
            "Failed to execute 'postMessage': 1 argument required.",
        )
        .unwrap();
        scope.throw_exception(v8::Exception::type_error(scope, message));
        return;
    }
    let message = args.get(0);
    let this_obj = args.this();
    if is_port_closed(scope, this_obj) {
        throw_dom(
            scope,
            "InvalidStateError",
            "Failed to execute 'postMessage' on 'MessagePort': Port is closed or transferred.",
        );
        return;
    }
    let Some(other_port) = this_obj
        .get(scope, v8::String::new(scope, "_otherPort").unwrap().into())
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    else {
        throw_dom(
            scope,
            "InvalidStateError",
            "Failed to execute 'postMessage' on 'MessagePort': Port is not entangled.",
        );
        return;
    };
    if is_port_closed(scope, other_port) {
        throw_dom(
            scope,
            "InvalidStateError",
            "Failed to execute 'postMessage' on 'MessagePort': Port is not entangled.",
        );
        return;
    }

    let transfer = if args.length() > 1 {
        args.get(1)
    } else {
        v8::undefined(scope).into()
    };
    let mut moved_ports: Vec<v8::Global<v8::Object>> = Vec::new();
    let mut buffers: Vec<v8::Global<v8::ArrayBuffer>> = Vec::new();
    if transfer.is_array() {
        if let Ok(list) = v8::Local::<v8::Array>::try_from(transfer) {
            for index in 0..list.length() {
                let Some(item) = list.get_index(scope, index) else {
                    continue;
                };
                if is_message_port(scope, item) {
                    let Ok(port) = v8::Local::<v8::Object>::try_from(item) else {
                        continue;
                    };
                    if port.strict_equals(this_obj.into()) || port.strict_equals(other_port.into())
                    {
                        throw_dom(
                            scope,
                            "DataCloneError",
                            "MessagePort cannot be transferred to itself.",
                        );
                        return;
                    }
                    if is_port_closed(scope, port) {
                        throw_dom(
                            scope,
                            "DataCloneError",
                            "MessagePort is already closed or transferred.",
                        );
                        return;
                    }
                    moved_ports.push(v8::Global::new(scope, port));
                } else if let Ok(buffer) = transfer_buffer(scope, item) {
                    if buffer.was_detached() {
                        throw_dom(scope, "DataCloneError", "ArrayBuffer is already detached.");
                        return;
                    }
                    buffers.push(v8::Global::new(scope, buffer));
                }
            }
        }
    }

    let message_port_index = moved_ports.iter().position(|port| {
        let port = v8::Local::new(scope, port);
        message.strict_equals(port.into())
    });
    let cloned_message = if message_port_index.is_some() {
        None
    } else {
        let global = scope.get_current_context().global(scope);
        let Some(structured_clone) = global
            .get(
                scope,
                v8::String::new(scope, "structuredClone").unwrap().into(),
            )
            .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
        else {
            throw_dom(scope, "DataCloneError", "structuredClone is unavailable");
            return;
        };
        let Some(cloned) = structured_clone.call(scope, v8::undefined(scope).into(), &[message])
        else {
            return;
        };
        Some(cloned)
    };

    let mut transferred = Vec::new();
    for port_global in moved_ports {
        let port = v8::Local::new(scope, port_global);
        if let Some(moved) = take_message_port(scope, port) {
            transferred.push(moved);
        }
    }
    let data = if let Some(index) = message_port_index {
        transferred
            .get(index)
            .copied()
            .map(|port| port.into())
            .unwrap_or_else(|| v8::undefined(scope).into())
    } else {
        cloned_message.unwrap_or_else(|| v8::undefined(scope).into())
    };

    for buffer_global in buffers {
        let buffer = v8::Local::new(scope, buffer_global);
        buffer.detach(None);
    }

    let ports_array = v8::Array::new(scope, transferred.len() as i32);
    for (index, port) in transferred.iter().enumerate() {
        ports_array.set_index(scope, index as u32, (*port).into());
    }
    let started = other_port
        .get(scope, v8::String::new(scope, "_started").unwrap().into())
        .is_some_and(|value| value.is_true());
    if started {
        dispatch_message_event(scope, other_port, data, ports_array);
        return;
    }
    let wrapper = v8::Object::new(scope);
    wrapper.set(
        scope,
        v8::String::new(scope, "_queuedMessage").unwrap().into(),
        v8::Boolean::new(scope, true).into(),
    );
    wrapper.set(scope, v8::String::new(scope, "data").unwrap().into(), data);
    wrapper.set(
        scope,
        v8::String::new(scope, "ports").unwrap().into(),
        ports_array.into(),
    );
    if let Some(queue) = other_port
        .get(
            scope,
            v8::String::new(scope, "_messageQueue").unwrap().into(),
        )
        .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
    {
        queue.set_index(scope, queue.length(), wrapper.into());
    }
    let pending = other_port
        .get(
            scope,
            v8::String::new(scope, "_pendingMessages").unwrap().into(),
        )
        .and_then(|value| value.to_int32(scope))
        .map(|value| value.value())
        .unwrap_or(0);
    other_port.set(
        scope,
        v8::String::new(scope, "_pendingMessages").unwrap().into(),
        v8::Integer::new(scope, pending + 1).into(),
    );
}

fn take_message_port<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    port: v8::Local<v8::Object>,
) -> Option<v8::Local<'a, v8::Object>> {
    let moved = v8::Object::new(scope);
    setup_message_port_properties(scope, moved);
    brand_message_port(scope, moved);
    let copy_names = [
        "_otherPort",
        "_messageQueue",
        "_pendingMessages",
        "_started",
        "_listeners",
    ];
    for name in copy_names {
        if let Some(value) = port.get(scope, v8::String::new(scope, name).unwrap().into()) {
            moved.set(scope, v8::String::new(scope, name).unwrap().into(), value);
        }
    }
    if let Some(peer) = moved
        .get(scope, v8::String::new(scope, "_otherPort").unwrap().into())
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    {
        peer.set(
            scope,
            v8::String::new(scope, "_otherPort").unwrap().into(),
            moved.into(),
        );
    }
    port.set(
        scope,
        v8::String::new(scope, "_transferred").unwrap().into(),
        v8::Boolean::new(scope, true).into(),
    );
    port.set(
        scope,
        v8::String::new(scope, "_closed").unwrap().into(),
        v8::Boolean::new(scope, true).into(),
    );
    let _ = port.delete(scope, v8::String::new(scope, "_otherPort").unwrap().into());
    Some(moved)
}

// BroadcastChannel API implementation for Web standard
// v0.3.312: Enables real-time communication between browsing contexts
// Provides cross-tab, cross-window, and cross-frame communication via named channels

use anyhow::Result;
use rusty_v8 as v8;

const REGISTRY_PRIVATE_KEY: &str = "Amber.BroadcastChannel#registry";

fn get_broadcast_registry<'s>(scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Array> {
    let global = scope.get_current_context().global(scope);
    let key_name = v8::String::new(scope, REGISTRY_PRIVATE_KEY).unwrap();
    let key = v8::Private::for_api(scope, Some(key_name));

    if let Some(registry_val) = global.get_private(scope, key) {
        if let Ok(registry) = v8::Local::<v8::Array>::try_from(registry_val) {
            return registry;
        }
    }

    let registry = v8::Array::new(scope, 0);
    let key_name = v8::String::new(scope, REGISTRY_PRIVATE_KEY).unwrap();
    let key = v8::Private::for_api(scope, Some(key_name));
    global.set_private(scope, key, registry.into());
    registry
}

fn get_string_property(
    scope: &mut v8::PinScope,
    object: v8::Local<v8::Object>,
    name: &str,
) -> Option<String> {
    let key = v8::String::new(scope, name).unwrap();
    object
        .get(scope, key.into())
        .map(|value| value.to_rust_string_lossy(scope))
}

fn is_channel_closed(scope: &mut v8::PinScope, channel: v8::Local<v8::Object>) -> bool {
    let closed_key = v8::String::new(scope, "_closed").unwrap();
    channel
        .get(scope, closed_key.into())
        .is_some_and(|value| value.is_true())
}

fn dispatch_message_event(
    scope: &mut v8::PinScope,
    target: v8::Local<v8::Object>,
    message: v8::Local<v8::Value>,
) {
    let event_obj = v8::Object::new(scope);

    let type_key = v8::String::new(scope, "type").unwrap();
    let message_type = v8::String::new(scope, "message").unwrap();
    event_obj.set(scope, type_key.into(), message_type.into());

    let data_key = v8::String::new(scope, "data").unwrap();
    event_obj.set(scope, data_key.into(), message);

    let origin_key = v8::String::new(scope, "origin").unwrap();
    let empty_origin = v8::String::new(scope, "").unwrap();
    event_obj.set(scope, origin_key.into(), empty_origin.into());

    let last_event_id_key = v8::String::new(scope, "lastEventId").unwrap();
    event_obj.set(scope, last_event_id_key.into(), empty_origin.into());

    let onmessage_key = v8::String::new(scope, "onmessage").unwrap();
    if let Some(onmessage_val) = target.get(scope, onmessage_key.into()) {
        if let Ok(onmessage_fn) = v8::Local::<v8::Function>::try_from(onmessage_val) {
            let _ = onmessage_fn.call(scope, target.into(), &[event_obj.into()]);
        }
    }

    let listeners_key = v8::String::new(scope, "_listeners").unwrap();
    let Some(listeners_val) = target.get(scope, listeners_key.into()) else {
        return;
    };
    let Ok(listeners_obj) = v8::Local::<v8::Object>::try_from(listeners_val) else {
        return;
    };
    let message_key = v8::String::new(scope, "message").unwrap();
    let Some(message_listeners_val) = listeners_obj.get(scope, message_key.into()) else {
        return;
    };
    let Ok(listeners_array) = v8::Local::<v8::Array>::try_from(message_listeners_val) else {
        return;
    };

    for i in 0..listeners_array.length() {
        if let Some(listener) = listeners_array.get_index(scope, i) {
            if let Ok(listener_fn) = v8::Local::<v8::Function>::try_from(listener) {
                let _ = listener_fn.call(scope, target.into(), &[event_obj.into()]);
            }
        }
    }
}

/// Setup BroadcastChannel API in V8 context
pub fn setup_broadcast_channel_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    // Create BroadcastChannel constructor template
    let broadcast_channel_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            // Get channel name from first argument
            let name = if args.length() > 0 {
                args.get(0).to_rust_string_lossy(scope)
            } else {
                "".to_string()
            };

            // Create the BroadcastChannel object
            let channel_obj: v8::Local<v8::Object> = v8::Object::new(scope);

            // Store name property
            let name_key = v8::String::new(scope, "name").unwrap();
            let name_value = v8::String::new(scope, &name).unwrap();
            channel_obj.set(scope, name_key.into(), name_value.into());

            // Create listener buckets stored on the object
            let listeners_key = v8::String::new(scope, "_listeners").unwrap();
            let listeners_obj: v8::Local<v8::Object> = v8::Object::new(scope);
            let message_key = v8::String::new(scope, "message").unwrap();
            let message_array: v8::Local<v8::Array> = v8::Array::new(scope, 0);
            listeners_obj.set(scope, message_key.into(), message_array.into());
            let messageerror_key = v8::String::new(scope, "messageerror").unwrap();
            let messageerror_array: v8::Local<v8::Array> = v8::Array::new(scope, 0);
            listeners_obj.set(scope, messageerror_key.into(), messageerror_array.into());
            channel_obj.set(scope, listeners_key.into(), listeners_obj.into());

            // Create closed flag
            let closed_key = v8::String::new(scope, "_closed").unwrap();
            let false_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            channel_obj.set(scope, closed_key.into(), false_val.into());

            // Create postMessage function
            let post_message_fn = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _rv: v8::ReturnValue| {
                    if args.length() == 0 {
                        return;
                    }

                    let message = args.get(0);
                    let this_obj = args.this();

                    if is_channel_closed(scope, this_obj) {
                        return;
                    }

                    let Some(sender_name) = get_string_property(scope, this_obj, "name") else {
                        return;
                    };

                    let global = scope.get_current_context().global(scope);
                    let structured_clone_key = v8::String::new(scope, "structuredClone").unwrap();
                    let Some(structured_clone_value) =
                        global.get(scope, structured_clone_key.into())
                    else {
                        let error_message =
                            v8::String::new(scope, "structuredClone is unavailable").unwrap();
                        let error = v8::Exception::error(scope, error_message);
                        scope.throw_exception(error);
                        return;
                    };
                    let Ok(structured_clone_fn) =
                        v8::Local::<v8::Function>::try_from(structured_clone_value)
                    else {
                        let error_message =
                            v8::String::new(scope, "structuredClone is unavailable").unwrap();
                        let error = v8::Exception::error(scope, error_message);
                        scope.throw_exception(error);
                        return;
                    };

                    let undefined: v8::Local<v8::Value> = v8::undefined(scope).into();
                    let Some(first_cloned_message) =
                        structured_clone_fn.call(scope, undefined, &[message])
                    else {
                        return;
                    };

                    let registry = get_broadcast_registry(scope);
                    let sender_value: v8::Local<v8::Value> = this_obj.into();
                    let mut targets: Vec<v8::Local<v8::Object>> = Vec::new();

                    for i in 0..registry.length() {
                        let Some(target_val) = registry.get_index(scope, i) else {
                            continue;
                        };
                        if target_val.strict_equals(sender_value) {
                            continue;
                        }
                        let Ok(target) = v8::Local::<v8::Object>::try_from(target_val) else {
                            continue;
                        };
                        if is_channel_closed(scope, target) {
                            continue;
                        }
                        if get_string_property(scope, target, "name").as_deref()
                            == Some(sender_name.as_str())
                        {
                            targets.push(target);
                        }
                    }

                    if targets.is_empty() {
                        return;
                    }

                    let mut cloned_messages: Vec<v8::Local<v8::Value>> =
                        Vec::with_capacity(targets.len());
                    cloned_messages.push(first_cloned_message);
                    while cloned_messages.len() < targets.len() {
                        let undefined: v8::Local<v8::Value> = v8::undefined(scope).into();
                        let Some(cloned_message) =
                            structured_clone_fn.call(scope, undefined, &[message])
                        else {
                            return;
                        };
                        cloned_messages.push(cloned_message);
                    }

                    for (target, cloned_message) in targets.into_iter().zip(cloned_messages) {
                        dispatch_message_event(scope, target, cloned_message);
                    }
                },
            );
            let post_message_instance = post_message_fn.get_function(scope).unwrap();
            let post_message_key = v8::String::new(scope, "postMessage").unwrap();
            channel_obj.set(scope, post_message_key.into(), post_message_instance.into());

            // Create close function
            let close_fn = v8::FunctionTemplate::new(
                scope,
                |_scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _rv: v8::ReturnValue| {
                    let this_obj = args.this();
                    let closed_key = v8::String::new(_scope, "_closed").unwrap();
                    let true_val: v8::Local<v8::Value> = v8::Boolean::new(_scope, true).into();
                    this_obj.set(_scope, closed_key.into(), true_val.into());
                },
            );
            let close_instance = close_fn.get_function(scope).unwrap();
            let close_key = v8::String::new(scope, "close").unwrap();
            channel_obj.set(scope, close_key.into(), close_instance.into());

            // Create addEventListener function
            let add_event_listener_fn = v8::FunctionTemplate::new(
                scope,
                |_scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _rv: v8::ReturnValue| {
                    if args.length() < 2 {
                        return;
                    }

                    let event_type = args.get(0).to_rust_string_lossy(_scope);
                    let listener = args.get(1);
                    let this_obj = args.this();

                    if (event_type == "message" || event_type == "messageerror")
                        && listener.is_function()
                    {
                        // Add to the event-specific listener array on the object
                        let listeners_key = v8::String::new(_scope, "_listeners").unwrap();
                        if let Some(listeners_val) = this_obj.get(_scope, listeners_key.into()) {
                            if let Ok(listeners_obj) =
                                v8::Local::<v8::Object>::try_from(listeners_val)
                            {
                                let event_key = v8::String::new(_scope, &event_type).unwrap();
                                if let Some(listeners_array_val) =
                                    listeners_obj.get(_scope, event_key.into())
                                {
                                    if let Ok(listeners_array) =
                                        v8::Local::<v8::Array>::try_from(listeners_array_val)
                                    {
                                        let length = listeners_array.length();
                                        listeners_array.set_index(_scope, length, listener);
                                    }
                                }
                            }
                        }
                    }
                },
            );
            let add_event_listener_instance = add_event_listener_fn.get_function(scope).unwrap();
            let add_event_listener_key = v8::String::new(scope, "addEventListener").unwrap();
            channel_obj.set(
                scope,
                add_event_listener_key.into(),
                add_event_listener_instance.into(),
            );

            // Create removeEventListener function
            let remove_event_listener_fn = v8::FunctionTemplate::new(
                scope,
                |_scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _rv: v8::ReturnValue| {
                    if args.length() < 2 {
                        return;
                    }

                    let event_type = args.get(0).to_rust_string_lossy(_scope);
                    let listener = args.get(1);
                    if (event_type != "message" && event_type != "messageerror")
                        || !listener.is_function()
                    {
                        return;
                    }

                    let this_obj = args.this();
                    let listeners_key = v8::String::new(_scope, "_listeners").unwrap();
                    let Some(listeners_val) = this_obj.get(_scope, listeners_key.into()) else {
                        return;
                    };
                    let Ok(listeners_obj) = v8::Local::<v8::Object>::try_from(listeners_val) else {
                        return;
                    };
                    let event_key = v8::String::new(_scope, &event_type).unwrap();
                    let Some(listeners_array_val) = listeners_obj.get(_scope, event_key.into())
                    else {
                        return;
                    };
                    let Ok(listeners_array) = v8::Local::<v8::Array>::try_from(listeners_array_val)
                    else {
                        return;
                    };

                    let filtered = v8::Array::new(_scope, 0);
                    let mut filtered_len = 0;
                    for i in 0..listeners_array.length() {
                        if let Some(existing_listener) = listeners_array.get_index(_scope, i) {
                            if !existing_listener.strict_equals(listener) {
                                filtered.set_index(_scope, filtered_len, existing_listener);
                                filtered_len += 1;
                            }
                        }
                    }

                    let event_key = v8::String::new(_scope, &event_type).unwrap();
                    listeners_obj.set(_scope, event_key.into(), filtered.into());
                },
            );
            let remove_event_listener_instance =
                remove_event_listener_fn.get_function(scope).unwrap();
            let remove_event_listener_key = v8::String::new(scope, "removeEventListener").unwrap();
            channel_obj.set(
                scope,
                remove_event_listener_key.into(),
                remove_event_listener_instance.into(),
            );

            // Create dispatchEvent function
            let dispatch_event_fn = v8::FunctionTemplate::new(
                scope,
                |_scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _rv: v8::ReturnValue| {
                    let this_obj = args.this();
                    if args.length() > 0 {
                        let event = args.get(0);
                        if let Ok(event_obj) = v8::Local::<v8::Object>::try_from(event) {
                            let type_key = v8::String::new(_scope, "type").unwrap();
                            if let Some(type_val) = event_obj.get(_scope, type_key.into()) {
                                let event_type = type_val.to_rust_string_lossy(_scope);

                                if event_type == "message" || event_type == "messageerror" {
                                    // Get event-specific listeners and call them
                                    let listeners_key =
                                        v8::String::new(_scope, "_listeners").unwrap();
                                    if let Some(listeners_val) =
                                        this_obj.get(_scope, listeners_key.into())
                                    {
                                        if let Ok(listeners_obj) =
                                            v8::Local::<v8::Object>::try_from(listeners_val)
                                        {
                                            let event_key =
                                                v8::String::new(_scope, &event_type).unwrap();
                                            if let Some(listeners_array_val) =
                                                listeners_obj.get(_scope, event_key.into())
                                            {
                                                if let Ok(listeners_array) =
                                                    v8::Local::<v8::Array>::try_from(
                                                        listeners_array_val,
                                                    )
                                                {
                                                    for i in 0..listeners_array.length() {
                                                        if let Some(listener) =
                                                            listeners_array.get_index(_scope, i)
                                                        {
                                                            if let Ok(listener_fn) =
                                                                v8::Local::<v8::Function>::try_from(
                                                                    listener,
                                                                )
                                                            {
                                                                let _ = listener_fn.call(
                                                                    _scope,
                                                                    this_obj.into(),
                                                                    &[event_obj.into()],
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
            );
            let dispatch_event_instance = dispatch_event_fn.get_function(scope).unwrap();
            let dispatch_event_key = v8::String::new(scope, "dispatchEvent").unwrap();
            channel_obj.set(
                scope,
                dispatch_event_key.into(),
                dispatch_event_instance.into(),
            );

            // Set onmessage property (for simple usage)
            let onmessage_key = v8::String::new(scope, "onmessage").unwrap();
            let undefined_val = v8::undefined(scope);
            channel_obj.set(scope, onmessage_key.into(), undefined_val.into());

            // Set onmessageerror property
            let onmessageerror_key = v8::String::new(scope, "onmessageerror").unwrap();
            channel_obj.set(scope, onmessageerror_key.into(), undefined_val.into());

            let registry = get_broadcast_registry(scope);
            registry.set_index(scope, registry.length(), channel_obj.into());

            retval.set(channel_obj.into());
        },
    );

    let broadcast_channel_constructor: v8::Local<v8::Function> =
        broadcast_channel_template.get_function(scope).unwrap();

    // Set BroadcastChannel to global scope
    let global: v8::Local<v8::Object> = context.global(scope);
    let broadcast_channel_key: v8::Local<v8::String> =
        v8::String::new(scope, "BroadcastChannel").unwrap();
    let broadcast_channel_val: v8::Local<v8::Value> = broadcast_channel_constructor.into();
    global.set(
        scope,
        broadcast_channel_key.into(),
        broadcast_channel_val.into(),
    );

    Ok(())
}

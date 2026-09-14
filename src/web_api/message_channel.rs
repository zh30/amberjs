// MessageChannel API implementation for Web standard
// v0.3.315: Enables port-based message communication between contexts
// Provides two connected MessagePorts for structured message passing

use anyhow::Result;
use rusty_v8 as v8;

fn is_port_closed(scope: &mut v8::PinScope, port: v8::Local<v8::Object>) -> bool {
    let closed_key = v8::String::new(scope, "_closed").unwrap();
    port.get(scope, closed_key.into())
        .is_some_and(|value| value.is_true())
}

/// Setup MessageChannel API in V8 context
pub fn setup_message_channel_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    // Get global object
    let global = context.global(scope);

    // Create MessageChannel constructor function
    let message_channel_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            // Create the MessageChannel object
            let channel_obj: v8::Local<v8::Object> = v8::Object::new(scope);

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

            retval.set(channel_obj.into());
        },
    );

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
            if args.length() == 0 {
                return;
            }

            let message = args.get(0);
            let this_obj = args.this();

            if is_port_closed(scope, this_obj) {
                return;
            }

            // Get the other port
            let other_port_key = v8::String::new(scope, "_otherPort").unwrap();
            if let Some(other_port_val) = this_obj.get(scope, other_port_key.into()) {
                if let Ok(other_port) = v8::Local::<v8::Object>::try_from(other_port_val) {
                    if is_port_closed(scope, other_port) {
                        return;
                    }

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
                    let Some(cloned_message) =
                        structured_clone_fn.call(scope, undefined, &[message])
                    else {
                        return;
                    };

                    let started_key = v8::String::new(scope, "_started").unwrap();
                    let target_started = other_port
                        .get(scope, started_key.into())
                        .is_some_and(|value| value.is_true());
                    if target_started {
                        dispatch_message_event(scope, other_port, cloned_message);
                        return;
                    }

                    // Queue the message on the other port until start() is called.
                    let queue_key = v8::String::new(scope, "_messageQueue").unwrap();
                    if let Some(queue_val) = other_port.get(scope, queue_key.into()) {
                        if let Ok(queue) = v8::Local::<v8::Array>::try_from(queue_val) {
                            let length = queue.length();
                            queue.set_index(scope, length, cloned_message);
                        }
                    }

                    // Increment pending count
                    let pending_key = v8::String::new(scope, "_pendingMessages").unwrap();
                    let pending_val = other_port.get(scope, pending_key.into()).unwrap();
                    let pending_int = pending_val.to_int32(scope).unwrap().value() as u32;
                    let new_pending = pending_int + 1;
                    let pending_int_val = v8::Integer::new(scope, new_pending as i32);
                    other_port.set(scope, pending_key.into(), pending_int_val.into());
                }
            }
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

                            dispatch_message_event(scope, this_obj, msg);
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

            // Set closed flag
            let closed_key = v8::String::new(scope, "_closed").unwrap();
            let true_val: v8::Local<v8::Value> = v8::Boolean::new(scope, true).into();
            this_obj.set(scope, closed_key.into(), true_val.into());

            // Clear the other port reference
            let other_port_key = v8::String::new(scope, "_otherPort").unwrap();
            let _ = this_obj.delete(scope, other_port_key.into());
        },
    );

    let close_key = v8::String::new(scope, "close").unwrap();
    let close_func = close_fn.get_function(scope).unwrap();
    port.set(scope, close_key.into(), close_func.into());

    // Set up event handler properties (onmessage, onmessageerror)
    let undefined = v8::undefined(scope);
    let onmessage_key = v8::String::new(scope, "onmessage").unwrap();
    port.set(scope, onmessage_key.into(), undefined.into());

    let onmessageerror_key = v8::String::new(scope, "onmessageerror").unwrap();
    port.set(scope, onmessageerror_key.into(), undefined.into());
}

/// Dispatch a message event to the port's onmessage handler
fn dispatch_message_event(
    scope: &mut v8::PinScope,
    port: v8::Local<v8::Object>,
    data: v8::Local<v8::Value>,
) {
    // Create MessageEvent object
    let event_obj = v8::Object::new(scope);

    // Set event properties
    let type_key = v8::String::new(scope, "type").unwrap();
    let message_type = v8::String::new(scope, "message").unwrap();
    event_obj.set(scope, type_key.into(), message_type.into());

    let data_key = v8::String::new(scope, "data").unwrap();
    event_obj.set(scope, data_key.into(), data);

    let origin_key = v8::String::new(scope, "origin").unwrap();
    let empty_origin = v8::String::new(scope, "").unwrap();
    event_obj.set(scope, origin_key.into(), empty_origin.into());

    let last_event_id_key = v8::String::new(scope, "lastEventId").unwrap();
    event_obj.set(scope, last_event_id_key.into(), empty_origin.into());

    let ports_key = v8::String::new(scope, "ports").unwrap();
    let empty_ports: v8::Local<v8::Array> = v8::Array::new(scope, 0);
    event_obj.set(scope, ports_key.into(), empty_ports.into());

    // First, check onmessage property
    let onmessage_key = v8::String::new(scope, "onmessage").unwrap();
    if let Some(onmessage_val) = port.get(scope, onmessage_key.into()) {
        if !onmessage_val.is_undefined() && onmessage_val.is_function() {
            if let Ok(onmessage_fn) = v8::Local::<v8::Function>::try_from(onmessage_val) {
                let _ = onmessage_fn.call(scope, port.into(), &[event_obj.into()]);
            }
        }
    }
}

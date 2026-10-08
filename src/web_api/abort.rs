/// AbortController / AbortSignal.
///
/// Fetch still watches `__amberAbortId` via `fetch::abort_fetch_signal`. Static
/// `timeout`, `any`, and `abort`, plus `reason` and the `abort` event, are the
/// surface beyond the fetch contract.
use anyhow::Result;
use rusty_v8 as v8;

pub fn setup_abort_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let signal_template = v8::FunctionTemplate::new(scope, abort_signal_constructor);
    signal_template.set_class_name(v8::String::new(scope, "AbortSignal").unwrap());
    let signal_proto = signal_template.prototype_template(scope);
    signal_proto.set(
        v8::String::new(scope, "addEventListener").unwrap().into(),
        v8::FunctionTemplate::new(scope, signal_add_event_listener).into(),
    );
    signal_proto.set(
        v8::String::new(scope, "removeEventListener")
            .unwrap()
            .into(),
        v8::FunctionTemplate::new(scope, signal_remove_event_listener).into(),
    );
    signal_proto.set(
        v8::String::new(scope, "throwIfAborted").unwrap().into(),
        v8::FunctionTemplate::new(scope, signal_throw_if_aborted).into(),
    );

    let signal_constructor = signal_template.get_function(scope).unwrap();
    let controller_template = v8::FunctionTemplate::new(scope, abort_controller_constructor);
    controller_template.set_class_name(v8::String::new(scope, "AbortController").unwrap());
    let controller_constructor = controller_template.get_function(scope).unwrap();

    let global = context.global(scope);
    global.set(
        scope,
        v8::String::new(scope, "AbortSignal").unwrap().into(),
        signal_constructor.into(),
    );
    global.set(
        scope,
        v8::String::new(scope, "AbortController").unwrap().into(),
        controller_constructor.into(),
    );

    install_static_methods(scope);

    Ok(())
}

fn abort_signal_constructor(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let message = v8::String::new(scope, "Illegal constructor").unwrap();
    let error = v8::Exception::type_error(scope, message);
    scope.throw_exception(error);
}

fn abort_controller_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if !args.is_construct_call() {
        let message =
            v8::String::new(scope, "AbortController constructor must be called with new").unwrap();
        scope.throw_exception(v8::Exception::type_error(scope, message));
        return;
    }

    let controller = args.this();
    let signal = create_signal_object(scope);
    controller.set(
        scope,
        v8::String::new(scope, "signal").unwrap().into(),
        signal.into(),
    );

    let abort_fn = v8::Function::new(scope, abort_method).unwrap();
    controller.set(
        scope,
        v8::String::new(scope, "abort").unwrap().into(),
        abort_fn.into(),
    );
    retval.set(controller.into());
}

fn create_signal_object<'a>(scope: &mut v8::PinScope<'a, '_>) -> v8::Local<'a, v8::Object> {
    let signal = v8::Object::new(scope);
    if let Some(proto) = abort_signal_prototype(scope) {
        let _ = signal.set_prototype(scope, proto.into());
    }

    signal.set(
        scope,
        v8::String::new(scope, "aborted").unwrap().into(),
        v8::Boolean::new(scope, false).into(),
    );
    signal.set(
        scope,
        v8::String::new(scope, "reason").unwrap().into(),
        v8::undefined(scope).into(),
    );
    signal.set(
        scope,
        v8::String::new(scope, "onabort").unwrap().into(),
        v8::null(scope).into(),
    );
    signal.set(
        scope,
        v8::String::new(scope, "_abortListeners").unwrap().into(),
        v8::Array::new(scope, 0).into(),
    );

    let (abort_id, _) = super::fetch::register_abort_flag();
    signal.set(
        scope,
        v8::String::new(scope, "__amberAbortId").unwrap().into(),
        v8::Number::new(scope, abort_id as f64).into(),
    );
    signal
}

fn abort_signal_prototype<'a>(
    scope: &mut v8::PinScope<'a, '_>,
) -> Option<v8::Local<'a, v8::Object>> {
    let global = scope.get_current_context().global(scope);
    let constructor = global.get(scope, v8::String::new(scope, "AbortSignal").unwrap().into())?;
    let constructor = v8::Local::<v8::Object>::try_from(constructor).ok()?;
    let prototype = constructor.get(scope, v8::String::new(scope, "prototype").unwrap().into())?;
    v8::Local::<v8::Object>::try_from(prototype).ok()
}

fn abort_method(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let controller = args.this();
    let Some(signal_val) = controller.get(scope, v8::String::new(scope, "signal").unwrap().into())
    else {
        return;
    };
    let Ok(signal) = v8::Local::<v8::Object>::try_from(signal_val) else {
        return;
    };
    let reason = if args.length() > 0 && !args.get(0).is_undefined() {
        Some(args.get(0))
    } else {
        None
    };
    abort_signal(scope, signal, reason);
}

fn signal_is_aborted(scope: &mut v8::PinScope, signal: v8::Local<v8::Object>) -> bool {
    signal
        .get(scope, v8::String::new(scope, "aborted").unwrap().into())
        .is_some_and(|value| value.is_true())
}

fn abort_signal(
    scope: &mut v8::PinScope,
    signal: v8::Local<v8::Object>,
    reason: Option<v8::Local<v8::Value>>,
) {
    if signal_is_aborted(scope, signal) {
        return;
    }

    signal.set(
        scope,
        v8::String::new(scope, "aborted").unwrap().into(),
        v8::Boolean::new(scope, true).into(),
    );

    if let Some(id_val) = signal.get(
        scope,
        v8::String::new(scope, "__amberAbortId").unwrap().into(),
    ) {
        if let Some(id_num) = id_val.to_number(scope) {
            let id = id_num.value() as u64;
            if id != 0 {
                super::fetch::abort_fetch_signal(id);
            }
        }
    }

    let reason_value =
        reason.unwrap_or_else(|| dom_exception(scope, "The operation was aborted", "AbortError"));
    signal.set(
        scope,
        v8::String::new(scope, "reason").unwrap().into(),
        reason_value,
    );

    let event = abort_event(scope, signal);
    let listeners = listener_snapshot(scope, signal);
    for listener in listeners {
        let listener = v8::Local::new(scope, listener);
        let listener = listener_callback(scope, listener);
        call_listener(scope, signal, listener, event);
    }
    if let Some(onabort) = signal.get(scope, v8::String::new(scope, "onabort").unwrap().into()) {
        call_listener(scope, signal, onabort, event);
    }
    signal.set(
        scope,
        v8::String::new(scope, "_abortListeners").unwrap().into(),
        v8::Array::new(scope, 0).into(),
    );
}

fn listener_snapshot(
    scope: &mut v8::PinScope,
    signal: v8::Local<v8::Object>,
) -> Vec<v8::Global<v8::Value>> {
    let Some(listeners_val) = signal.get(
        scope,
        v8::String::new(scope, "_abortListeners").unwrap().into(),
    ) else {
        return Vec::new();
    };
    let Ok(listeners) = v8::Local::<v8::Array>::try_from(listeners_val) else {
        return Vec::new();
    };
    let mut snapshot = Vec::new();
    for index in 0..listeners.length() {
        if let Some(listener) = listeners.get_index(scope, index) {
            snapshot.push(v8::Global::new(scope, listener));
        }
    }
    snapshot
}

fn call_listener(
    scope: &mut v8::PinScope,
    signal: v8::Local<v8::Object>,
    listener: v8::Local<v8::Value>,
    event: v8::Local<v8::Value>,
) {
    let function = if listener.is_function() {
        v8::Local::<v8::Function>::try_from(listener).ok()
    } else if listener.is_object() {
        v8::Local::<v8::Object>::try_from(listener)
            .ok()
            .and_then(|object| {
                object.get(scope, v8::String::new(scope, "handleEvent").unwrap().into())
            })
            .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    } else {
        None
    };
    let Some(function) = function else {
        return;
    };
    let receiver = if listener.is_function() {
        signal.into()
    } else {
        listener
    };
    v8::tc_scope!(let try_catch, scope);
    let _ = function.call(try_catch, receiver, &[event]);
    if try_catch.has_caught() {
        try_catch.reset();
    }
}

fn abort_event<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    signal: v8::Local<v8::Object>,
) -> v8::Local<'a, v8::Value> {
    let global = scope.get_current_context().global(scope);
    if let Some(constructor) = global
        .get(scope, v8::String::new(scope, "Event").unwrap().into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        let event_type = v8::String::new(scope, "abort").unwrap();
        if let Some(event) = constructor.new_instance(scope, &[event_type.into()]) {
            event.set(
                scope,
                v8::String::new(scope, "target").unwrap().into(),
                signal.into(),
            );
            event.set(
                scope,
                v8::String::new(scope, "currentTarget").unwrap().into(),
                signal.into(),
            );
            return event.into();
        }
    }

    let event = v8::Object::new(scope);
    event.set(
        scope,
        v8::String::new(scope, "type").unwrap().into(),
        v8::String::new(scope, "abort").unwrap().into(),
    );
    event.set(
        scope,
        v8::String::new(scope, "target").unwrap().into(),
        signal.into(),
    );
    event.into()
}

fn dom_exception<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    message: &str,
    name: &str,
) -> v8::Local<'a, v8::Value> {
    let global = scope.get_current_context().global(scope);
    let message_value = v8::String::new(scope, message).unwrap();
    let name_value = v8::String::new(scope, name).unwrap();
    if let Some(constructor) = global
        .get(
            scope,
            v8::String::new(scope, "DOMException").unwrap().into(),
        )
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        if let Some(error) =
            constructor.new_instance(scope, &[message_value.into(), name_value.into()])
        {
            return error.into();
        }
    }

    let error = v8::Exception::error(scope, message_value);
    if let Ok(error_obj) = v8::Local::<v8::Object>::try_from(error) {
        error_obj.set(
            scope,
            v8::String::new(scope, "name").unwrap().into(),
            name_value.into(),
        );
    }
    error
}

fn signal_add_event_listener(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    if args.length() < 2 {
        return;
    }
    let event_type = args
        .get(0)
        .to_string(scope)
        .map(|value| value.to_rust_string_lossy(scope))
        .unwrap_or_default();
    if event_type != "abort" {
        return;
    }
    let listener = args.get(1);
    if !is_event_listener(scope, listener) {
        return;
    }

    let signal = args.this();
    if signal_is_aborted(scope, signal) {
        let event = abort_event(scope, signal);
        call_listener(scope, signal, listener, event);
        return;
    }

    let listeners = listener_array(scope, signal);
    let once = listener_once(scope, args.get(2));
    if !once && listener_index(scope, listeners, listener).is_some() {
        return;
    }
    let stored = if once {
        let record = v8::Object::new(scope);
        record.set(
            scope,
            v8::String::new(scope, "callback").unwrap().into(),
            listener,
        );
        record.set(
            scope,
            v8::String::new(scope, "once").unwrap().into(),
            v8::Boolean::new(scope, true).into(),
        );
        record.into()
    } else {
        listener
    };
    listeners.set_index(scope, listeners.length(), stored);
}

fn signal_remove_event_listener(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    if args.length() < 2 {
        return;
    }
    let event_type = args
        .get(0)
        .to_string(scope)
        .map(|value| value.to_rust_string_lossy(scope))
        .unwrap_or_default();
    if event_type != "abort" {
        return;
    }
    let signal = args.this();
    let listeners = listener_array(scope, signal);
    let Some(index) = listener_index(scope, listeners, args.get(1)) else {
        return;
    };
    let filtered = v8::Array::new(scope, 0);
    let mut next = 0u32;
    for cursor in 0..listeners.length() {
        if cursor == index {
            continue;
        }
        if let Some(existing) = listeners.get_index(scope, cursor) {
            filtered.set_index(scope, next, existing);
            next += 1;
        }
    }
    signal.set(
        scope,
        v8::String::new(scope, "_abortListeners").unwrap().into(),
        filtered.into(),
    );
}

fn signal_throw_if_aborted(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let signal = args.this();
    if !signal_is_aborted(scope, signal) {
        return;
    }
    let reason = signal
        .get(scope, v8::String::new(scope, "reason").unwrap().into())
        .unwrap_or_else(|| v8::undefined(scope).into());
    scope.throw_exception(reason);
}

fn listener_array<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    signal: v8::Local<v8::Object>,
) -> v8::Local<'a, v8::Array> {
    if let Some(value) = signal.get(
        scope,
        v8::String::new(scope, "_abortListeners").unwrap().into(),
    ) {
        if let Ok(array) = v8::Local::<v8::Array>::try_from(value) {
            return array;
        }
    }
    let array = v8::Array::new(scope, 0);
    signal.set(
        scope,
        v8::String::new(scope, "_abortListeners").unwrap().into(),
        array.into(),
    );
    array
}

fn listener_callback<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    listener: v8::Local<'a, v8::Value>,
) -> v8::Local<'a, v8::Value> {
    if listener.is_object() && !listener.is_function() {
        if let Ok(record) = v8::Local::<v8::Object>::try_from(listener) {
            if let Some(callback) =
                record.get(scope, v8::String::new(scope, "callback").unwrap().into())
            {
                if !callback.is_undefined() {
                    return callback;
                }
            }
        }
    }
    listener
}

fn listener_index(
    scope: &mut v8::PinScope,
    listeners: v8::Local<v8::Array>,
    expected: v8::Local<v8::Value>,
) -> Option<u32> {
    for index in 0..listeners.length() {
        if let Some(existing) = listeners.get_index(scope, index) {
            if listener_callback(scope, existing).strict_equals(expected) {
                return Some(index);
            }
        }
    }
    None
}

fn listener_once(scope: &mut v8::PinScope, options: v8::Local<v8::Value>) -> bool {
    let Ok(options) = v8::Local::<v8::Object>::try_from(options) else {
        return false;
    };
    options
        .get(scope, v8::String::new(scope, "once").unwrap().into())
        .is_some_and(|value| value.is_true())
}

fn is_event_listener(scope: &mut v8::PinScope, listener: v8::Local<v8::Value>) -> bool {
    if listener.is_function() {
        return true;
    }
    let Ok(object) = v8::Local::<v8::Object>::try_from(listener) else {
        return false;
    };
    object
        .get(scope, v8::String::new(scope, "handleEvent").unwrap().into())
        .is_some_and(|value| value.is_function())
}

fn install_static_methods(scope: &mut v8::ContextScope<v8::HandleScope>) {
    let helper = r#"
    (function() {
        if (typeof AbortSignal !== 'function') return;

        AbortSignal.abort = function(reason) {
            const controller = new AbortController();
            if (arguments.length === 0 || reason === undefined) controller.abort();
            else controller.abort(reason);
            return controller.signal;
        };

        AbortSignal.timeout = function(milliseconds) {
            const ms = Number(milliseconds);
            if (!Number.isFinite(ms) || ms < 0) {
                throw new TypeError('AbortSignal.timeout milliseconds must be a finite non-negative number');
            }
            const controller = new AbortController();
            setTimeout(() => {
                let reason;
                if (typeof DOMException === 'function') {
                    reason = new DOMException('The operation was aborted due to timeout', 'TimeoutError');
                } else {
                    reason = new Error('The operation was aborted due to timeout');
                    reason.name = 'TimeoutError';
                }
                controller.abort(reason);
            }, ms);
            return controller.signal;
        };

        AbortSignal.any = function(signals) {
            if (signals == null || typeof signals[Symbol.iterator] !== 'function') {
                throw new TypeError('AbortSignal.any requires an iterable of AbortSignals');
            }
            const list = Array.from(signals);
            const controller = new AbortController();
            for (const signal of list) {
                if (signal == null || typeof signal !== 'object' || typeof signal.aborted !== 'boolean') {
                    throw new TypeError('AbortSignal.any element is not an AbortSignal');
                }
                if (signal.aborted) {
                    controller.abort(signal.reason);
                    return controller.signal;
                }
            }
            for (const signal of list) {
                signal.addEventListener('abort', () => {
                    controller.abort(signal.reason);
                }, { once: true });
            }
            return controller.signal;
        };
    })();
    "#;
    if let Some(code) = v8::String::new(scope, helper) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }
}

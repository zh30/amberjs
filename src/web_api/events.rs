// EventTarget and Event API implementation for Web standard
// Provides addEventListener, removeEventListener, dispatchEvent, Event, ExtendableEvent

use rusty_v8 as v8;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Pending `ExtendableEvent.waitUntil` promises on the main isolate.
/// Keeps `runtime_minimal`'s event loop alive until they settle (same idea as
/// Background Sync's counter). Service-worker install/activate waitUntil is
/// honored inside the worker isolate via the SW wrapper, not this counter.
static PENDING_EXTENDABLE_WAIT_UNTIL: AtomicUsize = AtomicUsize::new(0);

pub fn has_pending_wait_until() -> bool {
    PENDING_EXTENDABLE_WAIT_UNTIL.load(Ordering::SeqCst) > 0
}

pub fn reset_pending_wait_until() {
    PENDING_EXTENDABLE_WAIT_UNTIL.store(0, Ordering::SeqCst);
}

fn increment_pending_wait_until() {
    PENDING_EXTENDABLE_WAIT_UNTIL.fetch_add(1, Ordering::SeqCst);
}

fn decrement_pending_wait_until() {
    let _ =
        PENDING_EXTENDABLE_WAIT_UNTIL.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
            Some(count.saturating_sub(1))
        });
}

/// Event type enum
#[derive(Debug, Clone)]
pub enum EventType {
    Custom(String),
    BuiltIn(String),
}

/// Event structure
#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: String,
    pub target: Option<String>,
    pub bubbles: bool,
    pub cancelable: bool,
    pub composed: bool,
    pub current_target: Option<String>,
    pub default_prevented: bool,
    pub event_phase: u8,
    pub is_trusted: bool,
}
impl Event {
    pub fn new(event_type: String) -> Self {
        Self {
            event_type,
            target: None,
            bubbles: false,
            cancelable: false,
            composed: false,
            current_target: None,
            default_prevented: false,
            event_phase: 0,
            is_trusted: true,
        }
    }
}

/// ExtendableEvent - Base class for events that support waitUntil()
/// Used by ServiceWorker lifecycle events (install, activate)
#[derive(Debug, Clone)]
pub struct ExtendableEvent {
    pub event_type: String,
    pub target: Option<String>,
    pub bubbles: bool,
    pub cancelable: bool,
    pub composed: bool,
    pub current_target: Option<String>,
    pub default_prevented: bool,
    pub event_phase: u8,
    pub is_trusted: bool,
    pub is_extended: bool, // Whether waitUntil() has been called
}
impl ExtendableEvent {
    pub fn new(event_type: String) -> Self {
        Self {
            event_type,
            target: None,
            bubbles: false,
            cancelable: false,
            composed: false,
            current_target: None,
            default_prevented: false,
            event_phase: 0,
            is_trusted: true,
            is_extended: false,
        }
    }
}

fn bool_option(
    scope: &mut v8::PinScope,
    init: v8::Local<v8::Value>,
    key: &str,
    default: bool,
) -> bool {
    if !init.is_object() || init.is_null() {
        return default;
    }

    let Ok(init_obj) = v8::Local::<v8::Object>::try_from(init) else {
        return default;
    };
    let Some(key_string) = v8::String::new(scope, key) else {
        return default;
    };
    init_obj
        .get(scope, key_string.into())
        .map(|value| value.to_boolean(scope).boolean_value(scope))
        .unwrap_or(default)
}

fn prevent_default_if_cancelable(scope: &mut v8::PinScope, this: v8::Local<v8::Object>) {
    let Some(cancelable_key) = v8::String::new(scope, "cancelable") else {
        return;
    };
    let is_cancelable = this
        .get(scope, cancelable_key.into())
        .map(|value| value.to_boolean(scope).boolean_value(scope))
        .unwrap_or(false);
    if !is_cancelable {
        return;
    }

    let default_prevented_key = v8::String::new(scope, "defaultPrevented").unwrap();
    let true_val = v8::Boolean::new(scope, true);
    this.set(scope, default_prevented_key.into(), true_val.into());
}

/// EventTarget structure
#[derive(Clone)]
pub struct EventTarget {
    listeners: Arc<Mutex<HashMap<String, Vec<Box<dyn Fn(&Event) + Send + Sync>>>>>,
}
impl EventTarget {
    /// Create new EventTarget
    pub fn new() -> Self {
        Self {
            listeners: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    /// Add event listener
    pub fn add_event_listener(
        &self,
        event_type: String,
        listener: Box<dyn Fn(&Event) + Send + Sync>,
    ) {
        if let Ok(mut listeners) = self.listeners.lock() {
            listeners
                .entry(event_type)
                .or_insert_with(Vec::new)
                .push(listener);
        }
    }
    /// Remove event listener
    pub fn remove_event_listener(&self, event_type: &str) {
        if let Ok(mut listeners) = self.listeners.lock() {
            listeners.remove(event_type);
        }
    }
    /// Dispatch event
    pub fn dispatch_event(&self, event: &Event) -> bool {
        let result = true;
        if let Ok(listeners) = self.listeners.lock() {
            if let Some(event_listeners) = listeners.get(&event.event_type) {
                for listener in event_listeners {
                    listener(event);
                }
            }
        }
        result
    }
}
impl Default for EventTarget {
    fn default() -> Self {
        Self::new()
    }
}
fn set_event_constant(
    scope: &mut v8::PinScope,
    object: v8::Local<v8::Object>,
    name: &str,
    value: i32,
) {
    let key = v8::String::new(scope, name).unwrap();
    let number = v8::Integer::new(scope, value);
    object.set(scope, key.into(), number.into());
}

fn install_event_constants(scope: &mut v8::PinScope, object: v8::Local<v8::Object>) {
    set_event_constant(scope, object, "NONE", 0);
    set_event_constant(scope, object, "CAPTURING_PHASE", 1);
    set_event_constant(scope, object, "AT_TARGET", 2);
    set_event_constant(scope, object, "BUBBLING_PHASE", 3);
}

/// Setup EventTarget and Event API in V8 context
pub fn setup_events_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> anyhow::Result<()> {
    let event_target_template = v8::FunctionTemplate::new(scope, event_target_constructor_callback);
    event_target_template.set_class_name(v8::String::new(scope, "EventTarget").unwrap());
    let event_target_proto = event_target_template.prototype_template(scope);
    event_target_proto.set(
        v8::String::new(scope, "addEventListener").unwrap().into(),
        v8::FunctionTemplate::new(scope, event_target_add_event_listener_callback).into(),
    );
    event_target_proto.set(
        v8::String::new(scope, "removeEventListener")
            .unwrap()
            .into(),
        v8::FunctionTemplate::new(scope, event_target_remove_event_listener_callback).into(),
    );
    event_target_proto.set(
        v8::String::new(scope, "dispatchEvent").unwrap().into(),
        v8::FunctionTemplate::new(scope, event_target_dispatch_event_callback).into(),
    );
    let event_target_constructor = event_target_template.get_function(scope).unwrap();

    let global = context.global(scope);
    global.set(
        scope,
        v8::String::new(scope, "EventTarget").unwrap().into(),
        event_target_constructor.into(),
    );

    let event_template = v8::FunctionTemplate::new(scope, event_constructor_callback);
    event_template.set_class_name(v8::String::new(scope, "Event").unwrap());
    let event_proto = event_template.prototype_template(scope);
    event_proto.set(
        v8::String::new(scope, "preventDefault").unwrap().into(),
        v8::FunctionTemplate::new(scope, event_prevent_default_callback).into(),
    );
    event_proto.set(
        v8::String::new(scope, "stopPropagation").unwrap().into(),
        v8::FunctionTemplate::new(scope, event_stop_propagation_callback).into(),
    );
    event_proto.set(
        v8::String::new(scope, "stopImmediatePropagation")
            .unwrap()
            .into(),
        v8::FunctionTemplate::new(scope, event_stop_immediate_propagation_callback).into(),
    );
    event_proto.set(
        v8::String::new(scope, "composedPath").unwrap().into(),
        v8::FunctionTemplate::new(scope, event_composed_path_callback).into(),
    );
    let event_constructor = event_template.get_function(scope).unwrap();
    install_event_constants(scope, event_constructor.into());
    if let Some(prototype) = event_constructor
        .get(scope, v8::String::new(scope, "prototype").unwrap().into())
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    {
        install_event_constants(scope, prototype);
    }
    global.set(
        scope,
        v8::String::new(scope, "Event").unwrap().into(),
        event_constructor.into(),
    );

    let extendable_event_fn =
        v8::FunctionTemplate::new(scope, extendable_event_constructor_callback);
    extendable_event_fn.set_class_name(v8::String::new(scope, "ExtendableEvent").unwrap());
    let extendable_event_func = extendable_event_fn.get_function(scope).unwrap();
    global.set(
        scope,
        v8::String::new(scope, "ExtendableEvent").unwrap().into(),
        extendable_event_func.into(),
    );

    Ok(())
}
fn events_map<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    target: v8::Local<v8::Object>,
) -> v8::Local<'a, v8::Object> {
    let events_key = v8::String::new(scope, "_events").unwrap();
    target
        .get(scope, events_key.into())
        .filter(|value| value.is_object())
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
        .unwrap_or_else(|| {
            let events_obj = v8::Object::new(scope);
            let events_key = v8::String::new(scope, "_events").unwrap();
            target.set(scope, events_key.into(), events_obj.into());
            events_obj
        })
}

fn listener_bucket<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    events_obj: v8::Local<v8::Object>,
    event_type: &str,
) -> v8::Local<'a, v8::Array> {
    let listeners_key = v8::String::new(scope, event_type).unwrap();
    events_obj
        .get(scope, listeners_key.into())
        .filter(|value| value.is_array())
        .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
        .unwrap_or_else(|| {
            let new_array = v8::Array::new(scope, 0);
            let listeners_key = v8::String::new(scope, event_type).unwrap();
            events_obj.set(scope, listeners_key.into(), new_array.into());
            new_array
        })
}

fn object_bool(scope: &mut v8::PinScope, object: v8::Local<v8::Object>, key: &str) -> bool {
    object
        .get(scope, v8::String::new(scope, key).unwrap().into())
        .is_some_and(|value| value.is_true())
}

fn is_listener_value(scope: &mut v8::PinScope, listener: v8::Local<v8::Value>) -> bool {
    if listener.is_null() || listener.is_undefined() {
        return false;
    }
    if listener.is_function() {
        return true;
    }
    v8::Local::<v8::Object>::try_from(listener)
        .ok()
        .and_then(|object| object.get(scope, v8::String::new(scope, "handleEvent").unwrap().into()))
        .is_some_and(|value| value.is_function())
}

fn listener_flags<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    options: v8::Local<v8::Value>,
) -> (bool, bool, bool, Option<v8::Local<'a, v8::Object>>) {
    if options.is_boolean() {
        return (options.is_true(), false, false, None);
    }
    let Ok(object) = v8::Local::<v8::Object>::try_from(options) else {
        return (false, false, false, None);
    };
    let capture = object_bool(scope, object, "capture");
    let once = object_bool(scope, object, "once");
    let passive = object_bool(scope, object, "passive");
    let signal = object
        .get(scope, v8::String::new(scope, "signal").unwrap().into())
        .filter(|value| value.is_object())
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok());
    (capture, once, passive, signal)
}

fn record_callback<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    record: v8::Local<v8::Value>,
) -> Option<v8::Local<'a, v8::Value>> {
    let object = v8::Local::<v8::Object>::try_from(record).ok()?;
    object.get(scope, v8::String::new(scope, "callback").unwrap().into())
}

fn same_listener(
    scope: &mut v8::PinScope,
    record: v8::Local<v8::Value>,
    callback: v8::Local<v8::Value>,
    capture: bool,
) -> bool {
    let Some(existing_callback) = record_callback(scope, record) else {
        return false;
    };
    if !existing_callback.strict_equals(callback) {
        return false;
    }
    let Ok(object) = v8::Local::<v8::Object>::try_from(record) else {
        return false;
    };
    object_bool(scope, object, "capture") == capture
}

fn throw_type_error(scope: &mut v8::PinScope, message: &str) {
    let message = v8::String::new(scope, message).unwrap();
    scope.throw_exception(v8::Exception::type_error(scope, message));
}

fn throw_invalid_state(scope: &mut v8::PinScope, message: &str) {
    let global = scope.get_current_context().global(scope);
    let text = v8::String::new(scope, message).unwrap();
    let name = v8::String::new(scope, "InvalidStateError").unwrap();
    if let Some(constructor) = global
        .get(
            scope,
            v8::String::new(scope, "DOMException").unwrap().into(),
        )
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        if let Some(error) = constructor.new_instance(scope, &[text.into(), name.into()]) {
            scope.throw_exception(error.into());
            return;
        }
    }
    let error = v8::Exception::error(scope, text);
    if let Ok(object) = v8::Local::<v8::Object>::try_from(error) {
        object.set(
            scope,
            v8::String::new(scope, "name").unwrap().into(),
            name.into(),
        );
    }
    scope.throw_exception(error);
}

fn bind_listener_signal(
    scope: &mut v8::PinScope,
    signal: v8::Local<v8::Object>,
    target: v8::Local<v8::Object>,
    event_type: &str,
    callback: v8::Local<v8::Value>,
    capture: bool,
) {
    let Some(add) = signal
        .get(
            scope,
            v8::String::new(scope, "addEventListener").unwrap().into(),
        )
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    else {
        return;
    };
    let remover_source = r#"
        (function(target, type, listener, capture) {
            return function() { target.removeEventListener(type, listener, capture); };
        })
    "#;
    let Some(source) = v8::String::new(scope, remover_source) else {
        return;
    };
    let Some(script) = v8::Script::compile(scope, source, None) else {
        return;
    };
    let Some(factory_value) = script.run(scope) else {
        return;
    };
    let Ok(factory) = v8::Local::<v8::Function>::try_from(factory_value) else {
        return;
    };
    let event_type = v8::String::new(scope, event_type).unwrap();
    let capture_value = v8::Boolean::new(scope, capture);
    let Some(remover) = factory.call(
        scope,
        v8::undefined(scope).into(),
        &[
            target.into(),
            event_type.into(),
            callback,
            capture_value.into(),
        ],
    ) else {
        return;
    };
    let abort_type = v8::String::new(scope, "abort").unwrap();
    let _ = add.call(scope, signal.into(), &[abort_type.into(), remover]);
}

/// EventTarget constructor callback
fn event_target_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if !args.is_construct_call() {
        throw_type_error(scope, "EventTarget constructor must be called with new");
        return;
    }
    let event_target_obj = args.this();
    let events_obj = v8::Object::new(scope);
    event_target_obj.set(
        scope,
        v8::String::new(scope, "_events").unwrap().into(),
        events_obj.into(),
    );
    retval.set(event_target_obj.into());
}

fn event_target_add_event_listener_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    if args.length() < 2 {
        return;
    }
    let listener = args.get(1);
    if !is_listener_value(scope, listener) {
        return;
    }
    let event_type = args
        .get(0)
        .to_string(scope)
        .map(|value| value.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let (capture, once, passive, signal) = listener_flags(scope, args.get(2));
    if let Some(signal) = signal {
        if signal
            .get(scope, v8::String::new(scope, "aborted").unwrap().into())
            .is_some_and(|value| value.is_true())
        {
            return;
        }
    }

    let target = args.this();
    let events_obj = events_map(scope, target);
    let listeners = listener_bucket(scope, events_obj, &event_type);
    for index in 0..listeners.length() {
        if let Some(existing) = listeners.get_index(scope, index) {
            if same_listener(scope, existing, listener, capture) {
                return;
            }
        }
    }

    let record = v8::Object::new(scope);
    record.set(
        scope,
        v8::String::new(scope, "callback").unwrap().into(),
        listener,
    );
    record.set(
        scope,
        v8::String::new(scope, "capture").unwrap().into(),
        v8::Boolean::new(scope, capture).into(),
    );
    record.set(
        scope,
        v8::String::new(scope, "once").unwrap().into(),
        v8::Boolean::new(scope, once).into(),
    );
    record.set(
        scope,
        v8::String::new(scope, "passive").unwrap().into(),
        v8::Boolean::new(scope, passive).into(),
    );
    listeners.set_index(scope, listeners.length(), record.into());

    if let Some(signal) = signal {
        bind_listener_signal(scope, signal, target, &event_type, listener, capture);
    }
}

fn event_target_remove_event_listener_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    if args.length() < 2 {
        return;
    }
    let listener = args.get(1);
    if !is_listener_value(scope, listener) {
        return;
    }
    let event_type = args
        .get(0)
        .to_string(scope)
        .map(|value| value.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let (capture, _, _, _) = listener_flags(scope, args.get(2));
    let target = args.this();
    let events_obj = events_map(scope, target);
    let Some(listeners_value) =
        events_obj.get(scope, v8::String::new(scope, &event_type).unwrap().into())
    else {
        return;
    };
    let Ok(listeners) = v8::Local::<v8::Array>::try_from(listeners_value) else {
        return;
    };

    let filtered = v8::Array::new(scope, 0);
    let mut next = 0u32;
    for index in 0..listeners.length() {
        if let Some(existing) = listeners.get_index(scope, index) {
            if same_listener(scope, existing, listener, capture) {
                continue;
            }
            filtered.set_index(scope, next, existing);
            next += 1;
        }
    }
    events_obj.set(
        scope,
        v8::String::new(scope, &event_type).unwrap().into(),
        filtered.into(),
    );
}

fn invoke_listener(
    scope: &mut v8::PinScope,
    target: v8::Local<v8::Object>,
    callback: v8::Local<v8::Value>,
    event: v8::Local<v8::Value>,
) {
    let (function, receiver) = if callback.is_function() {
        (
            v8::Local::<v8::Function>::try_from(callback).ok(),
            target.into(),
        )
    } else {
        let function = v8::Local::<v8::Object>::try_from(callback)
            .ok()
            .and_then(|object| {
                object.get(scope, v8::String::new(scope, "handleEvent").unwrap().into())
            })
            .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok());
        (function, callback)
    };
    let Some(function) = function else {
        return;
    };
    v8::tc_scope!(let try_catch, scope);
    let _ = function.call(try_catch, receiver, &[event]);
    if try_catch.has_caught() {
        try_catch.reset();
    }
}

fn event_target_dispatch_event_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let event = args.get(0);
    let Ok(event_obj) = v8::Local::<v8::Object>::try_from(event) else {
        throw_type_error(
            scope,
            "Failed to execute 'dispatchEvent': parameter 1 is not of type 'Event'.",
        );
        return;
    };
    if object_bool(scope, event_obj, "_dispatching") {
        throw_invalid_state(scope, "The event is already being dispatched.");
        return;
    }

    let target = args.this();
    event_obj.set(
        scope,
        v8::String::new(scope, "_dispatching").unwrap().into(),
        v8::Boolean::new(scope, true).into(),
    );
    event_obj.set(
        scope,
        v8::String::new(scope, "target").unwrap().into(),
        target.into(),
    );
    event_obj.set(
        scope,
        v8::String::new(scope, "currentTarget").unwrap().into(),
        target.into(),
    );
    event_obj.set(
        scope,
        v8::String::new(scope, "eventPhase").unwrap().into(),
        v8::Integer::new(scope, 2).into(),
    );

    let event_type = event_obj
        .get(scope, v8::String::new(scope, "type").unwrap().into())
        .and_then(|value| value.to_string(scope))
        .map(|value| value.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let events_obj = events_map(scope, target);
    let listeners = listener_bucket(scope, events_obj, &event_type);
    let mut capture_listeners = Vec::new();
    let mut bubble_listeners = Vec::new();
    for index in 0..listeners.length() {
        let Some(record) = listeners.get_index(scope, index) else {
            continue;
        };
        let global = v8::Global::new(scope, record);
        let is_capture = record
            .to_object(scope)
            .is_some_and(|object| object_bool(scope, object, "capture"));
        if is_capture {
            capture_listeners.push(global);
        } else {
            bubble_listeners.push(global);
        }
    }

    for record_global in capture_listeners.into_iter().chain(bubble_listeners) {
        if object_bool(scope, event_obj, "_stopImmediate") {
            break;
        }
        let record = v8::Local::new(scope, record_global);
        let Some(record_obj) = record.to_object(scope) else {
            continue;
        };
        let once = object_bool(scope, record_obj, "once");
        let capture = object_bool(scope, record_obj, "capture");
        let passive = object_bool(scope, record_obj, "passive");
        let Some(callback) = record_callback(scope, record) else {
            continue;
        };
        let events = events_map(scope, target);
        let still_present = listener_bucket(scope, events, &event_type);
        let mut registered = false;
        for index in 0..still_present.length() {
            if let Some(existing) = still_present.get_index(scope, index) {
                if same_listener(scope, existing, callback, capture) {
                    registered = true;
                    break;
                }
            }
        }
        if !registered {
            continue;
        }
        if once {
            let filtered = v8::Array::new(scope, 0);
            let mut next = 0u32;
            for index in 0..still_present.length() {
                if let Some(existing) = still_present.get_index(scope, index) {
                    if same_listener(scope, existing, callback, capture) {
                        continue;
                    }
                    filtered.set_index(scope, next, existing);
                    next += 1;
                }
            }
            events_map(scope, target).set(
                scope,
                v8::String::new(scope, &event_type).unwrap().into(),
                filtered.into(),
            );
        }
        event_set_bool(scope, event_obj, "_passive", passive);
        invoke_listener(scope, target, callback, event);
        event_set_bool(scope, event_obj, "_passive", false);
    }

    event_obj.set(
        scope,
        v8::String::new(scope, "currentTarget").unwrap().into(),
        v8::null(scope).into(),
    );
    event_obj.set(
        scope,
        v8::String::new(scope, "eventPhase").unwrap().into(),
        v8::Integer::new(scope, 0).into(),
    );
    event_obj.set(
        scope,
        v8::String::new(scope, "_dispatching").unwrap().into(),
        v8::Boolean::new(scope, false).into(),
    );
    let default_prevented = object_bool(scope, event_obj, "defaultPrevented");
    rv.set(v8::Boolean::new(scope, !default_prevented).into());
}

fn event_set_bool(scope: &mut v8::PinScope, event: v8::Local<v8::Object>, key: &str, value: bool) {
    event.set(
        scope,
        v8::String::new(scope, key).unwrap().into(),
        v8::Boolean::new(scope, value).into(),
    );
}

fn event_timestamp(scope: &mut v8::PinScope) -> f64 {
    let global = scope.get_current_context().global(scope);
    if let Some(performance) = global
        .get(scope, v8::String::new(scope, "performance").unwrap().into())
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    {
        if let Some(now) = performance
            .get(scope, v8::String::new(scope, "now").unwrap().into())
            .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
        {
            if let Some(value) = now.call(scope, performance.into(), &[]) {
                if let Some(number) = value.to_number(scope) {
                    return number.value();
                }
            }
        }
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

fn event_prevent_default_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let this = args.this();
    if object_bool(scope, this, "_passive") {
        return;
    }
    prevent_default_if_cancelable(scope, this);
}

fn event_stop_propagation_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    event_set_bool(scope, args.this(), "_stopPropagation", true);
}

fn event_stop_immediate_propagation_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    event_set_bool(scope, args.this(), "_stopPropagation", true);
    event_set_bool(scope, args.this(), "_stopImmediate", true);
}

fn event_composed_path_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let path = v8::Array::new(scope, 0);
    let event = args.this();
    if object_bool(scope, event, "_dispatching") {
        if let Some(target) = event.get(
            scope,
            v8::String::new(scope, "currentTarget").unwrap().into(),
        ) {
            if target.is_object() {
                path.set_index(scope, 0, target);
            }
        }
    }
    rv.set(path.into());
}

/// Event constructor callback
fn event_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    if !args.is_construct_call() {
        throw_type_error(
            scope,
            "Failed to construct 'Event': Please use the 'new' operator.",
        );
        return;
    }
    if args.length() < 1 {
        throw_type_error(
            scope,
            "Failed to construct 'Event': 1 argument required, but only 0 present.",
        );
        return;
    }

    let event_obj = args.this();
    let event_type = args
        .get(0)
        .to_string(scope)
        .map(|value| value.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let init = if args.length() > 1 {
        args.get(1)
    } else {
        v8::undefined(scope).into()
    };
    let bubbles = bool_option(scope, init, "bubbles", false);
    let cancelable = bool_option(scope, init, "cancelable", false);
    let composed = bool_option(scope, init, "composed", false);
    let type_val = v8::String::new(scope, &event_type).unwrap();
    event_obj.set(
        scope,
        v8::String::new(scope, "type").unwrap().into(),
        type_val.into(),
    );
    event_set_bool(scope, event_obj, "bubbles", bubbles);
    event_set_bool(scope, event_obj, "cancelable", cancelable);
    event_set_bool(scope, event_obj, "composed", composed);
    event_set_bool(scope, event_obj, "defaultPrevented", false);
    event_set_bool(scope, event_obj, "isTrusted", false);
    event_set_bool(scope, event_obj, "_dispatching", false);
    event_set_bool(scope, event_obj, "_stopImmediate", false);
    event_set_bool(scope, event_obj, "_stopPropagation", false);
    event_set_bool(scope, event_obj, "_passive", false);
    event_obj.set(
        scope,
        v8::String::new(scope, "eventPhase").unwrap().into(),
        v8::Integer::new(scope, 0).into(),
    );
    event_obj.set(
        scope,
        v8::String::new(scope, "target").unwrap().into(),
        v8::null(scope).into(),
    );
    event_obj.set(
        scope,
        v8::String::new(scope, "currentTarget").unwrap().into(),
        v8::null(scope).into(),
    );
    let timestamp = event_timestamp(scope);
    let time_key = v8::String::new(scope, "timeStamp").unwrap();
    let time_value = v8::Number::new(scope, timestamp);
    event_obj.set(scope, time_key.into(), time_value.into());
    rv.set(event_obj.into());
}

/// ExtendableEvent constructor callback
fn extendable_event_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let event_obj = v8::Object::new(scope);

    // Get event type from arguments
    let event_type = if args.length() > 0 {
        args.get(0)
            .to_string(scope)
            .unwrap_or_else(|| v8::String::new(scope, "").unwrap())
            .to_rust_string_lossy(scope)
    } else {
        "".to_string()
    };
    let init = args.get(1);
    let bubbles = bool_option(scope, init, "bubbles", false);
    let cancelable = bool_option(scope, init, "cancelable", false);
    let composed = bool_option(scope, init, "composed", false);

    // Store type as internal property
    let type_key = v8::String::new(scope, "_type").unwrap();
    let type_val = v8::String::new(scope, &event_type).unwrap();
    event_obj.set(scope, type_key.into(), type_val.into());

    // Set properties - extract values first to avoid scope borrow issues
    let type_prop_key = v8::String::new(scope, "type").unwrap();
    event_obj.set(scope, type_prop_key.into(), type_val.into());

    let bubbles_false = v8::Boolean::new(scope, bubbles);
    let bubbles_key = v8::String::new(scope, "bubbles").unwrap();
    event_obj.set(scope, bubbles_key.into(), bubbles_false.into());

    let cancelable_true = v8::Boolean::new(scope, cancelable);
    let cancelable_key = v8::String::new(scope, "cancelable").unwrap();
    event_obj.set(scope, cancelable_key.into(), cancelable_true.into());

    let composed_key = v8::String::new(scope, "composed").unwrap();
    let composed_val = v8::Boolean::new(scope, composed);
    event_obj.set(scope, composed_key.into(), composed_val.into());

    let default_prevented_false = v8::Boolean::new(scope, false);
    let default_prevented_key = v8::String::new(scope, "defaultPrevented").unwrap();
    event_obj.set(
        scope,
        default_prevented_key.into(),
        default_prevented_false.into(),
    );

    let prevent_default_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {
            let this = args.this();
            prevent_default_if_cancelable(scope, this);
        },
    )
    .unwrap();
    let prevent_default_key = v8::String::new(scope, "preventDefault").unwrap();
    event_obj.set(scope, prevent_default_key.into(), prevent_default_fn.into());

    let wait_until_fn = v8::Function::new(scope, extendable_event_wait_until_callback).unwrap();
    let wait_until_key = v8::String::new(scope, "waitUntil").unwrap();
    event_obj.set(scope, wait_until_key.into(), wait_until_fn.into());

    rv.set(event_obj.into());
}

/// Shared `waitUntil` body for `ExtendableEvent` and Install/Activate ctors.
pub(crate) fn extendable_event_wait_until_for_sw(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    rv: v8::ReturnValue,
) {
    extendable_event_wait_until_callback(scope, args, rv);
}

/// `ExtendableEvent.waitUntil(promise)` — tracks pending promises on the main
/// isolate so the CLI event loop stays alive until they settle.
fn extendable_event_wait_until_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    if args.length() == 0 {
        let error = v8::String::new(scope, "waitUntil requires a promise").unwrap();
        let exception = v8::Exception::type_error(scope, error);
        scope.throw_exception(exception);
        return;
    }

    let promise = args.get(0);
    if !promise.is_promise() {
        // Non-promise values are wrapped so callers can pass thenables later;
        // a plain value settles immediately and does not keep the loop alive.
        rv.set(v8::undefined(scope).into());
        return;
    }

    increment_pending_wait_until();

    let done_func = v8::Function::new(scope, extendable_event_wait_until_done_callback).unwrap();
    let then_key = v8::String::new(scope, "then").unwrap();
    let mut attached_handler = false;

    if let Ok(promise_obj) = v8::Local::<v8::Object>::try_from(promise) {
        if let Some(then_value) = promise_obj.get(scope, then_key.into()) {
            if let Ok(then_func) = v8::Local::<v8::Function>::try_from(then_value) {
                let done_value: v8::Local<v8::Value> = done_func.into();
                let then_args = [done_value, done_value];
                if then_func.call(scope, promise, &then_args).is_some() {
                    attached_handler = true;
                }
            }
        }
    }

    if !attached_handler {
        decrement_pending_wait_until();
    }

    rv.set(v8::undefined(scope).into());
}

fn extendable_event_wait_until_done_callback(
    _scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    decrement_pending_wait_until();
}
#[cfg(test)]
mod tests {
    use super::{Event, EventTarget};
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_event_creation() {
        let event: _ = Event::new("click".to_string());
        assert_eq!(event.event_type, "click");
        assert_eq!(event.bubbles, false);
        assert_eq!(event.cancelable, false);
    }
    #[test]
    fn test_event_target_creation() {
        let target: _ = EventTarget::new();
        assert!(target.listeners.lock().is_ok());
    }
    #[test]
    fn test_event_listener_management() {
        let target: _ = EventTarget::new();
        let event_called: _ = Arc::new(Mutex::new(false));
        let event_called_clone: _ = event_called.clone();
        let listener: _ = Box::new(move |event: &Event| {
            if event.event_type == "test" {
                *event_called_clone.lock().unwrap() = true;
            }
        });
        target.add_event_listener("test".to_string(), listener);
        let event: _ = Event::new("test".to_string());
        let result: _ = target.dispatch_event(&event);
        assert!(result);
        assert!(*event_called.lock().unwrap());
    }
}

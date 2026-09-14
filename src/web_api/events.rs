// EventTarget and Event API implementation for Web standard
// Provides addEventListener, removeEventListener, dispatchEvent, Event, ExtendableEvent

use rusty_v8 as v8;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
/// Setup EventTarget and Event API in V8 context
pub fn setup_events_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> anyhow::Result<()> {
    // Create EventTarget constructor
    let event_target_template: _ =
        v8::FunctionTemplate::new(scope, event_target_constructor_callback);
    let event_target_constructor: _ = event_target_template.get_function(scope).unwrap();

    // Set EventTarget to global
    let global: _ = context.global(scope);
    let event_target_key: _ = v8::String::new(scope, "EventTarget").unwrap();
    global.set(
        scope,
        event_target_key.into(),
        event_target_constructor.into(),
    );

    // Set Event constructor to global
    let event_fn = v8::FunctionTemplate::new(scope, event_constructor_callback);
    let event_constructor_func = event_fn.get_function(scope).unwrap();
    let event_key: _ = v8::String::new(scope, "Event").unwrap();
    global.set(scope, event_key.into(), event_constructor_func.into());

    // Set ExtendableEvent to global
    let extendable_event_fn =
        v8::FunctionTemplate::new(scope, extendable_event_constructor_callback);
    let extendable_event_func = extendable_event_fn.get_function(scope).unwrap();
    let extendable_event_key: _ = v8::String::new(scope, "ExtendableEvent").unwrap();
    global.set(
        scope,
        extendable_event_key.into(),
        extendable_event_func.into(),
    );

    Ok(())
}
/// EventTarget constructor callback
fn event_target_constructor_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let event_target_obj = _args.this();

    let events_key = v8::String::new(scope, "_events").unwrap();
    let events_obj = v8::Object::new(scope);
    event_target_obj.set(scope, events_key.into(), events_obj.into());

    let add_event_key = v8::String::new(scope, "addEventListener").unwrap();
    let add_event_func = v8::FunctionTemplate::new(scope, event_target_add_event_listener_callback);
    let add_event_func_instance: _ = add_event_func.get_function(scope).unwrap();
    event_target_obj.set(scope, add_event_key.into(), add_event_func_instance.into());

    let remove_event_key = v8::String::new(scope, "removeEventListener").unwrap();
    let remove_event_func =
        v8::FunctionTemplate::new(scope, event_target_remove_event_listener_callback);
    let remove_event_func_instance: _ = remove_event_func.get_function(scope).unwrap();
    event_target_obj.set(
        scope,
        remove_event_key.into(),
        remove_event_func_instance.into(),
    );

    let dispatch_event_key = v8::String::new(scope, "dispatchEvent").unwrap();
    let dispatch_event_func =
        v8::FunctionTemplate::new(scope, event_target_dispatch_event_callback);
    let dispatch_event_func_instance: _ = dispatch_event_func.get_function(scope).unwrap();
    event_target_obj.set(
        scope,
        dispatch_event_key.into(),
        dispatch_event_func_instance.into(),
    );

    retval.set(event_target_obj.into());
}

fn event_target_add_event_listener_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let event_type = args.get(0);
    let listener = args.get(1);

    if !event_type.is_string() {
        let error =
            v8::String::new(scope, "addEventListener: event type must be a string").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    if !listener.is_function() {
        let error =
            v8::String::new(scope, "addEventListener: listener must be a function").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let target = args.this();
    let events_key = v8::String::new(scope, "_events").unwrap();
    let events_obj = target
        .get(scope, events_key.into())
        .filter(|value| value.is_object())
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
        .unwrap_or_else(|| {
            let events_obj = v8::Object::new(scope);
            let events_key = v8::String::new(scope, "_events").unwrap();
            target.set(scope, events_key.into(), events_obj.into());
            events_obj
        });
    let event_type_str = event_type
        .to_string(scope)
        .map(|value| value.to_rust_string_lossy(scope))
        .unwrap_or_default();

    let listeners_key = v8::String::new(scope, &event_type_str).unwrap();
    let listeners_array = events_obj
        .get(scope, listeners_key.into())
        .filter(|value| value.is_array())
        .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
        .unwrap_or_else(|| {
            let new_array = v8::Array::new(scope, 0);
            let listeners_key = v8::String::new(scope, &event_type_str).unwrap();
            events_obj.set(scope, listeners_key.into(), new_array.into());
            new_array
        });

    let index = listeners_array.length();
    listeners_array.set_index(scope, index, listener);
}

fn event_target_remove_event_listener_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let event_type = args.get(0);
    let listener = args.get(1);
    if !event_type.is_string() || !listener.is_function() {
        return;
    }

    let target = args.this();
    let events_key = v8::String::new(scope, "_events").unwrap();
    let events_obj = target
        .get(scope, events_key.into())
        .filter(|value| value.is_object())
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
        .unwrap_or_else(|| {
            let events_obj = v8::Object::new(scope);
            let events_key = v8::String::new(scope, "_events").unwrap();
            target.set(scope, events_key.into(), events_obj.into());
            events_obj
        });
    let event_type_str = event_type
        .to_string(scope)
        .map(|value| value.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let listeners_key = v8::String::new(scope, &event_type_str).unwrap();

    let Some(listeners_value) = events_obj.get(scope, listeners_key.into()) else {
        return;
    };
    if !listeners_value.is_array() {
        return;
    }

    let listeners_array = v8::Local::<v8::Array>::try_from(listeners_value).unwrap();
    let new_array = v8::Array::new(scope, 0);
    let mut new_len = 0;

    for i in 0..listeners_array.length() {
        if let Some(existing_listener) = listeners_array.get_index(scope, i) {
            if !existing_listener.strict_equals(listener) {
                new_array.set_index(scope, new_len, existing_listener);
                new_len += 1;
            }
        }
    }

    let listeners_key = v8::String::new(scope, &event_type_str).unwrap();
    events_obj.set(scope, listeners_key.into(), new_array.into());
}

fn event_target_dispatch_event_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let event = args.get(0);
    if !event.is_object() {
        let error = v8::String::new(scope, "dispatchEvent: event must be an object").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let target = args.this();
    let event_obj = v8::Local::<v8::Object>::try_from(event).unwrap();
    let target_key = v8::String::new(scope, "target").unwrap();
    event_obj.set(scope, target_key.into(), target.into());
    let current_target_key = v8::String::new(scope, "currentTarget").unwrap();
    event_obj.set(scope, current_target_key.into(), target.into());

    let event_type_key = v8::String::new(scope, "type").unwrap();
    let event_type = event_obj
        .get(scope, event_type_key.into())
        .and_then(|value| value.to_string(scope))
        .map(|value| value.to_rust_string_lossy(scope))
        .unwrap_or_default();

    let events_key = v8::String::new(scope, "_events").unwrap();
    let events_obj = target
        .get(scope, events_key.into())
        .filter(|value| value.is_object())
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
        .unwrap_or_else(|| {
            let events_obj = v8::Object::new(scope);
            let events_key = v8::String::new(scope, "_events").unwrap();
            target.set(scope, events_key.into(), events_obj.into());
            events_obj
        });
    let listeners_key = v8::String::new(scope, &event_type).unwrap();
    if let Some(listeners_value) = events_obj.get(scope, listeners_key.into()) {
        if listeners_value.is_array() {
            let listeners_array = v8::Local::<v8::Array>::try_from(listeners_value).unwrap();
            for i in 0..listeners_array.length() {
                if let Some(listener_value) = listeners_array.get_index(scope, i) {
                    if listener_value.is_function() {
                        let listener = v8::Local::<v8::Function>::try_from(listener_value).unwrap();
                        let _ = listener.call(scope, target.into(), &[event]);
                    }
                }
            }
        }
    }

    let default_prevented_key = v8::String::new(scope, "defaultPrevented").unwrap();
    let default_prevented = event_obj
        .get(scope, default_prevented_key.into())
        .map(|value| value.to_boolean(scope).boolean_value(scope))
        .unwrap_or(false);
    rv.set(v8::Boolean::new(scope, !default_prevented).into());
}

/// Event constructor callback
fn event_constructor_callback(
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

    // Store type as internal property (using symbol)
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

    rv.set(event_obj.into());
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

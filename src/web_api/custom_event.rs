// CustomEvent API implementation for Web standard
// Provides CustomEvent interface for custom event handling
// Used for custom event dispatching in AI agent systems and UI frameworks

use rusty_v8 as v8;

fn bool_option(
    scope: &mut v8::PinScope,
    init_obj: v8::Local<v8::Object>,
    key: &str,
    default: bool,
) -> bool {
    let Some(key_string) = v8::String::new(scope, key) else {
        return default;
    };
    init_obj
        .get(scope, key_string.into())
        .map(|value| value.to_boolean(scope).boolean_value(scope))
        .unwrap_or(default)
}

fn has_own_property(scope: &mut v8::PinScope, object: v8::Local<v8::Object>, key: &str) -> bool {
    let Some(key_string) = v8::String::new(scope, key) else {
        return false;
    };
    object
        .get(scope, key_string.into())
        .map(|value| !value.is_undefined())
        .unwrap_or(false)
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

    let default_prevented_key = v8::String::new(scope, "defaultPrevented").unwrap().into();
    let true_val = v8::Boolean::new(scope, true);
    this.set(scope, default_prevented_key, true_val.into());
}

/// Setup CustomEvent API in V8 context
/// CustomEvent provides a way to create custom events with custom data (detail)
pub fn setup_custom_event_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) {
    let global = context.global(scope);

    // Create CustomEvent constructor
    let custom_event_func = v8::Function::new(scope, custom_event_constructor).unwrap();
    let custom_event_name = v8::String::new(scope, "CustomEvent").unwrap();
    global.set(scope, custom_event_name.into(), custom_event_func.into());

    // Create prototype object
    let prototype = v8::Object::new(scope);
    let prototype_name = v8::String::new(scope, "CustomEventPrototype").unwrap();
    global.set(scope, prototype_name.into(), prototype.into());

    // Inherit from Event
    let event_func_name = v8::String::new(scope, "Event").unwrap();
    let event_func = global.get(scope, event_func_name.into()).unwrap();
    if event_func.is_function() {
        let event_func: v8::Local<v8::Function> = v8::Local::cast(event_func);
        let prototype_of = v8::String::new(scope, "prototype").unwrap();
        let event_proto = event_func.get(scope, prototype_of.into()).unwrap();
        if event_proto.is_object() {
            let event_proto: v8::Local<v8::Object> = v8::Local::cast(event_proto);
            prototype.set_prototype(scope, event_proto.into());
        }
    }

    // Set up prototype methods - preventDefault from Event
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
    let prevent_default_key: v8::Local<v8::Name> =
        v8::String::new(scope, "preventDefault").unwrap().into();
    prototype.set(scope, prevent_default_key.into(), prevent_default_fn.into());

    // Set CustomEvent as global constructor (for instanceof checks)
    global.set(scope, custom_event_name.into(), custom_event_func.into());
}

/// CustomEvent constructor callback
/// CustomEvent(type, eventInitDict)
///
/// eventInitDict:
///   - detail: Custom event data (default: null)
///   - bubbles: Whether event bubbles (default: false)
///   - cancelable: Whether event is cancelable (default: false)
fn custom_event_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // Initialize default values
    let mut event_type = String::from("custom");
    let mut detail: Option<v8::Local<v8::Value>> = None;
    let mut bubbles = false;
    let mut cancelable = false;

    // Parse arguments
    if args.length() >= 1 {
        let type_arg = args.get(0);
        if type_arg.is_string() {
            event_type = type_arg
                .to_string(scope)
                .unwrap()
                .to_rust_string_lossy(scope);
        }
    }

    // Parse eventInitDict if second argument is provided
    if args.length() >= 2 {
        let dict = args.get(1);
        if dict.is_object() {
            let dict: v8::Local<v8::Object> = v8::Local::cast(dict);
            let mut has_explicit_detail = false;
            let has_event_init_field = has_own_property(scope, dict, "bubbles")
                || has_own_property(scope, dict, "cancelable")
                || has_own_property(scope, dict, "composed");

            // Get detail property
            let detail_key = v8::String::new(scope, "detail").unwrap();
            if let Some(val) = dict.get(scope, detail_key.into()) {
                if !val.is_undefined() {
                    detail = Some(val);
                    has_explicit_detail = true;
                }
            }

            // Older Beejs examples passed the payload object directly, or under
            // a "data" key, instead of the standard { detail } eventInitDict.
            if !has_explicit_detail {
                let data_key = v8::String::new(scope, "data").unwrap();
                if let Some(val) = dict.get(scope, data_key.into()) {
                    if !val.is_undefined() {
                        detail = Some(val);
                        has_explicit_detail = true;
                    }
                }
            }

            if !has_explicit_detail {
                if has_event_init_field {
                    detail = None;
                } else {
                    detail = Some(dict.into());
                }
            }

            bubbles = bool_option(scope, dict, "bubbles", false);
            cancelable = bool_option(scope, dict, "cancelable", false);
        }
    }

    // Create the CustomEvent object
    let event_obj = v8::Object::new(scope);

    // Set type
    let type_key = v8::String::new(scope, "type").unwrap();
    let type_val = v8::String::new(scope, &event_type).unwrap();
    event_obj.set(scope, type_key.into(), type_val.into());

    // Set detail property (custom event data)
    let detail_key = v8::String::new(scope, "detail").unwrap();
    if let Some(d) = detail {
        event_obj.set(scope, detail_key.into(), d);
    } else {
        let null_val: v8::Local<v8::Value> = v8::null(scope).into();
        event_obj.set(scope, detail_key.into(), null_val);
    }

    // Set inherited Event properties
    let bubbles_key = v8::String::new(scope, "bubbles").unwrap();
    let bubbles_val = v8::Boolean::new(scope, bubbles);
    event_obj.set(scope, bubbles_key.into(), bubbles_val.into());

    let cancelable_key = v8::String::new(scope, "cancelable").unwrap();
    let cancelable_val = v8::Boolean::new(scope, cancelable);
    event_obj.set(scope, cancelable_key.into(), cancelable_val.into());

    let composed_key = v8::String::new(scope, "composed").unwrap();
    let composed_val = v8::Boolean::new(scope, false);
    event_obj.set(scope, composed_key.into(), composed_val.into());

    // Set defaultPrevented (readonly, but we set initial value)
    let default_prevented_key = v8::String::new(scope, "defaultPrevented").unwrap();
    let default_prevented_val = v8::Boolean::new(scope, false);
    event_obj.set(
        scope,
        default_prevented_key.into(),
        default_prevented_val.into(),
    );

    // Set isTrusted
    let is_trusted_key = v8::String::new(scope, "isTrusted").unwrap();
    let is_trusted_val = v8::Boolean::new(scope, false);
    event_obj.set(scope, is_trusted_key.into(), is_trusted_val.into());

    // Add preventDefault method
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
    let prevent_default_key: v8::Local<v8::Name> =
        v8::String::new(scope, "preventDefault").unwrap().into();
    event_obj.set(scope, prevent_default_key.into(), prevent_default_fn.into());

    rv.set(event_obj.into());
}

/// Create a CustomEvent object for event dispatching
/// This is a helper function that can be used by other modules
pub fn create_custom_event_object<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    event_type: &str,
    detail: Option<v8::Local<'a, v8::Value>>,
) -> v8::Local<'a, v8::Object> {
    let event_obj = v8::Object::new(scope);

    // Set type
    let type_key = v8::String::new(scope, "type").unwrap();
    let type_val = v8::String::new(scope, event_type).unwrap();
    event_obj.set(scope, type_key.into(), type_val.into());

    // Set detail property
    let detail_key = v8::String::new(scope, "detail").unwrap();
    if let Some(d) = detail {
        event_obj.set(scope, detail_key.into(), d);
    } else {
        let null_val: v8::Local<v8::Value> = v8::null(scope).into();
        event_obj.set(scope, detail_key.into(), null_val);
    }

    // Set inherited Event properties
    let bubbles_key = v8::String::new(scope, "bubbles").unwrap();
    let bubbles_val = v8::Boolean::new(scope, false);
    event_obj.set(scope, bubbles_key.into(), bubbles_val.into());

    let cancelable_key = v8::String::new(scope, "cancelable").unwrap();
    let cancelable_val = v8::Boolean::new(scope, false);
    event_obj.set(scope, cancelable_key.into(), cancelable_val.into());

    let composed_key = v8::String::new(scope, "composed").unwrap();
    let composed_val = v8::Boolean::new(scope, false);
    event_obj.set(scope, composed_key.into(), composed_val.into());

    let default_prevented_key = v8::String::new(scope, "defaultPrevented").unwrap();
    let default_prevented_val = v8::Boolean::new(scope, false);
    event_obj.set(
        scope,
        default_prevented_key.into(),
        default_prevented_val.into(),
    );

    let is_trusted_key = v8::String::new(scope, "isTrusted").unwrap();
    let is_trusted_val = v8::Boolean::new(scope, false);
    event_obj.set(scope, is_trusted_key.into(), is_trusted_val.into());

    // Add preventDefault method
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
    let prevent_default_key: v8::Local<v8::Name> =
        v8::String::new(scope, "preventDefault").unwrap().into();
    event_obj.set(scope, prevent_default_key.into(), prevent_default_fn.into());

    event_obj
}

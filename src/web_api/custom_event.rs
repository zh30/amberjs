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

/// Setup CustomEvent API in V8 context.
/// Instances inherit Event.prototype, so preventDefault and the phase flags work.
pub fn setup_custom_event_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) {
    let global = context.global(scope);
    let template = v8::FunctionTemplate::new(scope, custom_event_constructor);
    template.set_class_name(v8::String::new(scope, "CustomEvent").unwrap());
    let constructor = template.get_function(scope).unwrap();

    if let Some(event_constructor) = global
        .get(scope, v8::String::new(scope, "Event").unwrap().into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
    {
        if let Some(event_proto) =
            event_constructor.get(scope, v8::String::new(scope, "prototype").unwrap().into())
        {
            if let Some(custom_proto) = constructor
                .get(scope, v8::String::new(scope, "prototype").unwrap().into())
                .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
            {
                let _ = custom_proto.set_prototype(scope, event_proto);
            }
        }
    }

    global.set(
        scope,
        v8::String::new(scope, "CustomEvent").unwrap().into(),
        constructor.into(),
    );
}

/// CustomEvent constructor callback
/// CustomEvent(type, eventInitDict)
///
/// eventInitDict:
///   - detail: Custom event data (default: null)
///   - bubbles: Whether event bubbles (default: false)
///   - cancelable: Whether event is cancelable (default: false)
fn set_bool(scope: &mut v8::PinScope, object: v8::Local<v8::Object>, key: &str, value: bool) {
    object.set(
        scope,
        v8::String::new(scope, key).unwrap().into(),
        v8::Boolean::new(scope, value).into(),
    );
}

fn custom_event_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    if !args.is_construct_call() {
        let message = v8::String::new(
            scope,
            "Failed to construct 'CustomEvent': Please use the 'new' operator.",
        )
        .unwrap();
        scope.throw_exception(v8::Exception::type_error(scope, message));
        return;
    }
    if args.length() < 1 {
        let message = v8::String::new(
            scope,
            "Failed to construct 'CustomEvent': 1 argument required, but only 0 present.",
        )
        .unwrap();
        scope.throw_exception(v8::Exception::type_error(scope, message));
        return;
    }

    let event_obj = args.this();
    let event_type = args
        .get(0)
        .to_string(scope)
        .map(|value| value.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let mut detail: Option<v8::Local<v8::Value>> = None;
    let mut bubbles = false;
    let mut cancelable = false;
    let mut composed = false;
    if args.length() >= 2 {
        if let Ok(dict) = v8::Local::<v8::Object>::try_from(args.get(1)) {
            if let Some(value) = dict.get(scope, v8::String::new(scope, "detail").unwrap().into()) {
                if !value.is_undefined() {
                    detail = Some(value);
                }
            }
            bubbles = bool_option(scope, dict, "bubbles", false);
            cancelable = bool_option(scope, dict, "cancelable", false);
            composed = bool_option(scope, dict, "composed", false);
        }
    }

    event_obj.set(
        scope,
        v8::String::new(scope, "type").unwrap().into(),
        v8::String::new(scope, &event_type).unwrap().into(),
    );
    event_obj.set(
        scope,
        v8::String::new(scope, "detail").unwrap().into(),
        detail.unwrap_or_else(|| v8::null(scope).into()),
    );
    set_bool(scope, event_obj, "bubbles", bubbles);
    set_bool(scope, event_obj, "cancelable", cancelable);
    set_bool(scope, event_obj, "composed", composed);
    set_bool(scope, event_obj, "defaultPrevented", false);
    set_bool(scope, event_obj, "isTrusted", false);
    set_bool(scope, event_obj, "_dispatching", false);
    set_bool(scope, event_obj, "_stopImmediate", false);
    set_bool(scope, event_obj, "_stopPropagation", false);
    set_bool(scope, event_obj, "_passive", false);
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
    event_obj.set(
        scope,
        v8::String::new(scope, "timeStamp").unwrap().into(),
        v8::Number::new(scope, 0.0).into(),
    );
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

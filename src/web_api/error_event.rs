// ErrorEvent API implementation for Web standard
// Provides ErrorEvent interface for script error handling
// Used by window.onerror, WebSocket onerror, Worker onerror, etc.

use rusty_v8 as v8;

/// Setup ErrorEvent API in V8 context
/// ErrorEvent provides detailed information about script errors
pub fn setup_error_event_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) {
    let global = context.global(scope);

    // Create ErrorEvent constructor
    let error_event_func = v8::Function::new(scope, error_event_constructor).unwrap();
    let error_event_name = v8::String::new(scope, "ErrorEvent").unwrap();
    global.set(scope, error_event_name.into(), error_event_func.into());

    // Create prototype object
    let prototype = v8::Object::new(scope);
    let prototype_name = v8::String::new(scope, "ErrorEventPrototype").unwrap();
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

    // Set up prototype methods
    // Note: ErrorEvent doesn't add any specific methods beyond Event
    // The main difference is in the constructor arguments

    // Set up ErrorEvent as global constructor (for instanceof checks)
    global.set(scope, error_event_name.into(), error_event_func.into());

    // Set up window.onerror property
    setup_window_onerror(scope, global);
}

/// Set up window.onerror property
/// Simply initializes it as a property on window that JavaScript can set/get
fn setup_window_onerror(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    global: v8::Local<v8::Object>,
) {
    let onerror_key = v8::String::new(scope, "onerror").unwrap();

    let onerror_fn = v8::Function::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            rv.set(v8::Boolean::new(_scope, false).into());
        },
    )
    .unwrap();
    global.set(scope, onerror_key.into(), onerror_fn.into());

    // Set window as an alias to globalThis for browser compatibility
    let window_key = v8::String::new(scope, "window").unwrap();
    global.set(scope, window_key.into(), global.into());
}

/// ErrorEvent constructor callback
/// ErrorEvent(type, eventInitDict)
///
/// eventInitDict:
///   - message: Error message (default: "")
///   - filename: Script file that error occurred in (default: "")
///   - lineno: Line number (default: 0)
///   - colno: Column number (default: 0)
///   - error: Error object (default: null)
fn error_event_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // Initialize default values
    let mut message = String::new();
    let mut filename = String::new();
    let mut lineno = 0;
    let mut colno = 0;
    let mut error_obj: Option<v8::Local<v8::Value>> = None;

    if args.length() > 0 {
        let first_arg = args.get(0);
        if first_arg.is_string() {
            let first_arg = first_arg
                .to_string(scope)
                .unwrap()
                .to_rust_string_lossy(scope);
            if first_arg != "error" {
                message = first_arg;
            }
        }
    }

    // Parse eventInitDict if second argument is provided
    if args.length() > 1 {
        let dict = args.get(1);
        if dict.is_object() {
            let dict: v8::Local<v8::Object> = v8::Local::cast(dict);

            // Get message
            let message_key = v8::String::new(scope, "message").unwrap();
            if let Some(val) = dict.get(scope, message_key.into()) {
                if val.is_string() {
                    message = val.to_string(scope).unwrap().to_rust_string_lossy(scope);
                }
            }

            // Get filename
            let filename_key = v8::String::new(scope, "filename").unwrap();
            if let Some(val) = dict.get(scope, filename_key.into()) {
                if val.is_string() {
                    filename = val.to_string(scope).unwrap().to_rust_string_lossy(scope);
                }
            }

            // Get lineno
            let lineno_key = v8::String::new(scope, "lineno").unwrap();
            if let Some(val) = dict.get(scope, lineno_key.into()) {
                if val.is_number() {
                    lineno = val.number_value(scope).unwrap() as u32;
                }
            }

            // Get colno
            let colno_key = v8::String::new(scope, "colno").unwrap();
            if let Some(val) = dict.get(scope, colno_key.into()) {
                if val.is_number() {
                    colno = val.number_value(scope).unwrap() as u32;
                }
            }

            // Get error object
            let error_key = v8::String::new(scope, "error").unwrap();
            if let Some(val) = dict.get(scope, error_key.into()) {
                if !val.is_undefined() && !val.is_null() {
                    error_obj = Some(val);
                }
            }
        }
    }

    // Create the ErrorEvent object
    let event_obj = v8::Object::new(scope);

    // Set type to "error"
    let type_key = v8::String::new(scope, "type").unwrap();
    let type_val = v8::String::new(scope, "error").unwrap();
    event_obj.set(scope, type_key.into(), type_val.into());

    // Set ErrorEvent specific properties
    let message_key = v8::String::new(scope, "message").unwrap();
    let message_val = v8::String::new(scope, &message).unwrap();
    event_obj.set(scope, message_key.into(), message_val.into());

    let filename_key = v8::String::new(scope, "filename").unwrap();
    let filename_val = v8::String::new(scope, &filename).unwrap();
    event_obj.set(scope, filename_key.into(), filename_val.into());

    let lineno_key = v8::String::new(scope, "lineno").unwrap();
    let lineno_val = v8::Integer::new(scope, lineno as i32);
    event_obj.set(scope, lineno_key.into(), lineno_val.into());

    let colno_key = v8::String::new(scope, "colno").unwrap();
    let colno_val = v8::Integer::new(scope, colno as i32);
    event_obj.set(scope, colno_key.into(), colno_val.into());

    // Set error property
    let error_key = v8::String::new(scope, "error").unwrap();
    if let Some(err) = error_obj {
        event_obj.set(scope, error_key.into(), err);
    } else {
        let null_val: v8::Local<v8::Value> = v8::null(scope).into();
        event_obj.set(scope, error_key.into(), null_val);
    }

    // Set inherited Event properties
    let bubbles_key = v8::String::new(scope, "bubbles").unwrap();
    let bubbles_val = v8::Boolean::new(scope, false);
    event_obj.set(scope, bubbles_key.into(), bubbles_val.into());

    let cancelable_key = v8::String::new(scope, "cancelable").unwrap();
    let cancelable_val = v8::Boolean::new(scope, true);
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

    rv.set(event_obj.into());
}

/// Create an ErrorEvent for error reporting
/// This is a helper function that can be used by other modules
/// (like WebSocket, Worker, etc.) to dispatch error events
pub fn create_error_event_object<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    message: &str,
    filename: &str,
    lineno: u32,
    colno: u32,
    error: Option<v8::Local<'a, v8::Value>>,
) -> v8::Local<'a, v8::Object> {
    let event_obj = v8::Object::new(scope);

    // Set type
    let type_key = v8::String::new(scope, "type").unwrap();
    let type_val = v8::String::new(scope, "error").unwrap();
    event_obj.set(scope, type_key.into(), type_val.into());

    // Set ErrorEvent properties
    let message_key = v8::String::new(scope, "message").unwrap();
    let message_val = v8::String::new(scope, message).unwrap();
    event_obj.set(scope, message_key.into(), message_val.into());

    let filename_key = v8::String::new(scope, "filename").unwrap();
    let filename_val = v8::String::new(scope, filename).unwrap();
    event_obj.set(scope, filename_key.into(), filename_val.into());

    let lineno_key = v8::String::new(scope, "lineno").unwrap();
    let lineno_val = v8::Integer::new(scope, lineno as i32);
    event_obj.set(scope, lineno_key.into(), lineno_val.into());

    let colno_key = v8::String::new(scope, "colno").unwrap();
    let colno_val = v8::Integer::new(scope, colno as i32);
    event_obj.set(scope, colno_key.into(), colno_val.into());

    // Set error property
    let error_key = v8::String::new(scope, "error").unwrap();
    if let Some(err) = error {
        event_obj.set(scope, error_key.into(), err);
    } else {
        let null_val: v8::Local<v8::Value> = v8::null(scope).into();
        event_obj.set(scope, error_key.into(), null_val);
    }

    // Set inherited Event properties
    let bubbles_key = v8::String::new(scope, "bubbles").unwrap();
    let bubbles_val = v8::Boolean::new(scope, false);
    event_obj.set(scope, bubbles_key.into(), bubbles_val.into());

    let cancelable_key = v8::String::new(scope, "cancelable").unwrap();
    let cancelable_val = v8::Boolean::new(scope, true);
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

    event_obj
}

/// Call window.onerror handler with error information
/// Returns true if the error was handled (onerror returned true), false otherwise
/// This function is called from the runtime when an uncaught exception occurs
pub fn call_onerror_handler(
    scope: &mut v8::PinScope,
    message: &str,
    filename: &str,
    lineno: u32,
    colno: u32,
    error: Option<v8::Local<v8::Value>>,
) -> bool {
    if scope.is_execution_terminating() {
        return false;
    }

    // Get the context and global object
    let context = scope.get_current_context();
    let global = context.global(scope);

    // Get window.onerror from JavaScript
    let onerror_key = match v8::String::new(scope, "onerror") {
        Some(k) => k,
        None => return false,
    };
    let onerror_val = match global.get(scope, onerror_key.into()) {
        Some(v) => v,
        None => return false,
    };

    // Check if onerror is a function
    if !onerror_val.is_function() {
        return false;
    }

    let handler: v8::Local<v8::Function> = v8::Local::cast(onerror_val);

    // Prepare arguments for onerror callback:
    // (message, filename, lineno, colno, error)
    let message_str = match v8::String::new(scope, message) {
        Some(s) => s,
        None => return false,
    };
    let filename_str = match v8::String::new(scope, filename) {
        Some(s) => s,
        None => return false,
    };
    let lineno_val = v8::Integer::new(scope, lineno as i32);
    let colno_val = v8::Integer::new(scope, colno as i32);
    let error_val = error.unwrap_or_else(|| v8::null(scope).into());

    let args = vec![
        message_str.into(),
        filename_str.into(),
        lineno_val.into(),
        colno_val.into(),
        error_val,
    ];

    let recv = v8::undefined(scope).into();

    // Call the onerror handler - call returns a Value
    let result_val = handler.call(scope, recv, &args);

    // Check if the handler returned true (error was handled)
    if let Some(result) = result_val {
        return result.is_true();
    }

    false
}

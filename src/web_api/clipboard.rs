// Clipboard API implementation
// Provides clipboard read/write capability for AI workloads
// that need to copy/paste content
//
// v0.3.342: Initial implementation of Clipboard API

use anyhow::Result;
use rusty_v8 as v8;

/// Set up Clipboard API in the V8 context
pub fn setup_clipboard_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Create navigator property if it doesn't exist
    let navigator_key = v8::String::new(scope, "navigator").unwrap();
    let navigator_val = global.get(scope, navigator_key.into());

    // Create or get navigator object
    let navigator_obj = if let Some(val) = navigator_val {
        if val.is_object() {
            v8::Local::<v8::Object>::try_from(val).unwrap()
        } else {
            let new_navigator: v8::Local<v8::Object> = v8::Object::new(scope);
            global.set(scope, navigator_key.into(), new_navigator.into());
            new_navigator
        }
    } else {
        let new_navigator: v8::Local<v8::Object> = v8::Object::new(scope);
        global.set(scope, navigator_key.into(), new_navigator.into());
        new_navigator
    };

    // Create clipboard object
    let clipboard_obj: v8::Local<v8::Object> = v8::Object::new(scope);

    // Create writeText method template
    let write_text_template = v8::FunctionTemplate::new(scope, write_text_callback);
    let write_text_fn = write_text_template.get_function(scope).unwrap();
    let write_text_key = v8::String::new(scope, "writeText").unwrap();
    clipboard_obj.set(scope, write_text_key.into(), write_text_fn.into());

    // Create readText method template
    let read_text_template = v8::FunctionTemplate::new(scope, read_text_callback);
    let read_text_fn = read_text_template.get_function(scope).unwrap();
    let read_text_key = v8::String::new(scope, "readText").unwrap();
    clipboard_obj.set(scope, read_text_key.into(), read_text_fn.into());

    // Create read method template (modern API)
    let read_template = v8::FunctionTemplate::new(scope, read_callback);
    let read_fn = read_template.get_function(scope).unwrap();
    let read_key = v8::String::new(scope, "read").unwrap();
    clipboard_obj.set(scope, read_key.into(), read_fn.into());

    // Create write method template (modern API)
    let write_template = v8::FunctionTemplate::new(scope, write_callback);
    let write_fn = write_template.get_function(scope).unwrap();
    let write_key = v8::String::new(scope, "write").unwrap();
    clipboard_obj.set(scope, write_key.into(), write_fn.into());

    // Set clipboard object on navigator
    let clipboard_key = v8::String::new(scope, "clipboard").unwrap();
    navigator_obj.set(scope, clipboard_key.into(), clipboard_obj.into());

    Ok(())
}

/// writeText callback - writes text to clipboard
fn write_text_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    retval.set(promise.into());

    let message = v8::String::new(scope, "Clipboard API is not supported yet").unwrap();
    let error = v8::Exception::error(scope, message);
    resolver.reject(scope, error).unwrap();
}

/// readText callback - reads text from clipboard
fn read_text_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    retval.set(promise.into());

    let message = v8::String::new(scope, "Clipboard API is not supported yet").unwrap();
    let error = v8::Exception::error(scope, message);
    resolver.reject(scope, error).unwrap();
}

/// read callback - modern read API (returns ClipboardItem array)
fn read_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    retval.set(promise.into());

    let message = v8::String::new(scope, "Clipboard API is not supported yet").unwrap();
    let error = v8::Exception::error(scope, message);
    resolver.reject(scope, error).unwrap();
}

/// write callback - modern write API (takes ClipboardItem array)
fn write_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    retval.set(promise.into());

    let message = v8::String::new(scope, "Clipboard API is not supported yet").unwrap();
    let error = v8::Exception::error(scope, message);
    resolver.reject(scope, error).unwrap();
}

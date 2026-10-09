// Clipboard API (Preview) — in-process text store for the CLI path.
//
// `navigator.clipboard.writeText` / `readText` resolve against a process-local
// string buffer. This is not the OS clipboard and does not implement secure
// context, permissions, or user-activation checks.
//
// `read` / `write` (ClipboardItem) stay rejected: rich clipboard is out of scope.

use anyhow::Result;
use rusty_v8 as v8;
use std::sync::{Mutex, OnceLock};

/// Process-local text clipboard shared by isolates in this Amber process.
/// Not the host OS clipboard. Cleared only when the process exits or a caller
/// overwrites it.
fn clipboard_store() -> &'static Mutex<String> {
    static STORE: OnceLock<Mutex<String>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(String::new()))
}

/// Set up Clipboard API in the V8 context (`runtime_minimal` / `amber run`).
pub fn setup_clipboard_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    let navigator_key = v8::String::new(scope, "navigator").unwrap();
    let navigator_val = global.get(scope, navigator_key.into());

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

    let clipboard_obj: v8::Local<v8::Object> = v8::Object::new(scope);

    let write_text_template = v8::FunctionTemplate::new(scope, write_text_callback);
    let write_text_fn = write_text_template.get_function(scope).unwrap();
    let write_text_key = v8::String::new(scope, "writeText").unwrap();
    clipboard_obj.set(scope, write_text_key.into(), write_text_fn.into());

    let read_text_template = v8::FunctionTemplate::new(scope, read_text_callback);
    let read_text_fn = read_text_template.get_function(scope).unwrap();
    let read_text_key = v8::String::new(scope, "readText").unwrap();
    clipboard_obj.set(scope, read_text_key.into(), read_text_fn.into());

    let read_template = v8::FunctionTemplate::new(scope, read_callback);
    let read_fn = read_template.get_function(scope).unwrap();
    let read_key = v8::String::new(scope, "read").unwrap();
    clipboard_obj.set(scope, read_key.into(), read_fn.into());

    let write_template = v8::FunctionTemplate::new(scope, write_callback);
    let write_fn = write_template.get_function(scope).unwrap();
    let write_key = v8::String::new(scope, "write").unwrap();
    clipboard_obj.set(scope, write_key.into(), write_fn.into());

    let clipboard_key = v8::String::new(scope, "clipboard").unwrap();
    navigator_obj.set(scope, clipboard_key.into(), clipboard_obj.into());

    Ok(())
}

fn reject_promise(scope: &mut v8::PinScope, mut retval: v8::ReturnValue, message: &str) {
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    retval.set(promise.into());
    let msg = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::error(scope, msg);
    let _ = resolver.reject(scope, error);
}

fn arg_to_string(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> String {
    if value.is_undefined() || value.is_null() {
        return String::new();
    }
    value
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default()
}

/// `navigator.clipboard.writeText(text)` — store text in the in-process buffer.
fn write_text_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    retval.set(promise.into());

    let text = if args.length() < 1 {
        String::new()
    } else {
        arg_to_string(scope, args.get(0))
    };

    match clipboard_store().lock() {
        Ok(mut store) => {
            *store = text;
            let undef = v8::undefined(scope);
            let _ = resolver.resolve(scope, undef.into());
        }
        Err(_) => {
            let msg = v8::String::new(scope, "Clipboard store is poisoned").unwrap();
            let error = v8::Exception::error(scope, msg);
            let _ = resolver.reject(scope, error);
        }
    }
}

/// `navigator.clipboard.readText()` — read the in-process buffer (empty string if unset).
fn read_text_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    retval.set(promise.into());

    match clipboard_store().lock() {
        Ok(store) => {
            let text_val = v8::String::new(scope, store.as_str()).unwrap();
            let _ = resolver.resolve(scope, text_val.into());
        }
        Err(_) => {
            let msg = v8::String::new(scope, "Clipboard store is poisoned").unwrap();
            let error = v8::Exception::error(scope, msg);
            let _ = resolver.reject(scope, error);
        }
    }
}

/// `navigator.clipboard.read()` — ClipboardItem not implemented (Preview Limit).
fn read_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    reject_promise(
        scope,
        retval,
        "ClipboardItem read is not implemented; use navigator.clipboard.readText()",
    );
}

/// `navigator.clipboard.write()` — ClipboardItem not implemented (Preview Limit).
fn write_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    reject_promise(
        scope,
        retval,
        "ClipboardItem write is not implemented; use navigator.clipboard.writeText()",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_round_trip() {
        {
            let mut store = clipboard_store().lock().unwrap();
            *store = "unit-test".to_string();
        }
        assert_eq!(clipboard_store().lock().unwrap().as_str(), "unit-test");
        {
            let mut store = clipboard_store().lock().unwrap();
            store.clear();
        }
    }
}

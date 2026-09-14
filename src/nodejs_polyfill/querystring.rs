/// querystring polyfill
use rusty_v8 as v8;
use std::collections::{HashMap, BTreeMap};
pub fn register(scope: &mut v8::PinScope, global: &v8::Local<v8::Object>) {
    let qs_key: _ = v8::String::new(scope, "querystring").unwrap();
    let qs_obj: _ = v8::Object::new(scope);
    // Parse query string
    let parse_fn: _ = v8::FunctionTemplate::new(scope, parse).get_function(scope).unwrap();
    let parse_key: _ = v8::String::new(scope, "parse").unwrap().into();
    qs_obj.set(scope, parse_key, parse_fn.into());
    // Stringify
    let stringify_fn: _ = v8::FunctionTemplate::new(scope, stringify).get_function(scope).unwrap();
    let stringify_key: _ = v8::String::new(scope, "stringify").unwrap().into();
    qs_obj.set(scope, stringify_key, stringify_fn.into());
    global.set(scope, qs_key.into(), qs_obj.into());
}
fn parse(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue) {
    let query_arg: _ = args.get(0);
    let query_str: _ = query_arg.to_string(scope).unwrap().to_rust_string_lossy(scope);
    let obj: _ = v8::Object::new(scope);
    for pair in query_str.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let key_str: _ = v8::String::new(scope, key).unwrap().into();
            let value_str: _ = v8::String::new(scope, value).unwrap().into();
            obj.set(scope, key_str, value_str);
        }
    }
    retval.set(obj.into());
}
fn stringify(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue) {
    let obj_arg: _ = args.get(0);
    let obj: _ = obj_arg.to_object(scope).unwrap();
    let keys: _ = obj.get_own_property_names(scope, Default::default()).unwrap();
    let mut parts = Vec::new();
    for i in 0..keys.length() {
        let key_v8: _ = keys.get_index(scope, i).unwrap();
        let key_str: _ = key_v8.to_string(scope).unwrap().to_rust_string_lossy(scope);
        // Create a separate scope for the get operation to avoid borrow conflicts
        {
            let key_for_get: _ = v8::String::new(scope, &key_str).unwrap();
            let value_v8: _ = obj.get(scope, key_for_get.into()).unwrap();
            let value_str: _ = value_v8.to_string(scope).unwrap().to_rust_string_lossy(scope);
            parts.push(format!("{}={}", key_str, value_str));
        }
    }
    retval.set(v8::String::new(scope, &parts.join("&")).unwrap().into());
}
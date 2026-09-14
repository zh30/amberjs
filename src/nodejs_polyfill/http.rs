/// http polyfill
use rusty_v8 as v8;
use std::collections::{HashMap, BTreeMap};
pub fn register(scope: &mut v8::PinScope, global: &v8::Local<v8::Object>) {
    let http_key: _ = v8::String::new(scope, "http").unwrap();
    let http_obj: _ = v8::Object::new(scope);
    // Get
    let get_fn: _ = v8::FunctionTemplate::new(scope, get).get_function(scope).unwrap();
    let get_key: _ = v8::String::new(scope, "get").unwrap().into();
    http_obj.set(scope, get_key, get_fn.into());
    global.set(scope, http_key.into(), http_obj.into());
}
fn get(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue) {
    let url_arg: _ = args.get(0);
    let _url: _ = url_arg.to_string(scope).unwrap().to_rust_string_lossy(scope);
    // Simple async wrapper
    let result: _ = v8::Object::new(scope);
    let status_key: _ = v8::String::new(scope, "statusCode").unwrap();
    let status_val: _ = v8::Integer::new(scope, 200).into();
    result.set(scope, status_key.into(), status_val);
    let data_key: _ = v8::String::new(scope, "data").unwrap();
    let data_val: _ = v8::String::new(scope, "Mock response").unwrap().into();
    result.set(scope, data_key.into(), data_val);
    retval.set(result.into());
}
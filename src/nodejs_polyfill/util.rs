/// util polyfill
use rusty_v8 as v8;
use std::collections::{HashMap, BTreeMap};
pub fn register(scope: &mut v8::PinScope, global: &v8::Local<v8::Object>) {
    let util_key: _ = v8::String::new(scope, "util").unwrap();
    let util_obj: _ = v8::Object::new(scope);
    // Inspect
    let inspect_fn: _ = v8::FunctionTemplate::new(scope, inspect).get_function(scope).unwrap();
    let inspect_key: _ = v8::String::new(scope, "inspect").unwrap().into();
    util_obj.set(scope, inspect_key, inspect_fn.into());
    global.set(scope, util_key.into(), util_obj.into());
}
fn inspect(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue) {
    let obj: _ = args.get(0);
    let result: _ = obj.to_string(scope).unwrap().to_rust_string_lossy(scope);
    retval.set(v8::String::new(scope, &result).unwrap().into());
}
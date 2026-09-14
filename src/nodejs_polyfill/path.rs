/// path polyfill
use rusty_v8 as v8;
use std::collections::{HashMap, BTreeMap};
use std::path::Path;
pub fn register(scope: &mut v8::PinScope, global: &v8::Local<v8::Object>) {
    let path_key: _ = v8::String::new(scope, "path").unwrap();
    let path_obj: _ = v8::Object::new(scope);
    // Join paths
    let join_fn: _ = v8::FunctionTemplate::new(scope, join).get_function(scope).unwrap();
    let join_key: _ = v8::String::new(scope, "join").unwrap().into();
    path_obj.set(scope, join_key, join_fn.into());
    // Resolve
    let resolve_fn: _ = v8::FunctionTemplate::new(scope, resolve).get_function(scope).unwrap();
    let resolve_key: _ = v8::String::new(scope, "resolve").unwrap().into();
    path_obj.set(scope, resolve_key, resolve_fn.into());
    // Basename
    let basename_fn: _ = v8::FunctionTemplate::new(scope, basename).get_function(scope).unwrap();
    let basename_key: _ = v8::String::new(scope, "basename").unwrap().into();
    path_obj.set(scope, basename_key, basename_fn.into());
    global.set(scope, path_key.into(), path_obj.into());
}
fn join(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue) {
    let mut result = String::new();
    for i in 0..args.length() {
        let arg: _ = args.get(i).to_string(scope).unwrap().to_rust_string_lossy(scope);
        if i > 0 && !result.ends_with('/') && !arg.starts_with('/') {
            result.push('/');
        }
        result.push_str(&arg);
    }
    retval.set(v8::String::new(scope, &result).unwrap().into());
}
fn resolve(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue) {
    let mut paths = Vec::new();
    for i in 0..args.length() {
        paths.push(args.get(i).to_string(scope).unwrap().to_rust_string_lossy(scope));
    }
    let result: _ = std::path::Path::new(&paths.join("/"))
        .canonicalize()
        .unwrap_or_else(|_| std::path::Path::new(&paths.join("/")).to_path_buf());
    retval.set(v8::String::new(scope, &result.to_string_lossy()).unwrap().into());
}
fn basename(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue) {
    let path_arg: _ = args.get(0);
    let path_str: _ = path_arg.to_string(scope).unwrap().to_rust_string_lossy(scope);
    let path: _ = std::path::Path::new(&path_str);
    if let Some(file_name) = path.file_name() {
        retval.set(v8::String::new(scope, &file_name.to_string_lossy()).unwrap().into());
    } else {
        retval.set(v8::String::new(scope, "").unwrap().into());
    }
}
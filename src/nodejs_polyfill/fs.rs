/// fs (file system) polyfill
use rusty_v8 as v8;
use std::collections::{HashMap, BTreeMap};
use std::path::Path;
use std::io::Write;
use std::io::Read;
pub fn register(scope: &mut v8::PinScope, global: &v8::Local<v8::Object>) {
    let fs_key: _ = v8::String::new(scope, "fs").unwrap();
    let fs_obj: _ = v8::Object::new(scope);
    // Read file
    let read_file_fn: _ = v8::FunctionTemplate::new(scope, read_file).get_function(scope).unwrap();
    let read_file_key: _ = v8::String::new(scope, "readFile").unwrap().into();
    fs_obj.set(scope, read_file_key, read_file_fn.into());
    // Write file
    let write_file_fn: _ = v8::FunctionTemplate::new(scope, write_file).get_function(scope).unwrap();
    let write_file_key: _ = v8::String::new(scope, "writeFile").unwrap().into();
    fs_obj.set(scope, write_file_key, write_file_fn.into());
    // Exists
    let exists_fn: _ = v8::FunctionTemplate::new(scope, exists).get_function(scope).unwrap();
    let exists_key: _ = v8::String::new(scope, "existsSync").unwrap().into();
    fs_obj.set(scope, exists_key, exists_fn.into());
    global.set(scope, fs_key.into(), fs_obj.into());
}
fn read_file(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue) {
    let path_arg: _ = args.get(0);
    let path: _ = path_arg.to_string(scope).unwrap().to_rust_string_lossy(scope);
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let content_v8: _ = v8::String::new(scope, &content).unwrap();
            retval.set(content_v8.into());
        }
        Err(_) => {
            retval.set(v8::null(scope).into());
        }
    }
}
fn write_file(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue) {
    let path_arg: _ = args.get(0);
    let path: _ = path_arg.to_string(scope).unwrap().to_rust_string_lossy(scope);
    let content_arg: _ = args.get(1);
    let content: _ = content_arg.to_string(scope).unwrap().to_rust_string_lossy(scope);
    match std::fs::write(&path, content) {
        Ok(_) => {
            retval.set(v8::undefined(scope).into());
        }
        Err(_) => {
            retval.set(v8::undefined(scope).into());
        }
    }
}
fn exists(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue) {
    let path_arg: _ = args.get(0);
    let path: _ = path_arg.to_string(scope).unwrap().to_rust_string_lossy(scope);
    let exists: _ = std::path::Path::new(&path).exists();
    retval.set(v8::Boolean::new(scope, exists).into());
}
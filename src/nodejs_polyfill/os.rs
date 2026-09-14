/// os polyfill
use rusty_v8 as v8;
use std::collections::{HashMap, BTreeMap};
pub fn register(scope: &mut v8::PinScope, global: &v8::Local<v8::Object>) {
    let os_key: _ = v8::String::new(scope, "os").unwrap();
    let os_obj: _ = v8::Object::new(scope);
    // Platform
    let platform_fn: _ = v8::FunctionTemplate::new(scope, platform).get_function(scope).unwrap();
    let platform_key: _ = v8::String::new(scope, "platform").unwrap().into();
    os_obj.set(scope, platform_key, platform_fn.into());
    // Type
    let ostype_fn: _ = v8::FunctionTemplate::new(scope, ostype).get_function(scope).unwrap();
    let ostype_key: _ = v8::String::new(scope, "type").unwrap().into();
    os_obj.set(scope, ostype_key, ostype_fn.into());
    // Arch
    let arch_fn: _ = v8::FunctionTemplate::new(scope, arch).get_function(scope).unwrap();
    let arch_key: _ = v8::String::new(scope, "arch").unwrap().into();
    os_obj.set(scope, arch_key, arch_fn.into());
    global.set(scope, os_key.into(), os_obj.into());
}
fn platform(scope: &mut v8::PinScope, _args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue) {
    let platform = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "win32"
    } else {
        "unknown"
    };
    retval.set(v8::String::new(scope, platform).unwrap().into());
}
fn ostype(scope: &mut v8::PinScope, _args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue) {
    let ostype = if cfg!(target_os = "linux") {
        "Linux"
    } else if cfg!(target_os = "macos") {
        "Darwin"
    } else if cfg!(target_os = "windows") {
        "Windows_NT"
    } else {
        "Unknown"
    };
    retval.set(v8::String::new(scope, ostype).unwrap().into());
}
fn arch(scope: &mut v8::PinScope, _args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue) {
    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "unknown"
    };
    retval.set(v8::String::new(scope, arch).unwrap().into());
}
// Node.js Core Modules Polyfill
/// Stage 56.3 - Built-in Module Implementation
use rusty_v8 as v8;
use std::collections::{HashMap, BTreeMap};
pub mod fs;
pub mod path;
pub mod os;
pub mod crypto;
pub mod http;
pub mod url;
pub mod querystring;
pub mod util;
/// Register all built-in modules with the V8 context
pub fn register_builtins<'a>(scope: &mut v8::PinScope<'a, '_>) -> v8::Local<'a, v8::Object> {
    let global: _ = v8::Object::new(scope);
    
    // Register each module
    fs::register(scope, &global);
    path::register(scope, &global);
    os::register(scope, &global);
    crypto::register(scope, &global);
    http::register(scope, &global);
    url::register(scope, &global);
    querystring::register(scope, &global);
    util::register(scope, &global);
    
    global
}
/// Check if a module name is a built-in Node.js module
pub fn is_builtin_module(name: &str) -> bool {
    matches!(
        name,
        "assert" | "buffer" | "child_process" | "cluster" | "crypto" | "dns" |
        "domain" | "events" | "fs" | "http" | "https" | "net" | "os" |
        "path" | "querystring" | "readline" | "repl" | "stream" | "string_decoder" |
        "timers" | "tls" | "tty" | "url" | "util" | "v8" | "vm" | "wasi" |
        "worker_threads" | "zlib"
    )
}
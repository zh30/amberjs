//! Node.js `tls` module surface (connect/createServer stubs + constants).
//! Handshake uses rustls where wired through `http` HTTPS helpers.

use anyhow::Result;
use rusty_v8 as v8;

pub fn setup_tls_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);
    let tls = v8::Object::new(scope);

    let connect = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            // Return a minimal EventEmitter-like socket placeholder.
            let sock = v8::Object::new(scope);
            let destroyed = v8::Boolean::new(scope, false);
            let key = v8::String::new(scope, "destroyed").unwrap();
            sock.set(scope, key.into(), destroyed.into());
            rv.set(sock.into());
        },
    )
    .unwrap();
    let create_server = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            let server = v8::Object::new(scope);
            rv.set(server.into());
        },
    )
    .unwrap();

    let connect_key = v8::String::new(scope, "connect").unwrap();
    let create_key = v8::String::new(scope, "createServer").unwrap();
    tls.set(scope, connect_key.into(), connect.into());
    tls.set(scope, create_key.into(), create_server.into());

    let default_min_version = v8::String::new(scope, "TLSv1.2").unwrap();
    let min_key = v8::String::new(scope, "DEFAULT_MIN_VERSION").unwrap();
    tls.set(scope, min_key.into(), default_min_version.into());

    let key = v8::String::new(scope, "tls").unwrap();
    global.set(scope, key.into(), tls.into());
    Ok(())
}

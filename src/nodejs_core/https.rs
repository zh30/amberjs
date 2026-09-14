//! Node.js `https` module — HTTP server/client with rustls when `key`/`cert` are set.

use anyhow::Result;
use rusty_v8 as v8;

use super::http::{attach_tls_options_to_server, build_http_server_object};

pub fn setup_https_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);
    let https = v8::Object::new(scope);

    let http_key = v8::String::new(scope, "http").unwrap();
    if let Some(http_val) = global.get(scope, http_key.into()) {
        if let Ok(http_obj) = v8::Local::<v8::Object>::try_from(http_val) {
            for name in ["request", "get", "Agent", "globalAgent"] {
                let key = v8::String::new(scope, name).unwrap();
                if let Some(val) = http_obj.get(scope, key.into()) {
                    https.set(scope, key.into(), val);
                }
            }
        }
    }

    let create_server = v8::FunctionTemplate::new(scope, https_create_server_callback);
    let create_server_fn = create_server.get_function(scope).unwrap();
    let create_server_key = v8::String::new(scope, "createServer").unwrap();
    https.set(scope, create_server_key.into(), create_server_fn.into());

    let tls_flag = v8::Boolean::new(scope, true);
    let tls_key = v8::String::new(scope, "_beeUsesTls").unwrap();
    https.set(scope, tls_key.into(), tls_flag.into());

    let key = v8::String::new(scope, "https").unwrap();
    global.set(scope, key.into(), https.into());
    Ok(())
}

fn https_create_server_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let (options, handler) = if args.get(0).is_function() {
        (v8::undefined(scope).into(), args.get(0))
    } else {
        (args.get(0), args.get(1))
    };

    let server_obj = build_http_server_object(scope);
    if handler.is_function() {
        let handler_key = v8::String::new(scope, "_requestHandler").unwrap();
        server_obj.set(scope, handler_key.into(), handler);
        let context = scope.get_current_context();
        let global = context.global(scope);
        let global_handler_key = v8::String::new(scope, "_httpServerRequestHandler").unwrap();
        global.set(scope, global_handler_key.into(), handler);
    }
    attach_tls_options_to_server(scope, server_obj, options);
    let tls_flag = v8::Boolean::new(scope, true);
    let tls_key = v8::String::new(scope, "_beeUsesTls").unwrap();
    server_obj.set(scope, tls_key.into(), tls_flag.into());
    retval.set(server_obj.into());
}

//! Node.js `vm` module — Script.runInThisContext / runInNewContext subset.

use anyhow::Result;
use rusty_v8 as v8;

pub fn setup_vm_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);
    let vm = v8::Object::new(scope);

    let run_in_this = v8::Function::new(scope, run_in_this_context).unwrap();
    let run_in_new = v8::Function::new(scope, run_in_new_context).unwrap();
    let create_context = v8::Function::new(scope, create_context_cb).unwrap();
    let is_context = v8::Function::new(scope, is_context_cb).unwrap();
    let script_ctor = v8::Function::new(scope, script_constructor).unwrap();

    for (name, func) in [
        ("runInThisContext", run_in_this),
        ("runInNewContext", run_in_new),
        ("createContext", create_context),
        ("isContext", is_context),
        ("Script", script_ctor),
    ] {
        let key = v8::String::new(scope, name).unwrap();
        vm.set(scope, key.into(), func.into());
    }

    let key = v8::String::new(scope, "vm").unwrap();
    global.set(scope, key.into(), vm.into());
    Ok(())
}

fn compile_and_run(scope: &mut v8::PinScope, code: &str, rv: &mut v8::ReturnValue) {
    let source = v8::String::new(scope, code).unwrap();
    let script = match v8::Script::compile(scope, source, None) {
        Some(s) => s,
        None => return,
    };
    if let Some(result) = script.run(scope) {
        rv.set(result);
    }
}

fn run_in_this_context(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let code = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    compile_and_run(scope, &code, &mut rv);
}

fn run_in_new_context(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    rv: v8::ReturnValue,
) {
    // Same isolate/context for now; sandbox isolation is a follow-up.
    run_in_this_context(scope, args, rv);
}

fn create_context_cb(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    if args.length() > 0 && args.get(0).is_object() {
        rv.set(args.get(0));
    } else {
        rv.set(v8::Object::new(scope).into());
    }
}

fn is_context_cb(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    rv.set(v8::Boolean::new(scope, args.get(0).is_object()).into());
}

fn script_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let obj = v8::Object::new(scope);
    let code = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let code_key = v8::String::new(scope, "_code").unwrap();
    let code_val = v8::String::new(scope, &code).unwrap();
    obj.set(scope, code_key.into(), code_val.into());

    let run = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let this = args.this();
            let code_key = v8::String::new(scope, "_code").unwrap();
            let code = this
                .get(scope, code_key.into())
                .and_then(|v| v.to_string(scope))
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();
            compile_and_run(scope, &code, &mut rv);
        },
    )
    .unwrap();
    let run_key = v8::String::new(scope, "runInThisContext").unwrap();
    obj.set(scope, run_key.into(), run.into());
    rv.set(obj.into());
}

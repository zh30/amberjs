//! Node.js `vm` module — Script.runInThisContext / runInNewContext subset.

use std::collections::HashSet;

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

/// Collect own property names + values from a sandbox object (caller context).
fn collect_sandbox_entries(
    scope: &mut v8::PinScope,
    sandbox: v8::Local<v8::Object>,
) -> Vec<(String, v8::Global<v8::Value>)> {
    let mut entries = Vec::new();
    let Some(names) = sandbox.get_own_property_names(scope, Default::default()) else {
        return entries;
    };
    for i in 0..names.length() {
        let Some(key) = names.get_index(scope, i) else {
            continue;
        };
        let key_str = key
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_default();
        if let Some(val) = sandbox.get(scope, key.into()) {
            entries.push((key_str, v8::Global::new(scope, val)));
        }
    }
    entries
}

fn own_property_names(scope: &mut v8::PinScope, obj: v8::Local<v8::Object>) -> HashSet<String> {
    let mut names = HashSet::new();
    let Some(keys) = obj.get_own_property_names(scope, Default::default()) else {
        return names;
    };
    for i in 0..keys.length() {
        if let Some(key) = keys.get_index(scope, i) {
            let key_str = key
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();
            names.insert(key_str);
        }
    }
    names
}

/// Run `code` in a fresh V8 Context. Optional sandbox own-props are copied onto
/// the new global before execution and copied back afterward (sandbox keys plus
/// newly introduced own globals; fresh-context builtins are left alone).
fn run_code_in_new_context(
    scope: &mut v8::PinScope,
    code: &str,
    sandbox: Option<v8::Local<v8::Object>>,
    rv: &mut v8::ReturnValue,
) {
    let entries = sandbox
        .map(|obj| collect_sandbox_entries(scope, obj))
        .unwrap_or_default();
    let sandbox_keys: HashSet<String> = entries.iter().map(|(k, _)| k.clone()).collect();

    let new_context = v8::Context::new(scope, Default::default());
    let result_global = {
        let scope = &mut v8::ContextScope::new(scope, new_context);
        let global = new_context.global(scope);
        let builtins = own_property_names(scope, global);

        for (key_str, val_global) in &entries {
            let key = v8::String::new(scope, key_str).unwrap();
            let val = v8::Local::new(scope, val_global);
            global.set(scope, key.into(), val);
        }

        let source = v8::String::new(scope, code).unwrap();
        let script = match v8::Script::compile(scope, source, None) {
            Some(s) => s,
            None => return,
        };
        let Some(result) = script.run(scope) else {
            return;
        };

        if let Some(sandbox_obj) = sandbox {
            if let Some(names) = global.get_own_property_names(scope, Default::default()) {
                for i in 0..names.length() {
                    let Some(key) = names.get_index(scope, i) else {
                        continue;
                    };
                    let key_str = key
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    if !(sandbox_keys.contains(&key_str) || !builtins.contains(&key_str)) {
                        continue;
                    }
                    if let Some(val) = global.get(scope, key.into()) {
                        let sk = v8::String::new(scope, &key_str).unwrap();
                        sandbox_obj.set(scope, sk.into(), val);
                    }
                }
            }
        }

        v8::Global::new(scope, result)
    };

    let local = v8::Local::new(scope, &result_global);
    rv.set(local);
}

fn run_in_new_context(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let code = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let sandbox = if args.length() > 1 && args.get(1).is_object() {
        args.get(1).to_object(scope)
    } else {
        None
    };
    run_code_in_new_context(scope, &code, sandbox, &mut rv);
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

    let run_this = v8::Function::new(
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
    let run_this_key = v8::String::new(scope, "runInThisContext").unwrap();
    obj.set(scope, run_this_key.into(), run_this.into());

    let run_new = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let this = args.this();
            let code_key = v8::String::new(scope, "_code").unwrap();
            let code = this
                .get(scope, code_key.into())
                .and_then(|v| v.to_string(scope))
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();
            let sandbox = if args.length() > 0 && args.get(0).is_object() {
                args.get(0).to_object(scope)
            } else {
                None
            };
            run_code_in_new_context(scope, &code, sandbox, &mut rv);
        },
    )
    .unwrap();
    let run_new_key = v8::String::new(scope, "runInNewContext").unwrap();
    obj.set(scope, run_new_key.into(), run_new.into());

    rv.set(obj.into());
}

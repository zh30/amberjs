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

    let run_in_ctx = v8::Function::new(scope, run_in_new_context).unwrap();

    for (name, func) in [
        ("runInThisContext", run_in_this),
        ("runInNewContext", run_in_new),
        ("runInContext", run_in_ctx),
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

fn throw_caller_error(scope: &mut v8::PinScope, name: &str, message: &str) {
    let msg = v8::String::new(scope, message).unwrap();
    let exc = match name {
        "SyntaxError" => v8::Exception::syntax_error(scope, msg),
        "TypeError" => v8::Exception::type_error(scope, msg),
        "RangeError" => v8::Exception::range_error(scope, msg),
        "ReferenceError" => v8::Exception::reference_error(scope, msg),
        _ => v8::Exception::error(scope, msg),
    };
    scope.throw_exception(exc);
}

/// Run `code` in a fresh V8 Context. Optional sandbox own-props are copied onto
/// the new global before execution and copied back afterward (sandbox keys plus
/// newly introduced own globals; fresh-context builtins are left alone).
///
/// Exceptions are caught and rethrown as caller-context `Error`/`SyntaxError`/…
/// so `instanceof Error` works outside the sandbox (cross-context prototypes differ).
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
    let mut caught: Option<(String, String)> = None;
    let result_global = {
        let scope = &mut v8::ContextScope::new(scope, new_context);
        let global = new_context.global(scope);
        let builtins = own_property_names(scope, global);

        for (key_str, val_global) in &entries {
            let key = v8::String::new(scope, key_str).unwrap();
            let val = v8::Local::new(scope, val_global);
            global.set(scope, key.into(), val);
        }

        v8::tc_scope!(let tc, scope);
        let source = v8::String::new(tc, code).unwrap();
        let script = v8::Script::compile(tc, source, None);
        let result = script.and_then(|s| s.run(tc));

        if let Some(result) = result {
            if let Some(sandbox_obj) = sandbox {
                if let Some(names) = global.get_own_property_names(tc, Default::default()) {
                    for i in 0..names.length() {
                        let Some(key) = names.get_index(tc, i) else {
                            continue;
                        };
                        let key_str = key
                            .to_string(tc)
                            .map(|s| s.to_rust_string_lossy(tc))
                            .unwrap_or_default();
                        if !(sandbox_keys.contains(&key_str) || !builtins.contains(&key_str)) {
                            continue;
                        }
                        if let Some(val) = global.get(tc, key.into()) {
                            let sk = v8::String::new(tc, &key_str).unwrap();
                            sandbox_obj.set(tc, sk.into(), val);
                        }
                    }
                }
            }
            Some(v8::Global::new(tc, result))
        } else {
            if let Some(exc) = tc.exception() {
                let obj = exc.to_object(tc);
                let name = obj
                    .and_then(|o| {
                        let key = v8::String::new(tc, "name")?;
                        o.get(tc, key.into())
                    })
                    .and_then(|v| v.to_string(tc))
                    .map(|s| s.to_rust_string_lossy(tc))
                    .unwrap_or_else(|| "Error".to_string());
                let message = obj
                    .and_then(|o| {
                        let key = v8::String::new(tc, "message")?;
                        o.get(tc, key.into())
                    })
                    .and_then(|v| v.to_string(tc))
                    .map(|s| s.to_rust_string_lossy(tc))
                    .or_else(|| exc.to_string(tc).map(|s| s.to_rust_string_lossy(tc)))
                    .unwrap_or_else(|| "Error".to_string());
                caught = Some((name, message));
            } else {
                caught = Some(("Error".to_string(), "vm.runInNewContext failed".to_string()));
            }
            None
        }
    };

    if let Some((name, message)) = caught {
        throw_caller_error(scope, &name, &message);
        return;
    }

    if let Some(result_global) = result_global {
        let local = v8::Local::new(scope, &result_global);
        rv.set(local);
    }
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

/// G73：createContext 把 `_vmContext` 标记打到传入的 sandbox 上；
/// isContext 只认标记——此前对任意对象都 true 是 dishonest。
fn create_context_cb(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let ctx_obj = if args.length() > 0 && args.get(0).is_object() {
        args.get(0).to_object(scope).unwrap()
    } else {
        v8::Object::new(scope)
    };
    let mark_key = v8::String::new(scope, "_vmContext").unwrap();
    let mark_val = v8::Boolean::new(scope, true);
    // 冻结/不可扩展对象打不上标记——fail-closed 抛 TypeError，
    // 否则返回的对象 isContext 为 false，语义自相矛盾。
    // `set` 走 [[Set]] 语义，冻结时会静默不写却报成功，必须用
    // `create_data_property`（[[DefineOwnProperty]]）才能拿到真实结果。
    match ctx_obj.create_data_property(scope, mark_key.into(), mark_val.into()) {
        Some(true) => rv.set(ctx_obj.into()),
        _ => throw_caller_error(
            scope,
            "TypeError",
            "vm.createContext: cannot contextify a frozen or non-extensible object",
        ),
    }
}

fn is_context_cb(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // 只认 own property —— 原型链上继承的 `_vmContext` 不算标记。
    let marked = args
        .get(0)
        .to_object(scope)
        .map(|obj| {
            let mark_key = v8::String::new(scope, "_vmContext").unwrap();
            let own = obj
                .has_own_property(scope, mark_key.into())
                .unwrap_or(false);
            own && obj
                .get(scope, mark_key.into())
                .map(|v| v.is_true())
                .unwrap_or(false)
        })
        .unwrap_or(false);
    rv.set(v8::Boolean::new(scope, marked).into());
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

    // G73: script.runInContext(ctxObj)——对 contextified sandbox 执行，
    // 窄契约里与 runInNewContext 同一实现（sandbox 语义见契约 Limits）
    let run_ctx_key = v8::String::new(scope, "runInContext").unwrap();
    obj.set(scope, run_ctx_key.into(), run_new.into());

    rv.set(obj.into());
}

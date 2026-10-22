//! Minimal `__amber_createRealm` host for test262 cross-realm tests.
//!
//! Enabled when `AMBER_TEST262=1` (the harness sets this). Creates a persistent
//! V8 Context per realm so builtins (`Number`, …) and `eval` honor that realm.
//!
//! Note on `%ThrowTypeError%` / `Function.prototype.{arguments,caller}`:
//! V8 152 exposes distinct named prototype accessors (legacy `.caller` tracking)
//! instead of the single per-realm `%ThrowTypeError%` Node still ships. Rebinding
//! the prototype to the real `%ThrowTypeError%` intrinsic restores accessor
//! identity but regresses `features: [caller]` poison-pill tests (no own
//! `caller` data properties on function instances in this V8). Leave those
//! FAILs honest until an engine-level fix can reinstall own restricted props.

use std::cell::RefCell;

use anyhow::Result;
use rusty_v8 as v8;

thread_local! {
    static REALMS: RefCell<Vec<v8::Global<v8::Context>>> = const { RefCell::new(Vec::new()) };
}

pub fn test262_host_enabled() -> bool {
    matches!(
        std::env::var("AMBER_TEST262").ok().as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

pub fn setup_test262_host(
    scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    if !test262_host_enabled() {
        return Ok(());
    }

    let global = context.global(scope);
    let create = v8::Function::new(scope, create_realm_cb).unwrap();
    let key = v8::String::new(scope, "__amber_createRealm").unwrap();
    global.set(scope, key.into(), create.into());

    // Host `$262.detachArrayBuffer` — V8 ArrayBuffer::detach (test262 INTERPRETING.md).
    let detach = v8::Function::new(scope, detach_array_buffer_cb).unwrap();
    let d_key = v8::String::new(scope, "__amber_detachArrayBuffer").unwrap();
    global.set(scope, d_key.into(), detach.into());

    // Concurrent `$262.agent` host for Atomics.wait / notify tests.
    crate::test262_agent::setup_agent_host(scope, context);

    Ok(())
}

fn create_realm_cb(
    scope: &mut v8::PinScope<'_, '_>,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    match create_realm_record(scope) {
        Ok(obj) => rv.set(obj.into()),
        Err(msg) => {
            let s = v8::String::new(scope, &msg).unwrap();
            let exc = v8::Exception::error(scope, s);
            scope.throw_exception(exc);
        }
    }
}

fn create_realm_record<'s>(
    scope: &mut v8::PinScope<'s, '_>,
) -> Result<v8::Local<'s, v8::Object>, String> {
    let new_context = v8::Context::new(scope, Default::default());
    // Same security token as the caller context so cross-realm property
    // access (other.Number, other.eval, …) is allowed within the isolate.
    let token = scope.get_current_context().get_security_token(scope);
    new_context.set_security_token(token);
    let realm_id = REALMS.with(|realms| {
        let mut realms = realms.borrow_mut();
        let id = realms.len();
        realms.push(v8::Global::new(scope, new_context));
        id
    });

    // Enter the new realm: stamp id + replace eval with realm-aware callback.
    // Also install await-dictionary on this realm's Promise (bare V8 context).
    {
        let scope = &mut v8::ContextScope::new(scope, new_context);
        let global = new_context.global(scope);

        let id_key = v8::String::new(scope, "__amberRealmId").unwrap();
        let id_val = v8::Number::new(scope, realm_id as f64);
        global.set(scope, id_key.into(), id_val.into());

        let eval_fn = v8::Function::new(scope, realm_eval_cb).unwrap();
        let eval_key = v8::String::new(scope, "eval").unwrap();
        global.set(scope, eval_key.into(), eval_fn.into());

        crate::promise_await_dictionary::install(scope).map_err(|e| e.to_string())?;
    }

    let global_local = {
        let ctx = REALMS.with(|r| r.borrow()[realm_id].clone());
        let ctx_local = v8::Local::new(scope, &ctx);
        let g = {
            let scope = &mut v8::ContextScope::new(scope, ctx_local);
            let g = ctx_local.global(scope);
            v8::Global::new(scope, g)
        };
        v8::Local::new(scope, &g)
    };

    let record = v8::Object::new(scope);
    let id_key = v8::String::new(scope, "__amberRealmId").unwrap();
    let id_val = v8::Number::new(scope, realm_id as f64);
    record.set(scope, id_key.into(), id_val.into());

    let global_key = v8::String::new(scope, "global").unwrap();
    record.set(scope, global_key.into(), global_local.into());

    let eval_script = v8::Function::new(scope, realm_eval_script_cb).unwrap();
    let es_key = v8::String::new(scope, "evalScript").unwrap();
    record.set(scope, es_key.into(), eval_script.into());

    let create = v8::Function::new(scope, create_realm_cb).unwrap();
    let cr_key = v8::String::new(scope, "createRealm").unwrap();
    record.set(scope, cr_key.into(), create.into());

    let detach = v8::Function::new(scope, detach_array_buffer_cb).unwrap();
    let d_key = v8::String::new(scope, "detachArrayBuffer").unwrap();
    record.set(scope, d_key.into(), detach.into());

    let gc = v8::Function::new(scope, gc_noop).unwrap();
    let g_key = v8::String::new(scope, "gc").unwrap();
    record.set(scope, g_key.into(), gc.into());

    // INTERPRETING.md: createRealm defines the $262 API on the new realm's
    // global and returns that property. staging/sm TypedArray cases call
    // otherGlobal.$262.detachArrayBuffer — without this, $262 is undefined.
    {
        let dollar_key = v8::String::new(scope, "$262").unwrap();
        global_local.set(scope, dollar_key.into(), record.into());
    }

    Ok(record)
}

fn realm_id_from_value(
    scope: &mut v8::PinScope<'_, '_>,
    val: v8::Local<v8::Value>,
) -> Option<usize> {
    let obj = val.to_object(scope)?;
    let key = v8::String::new(scope, "__amberRealmId")?;
    let id_val = obj.get(scope, key.into())?;
    if id_val.is_number() {
        return Some(id_val.number_value(scope)? as usize);
    }
    None
}

fn run_in_realm<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    realm_id: usize,
    code: &str,
) -> Result<v8::Local<'s, v8::Value>, String> {
    let ctx_global = REALMS.with(|r| {
        r.borrow()
            .get(realm_id)
            .cloned()
            .ok_or_else(|| format!("invalid realm id {realm_id}"))
    })?;
    let ctx = v8::Local::new(scope, &ctx_global);
    let result_global = {
        let scope = &mut v8::ContextScope::new(scope, ctx);
        v8::tc_scope!(let tc, scope);
        let source = v8::String::new(tc, code).ok_or_else(|| "oom".to_string())?;
        let script = v8::Script::compile(tc, source, None)
            .ok_or_else(|| "SyntaxError: failed to compile realm script".to_string())?;
        match script.run(tc) {
            Some(v) => Ok(v8::Global::new(tc, v)),
            None => {
                let msg = tc
                    .exception()
                    .and_then(|e| e.to_string(tc))
                    .map(|s| s.to_rust_string_lossy(tc))
                    .unwrap_or_else(|| "Error in realm eval".to_string());
                Err(msg)
            }
        }
    }?;
    Ok(v8::Local::new(scope, &result_global))
}

fn realm_eval_cb(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let code = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let Some(realm_id) = realm_id_from_value(scope, args.this().into()) else {
        let s = v8::String::new(scope, "$262 realm eval: missing __amberRealmId").unwrap();
        let exc = v8::Exception::error(scope, s);
        scope.throw_exception(exc);
        return;
    };
    match run_in_realm(scope, realm_id, &code) {
        Ok(v) => rv.set(v),
        Err(msg) => {
            let s = v8::String::new(scope, &msg).unwrap();
            let exc = v8::Exception::error(scope, s);
            scope.throw_exception(exc);
        }
    }
}

fn realm_eval_script_cb(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let code = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    // `record.evalScript(code)` — id lives on the record (`this`).
    let Some(realm_id) = realm_id_from_value(scope, args.this().into()) else {
        let s = v8::String::new(scope, "evalScript: missing realm id").unwrap();
        let exc = v8::Exception::error(scope, s);
        scope.throw_exception(exc);
        return;
    };
    match run_in_realm(scope, realm_id, &code) {
        Ok(v) => rv.set(v),
        Err(msg) => {
            let s = v8::String::new(scope, &msg).unwrap();
            let exc = v8::Exception::error(scope, s);
            scope.throw_exception(exc);
        }
    }
}

/// `$262.detachArrayBuffer(arrayBuffer [, key])` — detach via V8.
///
/// Matches test262 host API: TypeError if not an ArrayBuffer; optional
/// `key` forwarded to `ArrayBuffer::detach` when provided.
fn detach_array_buffer_cb(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let buffer_val = args.get(0);
    if !buffer_val.is_array_buffer() {
        let s = v8::String::new(
            scope,
            "$262.detachArrayBuffer called with an argument that is not an ArrayBuffer",
        )
        .unwrap();
        let exc = v8::Exception::type_error(scope, s);
        scope.throw_exception(exc);
        return;
    }

    let Ok(buffer) = v8::Local::<v8::ArrayBuffer>::try_from(buffer_val) else {
        let s = v8::String::new(scope, "$262.detachArrayBuffer: invalid ArrayBuffer").unwrap();
        let exc = v8::Exception::type_error(scope, s);
        scope.throw_exception(exc);
        return;
    };

    // Optional detach key (arg 1). Do not treat missing/undefined as a key.
    let key = if args.length() >= 2 {
        let k = args.get(1);
        if k.is_undefined() || k.is_null() {
            None
        } else {
            Some(k)
        }
    } else {
        None
    };

    // Always call detach — a zero-length buffer may still be attached.
    let _ok = buffer.detach(key);
    rv.set(v8::undefined(scope).into());
}

fn gc_noop(
    _scope: &mut v8::PinScope<'_, '_>,
    _args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
}

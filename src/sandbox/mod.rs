//! Agent deterministic sandboxing and in-memory virtual filesystem

pub mod virtual_fs;

pub use virtual_fs::{
    disable, enable, export_snapshot, is_cow_enabled, is_enabled, list_virtual_files, reset,
    vfs_create_dir, vfs_create_dir_all, vfs_exists, vfs_metadata, vfs_read, vfs_read_dir,
    vfs_read_to_string, vfs_remove_dir_all, vfs_remove_file, vfs_write, VfsMetadata,
};

use anyhow::Result;
use rusty_v8 as v8;

fn vfs_is_enabled_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let val = is_enabled();
    retval.set(v8::Boolean::new(scope, val).into());
}

fn vfs_is_cow_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let val = is_cow_enabled();
    retval.set(v8::Boolean::new(scope, val).into());
}

fn vfs_enable_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let cow = if args.length() > 0 {
        args.get(0).to_boolean(scope).boolean_value(scope)
    } else {
        true
    };
    enable(cow);
    retval.set(v8::undefined(scope).into());
}

fn vfs_disable_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    disable();
    retval.set(v8::undefined(scope).into());
}

fn vfs_reset_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    reset();
    retval.set(v8::undefined(scope).into());
}

fn vfs_list_files_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let files = list_virtual_files();
    let arr = v8::Array::new(scope, files.len() as i32);
    for (i, file) in files.iter().enumerate() {
        let s = v8::String::new(scope, file).unwrap();
        arr.set_index(scope, i as u32, s.into());
    }
    retval.set(arr.into());
}

fn vfs_snapshot_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let snap = export_snapshot();
    let json_str = snap.to_string();
    let v8_str = v8::String::new(scope, &json_str).unwrap();
    if let Some(parsed) = v8::json::parse(scope, v8_str) {
        retval.set(parsed);
    } else {
        retval.set(v8::undefined(scope).into());
    }
}

pub fn setup_sandbox_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);
    let vfs_obj = v8::Object::new(scope);

    let is_enabled_fn = v8::Function::new(scope, vfs_is_enabled_callback).unwrap();
    let is_enabled_key = v8::String::new(scope, "isEnabled").unwrap();
    vfs_obj.set(scope, is_enabled_key.into(), is_enabled_fn.into());

    let is_cow_fn = v8::Function::new(scope, vfs_is_cow_callback).unwrap();
    let is_cow_key = v8::String::new(scope, "isCow").unwrap();
    vfs_obj.set(scope, is_cow_key.into(), is_cow_fn.into());

    let enable_fn = v8::Function::new(scope, vfs_enable_callback).unwrap();
    let enable_key = v8::String::new(scope, "enable").unwrap();
    vfs_obj.set(scope, enable_key.into(), enable_fn.into());

    let disable_fn = v8::Function::new(scope, vfs_disable_callback).unwrap();
    let disable_key = v8::String::new(scope, "disable").unwrap();
    vfs_obj.set(scope, disable_key.into(), disable_fn.into());

    let reset_fn = v8::Function::new(scope, vfs_reset_callback).unwrap();
    let reset_key = v8::String::new(scope, "reset").unwrap();
    vfs_obj.set(scope, reset_key.into(), reset_fn.into());

    let list_fn = v8::Function::new(scope, vfs_list_files_callback).unwrap();
    let list_key = v8::String::new(scope, "listFiles").unwrap();
    vfs_obj.set(scope, list_key.into(), list_fn.into());

    let snap_fn = v8::Function::new(scope, vfs_snapshot_callback).unwrap();
    let snap_key = v8::String::new(scope, "snapshot").unwrap();
    vfs_obj.set(scope, snap_key.into(), snap_fn.into());

    // Audit Log API
    let start_audit_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            if args.length() < 1 || !args.get(0).is_string() {
                let msg =
                    v8::String::new(scope, "startAuditLog requires a file path string").unwrap();
                let exc = v8::Exception::type_error(scope, msg);
                scope.throw_exception(exc);
                return;
            }
            let path_str = args.get(0).to_rust_string_lossy(scope);
            match crate::permissions::set_audit_log_path(Some(std::path::PathBuf::from(path_str))) {
                Ok(_) => rv.set(v8::Boolean::new(scope, true).into()),
                Err(e) => {
                    let msg = v8::String::new(scope, &e).unwrap();
                    let exc = v8::Exception::error(scope, msg);
                    scope.throw_exception(exc);
                }
            }
        },
    )
    .unwrap();
    let start_audit_key = v8::String::new(scope, "startAuditLog").unwrap();
    vfs_obj.set(scope, start_audit_key.into(), start_audit_fn.into());

    let stop_audit_fn = v8::Function::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            let _ = crate::permissions::set_audit_log_path(None);
            rv.set(v8::Boolean::new(_scope, true).into());
        },
    )
    .unwrap();
    let stop_audit_key = v8::String::new(scope, "stopAuditLog").unwrap();
    vfs_obj.set(scope, stop_audit_key.into(), stop_audit_fn.into());

    let get_audit_path_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            if let Some(p) = crate::permissions::get_audit_log_path() {
                let s = v8::String::new(scope, &p.to_string_lossy()).unwrap();
                rv.set(s.into());
            } else {
                rv.set(v8::null(scope).into());
            }
        },
    )
    .unwrap();
    let get_audit_path_key = v8::String::new(scope, "getAuditLogPath").unwrap();
    vfs_obj.set(scope, get_audit_path_key.into(), get_audit_path_fn.into());

    // Register on globalThis:
    let bee_vfs_key = v8::String::new(scope, "__bee_vfs").unwrap();
    global.set(scope, bee_vfs_key.into(), vfs_obj.into());

    let vfs_key = v8::String::new(scope, "vfs").unwrap();
    global.set(scope, vfs_key.into(), vfs_obj.into());

    let bee_vfs_mod_key = v8::String::new(scope, "bee:vfs").unwrap();
    global.set(scope, bee_vfs_mod_key.into(), vfs_obj.into());

    let bee_sandbox_mod_key = v8::String::new(scope, "bee:sandbox").unwrap();
    global.set(scope, bee_sandbox_mod_key.into(), vfs_obj.into());

    let bee_sandbox_key = v8::String::new(scope, "__bee_sandbox").unwrap();
    global.set(scope, bee_sandbox_key.into(), vfs_obj.into());

    let sandbox_key = v8::String::new(scope, "sandbox").unwrap();
    global.set(scope, sandbox_key.into(), vfs_obj.into());

    // Inject createEnclave via JS helper
    let enclave_helper_js = r#"
    (function() {
        const sb = globalThis.__bee_sandbox;
        if (!sb) return;

        sb.createEnclave = function(policyOrCode, options = {}) {
            function executeIsolated(codeOrFn, opts = {}) {
                const isFn = typeof codeOrFn === 'function';
                const src = isFn ? `(${codeOrFn.toString()})()` : String(codeOrFn);
                const safeContext = {
                    console, Math, JSON, Date, Array, Object, String, Number, Boolean, Promise,
                    RegExp, Map, Set, WeakMap, WeakSet, Error, TypeError, RangeError,
                    parseInt, parseFloat, isNaN, isFinite
                };
                if (opts.context && typeof opts.context === 'object') {
                    Object.assign(safeContext, opts.context);
                }
                const keys = Object.keys(safeContext);
                const vals = Object.values(safeContext);
                let runner;
                try {
                    runner = new Function(...keys, `"use strict"; return (${src});`);
                } catch {
                    runner = new Function(...keys, `"use strict"; ${src};`);
                }
                return runner(...vals);
            }

            if (policyOrCode && typeof policyOrCode === 'object' && !Array.isArray(policyOrCode)) {
                return {
                    policy: policyOrCode,
                    run: (codeOrFn, runOpts = {}) => executeIsolated(codeOrFn, Object.assign({}, policyOrCode, runOpts))
                };
            }

            if (typeof policyOrCode === 'string' || typeof policyOrCode === 'function') {
                return executeIsolated(policyOrCode, options);
            }

            return {
                policy: {},
                run: (codeOrFn, runOpts = {}) => executeIsolated(codeOrFn, runOpts)
            };
        };
    })();
    "#;
    if let Some(code) = v8::String::new(scope, enclave_helper_js) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }

    Ok(())
}

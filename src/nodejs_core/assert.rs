//! Node.js `assert` / `assert/strict` builtin (subset).

use anyhow::Result;
use rusty_v8 as v8;

pub fn setup_assert_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);
    let assert_obj = v8::Object::new(scope);

    let ok_fn = v8::Function::new(scope, assert_ok).unwrap();
    let equal_fn = v8::Function::new(scope, assert_equal).unwrap();
    let strict_equal_fn = v8::Function::new(scope, assert_strict_equal).unwrap();
    let deep_fn = v8::Function::new(scope, assert_deep_strict_equal).unwrap();
    let throws_fn = v8::Function::new(scope, assert_throws).unwrap();
    let fail_fn = v8::Function::new(scope, assert_fail).unwrap();
    let if_error_fn = v8::Function::new(scope, assert_if_error).unwrap();

    for (name, func) in [
        ("ok", ok_fn),
        ("equal", equal_fn),
        ("strictEqual", strict_equal_fn),
        ("deepEqual", deep_fn),
        ("deepStrictEqual", deep_fn),
        ("throws", throws_fn),
        ("fail", fail_fn),
        ("ifError", if_error_fn),
        ("assert", ok_fn),
    ] {
        let key = v8::String::new(scope, name).unwrap();
        assert_obj.set(scope, key.into(), func.into());
    }

    // Callable assert(value, message)
    let assert_callable = v8::Function::new(scope, assert_ok).unwrap();
    copy_props(scope, assert_obj, assert_callable);

    let assert_key = v8::String::new(scope, "assert").unwrap();
    global.set(scope, assert_key.into(), assert_callable.into());

    // assert/strict is the same object for now
    let strict_key = v8::String::new(scope, "assert_strict").unwrap();
    global.set(scope, strict_key.into(), assert_callable.into());

    Ok(())
}

fn copy_props(scope: &mut v8::PinScope, from: v8::Local<v8::Object>, to: v8::Local<v8::Function>) {
    let names = from
        .get_own_property_names(scope, Default::default())
        .unwrap();
    for i in 0..names.length() {
        if let Some(key) = names.get_index(scope, i) {
            if let Some(val) = from.get(scope, key) {
                to.set(scope, key, val);
            }
        }
    }
}

fn throw_assertion(scope: &mut v8::PinScope, message: &str) {
    let msg = v8::String::new(scope, message).unwrap();
    let err = v8::Exception::error(scope, msg);
    scope.throw_exception(err);
}

fn value_is_truthy(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> bool {
    if value.is_null_or_undefined() || value.is_false() {
        return false;
    }
    if value.is_number() {
        return value
            .number_value(scope)
            .map(|n| n != 0.0 && !n.is_nan())
            .unwrap_or(false);
    }
    if value.is_string() {
        return value
            .to_string(scope)
            .map(|s| !s.to_rust_string_lossy(scope).is_empty())
            .unwrap_or(false);
    }
    true
}

fn assert_ok(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let value = args.get(0);
    let truthy = value_is_truthy(scope, value);
    if !truthy {
        let message = if args.length() > 1 {
            args.get(1)
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_else(|| "Assertion failed".to_string())
        } else {
            "Assertion failed".to_string()
        };
        throw_assertion(scope, &message);
    }
}

fn assert_equal(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let a = args.get(0);
    let b = args.get(1);
    // loose equality via JS == by stringifying numbers/bools
    let a_num = a.number_value(scope);
    let b_num = b.number_value(scope);
    let equal = match (a_num, b_num) {
        (Some(x), Some(y)) if a.is_number() && b.is_number() => (x - y).abs() < f64::EPSILON,
        _ => {
            let as_ = a.to_string(scope).map(|s| s.to_rust_string_lossy(scope));
            let bs = b.to_string(scope).map(|s| s.to_rust_string_lossy(scope));
            as_ == bs
        }
    };
    if !equal {
        throw_assertion(scope, "AssertionError [ERR_ASSERTION]: values not equal");
    }
}

fn assert_strict_equal(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let a = args.get(0);
    let b = args.get(1);
    if !a.strict_equals(b) {
        let left = a
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_else(|| "undefined".into());
        let right = b
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_else(|| "undefined".into());
        throw_assertion(
            scope,
            &format!(
                "AssertionError [ERR_ASSERTION]: Expected values to be strictly equal: {} !== {}",
                left, right
            ),
        );
    }
}

fn assert_deep_strict_equal(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let a = args.get(0);
    let b = args.get(1);
    if a.strict_equals(b) {
        return;
    }
    // JSON-based deep compare for plain objects/arrays
    let json_key = v8::String::new(scope, "JSON").unwrap();
    let global = scope.get_current_context().global(scope);
    if let Some(json_val) = global.get(scope, json_key.into()) {
        if let Ok(json_obj) = v8::Local::<v8::Object>::try_from(json_val) {
            let stringify_key = v8::String::new(scope, "stringify").unwrap();
            if let Some(stringify_val) = json_obj.get(scope, stringify_key.into()) {
                if let Ok(stringify) = v8::Local::<v8::Function>::try_from(stringify_val) {
                    let a_json = stringify.call(scope, json_obj.into(), &[a]);
                    let b_json = stringify.call(scope, json_obj.into(), &[b]);
                    if let (Some(aj), Some(bj)) = (a_json, b_json) {
                        if aj.strict_equals(bj) {
                            return;
                        }
                    }
                }
            }
        }
    }
    throw_assertion(
        scope,
        "AssertionError [ERR_ASSERTION]: Expected values to be deeply equal",
    );
}

fn assert_throws(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let fn_val = args.get(0);
    if let Ok(func) = v8::Local::<v8::Function>::try_from(fn_val) {
        let undefined = v8::undefined(scope).into();
        let caught = {
            v8::tc_scope!(let try_catch, scope);
            let _ = func.call(try_catch, undefined, &[]);
            try_catch.has_caught()
        };
        if !caught {
            throw_assertion(scope, "Missing expected exception");
        }
    } else {
        throw_assertion(scope, "assert.throws requires a function");
    }
}

fn assert_fail(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let message = if args.length() > 0 {
        args.get(0)
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_else(|| "Failed".into())
    } else {
        "Failed".into()
    };
    throw_assertion(scope, &message);
}

fn assert_if_error(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let val = args.get(0);
    if !val.is_null() && !val.is_undefined() {
        scope.throw_exception(val);
    }
}

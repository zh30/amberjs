//! Pins docs/UTIL_CONTRACT.md.
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn runtime() -> MinimalRuntime {
    MinimalRuntime::new().expect("Failed to create runtime")
}

fn run(code: &str) -> String {
    runtime()
        .execute_code(code)
        .unwrap_or_else(|err| panic!("Execution failed: {err}"))
        .trim()
        .to_string()
}

#[test]
#[serial]
fn require_util_is_global_util() {
    let code = r#"
        const u = require('util');
        const nu = require('node:util');
        [
          u === globalThis.util,
          nu === globalThis.util,
          typeof u.format,
          typeof u.inspect,
          typeof u.promisify,
          typeof u.callbackify,
          typeof u.inherits
        ].join('|');
        "#;
    assert_eq!(
        run(code),
        "true|true|function|function|function|function|function"
    );
}

#[test]
#[serial]
fn format_and_inspect_basics() {
    let code = r#"
        const util = require('util');
        [
          util.format('Hello %s, count: %d', 'world', 42),
          util.format('%%s %s', 'x'),
          util.format('hi', 1, 2),
          util.inspect(null),
          util.inspect(undefined),
          util.inspect('ab'),
          util.inspect([1, 2]),
          util.inspect({ a: 1, b: 'two' }),
          util.inspect({ a: 1 }).includes('a')
        ].join('|');
        "#;
    assert_eq!(
        run(code),
        "Hello world, count: 42|%s x|hi 1 2|null|undefined|'ab'|Array(2)|{ a: 1, b: 'two' }|true"
    );
}

#[test]
#[serial]
fn types_stable_members() {
    let code = r#"
        const t = require('util').types;
        [
          t.isDate(new Date()),
          t.isDate({}),
          t.isMap(new Map()),
          t.isSet(new Set()),
          t.isPromise(Promise.resolve(1)),
          t.isTypedArray(new Uint8Array(1)),
          t.isAnyArrayBuffer(new ArrayBuffer(1)),
          t.isDataView(new DataView(new ArrayBuffer(1))),
          t.isNativeError(new Error('x')),
          t.isWeakMap(new WeakMap()),
          t.isWeakSet(new WeakSet()),
          t.isArgumentsObject((function () { return arguments; })()),
          t.isBooleanObject(new Boolean(true)),
          t.isNumberObject(new Number(1)),
          t.isStringObject(new String('a')),
          t.isBoxedPrimitive(new String('a')),
          t.isAsyncFunction(async function () {}),
          t.isGeneratorFunction(function* () {})
        ].join('|');
        "#;
    assert_eq!(
        run(code),
        "true|false|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true"
    );
}

#[test]
#[serial]
fn legacy_predicates() {
    let code = r#"
        const util = require('util');
        [
          util.isArray([1]),
          util.isArray({}),
          util.isBoolean(true),
          util.isNull(null),
          util.isNumber(123),
          util.isString('abc'),
          util.isUndefined(undefined),
          util.isObject({}),
          util.isObject(null),
          util.isFunction(function () {})
        ].join('|');
        "#;
    assert_eq!(
        run(code),
        "true|false|true|true|true|true|true|true|false|true"
    );
}

#[test]
#[serial]
fn promisify_callbackify_inherits_and_custom() {
    let code = r#"
        (async () => {
          const util = require('util');
          function callbackFn(x, cb) {
            if (x < 0) cb(new Error('negative'));
            else cb(null, x * 2);
          }
          const asyncFn = util.promisify(callbackFn);
          const ok = await asyncFn(21);
          let neg = '';
          try { await asyncFn(-1); } catch (e) { neg = e.message; }

          function customFn(cb) { cb(null, 1); }
          customFn[util.promisify.custom] = async () => 99;
          const custom = await util.promisify(customFn)();

          const cbf = util.callbackify(async (x) => x + 1);
          const cbRes = await new Promise((resolve, reject) => {
            cbf(2, (err, v) => err ? reject(err) : resolve(v));
          });

          function A() {}
          function B() {}
          util.inherits(B, A);
          const linked =
            Object.getPrototypeOf(B.prototype) === A.prototype && B.super_ === A;

          let bad = '';
          try { util.promisify(1); } catch (e) { bad = e.name; }

          return [
            ok,
            neg,
            custom,
            typeof util.promisify.custom,
            cbRes,
            linked,
            bad
          ].join('|');
        })()
        "#;
    assert_eq!(
        run(code),
        "42|negative|99|symbol|3|true|TypeError"
    );
}

#[test]
#[serial]
fn cli_require_util_reaches_the_contract() {
    let binary = env!("CARGO_BIN_EXE_amber");
    let dir = TempDir::new().expect("temp dir");
    let script = dir.path().join("probe.js");
    fs::write(
        &script,
        r#"
        const util = require('util');
        console.log([
          util.format('%s:%d', 'a', 1),
          util.inspect({ k: 2 }).includes('k'),
          typeof util.promisify,
          typeof util.inherits
        ].join('|'));
        "#,
    )
    .expect("script");
    let output = Command::new(binary)
        .arg("run")
        .arg(&script)
        .output()
        .expect("amber run");
    assert!(
        output.status.success(),
        "amber run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "a:1|true|function|function"
    );
}

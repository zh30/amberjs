//! Pins docs/ASSERT_CONTRACT.md (narrow Node assert Stable surface + G69 match/rejects).
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
fn require_assert_is_global_callable() {
    let code = r#"
        const a = require('assert');
        const na = require('node:assert');
        const s = require('assert/strict');
        const ns = require('node:assert/strict');
        [
          typeof a === 'function',
          a === globalThis.assert,
          na === a,
          s === a,
          ns === a,
          typeof a.ok,
          typeof a.strictEqual,
          typeof a.deepStrictEqual,
          typeof a.throws,
          typeof a.fail,
          typeof a.ifError,
          typeof a.match,
          typeof a.doesNotMatch,
          typeof a.rejects,
          typeof a.doesNotReject
        ].join('|');
        "#;
    assert_eq!(
        run(code),
        "true|true|true|true|true|function|function|function|function|function|function|function|function|function|function"
    );
}

#[test]
#[serial]
fn ok_and_callable_truthiness() {
    let code = r#"
        const assert = require('assert');
        const results = [];
        assert.ok(true);
        assert(1);
        assert('x');
        assert({});
        results.push('pass');
        for (const [label, fn] of [
          ['false', () => assert.ok(false)],
          ['0', () => assert.ok(0)],
          ['empty', () => assert.ok('')],
          ['null', () => assert.ok(null)],
          ['undef', () => assert.ok(undefined)],
          ['nan', () => assert.ok(NaN)],
          ['callable', () => assert(false, 'custom-ok')],
        ]) {
          try { fn(); results.push(label + ':no-throw'); }
          catch (e) { results.push(label + ':' + String(e.message)); }
        }
        results.join('|');
        "#;
    assert_eq!(
        run(code),
        "pass|false:Assertion failed|0:Assertion failed|empty:Assertion failed|null:Assertion failed|undef:Assertion failed|nan:Assertion failed|callable:custom-ok"
    );
}

#[test]
#[serial]
fn strict_equal_pass_and_fail() {
    let code = r#"
        const assert = require('assert');
        assert.strictEqual(1, 1);
        assert.strictEqual('a', 'a');
        let msg = '';
        try { assert.strictEqual(1, 2); } catch (e) { msg = String(e.message); }
        [
          msg.includes('strictly equal'),
          msg.includes('1'),
          msg.includes('2')
        ].join('|');
        "#;
    assert_eq!(run(code), "true|true|true");
}

#[test]
#[serial]
fn deep_strict_equal_json_objects() {
    let code = r#"
        const assert = require('assert');
        assert.deepStrictEqual({ a: 1, b: [2, 3] }, { a: 1, b: [2, 3] });
        assert.deepStrictEqual([1, { x: true }], [1, { x: true }]);
        let msg = '';
        try { assert.deepStrictEqual({ a: 1 }, { a: 2 }); }
        catch (e) { msg = String(e.message); }
        msg.includes('deeply equal');
        "#;
    assert_eq!(run(code), "true");
}

#[test]
#[serial]
fn throws_fail_if_error() {
    let code = r#"
        const assert = require('assert');
        assert.throws(() => { throw new Error('boom'); });
        let missing = '';
        try { assert.throws(() => 1); } catch (e) { missing = String(e.message); }
        let badFn = '';
        try { assert.throws(1); } catch (e) { badFn = String(e.message); }
        let failMsg = '';
        try { assert.fail('nope'); } catch (e) { failMsg = String(e.message); }
        let failDefault = '';
        try { assert.fail(); } catch (e) { failDefault = String(e.message); }
        assert.ifError(null);
        assert.ifError(undefined);
        let ifErr = '';
        try { assert.ifError(new Error('e1')); } catch (e) { ifErr = e.message; }
        [
          missing.includes('Missing expected exception'),
          badFn.includes('requires a function'),
          failMsg,
          failDefault,
          ifErr
        ].join('|');
        "#;
    assert_eq!(run(code), "true|true|nope|Failed|e1");
}

#[test]
#[serial]
fn match_and_does_not_match() {
    let code = r#"
        const assert = require('assert');
        assert.match('hello-42', /hello-\d+/);
        assert.doesNotMatch('hello', /world/);
        let mismatch = '';
        try { assert.match('abc', /xyz/); } catch (e) { mismatch = String(e.message); }
        let unwanted = '';
        try { assert.doesNotMatch('abc', /a/); } catch (e) { unwanted = String(e.message); }
        let custom = '';
        try { assert.match('x', /y/, 'custom-match'); } catch (e) { custom = String(e.message); }
        let badStr = '';
        try { assert.match(1, /x/); } catch (e) { badStr = e.name + ':' + String(e.message); }
        let badRe = '';
        try { assert.match('x', 'y'); } catch (e) { badRe = e.name + ':' + String(e.message); }
        [
          mismatch.includes('did not match'),
          unwanted.includes('not match'),
          custom,
          badStr.includes('TypeError') && badStr.includes('string'),
          badRe.includes('TypeError') && badRe.includes('regexp')
        ].join('|');
        "#;
    assert_eq!(run(code), "true|true|custom-match|true|true");
}

#[test]
#[serial]
fn rejects_and_does_not_reject() {
    let code = r#"
        (async () => {
          const assert = require('assert');
          await assert.rejects(Promise.reject(new Error('r1')));
          await assert.rejects(async () => { throw new Error('r2'); });
          await assert.rejects(() => { throw new Error('r3'); });
          await assert.doesNotReject(Promise.resolve(1));
          await assert.doesNotReject(async () => 2);
          await assert.doesNotReject(() => 3);

          let missing = '';
          try { await assert.rejects(Promise.resolve('ok')); }
          catch (e) { missing = String(e.message); }

          let unwanted = '';
          try { await assert.doesNotReject(Promise.reject(new Error('x'))); }
          catch (e) { unwanted = String(e.message); }

          return [
            missing.includes('Missing expected rejection'),
            unwanted.includes('Got unwanted rejection')
          ].join('|');
        })()
        "#;
    assert_eq!(run(code), "true|true");
}

#[test]
#[serial]
fn non_goals_equal_not_stable() {
    let code = r#"
        const assert = require('assert');
        // equal / deepEqual may exist but are not Stable — G69 pins match/rejects instead.
        [
          typeof assert.equal,
          typeof assert.deepEqual,
          typeof assert.doesNotThrow
        ].join('|');
        "#;
    assert_eq!(run(code), "function|function|undefined");
}

#[test]
#[serial]
fn cli_assert_basics_reach_the_contract() {
    let binary = env!("CARGO_BIN_EXE_amber");
    let dir = TempDir::new().expect("temp dir");
    let script = dir.path().join("probe.js");
    fs::write(
        &script,
        r#"
        const assert = require('assert');
        assert.ok(true);
        assert.strictEqual(2, 2);
        assert.deepStrictEqual({ a: 1 }, { a: 1 });
        assert.throws(() => { throw new Error('x'); });
        try { assert.fail('x'); } catch (_) {}
        assert.ifError(null);
        assert.match('ab12', /ab\d+/);
        assert.doesNotMatch('ab', /xy/);
        (async () => {
          await assert.rejects(Promise.reject(new Error('e')));
          await assert.doesNotReject(Promise.resolve(1));
          console.log([
            typeof assert === 'function',
            require('assert/strict') === assert,
            typeof assert.ok,
            typeof assert.match,
            typeof assert.rejects
          ].join('|'));
        })();
        "#,
    )
    .expect("script");
    let output = Command::new(binary)
        .arg("run")
        .arg(&script)
        .output()
        .expect("run amber");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "amber run failed: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("true|true|function|function|function"),
        "unexpected stdout: {stdout}"
    );
}

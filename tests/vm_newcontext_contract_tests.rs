//! Pins docs/VM_NEWCONTEXT_CONTRACT.md — Stable vm.runInNewContext sandbox (G66).
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

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
fn require_vm_exposes_run_in_new_context_and_script() {
    let output = run(r#"
        const vm = require('vm');
        const node = require('node:vm');
        [
          vm === node,
          vm === globalThis.vm,
          typeof vm.runInNewContext,
          typeof new vm.Script('1').runInNewContext
        ].join('|');
        "#);
    assert_eq!(output, "true|true|function|function");
}

#[test]
#[serial]
fn run_in_new_context_returns_completion_value() {
    let output = run(r#"
        const vm = require('vm');
        [
          vm.runInNewContext('1 + 2'),
          vm.runInNewContext('"amber"'),
          vm.runInNewContext('({ a: 1 }).a')
        ].join('|');
        "#);
    assert_eq!(output, "3|amber|1");
}

#[test]
#[serial]
fn run_in_new_context_isolates_caller_global_this() {
    let output = run(r#"
        const vm = require('vm');
        globalThis.__g66_caller = 1;
        vm.runInNewContext('globalThis.__g66_caller = 99; globalThis.__g66_leak = 7');
        const caller = globalThis.__g66_caller;
        const leaked = typeof globalThis.__g66_leak;
        delete globalThis.__g66_caller;
        delete globalThis.__g66_leak;
        [caller, leaked].join('|');
        "#);
    assert_eq!(output, "1|undefined");
}

#[test]
#[serial]
fn run_in_new_context_copies_sandbox_globals_in_and_back() {
    let output = run(r#"
        const vm = require('vm');
        const sandbox = { a: 1 };
        const result = vm.runInNewContext('a = a + 1; b = 9; a + b', sandbox);
        [result, sandbox.a, sandbox.b].join('|');
        "#);
    assert_eq!(output, "11|2|9");
}

#[test]
#[serial]
fn script_run_in_new_context_matches_module_helper() {
    let output = run(r#"
        const vm = require('vm');
        const sandbox = { n: 6 };
        const script = new vm.Script('n * 7');
        const viaFactory = vm.Script('n = n + 1; n');
        const a = script.runInNewContext(sandbox);
        const b = viaFactory.runInNewContext(sandbox);
        [a, b, sandbox.n].join('|');
        "#);
    assert_eq!(output, "42|7|7");
}

#[test]
#[serial]
fn thrown_errors_and_syntax_errors_surface_to_caller() {
    let output = run(r#"
        const vm = require('vm');
        function check(fn) {
          try { fn(); return 'ok'; }
          catch (e) { return e instanceof Error ? 'err' : 'other'; }
        }
        [
          check(() => vm.runInNewContext('throw new Error("g66-throw")')),
          check(() => vm.runInNewContext('this is not valid js @@@')),
          check(() => new vm.Script('throw new Error("script-throw")').runInNewContext())
        ].join('|');
        "#);
    assert_eq!(output, "err|err|err");
}

// ===== G73 additive: createContext/isContext markers + runInContext =====

#[test]
#[serial]
fn g73_create_context_marks_and_is_context() {
    let output = run(r#"
        const vm = require('vm');
        const r = [];
        r.push(vm.isContext({}));
        r.push(vm.isContext(42));
        r.push(vm.isContext('x'));
        const ctx = vm.createContext({ seed: 2 });
        r.push(vm.isContext(ctx));
        const fresh = vm.createContext();
        r.push(vm.isContext(fresh));
        r.join('|');
        "#);
    assert_eq!(output, "false|false|false|true|true");
}

#[test]
#[serial]
fn g73_run_in_context_top_level_and_script() {
    let output = run(r#"
        const vm = require('vm');
        const r = [];
        const ctx = vm.createContext({ x: 5 });
        r.push(vm.runInContext('x * 2', ctx));
        r.push(ctx.x);           // sandbox copy-back keeps mutations
        r.push(ctx.y === undefined ? 'u' : ctx.y);
        const s = new vm.Script('a + b');
        r.push(s.runInContext(vm.createContext({ a: 3, b: 4 })));
        vm.runInContext('y = 42; x = x * 10', ctx);
        r.push(ctx.x + '/' + ctx.y);  // 第二次调用拷回新全局 + 覆盖 sandbox key
        r.join('|');
        "#);
    assert_eq!(output, "10|5|u|7|50/42");
}

#[test]
#[serial]
fn g73_contextified_globals_do_not_leak_to_caller() {
    let output = run(r#"
        const vm = require('vm');
        const ctx = vm.createContext({});
        vm.runInContext('leaked = 1', ctx);
        String(typeof globalThis.leaked) + '|' + ctx.leaked;
        "#);
    assert_eq!(output, "undefined|1");
}

#[test]
#[serial]
fn g73_exceptions_rethrown_as_caller_errors() {
    let output = run(r#"
        const vm = require('vm');
        const r = [];
        const ctx = vm.createContext({});
        try { vm.runInContext('throw new RangeError("boom")', ctx); } catch (e) {
          r.push(e.name + ':' + (e instanceof Error) + ':' + e.message);
        }
        try { new vm.Script('syntax error((').runInContext(ctx); } catch (e) {
          r.push(e.name);
        }
        r.join('|');
        "#);
    assert_eq!(output, "RangeError:true:boom|SyntaxError");
}

#[test]
#[serial]
fn g73_is_context_rejects_inherited_marker() {
    let output = run(r#"
        const vm = require('vm');
        const proto = { _vmContext: true };
        const child = Object.create(proto);
        const forge = { _vmContext: true };  // own 标记可伪造——契约 Limit
        [
          vm.isContext(child),
          vm.isContext(forge),
          vm.isContext(vm.createContext({}))
        ].join('|');
        "#);
    assert_eq!(output, "false|true|true");
}

#[test]
#[serial]
fn g73_create_context_frozen_sandbox_fails_closed() {
    let output = run(r#"
        const vm = require('vm');
        const r = [];
        try {
          vm.createContext(Object.freeze({}));
          r.push('no-throw');
        } catch (e) {
          r.push(e.name + ':' + (e instanceof TypeError));
        }
        try {
          vm.createContext(Object.preventExtensions({}));
          r.push('no-throw-2');
        } catch (e) {
          r.push(e.name);
        }
        r.join('|');
        "#);
    assert_eq!(output, "TypeError:true|TypeError");
}

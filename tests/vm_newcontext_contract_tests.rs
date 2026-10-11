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
    let output = run(
        r#"
        const vm = require('vm');
        const node = require('node:vm');
        [
          vm === node,
          vm === globalThis.vm,
          typeof vm.runInNewContext,
          typeof new vm.Script('1').runInNewContext
        ].join('|');
        "#,
    );
    assert_eq!(output, "true|true|function|function");
}

#[test]
#[serial]
fn run_in_new_context_returns_completion_value() {
    let output = run(
        r#"
        const vm = require('vm');
        [
          vm.runInNewContext('1 + 2'),
          vm.runInNewContext('"amber"'),
          vm.runInNewContext('({ a: 1 }).a')
        ].join('|');
        "#,
    );
    assert_eq!(output, "3|amber|1");
}

#[test]
#[serial]
fn run_in_new_context_isolates_caller_global_this() {
    let output = run(
        r#"
        const vm = require('vm');
        globalThis.__g66_caller = 1;
        vm.runInNewContext('globalThis.__g66_caller = 99; globalThis.__g66_leak = 7');
        const caller = globalThis.__g66_caller;
        const leaked = typeof globalThis.__g66_leak;
        delete globalThis.__g66_caller;
        delete globalThis.__g66_leak;
        [caller, leaked].join('|');
        "#,
    );
    assert_eq!(output, "1|undefined");
}

#[test]
#[serial]
fn run_in_new_context_copies_sandbox_globals_in_and_back() {
    let output = run(
        r#"
        const vm = require('vm');
        const sandbox = { a: 1 };
        const result = vm.runInNewContext('a = a + 1; b = 9; a + b', sandbox);
        [result, sandbox.a, sandbox.b].join('|');
        "#,
    );
    assert_eq!(output, "11|2|9");
}

#[test]
#[serial]
fn script_run_in_new_context_matches_module_helper() {
    let output = run(
        r#"
        const vm = require('vm');
        const sandbox = { n: 6 };
        const script = new vm.Script('n * 7');
        const viaFactory = vm.Script('n = n + 1; n');
        const a = script.runInNewContext(sandbox);
        const b = viaFactory.runInNewContext(sandbox);
        [a, b, sandbox.n].join('|');
        "#,
    );
    assert_eq!(output, "42|7|7");
}

#[test]
#[serial]
fn thrown_errors_and_syntax_errors_surface_to_caller() {
    let output = run(
        r#"
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
        "#,
    );
    assert_eq!(output, "err|err|err");
}

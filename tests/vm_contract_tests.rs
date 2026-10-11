//! Pins docs/VM_CONTRACT.md — Stable vm.runInThisContext / Script.runInThisContext (G55).
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
fn require_vm_exposes_run_in_this_context_and_script() {
    let output = run(r#"
        const vm = require('vm');
        const node = require('node:vm');
        [
          vm === node,
          vm === globalThis.vm,
          typeof vm.runInThisContext,
          typeof vm.Script,
          typeof new vm.Script('1').runInThisContext
        ].join('|');
        "#);
    assert_eq!(output, "true|true|function|function|function");
}

#[test]
#[serial]
fn run_in_this_context_returns_completion_value() {
    let output = run(r#"
        const vm = require('vm');
        [
          vm.runInThisContext('1 + 2'),
          vm.runInThisContext('"amber"'),
          vm.runInThisContext('({ a: 1 }).a')
        ].join('|');
        "#);
    assert_eq!(output, "3|amber|1");
}

#[test]
#[serial]
fn run_in_this_context_mutates_current_global_this() {
    let output = run(r#"
        const vm = require('vm');
        globalThis.__g55_vm_pin = 1;
        vm.runInThisContext('globalThis.__g55_vm_pin = 42');
        const after = globalThis.__g55_vm_pin;
        delete globalThis.__g55_vm_pin;
        String(after);
        "#);
    assert_eq!(output, "42");
}

#[test]
#[serial]
fn script_run_in_this_context_matches_module_helper() {
    let output = run(r#"
        const vm = require('vm');
        const script = new vm.Script('7 * 6');
        const viaCall = vm.Script('globalThis.__g55_script = 9');
        viaCall.runInThisContext();
        const marker = globalThis.__g55_script;
        delete globalThis.__g55_script;
        [script.runInThisContext(), marker].join('|');
        "#);
    assert_eq!(output, "42|9");
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
          check(() => vm.runInThisContext('throw new Error("g55-throw")')),
          check(() => vm.runInThisContext('this is not valid js @@@')),
          check(() => new vm.Script('throw new Error("script-throw")').runInThisContext())
        ].join('|');
        "#);
    assert_eq!(output, "err|err|err");
}

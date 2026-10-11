//! Pins docs/TIMERS_PROMISES_CONTRACT.md (G50). Promise wrappers over G13/G42 timers;
//! does not replace timers_module_contract_tests or streams_timers_contract_tests.
use amberjs::nodejs_core::timers::{clear_all_async_timers, clear_all_timers};
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn runtime() -> MinimalRuntime {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_timer_drain_limit_ms(500);
    runtime
}

fn run(code: &str) -> String {
    runtime()
        .execute_code(code)
        .unwrap_or_else(|err| panic!("Execution failed: {err}"))
        .trim()
        .to_string()
}

fn reset_timers() {
    clear_all_timers();
    clear_all_async_timers();
}

#[test]
#[serial]
fn require_timers_promises_exports_set_timeout_and_set_immediate() {
    reset_timers();
    let output = run(r#"
        const a = require('timers/promises');
        const b = require('node:timers/promises');
        [
          typeof a.setTimeout,
          typeof a.setImmediate,
          typeof a.setInterval,
          typeof a.scheduler,
          typeof a.default,
          a.default && a.default.setTimeout === a.setTimeout,
          a.default && a.default.setImmediate === a.setImmediate,
          typeof b.setTimeout,
          typeof b.setImmediate
        ].join('|');
        "#);
    assert_eq!(
        output,
        "function|function|undefined|undefined|object|true|true|function|function"
    );
}

#[test]
#[serial]
fn set_timeout_resolves_value_after_delay_not_same_turn() {
    reset_timers();
    let output = run(r#"
        const { setTimeout } = require('timers/promises');
        (async () => {
          const order = [];
          order.push('sync');
          queueMicrotask(() => order.push('micro'));
          const p = setTimeout(0, 'done');
          order.push(typeof p.then);
          const v = await p;
          order.push(v);
          return order.join(',');
        })();
        "#);
    // sync + Promise creation, then microtasks, then zero-delay timer resolve
    assert_eq!(output, "sync,function,micro,done");
}

#[test]
#[serial]
fn set_immediate_resolves_value() {
    reset_timers();
    let output = run(r#"
        const { setImmediate } = require('timers/promises');
        setImmediate('imme');
        "#);
    assert_eq!(output, "imme");
}

#[test]
#[serial]
fn abort_signal_rejects_when_already_aborted() {
    reset_timers();
    let output = run(r#"
        const { setTimeout } = require('timers/promises');
        (async () => {
          const c = new AbortController();
          c.abort('early-reason');
          try {
            await setTimeout(1000, 'x', { signal: c.signal });
            return 'resolved';
          } catch (e) {
            return String(e);
          }
        })();
        "#);
    assert_eq!(output, "early-reason");
}

#[test]
#[serial]
fn abort_signal_rejects_during_wait_with_default_message() {
    reset_timers();
    let output = run(r#"
        const { setTimeout: sleep } = require('timers/promises');
        (async () => {
          const c = new AbortController();
          const p = sleep(200, 'late', { signal: c.signal });
          globalThis.setTimeout(() => c.abort(), 10);
          try {
            await p;
            return 'resolved';
          } catch (e) {
            return [
              e instanceof Error,
              String(e.message || '').includes('aborted')
            ].join('|');
          }
        })();
        "#);
    assert_eq!(output, "true|true");
}

#[test]
#[serial]
fn non_goals_absent_from_export_object() {
    reset_timers();
    let output = run(r#"
        const tp = require('timers/promises');
        const timers = require('timers');
        [
          typeof tp.setInterval,
          typeof tp.scheduler,
          typeof timers.promises
        ].join('|');
        "#);
    assert_eq!(output, "undefined|undefined|undefined");
}

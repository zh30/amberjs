//! Pins docs/TIMERS_MODULE_CONTRACT.md (G42). Node `require('timers')` re-exports;
//! does not replace G13 streams_timers_contract_tests.
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
fn require_timers_and_node_timers_reexport_globals() {
    reset_timers();
    let output = run(r#"
        const a = require('timers');
        const b = require('node:timers');
        const names = [
          'setTimeout', 'clearTimeout',
          'setInterval', 'clearInterval',
          'setImmediate', 'clearImmediate'
        ];
        const sameObj = a === b;
        const sameFns = names.every((n) => a[n] === b[n] && a[n] === globalThis[n]);
        const types = names.map((n) => typeof a[n]).join('|');
        [
          sameObj || sameFns,
          types,
          typeof a.promises,
          typeof a.queueMicrotask,
          a.clearTimeout === a.clearInterval,
          a.clearTimeout === a.clearImmediate
        ].join('::');
        "#);
    assert_eq!(
        output,
        "true::function|function|function|function|function|function::undefined::undefined::true::true"
    );
}

#[test]
#[serial]
fn module_set_timeout_zero_delay_after_microtasks_and_clear() {
    reset_timers();
    let output = run(r#"
        const { setTimeout, clearTimeout } = require('timers');
        new Promise((resolve) => {
            const order = [];
            order.push('sync');
            queueMicrotask(() => order.push('micro'));
            let cleared = 0;
            const cancelled = setTimeout(() => { cleared = 1; }, 40);
            clearTimeout(cancelled);
            setTimeout((label) => {
                order.push(label);
                order.push('cleared=' + cleared);
                order.push('id=' + typeof cancelled._timerId);
                resolve(order.join(','));
            }, 0, 'timeout');
        });
        "#);
    assert_eq!(output, "sync,micro,timeout,cleared=0,id=number");
}

#[test]
#[serial]
fn module_set_interval_repeats_until_clear() {
    reset_timers();
    let mut rt = runtime();
    rt.set_timer_drain_limit_ms(400);
    let output = rt
        .execute_code(
            r#"
            const { setInterval, clearInterval } = require('timers');
            new Promise((resolve) => {
                let n = 0;
                const id = setInterval(() => {
                    n += 1;
                    if (n === 2) {
                        clearInterval(id);
                        resolve('ticks:' + n + ':' + typeof id._timerId);
                    }
                }, 20);
            });
            "#,
        )
        .unwrap_or_else(|err| panic!("interval failed: {err}"));
    assert_eq!(output.trim(), "ticks:2:number");
}

#[test]
#[serial]
fn module_set_immediate_runs_and_clear_cancels() {
    reset_timers();
    let output = run(r#"
        const { setImmediate, clearImmediate } = require('timers');
        new Promise((resolve) => {
            let cleared = 0;
            const cancelled = setImmediate(() => { cleared = 1; });
            clearImmediate(cancelled);
            setImmediate((label) => {
                resolve([label, 'cleared=' + cleared, typeof cancelled._timerId].join('|'));
            }, 'imme');
        });
        "#);
    assert_eq!(output, "imme|cleared=0|number");
}

#[test]
#[serial]
fn module_callbacks_must_be_functions() {
    reset_timers();
    let err = runtime()
        .execute_code("require('timers').setTimeout(1, 0);")
        .expect_err("non-function setTimeout");
    let message = err.to_string();
    assert!(
        message.contains("callback must be a function"),
        "got {message}"
    );

    let err = runtime()
        .execute_code("require('timers').setImmediate('nope');")
        .expect_err("non-function setImmediate");
    let message = err.to_string();
    assert!(
        message.contains("callback must be a function"),
        "got {message}"
    );
}

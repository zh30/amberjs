//! Pins docs/ASYNC_HOOKS_CONTRACT.md — narrow AsyncResource + AsyncLocalStorage (G57).
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
fn require_async_hooks_exposes_resource_and_als() {
    let output = run(r#"
        const ah = require('async_hooks');
        const node = require('node:async_hooks');
        [
          ah === node,
          typeof ah.AsyncResource,
          typeof ah.AsyncLocalStorage,
          typeof ah.AsyncResource.bind,
          typeof ah.AsyncLocalStorage.snapshot
        ].join('|');
        "#);
    assert_eq!(output, "true|function|function|function|function");
}

#[test]
#[serial]
fn async_resource_ids_run_bind_and_destroy() {
    let output = run(r#"
        const { AsyncResource } = require('async_hooks');
        const r = new AsyncResource('TestResource', 7);
        let ran = false;
        const scopeRet = r.runInAsyncScope(function (a, b) {
          ran = this.tag === 'ctx' && a === 2 && b === 3;
          return a + b;
        }, { tag: 'ctx' }, 2, 3);
        const bound = r.bind((x, y) => x * y);
        const staticBound = AsyncResource.bind((x) => x + 1, 'StaticBound');
        const destroyed = r.emitDestroy();
        [
          typeof r.asyncId() === 'number' && r.asyncId() > 0,
          r.triggerAsyncId(),
          ran,
          scopeRet,
          bound(6, 7),
          staticBound(41),
          destroyed === r
        ].join('|');
        "#);
    assert_eq!(output, "true|7|true|5|42|42|true");
}

#[test]
#[serial]
fn async_local_storage_nested_run_and_exit() {
    let output = run(r#"
        const { AsyncLocalStorage } = require('async_hooks');
        const als = new AsyncLocalStorage();
        const parts = [];
        parts.push(als.getStore() === undefined);
        als.run({ user: 'alice' }, () => {
          parts.push(als.getStore().user);
          als.run({ user: 'bob' }, () => {
            parts.push(als.getStore().user);
          });
          parts.push(als.getStore().user);
          als.exit(() => {
            parts.push(als.getStore() === undefined);
          });
          parts.push(als.getStore().user);
        });
        parts.push(als.getStore() === undefined);
        parts.join('|');
        "#);
    assert_eq!(output, "true|alice|bob|alice|true|alice|true");
}

#[test]
#[serial]
fn async_local_storage_enter_with_disable_and_snapshot() {
    let output = run(r#"
        const { AsyncLocalStorage } = require('async_hooks');
        const als = new AsyncLocalStorage();
        const parts = [];
        als.enterWith({ n: 1 });
        parts.push(als.getStore().n);
        const snap = AsyncLocalStorage.snapshot();
        als.enterWith({ n: 2 });
        parts.push(als.getStore().n);
        snap(() => {
          parts.push(als.getStore().n);
        });
        parts.push(als.getStore().n);
        als.disable();
        parts.push(als.getStore() === undefined);
        parts.push(als.run({ n: 9 }, () => als.getStore()) === undefined);
        parts.join('|');
        "#);
    assert_eq!(output, "1|2|1|2|true|true");
}

#[test]
#[serial]
fn async_local_storage_thenable_run_restores_previous() {
    let output = run(r#"
        const { AsyncLocalStorage } = require('async_hooks');
        const als = new AsyncLocalStorage();
        (async () => {
          const outer = await als.run({ phase: 'outer' }, async () => {
            const mid = als.getStore().phase;
            await als.run({ phase: 'inner' }, async () => {
              if (als.getStore().phase !== 'inner') throw new Error('inner store');
              return 'ok';
            });
            return mid + '|' + als.getStore().phase;
          });
          return outer + '|' + (als.getStore() === undefined);
        })();
        "#);
    assert_eq!(output, "outer|outer|true");
}

#[test]
#[serial]
fn create_hook_is_not_stable_tracking() {
    // Honesty pin: createHook exists for shape but does not track async resources.
    // Stable Non-goal — must not claim real hook firing as Limits fiction.
    let output = run(r#"
        const ah = require('async_hooks');
        let fired = 0;
        const hook = ah.createHook({
          init() { fired++; },
          before() { fired++; },
          after() { fired++; },
          destroy() { fired++; }
        });
        hook.enable();
        const r = new ah.AsyncResource('NoHookFire');
        r.runInAsyncScope(() => {});
        r.emitDestroy();
        hook.disable();
        [
          typeof ah.createHook,
          typeof hook.enable,
          typeof hook.disable,
          fired
        ].join('|');
        "#);
    assert_eq!(output, "function|function|function|0");
}

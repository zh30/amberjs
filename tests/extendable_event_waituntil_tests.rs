//! Real ExtendableEvent.waitUntil on the CLI path.
//! SW install/activate honor waitUntil until the promise settles.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn runtime() -> MinimalRuntime {
    MinimalRuntime::new().expect("runtime")
}

fn exec(runtime: &mut MinimalRuntime, code: &str) {
    runtime
        .execute_code(code)
        .unwrap_or_else(|err| panic!("execution failed: {err}"));
}

fn read_result(runtime: &mut MinimalRuntime) -> String {
    runtime
        .execute_code("globalThis.__amberWaitUntilResult")
        .unwrap_or_else(|err| panic!("read failed: {err}"))
        .trim()
        .to_string()
}

#[test]
#[serial]
fn extendable_event_constructor_exposes_wait_until() {
    let mut runtime = runtime();
    exec(
        &mut runtime,
        r#"
        const e = new ExtendableEvent('install');
        globalThis.__amberWaitUntilResult = [
          typeof ExtendableEvent,
          typeof e.waitUntil,
          e.type
        ].join('|');
        "#,
    );
    assert_eq!(read_result(&mut runtime), "function|function|install");
}

#[test]
#[serial]
fn install_wait_until_resolve_delays_registration() {
    let mut runtime = runtime();
    exec(
        &mut runtime,
        r#"
        globalThis.__amberWaitUntilResult = 'pending';
        const source = [
          "self.addEventListener('install', (event) => {",
          "  event.waitUntil(new Promise((resolve) => {",
          "    setTimeout(() => { self.__settled = true; resolve(); }, 80);",
          "  }));",
          "  self.skipWaiting();",
          "});",
          "self.addEventListener('activate', () => { self.clients.claim(); });",
          "self.addEventListener('message', (event) => {",
          "  event.source.postMessage({ settled: !!self.__settled });",
          "});"
        ].join('\n');
        const url = 'data:text/javascript,' + encodeURIComponent(source);
        const started = Date.now();
        let early = false;
        const earlyTimer = setTimeout(() => {
          // register must still be pending ~40ms in if waitUntil holds lifetime
          if (globalThis.__amberWaitUntilResult === 'pending') early = true;
        }, 40);
        const timer = setTimeout(() => {
          globalThis.__amberWaitUntilResult = 'timeout';
        }, 2000);
        navigator.serviceWorker.register(url).then((reg) => {
          const elapsed = Date.now() - started;
          navigator.serviceWorker.onmessage = (event) => {
            clearTimeout(timer);
            clearTimeout(earlyTimer);
            globalThis.__amberWaitUntilResult = [
              early === true,
              elapsed >= 70,
              event.data.settled === true,
              reg.active && reg.active.state
            ].join('|');
            reg.unregister();
          };
          reg.active.postMessage({ ping: 1 });
        }, (err) => {
          clearTimeout(timer);
          clearTimeout(earlyTimer);
          globalThis.__amberWaitUntilResult = 'reject:' + (err && err.message || err);
        });
        "#,
    );
    assert_eq!(
        read_result(&mut runtime),
        "true|true|true|activated",
        "waitUntil(resolve) must delay install/activate until the promise settles"
    );
}

#[test]
#[serial]
fn install_wait_until_reject_fails_registration() {
    let mut runtime = runtime();
    exec(
        &mut runtime,
        r#"
        globalThis.__amberWaitUntilResult = 'pending';
        const source = [
          "self.addEventListener('install', (event) => {",
          "  event.waitUntil(Promise.reject(new Error('install-waituntil-failed')));",
          "});"
        ].join('\n');
        const url = 'data:text/javascript,' + encodeURIComponent(source);
        const timer = setTimeout(() => {
          globalThis.__amberWaitUntilResult = 'timeout';
        }, 2000);
        navigator.serviceWorker.register(url).then((reg) => {
          clearTimeout(timer);
          globalThis.__amberWaitUntilResult = 'resolved:' + (reg.waiting && reg.waiting.state);
          reg.unregister();
        }, (err) => {
          clearTimeout(timer);
          const message = String(err && err.message || err);
          globalThis.__amberWaitUntilResult =
            message.indexOf('install-waituntil-failed') >= 0 ? 'rejected' : 'reject:' + message;
        });
        "#,
    );
    assert_eq!(
        read_result(&mut runtime),
        "rejected",
        "waitUntil(reject) must fail registration"
    );
}

#[test]
#[serial]
fn activate_wait_until_resolve_runs_before_register_resolves() {
    let mut runtime = runtime();
    exec(
        &mut runtime,
        r#"
        globalThis.__amberWaitUntilResult = 'pending';
        const source = [
          "self.addEventListener('install', (event) => { self.skipWaiting(); });",
          "self.addEventListener('activate', (event) => {",
          "  event.waitUntil(new Promise((resolve) => {",
          "    setTimeout(() => { self.__activateDone = true; resolve(); }, 60);",
          "  }));",
          "  self.clients.claim();",
          "});",
          "self.addEventListener('message', (event) => {",
          "  event.source.postMessage({ activateDone: !!self.__activateDone });",
          "});"
        ].join('\n');
        const url = 'data:text/javascript,' + encodeURIComponent(source);
        const started = Date.now();
        const timer = setTimeout(() => {
          globalThis.__amberWaitUntilResult = 'timeout';
        }, 2000);
        navigator.serviceWorker.register(url).then((reg) => {
          const elapsed = Date.now() - started;
          navigator.serviceWorker.onmessage = (event) => {
            clearTimeout(timer);
            globalThis.__amberWaitUntilResult = [
              elapsed >= 50,
              event.data.activateDone === true,
              navigator.serviceWorker.controller === reg.active
            ].join('|');
            reg.unregister();
          };
          reg.active.postMessage({ ping: 1 });
        }, (err) => {
          clearTimeout(timer);
          globalThis.__amberWaitUntilResult = 'reject:' + (err && err.message || err);
        });
        "#,
    );
    assert_eq!(
        read_result(&mut runtime),
        "true|true|true",
        "activate waitUntil must settle before register resolves"
    );
}

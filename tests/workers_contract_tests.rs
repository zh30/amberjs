//! Pins the Stable worker contract in docs/WORKERS_CONTRACT.md.
//! Starts a dedicated worker and a service worker and exchanges a message.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::fs;

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
        .execute_code("globalThis.__amberWorkerContract")
        .unwrap_or_else(|err| panic!("read failed: {err}"))
        .trim()
        .to_string()
}

#[test]
#[serial]
fn dedicated_worker_exchanges_a_message_and_terminate_closes_it() {
    let mut runtime = runtime();
    let dir = tempfile::tempdir().expect("tempdir");
    let script_path = dir.path().join("worker.js");
    fs::write(
        &script_path,
        "self.onmessage = (e) => { postMessage({ reply: e.data.msg.toUpperCase(), from: 'file' }); };",
    )
    .expect("write worker");
    let path = script_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\'', "\\'");

    exec(
        &mut runtime,
        &format!(
            r#"
            globalThis.__amberWorkerContract = 'pending';
            let moduleError = '';
            try {{
              new Worker('data:,1', {{ type: 'module' }});
              moduleError = 'accepted';
            }} catch (e) {{
              moduleError = (e instanceof TypeError ? 'type' : 'other') + ':' + e.message;
            }}
            const shared = typeof SharedWorker;
            const worker = new Worker('{path}');
            worker.onmessage = (e) => {{
              let after = 'open';
              worker.terminate();
              try {{
                worker.postMessage({{ msg: 'again' }});
              }} catch (err) {{
                after = err.message;
              }}
              globalThis.__amberWorkerContract = [
                moduleError,
                shared,
                e.data.reply,
                e.data.from,
                after
              ].join('|');
            }};
            worker.postMessage({{ msg: 'hello worker' }});
            "#
        ),
    );

    assert_eq!(
        read_result(&mut runtime),
        "type:Worker type \"module\" is not supported|undefined|HELLO WORKER|file|Cannot postMessage to a terminated worker"
    );
}

#[test]
#[serial]
fn service_worker_scope_claim_and_message_do_not_wrap_fetch() {
    let mut runtime = runtime();
    exec(
        &mut runtime,
        r#"
        globalThis.__amberWorkerContract = 'pending';
        const fetchBefore = fetch;
        const source = [
          "self.addEventListener('install', () => { self.skipWaiting(); });",
          "self.addEventListener('activate', () => { self.clients.claim(); });",
          "self.addEventListener('message', (event) => {",
          "  event.source.postMessage({ reply: String(event.data.msg).toUpperCase() });",
          "});"
        ].join('\n');
        const url = 'data:text/javascript,' + encodeURIComponent(source);
        let registration = null;
        const timer = setTimeout(() => {
          globalThis.__amberWorkerContract = 'timeout';
          if (registration) registration.unregister();
        }, 2000);
        navigator.serviceWorker.onmessage = (event) => {
          clearTimeout(timer);
          const active = registration && registration.active;
          globalThis.__amberWorkerContract = [
            event.data.reply,
            registration.scope,
            active && active.state,
            navigator.serviceWorker.controller === active,
            fetch === fetchBefore
          ].join('|');
          registration.unregister();
        };
        navigator.serviceWorker.register(url, { scope: '/app/' }).then((reg) => {
          registration = reg;
          reg.active.postMessage({ msg: 'hello worker' });
        }, (err) => {
          clearTimeout(timer);
          globalThis.__amberWorkerContract = 'reject:' + (err && err.message || err);
        });
        "#,
    );

    assert_eq!(
        read_result(&mut runtime),
        "HELLO WORKER|/app/|activated|true|true"
    );
}

#[test]
#[serial]
fn service_worker_without_skip_waiting_stays_installed_and_missing_script_rejects() {
    let mut runtime = runtime();
    exec(
        &mut runtime,
        r#"
        globalThis.__amberWorkerContract = 'pending';
        const url = 'data:text/javascript,' + encodeURIComponent('/* classic, no skipWaiting */');
        const timer = setTimeout(() => {
          globalThis.__amberWorkerContract = 'timeout';
        }, 2000);
        navigator.serviceWorker.register(url).then((reg) => {
          clearTimeout(timer);
          const waitingState = reg.waiting ? reg.waiting.state : 'none';
          globalThis.__amberWorkerContract = [reg.scope, waitingState, String(reg.active)].join('|');
          return reg.unregister();
        }).then(() => {
          return navigator.serviceWorker.register('./definitely-missing-sw.js');
        }).then(() => {
          globalThis.__amberWorkerContract = 'missing-resolved';
        }, (err) => {
          const message = String(err && err.message || err);
          globalThis.__amberWorkerContract += '>' + (message.indexOf('could not load script') >= 0);
        });
        "#,
    );

    assert_eq!(read_result(&mut runtime), "/|installed|null>true");
}

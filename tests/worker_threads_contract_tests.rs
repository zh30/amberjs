use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

/// Workers are spawned against the process-global `WorkerHost`; every test
/// that spawns one must `terminate()` it (after any unref drain check) so the
/// OS thread / isolate is not left alive into the next test.
fn runtime() -> MinimalRuntime {
    MinimalRuntime::new().expect("MinimalRuntime::new")
}

fn run(code: &str) -> String {
    let mut runtime = runtime();
    runtime
        .execute_code(code)
        .expect("execute_code")
        .trim()
        .to_string()
}

#[test]
#[serial]
fn g75_module_bag_shape() {
    let output = run(r#"
        const wt = require('worker_threads');
        const wt2 = require('node:worker_threads');
        [
          wt === wt2,
          wt === globalThis.worker_threads,
          wt.isMainThread, wt.parentPort === null,
          wt.workerData === undefined, wt.threadId,
          typeof wt.Worker, typeof wt.MessagePort, typeof wt.MessageChannel
        ].join('|');
        "#);
    assert_eq!(
        output,
        "true|true|true|true|true|0|function|function|function"
    );
}

#[test]
#[serial]
fn g75_eval_worker_echo_and_workerdata() {
    let output = run(r#"
        const wt = require('worker_threads');
        let got = 'none'; let exitCode = 'none'; let selfOk = false;
        const src = `
          const wt = require('worker_threads');
          wt.parentPort.postMessage(JSON.stringify({
            isMain: wt.isMainThread, tid: wt.threadId > 0,
            data: wt.workerData, hasPort: typeof wt.parentPort.postMessage
          }));
          wt.parentPort.on('message', (m) => wt.parentPort.postMessage('echo:' + JSON.stringify(m)));
        `;
        const w = new wt.Worker(src, { eval: true, workerData: { hi: 7 } });
        w.on('message', (m) => {
          got = m;
          if (typeof m === 'string' && m.startsWith('echo')) w.terminate();
        });
        w.on('exit', (c) => { exitCode = c; });
        const shape = new Promise((resolve) => {
          const orig = w.listeners ? null : null;
          const seen = [];
          w.on('message', (m) => { seen.push(m); if (seen.length === 2) resolve(seen[0]); });
          w.postMessage({ ping: 1 });
        });
        shape;
        "#);
    // First delivery is the worker's startup shape message; echo arrives next.
    let v: serde_json::Value = serde_json::from_str(&output).expect("json shape");
    assert_eq!(v["isMain"], false);
    assert_eq!(v["tid"], true);
    assert_eq!(v["data"], serde_json::json!({"hi": 7}));
    assert_eq!(v["hasPort"], "function");
}

#[test]
#[serial]
fn g75_file_worker_round_trip() {
    let dir = std::env::temp_dir().join("amber_g75_test");
    std::fs::create_dir_all(&dir).unwrap();
    let worker_path = dir.join("g75_file_worker.js");
    std::fs::write(
        &worker_path,
        r#"const wt = require('worker_threads');
        wt.parentPort.on('message', (m) => wt.parentPort.postMessage({ back: m, wd: wt.workerData }));"#,
    )
    .unwrap();
    let code = format!(
        r#"
        const wt = require('worker_threads');
        const w = new wt.Worker('{}', {{ workerData: 'file-mode' }});
        const done = new Promise((resolve) => {{
          w.on('message', (m) => {{ w.terminate(); resolve(JSON.stringify(m)); }});
        }});
        w.postMessage({{ q: 9 }});
        done;
        "#,
        worker_path.display()
    );
    let output = run(&code);
    let v: serde_json::Value = serde_json::from_str(&output).expect("json");
    assert_eq!(v["back"], serde_json::json!({"q": 9}));
    assert_eq!(v["wd"], "file-mode");
}

#[test]
#[serial]
fn g75_worker_error_event_and_exit_code() {
    let output = run(r#"
        const wt = require('worker_threads');
        const w = new wt.Worker("throw new Error('g75-boom')", { eval: true });
        const done = new Promise((resolve) => {
          let errMsg = 'none'; let code = 'none';
          w.on('error', (e) => { errMsg = e.message; });
          w.on('exit', (c) => { code = c; resolve([errMsg.includes('g75-boom'), code].join('|')); });
        });
        done;
        "#);
    assert_eq!(output, "true|1");
}

#[test]
#[serial]
fn g75_worker_spawn_failure_throws() {
    let output = run(r#"
        const wt = require('worker_threads');
        let threw = 'no';
        try { new wt.Worker('/nonexistent-g75-xyz.js'); }
        catch (e) { threw = e.message.includes('could not load script') ? 'load' : 'other'; }
        threw;
        "#);
    assert_eq!(output, "load");
}

#[test]
#[serial]
fn g75_terminate_semantics() {
    let output = run(r#"
        const wt = require('worker_threads');
        const w = new wt.Worker("setTimeout(() => {}, 30000)", { eval: true });
        let exitCode = 'none';
        w.on('exit', (c) => { exitCode = c; });
        const ret = w.terminate();
        let threw = 'no';
        try { w.postMessage(1); } catch (_) { threw = 'yes'; }
        [ret === undefined, exitCode, threw].join('|');
        "#);
    assert_eq!(output, "true|0|yes");
}

#[test]
#[serial]
fn g75_worker_side_surface_limits() {
    // Inside the worker isolate: isMainThread false, threadId>0, require is a
    // whitelist (worker_threads + events only), nested Worker throws.
    let output = run(r#"
        const wt = require('worker_threads');
        const src = `
          const wt = require('worker_threads');
          const ev = require('events');
          let fsThrew = 'no'; let nested = 'no';
          try { require('fs'); } catch (_) { fsThrew = 'yes'; }
          try { new wt.Worker('x'); } catch (e) { nested = e.message.includes('Nested') ? 'yes' : 'other'; }
          wt.parentPort.postMessage(JSON.stringify({
            ev: typeof ev.EventEmitter === 'function', fsThrew, nested,
            isMain: wt.isMainThread, tid: wt.threadId
          }));
        `;
        const w = new wt.Worker(src, { eval: true });
        const done = new Promise((resolve) => {
          w.on('message', (m) => { w.terminate(); resolve(m); });
        });
        done;
        "#);
    let v: serde_json::Value = serde_json::from_str(&output).expect("json");
    assert_eq!(v["ev"], true);
    assert_eq!(v["fsThrew"], "yes");
    assert_eq!(v["nested"], "yes");
    assert_eq!(v["isMain"], false);
    assert!(v["tid"].as_i64().unwrap() > 0);
}

#[test]
#[serial]
fn g75_post_message_json_serialization() {
    // JSON-shaped transport (not structured clone): plain values pass;
    // circular / undefined / functions throw inside postMessage.
    // (The worker's pending timer keeps it alive — see the timer-liveness
    // test — so the throws here come from the serialization guard, not from
    // posting to a dead worker.)
    let output = run(r#"
        const wt = require('worker_threads');
        const w = new wt.Worker("setTimeout(() => {}, 30000)", { eval: true });
        const r = [];
        const t = (label, f) => { try { f(); r.push(label); } catch (_) { r.push(label + '-throw'); } };
        t('num', () => w.postMessage(1));
        t('obj', () => w.postMessage({ a: [1, 'x', null] }));
        t('circ', () => { const c = {}; c.s = c; w.postMessage(c); });
        t('undef', () => w.postMessage(undefined));
        t('fn', () => w.postMessage(() => 1));
        w.terminate();
        r.join(',');
        "#);
    assert_eq!(output, "num,obj,circ-throw,undef-throw,fn-throw");
}

#[test]
#[serial]
fn g75_worker_timer_runs_without_listener() {
    // A worker with a pending timer but no 'message' listener stays alive
    // until the timer drains (Node parity), then exits on its own.
    let output = run(r#"
        const wt = require('worker_threads');
        const w = new wt.Worker("setTimeout(() => require('worker_threads').parentPort.postMessage('tick'), 30)", { eval: true });
        const done = new Promise((resolve) => {
          w.on('message', (m) => { w.terminate(); resolve(m); });
        });
        done;
        "#);
    assert_eq!(output, "tick");
}

#[test]
#[serial]
fn g75_terminate_preempts_busy_worker() {
    // terminate() preempts a worker stuck in synchronous code via
    // terminate_execution — it does not wait for the script to yield.
    let output = run(r#"
        const wt = require('worker_threads');
        const w = new wt.Worker("while (true) {}", { eval: true });
        const done = new Promise((resolve) => {
          let exitCode = 'none';
          w.on('exit', (c) => { exitCode = c; });
          setTimeout(() => {
            w.terminate();
            resolve(['term-returned', exitCode].join('|'));
          }, 80);
        });
        done;
        "#);
    assert_eq!(output, "term-returned|0");
}

#[test]
#[serial]
fn g75_unref_lets_drain_finish() {
    // An unref'd live worker does not hold the runtime's drain loop — the eval
    // completes immediately with its value (same flag the CLI event loop reads).
    // Terminate afterward: WorkerHost is process-global, and leaving a live
    // isolate races the next serial test (macOS libc++ vector OOB / SIGABRT).
    let output = run(r#"
        const wt = require('worker_threads');
        const w = new wt.Worker("setTimeout(() => {}, 60000)", { eval: true });
        const r = w.unref();
        const drainOk = [r === w, 'done'].join('|');
        const exited = new Promise((resolve) => {
          w.once('exit', () => resolve(drainOk));
        });
        w.terminate();
        exited;
        "#);
    assert_eq!(output, "true|done");
}

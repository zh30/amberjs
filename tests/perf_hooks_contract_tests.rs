//! Pins docs/PERF_HOOKS_CONTRACT.md — Stable perf_hooks module surface (G74).
//!
//! `PERFORMANCE_STATE` (entries/marks) is process-global and shared across
//! runtimes in this test binary, so every test uses unique entry names and
//! filters by name instead of asserting whole-store contents.
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
fn g74_require_returns_real_bag() {
    let output = run(r#"
        const ph = require('perf_hooks');
        const nph = require('node:perf_hooks');
        [
          ph === nph,
          ph === globalThis.perf_hooks,
          ph.performance === globalThis.performance,
          typeof ph.PerformanceObserver,
          typeof ph.performance.timerify,
          typeof ph.performance.now,
          ph.performance.mark('g74-bag-mark') === undefined
        ].join('|');
        "#);
    assert_eq!(output, "true|true|true|function|function|function|true");
}

#[test]
#[serial]
fn g74_now_and_time_origin() {
    let output = run(r#"
        const p = require('perf_hooks').performance;
        const a = p.now(); const b = p.now();
        [
          typeof a, typeof p.timeOrigin,
          a >= 0, b >= a, p.timeOrigin > 0
        ].join('|');
        "#);
    assert_eq!(output, "number|number|true|true|true");
}

#[test]
#[serial]
fn g74_mark_measure_entry_shapes_and_forgiving_marks() {
    let output = run(r#"
        const p = require('perf_hooks').performance;
        p.mark('g74-m1'); p.measure('g74-meas', 'g74-m1');
        // G13 forgiving semantics stay pinned: an unknown mark name resolves to
        // start 0 rather than throwing.
        p.measure('g74-meas-missing', 'g74-no-such-mark');
        const marks = p.getEntriesByType('mark').filter(e => e.name === 'g74-m1');
        const meas = p.getEntriesByType('measure').filter(e => e.name === 'g74-meas');
        const miss = p.getEntriesByName('g74-meas-missing');
        const m = marks[0], v = meas[0], x = miss[0];
        [
          m.name, m.entryType, typeof m.startTime, m.duration,
          v.entryType, v.duration >= 0, v.startTime >= 0,
          x.startTime === 0, x.duration >= 0,
          p.getEntries().some(e => e.name === 'g74-m1'),
          p.getEntriesByName('g74-m1').length === 1
        ].join('|');
        "#);
    assert_eq!(
        output,
        "g74-m1|mark|number|0|measure|true|true|true|true|true|true"
    );
}

#[test]
#[serial]
fn g74_clear_marks_measures_named_filter() {
    let output = run(r#"
        const p = require('perf_hooks').performance;
        p.mark('g74-ca'); p.mark('g74-cb');
        p.measure('g74-cx'); p.measure('g74-cy');
        p.clearMarks('g74-ca'); p.clearMeasures('g74-cx');
        const marksLeft = p.getEntriesByType('mark').filter(e => e.name.startsWith('g74-c')).map(e => e.name).sort().join(',');
        const measLeft = p.getEntriesByType('measure').filter(e => e.name.startsWith('g74-c')).map(e => e.name).sort().join(',');
        p.clearMarks(); p.clearMeasures();
        const after = p.getEntries().filter(e => e.name.startsWith('g74-c')).length;
        [marksLeft, measLeft, after].join('|');
        "#);
    assert_eq!(output, "g74-cb|g74-cy|0");
}

#[test]
#[serial]
fn g74_timerify_records_function_entries() {
    let output = run(r#"
        const p = require('perf_hooks').performance;
        function g74Foo(a) { return a + 1; }
        const tf = p.timerify(g74Foo);
        const ret = tf(41);
        const entries = p.getEntriesByType('function').filter(e => e.name === 'g74Foo');
        const e = entries[entries.length - 1];
        let threw = 'no';
        try { p.timerify(42); } catch (err) { threw = String(err).includes('TypeError') ? 'TypeError' : 'other'; }
        [
          ret, tf.name, e.entryType, e.duration >= 0, e.startTime >= 0, threw
        ].join('|');
        "#);
    assert_eq!(output, "42|timerified g74Foo|function|true|true|TypeError");
}

#[test]
#[serial]
fn g74_timerify_records_on_throw() {
    let output = run(r#"
        const p = require('perf_hooks').performance;
        const tf = p.timerify(function g74Boom() { throw new Error('boom'); });
        let caught = 'no';
        try { tf(); } catch (err) { caught = err.message; }
        const entries = p.getEntriesByType('function').filter(e => e.name === 'g74Boom');
        [caught, entries.length >= 1, entries[entries.length - 1].entryType].join('|');
        "#);
    assert_eq!(output, "boom|true|function");
}

#[test]
#[serial]
fn g74_observer_take_records_sync_buffer() {
    let output = run(r#"
        const ph = require('perf_hooks');
        const p = ph.performance;
        const obs = new ph.PerformanceObserver(() => {});
        obs.observe({ entryTypes: ['mark'] });
        p.mark('g74-sync-m');
        const r = obs.takeRecords();
        const r2 = obs.takeRecords();
        const e = r[r.length - 1];
        [
          r.length >= 1, e.name, e.entryType, r2.length === 0,
          Array.isArray(r)
        ].join('|');
        "#);
    assert_eq!(output, "true|g74-sync-m|mark|true|true");
}

#[test]
#[serial]
fn g74_observer_async_callback_delivery() {
    let output = run(r#"
        const ph = require('perf_hooks');
        let got = 'none'; let selfOk = false; let byType = 0;
        const obs = new ph.PerformanceObserver((list, observer) => {
          const entries = list.getEntries();
          const hit = entries.find(e => e.name === 'g74-async-m');
          got = hit ? hit.entryType : 'missing';
          byType = list.getEntriesByType('mark').filter(e => e.name === 'g74-async-m').length;
          selfOk = observer === obs;
        });
        obs.observe({ entryTypes: ['mark'] });
        ph.performance.mark('g74-async-m');
        // Observer delivery rides the microtask queue (like Node's async
        // notification): it is observable once microtasks drain — here via a
        // promise resolved one turn later.
        new Promise((resolve) => queueMicrotask(() => resolve([got, byType, selfOk].join('|'))));
        "#);
    assert_eq!(output, "mark|1|true");
}

#[test]
#[serial]
fn g74_observer_take_records_prevents_callback() {
    let output = run(r#"
        const ph = require('perf_hooks');
        let calls = 0;
        const obs = new ph.PerformanceObserver(() => { calls++; });
        obs.observe({ entryTypes: ['mark'] });
        ph.performance.mark('g74-drained-m');
        const drained = obs.takeRecords().length;
        // The queued flush sees an empty buffer, so the callback stays
        // uncalled (Node semantics: takeRecords consumes the records).
        new Promise((resolve) => queueMicrotask(() => resolve([drained >= 1, calls].join('|'))));
        "#);
    assert_eq!(output, "true|0");
}

#[test]
#[serial]
fn g74_observer_disconnect_and_validation() {
    let output = run(r#"
        const ph = require('perf_hooks');
        const p = ph.performance;
        let calls = 0;
        const obs = new ph.PerformanceObserver(() => { calls++; });
        obs.observe({ entryTypes: ['mark'] });
        obs.disconnect();
        p.mark('g74-disc-m');
        const afterDisc = obs.takeRecords().length;
        const errs = [];
        try { new ph.PerformanceObserver(); } catch (e) { errs.push('ctor'); }
        try { new ph.PerformanceObserver(() => {}).observe(); } catch (e) { errs.push('noarg'); }
        try { new ph.PerformanceObserver(() => {}).observe({ entryTypes: [] }); } catch (e) { errs.push('empty'); }
        try { new ph.PerformanceObserver(() => {}).observe({ entryTypes: ['bogus'] }); } catch (e) { errs.push('bad'); }
        [calls, afterDisc, errs.join(',')].join('|');
        "#);
    assert_eq!(output, "0|0|ctor,noarg,empty,bad");
}

#[test]
#[serial]
fn g74_observer_type_filter_and_timerify_delivery() {
    let output = run(r#"
        const ph = require('perf_hooks');
        const p = ph.performance;
        let marks = 0; let funcs = 0;
        const obsM = new ph.PerformanceObserver(() => { marks++; });
        const obsF = new ph.PerformanceObserver((list) => {
          funcs += list.getEntriesByType('function').filter(e => e.name === 'g74Tf').length;
        });
        obsM.observe({ entryTypes: ['measure'] });
        obsF.observe({ entryTypes: ['function'] });
        p.mark('g74-filtered-m');
        p.timerify(function g74Tf() { return 7; })();
        new Promise((resolve) => queueMicrotask(() => resolve([marks, funcs].join('|'))));
        "#);
    assert_eq!(output, "0|1");
}

#[test]
#[serial]
fn g74_function_entries_bounded() {
    // Regression for the review finding: timerified calls must not grow the
    // process-global entry store without bound — it is capped at 10_000
    // (oldest dropped), declared as a Limit.
    let output = run(r#"
        const p = require('perf_hooks').performance;
        const tf = p.timerify(function g74Cap() { return 1; });
        for (let i = 0; i < 10005; i++) tf();
        const fns = p.getEntriesByType('function');
        [fns.length <= 10000, fns[fns.length - 1].name].join('|');
        "#);
    assert_eq!(output, "true|g74Cap");
}

#[test]
#[serial]
fn g74_batched_records_deliver_together() {
    // Regression for the review finding: a synchronous batch of entries is
    // delivered by the already-queued flush — the callback still sees every
    // record, batched into one delivery (not one microtask per entry).
    let output = run(r#"
        const ph = require('perf_hooks');
        let calls = 0; let seen = 0;
        const obs = new ph.PerformanceObserver((list) => {
          calls++;
          seen += list.getEntries().filter(e => e.name.startsWith('g74-b')).length;
        });
        obs.observe({ entryTypes: ['mark'] });
        ph.performance.mark('g74-b1');
        ph.performance.mark('g74-b2');
        ph.performance.mark('g74-b3');
        new Promise((resolve) => queueMicrotask(() => resolve([calls, seen].join('|'))));
        "#);
    assert_eq!(output, "1|3");
}

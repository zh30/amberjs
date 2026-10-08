//! Pins docs/NODE_EVENTS_CONTRACT.md. Distinct from web EventTarget (G12).
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

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
fn require_events_is_the_event_emitter_constructor() {
    let out = run(r#"
        const events = require('events');
        const { EventEmitter } = require('events');
        const nodeEvents = require('node:events');
        [
          typeof events,
          events === EventEmitter,
          events.EventEmitter === events,
          nodeEvents === events,
          typeof new events().on
        ].join('|');
        "#);
    assert_eq!(out, "function|true|true|true|function");
}

#[test]
#[serial]
fn on_emit_once_off_and_listener_count() {
    let out = run(r#"
        const EventEmitter = require('events');
        const ee = new EventEmitter();
        const seen = [];
        const a = (v) => seen.push('a' + v);
        const b = (v) => seen.push('b' + v);
        ee.on('ping', a);
        ee.once('ping', b);
        ee.emit('ping', 1);
        ee.emit('ping', 2);
        ee.off('ping', a);
        ee.emit('ping', 3);
        [
          seen.join(','),
          ee.listenerCount('ping'),
          EventEmitter.listenerCount(ee, 'ping'),
          ee.emit('missing')
        ].join('|');
        "#);
    assert_eq!(out, "a1,b1,a2|0|0|false");
}

#[test]
#[serial]
fn prepend_listener_order_and_event_names() {
    let out = run(r#"
        const EventEmitter = require('events');
        const ee = new EventEmitter();
        const order = [];
        ee.on('foo', () => order.push('second'));
        ee.prependListener('foo', () => order.push('first'));
        ee.prependOnceListener('foo', () => order.push('once'));
        ee.emit('foo');
        ee.emit('foo');
        [
          order.join(','),
          ee.eventNames().includes('foo'),
          ee.listeners('foo').length,
          ee.rawListeners('foo').length
        ].join('|');
        "#);
    assert_eq!(out, "once,first,second,first,second|true|2|2");
}

#[test]
#[serial]
fn error_without_listener_throws_and_max_listeners_warn() {
    let out = run(r#"
        const EventEmitter = require('events');
        const ee = new EventEmitter();
        ee.setMaxListeners(1);
        let warn = '';
        const orig = console.warn;
        console.warn = (msg) => { warn = String(msg); };
        ee.on('x', () => {});
        ee.on('x', () => {});
        console.warn = orig;
        let errKind = '';
        try {
          ee.emit('error', new Error('boom'));
          errKind = 'ok';
        } catch (e) {
          errKind = e instanceof Error && e.message === 'boom' ? 'error' : 'other';
        }
        let unhandled = '';
        try {
          ee.emit('error', 'nope');
          unhandled = 'ok';
        } catch (e) {
          unhandled = e.message.startsWith('Unhandled error.') && e.context === 'nope'
            ? 'unhandled'
            : 'other';
        }
        [
          warn.includes('MaxListenersExceededWarning'),
          ee.getMaxListeners(),
          EventEmitter.defaultMaxListeners,
          errKind,
          unhandled
        ].join('|');
        "#);
    assert_eq!(out, "true|1|10|error|unhandled");
}

#[test]
#[serial]
fn event_emitter_once_promise_and_abort_signal() {
    let out = run(r#"
        (async () => {
          const events = require('events');
          const ee = new events();
          const p = events.once(ee, 'ping');
          ee.emit('ping', 7, 'z');
          const args = await p;
          const ctrl = new AbortController();
          ctrl.abort();
          let aborted = '';
          try {
            await events.once(ee, 'later', { signal: ctrl.signal });
            aborted = 'ok';
          } catch (e) {
            aborted = e.message === 'This operation was aborted' ? 'aborted' : 'other';
          }
          return [args.join(','), aborted, args instanceof Array].join('|');
        })()
        "#);
    assert_eq!(out, "7,z|aborted|true");
}

#[test]
#[serial]
fn esm_named_and_default_export_event_emitter() {
    let dir = TempDir::new().expect("temp dir");
    let main_path = dir.path().join("main.mjs");
    let mut rt = runtime();
    rt.set_main_module_path(&main_path);
    rt.execute_code(
        r#"
        import eventsDefault, { EventEmitter } from 'events';
        import nodeDefault, { EventEmitter as NodeEE } from 'node:events';
        export const forceNativeModule = true;
        const ee = new EventEmitter();
        let hit = 0;
        ee.on('t', () => { hit = 1; });
        ee.emit('t');
        globalThis.__nodeEventsEsm = [
          typeof EventEmitter,
          EventEmitter === eventsDefault,
          EventEmitter === NodeEE,
          EventEmitter === nodeDefault,
          hit
        ].join('|');
        "#,
    )
    .expect("esm execute");
    let result = rt
        .execute_code("globalThis.__nodeEventsEsm")
        .expect("read esm result");
    assert_eq!(result.trim(), "function|true|true|true|1");
}

#[test]
#[serial]
fn cli_require_events_reaches_the_contract() {
    let binary = env!("CARGO_BIN_EXE_amber");
    let dir = TempDir::new().expect("temp dir");
    let script = dir.path().join("probe.js");
    fs::write(
        &script,
        r#"
        const EventEmitter = require('events');
        const ee = new EventEmitter();
        let n = 0;
        ee.on('x', (v) => { n += v; });
        ee.emit('x', 4);
        console.log([
          EventEmitter.EventEmitter === EventEmitter,
          n,
          typeof EventEmitter.once
        ].join('|'));
        "#,
    )
    .expect("script");
    let output = Command::new(binary)
        .arg("run")
        .arg(&script)
        .output()
        .expect("amber run");
    assert!(
        output.status.success(),
        "amber run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "true|4|function"
    );
}

#[test]
#[serial]
fn not_web_event_target() {
    let out = run(r#"
        const EventEmitter = require('events');
        const ee = new EventEmitter();
        [
          typeof ee.addEventListener,
          typeof ee.dispatchEvent,
          typeof EventTarget,
          ee instanceof (typeof EventTarget === 'function' ? EventTarget : Object)
        ].join('|');
        "#);
    // EventTarget exists (G12) but EventEmitter is not that API.
    assert_eq!(out, "undefined|undefined|function|false");
}

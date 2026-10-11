//! Pins docs/DIAGNOSTICS_CHANNEL_CONTRACT.md (G56).
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn run(script: &str) -> String {
    let mut runtime = MinimalRuntime::new().expect("runtime");
    runtime
        .execute_code(script)
        .expect("diagnostics_channel contract script")
        .trim()
        .to_string()
}

#[test]
#[serial]
fn require_exposes_channel_and_tracing_channel() {
    let output = run(r#"
        const dc = require('diagnostics_channel');
        const node = require('node:diagnostics_channel');
        [
          dc === node,
          typeof dc.channel,
          typeof dc.hasSubscribers,
          typeof dc.subscribe,
          typeof dc.unsubscribe,
          typeof dc.tracingChannel,
          typeof dc.Channel,
          typeof dc.TracingChannel
        ].join('|');
        "#);
    assert_eq!(
        output,
        "true|function|function|function|function|function|function|function"
    );
}

#[test]
#[serial]
fn channel_pubsub_same_name_identity() {
    let output = run(r#"
        const dc = require('diagnostics_channel');
        const a = dc.channel('g56.contract');
        const b = dc.channel('g56.contract');
        let received = null;
        function sub(msg, name) { received = { msg, name }; }
        a.subscribe(sub);
        [
          a === b,
          a.name,
          a.hasSubscribers,
          dc.hasSubscribers('g56.contract'),
          (a.publish({ k: 1 }), JSON.stringify(received)),
          (a.unsubscribe(sub), a.hasSubscribers),
          dc.hasSubscribers('g56.contract')
        ].join('|');
        "#);
    assert_eq!(
        output,
        r#"true|g56.contract|true|true|{"msg":{"k":1},"name":"g56.contract"}|false|false"#
    );
}

#[test]
#[serial]
fn channel_name_type_error() {
    let output = run(r#"
        const dc = require('diagnostics_channel');
        try {
          dc.channel(1);
          'ok';
        } catch (e) {
          [
            e instanceof TypeError,
            String(e.message).includes('string or symbol')
          ].join('|');
        }
        "#);
    assert_eq!(output, "true|true");
}

#[test]
#[serial]
fn tracing_channel_trace_sync_success() {
    let output = run(r#"
        const dc = require('diagnostics_channel');
        const tc = dc.tracingChannel('g56.trace');
        const events = [];
        tc.start.subscribe((ctx) => events.push('start:' + (ctx.tag || '')));
        tc.end.subscribe((ctx) => events.push('end:' + ctx.result));
        tc.error.subscribe(() => events.push('error'));
        const res = tc.traceSync((x, y) => x + y, { tag: 'ok' }, null, 10, 20);
        [res, events.join(',')].join('|');
        "#);
    assert_eq!(output, "30|start:ok,end:30");
}

#[test]
#[serial]
fn tracing_channel_trace_sync_error_publishes_error_and_end() {
    let output = run(r#"
        const dc = require('diagnostics_channel');
        const tc = dc.tracingChannel('g56.err');
        const events = [];
        tc.start.subscribe(() => events.push('start'));
        tc.end.subscribe((ctx) => events.push('end:' + !!(ctx.error)));
        tc.error.subscribe((ctx) => events.push('error:' + ctx.error.message));
        let threw = false;
        try {
          tc.traceSync(() => { throw new Error('boom'); }, {});
        } catch (e) {
          threw = e.message === 'boom';
        }
        [threw, events.join(',')].join('|');
        "#);
    assert_eq!(output, "true|start,error:boom,end:true");
}

#[test]
#[serial]
fn tracing_channel_wires_named_channels() {
    let output = run(r#"
        const dc = require('diagnostics_channel');
        const tc = dc.tracingChannel('g56.names');
        [
          tc.start === dc.channel('g56.names:start'),
          tc.end === dc.channel('g56.names:end'),
          tc.error === dc.channel('g56.names:error'),
          typeof tc.traceSync
        ].join('|');
        "#);
    assert_eq!(output, "true|true|true|function");
}

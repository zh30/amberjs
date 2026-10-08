//! Pins docs/NODE_STREAM_CONTRACT.md — narrow Node `require('stream')` Stable surface.
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn eval(code: &str) -> String {
    let mut runtime = MinimalRuntime::new().expect("runtime");
    runtime
        .execute_code(code)
        .unwrap_or_else(|error| panic!("{code} should evaluate: {error}"))
        .trim()
        .to_string()
}

#[test]
#[serial]
fn require_stream_and_node_stream_reach_same_object() {
    assert_eq!(
        eval(
            r#"
            const a = require('stream');
            const b = require('node:stream');
            [
              a === b,
              a === globalThis.stream,
              typeof a.Readable,
              typeof a.Writable,
              typeof a.PassThrough,
              a.PassThrough === a.passThrough
            ].join('|');
            "#
        ),
        "true|true|function|function|function|true"
    );
}

#[test]
#[serial]
fn readable_push_on_data_and_end() {
    assert_eq!(
        eval(
            r#"
            const { Readable } = require('stream');
            let out = '';
            let ended = false;
            const r = new Readable({ read() {} });
            r.on('data', (c) => { out += c; });
            r.on('end', () => { ended = true; });
            r.push('hello');
            r.push('world');
            r.push(null);
            [out, ended].join('|');
            "#
        ),
        "helloworld|true"
    );
}

#[test]
#[serial]
fn readable_pipe_to_writable_flows_and_finishes() {
    assert_eq!(
        eval(
            r#"
            const { Readable, Writable } = require('stream');
            let output = '';
            let finished = false;
            const r = new Readable({
              read() {
                this.push('hello');
                this.push('world');
                this.push(null);
              }
            });
            const w = new Writable({
              _write(chunk, encoding, callback) {
                output += chunk;
                callback();
              }
            });
            w.on('finish', () => { finished = true; });
            const dest = r.pipe(w);
            [dest === w, output, finished].join('|');
            "#
        ),
        "true|helloworld|true"
    );
}

#[test]
#[serial]
fn writable_write_returns_boolean_and_end_fires_finish() {
    assert_eq!(
        eval(
            r#"
            const { Writable } = require('stream');
            let got = '';
            let finished = false;
            const w = new Writable({
              write(chunk, encoding, callback) {
                got += chunk;
                callback();
              }
            });
            w.on('finish', () => { finished = true; });
            const ret = w.write('abc');
            w.end();
            [typeof ret, ret === true, got, finished].join('|');
            "#
        ),
        "boolean|true|abc|true"
    );
}

#[test]
#[serial]
fn passthrough_write_emits_data_and_pipes() {
    assert_eq!(
        eval(
            r#"
            const stream = require('stream');
            let direct = '';
            let piped = '';
            const pt = new stream.PassThrough();
            pt.on('data', (c) => { direct += c; });
            const w = new stream.Writable({
              _write(chunk, encoding, callback) {
                piped += chunk;
                callback();
              }
            });
            const dest = pt.pipe(w);
            pt.write('x');
            pt.write('y');
            [dest === w, direct, piped].join('|');
            "#
        ),
        "true|xy|xy"
    );
}

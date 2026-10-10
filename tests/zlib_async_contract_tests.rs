//! Pins docs/ZLIB_ASYNC_CONTRACT.md. Async gzip/gunzip via process.nextTick.
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
fn require_zlib_exposes_async_gzip_gunzip() {
    let output = run(r#"
        const zlib = require('zlib');
        const node = require('node:zlib');
        [
          typeof zlib.gzip,
          typeof zlib.gunzip,
          zlib.gzip === node.gzip,
          zlib.gunzip === node.gunzip,
          typeof zlib.createGzip,
          typeof zlib.constants,
          typeof zlib.deflate
        ].join('|');
        "#);
    assert_eq!(
        output,
        "function|function|true|true|undefined|undefined|undefined"
    );
}

#[test]
#[serial]
fn gzip_gunzip_callback_round_trip_on_later_turn() {
    let output = run(r#"
        const zlib = require('zlib');
        const input = 'Amber async zlib gzip/gunzip contract pin';
        let phase = 'sync';
        new Promise((resolve, reject) => {
          const ret = zlib.gzip(input, (err, compressed) => {
            if (err) return reject(err);
            zlib.gunzip(compressed, (err2, decompressed) => {
              if (err2) return reject(err2);
              resolve([
                phase,
                Buffer.isBuffer(compressed),
                Buffer.isBuffer(decompressed),
                compressed[0],
                compressed[1],
                decompressed.toString('utf8') === input
              ].join('|'));
            });
          });
          if (ret !== undefined) reject(new Error('expected undefined return'));
          phase = 'after-call';
        });
        "#);
    assert_eq!(output, "after-call|true|true|31|139|true");
}

#[test]
#[serial]
fn accepts_buffer_typed_array_and_array_buffer() {
    let output = run(r#"
        const zlib = require('zlib');
        const text = 'bytes';
        function once(fn, input) {
          return new Promise((resolve, reject) => {
            fn(input, (err, out) => err ? reject(err) : resolve(out));
          });
        }
        (async () => {
          const fromBuf = await once(zlib.gunzip, await once(zlib.gzip, Buffer.from(text)));
          const u8 = new Uint8Array([98, 121, 116, 101, 115]);
          const fromU8 = await once(zlib.gunzip, await once(zlib.gzip, u8));
          const ab = u8.buffer.slice(u8.byteOffset, u8.byteOffset + u8.byteLength);
          const fromAb = await once(zlib.gunzip, await once(zlib.gzip, ab));
          return [fromBuf.toString('utf8'), fromU8.toString('utf8'), fromAb.toString('utf8')].join('|');
        })();
        "#);
    assert_eq!(output, "bytes|bytes|bytes");
}

#[test]
#[serial]
fn invalid_input_and_callback_throw_synchronously() {
    let output = run(r#"
        const zlib = require('zlib');
        function check(fn, needle) {
          try { fn(); return 'ok'; }
          catch (e) {
            return (e instanceof TypeError) && String(e.message).includes(needle);
          }
        }
        [
          check(() => zlib.gzip(null, () => {}), 'invalid input'),
          check(() => zlib.gunzip(null, () => {}), 'invalid input'),
          check(() => zlib.gzip('x'), 'options'),
          check(() => zlib.gzip('x', {}), 'callback'),
          check(() => zlib.gzip('x', () => {}, { level: 1 }), 'options')
        ].join('|');
        "#);
    assert_eq!(output, "true|true|true|true|true");
}

#[test]
#[serial]
fn corrupt_gunzip_delivers_error_on_callback() {
    let output = run(r#"
        const zlib = require('zlib');
        const bad = Buffer.from([0x00, 0x01, 0x02, 0x03]);
        new Promise((resolve) => {
          zlib.gunzip(bad, (err, data) => {
            resolve([
              err instanceof Error,
              err instanceof TypeError,
              String(err && err.message).includes('failed'),
              data === undefined
            ].join('|'));
          });
        });
        "#);
    assert_eq!(output, "true|false|true|true");
}

#[test]
#[serial]
fn empty_gzip_round_trip() {
    let output = run(r#"
        const zlib = require('zlib');
        new Promise((resolve, reject) => {
          zlib.gzip('', (err, compressed) => {
            if (err) return reject(err);
            zlib.gunzip(compressed, (err2, out) => {
              if (err2) return reject(err2);
              resolve([
                compressed[0] === 0x1f,
                compressed[1] === 0x8b,
                out.toString('utf8') === ''
              ].join('|'));
            });
          });
        });
        "#);
    assert_eq!(output, "true|true|true");
}

#[test]
#[serial]
fn g13_and_g21_remain_distinct() {
    let output = run(r#"
        const zlib = require('zlib');
        [
          typeof CompressionStream,
          typeof zlib.gzipSync,
          typeof zlib.gzip,
          zlib === CompressionStream
        ].join('|');
        "#);
    assert_eq!(output, "function|function|function|false");
}

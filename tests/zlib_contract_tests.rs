//! Pins docs/ZLIB_CONTRACT.md. Sync-only Node zlib; not Web CompressionStream (G13).
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
fn require_zlib_and_node_zlib_expose_sync_methods() {
    let output = run(
        r#"
        const a = require('zlib');
        const b = require('node:zlib');
        const names = ['gzipSync','gunzipSync','deflateSync','inflateSync','deflateRawSync','inflateRawSync'];
        const same = a === b || (a && b && names.every((n) => a[n] === b[n]));
        names.map((n) => typeof a[n]).concat([same, typeof a.createGzip, typeof a.constants]).join('|');
        "#,
    );
    assert_eq!(
        output,
        "function|function|function|function|function|function|true|undefined|undefined"
    );
}

#[test]
#[serial]
fn gzip_gunzip_round_trip_returns_buffer_with_gzip_magic() {
    let output = run(
        r#"
        const zlib = require('zlib');
        const input = 'Amber deterministic engine & zlib compression test string!';
        const compressed = zlib.gzipSync(input);
        const decompressed = zlib.gunzipSync(compressed);
        [
          Buffer.isBuffer(compressed),
          Buffer.isBuffer(decompressed),
          compressed[0],
          compressed[1],
          compressed.length > 0,
          decompressed.toString('utf8') === input
        ].join('|');
        "#,
    );
    assert_eq!(output, "true|true|31|139|true|true");
}

#[test]
#[serial]
fn deflate_and_raw_round_trips() {
    let output = run(
        r#"
        const zlib = require('zlib');
        const input = 'deflate and raw deflate round trip';
        const z = zlib.deflateSync(input);
        const r = zlib.deflateRawSync(input);
        const emptyGzip = zlib.gzipSync('');
        [
          Buffer.isBuffer(z),
          Buffer.isBuffer(r),
          zlib.inflateSync(z).toString('utf8') === input,
          zlib.inflateRawSync(r).toString('utf8') === input,
          emptyGzip[0] === 0x1f && emptyGzip[1] === 0x8b,
          zlib.gunzipSync(emptyGzip).toString('utf8') === ''
        ].join('|');
        "#,
    );
    assert_eq!(output, "true|true|true|true|true|true");
}

#[test]
#[serial]
fn accepts_buffer_typed_array_and_array_buffer() {
    let output = run(
        r#"
        const zlib = require('zlib');
        const text = 'bytes';
        const fromBuf = zlib.gunzipSync(zlib.gzipSync(Buffer.from(text))).toString('utf8');
        const u8 = new Uint8Array([98, 121, 116, 101, 115]);
        const fromU8 = zlib.gunzipSync(zlib.gzipSync(u8)).toString('utf8');
        const ab = u8.buffer.slice(u8.byteOffset, u8.byteOffset + u8.byteLength);
        const fromAb = zlib.gunzipSync(zlib.gzipSync(ab)).toString('utf8');
        [fromBuf, fromU8, fromAb].join('|');
        "#,
    );
    assert_eq!(output, "bytes|bytes|bytes");
}

#[test]
#[serial]
fn invalid_input_throws_type_error_for_all_sync_methods() {
    let output = run(
        r#"
        const zlib = require('zlib');
        const methods = ['gzipSync','gunzipSync','deflateSync','inflateSync','deflateRawSync','inflateRawSync'];
        methods.map((name) => {
          try {
            zlib[name](null);
            return 'ok';
          } catch (e) {
            return (e instanceof TypeError) + ':' + String(e.message).includes('invalid input');
          }
        }).join('|');
        "#,
    );
    assert_eq!(
        output,
        "true:true|true:true|true:true|true:true|true:true|true:true"
    );
}

#[test]
#[serial]
fn corrupt_payload_throws_error() {
    let output = run(
        r#"
        const zlib = require('zlib');
        const bad = Buffer.from([0x00, 0x01, 0x02, 0x03]);
        function check(fn, label) {
          try {
            fn(bad);
            return label + ':ok';
          } catch (e) {
            return label + ':' + (e instanceof TypeError) + ':' + (e instanceof Error) + ':' + String(e.message).includes('failed');
          }
        }
        [
          check(zlib.gunzipSync, 'gunzip'),
          check(zlib.inflateSync, 'inflate'),
          check(zlib.inflateRawSync, 'inflateRaw')
        ].join('|');
        "#,
    );
    assert_eq!(
        output,
        "gunzip:false:true:true|inflate:false:true:true|inflateRaw:false:true:true"
    );
}

#[test]
#[serial]
fn g13_compression_stream_is_not_this_contract() {
    let output = run(
        r#"
        const zlib = require('zlib');
        const web = typeof CompressionStream;
        const nodeGzip = typeof zlib.gzipSync;
        // Web CompressionStream must remain available, but it is not require('zlib').
        [web, nodeGzip, zlib === CompressionStream].join('|');
        "#,
    );
    assert_eq!(output, "function|function|false");
}

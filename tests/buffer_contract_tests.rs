//! Pins docs/BUFFER_CONTRACT.md (setup_buffer_module in runtime_minimal).
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
fn require_buffer_is_global_buffer() {
    let out = run(r#"
        const mod = require('buffer');
        const node = require('node:buffer');
        [
          typeof Buffer === 'function',
          mod.Buffer === Buffer,
          node.Buffer === Buffer,
          mod.default && mod.default.Buffer === Buffer,
          Buffer.poolSize === 8192
        ].join('|');
        "#);
    assert_eq!(out, "true|true|true|true|true");
}

#[test]
#[serial]
fn from_string_encodings_and_to_string() {
    let out = run(r#"
        const utf = Buffer.from('hello', 'utf8');
        const hex = Buffer.from('414243', 'hex');
        const b64 = Buffer.from('YWJj', 'base64');
        // latin1/ascii/binary encode UTF-8 bytes after V8→Rust (ASCII stays 1:1).
        const latin = Buffer.from('AB', 'latin1');
        const high = Buffer.alloc(2);
        high[0] = 0x41;
        high[1] = 0xff;
        [
          utf.length,
          utf.toString('utf8'),
          hex.toString('utf8'),
          b64.toString('utf8'),
          latin.length,
          latin[0],
          latin[1],
          high.toString('latin1').charCodeAt(1),
          utf.toString('hex'),
          Buffer.from('hi').toString('utf8', 0, 1)
        ].join('|');
        "#);
    assert_eq!(out, "5|hello|ABC|abc|2|65|66|255|68656c6c6f|h");
}

#[test]
#[serial]
fn from_array_typed_array_and_array_buffer() {
    let out = run(r#"
        const arr = Buffer.from([0x41, 0x42, 0xff]);
        const u8 = new Uint8Array([1, 2, 3, 4]);
        const copied = Buffer.from(u8);
        u8[0] = 9;
        const ab = new ArrayBuffer(4);
        const view = new Uint8Array(ab);
        view[0] = 7; view[1] = 8; view[2] = 9; view[3] = 10;
        // FastBuffer.from forwards two args: (ab, byteOffset); length is ignored.
        const shared = Buffer.from(ab, 1);
        shared[0] = 55;
        [
          arr.length, arr[0], arr[1], arr[2],
          copied[0], copied[1],
          shared.length, shared[0], view[1]
        ].join('|');
        "#);
    assert_eq!(out, "3|65|66|255|1|2|3|55|55");
}

#[test]
#[serial]
fn alloc_alloc_unsafe_and_write() {
    let out = run(r#"
        const z = Buffer.alloc(4);
        const f = Buffer.alloc(4, 0xaa);
        const s = Buffer.alloc(6, 'AB', 'utf8');
        const u = Buffer.allocUnsafe(3);
        u.write('xy', 0, 'utf8');
        const w = Buffer.alloc(8);
        const n = w.write('Hello', 1, 4, 'utf8');
        [
          z[0], z[1], z[2], z[3],
          f[0], f[3],
          s.toString('utf8'),
          u.length, u.toString('utf8', 0, 2),
          n, w.toString('utf8', 1, 5)
        ].join('|');
        "#);
    assert_eq!(out, "0|0|0|0|170|170|ABABAB|3|xy|4|Hell");
}

#[test]
#[serial]
fn concat_byte_length_is_buffer_slice() {
    let out = run(r#"
        const a = Buffer.from('Hello');
        const b = Buffer.from('World');
        const c = Buffer.concat([a, b]);
        const trunc = Buffer.concat([a, b], 7);
        const base = Buffer.from('abcdef');
        const sl = base.slice(2, 5);
        sl[0] = 90; // 'Z' — shared memory with base
        const isBuf = Buffer.isBuffer(a);
        const isU8 = Buffer.isBuffer(new Uint8Array(1));
        const notBuf = Buffer.isBuffer('x');
        [
          c.toString('utf8'),
          trunc.toString('utf8'),
          Buffer.byteLength('hello', 'utf8'),
          Buffer.byteLength('4142', 'hex'),
          sl.toString('utf8'),
          base.toString('utf8'),
          isBuf, isU8, notBuf,
          a instanceof Buffer
        ].join('|');
        "#);
    assert_eq!(
        out,
        "HelloWorld|HelloWo|5|2|Zde|abZdef|true|true|false|true"
    );
}

#[test]
#[serial]
fn pool_size_and_alloc_unsafe_small_uses_pool() {
    let out = run(r#"
        // Two small allocUnsafe calls should share one ArrayBuffer when under poolSize/2.
        const a = Buffer.allocUnsafe(16);
        const b = Buffer.allocUnsafe(16);
        const samePool = a.buffer === b.buffer && a.byteOffset !== b.byteOffset;
        const big = Buffer.allocUnsafe(5000);
        const slow = Buffer.allocUnsafeSlow(16);
        [
          Buffer.poolSize,
          samePool,
          big.length === 5000,
          slow.length === 16,
          typeof Buffer.from === 'function',
          typeof Buffer.concat === 'function'
        ].join('|');
        "#);
    assert_eq!(out, "8192|true|true|true|true|true");
}

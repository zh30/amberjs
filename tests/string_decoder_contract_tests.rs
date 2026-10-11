//! Pins docs/STRING_DECODER_CONTRACT.md.
//! G39: UTF-8 StringDecoder write/end. G68: latin1/ascii/base64/hex/utf16le.
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::process::Command;

fn runtime() -> MinimalRuntime {
    MinimalRuntime::new().expect("runtime")
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
fn require_and_global_expose_same_string_decoder() {
    let out = run(r#"
        const a = require('string_decoder');
        const b = require('node:string_decoder');
        [
          a === b,
          a === string_decoder,
          typeof a.StringDecoder,
          a.default === a.StringDecoder
        ].join('|');
        "#);
    assert_eq!(out, "true|true|function|true");
}

#[test]
#[serial]
fn default_encoding_is_utf8_and_utf_8_normalizes() {
    let out = run(r#"
        const { StringDecoder } = require('string_decoder');
        const d1 = new StringDecoder();
        const d2 = new StringDecoder('utf-8');
        const d3 = new StringDecoder('utf8');
        [d1._encoding, d2._encoding, d3._encoding].join('|');
        "#);
    assert_eq!(out, "utf8|utf8|utf8");
}

#[test]
#[serial]
fn string_passthrough_and_buffer_write() {
    let out = run(r#"
        const { StringDecoder } = require('string_decoder');
        const d = new StringDecoder('utf8');
        const fromString = d.write('hello');
        const fromBuf = new StringDecoder('utf8').write(Buffer.from('world'));
        const fromU8 = new StringDecoder('utf8').write(new Uint8Array([0x61, 0x62]));
        const fromArr = new StringDecoder('utf8').write([0x63, 0x64]);
        const bad = new StringDecoder('utf8').write(null);
        [fromString, fromBuf, fromU8, fromArr, bad].join('|');
        "#);
    assert_eq!(out, "hello|world|ab|cd|");
}

#[test]
#[serial]
fn incomplete_utf8_multibyte_buffers_across_writes() {
    // Split 你 (E4 BD A0) then 好 (E5 A5 BD) — same shape as conformance fixture.
    let out = run(r#"
        const { StringDecoder } = require('string_decoder');
        const decoder = new StringDecoder('utf8');
        const part1 = decoder.write(Buffer.from([0xE4, 0xBD]));
        const part2 = decoder.write(Buffer.from([0xA0, 0xE5, 0xA5, 0xBD]));
        const endPart = decoder.end();
        [part1, part2, endPart, part1 + part2 + endPart].join('|');
        "#);
    assert_eq!(out, "|你好||你好");
}

#[test]
#[serial]
fn end_flushes_held_incomplete_bytes() {
    let out = run(r#"
        const { StringDecoder } = require('string_decoder');
        const decoder = new StringDecoder('utf8');
        const held = decoder.write(Buffer.from([0xE4, 0xBD]));
        const flushed = decoder.end(Buffer.from([0xA0]));
        [held, flushed].join('|');
        "#);
    assert_eq!(out, "|你");
}

#[test]
#[serial]
fn amber_cli_require_string_decoder_split_utf8() {
    let script = r#"
        const { StringDecoder } = require('string_decoder');
        const decoder = new StringDecoder('utf8');
        const part1 = decoder.write(Buffer.from([0xE4, 0xBD]));
        const part2 = decoder.write(Buffer.from([0xA0, 0xE5, 0xA5, 0xBD]));
        const endPart = decoder.end();
        console.log(part1 + part2 + endPart);
    "#;
    let output = Command::new(env!("CARGO_BIN_EXE_amber"))
        .args(["eval", script])
        .output()
        .expect("amber eval");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "amber eval string_decoder failed: {combined}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "你好",
        "CLI UTF-8 split decode: {combined}"
    );
}

#[test]
#[serial]
fn encoding_aliases_normalize() {
    let out = run(r#"
        const { StringDecoder } = require('string_decoder');
        const a = new StringDecoder('binary')._encoding;
        const b = new StringDecoder('ucs2')._encoding;
        const c = new StringDecoder('utf-16le')._encoding;
        [a, b, c].join('|');
        "#);
    assert_eq!(out, "latin1|utf16le|utf16le");
}

#[test]
#[serial]
fn unknown_encoding_throws() {
    let out = run(r#"
        const { StringDecoder } = require('string_decoder');
        try {
          new StringDecoder('nope');
          'no-throw';
        } catch (e) {
          [e instanceof TypeError, e.code, String(e.message).includes('Unknown encoding')].join('|');
        }
        "#);
    assert_eq!(out, "true|ERR_UNKNOWN_ENCODING|true");
}

#[test]
#[serial]
fn latin1_and_ascii_byte_decode() {
    let out = run(r#"
        const { StringDecoder } = require('string_decoder');
        const lat = new StringDecoder('latin1').write(Buffer.from([0xff, 0x00, 0x41]));
        const asc = new StringDecoder('ascii').write(Buffer.from([0xff, 0x00, 0x41]));
        const bin = new StringDecoder('binary').write(Buffer.from([0xc9]));
        [lat.charCodeAt(0), lat.charCodeAt(1), lat.charCodeAt(2),
         asc.charCodeAt(0), asc.charCodeAt(1), asc.charCodeAt(2),
         bin].join('|');
        "#);
    assert_eq!(out, "255|0|65|127|0|65|É");
}

#[test]
#[serial]
fn hex_encodes_each_byte() {
    let out = run(r#"
        const { StringDecoder } = require('string_decoder');
        const d = new StringDecoder('hex');
        const a = d.write(Buffer.from([0xde]));
        const b = d.write(Buffer.from([0xad, 0xbe, 0xef]));
        const e = d.end();
        [a, b, e, a + b + e].join('|');
        "#);
    assert_eq!(out, "de|adbeef||deadbeef");
}

#[test]
#[serial]
fn base64_buffers_incomplete_triplets() {
    let out = run(r#"
        const { StringDecoder } = require('string_decoder');
        const d = new StringDecoder('base64');
        const p1 = d.write(Buffer.from([0x4d]));
        const p2 = d.write(Buffer.from([0x61]));
        const p3 = d.write(Buffer.from([0x6e]));
        const e = d.end();
        const d2 = new StringDecoder('base64');
        const hold = d2.write(Buffer.from([0x4d, 0x61]));
        const pad = d2.end();
        [p1, p2, p3, e, hold, pad].join('|');
        "#);
    // write([0x4d]) / write([0x61]) hold; write([0x6e]) emits TWFu; end() empty.
    // second decoder: write([0x4d,0x61]) holds; end() pads TWE=.
    assert_eq!(out, "||TWFu|||TWE=");
}

#[test]
#[serial]
fn utf16le_buffers_odd_byte_and_surrogate() {
    let out = run(r#"
        const { StringDecoder } = require('string_decoder');
        const d = new StringDecoder('utf16le');
        const p1 = d.write(Buffer.from([0x61]));
        const p2 = d.write(Buffer.from([0x00, 0x62, 0x00]));
        const e = d.end();
        const d2 = new StringDecoder('ucs2');
        // 𝄞 U+1D11E = D834 DD1E
        const s1 = d2.write(Buffer.from([0x34, 0xD8]));
        const s2 = d2.write(Buffer.from([0x1E, 0xDD]));
        const d3 = new StringDecoder('utf16le');
        const odd = d3.write(Buffer.from([0x61]));
        const oddEnd = d3.end();
        [p1, p2, e, p1 + p2 + e, s1, s2, odd, oddEnd].join('|');
        "#);
    assert_eq!(out, "|ab||ab||𝄞||");
}

#[test]
#[serial]
fn amber_cli_multi_encoding_roundtrip() {
    let script = r#"
        const { StringDecoder } = require('string_decoder');
        const b64 = new StringDecoder('base64');
        const hex = new StringDecoder('hex');
        const u16 = new StringDecoder('utf16le');
        const lat = new StringDecoder('latin1');
        console.log([
          b64.write(Buffer.from([0x4d,0x61,0x6e])),
          hex.write(Buffer.from([0xde,0xad])),
          u16.write(Buffer.from([0x61,0x00,0x62,0x00])),
          lat.write(Buffer.from([0x41,0xc9]))
        ].join('|'));
    "#;
    let output = Command::new(env!("CARGO_BIN_EXE_amber"))
        .args(["eval", script])
        .output()
        .expect("amber eval");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "amber eval multi-encoding failed: {combined}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "TWFu|dead|ab|AÉ",
        "CLI multi-encoding: {combined}"
    );
}

//! Pins docs/STRING_DECODER_CONTRACT.md. UTF-8 StringDecoder write/end only.
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
    let out = run(
        r#"
        const a = require('string_decoder');
        const b = require('node:string_decoder');
        [
          a === b,
          a === string_decoder,
          typeof a.StringDecoder,
          a.default === a.StringDecoder
        ].join('|');
        "#,
    );
    assert_eq!(out, "true|true|function|true");
}

#[test]
#[serial]
fn default_encoding_is_utf8_and_utf_8_normalizes() {
    let out = run(
        r#"
        const { StringDecoder } = require('string_decoder');
        const d1 = new StringDecoder();
        const d2 = new StringDecoder('utf-8');
        const d3 = new StringDecoder('utf8');
        [d1._encoding, d2._encoding, d3._encoding].join('|');
        "#,
    );
    assert_eq!(out, "utf8|utf8|utf8");
}

#[test]
#[serial]
fn string_passthrough_and_buffer_write() {
    let out = run(
        r#"
        const { StringDecoder } = require('string_decoder');
        const d = new StringDecoder('utf8');
        const fromString = d.write('hello');
        const fromBuf = new StringDecoder('utf8').write(Buffer.from('world'));
        const fromU8 = new StringDecoder('utf8').write(new Uint8Array([0x61, 0x62]));
        const fromArr = new StringDecoder('utf8').write([0x63, 0x64]);
        const bad = new StringDecoder('utf8').write(null);
        [fromString, fromBuf, fromU8, fromArr, bad].join('|');
        "#,
    );
    assert_eq!(out, "hello|world|ab|cd|");
}

#[test]
#[serial]
fn incomplete_utf8_multibyte_buffers_across_writes() {
    // Split 你 (E4 BD A0) then 好 (E5 A5 BD) — same shape as conformance fixture.
    let out = run(
        r#"
        const { StringDecoder } = require('string_decoder');
        const decoder = new StringDecoder('utf8');
        const part1 = decoder.write(Buffer.from([0xE4, 0xBD]));
        const part2 = decoder.write(Buffer.from([0xA0, 0xE5, 0xA5, 0xBD]));
        const endPart = decoder.end();
        [part1, part2, endPart, part1 + part2 + endPart].join('|');
        "#,
    );
    assert_eq!(out, "|你好||你好");
}

#[test]
#[serial]
fn end_flushes_held_incomplete_bytes() {
    let out = run(
        r#"
        const { StringDecoder } = require('string_decoder');
        const decoder = new StringDecoder('utf8');
        const held = decoder.write(Buffer.from([0xE4, 0xBD]));
        const flushed = decoder.end(Buffer.from([0xA0]));
        [held, flushed].join('|');
        "#,
    );
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

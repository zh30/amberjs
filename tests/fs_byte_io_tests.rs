// Byte-correct fs read/write contract: encodings, Buffer defaults, append, and ENOENT.
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

fn runtime() -> amberjs::runtime_minimal::MinimalRuntime {
    amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime")
}

fn run(code: &str) -> String {
    runtime()
        .execute_code(code)
        .expect("Execution failed")
        .trim()
        .to_string()
}

#[test]
#[serial]
fn read_file_sync_without_encoding_returns_buffer_bytes() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("bin.dat");
    fs::write(&file, [0xff, 0x00, 0x41]).expect("seed");

    let code = format!(
        r#"
        const fs = require('fs');
        const buf = fs.readFileSync("{}");
        [
          typeof buf,
          Buffer.isBuffer(buf),
          buf.length,
          buf[0],
          buf[1],
          buf[2],
          buf.toString('hex'),
          buf.toString('utf8') === '\u00ff\u0000A'
        ].join('|');
        "#,
        file.display()
    );
    assert_eq!(
        run(&code),
        "object|true|3|255|0|65|ff0041|false",
        "omitted encoding must be a Buffer of the raw bytes"
    );
}

#[test]
#[serial]
fn read_and_write_honor_hex_base64_latin1_and_option_objects() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("enc.txt");
    let path = file.display().to_string();

    let code = format!(
        r#"
        const fs = require('fs');
        fs.writeFileSync("{path}", "6869", "hex");
        const hexRound = fs.readFileSync("{path}", "utf8");
        fs.writeFileSync("{path}", "aGk=", "base64");
        const b64Round = fs.readFileSync("{path}", {{ encoding: "utf8" }});
        fs.writeFileSync("{path}", "\u00ffA", "latin1");
        const latin1 = fs.readFileSync("{path}");
        const asHex = fs.readFileSync("{path}", {{ encoding: "hex" }});
        const asLatin1 = fs.readFileSync("{path}", "latin1");
        [hexRound, b64Round, latin1.length, latin1[0], asHex, asLatin1].join('|');
        "#
    );
    assert_eq!(
        run(&code),
        "hi|hi|2|255|ff41|\u{ff}A",
        "hex, base64, and latin1 must round-trip through option strings and objects"
    );
}

#[test]
#[serial]
fn invalid_hex_does_not_overwrite_and_append_flags_work() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("mut.txt");
    let path = file.display().to_string();

    let code = format!(
        r#"
        const fs = require('fs');
        fs.writeFileSync("{path}", "keep");
        let invalid = "";
        try {{
          fs.writeFileSync("{path}", "zz", "hex");
          invalid = "wrote";
        }} catch (error) {{
          invalid = error.code;
        }}
        const kept = fs.readFileSync("{path}", "utf8");
        fs.appendFileSync("{path}", "42", "hex");
        fs.writeFileSync("{path}", "C", {{ flag: "a" }});
        let exclusive = "";
        try {{
          fs.writeFileSync("{path}", "nope", {{ flag: "wx" }});
          exclusive = "wrote";
        }} catch (error) {{
          exclusive = error.code;
        }}
        [invalid, kept, fs.readFileSync("{path}", "utf8"), exclusive].join('|');
        "#
    );
    assert_eq!(run(&code), "ERR_INVALID_ARG_VALUE|keep|keepBC|EEXIST");
}

#[test]
#[serial]
fn callback_read_and_write_keep_binary_and_report_enoent() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("cb.dat");
    let path = file.display().to_string();
    fs::write(&file, [0xff, 0x10]).expect("seed");

    let code = format!(
        r#"
        const fs = require('fs');
        const out = "{path}.out";
        Promise.all([
          new Promise((resolve) => fs.readFile("{path}", (err, data) => {{
            resolve(err ? "err" : [Buffer.isBuffer(data), data.length, data[0], data.toString("hex")].join(","));
          }})),
          new Promise((resolve) => fs.readFile("{path}", "latin1", (err, data) => {{
            resolve(err ? "err" : data);
          }})),
          new Promise((resolve) => fs.writeFile(out, Buffer.from([255, 0, 65]), (err) => {{
            resolve(err ? err.code : "ok");
          }})),
          new Promise((resolve) => fs.readFile("{path}.missing", (err) => {{
            resolve(err && err.code + ":" + err.syscall);
          }}))
        ]).then((parts) => [parts[0], parts[1], parts[2], fs.readFileSync(out, "hex"), parts[3]].join('|'));
        "#
    );
    assert_eq!(
        run(&code),
        "true,2,255,ff10|\u{ff}\u{10}|ok|ff0041|ENOENT:open"
    );
}

#[test]
#[serial]
fn promises_read_defaults_to_buffer_and_honors_encoding_object() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("prom.dat");
    fs::write(&file, [0x68, 0x69]).expect("seed");
    let path = file.display().to_string();

    let code = format!(
        r#"
        const fs = require('fs');
        const raw = fs.promises.readFile("{path}");
        const hex = fs.promises.readFile("{path}", {{ encoding: "hex" }});
        const written = "{path}.out";
        const pending = fs.promises.writeFile(written, "aGk=", "base64");
        const appended = fs.promises.appendFile(written, "21", "hex");
        Promise.all([
          raw.then((buf) => [Buffer.isBuffer(buf), buf.length, buf[0], buf.toString("utf8")].join(",")),
          hex.then((text) => text),
          pending.then(() => "wrote"),
          appended.then(() => "appended")
        ]).then((parts) => parts.concat(fs.readFileSync(written, "utf8")).join('|'));
        "#
    );
    assert_eq!(run(&code), "true,2,104,hi|6869|wrote|appended|hi!");
}

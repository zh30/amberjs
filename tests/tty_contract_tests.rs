//! Pins docs/TTY_CONTRACT.md — require('tty').isatty via libc isatty (Unix).
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::io::Write;
use tempfile::NamedTempFile;

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
fn require_tty_exposes_isatty_identity() {
    let output = run(r#"
        const tty = require('tty');
        const node = require('node:tty');
        [
          typeof tty.isatty,
          tty.isatty === node.isatty,
          require('tty') === tty
        ].join('|');
        "#);
    assert_eq!(output, "function|true|true");
}

#[test]
#[serial]
fn isatty_negative_and_non_number_are_false() {
    let output = run(r#"
        const tty = require('tty');
        [
          tty.isatty(-1),
          tty.isatty(-100),
          tty.isatty('0'),
          tty.isatty(null),
          tty.isatty(undefined),
          tty.isatty()
        ].join('|');
        "#);
    assert_eq!(output, "false|false|false|false|false|false");
}

#[test]
#[serial]
fn isatty_regular_file_fd_is_false() {
    let mut file = NamedTempFile::new().expect("temp file");
    writeln!(file, "amber tty contract").expect("write temp");
    let path = file.path().to_string_lossy().replace('\\', "\\\\");

    let code = format!(
        r#"
        const fs = require('fs');
        const tty = require('tty');
        const fd = fs.openSync('{path}', 'r');
        const result = tty.isatty(fd);
        fs.closeSync(fd);
        String(result);
        "#
    );
    let output = run(&code);
    assert_eq!(output, "false");
}

#[test]
#[serial]
#[cfg(windows)]
fn isatty_always_false_on_windows() {
    let output = run(r#"
        const tty = require('tty');
        [tty.isatty(0), tty.isatty(1), tty.isatty(2)].join('|');
        "#);
    assert_eq!(output, "false|false|false");
}

//! Pins docs/CHILD_PROCESS_ASYNC_CONTRACT.md (tentative G47).
//! Narrow async exec / execFile: host-thread run + later-turn callback.
use serial_test::serial;
use std::process::Command;
use tempfile::TempDir;

fn amber() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

fn run_eval(script: &str) -> (bool, String, String) {
    let output = Command::new(amber())
        .args(["eval", script])
        .output()
        .expect("amber eval");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

#[test]
#[serial]
fn exec_callback_is_not_same_turn() {
    let (ok, stdout, combined) = run_eval(
        r#"
        const cp = require('child_process');
        const order = [];
        order.push('before');
        cp.exec('printf amberjs_async_exec', (err, stdout) => {
          order.push('cb');
          console.log([
            order.join('|'),
            err === null,
            String(stdout).includes('amberjs_async_exec')
          ].join('::'));
        });
        order.push('after');
        undefined;
        "#,
    );
    assert!(ok, "exec deferred callback failed: {combined}");
    assert_eq!(stdout, "before|after|cb::true::true");
}

#[test]
#[serial]
fn exec_does_not_block_isolate_during_sleep() {
    let (ok, stdout, combined) = run_eval(
        r#"
        const cp = require('child_process');
        let progressed = false;
        cp.exec('sleep 0.25', () => {
          console.log(progressed ? 'nonblocking_ok' : 'still_blocked');
        });
        setTimeout(() => { progressed = true; }, 40);
        undefined;
        "#,
    );
    assert!(ok, "exec non-blocking probe failed: {combined}");
    assert_eq!(stdout, "nonblocking_ok");
}

#[test]
#[serial]
fn exec_file_callback_receives_stdout() {
    let (ok, stdout, combined) = run_eval(
        r#"
        const cp = require('child_process');
        cp.execFile('/bin/echo', ['amberjs_exec_file'], (err, out) => {
          console.log([
            err === null,
            String(out).trim()
          ].join('|'));
        });
        undefined;
        "#,
    );
    assert!(ok, "execFile callback failed: {combined}");
    assert_eq!(stdout, "true|amberjs_exec_file");
}

#[test]
#[serial]
fn exec_nonzero_exit_delivers_error_to_callback() {
    let (ok, stdout, combined) = run_eval(
        r#"
        const cp = require('child_process');
        cp.exec('exit 9', (err, stdout, stderr) => {
          console.log([
            err instanceof Error,
            err && err.code,
            typeof stdout === 'string',
            typeof stderr === 'string'
          ].join('|'));
        });
        undefined;
        "#,
    );
    assert!(ok, "exec nonzero callback failed: {combined}");
    assert_eq!(stdout, "true|9|true|true");
}

#[test]
#[serial]
fn exec_returns_pending_stub_before_exit() {
    let (ok, stdout, combined) = run_eval(
        r#"
        const cp = require('child_process');
        const child = cp.exec('printf done', () => {
          console.log('cb|' + [
            child.pid,
            child.killed,
            child.exitCode === null,
            child.signal === null
          ].join('|'));
        });
        console.log('ret|' + [
          typeof child.on === 'function',
          child.pid,
          child.killed,
          child.exitCode === null,
          child.signal === null,
          child.stdout === undefined,
          child.stderr === undefined
        ].join('|'));
        undefined;
        "#,
    );
    assert!(ok, "pending stub failed: {combined}");
    // First line is sync return shape; callback line proves drain completed.
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.contains(&"ret|true|0|false|true|true|true|true"),
        "missing pending return shape: {combined}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("cb|0|false|true|true")),
        "missing callback drain: {combined}"
    );
}

#[test]
#[serial]
fn sandbox_denies_exec_synchronously() {
    let script = r#"
        const cp = require('child_process');
        let denied = false;
        let cbFired = false;
        try {
          cp.exec('echo denied', () => { cbFired = true; });
        } catch (e) {
          denied = String(e && e.message || e).toLowerCase().includes('permission')
            || String(e && e.message || e).toLowerCase().includes('denied')
            || String(e && e.message || e).toLowerCase().includes('sandbox');
        }
        console.log([denied, cbFired].join('|'));
        "#;
    let output = Command::new(amber())
        .args(["eval", "--sandbox", script])
        .output()
        .expect("amber eval --sandbox");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "sandbox denial probe must run: {combined}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "true|false",
        "sandbox must deny exec before callback: {combined}"
    );
}

#[test]
#[serial]
fn cli_require_child_process_reaches_async_surface() {
    let dir = TempDir::new().expect("temp dir");
    let script = dir.path().join("probe.js");
    std::fs::write(
        &script,
        r#"
        const cp = require('child_process');
        cp.exec('printf cli_async_ok', (err, out) => {
          console.log([
            typeof cp.exec,
            typeof cp.execFile,
            err === null,
            String(out)
          ].join('|'));
        });
        "#,
    )
    .expect("script");
    let output = Command::new(amber())
        .arg("run")
        .arg(&script)
        .output()
        .expect("amber run");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "amber run failed: {combined}");
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|l| l.trim() == "function|function|true|cli_async_ok"),
        "missing async require probe line: {combined}"
    );
}

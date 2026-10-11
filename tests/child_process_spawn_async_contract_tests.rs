//! Pins docs/CHILD_PROCESS_SPAWN_ASYNC_CONTRACT.md (tentative G60).
//! Narrow async spawn: host-thread run + later-turn exit callback.
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
fn spawn_callback_is_not_same_turn() {
    let (ok, stdout, combined) = run_eval(
        r#"
        const cp = require('child_process');
        const order = [];
        order.push('before');
        cp.spawn('/bin/echo', ['amberjs_async_spawn'], (err, stdout) => {
          order.push('cb');
          console.log([
            order.join('|'),
            err === null,
            String(stdout).includes('amberjs_async_spawn')
          ].join('::'));
        });
        order.push('after');
        undefined;
        "#,
    );
    assert!(ok, "spawn deferred callback failed: {combined}");
    assert_eq!(stdout, "before|after|cb::true::true");
}

#[test]
#[serial]
fn spawn_does_not_block_isolate_during_sleep() {
    let (ok, stdout, combined) = run_eval(
        r#"
        const cp = require('child_process');
        let progressed = false;
        cp.spawn('/bin/sleep', ['0.25'], () => {
          console.log(progressed ? 'nonblocking_ok' : 'still_blocked');
        });
        setTimeout(() => { progressed = true; }, 40);
        undefined;
        "#,
    );
    assert!(ok, "spawn non-blocking probe failed: {combined}");
    assert_eq!(stdout, "nonblocking_ok");
}

#[test]
#[serial]
fn spawn_nonzero_exit_delivers_error_to_callback() {
    let (ok, stdout, combined) = run_eval(
        r#"
        const cp = require('child_process');
        cp.spawn('/bin/sh', ['-c', 'exit 9'], (err, stdout, stderr) => {
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
    assert!(ok, "spawn nonzero callback failed: {combined}");
    assert_eq!(stdout, "true|9|true|true");
}

#[test]
#[serial]
fn spawn_returns_pending_stub_before_exit() {
    let (ok, stdout, combined) = run_eval(
        r#"
        const cp = require('child_process');
        const child = cp.spawn('/bin/echo', ['done'], () => {
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
fn sandbox_denies_spawn_synchronously() {
    let script = r#"
        const cp = require('child_process');
        let denied = false;
        let cbFired = false;
        try {
          cp.spawn('/bin/echo', ['denied'], () => { cbFired = true; });
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
        "sandbox must deny spawn before callback: {combined}"
    );
}

#[test]
#[serial]
fn cli_require_child_process_reaches_spawn_async_surface() {
    let dir = TempDir::new().expect("temp dir");
    let script = dir.path().join("probe.js");
    std::fs::write(
        &script,
        r#"
        const cp = require('child_process');
        cp.spawn('/bin/echo', ['cli_spawn_ok'], (err, out) => {
          console.log([
            typeof cp.spawn,
            err === null,
            String(out).trim()
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
            .any(|l| l.trim() == "function|true|cli_spawn_ok"),
        "missing spawn require probe line: {combined}"
    );
}

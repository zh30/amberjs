//! Pins docs/CHILD_PROCESS_CONTRACT.md. Sync-only execSync / spawnSync.
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
fn exec_sync_returns_buffer_and_utf8_string() {
    let (ok, stdout, combined) = run_eval(
        r#"
        const cp = require('child_process');
        const buf = cp.execSync('printf amberjs_exec_sync');
        const str = cp.execSync('printf amberjs_utf8', { encoding: 'utf8' });
        console.log([
          typeof buf.toString === 'function',
          buf.toString(),
          typeof str,
          str
        ].join('|'));
        "#,
    );
    assert!(ok, "execSync contract failed: {combined}");
    assert_eq!(stdout, "true|amberjs_exec_sync|string|amberjs_utf8");
}

#[test]
#[serial]
fn spawn_sync_returns_status_stdout_and_output_array() {
    let (ok, stdout, combined) = run_eval(
        r#"
        const cp = require('child_process');
        const res = cp.spawnSync('/bin/echo', ['amberjs_spawn_sync']);
        console.log([
          res.status,
          res.signal === null,
          res.pid,
          res.stdout.toString().trim(),
          Array.isArray(res.output),
          res.output[0] === null,
          res.output[1].toString().trim(),
          res.error === undefined
        ].join('|'));
        "#,
    );
    assert!(ok, "spawnSync contract failed: {combined}");
    assert_eq!(
        stdout,
        "0|true|0|amberjs_spawn_sync|true|true|amberjs_spawn_sync|true"
    );
}

#[test]
#[serial]
fn exec_sync_nonzero_exit_throws_with_status_and_streams() {
    let (ok, stdout, combined) = run_eval(
        r#"
        const cp = require('child_process');
        let observed = 'ok';
        try {
          cp.execSync('printf out_part; echo err_part >&2; exit 7', { encoding: 'utf8' });
        } catch (e) {
          observed = [
            e instanceof Error,
            String(e.message).startsWith('Command failed:'),
            e.status,
            e.stdout,
            String(e.stderr).includes('err_part')
          ].join('|');
        }
        console.log(observed);
        "#,
    );
    assert!(ok, "execSync failure shape failed: {combined}");
    assert_eq!(stdout, "true|true|7|out_part|true");
}

#[test]
#[serial]
fn spawn_sync_missing_command_sets_error_without_throw() {
    let (ok, stdout, combined) = run_eval(
        r#"
        const cp = require('child_process');
        const res = cp.spawnSync('/nonexistent/amberjs-missing-binary-xyz');
        console.log([
          res.status,
          res.signal === null,
          res.pid,
          res.stdout === '',
          typeof res.stderr === 'string' && res.stderr.length > 0,
          res.error instanceof Error
        ].join('|'));
        "#,
    );
    assert!(ok, "spawnSync missing command failed: {combined}");
    assert_eq!(stdout, "1|true|0|true|true|true");
}

#[test]
#[serial]
fn sandbox_denies_exec_sync_and_spawn_sync() {
    let script = r#"
        const cp = require('child_process');
        let execDenied = false;
        let spawnDenied = false;
        try { cp.execSync('echo denied'); } catch (e) {
          execDenied = String(e && e.message || e).toLowerCase().includes('permission')
            || String(e && e.message || e).toLowerCase().includes('denied')
            || String(e && e.message || e).toLowerCase().includes('sandbox');
        }
        try { cp.spawnSync('echo', ['denied']); } catch (e) {
          spawnDenied = String(e && e.message || e).toLowerCase().includes('permission')
            || String(e && e.message || e).toLowerCase().includes('denied')
            || String(e && e.message || e).toLowerCase().includes('sandbox');
        }
        console.log([execDenied, spawnDenied].join('|'));
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
        "true|true",
        "sandbox must deny execSync and spawnSync: {combined}"
    );
}

#[test]
#[serial]
fn cli_require_child_process_reaches_sync_surface() {
    let dir = TempDir::new().expect("temp dir");
    let script = dir.path().join("probe.js");
    std::fs::write(
        &script,
        r#"
        const cp = require('child_process');
        const out = cp.execSync('printf cli_ok', { encoding: 'utf8' });
        const res = cp.spawnSync('/bin/echo', ['spawn_cli']);
        console.log([
          typeof cp.execSync,
          typeof cp.spawnSync,
          out,
          res.status,
          res.stdout.toString().trim()
        ].join('|'));
        "#,
    )
    .expect("script");
    let output = Command::new(amber())
        .arg("run")
        .arg(&script)
        .output()
        .expect("amber run");
    assert!(
        output.status.success(),
        "amber run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "function|function|cli_ok|0|spawn_cli"
    );
}

//! Pins docs/CHILD_PROCESS_FORK_CONTRACT.md (G67).
//! Real OS-process fork + TCP IPC message round-trip.
use serial_test::serial;
use std::process::Command;
use tempfile::TempDir;

fn amber() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

#[test]
#[serial]
fn fork_ipc_message_round_trip() {
    let dir = TempDir::new().expect("temp dir");
    let child_script = dir.path().join("fork_child.js");
    let parent_script = dir.path().join("fork_parent.js");
    std::fs::write(
        &child_script,
        r#"
        process.on('message', (msg) => {
          process.send({ echo: msg, from: 'child' });
          // Exit after reply so the parent event loop cannot hang if IPC
          // teardown races with disconnect().
          process.exit(0);
        });
        "#,
    )
    .expect("child script");
    std::fs::write(
        &parent_script,
        format!(
            r#"
        const cp = require('child_process');
        const child = cp.fork({child:?});
        const seen = [];
        child.on('message', (msg) => {{
          seen.push(msg);
          console.log(['msg', msg.echo.hello, msg.from, child.pid > 0, child.connected].join('|'));
          child.disconnect();
        }});
        child.on('exit', (code) => {{
          console.log(['exit', code, seen.length].join('|'));
        }});
        child.send({{ hello: 'amberjs_fork' }});
        "#,
            child = child_script
        ),
    )
    .expect("parent script");

    let output = Command::new(amber())
        .arg("run")
        .arg(&parent_script)
        .output()
        .expect("amber run parent");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, String::from_utf8_lossy(&output.stderr));
    assert!(
        output.status.success(),
        "fork round-trip failed: {combined}"
    );
    assert!(
        stdout
            .lines()
            .any(|l| l.trim() == "msg|amberjs_fork|child|true|true"),
        "missing message line: {combined}"
    );
    assert!(
        stdout.lines().any(|l| l.trim().starts_with("exit|")),
        "missing exit line: {combined}"
    );
}

#[test]
#[serial]
fn fork_symbol_reachable_via_require() {
    let (ok, stdout, combined) = {
        let output = Command::new(amber())
            .args([
                "eval",
                "const cp = require('child_process'); console.log([typeof cp.fork, typeof cp.exec, typeof cp.execSync].join('|'));",
            ])
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
    };
    assert!(ok, "require fork probe failed: {combined}");
    assert_eq!(stdout, "function|function|function");
}

#[test]
#[serial]
fn fork_sandbox_denies_synchronously() {
    let dir = TempDir::new().expect("temp dir");
    let child_script = dir.path().join("denied_child.js");
    std::fs::write(&child_script, "process.on('message', () => {});").expect("child");
    let script = format!(
        r#"
        const cp = require('child_process');
        let denied = false;
        try {{
          cp.fork({child:?});
        }} catch (e) {{
          const m = String(e && e.message || e).toLowerCase();
          denied = m.includes('permission') || m.includes('denied') || m.includes('sandbox');
        }}
        console.log(denied ? 'denied' : 'allowed');
        "#,
        child = child_script
    );
    let output = Command::new(amber())
        .args(["eval", "--sandbox", &script])
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
        "denied",
        "sandbox must deny fork before spawn: {combined}"
    );
}

#[test]
#[serial]
fn fork_child_exits_and_reports_code() {
    let dir = TempDir::new().expect("temp dir");
    let child_script = dir.path().join("exit_child.js");
    let parent_script = dir.path().join("exit_parent.js");
    std::fs::write(
        &child_script,
        r#"
        process.on('message', () => {
          process.exit(7);
        });
        "#,
    )
    .expect("child");
    std::fs::write(
        &parent_script,
        format!(
            r#"
        const cp = require('child_process');
        const child = cp.fork({child:?});
        child.on('exit', (code) => {{
          console.log(['exit', code, child.exitCode].join('|'));
        }});
        child.send({{ go: true }});
        "#,
            child = child_script
        ),
    )
    .expect("parent");
    let output = Command::new(amber())
        .arg("run")
        .arg(&parent_script)
        .output()
        .expect("amber run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, String::from_utf8_lossy(&output.stderr));
    assert!(
        output.status.success(),
        "exit code probe failed: {combined}"
    );
    assert!(
        stdout.lines().any(|l| l.trim() == "exit|7|7"),
        "expected exit|7|7: {combined}"
    );
}

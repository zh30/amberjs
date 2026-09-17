use std::process::Command;

fn amber_path() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

#[test]
fn test_exec_sync_basic() {
    let script = "const cp = require('child_process'); console.log(cp.execSync('echo sync_hello').toString().trim());";
    let output = Command::new(amber_path())
        .args(["eval", script])
        .output()
        .expect("failed to execute amber eval");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, "sync_hello");
}

#[test]
fn test_spawn_sync_basic() {
    let script = "const cp = require('child_process'); const res = cp.spawnSync('echo', ['spawn_ok']); console.log(res.status, res.stdout.toString().trim());";
    let output = Command::new(amber_path())
        .args(["eval", script])
        .output()
        .expect("failed to execute amber eval");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, "0 spawn_ok");
}

#[test]
fn test_exec_sync_denied_in_sandbox() {
    let script = "const cp = require('child_process'); cp.execSync('echo denied');";
    let output = Command::new(amber_path())
        .args(["eval", "--sandbox", script])
        .output()
        .expect("failed to execute amber eval");
    assert!(
        !output.status.success(),
        "execSync must fail under --sandbox"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Permission")
            || combined.contains("sandbox")
            || combined.contains("denied")
    );
}

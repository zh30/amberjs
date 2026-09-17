use std::process::Command;
use tempfile::tempdir;

fn amber_path() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

#[test]
fn version_uses_cargo_package_version() {
    let output = Command::new(amber_path())
        .arg("--version")
        .output()
        .expect("failed to execute amber --version");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), format!("amber {}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn eval_prints_only_result_by_default() {
    let output = Command::new(amber_path())
        .args(["eval", "1 + 1"])
        .output()
        .expect("failed to execute amber eval");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "2");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Setting up") && !stderr.contains("[v0.3."),
        "default eval should not print runtime setup logs: {stderr}"
    );
}

#[test]
fn eval_does_not_print_trailing_undefined_for_console_output() {
    let output = Command::new(amber_path())
        .args(["eval", "console.log('hello')"])
        .output()
        .expect("failed to execute amber eval");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello");
}

#[test]
fn run_prints_user_output_without_trailing_undefined() {
    let dir = tempdir().expect("failed to create tempdir");
    let script = dir.path().join("hello.js");
    std::fs::write(&script, "console.log('hello from run');").expect("failed to write script");

    let output = Command::new(amber_path())
        .arg("run")
        .arg(&script)
        .output()
        .expect("failed to execute amber run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello from run"
    );
}

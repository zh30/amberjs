use std::process::Command;

fn amber_path() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

#[test]
fn runtime_binary_is_named_amber() {
    let output = Command::new(amber_path())
        .arg("--version")
        .output()
        .expect("failed to execute amber --version");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        format!("amber {}", env!("CARGO_PKG_VERSION"))
    );
}

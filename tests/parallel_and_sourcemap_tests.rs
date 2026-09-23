use std::process::Command;
use tempfile::tempdir;

fn amber() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

#[test]
fn amber_test_parallel_exits_two() {
    let output = Command::new(amber())
        .args(["test", "--parallel", "examples/testing"])
        .output()
        .expect("amber test --parallel");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(2),
        "amber test --parallel must exit 2 (stdout={stdout} stderr={stderr})"
    );
    assert!(
        stderr.contains("not supported") || stdout.contains("not supported"),
        "should explain that --parallel is unsupported: {stdout}{stderr}"
    );
}

#[test]
fn typescript_throw_stack_names_original_file_and_line() {
    let dir = tempdir().unwrap();
    let ts = dir.path().join("foo.ts");
    std::fs::write(
        &ts,
        r#"function fail(): never {
    throw new Error("mapped-boom");
}
fail();
"#,
    )
    .unwrap();

    let output = Command::new(amber())
        .arg("run")
        .arg(&ts)
        .output()
        .expect("amber run foo.ts");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.status.success(), "foo.ts should throw: {combined}");
    assert!(
        combined.contains("foo.ts:2") || combined.contains("foo.ts:2:"),
        "stack must name foo.ts:2 (throw line), got: {combined}"
    );
}

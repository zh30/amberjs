use std::process::Command;

fn amber_path() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

#[test]
fn test_deterministic_seed_math_random() {
    let script = "console.log(Math.random(), Math.random());";
    let run = || -> String {
        let output = Command::new(amber_path())
            .args(["eval", "--seed", "42", script])
            .output()
            .expect("failed to execute amber eval");
        assert!(output.status.success());
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };

    let first = run();
    let second = run();
    assert_eq!(
        first, second,
        "Math.random with seed 42 must produce identical output"
    );
    assert!(!first.is_empty());
}

#[test]
fn test_deterministic_seed_crypto_random_bytes() {
    let script = "const crypto = require('crypto'); console.log(Array.from(crypto.randomBytes(8)).join(','));";
    let run = || -> String {
        let output = Command::new(amber_path())
            .args(["eval", "--seed", "99999", script])
            .output()
            .expect("failed to execute amber eval");
        assert!(output.status.success());
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };

    let first = run();
    let second = run();
    assert_eq!(
        first, second,
        "crypto.randomBytes with seed 99999 must produce identical output"
    );
}

#[test]
fn test_virtual_time_freeze_time() {
    let script = "console.log(Date.now(), new Date().toISOString(), performance.now());";
    let output = Command::new(amber_path())
        .args(["eval", "--freeze-time", "2026-09-04T12:00:00Z", script])
        .output()
        .expect("failed to execute amber eval");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, "1788523200000 2026-09-04T12:00:00.000Z 0");
}

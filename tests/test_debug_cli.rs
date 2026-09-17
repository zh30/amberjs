// Test Stage 59: CLI Debugger Integration
//
// This test suite verifies the CLI debug command functionality

use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_debug_command_exists() {
    // Verify that the debug command is recognized
    let output = Command::new("cargo")
        .args(["run", "--", "debug", "--help"])
        .output()
        .expect("Failed to run amber debug --help");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check that help output is shown (not an error about unknown command)
    assert!(
        output.status.success(),
        "Debug command should be recognized. stderr: {}",
        stderr
    );
}

#[test]
fn test_debug_script_command() {
    // Test that debug command accepts a script file
    let test_script = PathBuf::from("/tmp/test_debug_script.js");

    // Create a simple test script
    std::fs::write(&test_script, "console.log('test');").expect("Failed to create test script");

    let output = Command::new("cargo")
        .args(["run", "--", "debug", test_script.to_str().unwrap()])
        .output()
        .expect("Failed to run amber debug script.js");

    // The command should at least start (may exit with error due to unimplemented features)
    // but should not fail with "unknown command" error
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Clean up
    let _ = std::fs::remove_file(&test_script);

    // Debug command should be recognized (even if not fully implemented)
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("unknown command"),
        "Debug command should be recognized. stderr: {}",
        stderr
    );
}

#[test]
fn test_debug_with_options() {
    // Current public CLI only supports `amber debug <file>`.
    // Historical Stage 59 flags such as --port are intentionally not exposed.
    let test_script = PathBuf::from("/tmp/test_debug_options.js");
    std::fs::write(&test_script, "let x = 42; console.log(x);")
        .expect("Failed to create test script");

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "debug",
            test_script.to_str().unwrap(),
            "--port",
            "9229",
        ])
        .output()
        .expect("Failed to run amber debug with options");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Clean up
    let _ = std::fs::remove_file(&test_script);

    assert!(
        !output.status.success()
            && stderr.contains("unexpected argument '--port'")
            && stderr.contains("Usage: amber debug"),
        "Unsupported debug --port flag should be rejected by the current public CLI. stderr: {}",
        stderr
    );
}

#[test]
fn test_debug_attach_command() {
    // Attach mode is a historical Stage 59 design, not part of the current public CLI.
    let output = Command::new("cargo")
        .args(["run", "--", "debug", "attach", "--pid", "1234"])
        .output()
        .expect("Failed to run amber debug attach");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success()
            && stderr.contains("unexpected argument '--pid'")
            && stderr.contains("Usage: amber debug"),
        "Unsupported debug attach mode should be rejected by the current public CLI. stderr: {}",
        stderr
    );
}

#[test]
fn test_debug_inspect_command() {
    // Inspect mode is a historical Stage 59 design, not part of the current public CLI.
    let output = Command::new("cargo")
        .args(["run", "--", "debug", "inspect", "--port", "8080"])
        .output()
        .expect("Failed to run amber debug inspect");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success()
            && stderr.contains("unexpected argument '--port'")
            && stderr.contains("Usage: amber debug"),
        "Unsupported debug inspect mode should be rejected by the current public CLI. stderr: {}",
        stderr
    );
}

#[test]
fn test_debug_web_flag() {
    // Web UI debugging is documented as future work and is not exposed in the current CLI.
    let test_script = PathBuf::from("/tmp/test_debug_web.js");
    std::fs::write(&test_script, "console.log('web debug');")
        .expect("Failed to create test script");

    let output = Command::new("cargo")
        .args(["run", "--", "debug", test_script.to_str().unwrap(), "--web"])
        .output()
        .expect("Failed to run amber debug --web");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Clean up
    let _ = std::fs::remove_file(&test_script);

    assert!(
        !output.status.success()
            && stderr.contains("unexpected argument '--web'")
            && stderr.contains("Usage: amber debug"),
        "Unsupported debug --web flag should be rejected by the current public CLI. stderr: {}",
        stderr
    );
}

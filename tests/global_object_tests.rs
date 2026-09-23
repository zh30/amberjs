//! Tests for globalThis.global object compatibility
//! v0.3.42: Node.js/Bun compatibility - globalThis.global should equal globalThis

use serial_test::serial;
use std::path::PathBuf;
use std::process::Command;

fn amber_path() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_BIN_EXE_amber").unwrap_or_else(|_| "./target/debug/amber".to_string()),
    )
}

#[test]
#[serial]
fn test_global_object_exists() {
    let output = Command::new(amber_path())
        .args(["eval", "typeof globalThis.global"])
        .output()
        .expect("Failed to execute amber");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("object"),
        "global should be an object, got: {}",
        stdout
    );
}

#[test]
#[serial]
fn test_global_equals_global_this() {
    let output = Command::new(amber_path())
        .args(["eval", "globalThis.global === globalThis"])
        .output()
        .expect("Failed to execute amber");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("true"),
        "globalThis.global should equal globalThis, got: {}",
        stdout
    );
}

#[test]
#[serial]
fn test_global_contains_all_globals() {
    let output = Command::new(amber_path())
        .args(["eval", "globalThis.global.setTimeout === setTimeout"])
        .output()
        .expect("Failed to execute amber");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("true"),
        "global should contain setTimeout, got: {}",
        stdout
    );
}

#[test]
#[serial]
fn test_global_contains_process() {
    let output = Command::new(amber_path())
        .args(["eval", "globalThis.global.process === process"])
        .output()
        .expect("Failed to execute amber");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("true"),
        "global should contain process, got: {}",
        stdout
    );
}

#[test]
#[serial]
fn test_global_contains_console() {
    let output = Command::new(amber_path())
        .args(["eval", "globalThis.global.console === console"])
        .output()
        .expect("Failed to execute amber");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("true"),
        "global should contain console, got: {}",
        stdout
    );
}

#[test]
#[serial]
fn test_global_contains_buffer() {
    let output = Command::new(amber_path())
        .args(["eval", "globalThis.global.Buffer === Buffer"])
        .output()
        .expect("Failed to execute amber");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("true"),
        "global should contain Buffer, got: {}",
        stdout
    );
}

#[test]
#[serial]
fn test_global_this_is_same_reference() {
    // Test that globalThis.global and globalThis point to the same object reference
    let output = Command::new(amber_path())
        .args(["eval", "Object.is(globalThis.global, globalThis)"])
        .output()
        .expect("Failed to execute amber");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("true"),
        "Object.is(globalThis.global, globalThis) should be true, got: {}",
        stdout
    );
}

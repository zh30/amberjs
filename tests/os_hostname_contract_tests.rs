//! Pins docs/OS_HOSTNAME_CONTRACT.md — os.hostname via real OS syscall (G62).
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::process::Command;
use std::sync::Mutex;

/// Serializes env mutation across hostname honesty tests.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn runtime() -> MinimalRuntime {
    MinimalRuntime::new().expect("Failed to create runtime")
}

fn run(code: &str) -> String {
    runtime()
        .execute_code(code)
        .unwrap_or_else(|err| panic!("Execution failed: {err}"))
        .trim()
        .to_string()
}

/// Host name from the OS `hostname` utility (same kernel name as gethostname(2) /
/// GetComputerNameW on normal CI hosts). Used only as an independent oracle.
fn expected_host_hostname() -> String {
    let output = Command::new("hostname")
        .output()
        .expect("hostname command must run");
    assert!(
        output.status.success(),
        "hostname command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .trim_end_matches('.')
        .to_string()
}

#[test]
#[serial]
fn require_hostname_is_function_and_identity() {
    let output = run(r#"
        const os = require('os');
        const nodeOs = require('node:os');
        [
          typeof os.hostname,
          os.hostname === nodeOs.hostname,
          require('os') === os,
          os === globalThis.os
        ].join('|');
        "#);
    assert_eq!(output, "function|true|true|true");
}

#[test]
#[serial]
fn hostname_matches_os_hostname_utility() {
    let expected = expected_host_hostname();
    let output = run(r#"
        const os = require('os');
        os.hostname();
        "#);
    assert_eq!(
        output, expected,
        "os.hostname() must match OS hostname, got {output:?} expected {expected:?}"
    );
    assert!(
        !output.is_empty(),
        "hostname should be non-empty on a normal CI host"
    );
}

#[test]
#[serial]
fn hostname_ignores_hostname_env_fiction() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let fiction = "env-fiction-not-real-hostname-g62";
    let prev = std::env::var_os("HOSTNAME");
    // SAFETY: serialized by ENV_LOCK; restored below.
    unsafe { std::env::set_var("HOSTNAME", fiction) };

    let expected = expected_host_hostname();
    let output = run(r#"
        const os = require('os');
        os.hostname();
        "#);

    match prev {
        Some(v) => unsafe { std::env::set_var("HOSTNAME", v) },
        None => unsafe { std::env::remove_var("HOSTNAME") },
    }

    assert_ne!(
        output, fiction,
        "os.hostname() must not read HOSTNAME env (pre-G62 fiction)"
    );
    assert_eq!(output, expected);
}

#[test]
#[serial]
fn hostname_ignores_computername_env_fiction() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let fiction = "COMPUTERNAME-fiction-g62";
    let prev = std::env::var_os("COMPUTERNAME");
    unsafe { std::env::set_var("COMPUTERNAME", fiction) };

    let expected = expected_host_hostname();
    let output = run(r#"
        const os = require('os');
        os.hostname();
        "#);

    match prev {
        Some(v) => unsafe { std::env::set_var("COMPUTERNAME", v) },
        None => unsafe { std::env::remove_var("COMPUTERNAME") },
    }

    assert_ne!(
        output, fiction,
        "os.hostname() must not read COMPUTERNAME env (pre-G62 fiction)"
    );
    assert_eq!(output, expected);
}

#[test]
#[serial]
fn hostname_is_not_localhost_env_fallback() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let prev_h = std::env::var_os("HOSTNAME");
    let prev_c = std::env::var_os("COMPUTERNAME");
    unsafe {
        std::env::remove_var("HOSTNAME");
        std::env::remove_var("COMPUTERNAME");
    }

    let expected = expected_host_hostname();
    let output = run(r#"
        const os = require('os');
        os.hostname();
        "#);

    match prev_h {
        Some(v) => unsafe { std::env::set_var("HOSTNAME", v) },
        None => unsafe { std::env::remove_var("HOSTNAME") },
    }
    match prev_c {
        Some(v) => unsafe { std::env::set_var("COMPUTERNAME", v) },
        None => unsafe { std::env::remove_var("COMPUTERNAME") },
    }

    // Pre-G62 returned "localhost" when both env vars were unset.
    if expected != "localhost" {
        assert_ne!(
            output, "localhost",
            "must not invent localhost when env vars are unset"
        );
    }
    assert_eq!(output, expected);
}

#[test]
#[serial]
fn esm_named_hostname_export() {
    let expected = expected_host_hostname();
    let settled = r#"
        (async () => {
          const mod = await import('node:os');
          const os = require('os');
          return [
            typeof mod.hostname === 'function',
            mod.hostname === os.hostname,
            mod.hostname()
          ].join('|');
        })()
        "#;
    assert_eq!(run(settled), format!("true|true|{expected}"));
}

#[test]
#[serial]
fn network_interfaces_stays_outside_stable_pin() {
    // Honesty: method may exist as Preview; this test does not pin fake lo/lo0.
    let output = run(r#"
        const os = require('os');
        typeof os.networkInterfaces;
        "#);
    assert_eq!(output, "function");
}

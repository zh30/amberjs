//! Pins docs/DNS_CONTRACT.md. getaddrinfo lookup/resolve only; no reverse/getServers.
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::process::Command;
use tempfile::TempDir;

fn runtime() -> MinimalRuntime {
    MinimalRuntime::new().expect("runtime")
}

fn run(code: &str) -> String {
    runtime()
        .execute_code(code)
        .unwrap_or_else(|err| panic!("Execution failed: {err}"))
        .trim()
        .to_string()
}

#[test]
#[serial]
fn lookup_localhost_returns_address_and_sync_callback() {
    let out = run(r#"
        const dns = require('dns');
        const ret = dns.lookup('localhost');
        let err = 'unset';
        let address = 'unset';
        let family = 'unset';
        dns.lookup('localhost', (e, a, f) => { err = e; address = a; family = f; });
        [
          typeof ret === 'string' && ret.length > 0,
          ret === '127.0.0.1' || ret === '::1',
          err === null,
          address === ret,
          family === 4,
          typeof dns.lookup === 'function'
        ].join('|');
        "#);
    assert_eq!(out, "true|true|true|true|true|true");
}

#[test]
#[serial]
fn resolve_resolve4_resolve6_localhost() {
    let out = run(r#"
        const dns = require('dns');
        const all = dns.resolve('localhost');
        const v4 = dns.resolve4('localhost');
        const v6 = dns.resolve6('localhost');
        const ignored = dns.resolve('localhost', 'MX');
        [
          Array.isArray(all),
          all.length > 0,
          all.every((a) => typeof a === 'string'),
          Array.isArray(v4),
          v4.includes('127.0.0.1') || v4.length === 0,
          v4.every((a) => !a.includes(':')),
          Array.isArray(v6),
          v6.includes('::1') || v6.length === 0,
          v6.every((a) => a.includes(':')),
          Array.isArray(ignored),
          ignored.join(',') === all.join(',')
        ].join('|');
        "#);
    assert_eq!(
        out,
        "true|true|true|true|true|true|true|true|true|true|true"
    );
}

#[test]
#[serial]
fn empty_hostname_and_lookup_failure_are_error_strings() {
    let out = run(r#"
        const dns = require('dns');
        const empty = dns.lookup('');
        let cbEmpty = 'unset';
        dns.lookup('', (e) => { cbEmpty = e; });
        const miss = dns.resolve4('this-host-should-not-exist.invalid');
        [
          empty === 'Error: hostname is required',
          cbEmpty === 'Error: hostname is required',
          typeof miss === 'string',
          String(miss).startsWith('Error: dns.resolve4 ')
        ].join('|');
        "#);
    assert_eq!(out, "true|true|true|true");
}

#[test]
#[serial]
fn sandbox_denies_lookup_without_throwing() {
    let script = r#"
        const dns = require('dns');
        const ret = dns.lookup('localhost');
        let cb = 'unset';
        dns.lookup('localhost', (e) => { cb = e; });
        const denied = (s) => typeof s === 'string' && /permission|denied|sandbox/i.test(s);
        console.log([denied(ret), denied(cb)].join('|'));
        "#;
    let output = Command::new(env!("CARGO_BIN_EXE_amber"))
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
        "sandbox dns probe must run: {combined}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "true|true",
        "sandbox must deny dns.lookup without throw: {combined}"
    );
}

#[test]
#[serial]
fn reverse_and_get_servers_are_outside_the_stable_contract() {
    // Documented as Preview stubs: they exist, but this contract does not pin them.
    // The test only asserts the Stable methods are present and the module is reachable.
    let out = run(r#"
        const dns = require('dns');
        [
          typeof dns.lookup,
          typeof dns.resolve,
          typeof dns.resolve4,
          typeof dns.resolve6
        ].join('|');
        "#);
    assert_eq!(out, "function|function|function|function");
}

#[test]
#[serial]
fn cli_require_dns_reaches_lookup() {
    let dir = TempDir::new().expect("temp dir");
    let script = dir.path().join("probe.js");
    std::fs::write(
        &script,
        r#"
        const dns = require('dns');
        const ip = dns.lookup('localhost');
        console.log([
          typeof dns.lookup,
          typeof dns.resolve4,
          ip === '127.0.0.1' || ip === '::1'
        ].join('|'));
        "#,
    )
    .expect("script");
    let output = Command::new(env!("CARGO_BIN_EXE_amber"))
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
        "function|function|true"
    );
}

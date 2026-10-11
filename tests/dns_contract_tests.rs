//! Pins docs/DNS_CONTRACT.md — G27 getaddrinfo + G65 reverse/getServers.
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
fn reverse_localhost_is_real_ptr_not_echo() {
    let out = run(r#"
        const dns = require('dns');
        const names = dns.reverse('127.0.0.1');
        const empty = dns.reverse('');
        const bad = dns.reverse('not-an-ip');
        [
          Array.isArray(names),
          names.length >= 1,
          names.every((n) => typeof n === 'string' && n.length > 0),
          !names.includes('127.0.0.1'),
          empty === 'Error: IP address is required',
          typeof bad === 'string' && String(bad).startsWith('Error: dns.reverse ')
        ].join('|');
        "#);
    assert_eq!(out, "true|true|true|true|true|true");
}

#[test]
#[serial]
fn get_servers_reads_system_resolver_list() {
    let out = run(r#"
        const dns = require('dns');
        const servers = dns.getServers();
        [
          Array.isArray(servers),
          servers.every((s) => typeof s === 'string' && s.length > 0)
        ].join('|');
        "#);
    assert_eq!(out, "true|true");

    // Honesty: must not invent 8.8.8.8. Compare to /etc/resolv.conf on Unix.
    #[cfg(unix)]
    {
        let expected: Vec<String> = std::fs::read_to_string("/etc/resolv.conf")
            .unwrap_or_default()
            .lines()
            .filter_map(|line| {
                let t = line.trim();
                if t.is_empty() || t.starts_with('#') || t.starts_with(';') {
                    return None;
                }
                let mut parts = t.split_whitespace();
                if parts.next() != Some("nameserver") {
                    return None;
                }
                parts.next().map(|s| s.to_string())
            })
            .collect();
        let got = run(r#"
            const dns = require('dns');
            JSON.stringify(dns.getServers());
            "#);
        let got_vec: Vec<String> = serde_json::from_str(&got).expect("getServers JSON array");
        assert_eq!(
            got_vec, expected,
            "getServers must match /etc/resolv.conf nameserver lines (no fake 8.8.8.8)"
        );
    }
}

#[test]
#[serial]
fn sandbox_denies_reverse_without_throwing() {
    let script = r#"
        const dns = require('dns');
        const ret = dns.reverse('127.0.0.1');
        const denied = (s) => typeof s === 'string' && /permission|denied|sandbox/i.test(s);
        console.log(denied(ret) ? 'true' : 'false');
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
        "sandbox reverse probe must run: {combined}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "true",
        "sandbox must deny dns.reverse without throw: {combined}"
    );
}

#[test]
#[serial]
fn cli_require_dns_reaches_lookup_and_reverse() {
    let dir = TempDir::new().expect("temp dir");
    let script = dir.path().join("probe.js");
    std::fs::write(
        &script,
        r#"
        const dns = require('dns');
        const ip = dns.lookup('localhost');
        const names = dns.reverse('127.0.0.1');
        const servers = dns.getServers();
        console.log([
          typeof dns.lookup,
          typeof dns.resolve4,
          typeof dns.reverse,
          typeof dns.getServers,
          ip === '127.0.0.1' || ip === '::1',
          Array.isArray(names) && names.length >= 1 && !names.includes('127.0.0.1'),
          Array.isArray(servers)
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
        "function|function|function|function|true|true|true"
    );
}

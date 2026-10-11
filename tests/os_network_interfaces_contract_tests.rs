//! Pins docs/OS_NETWORK_INTERFACES_CONTRACT.md (G63 real os.networkInterfaces).
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::collections::HashSet;
use std::process::Command;

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

/// Host interface names from the same crate the runtime uses.
fn host_iface_names() -> HashSet<String> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .map(|i| i.name)
        .collect()
}

#[test]
#[serial]
fn require_exposes_network_interfaces_function() {
    let output = run(r#"
        const os = require('os');
        const nodeOs = require('node:os');
        [
          typeof os.networkInterfaces,
          os.networkInterfaces === nodeOs.networkInterfaces,
          os.networkInterfaces === globalThis.os.networkInterfaces
        ].join('|');
        "#);
    assert_eq!(output, "function|true|true");
}

#[test]
#[serial]
fn enumerates_real_host_interface_names() {
    let host = host_iface_names();
    assert!(
        !host.is_empty(),
        "test host must expose at least one interface via if-addrs"
    );

    let output = run(r#"
        const ni = require('os').networkInterfaces();
        Object.keys(ni).sort().join(',');
        "#);
    let amber: HashSet<String> = output
        .split(',')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    // Every Amber key must be a real host interface (no invented lo/lo0 fiction).
    for name in &amber {
        assert!(
            host.contains(name),
            "Amber reported unknown interface {name:?}; host has {host:?}"
        );
    }
    // At least one real host interface must appear.
    assert!(
        !amber.is_empty(),
        "networkInterfaces() returned no interfaces; host has {host:?}"
    );
    assert!(
        amber.iter().any(|n| host.contains(n)),
        "no overlap between Amber {amber:?} and host {host:?}"
    );
}

#[test]
#[serial]
fn loopback_entry_shape_and_internal_flag() {
    let output = run(r#"
        const ni = require('os').networkInterfaces();
        let found = null;
        for (const name of Object.keys(ni)) {
          for (const entry of ni[name]) {
            if (entry.family === 'IPv4' && entry.address === '127.0.0.1') {
              found = entry;
              break;
            }
          }
          if (found) break;
        }
        if (!found) {
          // Some hosts may only expose ::1; accept IPv6 loopback.
          for (const name of Object.keys(ni)) {
            for (const entry of ni[name]) {
              if (entry.family === 'IPv6' && (entry.address === '::1' || entry.address === '0:0:0:0:0:0:0:1')) {
                found = entry;
                break;
              }
            }
            if (found) break;
          }
        }
        if (!found) 'missing-loopback';
        else [
          typeof found.address,
          typeof found.netmask,
          found.family === 'IPv4' || found.family === 'IPv6',
          typeof found.mac === 'string',
          found.internal === true,
          typeof found.cidr === 'string' && found.cidr.includes('/')
        ].join('|');
        "#);
    assert_eq!(
        output, "string|string|true|true|true|true",
        "expected loopback entry shape, got {output:?}"
    );
}

#[test]
#[serial]
fn not_hardcoded_dual_lo_lo0_fiction() {
    // Pre-G63 always set both "lo" and "lo0" to the same synthetic array.
    // Real hosts typically have one loopback name, not both invented keys.
    let output = run(r#"
        const ni = require('os').networkInterfaces();
        const hasLo = Object.prototype.hasOwnProperty.call(ni, 'lo');
        const hasLo0 = Object.prototype.hasOwnProperty.call(ni, 'lo0');
        // If both exist, they must not be the only interfaces and must
        // reflect distinct OS-reported groups (still allowed on odd hosts).
        // The old stub always had BOTH keys even when the OS had only one.
        const keys = Object.keys(ni);
        const onlyFiction = keys.length === 2 && hasLo && hasLo0
          && ni.lo.length === 1 && ni.lo0.length === 1
          && ni.lo[0].address === '127.0.0.1' && ni.lo0[0].address === '127.0.0.1'
          && !keys.some(k => k !== 'lo' && k !== 'lo0');
        (!onlyFiction).toString();
        "#);
    assert_eq!(
        output, "true",
        "must not be the pre-G63 lo+lo0-only fiction"
    );
}

#[test]
#[serial]
fn entry_fields_are_present_for_every_address() {
    let output = run(r#"
        const ni = require('os').networkInterfaces();
        let ok = true;
        let count = 0;
        for (const name of Object.keys(ni)) {
          if (!Array.isArray(ni[name])) { ok = false; break; }
          for (const e of ni[name]) {
            count++;
            if (typeof e.address !== 'string' || e.address.length === 0) ok = false;
            if (typeof e.netmask !== 'string' || e.netmask.length === 0) ok = false;
            if (e.family !== 'IPv4' && e.family !== 'IPv6') ok = false;
            if (typeof e.mac !== 'string') ok = false;
            if (typeof e.internal !== 'boolean') ok = false;
            if (typeof e.cidr !== 'string' || !e.cidr.startsWith(e.address + '/')) ok = false;
          }
        }
        [ok, count > 0].join('|');
        "#);
    assert_eq!(output, "true|true");
}

#[test]
#[serial]
fn cli_amber_run_reaches_network_interfaces() {
    let binary = env!("CARGO_BIN_EXE_amber");
    let script = std::env::temp_dir().join("amber_g63_netif_probe.js");
    std::fs::write(
        &script,
        r#"
        const ni = require('os').networkInterfaces();
        const keys = Object.keys(ni);
        let hasLoop = false;
        for (const k of keys) {
          for (const e of ni[k]) {
            if (e.internal === true) hasLoop = true;
          }
        }
        console.log([keys.length > 0, hasLoop, typeof ni[keys[0]][0].address].join('|'));
        "#,
    )
    .expect("write probe");
    let output = Command::new(binary)
        .arg("run")
        .arg(&script)
        .output()
        .expect("amber run");
    let _ = std::fs::remove_file(&script);
    assert!(
        output.status.success(),
        "amber run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && *line != "[object Object]")
        .unwrap_or("");
    assert_eq!(line, "true|true|string");
}

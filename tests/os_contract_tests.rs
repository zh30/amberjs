//! Pins docs/OS_CONTRACT.md against src/nodejs_core/os.rs.
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

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

fn expected_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        "x86" => "ia32",
        other => other,
    }
}

fn expected_platform() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    }
}

fn expected_type() -> &'static str {
    if cfg!(windows) {
        "Windows_NT"
    } else if cfg!(target_os = "macos") {
        "Darwin"
    } else {
        "Linux"
    }
}

fn expected_eol_json() -> String {
    let eol = if cfg!(windows) { "\r\n" } else { "\n" };
    serde_json::to_string(eol).expect("eol json")
}

fn expected_tmpdir() -> String {
    if cfg!(windows) {
        std::env::var("TEMP").unwrap_or_else(|_| "C:\\Windows\\Temp".to_string())
    } else {
        "/tmp".to_string()
    }
}

#[test]
#[serial]
fn require_global_platform_arch_type_eol() {
    let code = r#"
        const os = require('os');
        const nodeOs = require('node:os');
        [
          os === globalThis.os,
          nodeOs === os,
          os.platform(),
          os.arch(),
          os.type(),
          JSON.stringify(os.EOL)
        ].join('|');
        "#;
    assert_eq!(
        run(code),
        format!(
            "true|true|{}|{}|{}|{}",
            expected_platform(),
            expected_arch(),
            expected_type(),
            expected_eol_json()
        )
    );
}

#[test]
#[serial]
fn release_uptime_homedir_tmpdir() {
    let tmpdir = expected_tmpdir();
    let code = r#"
        const os = require('os');
        [
          typeof os.release() === 'string' && os.release().length > 0,
          typeof os.uptime() === 'number' && os.uptime() >= 0,
          typeof os.homedir() === 'string' && os.homedir().length > 0,
          os.tmpdir()
        ].join('|');
        "#;
    assert_eq!(run(code), format!("true|true|true|{tmpdir}"));
}

#[test]
#[serial]
fn cpus_length_and_model() {
    let expected_len = num_cpus::get();
    let code = r#"
        const os = require('os');
        const cpus = os.cpus();
        [
          cpus.length,
          typeof cpus[0].model === 'string' && cpus[0].model.length > 0,
          cpus.every((c) => c.model === cpus[0].model)
        ].join('|');
        "#;
    assert_eq!(run(code), format!("{expected_len}|true|true"));
}

#[test]
#[serial]
fn freemem_and_totalmem() {
    let code = r#"
        const os = require('os');
        const free = os.freemem();
        const total = os.totalmem();
        [
          typeof free === 'number' && free >= 0,
          typeof total === 'number' && total >= 0,
          total === 0 || free <= total
        ].join('|');
        "#;
    assert_eq!(run(code), "true|true|true");
}

#[test]
#[serial]
fn constants_signals() {
    let code = r#"
        const os = require('os');
        const s = os.constants.signals;
        [
          s.SIGHUP,
          s.SIGINT,
          s.SIGQUIT,
          s.SIGKILL,
          s.SIGTERM,
          s.SIGUSR1,
          s.SIGUSR2,
          s.SIGCHLD,
          s.SIGWINCH
        ].join('|');
        "#;
    assert_eq!(run(code), "1|2|3|9|15|10|12|17|28");
}

#[test]
#[serial]
fn esm_named_exports_for_stable_surface() {
    let settled = r#"
        (async () => {
          const mod = await import('node:os');
          const os = require('os');
          const names = [
            'platform', 'arch', 'cpus', 'freemem', 'totalmem',
            'uptime', 'type', 'release', 'homedir', 'tmpdir'
          ];
          const namedOk = names.every((n) => typeof mod[n] === 'function' && mod[n] === os[n]);
          return [mod.default === os, namedOk].join('|');
        })()
        "#;
    assert_eq!(run(settled), "true|true");
}

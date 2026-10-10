//! Pins docs/PROCESS_CONTRACT.md (G22 basics + G44 stdout/stderr.write carve).
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

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

#[test]
#[serial]
fn require_process_is_global_process() {
    let code = r#"
        [
          require('process') === process,
          require('node:process') === process,
          typeof process.nextTick,
          typeof process.env,
          typeof process.cwd,
          typeof process.pid,
          typeof process.platform
        ].join('|');
        "#;
    assert_eq!(
        run(code),
        "true|true|function|object|function|number|string"
    );
}

#[test]
#[serial]
fn next_tick_runs_before_microtasks_with_args() {
    let code = r#"
        (async () => {
          const order = [];
          Promise.resolve().then(() => order.push('micro'));
          process.nextTick(() => order.push('tick'));
          process.nextTick((a, b) => order.push('args:' + a + ':' + b), 1, 'x');
          await new Promise((r) => setTimeout(r, 0));
          let bad = '';
          try { process.nextTick(1); } catch (e) { bad = e.name + ':' + String(e.message).includes('callback must be a function'); }
          return order.join(',') + '|' + bad;
        })()
        "#;
    assert_eq!(run(code), "tick,args:1:x,micro|TypeError:true");
}

#[test]
#[serial]
fn env_cwd_pid_platform() {
    let code = r#"
        process.env.AMBER_PROC_CONTRACT = 123;
        const coerced = process.env.AMBER_PROC_CONTRACT;
        const cwd = process.cwd();
        [
          typeof coerced,
          coerced,
          typeof cwd === 'string' && cwd.length > 0,
          typeof process.pid === 'number' && process.pid > 0,
          ['linux', 'darwin', 'win32', 'unknown'].includes(process.platform)
        ].join('|');
        "#;
    assert_eq!(run(code), "string|123|true|true|true");
}

#[test]
#[serial]
fn cli_process_basics_reach_the_contract() {
    let binary = env!("CARGO_BIN_EXE_amber");
    let dir = TempDir::new().expect("temp dir");
    let script = dir.path().join("probe.js");
    fs::write(
        &script,
        r#"
        let ran = false;
        process.nextTick(() => { ran = true; });
        setTimeout(() => {
          console.log([
            ran,
            typeof process.env,
            process.cwd().length > 0,
            process.pid > 0,
            typeof process.platform
          ].join('|'));
        }, 10);
        "#,
    )
    .expect("script");
    let output = Command::new(binary)
        .arg("run")
        .arg(&script)
        .output()
        .expect("amber run");
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
    assert_eq!(line, "true|object|true|true|string");
}

#[test]
#[serial]
fn stdout_stderr_write_are_functions_returning_true() {
    let code = r#"
        [
          typeof process.stdout.write,
          typeof process.stderr.write,
          process.stdout.write('') === true,
          process.stderr.write('') === true,
          process.stdout.write(42) === true,
          process.stderr.write(null) === true,
          process.stdout.write(undefined) === true
        ].join('|');
        "#;
    assert_eq!(run(code), "function|function|true|true|true|true|true");
}

#[test]
#[serial]
fn cli_stdout_write_reaches_host_stdout() {
    let binary = env!("CARGO_BIN_EXE_amber");
    let dir = TempDir::new().expect("temp dir");
    let script = dir.path().join("stdout_write.js");
    fs::write(
        &script,
        r#"
        const ok = process.stdout.write('AMBER_PROC_STDOUT_G44\n');
        if (ok !== true) throw new Error('stdout.write did not return true');
        "#,
    )
    .expect("script");
    let output = Command::new(binary)
        .arg("run")
        .arg(&script)
        .output()
        .expect("amber run");
    assert!(
        output.status.success(),
        "amber run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("AMBER_PROC_STDOUT_G44"),
        "expected host stdout marker, got: {stdout:?}"
    );
}

#[test]
#[serial]
fn cli_stderr_write_reaches_host_stderr() {
    let binary = env!("CARGO_BIN_EXE_amber");
    let dir = TempDir::new().expect("temp dir");
    let script = dir.path().join("stderr_write.js");
    fs::write(
        &script,
        r#"
        const ok = process.stderr.write('AMBER_PROC_STDERR_G44\n');
        if (ok !== true) throw new Error('stderr.write did not return true');
        "#,
    )
    .expect("script");
    let output = Command::new(binary)
        .arg("run")
        .arg(&script)
        .output()
        .expect("amber run");
    assert!(
        output.status.success(),
        "amber run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("AMBER_PROC_STDERR_G44"),
        "expected host stderr marker, got: {stderr:?}"
    );
}

#[test]
#[serial]
fn cli_stdout_write_coerces_number_and_buffer() {
    let binary = env!("CARGO_BIN_EXE_amber");
    let dir = TempDir::new().expect("temp dir");
    let script = dir.path().join("stdout_coerce.js");
    fs::write(
        &script,
        r#"
        process.stdout.write(7);
        process.stdout.write('|');
        process.stdout.write(Buffer.from('bufmark'));
        process.stdout.write('\n');
        "#,
    )
    .expect("script");
    let output = Command::new(binary)
        .arg("run")
        .arg(&script)
        .output()
        .expect("amber run");
    assert!(
        output.status.success(),
        "amber run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("7|bufmark"),
        "expected coerced stdout payload, got: {stdout:?}"
    );
}

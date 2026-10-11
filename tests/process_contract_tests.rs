//! Pins docs/PROCESS_CONTRACT.md (G22 basics + G44 stdout/stderr.write + G53 hrtime + G54 memoryUsage).
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

#[test]
#[serial]
fn hrtime_returns_epoch_split_array() {
    let code = r#"
        const t = process.hrtime();
        [
          typeof process.hrtime,
          Array.isArray(t),
          t.length,
          typeof t[0] === 'number' && t[0] >= 0,
          typeof t[1] === 'number' && t[1] >= 0 && t[1] < 1e9,
          // Epoch-based: whole seconds since Unix epoch are large (not process-relative).
          t[0] > 1_000_000_000
        ].join('|');
        "#;
    assert_eq!(run(code), "function|true|2|true|true|true");
}

#[test]
#[serial]
fn hrtime_previous_value_is_ignored() {
    // Honest Limit: no previous-value diff overload — args are ignored.
    let code = r#"
        const a = process.hrtime();
        const withPrev = process.hrtime(a);
        const withFake = process.hrtime([0, 0]);
        [
          Array.isArray(withPrev) && withPrev.length === 2,
          Array.isArray(withFake) && withFake.length === 2,
          // Diff overload would yield ~0; epoch absolute stays large.
          withPrev[0] > 1_000_000_000,
          withFake[0] > 1_000_000_000,
          // Not a tiny elapsed delta from a prior tuple.
          !(withPrev[0] === 0 && withPrev[1] < 1e9)
        ].join('|');
        "#;
    assert_eq!(run(code), "true|true|true|true|true");
}

#[test]
#[serial]
fn hrtime_bigint_returns_epoch_nanos() {
    let code = r#"
        const n = process.hrtime.bigint();
        const t = process.hrtime();
        const fromTuple = BigInt(t[0]) * 1000000000n + BigInt(t[1]);
        const delta = n > fromTuple ? n - fromTuple : fromTuple - n;
        [
          typeof process.hrtime.bigint,
          typeof n === 'bigint',
          n > 0n,
          // Same wall-clock source; allow a small scheduling skew.
          delta < 1000000000n
        ].join('|');
        "#;
    assert_eq!(run(code), "function|true|true|true");
}

#[test]
#[serial]
fn cli_hrtime_reaches_the_contract() {
    let binary = env!("CARGO_BIN_EXE_amber");
    let dir = TempDir::new().expect("temp dir");
    let script = dir.path().join("hrtime_probe.js");
    fs::write(
        &script,
        r#"
        const t = process.hrtime();
        const n = process.hrtime.bigint();
        const ignored = process.hrtime([1, 2]);
        console.log([
          typeof process.hrtime,
          Array.isArray(t) && t.length === 2,
          t[0] > 1_000_000_000,
          typeof n === 'bigint' && n > 0n,
          ignored[0] > 1_000_000_000
        ].join('|'));
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
    assert_eq!(line, "function|true|true|true|true");
}

#[test]
#[serial]
fn memory_usage_shape_and_numeric_fields() {
    let code = r#"
        const m = process.memoryUsage();
        [
          typeof process.memoryUsage,
          typeof m,
          typeof m.rss,
          typeof m.heapTotal,
          typeof m.heapUsed,
          typeof m.external,
          typeof m.arrayBuffers,
          typeof m.arrayBuffers.used,
          m.rss > 0,
          m.heapTotal >= 2 * 1024 * 1024,
          m.heapUsed >= 512 * 1024,
          m.heapUsed <= m.heapTotal,
          m.external === 0,
          m.arrayBuffers.used === 0
        ].join('|');
        "#;
    assert_eq!(
        run(code),
        "function|object|number|number|number|number|object|number|true|true|true|true|true|true"
    );
}

#[test]
#[serial]
fn memory_usage_heap_fields_follow_estimate_caps() {
    // Honesty pin: heapTotal/heapUsed are RSS-derived estimates with fixed caps
    // (not V8 HeapStatistics / Node heap parity). See PROCESS_CONTRACT Limits.
    let code = r#"
        const m = process.memoryUsage();
        const maxHeap = 100 * 1024 * 1024;
        const minHeap = 2 * 1024 * 1024;
        // Match process.rs: ((rss * 0.25).min(100MiB)).max(2MiB) as u64
        const expectedFromRss = Math.trunc(Math.max(minHeap, Math.min(maxHeap, m.rss * 0.25)));
        // Match process.rs: ((heapTotal as f64) / 2.0).max(512KiB) as u64
        const expectedUsed = Math.trunc(Math.max(512 * 1024, m.heapTotal / 2));
        [
          m.heapTotal === expectedFromRss,
          m.heapUsed === expectedUsed,
          m.external === 0,
          typeof m.arrayBuffers === 'object' && m.arrayBuffers.used === 0
        ].join('|');
        "#;
    assert_eq!(run(code), "true|true|true|true");
}

#[test]
#[serial]
fn cli_memory_usage_reaches_the_contract() {
    let binary = env!("CARGO_BIN_EXE_amber");
    let dir = TempDir::new().expect("temp dir");
    let script = dir.path().join("memory_usage.js");
    fs::write(
        &script,
        r#"
        const m = process.memoryUsage();
        console.log([
          typeof process.memoryUsage,
          m.rss > 0,
          m.heapTotal >= 2 * 1024 * 1024,
          m.heapUsed <= m.heapTotal,
          m.external === 0,
          typeof m.arrayBuffers === 'object' && m.arrayBuffers.used === 0
        ].join('|'));
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
    assert_eq!(line, "function|true|true|true|true|true");
}

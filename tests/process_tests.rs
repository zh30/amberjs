// Tests for process global object (v0.3.17)
// Tests for process.version, process.platform, process.env, process.argv, etc.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_process_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_process_version() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("process.version");
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    assert!(
        output.starts_with("v"),
        "Expected version to start with 'v', got: {}",
        output
    );
}

#[test]
#[serial]
fn test_process_versions_object() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.versions");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_process_versions_v8() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.versions.v8");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "string");
}

#[test]
#[serial]
fn test_process_versions_node() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.versions.node");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "string");
}

#[test]
#[serial]
fn test_process_versions_amberjs() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.versions.amberjs");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "string");
}

#[test]
#[serial]
fn test_process_platform() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("process.platform");
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    assert!(!output.is_empty(), "Expected platform to be non-empty");
}

#[test]
#[serial]
fn test_process_arch() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("process.arch");
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    assert!(!output.is_empty(), "Expected arch to be non-empty");
}

#[test]
#[serial]
fn test_process_pid() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.pid");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "number");
}

#[test]
#[serial]
fn test_process_pid_positive() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("process.pid > 0");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_process_title() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.title");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "string");
}

#[test]
#[serial]
fn test_process_env() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.env");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_process_env_path() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.env.PATH");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "string");
}

#[test]
#[serial]
fn test_process_argv() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("Array.isArray(process.argv)");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_process_argv_length() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("process.argv.length >= 0");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_process_exec_argv() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("Array.isArray(process.execArgv)");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_process_exec_path() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.execPath");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "string");
}

#[test]
#[serial]
fn test_process_cwd() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.cwd");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_process_cwd_returns_string() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.cwd()");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "string");
}

#[test]
#[serial]
fn test_process_chdir() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.chdir");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_process_memory_usage() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.memoryUsage");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_process_memory_usage_returns_object() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.memoryUsage()");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_process_uptime() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.uptime");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_process_uptime_returns_number() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.uptime()");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "number");
}

#[test]
#[serial]
fn test_process_uptime_positive() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("process.uptime() >= 0");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_process_hrtime() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.hrtime");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_process_hrtime_returns_array() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("Array.isArray(process.hrtime())");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_process_hrtime_length() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("process.hrtime().length");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "2");
}

#[test]
#[serial]
fn test_process_exit() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.exit");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_process_exit_code() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.exitCode");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "number");
}

#[test]
#[serial]
fn test_process_features() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.features");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_process_features_debug() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.features.debug");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "boolean");
}

#[test]
#[serial]
fn test_process_features_ipc() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof process.features.ipc");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "boolean");
}

#[test]
#[serial]
fn test_process_is_amberjs() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("process.isAmber === true");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_process_browser_false() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("process.browser === false");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

// Tests for process.release - temporarily disabled (not implemented)
//
// #[test]
// #[serial]
// fn test_process_release_object() {
//     let mut runtime = MinimalRuntime::new().unwrap();
//     let result = runtime.execute_code("typeof process.release");
//     assert!(result.is_ok());
//     assert_eq!(result.unwrap().trim(), "object");
// }
//
// #[test]
// #[serial]
// fn test_process_release_name() {
//     let mut runtime = MinimalRuntime::new().unwrap();
//     let result = runtime.execute_code("process.release.name");
//     assert!(result.is_ok());
//     let binding = result.unwrap();
//     let output = binding.trim();
//     assert!(!output.is_empty(), "Expected release name to be non-empty");
// }

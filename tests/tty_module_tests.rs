// Tests for tty module
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_tty_module_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof require('tty')");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_tty_isatty_function() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof require('tty').isatty");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_tty_isatty_invalid_fd() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("require('tty').isatty(-1)");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "false");
}

#[test]
#[serial]
fn test_tty_constructors() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof require('tty').ReadStream === 'function' && typeof require('tty').WriteStream === 'function'");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_process_stdout_tty_properties() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        "typeof process.stdout.isTTY === 'boolean' && typeof process.stdout.columns === 'number'",
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

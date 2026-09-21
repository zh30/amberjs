// v0.3.281: Test readline require() functionality
// Tests that readline module can be loaded via require()

use serial_test::serial;

#[test]
#[serial]
fn test_readline_global_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"'readline' in global"#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "readline should exist in global");
}

#[test]
#[serial]
fn test_readline_has_create_interface() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"typeof global.readline.createInterface"#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "readline.createInterface should be a function"
    );
}

#[test]
#[serial]
fn test_readline_require_returns_object() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const rl = require('readline');
        typeof rl
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "require('readline') should return an object"
    );
}

#[test]
#[serial]
fn test_readline_require_has_interface() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const rl = require('readline');
        rl && typeof rl.Interface
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "require('readline').Interface should be a function"
    );
}

#[test]
#[serial]
fn test_readline_require_has_create_interface() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const rl = require('readline');
        rl && typeof rl.createInterface
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "require('readline').createInterface should be a function"
    );
}

#[test]
#[serial]
fn test_readline_require_default() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const rl = require('readline');
        rl && typeof rl.default
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "require('readline').default should be an object"
    );
}

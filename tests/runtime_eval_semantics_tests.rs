use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn execute_code_does_not_increment_again_when_reading_result() {
    let mut runtime = MinimalRuntime::new().expect("runtime should initialize");

    let result = runtime
        .execute_code("let i = 0; i++; i;")
        .expect("script should execute");

    assert_eq!(result.trim(), "1");
}

#[test]
#[serial]
fn execute_code_does_not_replay_side_effectful_last_expression() {
    let mut runtime = MinimalRuntime::new().expect("runtime should initialize");

    let result = runtime
        .execute_code(
            r#"
var calls = 0;
var payload = {
    get x() {
        calls += 1;
        return calls;
    }
};
JSON.stringify(payload);
"#,
        )
        .expect("script should execute");

    assert_eq!(result.trim(), r#"{"x":1}"#);
}

#[test]
#[serial]
fn execute_code_returns_main_script_completion_value() {
    let mut runtime = MinimalRuntime::new().expect("runtime should initialize");

    let result = runtime
        .execute_code(
            r#"
let value = 0;
setTimeout(() => {
    value = 2;
}, 0);
value;
"#,
        )
        .expect("script should execute");

    assert_eq!(result.trim(), "0");
}

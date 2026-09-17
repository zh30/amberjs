use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn run_querystring_script(script: &str) -> String {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime
        .execute_code(script)
        .expect("querystring script should execute")
        .trim()
        .to_string()
}

#[test]
#[serial]
fn require_querystring_exposes_node_compatible_methods() {
    let output = run_querystring_script(
        r#"
        const qs = require('querystring');
        `${typeof qs}:${typeof qs.parse}:${typeof qs.stringify}:${typeof qs.escape}:${typeof qs.unescape}`;
        "#,
    );

    assert_eq!(output, "object:function:function:function:function");
}

#[test]
#[serial]
fn querystring_parse_decodes_values_and_preserves_repeated_keys() {
    let output = run_querystring_script(
        r#"
        const qs = require('querystring');
        const parsed = qs.parse('name=bee%20js&tag=runtime&tag=v8&empty=&encoded=a%2Bb');
        `${parsed.name}:${parsed.tag.join('|')}:${parsed.empty}:${parsed.encoded}`;
        "#,
    );

    assert_eq!(output, "bee js:runtime|v8::a+b");
}

#[test]
#[serial]
fn querystring_stringify_encodes_objects_and_arrays() {
    let output = run_querystring_script(
        r#"
        const qs = require('querystring');
        qs.stringify({
            name: 'bee js',
            tag: ['runtime', 'v8'],
            plus: 'a+b',
            enabled: true
        });
        "#,
    );

    assert_eq!(
        output,
        "name=bee%20js&tag=runtime&tag=v8&plus=a%2Bb&enabled=true"
    );
}

#[test]
#[serial]
fn querystring_escape_and_unescape_round_trip() {
    let output = run_querystring_script(
        r#"
        const qs = require('querystring');
        const escaped = qs.escape('bee js+a/b');
        `${escaped}:${qs.unescape(escaped)}`;
        "#,
    );

    assert_eq!(output, "bee%20js%2Ba%2Fb:bee js+a/b");
}

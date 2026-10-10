//! Stable contract pins for Node `querystring` (G38).
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn run_qs(script: &str) -> String {
    let mut runtime = MinimalRuntime::new().expect("runtime");
    runtime
        .execute_code(script)
        .expect("querystring contract script")
        .trim()
        .to_string()
}

#[test]
#[serial]
fn querystring_contract_require_surface() {
    let output = run_qs(
        r#"
        const qs = require('querystring');
        const qs2 = require('node:querystring');
        `${qs === qs2}:${typeof qs.parse}:${typeof qs.stringify}:${typeof qs.escape}:${typeof qs.unescape}`;
        "#,
    );
    assert_eq!(output, "true:function:function:function:function");
}

#[test]
#[serial]
fn querystring_contract_parse_decode_and_repeated_keys() {
    let output = run_qs(
        r#"
        const qs = require('querystring');
        const parsed = qs.parse('name=amber%20js&tag=runtime&tag=v8&empty=&encoded=a%2Bb');
        `${parsed.name}:${parsed.tag.join('|')}:${parsed.empty}:${parsed.encoded}`;
        "#,
    );
    assert_eq!(output, "amber js:runtime|v8::a+b");
}

#[test]
#[serial]
fn querystring_contract_stringify_encodes_objects_and_arrays() {
    let output = run_qs(
        r#"
        const qs = require('querystring');
        qs.stringify({
            name: 'amber js',
            tag: ['runtime', 'v8'],
            plus: 'a+b',
            enabled: true
        });
        "#,
    );
    assert_eq!(
        output,
        "name=amber%20js&tag=runtime&tag=v8&plus=a%2Bb&enabled=true"
    );
}

#[test]
#[serial]
fn querystring_contract_escape_unescape_round_trip() {
    let output = run_qs(
        r#"
        const qs = require('querystring');
        const escaped = qs.escape('amber js+a/b');
        `${escaped}:${qs.unescape(escaped)}`;
        "#,
    );
    assert_eq!(output, "amber%20js%2Ba%2Fb:amber js+a/b");
}

#[test]
#[serial]
fn querystring_contract_parse_plus_space_unescape_keeps_plus() {
    // Honesty pin: parse maps '+' → space; unescape does not.
    let output = run_qs(
        r#"
        const qs = require('querystring');
        const parsed = qs.parse('x=a+b');
        `${parsed.x}:${qs.unescape('a+b')}`;
        "#,
    );
    assert_eq!(output, "a b:a+b");
}

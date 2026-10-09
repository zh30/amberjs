//! Pins docs/ERROR_EVENT_CONTRACT.md (provisional G30).
//! Narrow ErrorEvent constructor + global onerror; not HTML document errors.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn run(code: &str) -> String {
    MinimalRuntime::new()
        .expect("runtime")
        .execute_code(code)
        .unwrap_or_else(|err| panic!("Execution failed: {err}"))
        .trim()
        .to_string()
}

#[test]
#[serial]
fn error_event_constructor_shape_and_defaults() {
    let out = run(
        r#"
        const e = new ErrorEvent('error', {
            message: 'Test error message',
            filename: 'test.js',
            lineno: 10,
            colno: 5,
            error: new Error('orig')
        });
        const d = new ErrorEvent('error');
        [
          typeof ErrorEvent,
          e.type === 'error',
          e.message === 'Test error message',
          e.filename === 'test.js',
          e.lineno === 10,
          e.colno === 5,
          e.error instanceof Error && e.error.message === 'orig',
          e.bubbles === false,
          e.cancelable === true,
          e.composed === false,
          e.defaultPrevented === false,
          e.isTrusted === false,
          d.message === '' && d.filename === '' && d.lineno === 0 && d.colno === 0 && d.error === null
        ].join('|');
        "#,
    );
    assert_eq!(
        out,
        "function|true|true|true|true|true|true|true|true|true|true|true|true",
        "got {out}"
    );
}

#[test]
#[serial]
fn error_event_type_fixed_and_non_error_first_arg_as_message() {
    let out = run(
        r#"
        const a = new ErrorEvent('oops');
        const b = new ErrorEvent('error', { message: 'from-init' });
        [
          a.type === 'error',
          a.message === 'oops',
          b.type === 'error',
          b.message === 'from-init'
        ].join('|');
        "#,
    );
    assert_eq!(out, "true|true|true|true", "got {out}");
}

#[test]
#[serial]
fn error_event_not_instanceof_event_or_errorevent() {
    let out = run(
        r#"
        const e = new ErrorEvent('error', { message: 'x' });
        [
          e instanceof Event,
          e instanceof ErrorEvent,
          typeof e.preventDefault,
          typeof e.stopPropagation
        ].join('|');
        "#,
    );
    assert_eq!(out, "false|false|undefined|undefined", "got {out}");
}

#[test]
#[serial]
fn global_onerror_default_and_window_alias() {
    let out = run(
        r#"
        [
          typeof onerror === 'function',
          typeof window !== 'undefined' && window === globalThis,
          typeof window.onerror === 'function'
        ].join('|');
        "#,
    );
    assert_eq!(out, "true|true|true", "got {out}");
}

#[test]
#[serial]
fn onerror_handles_uncaught_throw_when_returning_true() {
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let first = runtime
        .execute_code(
            r#"
            let caught = false;
            let received = '';
            let gotError = false;
            onerror = function(message, filename, lineno, colno, error) {
                caught = true;
                received = String(message);
                gotError = error instanceof Error;
                return true;
            };
            throw new Error('Specific contract error');
            "#,
        )
        .expect("throw with onerror=true should be handled");
    assert_eq!(first.trim(), "undefined");

    let check = runtime
        .execute_code(
            r#"
            [caught === true, received.includes('Specific contract error'), gotError === true].join('|');
            "#,
        )
        .expect("check");
    assert_eq!(check.trim(), "true|true|true", "got {}", check.trim());
}

#[test]
#[serial]
fn onerror_overwrite_and_false_does_not_swallow() {
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let _ = runtime
        .execute_code(
            r#"
            let calls = 0;
            onerror = function() { calls += 1; return false; };
            onerror = function() { calls += 10; return false; };
            "#,
        )
        .expect("setup");

    let result = runtime.execute_code("throw new Error('should surface');");
    assert!(
        result.is_err(),
        "onerror returning false should not swallow the throw"
    );

    let check = runtime
        .execute_code("String(calls)")
        .expect("calls");
    assert_eq!(check.trim(), "10", "only the last onerror should run, got {}", check.trim());
}

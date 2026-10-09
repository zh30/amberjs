//! Pins docs/TYPESCRIPT_CONTRACT.md (G36).
//! oxc transpile-only: erase / TSX classic / using emit / source-map stacks / fail-closed parse.
//! Not tsc type-check.

use std::process::Command;

use amberjs::runtime_minimal::MinimalRuntime;
use amberjs::typescript::compile_typescript;
use serial_test::serial;
use tempfile::tempdir;

fn amber() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

fn execute(code: &str) -> String {
    MinimalRuntime::new()
        .expect("runtime")
        .execute_code(code)
        .unwrap_or_else(|err| panic!("Execution failed: {err}"))
        .trim()
        .to_string()
}

fn assert_strips(js: &str, forbidden: &[&str]) {
    for token in forbidden {
        assert!(
            !js.contains(token),
            "transpiled JS still contains {token:?}:\n{js}"
        );
    }
}

#[test]
fn type_annotations_and_interfaces_are_erased() {
    let ts = r#"
interface Point { x: number; y: number }
type Id = number;
const p: Point = { x: 1, y: 2 };
const n: Id = 40;
p.x + p.y + n;
"#;
    let output = compile_typescript(ts, "erase.ts").expect("compile");
    assert_strips(
        &output.js_code,
        &["interface Point", "type Id", ": Point", ": Id", ": number"],
    );
    assert_eq!(execute(ts).trim(), "43");
}

#[test]
fn import_type_erased_unused_value_imports_kept() {
    let ts = r#"
import type { OnlyType } from "./types";
import unusedDefault from "./side-effect";
import { unusedNamed } from "./named";
const x: number = 1;
x;
"#;
    let output = compile_typescript(ts, "imports.ts").expect("compile");
    assert!(
        !output.js_code.contains("OnlyType") || !output.js_code.contains("from \"./types\""),
        "type-only import should be erased:\n{}",
        output.js_code
    );
    assert!(
        output.js_code.contains("./side-effect"),
        "unused default value import should stay:\n{}",
        output.js_code
    );
    assert!(
        output.js_code.contains("unusedNamed") && output.js_code.contains("./named"),
        "unused named value import should stay:\n{}",
        output.js_code
    );
}

#[test]
fn tsx_emits_classic_react_create_element() {
    let ts = r#"
const view = <div className="ok" />;
view;
"#;
    let output = compile_typescript(ts, "view.tsx").expect("compile");
    assert!(
        output.js_code.contains("React.createElement"),
        "classic JSX emit expected: {}",
        output.js_code
    );
    assert_strips(&output.js_code, &["<div"]);
}

#[test]
#[serial]
fn runtime_executes_tsx_with_caller_provided_react() {
    let ts = r#"
globalThis.React = {
    createElement(type) {
        return { type };
    }
};
const view = <span />;
view.type;
"#;
    assert_eq!(execute(ts), "span");
}

#[test]
#[serial]
fn using_declaration_downlevels_and_runs() {
    let ts = r#"
let disposed = false;
{
    using resource = {
        [Symbol.dispose]() {
            disposed = true;
        }
    };
    resource;
}
disposed;
"#;
    let output = compile_typescript(ts, "using.ts").expect("compile");
    assert_strips(&output.js_code, &["using resource"]);
    assert_eq!(execute(ts), "true");
}

#[test]
fn parse_failure_fails_closed_with_location() {
    let error = compile_typescript("const x: = 1;", "broken.ts").expect_err("should fail");
    assert!(
        error.contains("broken.ts"),
        "parse error should name the file: {error}"
    );
}

#[test]
fn intentional_type_mismatch_still_emits_and_runs() {
    // Honesty pin: transpile-only must NOT reject a type error that tsc would.
    let ts = r#"
const n: number = "not-a-number";
typeof n;
"#;
    let output = compile_typescript(ts, "no_typecheck.ts").expect("compile");
    assert!(
        output.diagnostics.is_empty(),
        "oxc product path must not claim tsc-style type diagnostics: {:?}",
        output.diagnostics
    );
    assert_eq!(execute(ts), "string");
}

#[test]
fn cli_throw_stack_names_original_ts_line() {
    let dir = tempdir().unwrap();
    let ts = dir.path().join("foo.ts");
    std::fs::write(
        &ts,
        r#"function fail(): never {
    throw new Error("mapped-boom");
}
fail();
"#,
    )
    .unwrap();

    let output = Command::new(amber())
        .arg("run")
        .arg(&ts)
        .output()
        .expect("amber run foo.ts");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.status.success(), "foo.ts should throw: {combined}");
    assert!(
        combined.contains("foo.ts:2") || combined.contains("foo.ts:2:"),
        "stack must name foo.ts:2 (throw line), got: {combined}"
    );
}

#[test]
#[serial]
fn require_loads_typescript_module_via_oxc() {
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("typed.ts"),
        "const n: number = 9; module.exports = { n };",
    )
    .expect("ts");
    let root = dir.path().display().to_string().replace('\\', "/");
    let code = format!(
        r#"
        globalThis.__dirname = '{root}';
        require('./typed.ts').n;
        "#
    );
    assert_eq!(execute(&code), "9");
}

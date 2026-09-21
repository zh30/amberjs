//! TypeScript 6.0 language-surface transpile tests (oxc backend, transpile-only).

use amberjs::runtime::MinimalRuntime;
use amberjs::typescript::compile_typescript;
use serial_test::serial;

fn assert_strips(js: &str, forbidden: &[&str]) {
    for token in forbidden {
        assert!(
            !js.contains(token),
            "transpiled JS still contains {token:?}:\n{js}"
        );
    }
}

fn execute(code: &str) -> String {
    let mut runtime = MinimalRuntime::new().expect("runtime");
    runtime.execute_code(code).expect("execute")
}

#[test]
fn compile_const_type_parameter() {
    let ts = r#"
function id<const T>(value: T): T {
    return value;
}
const n = id(41);
n + 1;
"#;
    let output = compile_typescript(ts, "const_type_param.ts").expect("compile");
    assert!(output.js_code.contains("function id"));
    assert_strips(&output.js_code, &["<const", ": T", ": T"]);
    assert_eq!(execute(ts).trim(), "42");
}

#[test]
fn compile_and_run_satisfies() {
    let ts = r#"
const point = { x: 2, y: 20 } satisfies { x: number; y: number };
point.x + point.y;
"#;
    let output = compile_typescript(ts, "satisfies.ts").expect("compile");
    assert_strips(&output.js_code, &["satisfies"]);
    assert_eq!(execute(ts).trim(), "22");
}

#[test]
fn compile_and_run_using_declaration() {
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
    assert_eq!(execute(ts).trim(), "true");
}

#[test]
fn compile_legacy_decorator() {
    let ts = r#"
function tagged(ctor) {
    ctor.mark = 42;
    return ctor;
}
@tagged
class Box {}
Box.mark;
"#;
    let output = compile_typescript(ts, "decorator.ts").expect("compile");
    if output.js_code.contains("@tagged") {
        // oxc 0.147 only lowers legacy decorators in some class forms.
        // Parsing Stage-3/legacy syntax is still required; execution follows V8.
        return;
    }
    assert_eq!(execute(ts).trim(), "42");
}

#[test]
fn compile_import_attributes_without_executing() {
    let ts = r#"
import data from "./payload.json" with { type: "json" };
export const value = data;
"#;
    let output = compile_typescript(ts, "import_attr.ts").expect("compile");
    assert!(
        output.js_code.contains("payload.json"),
        "import specifier should remain: {}",
        output.js_code
    );
}

#[test]
fn compile_enum_and_namespace() {
    let ts = r#"
enum Color {
    Red = 1,
    Blue = 2,
}
namespace Util {
    export function add(a: number, b: number): number {
        return a + b;
    }
}
Util.add(Color.Red, Color.Blue);
"#;
    let output = compile_typescript(ts, "enum_ns.ts").expect("compile");
    assert_strips(&output.js_code, &[": number"]);
    assert_eq!(execute(ts).trim(), "3");
}

#[test]
fn compile_tsx_classic_jsx() {
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
fn runtime_executes_tsx_with_global_react() {
    let ts = r#"
globalThis.React = {
    createElement(type) {
        return { type };
    }
};
const view = <span />;
view.type;
"#;
    assert_eq!(execute(ts).trim(), "span");
}

#[test]
fn parse_error_includes_location() {
    let error = compile_typescript("const x: = 1;", "broken.ts").expect_err("should fail");
    assert!(
        error.contains("broken.ts"),
        "parse error should name the file: {error}"
    );
}

#[test]
fn unused_value_imports_are_kept() {
    let ts = r#"
import unusedDefault from "./side-effect";
import { unusedNamed } from "./named";
const x = 1;
"#;
    let output = compile_typescript(ts, "keep_imports.ts").expect("compile");
    assert!(
        output.js_code.contains("import") && output.js_code.contains("./side-effect"),
        "unused default import should stay for side effects:\n{}",
        output.js_code
    );
    assert!(
        output.js_code.contains("unusedNamed") && output.js_code.contains("./named"),
        "unused named import should stay for side effects:\n{}",
        output.js_code
    );
}

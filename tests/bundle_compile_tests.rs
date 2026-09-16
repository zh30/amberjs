// Production-grade Bundler 2.0 & SEA Compiler Integration Tests
// Tests `bee bundle` (oxc-backed AST bundling) and `bee compile` (Single Executable Application)

use beejs::runtime_minimal::MinimalRuntime;
use beejs::tooling::bundler::{bundle_project, BundleOptions};
use beejs::tooling::compiler::compile_binary;
use serial_test::serial;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
#[serial]
fn test_oxc_bundle_typescript_multiple_files() {
    let dir = tempdir().expect("tempdir");

    let math_ts = dir.path().join("math.ts");
    fs::write(
        &math_ts,
        r#"
export function add(a: number, b: number): number {
    return a + b;
}
export function multiply(a: number, b: number): number {
    return a * b;
}
export const PI: number = 3.14159;
"#,
    )
    .expect("write math.ts");

    let greeter_ts = dir.path().join("greeter.ts");
    fs::write(
        &greeter_ts,
        r#"
export class Greeter {
    constructor(private name: string) {}
    greet(): string {
        return `Hello, ${this.name}!`;
    }
}
export default function welcome(name: string): string {
    return new Greeter(name).greet();
}
"#,
    )
    .expect("write greeter.ts");

    let entry_ts = dir.path().join("index.ts");
    fs::write(
        &entry_ts,
        r#"
import { add, multiply, PI } from "./math.ts";
import welcome, { Greeter } from "./greeter.ts";

const sum = add(10, 20);
const prod = multiply(5, 6);
const msg = welcome("Beejs");
const g = new Greeter("World");

const outputStr = `SUM:${sum}|PROD:${prod}|PI:${PI}|MSG:${msg}|GREET:${g.greet()}`;
console.log(outputStr);
module.exports = outputStr;
"#,
    )
    .expect("write index.ts");

    let outfile = dir.path().join("bundle.js");
    let options = BundleOptions {
        entry: entry_ts,
        outfile: Some(outfile.clone()),
        minify: false,
        sourcemap: true,
        target: "es2022".to_string(),
        import_map: None,
    };

    let result = bundle_project(&options).expect("bundle_project should succeed");
    assert_eq!(result.module_count, 3);
    assert!(result.code.contains("__beejs_require__"));
    assert!(outfile.exists());

    let map_path = dir.path().join("bundle.map");
    assert!(map_path.exists());

    // Execute the bundled JavaScript in runtime
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let output = runtime
        .execute_code(&result.code)
        .expect("Execution should succeed");

    assert!(output.contains("SUM:30|PROD:30|PI:3.14159|MSG:Hello, Beejs!|GREET:Hello, World!"));
}

#[test]
#[serial]
fn test_oxc_bundle_with_json_and_commonjs() {
    let dir = tempdir().expect("tempdir");

    let config_json = dir.path().join("config.json");
    fs::write(
        &config_json,
        r#"{ "appName": "BeeBundleTest", "version": "2.0.0", "port": 8080 }"#,
    )
    .expect("write config.json");

    let legacy_js = dir.path().join("legacy.js");
    fs::write(
        &legacy_js,
        r#"
function getPlatform() {
    return "Beejs-Runtime";
}
module.exports = { getPlatform };
"#,
    )
    .expect("write legacy.js");

    let entry_ts = dir.path().join("main.ts");
    fs::write(
        &entry_ts,
        r#"
import config from "./config.json";
const legacy = require("./legacy.js");

const outputStr = `APP:${config.appName}|VER:${config.version}|PORT:${config.port}|PLATFORM:${legacy.getPlatform()}`;
console.log(outputStr);
module.exports = outputStr;
"#,
    )
    .expect("write main.ts");

    let outfile = dir.path().join("bundle.js");
    let options = BundleOptions {
        entry: entry_ts,
        outfile: Some(outfile.clone()),
        minify: false,
        sourcemap: false,
        target: "es2022".to_string(),
        import_map: None,
    };

    let result = bundle_project(&options).expect("bundle_project");
    assert_eq!(result.module_count, 3);

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let output = runtime
        .execute_code(&result.code)
        .expect("Execution should succeed");

    assert!(output.contains("APP:BeeBundleTest|VER:2.0.0|PORT:8080|PLATFORM:Beejs-Runtime"));
}

#[test]
#[serial]
fn test_sea_compile_and_execute_standalone_binary() {
    let dir = tempdir().expect("tempdir");

    let helper_ts = dir.path().join("calc.ts");
    fs::write(
        &helper_ts,
        r#"
export function compute(x: number, y: number): number {
    return (x * y) + 10;
}
"#,
    )
    .expect("write calc.ts");

    let main_ts = dir.path().join("main.ts");
    fs::write(
        &main_ts,
        r#"
import { compute } from "./calc.ts";
const val = compute(5, 6);
const userArgs = process.argv.slice(2).join(",");
console.log(`SEA_STANDALONE_SUCCESS: val=${val} args=${userArgs}`);
"#,
    )
    .expect("write main.ts");

    #[cfg(windows)]
    let out_bin = dir.path().join("my_standalone_app.exe");
    #[cfg(not(windows))]
    let out_bin = dir.path().join("my_standalone_app");

    // Compile into standalone executable
    compile_binary(&main_ts, &out_bin).expect("compile_binary should succeed");
    assert!(out_bin.exists(), "Compiled binary must exist");

    // Execute standalone binary as child process
    let output = Command::new(&out_bin)
        .args(["alpha", "beta", "gamma"])
        .output()
        .expect("Failed to execute compiled binary");

    assert!(
        output.status.success(),
        "Binary execution failed with code: {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("SEA_STANDALONE_SUCCESS: val=40 args=alpha,beta,gamma"),
        "Stdout mismatch: {}",
        stdout
    );
}

#[test]
#[serial]
fn test_bundle_minify_compression() {
    let dir = tempdir().expect("tempdir");

    let entry_ts = dir.path().join("verbose.ts");
    fs::write(
        &entry_ts,
        r#"
// This is a long explanatory comment that should be stripped
function calculate(x: number, y: number): number {
    // Another inner comment
    const result = x + y;
    return result;
}
console.log(calculate(100, 200));
"#,
    )
    .expect("write verbose.ts");

    let unminified_out = dir.path().join("unminified.js");
    let unminified = bundle_project(&BundleOptions {
        entry: entry_ts.clone(),
        outfile: Some(unminified_out),
        minify: false,
        sourcemap: false,
        target: "es2022".to_string(),
        import_map: None,
    })
    .expect("unminified bundle");

    let minified_out = dir.path().join("minified.js");
    let minified = bundle_project(&BundleOptions {
        entry: entry_ts,
        outfile: Some(minified_out),
        minify: true,
        sourcemap: false,
        target: "es2022".to_string(),
        import_map: None,
    })
    .expect("minified bundle");

    assert!(minified.total_bytes > 0);
    assert!(unminified.total_bytes > 0);
    assert!(!minified
        .code
        .contains("// This is a long explanatory comment"));
}

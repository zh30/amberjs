// Pins the Stable `amber bundle` compatibility contract in docs/BUNDLE_CONTRACT.md.
// CI runs this target explicitly so the written contract cannot silently regress.

use amberjs::tooling::bundler::{bundle_project, BundleOptions, BUNDLE_ERROR_PREFIX};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn amber_path() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, contents).expect("write fixture");
}

fn combined(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn run_bundle(args: &[&str]) -> std::process::Output {
    Command::new(amber_path())
        .arg("bundle")
        .args(args)
        .output()
        .expect("failed to execute amber bundle")
}

fn assert_stable_error(output: &std::process::Output, needles: &[&str]) {
    let text = combined(output);
    assert!(
        !output.status.success(),
        "expected amber bundle to fail. output: {text}"
    );
    assert!(
        text.contains(BUNDLE_ERROR_PREFIX),
        "failure must use {BUNDLE_ERROR_PREFIX}. output: {text}"
    );
    for needle in needles {
        assert!(
            text.contains(needle),
            "failure must contain {needle:?}. output: {text}"
        );
    }
}

#[test]
fn contract_entry_missing_uses_stable_diagnostic() {
    let dir = tempdir().expect("tempdir");
    let missing = dir.path().join("nope.js");
    let outfile = dir.path().join("out.js");
    let output = run_bundle(&[
        missing.to_str().unwrap(),
        "--outfile",
        outfile.to_str().unwrap(),
    ]);
    assert_stable_error(&output, &["entry file not found"]);
    assert!(!outfile.exists(), "failed bundle must not write output");
}

#[test]
fn contract_relative_unresolved_fails_before_write() {
    let dir = tempdir().expect("tempdir");
    let entry = dir.path().join("entry.js");
    let outfile = dir.path().join("dist").join("bundle.js");
    write(
        &entry,
        "import { x } from './missing.js';\nconsole.log(x);\n",
    );
    let output = run_bundle(&[
        entry.to_str().unwrap(),
        "--outfile",
        outfile.to_str().unwrap(),
    ]);
    assert_stable_error(&output, &["cannot resolve", "./missing.js"]);
    assert!(!outfile.exists(), "failed relative resolve must not write");
}

#[test]
fn contract_missing_named_export_fails_before_write() {
    let dir = tempdir().expect("tempdir");
    let dep = dir.path().join("dep.js");
    let entry = dir.path().join("entry.js");
    let outfile = dir.path().join("bundle.js");
    write(&dep, "export const present = 1;\n");
    write(
        &entry,
        "import { absent } from './dep.js';\nconsole.log(absent);\n",
    );
    let output = run_bundle(&[
        entry.to_str().unwrap(),
        "--outfile",
        outfile.to_str().unwrap(),
    ]);
    assert_stable_error(&output, &["missing export", "'absent'", "./dep.js"]);
    assert!(!outfile.exists());
}

#[test]
fn contract_missing_reexport_fails_before_write() {
    let dir = tempdir().expect("tempdir");
    let leaf = dir.path().join("leaf.js");
    let barrel = dir.path().join("barrel.js");
    let entry = dir.path().join("entry.js");
    let outfile = dir.path().join("bundle.js");
    write(&leaf, "export const present = 1;\n");
    write(&barrel, "export { ghost } from './leaf.js';\n");
    write(&entry, "import { ghost } from './barrel.js';\n");
    let output = run_bundle(&[
        entry.to_str().unwrap(),
        "--outfile",
        outfile.to_str().unwrap(),
    ]);
    assert_stable_error(&output, &["missing export", "cannot re-export", "'ghost'"]);
    assert!(!outfile.exists());
}

#[test]
fn contract_unsupported_asset_fails() {
    let dir = tempdir().expect("tempdir");
    let css = dir.path().join("app.css");
    let entry = dir.path().join("entry.js");
    let outfile = dir.path().join("bundle.js");
    write(&css, "body{}\n");
    write(&entry, "import './app.css';\n");
    let output = run_bundle(&[
        entry.to_str().unwrap(),
        "--outfile",
        outfile.to_str().unwrap(),
    ]);
    assert_stable_error(&output, &["unsupported module", "app.css"]);
    assert!(!outfile.exists());
}

#[test]
fn contract_esm_cjs_json_graph_runs() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("src");
    let dist = dir.path().join("dist");
    write(
        &src.join("config.json"),
        r#"{ "name": "contract", "n": 7 }"#,
    );
    write(
        &src.join("legacy.cjs"),
        "function tag() { return 'cjs'; }\nmodule.exports = { tag };\n",
    );
    write(
        &src.join("math.ts"),
        "export function add(a: number, b: number): number { return a + b; }\n",
    );
    write(
        &src.join("entry.ts"),
        r#"
import { add } from "./math.ts";
import config from "./config.json";
const legacy = require("./legacy.cjs");
console.log(`bundle:${config.name}:${add(config.n, 1)}:${legacy.tag()}`);
"#,
    );
    let outfile = dist.join("bundle.js");
    let bundle = run_bundle(&[
        src.join("entry.ts").to_str().unwrap(),
        "-o",
        outfile.to_str().unwrap(),
    ]);
    let text = combined(&bundle);
    assert!(
        bundle.status.success(),
        "graph bundle should succeed: {text}"
    );
    assert!(outfile.exists());

    let run = Command::new(amber_path())
        .arg("run")
        .arg(&outfile)
        .output()
        .expect("run bundled output");
    let run_text = combined(&run);
    assert!(
        run.status.success(),
        "bundled output should run: {run_text}"
    );
    assert!(
        run_text.contains("bundle:contract:8:cjs"),
        "ESM + CJS + JSON contract output mismatch: {run_text}"
    );
}

#[test]
fn contract_bare_specifier_is_runtime_external() {
    let dir = tempdir().expect("tempdir");
    let entry = dir.path().join("entry.js");
    let outfile = dir.path().join("bundle.js");
    write(
        &entry,
        "const path = require('path');\nconsole.log('ext:' + path.basename('a/b/c.txt'));\n",
    );
    let bundle = run_bundle(&[
        entry.to_str().unwrap(),
        "--outfile",
        outfile.to_str().unwrap(),
    ]);
    let text = combined(&bundle);
    assert!(
        bundle.status.success(),
        "external bundle should succeed: {text}"
    );
    let code = fs::read_to_string(&outfile).expect("read bundle");
    assert!(
        code.contains("require('path')"),
        "bare specifier must remain a runtime external: {code}"
    );

    let run = Command::new(amber_path())
        .arg("run")
        .arg(&outfile)
        .output()
        .expect("run");
    let run_text = combined(&run);
    assert!(
        run.status.success(),
        "external require should run: {run_text}"
    );
    assert!(run_text.contains("ext:c.txt"), "{run_text}");
}

#[test]
fn contract_sourcemap_is_inventory_only() {
    let dir = tempdir().expect("tempdir");
    let helper = dir.path().join("helper.js");
    let entry = dir.path().join("entry.js");
    let outfile = dir.path().join("bundle.js");
    write(&helper, "export const v = 1;\n");
    write(
        &entry,
        "import { v } from './helper.js';\nconsole.log(v);\n",
    );
    let bundle = run_bundle(&[
        entry.to_str().unwrap(),
        "--outfile",
        outfile.to_str().unwrap(),
        "--sourcemap",
    ]);
    let text = combined(&bundle);
    assert!(bundle.status.success(), "{text}");

    // with_extension("map") on bundle.js → bundle.map (not bundle.js.map)
    let map_path = dir.path().join("bundle.map");
    assert!(
        map_path.exists(),
        "sourcemap must be written next to outfile as *.map"
    );
    let map: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&map_path).expect("read map")).expect("json");
    assert_eq!(map["version"], 3);
    assert_eq!(map["mappings"], "");
    let sources = map["sources"]
        .as_array()
        .expect("sources array")
        .iter()
        .map(|s| s.as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert!(
        sources.iter().any(|s| s.ends_with("entry.js")),
        "sources must list the entry: {sources:?}"
    );
    assert!(
        sources.iter().any(|s| s.ends_with("helper.js")),
        "sources must list bundled deps: {sources:?}"
    );
    let code = fs::read_to_string(&outfile).expect("read bundle");
    assert!(
        !code.contains("sourceMappingURL"),
        "inventory sourcemap does not append sourceMappingURL"
    );
}

#[test]
fn contract_default_outfile_is_bundle_js() {
    let dir = tempdir().expect("tempdir");
    let entry = dir.path().join("app.js");
    write(&entry, "console.log('default-out');\n");
    let bundle = run_bundle(&[entry.to_str().unwrap()]);
    let text = combined(&bundle);
    assert!(bundle.status.success(), "{text}");
    let expected = dir.path().join("app.bundle.js");
    assert!(
        expected.exists(),
        "default outfile should be <stem>.bundle.js"
    );
}

#[test]
fn contract_import_map_remaps_bare_specifier() {
    let dir = tempdir().expect("tempdir");
    write(
        &dir.path().join("import_map.json"),
        r#"{ "imports": { "label": "./label.js" } }"#,
    );
    write(
        &dir.path().join("label.js"),
        "export const message = 'mapped';\n",
    );
    write(
        &dir.path().join("entry.js"),
        "import { message } from 'label';\nconsole.log('bundle:' + message);\n",
    );
    let outfile = dir.path().join("bundle.js");
    let bundle = run_bundle(&[
        "--import-map",
        dir.path().join("import_map.json").to_str().unwrap(),
        dir.path().join("entry.js").to_str().unwrap(),
        "--outfile",
        outfile.to_str().unwrap(),
    ]);
    let text = combined(&bundle);
    assert!(
        bundle.status.success(),
        "import-map bundle should succeed: {text}"
    );

    let run = Command::new(amber_path())
        .arg("run")
        .arg(&outfile)
        .output()
        .expect("run");
    let run_text = combined(&run);
    assert!(run.status.success(), "{run_text}");
    assert!(run_text.contains("bundle:mapped"), "{run_text}");
}

#[test]
fn contract_node_modules_main_is_inlined() {
    let dir = tempdir().expect("tempdir");
    write(
        &dir.path().join("node_modules/leftpad/package.json"),
        r#"{ "name": "leftpad", "main": "index.js" }"#,
    );
    write(
        &dir.path().join("node_modules/leftpad/index.js"),
        "module.exports = function (s) { return '0' + s; };\n",
    );
    write(
        &dir.path().join("entry.js"),
        "const pad = require('leftpad');\nconsole.log('bundle:' + pad('7'));\n",
    );
    let outfile = dir.path().join("bundle.js");
    let bundle = run_bundle(&[
        dir.path().join("entry.js").to_str().unwrap(),
        "--outfile",
        outfile.to_str().unwrap(),
    ]);
    let text = combined(&bundle);
    assert!(bundle.status.success(), "{text}");
    let code = fs::read_to_string(&outfile).expect("read");
    assert!(
        code.contains("0") && !code.contains("require('leftpad')"),
        "package.json main should be inlined, not left as an external: {code}"
    );

    let run = Command::new(amber_path())
        .arg("run")
        .arg(&outfile)
        .output()
        .expect("run");
    let run_text = combined(&run);
    assert!(run.status.success(), "{run_text}");
    assert!(run_text.contains("bundle:07"), "{run_text}");
}

#[test]
fn contract_tree_shake_is_accepted_noop() {
    let dir = tempdir().expect("tempdir");
    write(
        &dir.path().join("lib.js"),
        "export const used = 'keep';\nexport const unused = 'still-here';\n",
    );
    write(
        &dir.path().join("entry.js"),
        "import { used } from './lib.js';\nconsole.log(used);\n",
    );
    let outfile = dir.path().join("bundle.js");
    let bundle = run_bundle(&[
        dir.path().join("entry.js").to_str().unwrap(),
        "--outfile",
        outfile.to_str().unwrap(),
        "--tree-shake",
    ]);
    let text = combined(&bundle);
    assert!(
        bundle.status.success(),
        "--tree-shake must be accepted: {text}"
    );
    let code = fs::read_to_string(&outfile).expect("read");
    assert!(
        code.contains("still-here"),
        "--tree-shake is a no-op; unused exports stay in the graph: {code}"
    );
}

#[test]
fn contract_target_is_header_comment_only() {
    let dir = tempdir().expect("tempdir");
    write(&dir.path().join("entry.js"), "console.log('target');\n");
    let outfile = dir.path().join("bundle.js");
    let bundle = run_bundle(&[
        dir.path().join("entry.js").to_str().unwrap(),
        "--outfile",
        outfile.to_str().unwrap(),
        "--target",
        "node",
    ]);
    assert!(bundle.status.success(), "{}", combined(&bundle));
    let code = fs::read_to_string(&outfile).expect("read");
    assert!(
        code.contains("Target: node"),
        "--target is recorded in the bundle header only: {code}"
    );
}

#[test]
fn contract_library_errors_share_cli_prefix() {
    let dir = tempdir().expect("tempdir");
    let err = bundle_project(&BundleOptions {
        entry: PathBuf::from("/definitely/missing/amber-bundle-contract.js"),
        outfile: None,
        minify: false,
        sourcemap: false,
        target: "es2022".to_string(),
        import_map: None,
    })
    .unwrap_err()
    .to_string();
    assert!(
        err.starts_with(BUNDLE_ERROR_PREFIX),
        "library diagnostics must match CLI prefix: {err}"
    );
    assert!(err.contains("entry file not found"), "{err}");
}

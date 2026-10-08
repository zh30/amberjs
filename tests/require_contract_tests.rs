//! Pins docs/REQUIRE_CONTRACT.md for CommonJS require and Node-shaped resolution.
use amberjs::nodejs_core::commonjs_resolver::{resolve_commonjs_module, ResolvedModule};
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn runtime() -> MinimalRuntime {
    MinimalRuntime::new().expect("Failed to create runtime")
}

fn run(code: &str) -> String {
    runtime()
        .execute_code(code)
        .unwrap_or_else(|err| panic!("Execution failed: {err}"))
        .trim()
        .to_string()
}

fn js_path(path: &std::path::Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
}

fn resolved_file(module: ResolvedModule) -> std::path::PathBuf {
    match module {
        ResolvedModule::File(path) => path,
        ResolvedModule::Builtin(name) => panic!("expected file, got builtin {name}"),
    }
}

fn resolved_builtin(module: ResolvedModule) -> String {
    match module {
        ResolvedModule::Builtin(name) => name,
        ResolvedModule::File(path) => panic!("expected builtin, got {}", path.display()),
    }
}

#[test]
#[serial]
fn require_module_exports_and_main_bindings() {
    let dir = TempDir::new().expect("temp");
    let entry = dir.path().join("entry.js");
    fs::write(&entry, "").expect("entry");
    let mut rt = runtime();
    rt.set_main_module_path(&entry);
    let code = r#"
        [
          typeof require,
          module.exports === exports,
          require.main === module,
          typeof require.resolve,
          typeof module.createRequire,
          typeof require('module').createRequire,
          typeof require.cache,
          Array.isArray(module.children),
          module.children.length,
          module.parent === null
        ].join('|');
    "#;
    let result = rt
        .execute_code(code)
        .unwrap_or_else(|err| panic!("Execution failed: {err}"));
    assert_eq!(
        result.trim(),
        "function|true|true|function|function|function|undefined|true|0|true"
    );
}

#[test]
#[serial]
fn module_exports_reassign_and_identity_cache() {
    let dir = TempDir::new().expect("temp");
    let root = js_path(dir.path());
    fs::write(
        dir.path().join("reassign.js"),
        "module.exports = { ok: true }; exports.nope = 1;",
    )
    .expect("reassign");
    fs::write(
        dir.path().join("counter.js"),
        "let n = 0; module.exports = { tick() { return ++n; } };",
    )
    .expect("counter");
    let code = format!(
        r#"
        globalThis.__dirname = '{root}';
        const a = require('./reassign.js');
        const once = require('./counter.js');
        const twice = require('./counter.js');
        [JSON.stringify(a), once === twice, once.tick(), twice.tick()].join('|');
        "#
    );
    assert_eq!(run(&code), r#"{"ok":true}|true|1|2"#);
}

#[test]
#[serial]
fn nested_require_uses_module_dirname_not_mutated_global() {
    let dir = TempDir::new().expect("temp");
    let nested = dir.path().join("nested");
    fs::create_dir_all(&nested).expect("nested");
    fs::write(nested.join("sibling.js"), "module.exports = { v: 11 };").expect("sibling");
    fs::write(
        nested.join("loader.js"),
        r#"
        globalThis.__dirname = '/tmp/wrong';
        module.exports = require('./sibling.js');
        "#,
    )
    .expect("loader");
    let root = js_path(dir.path());
    let code = format!(
        r#"
        globalThis.__dirname = '{root}';
        require('./nested/loader.js').v;
        "#
    );
    assert_eq!(run(&code), "11");
}

#[test]
#[serial]
fn resolve_builtins_and_unknown_node_prefix() {
    assert_eq!(
        resolved_builtin(resolve_commonjs_module("node:fs", std::path::Path::new(".")).unwrap()),
        "fs"
    );
    let code = r#"
        const missing = (() => {
          try { require('node:not-a-real-builtin'); return 'ok'; }
          catch (e) { return [e.name, e.code === undefined, e.message].join(':'); }
        })();
        [require.resolve('fs'), require.resolve('node:path'), missing].join('|');
    "#;
    assert_eq!(
        run(code),
        "fs|path|Error:true:Cannot find module 'node:not-a-real-builtin'"
    );
}

#[test]
#[serial]
fn builtin_fs_and_path_objects_load_without_graduating_their_apis() {
    let code = r#"
        [
          typeof require('fs').readFileSync,
          typeof require('node:fs').readFileSync,
          typeof require('path').join,
          require('path').join('a', 'b')
        ].join('|');
    "#;
    assert_eq!(run(code), "function|function|function|a/b");
}

#[test]
#[serial]
fn relative_json_typescript_and_directory_index() {
    let dir = TempDir::new().expect("temp");
    let lib = dir.path().join("lib");
    fs::create_dir_all(&lib).expect("lib");
    fs::write(dir.path().join("data.json"), r#"{"x":3}"#).expect("json");
    fs::write(
        dir.path().join("typed.ts"),
        "const n: number = 9; module.exports = { n };",
    )
    .expect("ts");
    fs::write(lib.join("index.js"), "exports.value = 99;").expect("index");
    let root = js_path(dir.path());
    let code = format!(
        r#"
        globalThis.__dirname = '{root}';
        [require('./data.json').x, require('./typed.ts').n, require('./lib').value].join('|');
        "#
    );
    assert_eq!(run(&code), "3|9|99");
}

#[test]
#[serial]
fn package_main_exports_conditions_and_ignores_module_field() {
    let dir = TempDir::new().expect("temp");
    let app = dir.path().join("app");
    let pkg = app.join("node_modules/pkg");
    let cond = app.join("node_modules/cond");
    let modfield = app.join("node_modules/modfield");
    fs::create_dir_all(pkg.join("dist")).expect("pkg");
    fs::create_dir_all(&cond).expect("cond");
    fs::create_dir_all(modfield.join("dist")).expect("modfield");
    fs::write(
        pkg.join("package.json"),
        r#"{"name":"pkg","main":"dist/main.js"}"#,
    )
    .unwrap();
    fs::write(
        pkg.join("dist/main.js"),
        "module.exports = { answer: 123 };",
    )
    .unwrap();
    fs::write(
        cond.join("package.json"),
        r#"{"name":"cond","exports":{".":{"require":"./cjs.js","import":"./esm.js","default":"./def.js"}}}"#,
    )
    .unwrap();
    fs::write(cond.join("cjs.js"), "module.exports = { mode: 'cjs' };").unwrap();
    fs::write(cond.join("esm.js"), "module.exports = { mode: 'esm' };").unwrap();
    fs::write(cond.join("def.js"), "module.exports = { mode: 'def' };").unwrap();
    fs::write(
        modfield.join("package.json"),
        r#"{"name":"modfield","module":"dist/esm.js","main":"dist/cjs.js"}"#,
    )
    .unwrap();
    fs::write(
        modfield.join("dist/cjs.js"),
        "module.exports = { mode: 'cjs' };",
    )
    .unwrap();
    fs::write(
        modfield.join("dist/esm.js"),
        "module.exports = { mode: 'module-field' };",
    )
    .unwrap();

    let root = js_path(&app);
    let code = format!(
        r#"
        globalThis.__dirname = '{root}';
        [require('pkg').answer, require('cond').mode, require('modfield').mode].join('|');
        "#
    );
    assert_eq!(run(&code), "123|cjs|cjs");
}

#[test]
#[serial]
fn package_exports_block_unexported_subpath_with_reason_token() {
    let dir = TempDir::new().expect("temp");
    let app = dir.path().join("app");
    let pkg = app.join("node_modules/pkg");
    fs::create_dir_all(&pkg).expect("pkg");
    fs::write(
        pkg.join("package.json"),
        r#"{"name":"pkg","exports":{"./public.js":"./public.js"}}"#,
    )
    .unwrap();
    fs::write(pkg.join("public.js"), "module.exports = { ok: 1 };").unwrap();
    fs::write(pkg.join("secret.js"), "module.exports = { secret: 1 };").unwrap();
    let root = js_path(&app);
    let code = format!(
        r#"
        globalThis.__dirname = '{root}';
        try {{ require('pkg/secret.js'); 'leak'; }}
        catch (e) {{
          [e.message.includes('ERR_PACKAGE_PATH_NOT_EXPORTED'), e.code === undefined].join('|');
        }}
        "#
    );
    assert_eq!(run(&code), "true|true");
}

#[test]
#[serial]
fn package_imports_hash_and_node_modules_walk() {
    let dir = TempDir::new().expect("temp");
    let app = dir.path().join("app");
    let nested = app.join("src/deep");
    let dep = app.join("node_modules/dep");
    fs::create_dir_all(&nested).unwrap();
    fs::create_dir_all(&dep).unwrap();
    fs::write(
        app.join("package.json"),
        r##"{"name":"app","imports":{"#config":"./src/config.js"}}"##,
    )
    .unwrap();
    fs::write(
        app.join("src/config.js"),
        "module.exports = { answer: 42 };",
    )
    .unwrap();
    fs::write(dep.join("index.js"), "module.exports = { name: 'dep' };").unwrap();

    let from = js_path(&nested);
    let code = format!(
        r##"
        globalThis.__dirname = '{from}';
        [require('#config').answer, require('dep').name].join('|');
        "##
    );
    assert_eq!(run(&code), "42|dep");
}

#[test]
#[serial]
fn missing_relative_has_message_without_code() {
    let dir = TempDir::new().expect("temp");
    let root = js_path(dir.path());
    let code = format!(
        r#"
        globalThis.__dirname = '{root}';
        try {{ require('./missing-file.js'); 'ok'; }}
        catch (e) {{
          [e.name, e.code === undefined, e.message.includes("Cannot find module './missing-file.js' from")].join('|');
        }}
        "#
    );
    assert_eq!(run(&code), "Error|true|true");
}

#[test]
#[serial]
fn create_require_resolves_relative_without_resolve_helper() {
    let dir = TempDir::new().expect("temp");
    fs::write(dir.path().join("data.json"), r#"{"x":5}"#).unwrap();
    let filename = js_path(&dir.path().join("entry.js"));
    let code = format!(
        r#"
        const created = require('module').createRequire('{filename}');
        [typeof created.resolve, created('./data.json').x, created('path').join('a','b')].join('|');
        "#
    );
    assert_eq!(run(&code), "undefined|5|a/b");
}

#[test]
#[serial]
fn circular_commonjs_sees_partial_exports() {
    let dir = TempDir::new().expect("temp");
    fs::write(
        dir.path().join("a.js"),
        r#"
        exports.name = 'A';
        const b = require('./b.js');
        exports.fromB = b.name;
        "#,
    )
    .unwrap();
    fs::write(
        dir.path().join("b.js"),
        r#"
        exports.name = 'B';
        const a = require('./a.js');
        exports.fromA = a.name;
        exports.partialA = typeof a.name;
        "#,
    )
    .unwrap();
    let root = js_path(dir.path());
    let code = format!(
        r#"
        globalThis.__dirname = '{root}';
        const a = require('./a.js');
        const b = require('./b.js');
        [a.name, a.fromB, b.fromA, b.partialA].join('|');
        "#
    );
    assert_eq!(run(&code), "A|B|A|string");
}

#[test]
#[serial]
fn require_esm_namespace_and_type_module_js() {
    let dir = TempDir::new().expect("temp");
    fs::write(dir.path().join("lib.mjs"), "export const v = 3;").unwrap();
    let pkg = dir.path().join("esm-pkg");
    fs::create_dir_all(&pkg).unwrap();
    fs::write(pkg.join("package.json"), r#"{"type":"module"}"#).unwrap();
    fs::write(pkg.join("mod.js"), "export const mode = 'esm-js';").unwrap();
    let root = js_path(dir.path());
    let code = format!(
        r#"
        globalThis.__dirname = '{root}';
        const m = require('./lib.mjs');
        const j = require('./esm-pkg/mod.js');
        [m.v, Object.keys(m).join(','), j.mode].join('|');
        "#
    );
    assert_eq!(run(&code), "3|v|esm-js");
}

#[test]
fn resolver_prefers_exports_string_over_main() {
    let dir = TempDir::new().expect("temp");
    let app = dir.path().join("app");
    let pkg = app.join("node_modules/pkg");
    fs::create_dir_all(pkg.join("dist")).unwrap();
    fs::write(
        pkg.join("package.json"),
        r#"{"name":"pkg","main":"dist/main.js","exports":"./dist/exports.js"}"#,
    )
    .unwrap();
    fs::write(pkg.join("dist/main.js"), "module.exports = { answer: 1 };").unwrap();
    let exports_path = pkg.join("dist/exports.js");
    fs::write(&exports_path, "module.exports = { answer: 2026 };").unwrap();
    let resolved = resolved_file(resolve_commonjs_module("pkg", &app).unwrap());
    assert_eq!(resolved, fs::canonicalize(exports_path).unwrap());
}

#[test]
#[serial]
fn cli_run_loads_relative_commonjs_entry_graph() {
    let binary = env!("CARGO_BIN_EXE_amber");
    let dir = TempDir::new().expect("temp");
    fs::write(dir.path().join("dep.js"), "module.exports = { v: 8 };").unwrap();
    let entry = dir.path().join("entry.js");
    fs::write(
        &entry,
        "const dep = require('./dep.js');\nconsole.log(dep.v);\n",
    )
    .unwrap();
    let output = Command::new(binary)
        .arg("run")
        .arg(&entry)
        .output()
        .expect("spawn amber");
    assert!(
        output.status.success(),
        "amber run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "8");
}

#[test]
#[serial]
fn sandbox_denies_require_of_sibling_file_with_permission_typeerror() {
    let binary = env!("CARGO_BIN_EXE_amber");
    let dir = TempDir::new().expect("temp");
    fs::write(
        dir.path().join("secret.js"),
        "module.exports = { leaked: true };\n",
    )
    .unwrap();
    let entry = dir.path().join("entry.js");
    fs::write(
        &entry,
        r#"
        let name = '';
        let message = '';
        try {
          require('./secret.js');
        } catch (e) {
          name = e && e.name ? String(e.name) : '';
          message = String(e && e.message || e).toLowerCase();
        }
        console.log([name, message.includes('permission denied')].join('|'));
        "#,
    )
    .unwrap();
    let output = Command::new(binary)
        .args(["run", "--sandbox"])
        .arg(&entry)
        .output()
        .expect("spawn amber run --sandbox");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "sandbox require denial probe must run: {combined}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "TypeError|true",
        "sandbox must deny sibling require with TypeError / permission denied: {combined}"
    );
}

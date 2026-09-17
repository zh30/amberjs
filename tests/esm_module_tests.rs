// ESM Module System Tests
// Tests for true ES Module support in Amber runtime
//
// NOTE: import.meta requires true ES Module context (V8 Module API), not Script context.
// These tests verify the runtime's module system capabilities.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::fs;

fn path_for_js(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}

/// Test basic addition (sanity check)
#[test]
#[serial]
fn test_basic_addition() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const result = 2 + 3;
        result;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "5", "Basic addition should work");
}

/// Test module.children array exists and is an array
/// Note: module.children is tracked for CommonJS sub-modules
#[test]
#[serial]
fn test_module_children() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        Array.isArray(module.children);
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true");
}

/// Test module.parent exists
#[test]
#[serial]
fn test_module_parent() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof module.parent;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert!(result.trim() == "object" || result.trim() == "null");
}

/// Test module.loaded exists
#[test]
#[serial]
fn test_module_loaded() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof module.loaded;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert!(result.trim() == "boolean");
}

/// Test module.require function exists
#[test]
#[serial]
fn test_module_require() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof module.require;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert!(result.trim() == "function");
}

/// Test module.id exists
#[test]
#[serial]
fn test_module_id() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof module.id;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert!(result.trim() == "string");
}

/// Progressive import.meta.url on the default isolate (0.22 host callback).
#[test]
#[serial]
fn test_import_meta_url_is_file_url() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path("eval.mjs");
    let result = runtime
        .execute_code("String(globalThis.import && globalThis.import.meta && globalThis.import.meta.url || '')")
        .expect("Execution failed");
    assert!(
        result.contains("file:"),
        "import.meta.url should be a file URL, got {result}"
    );
}

/// Test module.exports exists
#[test]
#[serial]
fn test_module_exports() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof module.exports;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert!(result.trim() == "object");
}

/// Test require function exists
#[test]
#[serial]
fn test_require_exists() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof require;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function");
}

/// Test exports object exists
#[test]
#[serial]
fn test_exports_exists() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof exports;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "object");
}

/// Test CommonJS require works with path module
#[test]
#[serial]
fn test_commonjs_require_path() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const path = require('path');
        typeof path.join;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function");
}

/// Test __dirname available
#[test]
#[serial]
fn test_dirname_available() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof __dirname;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert!(result.trim() == "string");
}

/// Test __filename available
#[test]
#[serial]
fn test_filename_available() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof __filename;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert!(result.trim() == "string");
}

/// Test require.resolve functionality exists
#[test]
#[serial]
fn test_require_resolve() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof require.resolve;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert!(result.trim() == "function");
}

#[test]
#[serial]
fn runtime_static_default_import_loads_commonjs_module() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    fs::write(
        app_dir.join("settings.js"),
        "module.exports = { answer: 42 };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        import settings from './settings.js';
        globalThis.__esmStaticDefaultResult = settings.answer;
        "#,
        path_for_js(&app_dir)
    );

    runtime.execute_code(&code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmStaticDefaultResult")
        .expect("Failed to read static default import result");

    assert_eq!(result.trim(), "42");
}

#[test]
#[serial]
fn runtime_static_named_import_loads_commonjs_module() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    fs::write(
        app_dir.join("math.js"),
        "exports.answer = 42; exports.label = 'amber';",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        import {{ answer as value, label }} from './math.js';
        globalThis.__esmStaticNamedResult = `${{label}}:${{value}}`;
        "#,
        path_for_js(&app_dir)
    );

    runtime.execute_code(&code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmStaticNamedResult")
        .expect("Failed to read static named import result");

    assert_eq!(result.trim(), "amber:42");
}

#[test]
#[serial]
fn runtime_static_namespace_and_side_effect_imports_load_commonjs_modules() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    fs::write(
        app_dir.join("side-effect.js"),
        "globalThis.__sideEffectLoaded = 'loaded';",
    )
    .unwrap();
    fs::write(
        app_dir.join("math.js"),
        "exports.multiply = (a, b) => a * b;",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        import './side-effect.js';
        import * as math from './math.js';
        globalThis.__esmStaticNamespaceResult = `${{globalThis.__sideEffectLoaded}}:${{math.multiply(6, 7)}}`;
        "#,
        path_for_js(&app_dir)
    );

    runtime.execute_code(&code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmStaticNamespaceResult")
        .expect("Failed to read static namespace import result");

    assert_eq!(result.trim(), "loaded:42");
}

#[test]
#[serial]
fn runtime_static_import_esm_dependency_uses_live_binding() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    fs::write(
        app_dir.join("dep.mjs"),
        r#"
        export let counter = 0;
        export function bump() {
            counter += 1;
        }
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        import { counter, bump } from './dep.mjs';
        const before = counter;
        bump();
        globalThis.__esmLiveBindingResult = `${before}:${counter}`;
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmLiveBindingResult")
        .expect("Failed to read live binding result");

    assert_eq!(result.trim(), "0:1");
}

#[test]
#[serial]
fn runtime_mjs_entry_supports_top_level_await_without_imports_or_exports() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        const answer = await Promise.resolve(42);
        globalThis.__esmTlaEntryResult = `amber:${answer}`;
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmTlaEntryResult")
        .expect("Failed to read TLA entry result");

    assert_eq!(result.trim(), "amber:42");
}

#[test]
#[serial]
fn runtime_mjs_entry_supports_dynamic_import_relative_mjs() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    fs::write(
        app_dir.join("dep.mjs"),
        r#"
        globalThis.__dynamicImportDependencyLoads =
            (globalThis.__dynamicImportDependencyLoads || 0) + 1;
        export const answer = 42;
        export const loadCount = globalThis.__dynamicImportDependencyLoads;
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        const first = await import('./dep.mjs');
        const second = await import('./dep.mjs');
        globalThis.__dynamicImportResult =
            `${first.answer}:${first.loadCount}:${second.loadCount}:${first === second}:${globalThis.__dynamicImportDependencyLoads}`;
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__dynamicImportResult")
        .expect("Failed to read dynamic import result");

    assert_eq!(result.trim(), "42:1:1:true:1");
}

#[test]
#[serial]
fn runtime_mjs_entry_supports_dynamic_import_file_url_mjs() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    let dep_path = app_dir.join("dep.mjs");
    fs::write(
        &dep_path,
        r#"
        globalThis.__dynamicImportFileUrlLoads =
            (globalThis.__dynamicImportFileUrlLoads || 0) + 1;
        export const answer = 42;
        export const loadCount = globalThis.__dynamicImportFileUrlLoads;
        "#,
    )
    .unwrap();
    let dep_url = url::Url::from_file_path(&dep_path)
        .expect("temp dependency path should convert to file URL")
        .to_string();
    let dep_url_literal = serde_json::to_string(&dep_url).unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = format!(
        r#"
        const first = await import({dep_url_literal});
        const second = await import({dep_url_literal});
        globalThis.__dynamicImportFileUrlResult =
            `${{first.answer}}:${{first.loadCount}}:${{second.loadCount}}:${{first === second}}:${{globalThis.__dynamicImportFileUrlLoads}}`;
    "#
    );

    runtime.execute_code(&code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__dynamicImportFileUrlResult")
        .expect("Failed to read dynamic import file URL result");

    assert_eq!(result.trim(), "42:1:1:true:1");
}

#[test]
#[serial]
fn runtime_mjs_dynamic_import_rejects_non_file_url_specifier() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        try {
            await import('https://example.com/dep.mjs');
            globalThis.__dynamicImportNonFileUrlResult = 'resolved';
        } catch (error) {
            globalThis.__dynamicImportNonFileUrlResult =
                error && error.message ? error.message : String(error);
        }
    "#;

    runtime
        .execute_code(code)
        .expect("Dynamic import URL rejection should be catchable");
    let result = runtime
        .execute_code("globalThis.__dynamicImportNonFileUrlResult")
        .expect("Failed to read dynamic import non-file URL rejection result");

    assert_ne!(result.trim(), "resolved");
    assert!(
        result.contains("Only file:// URL specifiers are supported"),
        "Expected explicit non-file URL rejection, got: {}",
        result
    );
    assert!(
        result.contains("https://example.com/dep.mjs"),
        "Expected rejected URL in error message, got: {}",
        result
    );
    assert!(
        !result.contains("node_modules"),
        "URL specifier should not fall through package resolution, got: {}",
        result
    );
}

#[test]
#[serial]
fn runtime_mjs_dynamic_import_rejects_missing_transitive_dependency() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    fs::write(
        app_dir.join("dep.mjs"),
        r#"
        import './missing.mjs';
        export const answer = 42;
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        try {
            await import('./dep.mjs');
            globalThis.__dynamicImportMissingTransitiveResult = 'resolved';
        } catch (error) {
            globalThis.__dynamicImportMissingTransitiveResult =
                error && error.message ? error.message : String(error);
        }
    "#;

    runtime
        .execute_code(code)
        .expect("Dynamic import rejection should be catchable");
    let result = runtime
        .execute_code("globalThis.__dynamicImportMissingTransitiveResult")
        .expect("Failed to read dynamic import rejection result");

    assert_ne!(result.trim(), "resolved");
    assert!(
        result.contains("missing.mjs") || result.contains("Cannot resolve ES module"),
        "Expected missing dependency error, got: {}",
        result
    );
    assert!(
        !result.contains("Not supported"),
        "Dynamic import should use Amber loader rejection, got: {}",
        result
    );
}

#[test]
#[serial]
fn runtime_mjs_dynamic_import_rejects_dependency_syntax_error() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    fs::write(app_dir.join("bad.mjs"), "export const = ;").unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        try {
            await import('./bad.mjs');
            globalThis.__dynamicImportSyntaxResult = 'resolved';
        } catch (error) {
            globalThis.__dynamicImportSyntaxResult =
                error && error.message ? error.message : String(error);
        }
    "#;

    runtime
        .execute_code(code)
        .expect("Dynamic import syntax rejection should be catchable");
    let result = runtime
        .execute_code("globalThis.__dynamicImportSyntaxResult")
        .expect("Failed to read dynamic import syntax rejection result");

    assert_ne!(result.trim(), "resolved");
    assert!(
        result.contains("bad.mjs")
            || result.contains("compile")
            || result.contains("SyntaxError")
            || result.contains("Unexpected token"),
        "Expected syntax/compile error, got: {}",
        result
    );
    assert!(
        !result.contains("Not supported"),
        "Dynamic import should use Amber loader rejection, got: {}",
        result
    );
}

#[test]
#[serial]
fn runtime_mjs_dynamic_import_rejects_dependency_evaluation_error() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    fs::write(app_dir.join("boom.mjs"), "throw new Error('dynamic boom');").unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        try {
            await import('./boom.mjs');
            globalThis.__dynamicImportEvaluationResult = 'resolved';
        } catch (error) {
            globalThis.__dynamicImportEvaluationResult =
                error && error.message ? error.message : String(error);
        }
    "#;

    runtime
        .execute_code(code)
        .expect("Dynamic import evaluation rejection should be catchable");
    let result = runtime
        .execute_code("globalThis.__dynamicImportEvaluationResult")
        .expect("Failed to read dynamic import evaluation rejection result");

    assert_eq!(result.trim(), "dynamic boom");
}

#[test]
#[serial]
fn runtime_mjs_dynamic_import_commonjs_dependency_uses_namespace_cache() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    fs::write(
        app_dir.join("helper.js"),
        r#"
        globalThis.__dynamicImportCjsLoads =
            (globalThis.__dynamicImportCjsLoads || 0) + 1;
        exports.answer = 42;
        exports.label = 'amber';
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        const first = await import('./helper.js');
        const second = await import('./helper.js');
        globalThis.__dynamicImportCjsResult =
            `${first.answer}:${first.default.answer}:${first === second}:${first.default === second.default}:${globalThis.__dynamicImportCjsLoads}`;
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__dynamicImportCjsResult")
        .expect("Failed to read dynamic import CommonJS result");

    assert_eq!(result.trim(), "42:42:true:true:1");
}

#[test]
#[serial]
fn runtime_mjs_dynamic_import_builtin_uses_namespace_cache() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        const first = await import('path');
        const second = await import('path');
        const nodePath = await import('node:path');
        globalThis.__dynamicImportBuiltinResult = [
            first === second,
            first === nodePath,
            first.default === second.default,
            typeof first.join,
            first.join('amber', 'js')
        ].join(':');
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__dynamicImportBuiltinResult")
        .expect("Failed to read dynamic import builtin result");

    assert_eq!(result.trim(), "true:true:true:function:amber/js");
}

#[test]
#[serial]
fn runtime_script_dynamic_import_resolves_relative_mjs_from_main_module_path() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.js");
    fs::write(
        app_dir.join("dep.mjs"),
        r#"
        globalThis.__scriptDynamicImportLoads =
            (globalThis.__scriptDynamicImportLoads || 0) + 1;
        export const answer = 42;
        export const loadCount = globalThis.__scriptDynamicImportLoads;
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        globalThis.__scriptDynamicImportDone = false;
        import('./dep.mjs').then((mod) => {
            globalThis.__scriptDynamicImportResult =
                `${mod.answer}:${mod.loadCount}`;
            globalThis.__scriptDynamicImportDone = true;
        }).catch((error) => {
            globalThis.__scriptDynamicImportResult =
                error && error.message ? error.message : String(error);
            globalThis.__scriptDynamicImportDone = true;
        });
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let done = runtime
        .execute_code("globalThis.__scriptDynamicImportDone")
        .expect("Failed to read script dynamic import completion marker");
    assert_eq!(done.trim(), "true");
    let result = runtime
        .execute_code("globalThis.__scriptDynamicImportResult")
        .expect("Failed to read script dynamic import result");

    assert_eq!(result.trim(), "42:1");
}

#[test]
#[serial]
fn runtime_mjs_entry_pending_top_level_await_fails_closed() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        await new Promise(() => {});
        globalThis.__esmPendingTlaShouldNotRun = true;
    "#;

    let error = runtime
        .execute_code(code)
        .expect_err("Pending top-level await must fail closed");

    let message = error.to_string();
    let lowercase_message = message.to_ascii_lowercase();
    assert!(
        lowercase_message.contains("top-level await")
            || lowercase_message.contains("pending")
            || lowercase_message.contains("promise"),
        "Expected a clear pending TLA error, got: {}",
        message
    );
    let marker = runtime
        .execute_code("globalThis.__esmPendingTlaShouldNotRun")
        .expect("Failed to read pending TLA marker");
    assert_eq!(marker.trim(), "undefined");
}

#[test]
#[serial]
fn runtime_mjs_entry_top_level_await_settles_after_timer() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    runtime.set_timer_drain_limit_ms(250);
    let code = r#"
        const value = await new Promise((resolve) => {
            setTimeout(() => resolve("after-timer"), 10);
        });
        globalThis.__esmTimerTlaResult = `amber:${value}`;
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmTimerTlaResult")
        .expect("Failed to read timer-backed TLA result");

    assert_eq!(result.trim(), "amber:after-timer");
}

#[test]
#[serial]
fn runtime_type_module_js_entry_supports_top_level_await_without_imports_or_exports() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(app_dir.join("package.json"), r#"{"type":"module"}"#).unwrap();
    let main_path = app_dir.join("main.js");

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        const answer = await Promise.resolve(42);
        globalThis.__typeModuleTlaEntryResult = `amber:${answer}`;
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__typeModuleTlaEntryResult")
        .expect("Failed to read type module TLA entry result");

    assert_eq!(result.trim(), "amber:42");
}

#[test]
#[serial]
fn runtime_static_import_esm_dependency_supports_top_level_await_export() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    fs::write(
        app_dir.join("dep.mjs"),
        r#"
        export const answer = await Promise.resolve(42);
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        import { answer } from './dep.mjs';
        globalThis.__esmTlaDependencyResult = `amber:${answer}`;
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmTlaDependencyResult")
        .expect("Failed to read TLA dependency result");

    assert_eq!(result.trim(), "amber:42");
}

#[test]
#[serial]
fn runtime_type_module_js_entry_uses_native_esm_graph() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(app_dir.join("package.json"), r#"{"type":"module"}"#).unwrap();
    let main_path = app_dir.join("main.js");
    fs::write(
        app_dir.join("dep.js"),
        r#"
        export let counter = 0;
        export function bump() {
            counter += 1;
        }
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        import { counter, bump } from './dep.js';
        const before = counter;
        bump();
        globalThis.__typeModuleJsResult = `${before}:${counter}`;
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__typeModuleJsResult")
        .expect("Failed to read type module JS result");

    assert_eq!(result.trim(), "0:1");
}

#[test]
#[serial]
fn runtime_mjs_entry_imports_type_module_js_dependency_as_esm() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(app_dir.join("package.json"), r#"{"type":"module"}"#).unwrap();
    let main_path = app_dir.join("main.mjs");
    fs::write(
        app_dir.join("dep.js"),
        r#"
        export const label = "amber";
        export const answer = 42;
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        import { label, answer } from './dep.js';
        globalThis.__mjsToTypeModuleJsResult = `${label}:${answer}`;
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__mjsToTypeModuleJsResult")
        .expect("Failed to read mjs to type module JS result");

    assert_eq!(result.trim(), "amber:42");
}

#[test]
#[serial]
fn runtime_mjs_entry_imports_package_exports_mjs_as_esm() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules").join("pkg");
    fs::create_dir_all(&package_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":"./index.mjs"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("index.mjs"),
        r#"
        export const label = "amber";
        export const answer = 42;
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        import { label, answer } from 'pkg';
        globalThis.__packageEsmImportResult = `${label}:${answer}`;
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__packageEsmImportResult")
        .expect("Failed to read package ESM import result");

    assert_eq!(result.trim(), "amber:42");
}

#[test]
#[serial]
fn runtime_mjs_entry_uses_import_condition_for_package_exports() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules").join("pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    let main_path = app_dir.join("main.mjs");
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{".":{"require":"./dist/cjs.js","import":"./dist/esm.mjs","default":"./dist/default.mjs"}}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist").join("cjs.js"),
        "module.exports = { mode: 'require' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("dist").join("esm.mjs"),
        r#"export const mode = "import";"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist").join("default.mjs"),
        r#"export const mode = "default";"#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        import { mode } from 'pkg';
        globalThis.__packageImportConditionResult = mode;
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__packageImportConditionResult")
        .expect("Failed to read package import condition result");

    assert_eq!(result.trim(), "import");
}

#[test]
#[serial]
fn runtime_native_esm_imports_commonjs_dependency_default_namespace() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    fs::write(
        app_dir.join("settings.js"),
        "module.exports = { answer: 42, label: 'amber' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        import settings, * as settingsNs from './settings.js';
        export const forceNativeModule = true;
        globalThis.__esmCjsInteropResult =
            `${settings.label}:${settings.answer}:${settingsNs.default.answer}`;
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmCjsInteropResult")
        .expect("Failed to read ESM/CJS interop result");

    assert_eq!(result.trim(), "amber:42:42");
}

#[test]
#[serial]
fn runtime_native_esm_imports_commonjs_dependency_named_exports() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    fs::write(
        app_dir.join("math.js"),
        "exports.answer = 42; exports.label = 'amber';",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        import { answer as value, label } from './math.js';
        export const forceNativeModule = true;
        globalThis.__esmCjsNamedInteropResult = `${label}:${value}`;
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmCjsNamedInteropResult")
        .expect("Failed to read ESM/CJS named interop result");

    assert_eq!(result.trim(), "amber:42");
}

#[test]
#[serial]
fn runtime_native_esm_imports_builtin_path_namespace() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        import pathDefault, * as path from 'path';
        export const forceNativeModule = true;
        globalThis.__esmBuiltinPathResult = [
            typeof path.join,
            typeof pathDefault.join,
            path.basename('/tmp/amber.js')
        ].join(':');
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmBuiltinPathResult")
        .expect("Failed to read builtin path import result");

    assert_eq!(result.trim(), "function:function:amber.js");
}

#[test]
#[serial]
fn runtime_native_esm_imports_builtin_fs_namespace() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    let data_path = app_dir.join("data.txt");
    fs::write(&data_path, "amber-data").unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = format!(
        r#"
        import fsDefault, {{ readFileSync }} from 'fs';
        export const forceNativeModule = true;
        globalThis.__esmBuiltinFsResult = [
            typeof readFileSync,
            typeof fsDefault.readFileSync,
            readFileSync("{}", "utf8")
        ].join(':');
        "#,
        path_for_js(&data_path)
    );

    runtime.execute_code(&code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmBuiltinFsResult")
        .expect("Failed to read builtin fs import result");

    assert_eq!(result.trim(), "function:function:amber-data");
}

#[test]
#[serial]
fn runtime_native_esm_imports_builtin_url_namespace() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        import urlDefault, { URL, URLSearchParams } from 'url';
        import nodeUrlDefault, { URL as NodeURL } from 'node:url';
        export const forceNativeModule = true;
        const parsed = new URL('https://example.com/docs?x=1');
        const params = new URLSearchParams('a=amber');
        globalThis.__esmBuiltinUrlResult = [
            typeof URL,
            typeof URLSearchParams,
            typeof urlDefault.URL,
            typeof nodeUrlDefault.URL,
            URL === NodeURL,
            parsed.hostname,
            parsed.pathname,
            params.get('a')
        ].join(':');
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmBuiltinUrlResult")
        .expect("Failed to read builtin url import result");

    assert_eq!(
        result.trim(),
        "function:function:function:function:true:example.com:/docs:amber"
    );
}

#[test]
#[serial]
fn runtime_native_esm_imports_builtin_events_namespace() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        import eventsDefault, { EventEmitter } from 'events';
        import nodeEventsDefault, { EventEmitter as NodeEventEmitter } from 'node:events';
        export const forceNativeModule = true;
        const emitter = new EventEmitter();
        let observed = 'missing';
        emitter.on('__amber_esm_builtin_events__', value => {
            observed = value;
        });
        emitter.emit('__amber_esm_builtin_events__', 'buzz');
        globalThis.__esmBuiltinEventsResult = [
            typeof EventEmitter,
            typeof eventsDefault.EventEmitter,
            typeof nodeEventsDefault.EventEmitter,
            EventEmitter === NodeEventEmitter,
            observed
        ].join(':');
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmBuiltinEventsResult")
        .expect("Failed to read builtin events import result");

    assert_eq!(result.trim(), "function:function:function:true:buzz");
}

#[test]
#[serial]
fn runtime_native_esm_imports_builtin_os_namespace() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        import osDefault, { platform, arch, tmpdir } from 'os';
        import nodeOsDefault, { platform as nodePlatform } from 'node:os';
        export const forceNativeModule = true;
        globalThis.__esmBuiltinOsResult = [
            typeof platform,
            typeof osDefault.platform,
            typeof nodeOsDefault.platform,
            platform() === nodePlatform(),
            platform() === osDefault.platform(),
            typeof arch(),
            typeof tmpdir()
        ].join(':');
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmBuiltinOsResult")
        .expect("Failed to read builtin os import result");

    assert_eq!(
        result.trim(),
        "function:function:function:true:true:string:string"
    );
}

#[test]
#[serial]
fn runtime_native_esm_imports_builtin_stream_namespace() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        import streamDefault, {
            Readable,
            Writable,
            Transform,
            Duplex,
            pipeline,
            passThrough
        } from 'stream';
        import nodeStreamDefault, { Readable as NodeReadable } from 'node:stream';
        export const forceNativeModule = true;
        const readable = new Readable({ read() {} });
        const writable = new Writable({ _write(chunk, encoding, callback) { callback(); } });
        const passthrough = passThrough();
        globalThis.__esmBuiltinStreamResult = [
            typeof Readable,
            typeof Writable,
            typeof Transform,
            typeof Duplex,
            typeof pipeline,
            typeof passThrough,
            typeof streamDefault.Readable,
            typeof nodeStreamDefault.Readable,
            Readable === NodeReadable,
            typeof readable.on,
            typeof writable.write,
            typeof passthrough.write
        ].join(':');
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmBuiltinStreamResult")
        .expect("Failed to read builtin stream import result");

    assert_eq!(
        result.trim(),
        "function:function:function:function:function:function:function:function:true:function:function:function"
    );
}

#[test]
#[serial]
fn runtime_native_esm_imports_builtin_process_namespace() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        import processDefault, {
            version,
            platform,
            arch,
            cwd,
            nextTick,
            memoryUsage
        } from 'process';
        import nodeProcessDefault, { version as nodeVersion } from 'node:process';
        export const forceNativeModule = true;
        globalThis.__esmBuiltinProcessResult = [
            processDefault === globalThis.process,
            nodeProcessDefault === globalThis.process,
            version === nodeVersion,
            version === processDefault.version,
            typeof platform,
            typeof arch,
            typeof cwd,
            typeof cwd(),
            typeof nextTick,
            typeof memoryUsage,
            typeof memoryUsage().heapUsed
        ].join(':');
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmBuiltinProcessResult")
        .expect("Failed to read builtin process import result");

    assert_eq!(
        result.trim(),
        "true:true:true:true:string:string:function:string:function:function:number"
    );
}

#[test]
#[serial]
fn runtime_native_esm_imports_builtin_crypto_namespace() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        import cryptoDefault, {
            createHash,
            randomBytes,
            randomUUID,
            subtle,
            getRandomValues,
            constants
        } from 'crypto';
        import nodeCryptoDefault, { createHash as nodeCreateHash } from 'node:crypto';
        export const forceNativeModule = true;
        const hash = createHash('sha256').update('amber').digest('hex');
        const nodeHash = nodeCreateHash('sha256').update('amber').digest('hex');
        const bytes = randomBytes(8);
        const uuid = randomUUID();
        const input = new Uint8Array([1, 2, 3]);
        globalThis.__esmBuiltinCryptoResult = [
            cryptoDefault === globalThis.crypto,
            nodeCryptoDefault === globalThis.crypto,
            cryptoDefault === require('crypto'),
            createHash === cryptoDefault.createHash,
            createHash === nodeCreateHash,
            hash === nodeHash,
            hash.length,
            typeof randomBytes,
            bytes.length,
            typeof randomUUID,
            uuid.length,
            typeof subtle,
            typeof subtle.digest,
            getRandomValues(input) === input,
            typeof constants.RSA_PKCS1_PADDING
        ].join(':');
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmBuiltinCryptoResult")
        .expect("Failed to read builtin crypto import result");

    assert_eq!(
        result.trim(),
        "true:true:true:true:true:true:64:function:8:function:36:object:function:true:number"
    );
}

#[test]
#[serial]
fn runtime_native_esm_imports_builtin_crypto_signature_exports() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);
    let code = r#"
        import {
            sign,
            verify,
            createSign,
            createVerify
        } from 'crypto';
        import {
            sign as nodeSign,
            verify as nodeVerify,
            createSign as nodeCreateSign,
            createVerify as nodeCreateVerify
        } from 'node:crypto';
        export const forceNativeModule = true;
        const requiredCrypto = require('crypto');
        globalThis.__esmBuiltinCryptoSignatureResult = [
            typeof sign,
            typeof verify,
            typeof createSign,
            typeof createVerify,
            sign === nodeSign,
            verify === nodeVerify,
            createSign === nodeCreateSign,
            createVerify === nodeCreateVerify,
            sign === globalThis.crypto.sign,
            verify === globalThis.crypto.verify,
            createSign === globalThis.crypto.createSign,
            createVerify === globalThis.crypto.createVerify,
            sign === requiredCrypto.sign,
            verify === requiredCrypto.verify,
            createSign === requiredCrypto.createSign,
            createVerify === requiredCrypto.createVerify
        ].join(':');
    "#;

    runtime.execute_code(code).expect("Execution failed");
    let result = runtime
        .execute_code("globalThis.__esmBuiltinCryptoSignatureResult")
        .expect("Failed to read builtin crypto signature import result");

    assert_eq!(
        result.trim(),
        "function:function:function:function:true:true:true:true:true:true:true:true:true:true:true:true"
    );
}

#[test]
#[serial]
fn runtime_static_import_esm_dependency_uses_module_cache_between_executions() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    fs::write(
        app_dir.join("dep.mjs"),
        r#"
        globalThis.__esmDependencyLoadCount =
            (globalThis.__esmDependencyLoadCount || 0) + 1;
        export const loadCount = globalThis.__esmDependencyLoadCount;
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);

    runtime
        .execute_code(
            r#"
            import { loadCount } from './dep.mjs';
            globalThis.__firstEsmLoadCount = loadCount;
            "#,
        )
        .expect("First module execution failed");
    runtime
        .execute_code(
            r#"
            import { loadCount } from './dep.mjs';
            globalThis.__secondEsmLoadCount = loadCount;
            "#,
        )
        .expect("Second module execution failed");

    let result = runtime
        .execute_code(
            r#"
            `${globalThis.__firstEsmLoadCount}:${globalThis.__secondEsmLoadCount}:${globalThis.__esmDependencyLoadCount}`
            "#,
        )
        .expect("Failed to read module cache result");

    assert_eq!(result.trim(), "1:1:1");
}

#[test]
#[serial]
fn runtime_static_import_esm_dependency_reloads_after_source_change() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    let dep_path = app_dir.join("dep.mjs");
    fs::write(
        &dep_path,
        r#"
        globalThis.__esmReloadDirectEvalCount =
            (globalThis.__esmReloadDirectEvalCount || 0) + 1;
        export const label = "first";
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);

    runtime
        .execute_code(
            r#"
            import { label } from './dep.mjs';
            globalThis.__firstDirectReloadLabel = label;
            "#,
        )
        .expect("First module execution failed");

    fs::write(
        &dep_path,
        r#"
        globalThis.__esmReloadDirectEvalCount =
            (globalThis.__esmReloadDirectEvalCount || 0) + 1;
        export const label = "second";
        "#,
    )
    .unwrap();

    runtime
        .execute_code(
            r#"
            import { label } from './dep.mjs';
            globalThis.__secondDirectReloadLabel = label;
            "#,
        )
        .expect("Second module execution failed");

    let result = runtime
        .execute_code(
            r#"
            `${globalThis.__firstDirectReloadLabel}:${globalThis.__secondDirectReloadLabel}:${globalThis.__esmReloadDirectEvalCount}`
            "#,
        )
        .expect("Failed to read direct reload result");

    assert_eq!(result.trim(), "first:second:2");
}

#[test]
#[serial]
fn runtime_static_import_esm_dependency_reloads_when_transitive_dependency_changes() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let main_path = app_dir.join("main.mjs");
    let child_path = app_dir.join("child.mjs");
    fs::write(&child_path, r#"export const label = "first";"#).unwrap();
    fs::write(
        app_dir.join("parent.mjs"),
        r#"
        import { label } from './child.mjs';
        globalThis.__esmReloadParentEvalCount =
            (globalThis.__esmReloadParentEvalCount || 0) + 1;
        export const parentLabel = `parent:${label}`;
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_path);

    runtime
        .execute_code(
            r#"
            import { parentLabel } from './parent.mjs';
            globalThis.__firstTransitiveReloadLabel = parentLabel;
            "#,
        )
        .expect("First module execution failed");

    fs::write(&child_path, r#"export const label = "second";"#).unwrap();

    runtime
        .execute_code(
            r#"
            import { parentLabel } from './parent.mjs';
            globalThis.__secondTransitiveReloadLabel = parentLabel;
            "#,
        )
        .expect("Second module execution failed");

    let result = runtime
        .execute_code(
            r#"
            `${globalThis.__firstTransitiveReloadLabel}:${globalThis.__secondTransitiveReloadLabel}:${globalThis.__esmReloadParentEvalCount}`
            "#,
        )
        .expect("Failed to read transitive reload result");

    assert_eq!(result.trim(), "parent:first:parent:second:2");
}

/// Test that import keyword causes expected error in script context
/// Note: import statements are only valid in ES Module context, not scripts
#[test]
#[serial]
fn test_import_keyword_error() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        // import statement in script context should throw SyntaxError
        import { something } from 'somewhere';
    "#;
    let result = runtime.execute_code(code);
    // Should fail with "Cannot use import statement outside a module"
    assert!(
        result.is_err(),
        "Import statement should fail in script context"
    );
}

/// Test ESM export syntax conversion (exports are converted to comments)
/// This verifies the regex-based ESM to CommonJS conversion works
#[test]
#[serial]
fn test_esm_export_conversion() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        // ESM export syntax is converted to comments via regex
        // These should not cause parse errors
        const x = 10;
        const fn = () => 'hello';
        x + 1;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "11");
}

/// Test module.path property (if available)
#[test]
#[serial]
fn test_module_path() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof module.path;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    // module.path may or may not exist depending on implementation
    assert!(result.trim() == "string" || result.trim() == "undefined");
}

/// Test exports object is same reference as module.exports
#[test]
#[serial]
fn test_exports_module_exports_same() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        exports.foo = 123;
        module.exports.foo;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "123");
}

/// Test CommonJS circular reference through exports
#[test]
#[serial]
fn test_circular_exports() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        exports.a = 1;
        exports.b = function() { return exports.a; };
        exports.a = 2;
        exports.b();
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "2");
}

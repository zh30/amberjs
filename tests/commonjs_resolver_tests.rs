use amberjs::nodejs_core::commonjs_resolver::{resolve_commonjs_module, ResolvedModule};
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::fs;

fn path_for_js(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}

fn resolved_file_path(module: ResolvedModule) -> std::path::PathBuf {
    match module {
        ResolvedModule::File(path) => path,
        ResolvedModule::Builtin(name) => panic!("expected file module, got builtin {name}"),
    }
}

fn resolved_builtin_name(module: ResolvedModule) -> String {
    match module {
        ResolvedModule::Builtin(name) => name,
        ResolvedModule::File(path) => {
            panic!("expected builtin module, got file {}", path.display())
        }
    }
}

#[test]
fn resolves_node_prefix_builtin_module() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();

    let resolved = resolved_builtin_name(resolve_commonjs_module("node:path", &app_dir).unwrap());

    assert_eq!(resolved, "path");
}

#[test]
fn resolves_relative_module_with_js_extension() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let util_path = app_dir.join("util.js");
    fs::write(&util_path, "exports.value = 42;").unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("./util", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(util_path).unwrap());
}

#[test]
fn resolves_relative_typescript_module_extension() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let typed_path = app_dir.join("typed.ts");
    fs::write(
        &typed_path,
        "const answer: number = 42; module.exports = { answer };",
    )
    .unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("./typed", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(typed_path).unwrap());
}

#[test]
fn resolves_directory_index_module() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("lib");
    fs::create_dir_all(&package_dir).unwrap();
    let index_path = package_dir.join("index.js");
    fs::write(&index_path, "exports.value = 7;").unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("./lib", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(index_path).unwrap());
}

#[test]
fn resolves_dot_as_current_directory_package() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(app_dir.join("src")).unwrap();
    fs::write(
        app_dir.join("package.json"),
        r#"{"name":"app","main":"src/main.js"}"#,
    )
    .unwrap();
    let main_path = app_dir.join("src/main.js");
    fs::write(&main_path, "module.exports = { value: 1 };").unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module(".", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(main_path).unwrap());
}

#[test]
fn resolves_dot_dot_as_parent_directory_package() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let nested_dir = app_dir.join("src");
    fs::create_dir_all(&nested_dir).unwrap();
    let index_path = app_dir.join("index.js");
    fs::write(&index_path, "exports.value = 2;").unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("..", &nested_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(index_path).unwrap());
}

#[test]
fn resolves_package_json_main_from_node_modules() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","main":"dist/main.js"}"#,
    )
    .unwrap();
    let main_path = package_dir.join("dist/main.js");
    fs::write(&main_path, "module.exports = { answer: 42 };").unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("pkg", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(main_path).unwrap());
}

#[test]
fn ignores_package_json_module_field_for_commonjs_require() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","module":"dist/esm.js","main":"dist/cjs.js"}"#,
    )
    .unwrap();
    let cjs_path = package_dir.join("dist/cjs.js");
    fs::write(&cjs_path, "module.exports = { mode: 'cjs' };").unwrap();
    fs::write(
        package_dir.join("dist/esm.js"),
        "module.exports = { mode: 'module-field' };",
    )
    .unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("pkg", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(cjs_path).unwrap());
}

#[test]
fn resolves_package_json_exports_string_before_main() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":"./dist/cjs.js","main":"legacy.js"}"#,
    )
    .unwrap();
    let exports_path = package_dir.join("dist/cjs.js");
    fs::write(&exports_path, "module.exports = { answer: 2026 };").unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { answer: 1 };",
    )
    .unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("pkg", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(exports_path).unwrap());
}

#[test]
fn package_json_exports_target_without_dot_slash_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":"dist/cjs.js","main":"legacy.js"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/cjs.js"),
        "module.exports = { mode: 'invalid-export-target' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let error = resolve_commonjs_module("pkg", &app_dir)
        .expect_err("CommonJS exports targets must start with ./");
    let message = error.to_string();

    assert!(
        message.contains("ERR_INVALID_PACKAGE_TARGET"),
        "expected invalid exports target to block package root, got: {message}"
    );
}

#[test]
fn package_json_exports_target_with_node_modules_segment_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    let private_dir = package_dir.join("node_modules/internal");
    fs::create_dir_all(&private_dir).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":"./node_modules/internal/index.js","main":"legacy.js"}"#,
    )
    .unwrap();
    fs::write(
        private_dir.join("index.js"),
        "module.exports = { mode: 'internal-dependency' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let error = resolve_commonjs_module("pkg", &app_dir)
        .expect_err("exports targets must not traverse into node_modules segments");
    let message = error.to_string();

    assert!(
        message.contains("ERR_INVALID_PACKAGE_TARGET"),
        "expected node_modules export target to block package root, got: {message}"
    );
}

#[test]
fn package_json_exports_target_with_dot_segment_after_prefix_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":"././dist/cjs.js","main":"legacy.js"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/cjs.js"),
        "module.exports = { mode: 'dot-segment' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let error = resolve_commonjs_module("pkg", &app_dir)
        .expect_err("exports targets must not contain '.' segments after the './' prefix");
    let message = error.to_string();

    assert!(
        message.contains("ERR_INVALID_PACKAGE_TARGET"),
        "expected dot-segment export target to block package root, got: {message}"
    );
}

#[test]
fn package_json_exports_invalid_primitive_rejects_main_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":true,"main":"legacy.js"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let error = resolve_commonjs_module("pkg", &app_dir)
        .expect_err("invalid primitive exports value must block main fallback");
    let message = error.to_string();

    assert!(
        message.contains("ERR_INVALID_PACKAGE_TARGET"),
        "expected primitive exports target to report invalid package target, got: {message}"
    );
}

#[test]
fn malformed_package_json_blocks_index_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(package_dir.join("package.json"), r#"{"name":"pkg","#).unwrap();
    fs::write(
        package_dir.join("index.js"),
        "module.exports = { mode: 'fallback' };",
    )
    .unwrap();

    let error = resolve_commonjs_module("pkg", &app_dir)
        .expect_err("malformed package.json must block index fallback");
    let message = error.to_string();

    assert!(
        message.contains("ERR_INVALID_PACKAGE_CONFIG"),
        "expected malformed package.json to report invalid package config, got: {message}"
    );
}

#[test]
fn resolves_package_json_conditional_exports_for_require() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{".":{"require":"./dist/cjs.js","node":"./dist/node.js","default":"./dist/esm.js"}}}"#,
    )
    .unwrap();
    let cjs_path = package_dir.join("dist/cjs.js");
    fs::write(&cjs_path, "module.exports = { mode: 'cjs' };").unwrap();
    fs::write(
        package_dir.join("dist/node.js"),
        "module.exports = { mode: 'node' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/esm.js"),
        "module.exports = { mode: 'esm' };",
    )
    .unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("pkg", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(cjs_path).unwrap());
}

#[test]
fn resolves_package_json_conditional_exports_in_package_order() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{".":{"default":"./dist/esm.js","require":"./dist/cjs.js"}}}"#,
    )
    .unwrap();
    let esm_path = package_dir.join("dist/esm.js");
    fs::write(&esm_path, "module.exports = { mode: 'default-first' };").unwrap();
    fs::write(
        package_dir.join("dist/cjs.js"),
        "module.exports = { mode: 'require' };",
    )
    .unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("pkg", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(esm_path).unwrap());
}

#[test]
fn package_json_conditional_exports_empty_array_blocks_later_conditions() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{".":{"require":[],"default":"./dist/default.js"}}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/default.js"),
        "module.exports = { mode: 'default' };",
    )
    .unwrap();

    let error = resolve_commonjs_module("pkg", &app_dir)
        .expect_err("empty array fallback should block later conditional exports");
    let message = error.to_string();

    assert!(
        message.contains("ERR_PACKAGE_PATH_NOT_EXPORTED"),
        "expected empty array condition to block package root, got: {message}"
    );
}

#[test]
fn package_json_exports_mixed_dot_and_condition_keys_is_invalid_config() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{".":"./dist/index.js","default":"./dist/default.js"}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/index.js"),
        "module.exports = { mode: 'root' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/default.js"),
        "module.exports = { mode: 'default' };",
    )
    .unwrap();

    let error = resolve_commonjs_module("pkg", &app_dir)
        .expect_err("mixed exports keys must be an invalid package configuration");
    let message = error.to_string();

    assert!(
        message.contains("ERR_INVALID_PACKAGE_CONFIG"),
        "expected invalid package config for mixed exports keys, got: {message}"
    );
}

#[test]
fn package_json_subpath_exports_mixed_dot_and_condition_keys_is_invalid_config() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{"./feature":"./feature.js","default":"./default.js"}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("feature.js"),
        "module.exports = { mode: 'feature' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("default.js"),
        "module.exports = { mode: 'default' };",
    )
    .unwrap();

    let error = resolve_commonjs_module("pkg/feature", &app_dir)
        .expect_err("mixed exports keys must invalidate subpath exports too");
    let message = error.to_string();

    assert!(
        message.contains("ERR_INVALID_PACKAGE_CONFIG"),
        "expected invalid package config for mixed subpath exports keys, got: {message}"
    );
}

#[test]
fn resolves_package_json_conditional_exports_preserves_require_before_node_order() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{".":{"require":"./dist/cjs.js","node":"./dist/node.js"}}}"#,
    )
    .unwrap();
    let cjs_path = package_dir.join("dist/cjs.js");
    fs::write(&cjs_path, "module.exports = { mode: 'require-first' };").unwrap();
    fs::write(
        package_dir.join("dist/node.js"),
        "module.exports = { mode: 'node' };",
    )
    .unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("pkg", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(cjs_path).unwrap());
}

#[test]
fn resolves_package_json_exports_array_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{".":["./missing.js","./dist/cjs.js"]},"main":"legacy.js"}"#,
    )
    .unwrap();
    let cjs_path = package_dir.join("dist/cjs.js");
    fs::write(&cjs_path, "module.exports = { mode: 'array-fallback' };").unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("pkg", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(cjs_path).unwrap());
}

#[test]
fn package_json_exports_array_null_entry_blocks_later_targets() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{".":[null,"./dist/cjs.js"]},"main":"legacy.js"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/cjs.js"),
        "module.exports = { mode: 'should-not-load' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let error = resolve_commonjs_module("pkg", &app_dir)
        .expect_err("null array entry must block later fallback targets");
    let message = error.to_string();

    assert!(
        message.contains("ERR_PACKAGE_PATH_NOT_EXPORTED"),
        "expected null array entry to block package root, got: {message}"
    );
}

#[test]
fn package_json_root_null_export_blocks_main_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{".":null},"main":"legacy.js"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let error = resolve_commonjs_module("pkg", &app_dir)
        .expect_err("null root export must block main fallback");
    let message = error.to_string();

    assert!(
        message.contains("ERR_PACKAGE_PATH_NOT_EXPORTED"),
        "expected package exports error, got: {message}"
    );
    assert!(
        message.contains("."),
        "error should identify package root export, got: {message}"
    );
}

#[test]
fn package_json_subpath_exports_without_root_blocks_main_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{"./feature":"./feature.js"},"main":"legacy.js"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("feature.js"),
        "module.exports = { feature: true };",
    )
    .unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let error = resolve_commonjs_module("pkg", &app_dir)
        .expect_err("subpath-only exports must block package root main fallback");
    let message = error.to_string();

    assert!(
        message.contains("ERR_PACKAGE_PATH_NOT_EXPORTED"),
        "expected package exports error, got: {message}"
    );
}

#[test]
fn resolves_package_json_subpath_exports_string() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{"./feature":"./dist/feature.js"}}"#,
    )
    .unwrap();
    let feature_path = package_dir.join("dist/feature.js");
    fs::write(&feature_path, "module.exports = { feature: true };").unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("pkg/feature", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(feature_path).unwrap());
}

#[test]
fn resolves_package_json_subpath_exports_array_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{"./feature":["./missing.js","./dist/feature.js"]}}"#,
    )
    .unwrap();
    let feature_path = package_dir.join("dist/feature.js");
    fs::write(&feature_path, "module.exports = { feature: true };").unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("pkg/feature", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(feature_path).unwrap());
}

#[test]
fn package_json_null_subpath_export_blocks_private_file() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{"./private":null}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("private.js"),
        "module.exports = { private: true };",
    )
    .unwrap();

    let error = resolve_commonjs_module("pkg/private", &app_dir)
        .expect_err("null subpath export must block private file fallback");
    let message = error.to_string();

    assert!(
        message.contains("ERR_PACKAGE_PATH_NOT_EXPORTED"),
        "expected package exports error, got: {message}"
    );
    assert!(
        message.contains("./private"),
        "error should identify blocked subpath, got: {message}"
    );
}

#[test]
fn resolves_package_json_subpath_exports_pattern() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist/features")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{"./features/*":"./dist/features/*.js"}}"#,
    )
    .unwrap();
    let feature_path = package_dir.join("dist/features/button.js");
    fs::write(&feature_path, "module.exports = { feature: 'button' };").unwrap();

    let resolved =
        resolved_file_path(resolve_commonjs_module("pkg/features/button", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(feature_path).unwrap());
}

#[test]
fn package_json_subpath_pattern_trailer_requires_non_empty_capture() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist/features")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{"./features/*.js":"./dist/features/*.js"}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/features/.js"),
        "module.exports = { leaked: true };",
    )
    .unwrap();

    let error = resolve_commonjs_module("pkg/features/.js", &app_dir)
        .expect_err("pattern trailer must not match an empty star capture");
    let message = error.to_string();

    assert!(
        message.contains("ERR_PACKAGE_PATH_NOT_EXPORTED"),
        "empty pattern capture should remain unexported, got: {message}"
    );
}

#[test]
fn package_json_subpath_pattern_prefers_longer_base_over_longer_trailer() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist/suffix/special")).unwrap();
    fs::create_dir_all(package_dir.join("dist/base")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{"./x/*.very-long-suffix":"./dist/suffix/*.js","./x/special/*":"./dist/base/*.js"}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/suffix/special/name.js"),
        "module.exports = { picked: 'suffix' };",
    )
    .unwrap();
    let base_path = package_dir.join("dist/base/name.very-long-suffix.js");
    fs::write(&base_path, "module.exports = { picked: 'base' };").unwrap();

    let resolved = resolved_file_path(
        resolve_commonjs_module("pkg/x/special/name.very-long-suffix", &app_dir).unwrap(),
    );

    assert_eq!(resolved, fs::canonicalize(base_path).unwrap());
}

#[test]
fn package_json_subpath_pattern_capture_parent_segment_is_invalid_module_specifier() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{"./features/*":"./dist/*.js"}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("secret.js"),
        "module.exports = { leaked: true };",
    )
    .unwrap();

    let error = resolve_commonjs_module("pkg/features/../secret", &app_dir)
        .expect_err("pattern capture with parent segment must be an invalid module specifier");
    let message = error.to_string();

    assert!(
        message.contains("ERR_INVALID_MODULE_SPECIFIER"),
        "expected invalid module specifier for pattern capture, got: {message}"
    );
}

#[test]
fn package_json_subpath_pattern_capture_node_modules_segment_is_invalid_module_specifier() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist/node_modules/private")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{"./features/*":"./dist/*.js"}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/node_modules/private/index.js"),
        "module.exports = { leaked: true };",
    )
    .unwrap();

    let error = resolve_commonjs_module("pkg/features/node_modules/private/index", &app_dir)
        .expect_err("pattern capture with node_modules segment must be invalid");
    let message = error.to_string();

    assert!(
        message.contains("ERR_INVALID_MODULE_SPECIFIER"),
        "expected invalid module specifier for node_modules capture, got: {message}"
    );
}

#[test]
fn package_json_exports_blocks_unexported_subpath_with_specific_error() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{"./public":"./public.js"}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("public.js"),
        "module.exports = { public: true };",
    )
    .unwrap();
    fs::write(
        package_dir.join("private.js"),
        "module.exports = { private: true };",
    )
    .unwrap();

    let error = resolve_commonjs_module("pkg/private", &app_dir)
        .expect_err("unexported package subpath must be rejected");
    let message = error.to_string();

    assert!(
        message.contains("ERR_PACKAGE_PATH_NOT_EXPORTED"),
        "expected package exports error, got: {message}"
    );
    assert!(
        message.contains("./private"),
        "error should identify the blocked subpath, got: {message}"
    );
}

#[test]
fn resolves_scoped_package_json_subpath_exports_string() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/@scope/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"@scope/pkg","exports":{"./feature":"./dist/feature.js"}}"#,
    )
    .unwrap();
    let feature_path = package_dir.join("dist/feature.js");
    fs::write(&feature_path, "module.exports = { feature: true };").unwrap();

    let resolved =
        resolved_file_path(resolve_commonjs_module("@scope/pkg/feature", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(feature_path).unwrap());
}

#[test]
fn resolves_package_self_reference_root_exports() {
    let temp = tempfile::tempdir().unwrap();
    let package_dir = temp.path().join("workspace/packages/pkg");
    let src_dir = package_dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{".":"./dist/index.js"},"main":"legacy.js"}"#,
    )
    .unwrap();
    let exports_path = package_dir.join("dist/index.js");
    fs::write(&exports_path, "module.exports = { mode: 'self-root' };").unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("pkg", &src_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(exports_path).unwrap());
}

#[test]
fn resolves_scoped_package_self_reference_subpath_exports() {
    let temp = tempfile::tempdir().unwrap();
    let package_dir = temp.path().join("workspace/packages/scoped-pkg");
    let src_dir = package_dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"@scope/pkg","exports":{"./feature":"./dist/feature.js"}}"#,
    )
    .unwrap();
    let feature_path = package_dir.join("dist/feature.js");
    fs::write(&feature_path, "module.exports = { feature: 'self' };").unwrap();

    let resolved =
        resolved_file_path(resolve_commonjs_module("@scope/pkg/feature", &src_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(feature_path).unwrap());
}

#[test]
fn resolves_package_json_imports_exact_for_commonjs_require() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(app_dir.join("src")).unwrap();
    fs::write(
        app_dir.join("package.json"),
        r##"{"name":"app","imports":{"#config":"./src/config.js"}}"##,
    )
    .unwrap();
    let config_path = app_dir.join("src/config.js");
    fs::write(&config_path, "module.exports = { answer: 42 };").unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("#config", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(config_path).unwrap());
}

#[test]
fn resolves_package_json_imports_exact_external_package_target() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let dep_dir = app_dir.join("node_modules/dep");
    fs::create_dir_all(&dep_dir).unwrap();
    fs::write(
        app_dir.join("package.json"),
        r##"{"name":"app","imports":{"#dep":"dep"}}"##,
    )
    .unwrap();
    let dep_path = dep_dir.join("index.js");
    fs::write(&dep_path, "module.exports = { name: 'dep' };").unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("#dep", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(dep_path).unwrap());
}

#[test]
fn resolves_package_json_imports_pattern_for_commonjs_require() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(app_dir.join("src/features")).unwrap();
    fs::write(
        app_dir.join("package.json"),
        r##"{"name":"app","imports":{"#features/*":"./src/features/*.js"}}"##,
    )
    .unwrap();
    let feature_path = app_dir.join("src/features/button.js");
    fs::write(&feature_path, "module.exports = { feature: 'button' };").unwrap();

    let resolved =
        resolved_file_path(resolve_commonjs_module("#features/button", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(feature_path).unwrap());
}

#[test]
fn resolves_package_json_imports_pattern_external_package_target() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let dep_dir = app_dir.join("node_modules/dep");
    fs::create_dir_all(dep_dir.join("dist")).unwrap();
    fs::write(
        app_dir.join("package.json"),
        r##"{"name":"app","imports":{"#dep/*":"dep/*"}}"##,
    )
    .unwrap();
    fs::write(
        dep_dir.join("package.json"),
        r#"{"name":"dep","exports":{"./feature":"./dist/feature.js"}}"#,
    )
    .unwrap();
    let feature_path = dep_dir.join("dist/feature.js");
    fs::write(&feature_path, "module.exports = { feature: 'dep' };").unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("#dep/feature", &app_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(feature_path).unwrap());
}

#[test]
fn package_json_imports_pattern_prefers_longer_base_over_longer_trailer() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(app_dir.join("src/suffix/special")).unwrap();
    fs::create_dir_all(app_dir.join("src/base")).unwrap();
    fs::write(
        app_dir.join("package.json"),
        r##"{"name":"app","imports":{"#x/*.very-long-suffix":"./src/suffix/*.js","#x/special/*":"./src/base/*.js"}}"##,
    )
    .unwrap();
    fs::write(
        app_dir.join("src/suffix/special/name.js"),
        "module.exports = { picked: 'suffix' };",
    )
    .unwrap();
    let base_path = app_dir.join("src/base/name.very-long-suffix.js");
    fs::write(&base_path, "module.exports = { picked: 'base' };").unwrap();

    let resolved = resolved_file_path(
        resolve_commonjs_module("#x/special/name.very-long-suffix", &app_dir).unwrap(),
    );

    assert_eq!(resolved, fs::canonicalize(base_path).unwrap());
}

#[test]
fn package_json_imports_pattern_capture_parent_segment_is_invalid_module_specifier() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(app_dir.join("src")).unwrap();
    fs::write(
        app_dir.join("package.json"),
        r##"{"name":"app","imports":{"#features/*":"./src/*.js"}}"##,
    )
    .unwrap();
    fs::write(
        app_dir.join("secret.js"),
        "module.exports = { leaked: true };",
    )
    .unwrap();

    let error = resolve_commonjs_module("#features/../secret", &app_dir)
        .expect_err("imports pattern capture with parent segment must be invalid");
    let message = error.to_string();

    assert!(
        message.contains("ERR_INVALID_MODULE_SPECIFIER"),
        "expected invalid module specifier for imports pattern capture, got: {message}"
    );
}

#[test]
fn package_json_imports_undefined_reports_specific_error() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(app_dir.join("src")).unwrap();
    fs::write(
        app_dir.join("package.json"),
        r##"{"name":"app","imports":{"#config":"./src/config.js"}}"##,
    )
    .unwrap();
    fs::write(
        app_dir.join("src/config.js"),
        "module.exports = { answer: 42 };",
    )
    .unwrap();

    let error = resolve_commonjs_module("#missing", &app_dir)
        .expect_err("undefined package imports aliases must be rejected specifically");
    let message = error.to_string();

    assert!(
        message.contains("ERR_PACKAGE_IMPORT_NOT_DEFINED"),
        "expected package import error, got: {message}"
    );
    assert!(
        message.contains("#missing"),
        "error should identify the missing import, got: {message}"
    );
}

#[test]
fn package_json_imports_external_parent_target_is_invalid_package_target() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(
        app_dir.join("package.json"),
        r##"{"name":"app","imports":{"#dep":"../dep"}}"##,
    )
    .unwrap();

    let error = resolve_commonjs_module("#dep", &app_dir)
        .expect_err("external package imports targets must not start with ../");
    let message = error.to_string();

    assert!(
        message.contains("ERR_INVALID_PACKAGE_TARGET"),
        "expected invalid package target, got: {message}"
    );
}

#[test]
fn package_json_imports_trailing_slash_is_invalid_module_specifier() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(
        app_dir.join("package.json"),
        r##"{"name":"app","imports":{"#features/*":"./src/features/*.js"}}"##,
    )
    .unwrap();

    let error = resolve_commonjs_module("#features/", &app_dir)
        .expect_err("package imports specifiers ending in slash are invalid");
    let message = error.to_string();

    assert!(
        message.contains("ERR_INVALID_MODULE_SPECIFIER"),
        "expected invalid module specifier, got: {message}"
    );
}

#[test]
fn package_json_imports_hash_only_is_invalid_module_specifier() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(app_dir.join("package.json"), r#"{"name":"app"}"#).unwrap();

    let error = resolve_commonjs_module("#", &app_dir)
        .expect_err("bare # is not a valid package imports specifier");
    let message = error.to_string();

    assert!(
        message.contains("ERR_INVALID_MODULE_SPECIFIER"),
        "expected invalid module specifier, got: {message}"
    );
}

#[test]
fn resolves_node_modules_by_walking_parent_directories() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let nested_dir = app_dir.join("src/features");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(&nested_dir).unwrap();
    fs::create_dir_all(&package_dir).unwrap();
    let index_path = package_dir.join("index.js");
    fs::write(&index_path, "exports.answer = 42;").unwrap();

    let resolved = resolved_file_path(resolve_commonjs_module("pkg", &nested_dir).unwrap());

    assert_eq!(resolved, fs::canonicalize(index_path).unwrap());
}

#[test]
#[serial]
fn runtime_require_resolves_directory_index() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let lib_dir = app_dir.join("lib");
    fs::create_dir_all(&lib_dir).unwrap();
    fs::write(lib_dir.join("index.js"), "exports.value = 99;").unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        require("./lib").value;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "99");
}

#[test]
#[serial]
fn runtime_require_resolves_package_main() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","main":"dist/main.js"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/main.js"),
        "module.exports = { answer: 123 };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        require("pkg").answer;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "123");
}

#[test]
#[serial]
fn runtime_require_resolves_package_json_imports_exact() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(app_dir.join("src")).unwrap();
    fs::write(
        app_dir.join("package.json"),
        r##"{"name":"app","imports":{"#config":"./src/config.js"}}"##,
    )
    .unwrap();
    fs::write(
        app_dir.join("src/config.js"),
        "module.exports = { answer: 42 };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r##"
        globalThis.__dirname = "{}";
        require("#config").answer;
        "##,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "42");
}

#[test]
#[serial]
fn runtime_require_resolves_package_json_imports_external_package_target() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let dep_dir = app_dir.join("node_modules/dep");
    fs::create_dir_all(&dep_dir).unwrap();
    fs::write(
        app_dir.join("package.json"),
        r##"{"name":"app","imports":{"#dep":"dep"}}"##,
    )
    .unwrap();
    fs::write(
        dep_dir.join("index.js"),
        "module.exports = { name: 'dep' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r##"
        globalThis.__dirname = "{}";
        require("#dep").name;
        "##,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "dep");
}

#[test]
#[serial]
fn runtime_require_resolves_package_json_imports_pattern() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(app_dir.join("src/features")).unwrap();
    fs::write(
        app_dir.join("package.json"),
        r##"{"name":"app","imports":{"#features/*":"./src/features/*.js"}}"##,
    )
    .unwrap();
    fs::write(
        app_dir.join("src/features/button.js"),
        "module.exports = { feature: 'button' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r##"
        globalThis.__dirname = "{}";
        require("#features/button").feature;
        "##,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "button");
}

#[test]
#[serial]
fn runtime_require_ignores_package_json_module_field() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","module":"dist/esm.js","main":"dist/cjs.js"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/cjs.js"),
        "module.exports = { mode: 'cjs' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/esm.js"),
        "module.exports = { mode: 'module-field' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        require("pkg").mode;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "cjs");
}

#[test]
#[serial]
fn runtime_require_resolves_package_exports_string_before_main() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":"./dist/cjs.js","main":"legacy.js"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/cjs.js"),
        "module.exports = { answer: 2026 };",
    )
    .unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { answer: 1 };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        require("pkg").answer;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "2026");
}

#[test]
#[serial]
fn runtime_require_rejects_package_exports_target_without_dot_slash() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":"dist/cjs.js","main":"legacy.js"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/cjs.js"),
        "module.exports = { mode: 'invalid-export-target' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        try {{
            require("pkg");
            "loaded";
        }} catch (error) {{
            String(error && error.message || error);
        }}
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert!(
        result.contains("ERR_INVALID_PACKAGE_TARGET"),
        "runtime require should reject invalid exports target, got: {result}"
    );
}

#[test]
#[serial]
fn runtime_require_rejects_package_exports_target_with_node_modules_segment() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    let private_dir = package_dir.join("node_modules/internal");
    fs::create_dir_all(&private_dir).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":"./node_modules/internal/index.js","main":"legacy.js"}"#,
    )
    .unwrap();
    fs::write(
        private_dir.join("index.js"),
        "module.exports = { mode: 'internal-dependency' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        try {{
            require("pkg");
            "loaded";
        }} catch (error) {{
            String(error && error.message || error);
        }}
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert!(
        result.contains("ERR_INVALID_PACKAGE_TARGET"),
        "runtime require should reject node_modules export target, got: {result}"
    );
}

#[test]
#[serial]
fn runtime_require_rejects_package_exports_target_with_dot_segment_after_prefix() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":"././dist/cjs.js","main":"legacy.js"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/cjs.js"),
        "module.exports = { mode: 'dot-segment' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        try {{
            require("pkg");
            "loaded";
        }} catch (error) {{
            String(error && error.message || error);
        }}
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert!(
        result.contains("ERR_INVALID_PACKAGE_TARGET"),
        "runtime require should reject dot-segment export target, got: {result}"
    );
}

#[test]
#[serial]
fn runtime_require_rejects_package_exports_invalid_primitive() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":true,"main":"legacy.js"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        try {{
            require("pkg");
            "loaded";
        }} catch (error) {{
            String(error && error.message || error);
        }}
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert!(
        result.contains("ERR_INVALID_PACKAGE_TARGET"),
        "runtime require should reject primitive exports target, got: {result}"
    );
}

#[test]
#[serial]
fn runtime_require_reports_invalid_package_config_for_malformed_package_json() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(package_dir.join("package.json"), r#"{"name":"pkg","#).unwrap();
    fs::write(
        package_dir.join("index.js"),
        "module.exports = { mode: 'fallback' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        try {{
            require("pkg");
            "loaded";
        }} catch (error) {{
            String(error && error.message || error);
        }}
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert!(
        result.contains("ERR_INVALID_PACKAGE_CONFIG"),
        "runtime require should reject malformed package.json, got: {result}"
    );
    assert!(
        !result.contains("loaded"),
        "runtime require must not fall back to index.js for malformed package.json, got: {result}"
    );
}

#[test]
#[serial]
fn runtime_require_rejects_package_exports_mixed_dot_and_condition_keys() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{".":"./dist/index.js","default":"./dist/default.js"}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/index.js"),
        "module.exports = { mode: 'root' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/default.js"),
        "module.exports = { mode: 'default' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        try {{
            require("pkg");
            "loaded";
        }} catch (error) {{
            String(error && error.message || error);
        }}
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert!(
        result.contains("ERR_INVALID_PACKAGE_CONFIG"),
        "runtime require should reject mixed exports keys, got: {result}"
    );
}

#[test]
#[serial]
fn runtime_require_resolves_package_conditional_exports_for_require() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{".":{"require":"./dist/cjs.js","node":"./dist/node.js","default":"./dist/esm.js"}}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/cjs.js"),
        "module.exports = { mode: 'cjs' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/node.js"),
        "module.exports = { mode: 'node' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/esm.js"),
        "module.exports = { mode: 'esm' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        require("pkg").mode;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "cjs");
}

#[test]
#[serial]
fn runtime_require_respects_package_conditional_export_order() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{".":{"default":"./dist/esm.js","require":"./dist/cjs.js"}}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/esm.js"),
        "module.exports = { mode: 'default-first' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/cjs.js"),
        "module.exports = { mode: 'require' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        require("pkg").mode;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "default-first");
}

#[test]
#[serial]
fn runtime_require_resolves_package_exports_array_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{".":["./missing.js","./dist/cjs.js"]},"main":"legacy.js"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/cjs.js"),
        "module.exports = { mode: 'array-fallback' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        require("pkg").mode;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "array-fallback");
}

#[test]
#[serial]
fn runtime_require_reports_package_exports_array_null_entry() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{".":[null,"./dist/cjs.js"]},"main":"legacy.js"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/cjs.js"),
        "module.exports = { mode: 'should-not-load' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        try {{
            require("pkg");
            "loaded";
        }} catch (error) {{
            String(error && error.message || error);
        }}
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert!(
        result.contains("ERR_PACKAGE_PATH_NOT_EXPORTED"),
        "runtime require should report null array entry, got: {result}"
    );
}

#[test]
#[serial]
fn runtime_require_reports_null_package_root_export() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{".":null},"main":"legacy.js"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        try {{
            require("pkg");
            "loaded";
        }} catch (error) {{
            String(error && error.message || error);
        }}
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert!(
        result.contains("ERR_PACKAGE_PATH_NOT_EXPORTED"),
        "runtime require should report null root export, got: {result}"
    );
}

#[test]
#[serial]
fn runtime_require_reports_subpath_only_exports_package_root() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{"./feature":"./feature.js"},"main":"legacy.js"}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("feature.js"),
        "module.exports = { feature: true };",
    )
    .unwrap();
    fs::write(
        package_dir.join("legacy.js"),
        "module.exports = { mode: 'legacy' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        try {{
            require("pkg");
            "loaded";
        }} catch (error) {{
            String(error && error.message || error);
        }}
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert!(
        result.contains("ERR_PACKAGE_PATH_NOT_EXPORTED"),
        "runtime require should report subpath-only exports root error, got: {result}"
    );
}

#[test]
#[serial]
fn runtime_require_resolves_package_subpath_exports_string() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{"./feature":"./dist/feature.js"}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/feature.js"),
        "module.exports = { answer: 808 };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        require("pkg/feature").answer;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "808");
}

#[test]
#[serial]
fn runtime_require_resolves_package_subpath_exports_pattern() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(package_dir.join("dist/features")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{"./features/*":"./dist/features/*.js"}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("dist/features/button.js"),
        "module.exports = { feature: 'button' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        require("pkg/features/button").feature;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "button");
}

#[test]
#[serial]
fn runtime_require_reports_unexported_package_subpath() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("node_modules/pkg");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{"./public":"./public.js"}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("public.js"),
        "module.exports = { public: true };",
    )
    .unwrap();
    fs::write(
        package_dir.join("private.js"),
        "module.exports = { private: true };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        try {{
            require("pkg/private");
            "loaded";
        }} catch (error) {{
            String(error && error.message || error);
        }}
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert!(
        result.contains("ERR_PACKAGE_PATH_NOT_EXPORTED"),
        "runtime require should report exports error, got: {result}"
    );
}

#[test]
#[serial]
fn runtime_require_resolves_package_self_reference_subpath_exports() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("packages/pkg");
    fs::create_dir_all(package_dir.join("src")).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"pkg","exports":{"./feature":"./src/feature.js"}}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("src/feature.js"),
        "module.exports = { value: 'self-reference' };",
    )
    .unwrap();
    fs::write(
        package_dir.join("src/index.js"),
        r#"module.exports = require("pkg/feature");"#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        require("./packages/pkg/src/index").value;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "self-reference");
}

#[test]
#[serial]
fn runtime_require_loads_mjs_module_namespace() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(
        app_dir.join("esm.mjs"),
        r#"
        globalThis.__mjsLoadCount = (globalThis.__mjsLoadCount || 0) + 1;
        export const value = 42;
        export default { label: 'amber' };
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        const first = require("./esm.mjs");
        const second = require("./esm.mjs");
        `${{first.default.label}}:${{first.value}}:${{first === second}}:${{globalThis.__mjsLoadCount}}`;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "amber:42:true:1");
}

#[test]
#[serial]
fn runtime_require_mjs_with_pending_top_level_await_fails_closed() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(
        app_dir.join("pending.mjs"),
        r#"
        await new Promise(() => {});
        globalThis.__requirePendingMjsShouldNotRun = true;
        export const value = 42;
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        try {{
            require("./pending.mjs");
            "FAIL";
        }} catch (error) {{
            `${{String(error.message || error).includes("Pending") || String(error.message || error).includes("Promise")}}:${{globalThis.__requirePendingMjsShouldNotRun}}`;
        }}
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "true:undefined");
}

#[test]
#[serial]
fn runtime_require_mjs_module_namespace_reloads_after_source_change() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let esm_path = app_dir.join("esm.mjs");
    fs::write(
        &esm_path,
        r#"
        globalThis.__mjsReloadEvalCount = (globalThis.__mjsReloadEvalCount || 0) + 1;
        export const label = "first";
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let first_code = format!(
        r#"
        globalThis.__dirname = "{}";
        globalThis.__firstMjsReloadLabel = require("./esm.mjs").label;
        "#,
        path_for_js(&app_dir)
    );
    runtime.execute_code(&first_code).unwrap();

    fs::write(
        &esm_path,
        r#"
        globalThis.__mjsReloadEvalCount = (globalThis.__mjsReloadEvalCount || 0) + 1;
        export const label = "second";
        "#,
    )
    .unwrap();

    let second_code = format!(
        r#"
        globalThis.__dirname = "{}";
        globalThis.__secondMjsReloadLabel = require("./esm.mjs").label;
        "#,
        path_for_js(&app_dir)
    );
    runtime.execute_code(&second_code).unwrap();

    let result = runtime
        .execute_code(
            r#"
            `${globalThis.__firstMjsReloadLabel}:${globalThis.__secondMjsReloadLabel}:${globalThis.__mjsReloadEvalCount}`
            "#,
        )
        .unwrap();

    assert_eq!(result.trim(), "first:second:2");
}

#[test]
#[serial]
fn runtime_require_mjs_module_namespace_reloads_when_transitive_dependency_changes() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let child_path = app_dir.join("child.mjs");
    fs::write(&child_path, r#"export const label = "first";"#).unwrap();
    fs::write(
        app_dir.join("parent.mjs"),
        r#"
        import { label } from './child.mjs';
        globalThis.__mjsReloadParentEvalCount =
            (globalThis.__mjsReloadParentEvalCount || 0) + 1;
        export const parentLabel = `parent:${label}`;
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let first_code = format!(
        r#"
        globalThis.__dirname = "{}";
        globalThis.__firstMjsTransitiveReloadLabel = require("./parent.mjs").parentLabel;
        "#,
        path_for_js(&app_dir)
    );
    runtime.execute_code(&first_code).unwrap();

    fs::write(&child_path, r#"export const label = "second";"#).unwrap();

    let second_code = format!(
        r#"
        globalThis.__dirname = "{}";
        globalThis.__secondMjsTransitiveReloadLabel = require("./parent.mjs").parentLabel;
        "#,
        path_for_js(&app_dir)
    );
    runtime.execute_code(&second_code).unwrap();

    let result = runtime
        .execute_code(
            r#"
            `${globalThis.__firstMjsTransitiveReloadLabel}:${globalThis.__secondMjsTransitiveReloadLabel}:${globalThis.__mjsReloadParentEvalCount}`
            "#,
        )
        .unwrap();

    assert_eq!(result.trim(), "parent:first:parent:second:2");
}

#[test]
#[serial]
fn runtime_require_loads_js_inside_type_module_package_namespace() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("pkg");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(package_dir.join("package.json"), r#"{"type":"module"}"#).unwrap();
    fs::write(
        package_dir.join("index.js"),
        r#"
        export const answer = 42;
        export default 'amber';
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        const mod = require("./pkg/index.js");
        `${{mod.default}}:${{mod.answer}}`;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "amber:42");
}

#[test]
#[serial]
fn runtime_require_type_module_js_namespace_reloads_after_source_change() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("pkg");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(package_dir.join("package.json"), r#"{"type":"module"}"#).unwrap();
    let module_path = package_dir.join("index.js");
    fs::write(
        &module_path,
        r#"
        globalThis.__typeModuleReloadEvalCount =
            (globalThis.__typeModuleReloadEvalCount || 0) + 1;
        export const label = "first";
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let first_code = format!(
        r#"
        globalThis.__dirname = "{}";
        globalThis.__firstTypeModuleReloadLabel = require("./pkg/index.js").label;
        "#,
        path_for_js(&app_dir)
    );
    runtime.execute_code(&first_code).unwrap();

    fs::write(
        &module_path,
        r#"
        globalThis.__typeModuleReloadEvalCount =
            (globalThis.__typeModuleReloadEvalCount || 0) + 1;
        export const label = "second";
        "#,
    )
    .unwrap();

    let second_code = format!(
        r#"
        globalThis.__dirname = "{}";
        globalThis.__secondTypeModuleReloadLabel = require("./pkg/index.js").label;
        "#,
        path_for_js(&app_dir)
    );
    runtime.execute_code(&second_code).unwrap();

    let result = runtime
        .execute_code(
            r#"
            `${globalThis.__firstTypeModuleReloadLabel}:${globalThis.__secondTypeModuleReloadLabel}:${globalThis.__typeModuleReloadEvalCount}`
            "#,
        )
        .unwrap();

    assert_eq!(result.trim(), "first:second:2");
}

#[test]
#[serial]
fn runtime_require_allows_cjs_inside_type_module_package() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let package_dir = app_dir.join("pkg");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(package_dir.join("package.json"), r#"{"type":"module"}"#).unwrap();
    fs::write(
        package_dir.join("index.cjs"),
        "module.exports = { value: 'cjs-in-module-package' };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        require("./pkg/index.cjs").value;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "cjs-in-module-package");
}

#[test]
#[serial]
fn runtime_require_loads_tsx_without_jsx_using_typescript_transpile() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(
        app_dir.join("typed.tsx"),
        "const answer: number = 64; module.exports = { answer };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        require("./typed.tsx").answer;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "64");
}

#[test]
#[serial]
fn runtime_require_loads_tsx_element_syntax_with_global_react() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(
        app_dir.join("component.tsx"),
        "const view = <div />; module.exports = { view };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.React = {{
            createElement(type) {{
                return {{ type }};
            }}
        }};
        globalThis.__dirname = "{}";
        require("./component.tsx").view.type;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "div");
}

#[test]
#[serial]
fn runtime_require_loads_jsx_element_syntax_with_global_react() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(app_dir.join("component.jsx"), "module.exports = <span />;").unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.React = {{
            createElement(type) {{
                return {{ type }};
            }}
        }};
        globalThis.__dirname = "{}";
        require("./component.jsx").type;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "span");
}

#[test]
#[serial]
fn runtime_module_require_uses_module_directory_not_global_dirname() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    let lib_dir = app_dir.join("lib");
    fs::create_dir_all(&lib_dir).unwrap();
    fs::write(lib_dir.join("child.js"), "exports.value = 456;").unwrap();
    fs::write(
        lib_dir.join("index.js"),
        r#"
        globalThis.__dirname = "/";
        module.exports = require("./child");
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        require("./lib").value;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "456");
}

#[test]
#[serial]
fn runtime_require_resolve_returns_resolved_file_path() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let util_path = app_dir.join("util.js");
    fs::write(&util_path, "exports.value = 1;").unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        require.resolve("./util");
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(
        result.trim(),
        fs::canonicalize(util_path).unwrap().to_string_lossy()
    );
}

#[test]
#[serial]
fn runtime_require_loads_typescript_module() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(
        app_dir.join("typed.ts"),
        "const answer: number = 42; module.exports = { answer };",
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        require("./typed").answer;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "42");
}

#[test]
#[serial]
fn runtime_require_loads_typescript_enum_module_with_compiler() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(
        app_dir.join("typed.ts"),
        r#"
        enum Mode {
            Ready = "ready"
        }
        module.exports = { value: Mode.Ready };
        "#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        require("./typed").value;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "ready");
}

#[test]
#[serial]
fn runtime_require_loads_json_module_as_object() {
    let temp = tempfile::tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(
        app_dir.join("data.json"),
        r#"{"answer":42,"nested":{"ok":true},"items":[1,2,3]}"#,
    )
    .unwrap();

    let mut runtime = MinimalRuntime::new().unwrap();
    let code = format!(
        r#"
        globalThis.__dirname = "{}";
        const first = require("./data");
        const second = require("./data.json");
        first.answer === 42 &&
          first.nested.ok === true &&
          first.items.length === 3 &&
          first === second;
        "#,
        path_for_js(&app_dir)
    );

    let result = runtime.execute_code(&code).unwrap();

    assert_eq!(result.trim(), "true");
}

#[test]
#[serial]
fn runtime_require_supports_node_prefix_for_builtin_modules() {
    let mut runtime = MinimalRuntime::new().unwrap();

    let result = runtime
        .execute_code(
            r#"
            const pathFromNodePrefix = require("node:path");
            const pathFromPlainName = require("path");
            pathFromNodePrefix.sep === pathFromPlainName.sep &&
              pathFromNodePrefix.join("a", "b") === pathFromPlainName.join("a", "b");
            "#,
        )
        .unwrap();

    assert_eq!(result.trim(), "true");
}

#[test]
#[serial]
fn runtime_require_supports_node_prefix_for_performance_builtin() {
    let mut runtime = MinimalRuntime::new().unwrap();

    let result = runtime
        .execute_code(
            r#"
            const prefixed = require("node:performance");
            const bare = require("performance");
            prefixed === performance &&
              bare === performance &&
              prefixed === bare &&
              typeof prefixed.now === "function";
            "#,
        )
        .unwrap();

    assert_eq!(result.trim(), "true");
}

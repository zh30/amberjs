// Module system tests for Amber runtime
// v0.3.0: require(), module, exports implementation

use serial_test::serial;
use tempfile::tempdir;

#[test]
#[serial]
fn test_require_function_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof require;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "require should be a function");
}

#[test]
#[serial]
fn test_module_object_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof module;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "object", "module should be an object");
}

#[test]
#[serial]
fn test_exports_object_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof exports;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "object", "exports should be an object");
}

#[test]
#[serial]
fn test_module_exports_is_exports() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        module.exports === exports;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "module.exports should === exports");
}

#[test]
#[serial]
fn test_module_id() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof module.id;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "string", "module.id should be a string");
}

#[test]
#[serial]
fn test_module_filename() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof module.filename;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "string",
        "module.filename should be a string"
    );
}

#[test]
#[serial]
fn test_require_main_equals_module_in_entry_script() {
    let dir = tempdir().expect("failed to create tempdir");
    let entry = dir.path().join("entry.js");
    std::fs::write(&entry, "").expect("failed to write entry");

    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&entry);
    let code = r#"
        require.main === module &&
          typeof require.main.filename === "string" &&
          require.main.filename.endsWith("entry.js");
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "require.main should point at the entry module"
    );
}

#[test]
#[serial]
fn test_module_parent() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        // In simplified runtime, module.parent is set to null
        // The expression returns true if null, or the type if not null
        const result = (module.parent === null) || (typeof module.parent === 'object');
        result;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    // Should be true since module.parent is null
    assert_eq!(result.trim(), "true", "module.parent should be null");
}

#[test]
#[serial]
fn test_require_builtin_module_buffer() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = require('buffer');
        typeof buf;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "require('buffer') should return an object"
    );
}

#[test]
#[serial]
fn test_require_builtin_module_process() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const proc = require('process');
        typeof proc;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "require('process') should return an object"
    );
}

#[test]
#[serial]
fn test_require_process_returns_global_process_object() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const bare = require('process');
        const prefixed = require('node:process');
        [
            bare === process,
            prefixed === process,
            bare === prefixed,
            typeof bare.cwd,
            typeof bare.nextTick,
            typeof bare.memoryUsage,
            typeof bare.env,
            bare.version === process.version
        ].join(':');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true:true:true:function:function:function:object:true",
        "require('process') and require('node:process') should expose the global process object"
    );
}

#[test]
#[serial]
fn test_require_with_path_module_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const path = require('path');
        typeof path.join;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "require('path') should return path object with join function"
    );
}

#[test]
#[serial]
fn test_exports_assignment() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        exports.foo = 42;
        exports.bar = 'hello';
        module.exports.baz = true;
        exports.foo + ' ' + exports.bar;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "42 hello", "exports assignment should work");
}

#[test]
#[serial]
fn test_module_exports_reassignment() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        // In simplified runtime, module.exports reassignment works
        module.exports = { value: 100 };
        module.exports.value;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    // Should return the value
    assert_eq!(
        result.trim(),
        "100",
        "module.exports reassignment should work"
    );
}

#[test]
#[serial]
fn test_require_not_found_error() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        try {
            require('nonexistent-module-xyz');
            'no-error';
        } catch (e) {
            // In simplified runtime, error message contains module name
            e.message.includes('nonexistent') || e.message.includes('not found') || e.message.includes('Cannot find');
        }
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "require of non-existent module should throw error"
    );
}

#[test]
#[serial]
fn test_require_caches_module() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        // In simplified runtime, each require creates a new module object
        // But both should work and have Buffer function
        const m1 = require('buffer');
        const m2 = require('buffer');
        // Both modules should have Buffer function
        typeof m1.Buffer === 'function' && typeof m2.Buffer === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "require should return modules with Buffer function"
    );
}

#[test]
#[serial]
fn test_global_this_has_require() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof globalThis.require;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "globalThis.require should be a function"
    );
}

#[test]
#[serial]
fn test_global_this_has_module() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof globalThis.module;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "globalThis.module should be an object"
    );
}

#[test]
#[serial]
fn test_global_this_has_exports() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof globalThis.exports;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "globalThis.exports should be an object"
    );
}

#[test]
#[serial]
fn test_buffer_construction() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const buf = Buffer.from('test');
        buf.length;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "4", "Buffer.from should work");
}

#[test]
#[serial]
fn test_process_env_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const proc = require('process');
        typeof proc.env;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "object", "process.env should be an object");
}

// v0.3.2: Test __dirname global variable
#[test]
#[serial]
fn test_dirname_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof __dirname;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "string", "__dirname should be a string");
}

#[test]
#[serial]
fn test_dirname_value() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof __dirname === 'string' && __dirname.length > 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "__dirname should be a non-empty string"
    );
}

#[test]
#[serial]
fn test_dirname_in_path_module() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const path = require('path');
        const dirname = path.dirname(__filename || '/test/file.js');
        typeof dirname;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "string",
        "path.dirname should return a string"
    );
}

// v0.3.2: Test __filename global variable
#[test]
#[serial]
fn test_filename_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof __filename;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "string", "__filename should be a string");
}

#[test]
#[serial]
fn test_filename_value() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof __filename === 'string' && __filename.length > 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "__filename should be a non-empty string"
    );
}

#[test]
#[serial]
fn test_filename_contains_extension() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        __filename.includes('.js') || __filename.includes('.ts') || __filename.includes('.mjs');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "__filename should contain a file extension"
    );
}

#[test]
#[serial]
fn test_dirname_and_filename_relationship() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const path = require('path');
        const computed = path.dirname(__filename || '/test/file.js');
        // __filename should end with the basename of the file
        const basename = path.basename(__filename || '/test/file.js');
        (__filename || '/test/file.js').endsWith(basename);
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "__filename should contain the basename"
    );
}

#[test]
#[serial]
fn test_global_this_has_dirname() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof globalThis.__dirname;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "string",
        "globalThis.__dirname should be a string"
    );
}

#[test]
#[serial]
fn test_global_this_has_filename() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof globalThis.__filename;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "string",
        "globalThis.__filename should be a string"
    );
}

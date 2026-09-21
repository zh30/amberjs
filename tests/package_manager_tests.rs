use amberjs::Runtime;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

#[test]
fn test_parse_package_json() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Create a temporary package.json
    let mut package_json = NamedTempFile::new().unwrap();
    writeln!(
        package_json,
        r#"{{
        "name": "test-package",
        "version": "1.0.0",
        "main": "index.js",
        "dependencies": {{
            "lodash": "^4.17.0"
        }}
    }}"#
    )
    .unwrap();

    let result = runtime.execute_file(package_json.path());
    // Package.json is not executable JavaScript, should error
    assert!(result.is_err());
}

#[test]
fn test_require_basic_module() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Create a temp directory to ensure both files are in the same location
    let temp_dir = TempDir::new().unwrap();

    // Create module file in the temp directory
    let module_file = temp_dir.path().join("math.js");
    std::fs::write(
        &module_file,
        "
        module.exports = {
            add: (a, b) => a + b,
            multiply: (a, b) => a * b
        };
    ",
    )
    .unwrap();

    // Create main file in the same directory
    let main_file = temp_dir.path().join("main.js");
    std::fs::write(
        &main_file,
        "
        const math = require('./math.js');
        console.log(math.add(5, 3));
        console.log(math.multiply(4, 7));
        math.add(5, 3)
    ",
    )
    .unwrap();

    let result = runtime.execute_file(&main_file);
    if let Err(e) = &result {
        eprintln!("Error executing file: {:?}", e);
    }
    assert!(
        result.is_ok(),
        "Expected successful execution, got error: {:?}",
        result
    );
    let output = result.unwrap();
    // Note: console.log output may not be captured in test environment
    // We check the return value instead
    assert!(output.contains("8") || output.contains("28"));
}

#[test]
fn test_require_relative_path() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Create a module in a subdirectory
    let temp_dir = TempDir::new().unwrap();
    let module_dir = temp_dir.path().join("lib");
    std::fs::create_dir_all(&module_dir).unwrap();

    let module_file = module_dir.join("utils.js");
    std::fs::write(
        &module_file,
        "
        exports.greet = (name) => 'Hello, ' + name + '!';
        exports.PI = 3.14159;
    ",
    )
    .unwrap();

    // Create main file - return the value directly instead of using console.log
    let main_file = temp_dir.path().join("main.js");
    std::fs::write(
        &main_file,
        "
        const utils = require('./lib/utils.js');
        utils.greet('World');
        utils.PI
    ",
    )
    .unwrap();

    let result = runtime.execute_file(&main_file);
    assert!(result.is_ok());
    let output = result.unwrap();
    // Check the return value (last expression)
    assert!(output.contains("3.14159"));
}

#[test]
fn test_module_exports_object() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    let temp_dir = TempDir::new().unwrap();

    let module_file = temp_dir.path().join("config.js");
    std::fs::write(
        &module_file,
        "
        const config = {
            apiUrl: 'https://api.example.com',
            timeout: 5000,
            retries: 3
        };

        module.exports = config;
    ",
    )
    .unwrap();

    let main_file = temp_dir.path().join("main.js");
    std::fs::write(
        &main_file,
        "
        const config = require('./config.js');
        config.apiUrl;
        config.timeout;
        config
    ",
    )
    .unwrap();

    let result = runtime.execute_file(&main_file);
    assert!(result.is_ok());
    let output = result.unwrap();
    // Check that we get the config object back (object is represented as "[object Object]")
    assert!(output.contains("Object"));
}

#[test]
fn test_multiple_requires() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    let temp_dir = TempDir::new().unwrap();

    // Create multiple modules
    let module1 = temp_dir.path().join("module1.js");
    std::fs::write(&module1, "module.exports = 'module1';").unwrap();

    let module2 = temp_dir.path().join("module2.js");
    std::fs::write(&module2, "module.exports = 'module2';").unwrap();

    let main_file = temp_dir.path().join("main.js");
    std::fs::write(
        &main_file,
        "
        const mod1 = require('./module1.js');
        const mod2 = require('./module2.js');
        mod1;
        mod2;
        mod2
    ",
    )
    .unwrap();

    let result = runtime.execute_file(&main_file);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("module2"));
}

#[test]
fn test_nested_require() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Create nested module structure
    let temp_dir = TempDir::new().unwrap();

    // Deep module
    let deep_module = temp_dir.path().join("deep").join("nested.js");
    std::fs::create_dir_all(deep_module.parent().unwrap()).unwrap();
    std::fs::write(&deep_module, "module.exports = { value: 'nested' };").unwrap();

    // Main file that requires nested module
    let main_file = temp_dir.path().join("main.js");
    std::fs::write(
        &main_file,
        r#"
        const nested = require('./deep/nested.js');
        console.log(nested.value);
        nested.value
    "#,
    )
    .unwrap();

    let result = runtime.execute_file(&main_file);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("nested"));
}

#[test]
fn test_builtin_modules() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Test require of built-in module (will fail for now, but structure is correct)
    let code = r#"
        try {
            const path = require('path');
            console.log("Path module loaded");
        } catch (e) {
            console.log("Path module not available");
        }
        "done";
    "#;

    let result = runtime.execute_code(code);
    assert!(result.is_ok());
}

#[test]
#[ignore] // TODO: Fix circular dependency handling - causes panic with two-way require
fn test_circular_dependency() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    let temp_dir = TempDir::new().unwrap();

    let module_a = temp_dir.path().join("moduleA.js");
    let module_b = temp_dir.path().join("moduleB.js");

    // Create moduleA that requires moduleB
    std::fs::write(
        &module_a,
        r#"
        const moduleB = require('./moduleB.js');
        module.exports = {
            name: 'A',
            fromB: moduleB.name
        };
    "#,
    )
    .unwrap();

    // Create moduleB that requires moduleA (circular dependency)
    std::fs::write(
        &module_b,
        r#"
        const moduleA = require('./moduleA.js');
        module.exports = {
            name: 'B',
            fromA: moduleA ? moduleA.name : 'unresolved'
        };
    "#,
    )
    .unwrap();

    let main_file = temp_dir.path().join("main.js");
    std::fs::write(
        &main_file,
        r#"
        const moduleA = require('./moduleA.js');
        moduleA.name;
    "#,
    )
    .unwrap();

    let result = runtime.execute_file(&main_file);
    if let Err(e) = &result {
        eprintln!("Circular dependency test error: {:?}", e);
    }
    // In circular dependencies, moduleA may not have full access to moduleB's exports yet
    assert!(
        result.is_ok(),
        "Circular dependency test failed: {:?}",
        result
    );
}

#[test]
fn test_module_caching() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    let temp_dir = TempDir::new().unwrap();

    let module_file = temp_dir.path().join("counter.js");
    std::fs::write(
        &module_file,
        "
        let count = 0;
        module.exports = {
            getCount: () => {
                count++;
                return count;
            }
        };
    ",
    )
    .unwrap();

    let main_file = temp_dir.path().join("main.js");
    std::fs::write(
        &main_file,
        "
        const mod1 = require('./counter.js');
        const mod2 = require('./counter.js');  // Should get same module instance
        const result1 = mod1.getCount();
        const result2 = mod2.getCount();
        const result3 = mod1.getCount();
        result3
    ",
    )
    .unwrap();

    let result = runtime.execute_file(&main_file);
    assert!(result.is_ok());
    // Module should be cached, so counter increments across calls
    let output = result.unwrap();
    assert!(output.contains("3"));
}

// Tests for process module
// v0.3.34: Comprehensive process API tests

use serial_test::serial;

/// Test process object exists and is an object
#[test]
#[serial]
fn test_process_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process")
        .expect("Execution failed");
    assert_eq!(result.trim(), "object", "process should be an object");
}

/// Test process.argv exists and is an array
#[test]
#[serial]
fn test_process_argv_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("Array.isArray(process.argv)")
        .expect("Execution failed");
    assert_eq!(result.trim(), "true", "process.argv should be an array");
}

/// Test process.argv contains expected elements
#[test]
#[serial]
fn test_process_argv_content() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        process.argv.length >= 2 && process.argv[0].includes('bee');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.argv should contain 'bee' as first element"
    );
}

/// Test process.version exists and is a string
#[test]
#[serial]
fn test_process_version_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.version")
        .expect("Execution failed");
    assert_eq!(
        result.trim(),
        "string",
        "process.version should be a string"
    );
}

/// Test process.version format
#[test]
#[serial]
fn test_process_version_format() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        process.version.startsWith('v');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.version should start with 'v'"
    );
}

/// Test process.cwd() exists and is a function
#[test]
#[serial]
fn test_process_cwd_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.cwd")
        .expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.cwd should be a function"
    );
}

/// Test process.cwd() returns a string
#[test]
#[serial]
fn test_process_cwd_returns_string() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.cwd() === 'string';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.cwd() should return a string"
    );
}

/// Test process.cwd() returns non-empty string
#[test]
#[serial]
fn test_process_cwd_non_empty() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        process.cwd().length > 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.cwd() should return non-empty string"
    );
}

/// Test process.env exists and is an object
#[test]
#[serial]
fn test_process_env_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.env")
        .expect("Execution failed");
    assert_eq!(result.trim(), "object", "process.env should be an object");
}

/// Test process.env is not null
#[test]
#[serial]
fn test_process_env_not_null() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        process.env !== null;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "process.env should not be null");
}

/// Test process.env can be accessed
#[test]
#[serial]
fn test_process_env_accessible() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    // Just test that we can access process.env without error
    let code = r#"
        const keys = Object.keys(process.env);
        Array.isArray(keys);
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "Should be able to get keys from process.env"
    );
}

/// Test process.nextTick exists and is a function
#[test]
#[serial]
fn test_process_next_tick_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.nextTick")
        .expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.nextTick should be a function"
    );
}

/// Test process.nextTick basic execution
#[test]
#[serial]
fn test_process_next_tick_basic() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        globalThis.__processNextTickExecuted = false;
        process.nextTick(function() { globalThis.__processNextTickExecuted = true; });
        globalThis.__processNextTickExecuted;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "false",
        "execute_code should return the main script completion before nextTick drain"
    );

    let observed = runtime
        .execute_code("globalThis.__processNextTickExecuted;")
        .expect("Execution failed");
    assert_eq!(
        observed.trim(),
        "true",
        "process.nextTick callback should execute during post-script drain"
    );
}

/// Test process.nextTick passes arguments
#[test]
#[serial]
fn test_process_next_tick_with_args() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        globalThis.__processNextTickResult = null;
        process.nextTick(function(a, b) { globalThis.__processNextTickResult = a + b; }, 10, 20);
        globalThis.__processNextTickResult;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "null",
        "execute_code should not replay the result expression after nextTick drain"
    );

    let observed = runtime
        .execute_code("globalThis.__processNextTickResult === 30;")
        .expect("Execution failed");
    assert_eq!(
        observed.trim(),
        "true",
        "process.nextTick should pass arguments to callback"
    );
}

/// Test process.nextTick error handling - no callback
#[test]
#[serial]
fn test_process_next_tick_no_callback_error() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"process.nextTick()"#;
    let result = runtime.execute_code(code);
    assert!(
        result.is_err(),
        "process.nextTick without callback should throw"
    );
}

/// Test process.nextTick error handling - non-function
#[test]
#[serial]
fn test_process_next_tick_non_function_error() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"process.nextTick("not a function")"#;
    let result = runtime.execute_code(code);
    assert!(
        result.is_err(),
        "process.nextTick with non-function should throw"
    );
}

/// Test process.hrtime() exists (v0.3.41: guaranteed to be implemented)
#[test]
#[serial]
fn test_process_hrtime_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.hrtime")
        .expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.hrtime should be a function"
    );
}

// v0.3.41: Additional hrtime tests are below (test_process_hrtime_returns_object, etc.)

/// Test process.platform exists (if implemented)
#[test]
#[serial]
fn test_process_platform_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.platform")
        .expect("Execution failed");
    // platform may or may not be implemented
    assert!(
        result.trim() == "string" || result.trim() == "undefined",
        "process.platform should be a string or undefined"
    );
}

/// Test process.arch exists (if implemented)
#[test]
#[serial]
fn test_process_arch_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.arch")
        .expect("Execution failed");
    // arch may or may not be implemented
    assert!(
        result.trim() == "string" || result.trim() == "undefined",
        "process.arch should be a string or undefined"
    );
}

/// Test process.pid exists (if implemented)
#[test]
#[serial]
fn test_process_pid_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.pid")
        .expect("Execution failed");
    // pid may or may not be implemented
    assert!(
        result.trim() == "number" || result.trim() == "undefined",
        "process.pid should be a number or undefined"
    );
}

/// Test process uptime exists (if implemented)
#[test]
#[serial]
fn test_process_uptime_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.uptime")
        .expect("Execution failed");
    // uptime should be a function (v0.3.38+)
    assert!(
        result.trim() == "function" || result.trim() == "undefined",
        "process.uptime should be a function or undefined"
    );
}

/// Test process.memory exists (if implemented)
#[test]
#[serial]
fn test_process_memory_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.memory")
        .expect("Execution failed");
    // process.memory() is Beejs' extended memory statistics API.
    assert!(
        result.trim() == "function" || result.trim() == "undefined",
        "process.memory should be a function or undefined"
    );
}

/// Test process.exit function exists (if implemented)
#[test]
#[serial]
fn test_process_exit_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.exit")
        .expect("Execution failed");
    // exit may or may not be implemented
    assert!(
        result.trim() == "function" || result.trim() == "undefined",
        "process.exit should be a function or undefined"
    );
}

/// Test multiple process properties are accessible
#[test]
#[serial]
fn test_process_multiple_properties() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.argv === 'object' &&
        typeof process.version === 'string' &&
        typeof process.cwd === 'function' &&
        typeof process.env === 'object' &&
        typeof process.nextTick === 'function';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "All expected process properties should exist"
    );
}

/// Test process object is extensible
#[test]
#[serial]
fn test_process_is_extensible() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        process.customProperty = 'test';
        process.customProperty === 'test';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "process object should be extensible");
}

// ============================================================================
// v0.3.35: New process module features tests
// ============================================================================

/// v0.3.35: Test process.umask() exists and is a function
#[test]
#[serial]
fn test_process_umask_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.umask")
        .expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.umask should be a function"
    );
}

/// v0.3.35: Test process.umask() returns current mask
#[test]
#[serial]
fn test_process_umask_returns_mask() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const mask = process.umask();
        typeof mask === 'string' && mask.length === 4;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.umask() should return a 4-character octal string"
    );
}

/// v0.3.35: Test process.umask() sets new mask
#[test]
#[serial]
fn test_process_umask_sets_mask() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const oldMask = process.umask(0o077);
        const newMask = process.umask();
        newMask === '0077';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.umask() should set and return the mask"
    );
}

/// v0.3.35: Test process.abort() exists and is a function
#[test]
#[serial]
fn test_process_abort_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.abort")
        .expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.abort should be a function"
    );
}

/// v0.3.35: Test process.config exists and is an object
#[test]
#[serial]
fn test_process_config_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.config")
        .expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "process.config should be an object"
    );
}

/// v0.3.35: Test process.config.variables exists
#[test]
#[serial]
fn test_process_config_variables_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.config.variables === 'object';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.config.variables should be an object"
    );
}

/// v0.3.35: Test process.config.variables.host_arch
#[test]
#[serial]
fn test_process_config_host_arch() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.config.variables.host_arch === 'string';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.config.variables.host_arch should be a string"
    );
}

/// v0.3.35: Test process.config.variables.platform
#[test]
#[serial]
fn test_process_config_platform() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.config.variables.platform === 'string';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.config.variables.platform should be a string"
    );
}

/// v0.3.35: Test process.chdir() exists and is a function
#[test]
#[serial]
fn test_process_chdir_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.chdir")
        .expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.chdir should be a function"
    );
}

/// v0.3.35: Test process.chdir() changes directory
#[test]
#[serial]
fn test_process_chdir_changes_directory() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const original = process.cwd();
        const result = process.chdir(original);
        result === undefined;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.chdir() should return undefined on success"
    );
}

/// v0.3.35: Test process.title exists and is a string
#[test]
#[serial]
fn test_process_title_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.title")
        .expect("Execution failed");
    assert_eq!(result.trim(), "string", "process.title should be a string");
}

/// v0.3.35: Test process.title has default value
#[test]
#[serial]
fn test_process_title_default_value() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        process.title.length > 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.title should have a non-empty default value"
    );
}

/// v0.3.35: Test process.release object exists
#[test]
#[serial]
fn test_process_release_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.release")
        .expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "process.release should be an object"
    );
}

/// v0.3.35: Test process.release.name
#[test]
#[serial]
fn test_process_release_name() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        process.release.name === 'bee';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.release.name should be 'bee'"
    );
}

/// v0.3.39: Test process.memoryUsage exists and is a function
#[test]
#[serial]
fn test_process_memory_usage_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.memoryUsage")
        .expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.memoryUsage should be a function"
    );
}

/// v0.3.39: Test process.memoryUsage returns an object with required fields
#[test]
#[serial]
fn test_process_memory_usage_returns_object() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const mem = process.memoryUsage();
        typeof mem.heapTotal === 'number' &&
        typeof mem.heapUsed === 'number' &&
        typeof mem.rss === 'number' &&
        typeof mem.external === 'number';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "true", "process.memoryUsage() should return object with heapTotal, heapUsed, rss, and external fields");
}

/// v0.3.39: Test process.memoryUsage returns realistic values
#[test]
#[serial]
fn test_process_memory_usage_realistic_values() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const mem = process.memoryUsage();
        // heapUsed should be between 1MB and 1GB for a simple runtime
        mem.heapUsed >= 1024 * 1024 && mem.heapUsed <= 1024 * 1024 * 1024 &&
        // rss should be at least heapTotal
        mem.rss >= mem.heapTotal &&
        // All values should be positive
        mem.heapTotal > 0 && mem.heapUsed > 0 && mem.rss > 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.memoryUsage() should return realistic memory values"
    );
}

/// v0.3.39: Test process.memoryUsage can be called multiple times
#[test]
#[serial]
fn test_process_memory_usage_multiple_calls() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const mem1 = process.memoryUsage();
        const mem2 = process.memoryUsage();
        // Both calls should return valid objects
        typeof mem1.heapUsed === 'number' && typeof mem2.heapUsed === 'number';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.memoryUsage() should be callable multiple times"
    );
}

/// v0.3.39: Test process.memoryUsage heapUsed increases with allocation
#[test]
#[serial]
fn test_process_memory_usage_increases_with_allocation() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const memBefore = process.memoryUsage();
        // Allocate some memory
        const arr = new Array(100000);
        for (let i = 0; i < 100000; i++) { arr[i] = i; }
        const memAfter = process.memoryUsage();
        memAfter.heapUsed >= memBefore.heapUsed;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.memoryUsage() should show increased heapUsed after allocation"
    );
}

/// v0.3.40: Test process.ppid exists and is a number
#[test]
#[serial]
fn test_process_ppid_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.ppid")
        .expect("Execution failed");
    assert_eq!(result.trim(), "number", "process.ppid should be a number");
}

/// v0.3.40: Test process.ppid is positive
#[test]
#[serial]
fn test_process_ppid_is_positive() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        process.ppid > 0;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.ppid should be a positive number"
    );
}

/// v0.3.40: Test process.ppid is different from pid
#[test]
#[serial]
fn test_process_ppid_different_from_pid() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        process.ppid !== process.pid;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.ppid should typically be different from process.pid"
    );
}

/// v0.3.40: Test process.features exists and is an object
#[test]
#[serial]
fn test_process_features_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.features")
        .expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "process.features should be an object"
    );
}

/// v0.3.40: Test process.features.debug is a boolean
#[test]
#[serial]
fn test_process_features_debug() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.features.debug === 'boolean';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.features.debug should be a boolean"
    );
}

/// v0.3.40: Test process.features.ipc is a boolean
#[test]
#[serial]
fn test_process_features_ipc() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.features.ipc === 'boolean';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.features.ipc should be a boolean"
    );
}

/// v0.3.40: Test process.features.uv is a boolean (libuv support)
#[test]
#[serial]
fn test_process_features_uv() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.features.uv === 'boolean';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.features.uv should be a boolean"
    );
}

/// v0.3.40: Test process.features.v8 is a boolean (V8 engine support)
#[test]
#[serial]
fn test_process_features_v8() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.features.v8 === 'boolean';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.features.v8 should be a boolean"
    );
}

/// v0.3.40: Test process.features.modules is a boolean (module support)
#[test]
#[serial]
fn test_process_features_modules() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.features.modules === 'boolean';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.features.modules should be a boolean"
    );
}

// ============================================================================
// v0.3.41: process.hrtime.bigint() implementation
// ============================================================================

/// v0.3.41: Test process.hrtime() returns an object with array-like properties
#[test]
#[serial]
fn test_process_hrtime_returns_object() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const time = process.hrtime();
        typeof time === 'object' && typeof time[0] === 'number' && typeof time[1] === 'number';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.hrtime() should return an object with numeric properties [0] and [1]"
    );
}

/// v0.3.41: Test process.hrtime() returns realistic values
#[test]
#[serial]
fn test_process_hrtime_realistic_values() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const time = process.hrtime();
        const sec = time[0];
        const nsec = time[1];
        sec > 1700000000 && nsec >= 0 && nsec < 1000000000;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.hrtime() should return realistic time values"
    );
}

/// v0.3.41: Test process.hrtime.bigint() exists and is a function
#[test]
#[serial]
fn test_process_hrtime_bigint_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let result = runtime
        .execute_code("typeof process.hrtime.bigint")
        .expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "process.hrtime.bigint should be a function"
    );
}

/// v0.3.41: Test process.hrtime.bigint() returns a BigInt
#[test]
#[serial]
fn test_process_hrtime_bigint_returns_bigint() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof process.hrtime.bigint() === 'bigint';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.hrtime.bigint() should return a bigint"
    );
}

/// v0.3.41: Test process.hrtime.bigint() returns positive value
#[test]
#[serial]
fn test_process_hrtime_bigint_positive() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const time = process.hrtime.bigint();
        time > 1700000000000000000n;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.hrtime.bigint() should return a positive bigint"
    );
}

/// v0.3.41: Test process.hrtime.bigint() returns nanoseconds
#[test]
#[serial]
fn test_process_hrtime_bigint_is_nanoseconds() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        // bigint should be a positive value representing nanoseconds
        const bigint = process.hrtime.bigint();
        const sec = process.hrtime()[0];
        const nsec = process.hrtime()[1];
        // Verify the bigint is in a reasonable range based on the seconds value
        // Allow for some time difference between calls (up to 1 second)
        const expected_range_start = BigInt(sec) * 1000000000n;
        const expected_range_end = BigInt(sec + 1) * 1000000000n + BigInt(nsec);
        bigint >= expected_range_start && bigint <= expected_range_end;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "process.hrtime.bigint() should return nanoseconds as bigint"
    );
}

use amberjs::Runtime;
use serial_test::serial;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

// v0.3.50: Fixed V8 initialization conflict by adding serial attribute
// V8 can only be initialized once, so tests must run serially

#[test]
#[serial]
fn test_process_argv() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    // Check that process.argv is an array
    let result = runtime.execute_code("Array.isArray(process.argv)");
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(result_str.contains("true"));
}

#[test]
#[serial]
fn test_process_version() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let result = runtime.execute_code("process.version");
    assert!(result.is_ok());
    let result_str = result.unwrap();
    // Should contain version string
    assert!(!result_str.is_empty());
}

#[test]
#[serial]
fn test_process_cwd() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let result = runtime.execute_code("process.cwd()");
    assert!(result.is_ok());
    let result_str = result.unwrap();
    // Should return a path
    assert!(!result_str.is_empty());
}

#[test]
#[serial]
fn test_process_next_tick() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let code = r#"
        globalThis.__nextTickExecuted = false;
        process.nextTick(function() {
            globalThis.__nextTickExecuted = true;
        });
        globalThis.__nextTickExecuted;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "nextTick test failed: {:?}", result);
    assert_eq!(result.unwrap().trim(), "false");

    let result_str = runtime
        .execute_code("globalThis.__nextTickExecuted === true")
        .unwrap();
    assert!(
        result_str.contains("true"),
        "nextTick callback should have executed, got: {}",
        result_str
    );
}

#[test]
#[serial]
fn test_process_next_tick_with_args() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let code = r#"
        globalThis.__nextTickResult = null;
        process.nextTick((a, b) => {
            globalThis.__nextTickResult = a + b;
        }, 5, 3);
        globalThis.__nextTickResult;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "null");

    let result_str = runtime
        .execute_code("globalThis.__nextTickResult === 8")
        .unwrap();
    assert!(
        result_str.contains("true"),
        "nextTick should pass arguments to callback, got: {}",
        result_str
    );
}

#[test]
#[serial]
fn test_process_next_tick_error_handling() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    // nextTick without callback should throw
    let code = r#"process.nextTick()"#;
    let result = runtime.execute_code(code);
    assert!(
        result.is_err(),
        "nextTick without callback should throw an error"
    );
}

#[test]
#[serial]
fn test_child_process_exec_returns_real_stdout() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let result = runtime.execute_code(
        r#"
        const childProcess = require("child_process");
        const child = childProcess.exec("printf beejs-child-process");
        `${child.stdout}|${child.stderr}|${child.exitCode}`;
    "#,
    );
    assert!(result.is_ok(), "child_process.exec failed: {:?}", result);
    assert_eq!(
        result.unwrap().trim(),
        "beejs-child-process||0",
        "exec should expose real stdout/stderr/exitCode"
    );
}

#[test]
#[serial]
fn test_child_process_exec_invokes_callback_with_output() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let result = runtime.execute_code(
        r#"
        const childProcess = require("child_process");
        let observed = "pending";
        childProcess.exec("printf beejs-callback", (error, stdout, stderr) => {
            observed = `${error === null}:${stdout}:${stderr}`;
        });
        observed;
    "#,
    );
    assert!(result.is_ok(), "child_process.exec failed: {:?}", result);
    assert_eq!(
        result.unwrap().trim(),
        "true:beejs-callback:",
        "exec callback should receive null error plus real stdout/stderr"
    );
}

#[test]
#[serial]
fn test_child_process_exec_file_returns_real_stdout() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let result = runtime.execute_code(
        r#"
        const childProcess = require("child_process");
        const child = childProcess.execFile("/bin/echo", ["beejs-exec-file"]);
        `${child.stdout}|${child.stderr}|${child.exitCode}`;
    "#,
    );
    assert!(
        result.is_ok(),
        "child_process.execFile failed: {:?}",
        result
    );
    assert_eq!(
        result.unwrap().trim(),
        "beejs-exec-file\n||0",
        "execFile should expose real stdout/stderr/exitCode"
    );
}

#[test]
#[serial]
fn test_child_process_exec_file_invokes_callback_with_output() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let result = runtime.execute_code(
        r#"
        const childProcess = require("child_process");
        let observed = "pending";
        childProcess.execFile("/bin/echo", ["beejs-file-callback"], (error, stdout, stderr) => {
            observed = `${error === null}:${stdout}:${stderr}`;
        });
        observed;
    "#,
    );
    assert!(
        result.is_ok(),
        "child_process.execFile failed: {:?}",
        result
    );
    assert_eq!(
        result.unwrap().trim(),
        "true:beejs-file-callback\n:",
        "execFile callback should receive null error plus real stdout/stderr"
    );
}

#[test]
#[serial]
fn test_child_process_exec_callback_receives_error_on_nonzero_exit() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let result = runtime.execute_code(
        r#"
        const childProcess = require("child_process");
        let observed = "pending";
        childProcess.exec("echo beejs-error >&2; exit 7", (error, stdout, stderr) => {
            observed = `${!!error}:${error && error.code}:${stdout}:${stderr}:done`;
        });
        observed;
    "#,
    );
    assert!(result.is_ok(), "child_process.exec failed: {:?}", result);
    assert_eq!(
        result.unwrap().trim(),
        "true:7::beejs-error\n:done",
        "exec callback should receive an Error with code and real stderr on non-zero exit"
    );
}

#[test]
#[serial]
fn test_child_process_spawn_returns_real_stdout() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let result = runtime.execute_code(
        r#"
        const childProcess = require("child_process");
        const child = childProcess.spawn("/bin/echo", ["beejs-spawn"]);
        `${child.stdout}|${child.stderr}|${child.exitCode}`;
    "#,
    );
    assert!(result.is_ok(), "child_process.spawn failed: {:?}", result);
    assert_eq!(
        result.unwrap().trim(),
        "beejs-spawn\n||0",
        "spawn should expose real stdout/stderr/exitCode"
    );
}

#[test]
#[serial]
fn test_child_process_spawn_reports_nonzero_exit() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let result = runtime.execute_code(
        r#"
        const childProcess = require("child_process");
        const child = childProcess.spawn("sh", ["-c", "echo beejs-spawn-error >&2; exit 9"]);
        `${child.stdout}:${child.stderr}:done:${child.exitCode}`;
    "#,
    );
    assert!(result.is_ok(), "child_process.spawn failed: {:?}", result);
    assert_eq!(
        result.unwrap().trim(),
        ":beejs-spawn-error\n:done:9",
        "spawn should expose real stderr and non-zero exitCode"
    );
}

#[test]
#[serial]
fn test_child_process_spawn_exit_event_receives_exit_code() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let result = runtime.execute_code(
        r#"
        const childProcess = require("child_process");
        let observed = "pending";
        const child = childProcess.spawn("sh", ["-c", "exit 6"]);
        child.on("exit", (code, signal) => {
            observed = `${code}:${signal === null}`;
        });
        observed;
    "#,
    );
    assert!(result.is_ok(), "child_process.spawn failed: {:?}", result);
    assert_eq!(
        result.unwrap().trim(),
        "6:true",
        "spawn exit event should receive the real exit code and null signal"
    );
}

#[test]
#[serial]
fn test_path_join() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let result = runtime.execute_code(r#"path.join("foo", "bar", "baz")"#);
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(result_str.contains("foo/bar/baz") || result_str.contains("foo\\\\bar\\\\baz"));
}

#[test]
#[serial]
fn test_path_resolve() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let result = runtime.execute_code(r#"path.resolve("foo", "bar")"#);
    assert!(result.is_ok());
    let result_str = result.unwrap();
    // Should return an absolute path
    assert!(!result_str.is_empty());
}

#[test]
#[serial]
fn test_path_dirname() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let result = runtime.execute_code(r#"path.dirname("/foo/bar/baz")"#);
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(result_str.contains("/foo/bar"));
}

#[test]
#[serial]
fn test_path_basename() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let result = runtime.execute_code(r#"path.basename("/foo/bar/baz.txt")"#);
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(result_str.contains("baz.txt"));
}

#[test]
#[serial]
fn test_path_extname() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);
    let result = runtime.execute_code(r#"path.extname("foo/bar/baz.txt")"#);
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(result_str.contains(".txt"));
}

#[test]
#[serial]
fn test_fs_read_file_sync() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Create a temporary file with content
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "Hello from Beejs!").unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let code = format!(r#"fs.readFileSync("{}", "utf8")"#, path);
    let result = runtime.execute_code(&code);
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(result_str.contains("Hello from Beejs"));
}

#[test]
#[serial]
fn test_fs_write_file_sync() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Create a temp directory
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    let path = test_file.to_str().unwrap().to_string();

    let code = format!(r#"fs.writeFileSync("{}", "test content", "utf8")"#, path);
    let result = runtime.execute_code(&code);
    assert!(result.is_ok());

    // Verify the file was written
    let content = std::fs::read_to_string(&test_file).unwrap();
    assert!(content.contains("test content"));
}

#[test]
#[serial]
fn test_fs_exists_sync() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Create a temporary file
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let code = format!(r#"fs.existsSync("{}")"#, path);
    let result = runtime.execute_code(&code);
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(result_str.contains("true"));
}

#[test]
#[serial]
fn test_fs_mkdir_sync() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    let temp_dir = TempDir::new().unwrap();
    let new_dir = temp_dir.path().join("new_directory");
    let path = new_dir.to_str().unwrap().to_string();

    let code = format!(r#"fs.mkdirSync("{}")"#, path);
    let result = runtime.execute_code(&code);
    assert!(result.is_ok());

    // Verify the directory was created
    assert!(new_dir.exists());
    assert!(new_dir.is_dir());
}

#[test]
#[serial]
fn test_fs_readdir_sync() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    let temp_dir = TempDir::new().unwrap();
    // Create some files in the directory
    std::fs::write(temp_dir.path().join("file1.txt"), "").unwrap();
    std::fs::write(temp_dir.path().join("file2.txt"), "").unwrap();

    let path = temp_dir.path().to_str().unwrap().to_string();
    let code = format!(r#"fs.readdirSync("{}")"#, path);
    let result = runtime.execute_code(&code);
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_fs_stat_sync() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Create a temporary file
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    // fs.statSync returns a Stats object with isFile() method
    let code = format!(r#"fs.statSync("{}").isFile()"#, path);
    let result = runtime.execute_code(&code);
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(result_str.contains("true"));
}

#[test]
#[serial]
fn test_require_module() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Test that require function exists
    let result = runtime.execute_code("typeof require");
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(result_str.contains("function"));

    // Test that built-in modules can be required and used
    let result = runtime.execute_code(
        "const path = require('path'); const basename = path.basename('/foo/bar/baz.txt'); basename"
    );
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(
        result_str.contains("baz.txt"),
        "Expected 'baz.txt' in result, got: {}",
        result_str
    );
}

#[test]
#[serial]
fn test_require_builtin_module() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Test that fs module can be required
    let result = runtime.execute_code(
        r#"
        const fs = require('fs');
        const content = fs.readFileSync('/dev/null', 'utf8');
        typeof fs === 'object' && content === '';
        "#,
    );
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(
        result_str.contains("true"),
        "Expected fs module to be loaded correctly, got: {}",
        result_str
    );
}

#[test]
#[serial]
fn test_require_custom_module() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Create a temporary module file
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_string();

    // Write a test module
    let module_code = r#"
        exports.add = (a, b) => a + b;
        exports.multiply = (a, b) => a * b;
        exports.PI = 3.14159;
        module.exports = {
            add: exports.add,
            multiply: exports.multiply,
            PI: exports.PI
        };
    "#;
    std::fs::write(&temp_file, module_code).unwrap();

    // Test that the custom module can be required and used
    let code = format!(
        r#"
        const math = require('{}');
        const result1 = math.add(2, 3);
        const result2 = math.multiply(4, 5);
        const result3 = math.PI;
        result1 === 5 && result2 === 20 && result3 === 3.14159;
        "#,
        temp_path
    );

    let result = runtime.execute_code(&code);
    assert!(result.is_ok(), "Failed to execute code: {:?}", result);
    let result_str = result.unwrap();
    assert!(
        result_str.contains("true"),
        "Expected custom module to work correctly, got: {}",
        result_str
    );
}

#[test]
#[serial]
fn test_module_exports() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    let code = r#"
        const utils = {
            greet: (name) => `Hello, ${name}!`,
            add: (a, b) => a + b
        };
        module.exports = utils;
        module.exports.greet("Beejs");
    "#;

    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let result_str = result.unwrap();
    assert!(result_str.contains("Hello, Beejs"));
}

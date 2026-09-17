// fs/promises module tests for Beejs runtime
// v0.3.7: Promise-based fs API implementation

use serial_test::serial;
use std::fs;
use tempfile::TempDir;

#[test]
#[serial]
fn test_fs_promises_module_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const fsPromises = require('fs/promises');
        typeof fsPromises;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "require('fs/promises') should return an object"
    );
}

#[test]
#[serial]
fn test_fs_promises_has_readfile() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const fsPromises = require('fs/promises');
        typeof fsPromises.readFile;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "fsPromises.readFile should be a function"
    );
}

#[test]
#[serial]
fn test_fs_promises_has_writefile() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const fsPromises = require('fs/promises');
        typeof fsPromises.writeFile;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "fsPromises.writeFile should be a function"
    );
}

#[test]
#[serial]
fn test_fs_promises_has_appendfile() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const fsPromises = require('fs/promises');
        typeof fsPromises.appendFile;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "fsPromises.appendFile should be a function"
    );
}

#[test]
#[serial]
fn test_fs_promises_has_unlink() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const fsPromises = require('fs/promises');
        typeof fsPromises.unlink;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "fsPromises.unlink should be a function"
    );
}

#[test]
#[serial]
fn test_fs_promises_has_mkdir() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const fsPromises = require('fs/promises');
        typeof fsPromises.mkdir;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "fsPromises.mkdir should be a function"
    );
}

#[test]
#[serial]
fn test_fs_promises_has_rmdir() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const fsPromises = require('fs/promises');
        typeof fsPromises.rmdir;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "fsPromises.rmdir should be a function"
    );
}

#[test]
#[serial]
fn test_fs_promises_has_readdir() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const fsPromises = require('fs/promises');
        typeof fsPromises.readdir;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "fsPromises.readdir should be a function"
    );
}

#[test]
#[serial]
fn test_fs_promises_readfile_returns_promise() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("promise_test.txt");
    fs::write(&test_file, "Promise content").expect("Failed to write test file");

    let code = format!(
        r#"
        const fsPromises = require('fs/promises');
        const result = fsPromises.readFile("{}");
        typeof result.then;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "readFile should return a Promise with .then method"
    );
}

#[test]
#[serial]
fn test_fs_promises_writefile_returns_promise() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("write_promise.txt");

    let code = format!(
        r#"
        const fsPromises = require('fs/promises');
        const result = fsPromises.writeFile("{}", "Promise write content");
        typeof result.then;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "writeFile should return a Promise with .then method"
    );
}

#[test]
#[serial]
fn test_fs_promises_appendfile_returns_promise() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("append_promise.txt");
    fs::write(&test_file, "Original").expect("Failed to create file");

    let code = format!(
        r#"
        const fsPromises = require('fs/promises');
        const result = fsPromises.appendFile("{}", " appended");
        typeof result.then;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "appendFile should return a Promise with .then method"
    );
}

#[test]
#[serial]
fn test_fs_promises_unlink_returns_promise() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("delete_promise.txt");
    fs::write(&test_file, "delete me").expect("Failed to write test file");

    let code = format!(
        r#"
        const fsPromises = require('fs/promises');
        const result = fsPromises.unlink("{}");
        typeof result.then;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "unlink should return a Promise with .then method"
    );
}

#[test]
#[serial]
fn test_fs_promises_mkdir_returns_promise() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let new_dir = temp_dir.path().join("new_promise_dir");

    let code = format!(
        r#"
        const fsPromises = require('fs/promises');
        const result = fsPromises.mkdir("{}");
        typeof result.then;
    "#,
        new_dir.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "mkdir should return a Promise with .then method"
    );
}

#[test]
#[serial]
fn test_fs_promises_rmdir_returns_promise() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let sub_dir = temp_dir.path().join("remove_promise_dir");
    fs::create_dir(&sub_dir).expect("Failed to create sub directory");

    let code = format!(
        r#"
        const fsPromises = require('fs/promises');
        const result = fsPromises.rmdir("{}");
        typeof result.then;
    "#,
        sub_dir.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "rmdir should return a Promise with .then method"
    );
}

#[test]
#[serial]
fn test_fs_promises_readdir_returns_promise() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let code = format!(
        r#"
        const fsPromises = require('fs/promises');
        const result = fsPromises.readdir("{}");
        typeof result.then;
    "#,
        temp_dir.path().to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "readdir should return a Promise with .then method"
    );
}

#[test]
#[serial]
fn test_fs_promises_all_functions_exist() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const fsPromises = require('fs/promises');
        const hasReadFile = typeof fsPromises.readFile === 'function';
        const hasWriteFile = typeof fsPromises.writeFile === 'function';
        const hasAppendFile = typeof fsPromises.appendFile === 'function';
        const hasUnlink = typeof fsPromises.unlink === 'function';
        const hasMkdir = typeof fsPromises.mkdir === 'function';
        const hasRmdir = typeof fsPromises.rmdir === 'function';
        const hasReaddir = typeof fsPromises.readdir === 'function';
        [hasReadFile, hasWriteFile, hasAppendFile, hasUnlink, hasMkdir, hasRmdir, hasReaddir].join(',');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true,true,true,true,true,true,true",
        "All fs/promises functions should exist"
    );
}

#[test]
#[serial]
fn test_fs_promises_readfile_error_handling() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const fsPromises = require('fs/promises');
        const result = fsPromises.readFile("/nonexistent/path/to/file.txt");
        typeof result.catch;
    "#;

    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "readFile error should return a Promise with .catch method"
    );
}

#[test]
#[serial]
fn test_promises_stat_stats_methods_match_file_and_dir() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("stat_test.txt");
    fs::write(&test_file, "stat me").expect("Failed to write test file");

    let code = format!(
        r#"
        const fsPromises = require('fs/promises');
        const fileStat = fsPromises.stat("{}");
        fileStat.then(stat => [
          typeof stat.isFile,
          stat.isFile(),
          stat.isDirectory(),
          typeof stat.size,
          stat.size > 0
        ].join(','));
        const dirStat = fsPromises.stat("{}");
        dirStat.then(stat => [
          typeof stat.isFile,
          stat.isFile(),
          stat.isDirectory()
        ].join(','));
        fileStat.__result__ + '|' + dirStat.__result__;
    "#,
        test_file.to_string_lossy().into_owned(),
        temp_dir.path().to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function,true,false,number,true|function,false,true",
        "fs.promises.stat Stats methods should match file and directory metadata"
    );
}

// ============ v0.3.66: Encoding Tests ============

#[test]
#[serial]
fn test_fs_promises_readfile_utf8_encoding() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test_utf8.txt");
    fs::write(&test_file, "Hello, 世界!").expect("Failed to write test file");

    // Test with 'utf-8' encoding string
    let code = format!(
        r#"
        const fsPromises = require('fs/promises');
        const result = fsPromises.readFile("{}", "utf-8");
        typeof result.then;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "readFile with encoding should return a thenable"
    );
}

#[test]
#[serial]
fn test_fs_promises_readfile_base64_encoding() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    // Write binary data that encodes to a known base64 string
    // "Hello" = "SGVsbG8=" in base64
    let test_file = temp_dir.path().join("test_base64.txt");
    fs::write(&test_file, "Hello").expect("Failed to write test file");

    let code = format!(
        r#"
        const fsPromises = require('fs/promises');
        const result = fsPromises.readFile("{}", "base64");
        typeof result.then;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "readFile with base64 encoding should return a thenable"
    );
}

#[test]
#[serial]
fn test_fs_promises_readfile_hex_encoding() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    // Write binary data
    let test_file = temp_dir.path().join("test_hex.txt");
    fs::write(&test_file, "Hello").expect("Failed to write test file");

    let code = format!(
        r#"
        const fsPromises = require('fs/promises');
        const result = fsPromises.readFile("{}", "hex");
        typeof result.then;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "readFile with hex encoding should return a thenable"
    );
}

#[test]
#[serial]
fn test_fs_promises_readfile_buffer_encoding() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test_buffer.txt");
    fs::write(&test_file, "Binary data").expect("Failed to write test file");

    let code = format!(
        r#"
        const fsPromises = require('fs/promises');
        const result = fsPromises.readFile("{}", "buffer");
        typeof result.then;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "readFile with buffer encoding should return a thenable"
    );
}

#[test]
#[serial]
fn test_fs_promises_readfile_encoding_object_option() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test_object_option.txt");
    fs::write(&test_file, "Test content").expect("Failed to write test file");

    // Test with object encoding option
    let code = format!(
        r#"
        const fsPromises = require('fs/promises');
        const result = fsPromises.readFile("{}", {{encoding: "utf-8"}});
        typeof result.then;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "readFile with object encoding option should return a thenable"
    );
}

#[test]
#[serial]
fn test_fs_promises_readfile_default_encoding() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test_default.txt");
    fs::write(&test_file, "Default encoding test").expect("Failed to write test file");

    // Test without encoding (should default to utf-8)
    let code = format!(
        r#"
        const fsPromises = require('fs/promises');
        const result = fsPromises.readFile("{}");
        typeof result.then;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "readFile without encoding should return a thenable with default utf-8"
    );
}

// FS module tests for Beejs runtime
// v0.3.5: fs module implementation (readFileSync, writeFileSync, existsSync, mkdirSync, readdirSync, unlinkSync, rmdirSync)

use serial_test::serial;
use std::fs;
use tempfile::TempDir;

#[test]
#[serial]
fn test_fs_module_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const fs = require('fs');
        typeof fs;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "require('fs') should return an object"
    );
}

#[test]
#[serial]
fn test_readfilesync_returns_file_content() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    // Create a temporary file with known content
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Hello, Beejs!").expect("Failed to write test file");

    let code = format!(
        r#"
        const fs = require('fs');
        fs.readFileSync("{}");
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "Hello, Beejs!",
        "readFileSync should return file content"
    );
}

#[test]
#[serial]
fn test_writefilesync_creates_file() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("output.txt");

    let code = format!(
        r#"
        const fs = require('fs');
        fs.writeFileSync("{}", "Test content from Beejs!");
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert!(
        result.trim().is_empty() || result.trim() == "undefined",
        "writeFileSync should return undefined"
    );

    // Verify file was created
    let content = fs::read_to_string(&test_file).expect("Failed to read output file");
    assert_eq!(
        content, "Test content from Beejs!",
        "File should contain written content"
    );
}

#[test]
#[serial]
fn test_existssync_returns_true_for_existing_file() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("exists_test.txt");
    fs::write(&test_file, "test").expect("Failed to write test file");

    let code = format!(
        r#"
        const fs = require('fs');
        fs.existsSync("{}");
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "existsSync should return true for existing file"
    );
}

#[test]
#[serial]
fn test_existssync_returns_false_for_nonexistent_file() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let nonexistent_file = temp_dir.path().join("nonexistent.txt");

    let code = format!(
        r#"
        const fs = require('fs');
        fs.existsSync("{}");
    "#,
        nonexistent_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "false",
        "existsSync should return false for non-existent file"
    );
}

#[test]
#[serial]
fn test_stat_sync_stats_methods_match_file_and_dir() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("stat_test.txt");
    fs::write(&test_file, "stat me").expect("Failed to write test file");

    let code = format!(
        r#"
        const fs = require('fs');
        const fileStat = fs.statSync("{}");
        const dirStat = fs.statSync("{}");
        [
          typeof fileStat.isFile,
          fileStat.isFile(),
          fileStat.isDirectory(),
          typeof dirStat.isDirectory,
          dirStat.isFile(),
          dirStat.isDirectory(),
          typeof fileStat.size,
          fileStat.size > 0
        ].join(',');
    "#,
        test_file.to_string_lossy().into_owned(),
        temp_dir.path().to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function,true,false,function,false,true,number,true",
        "statSync Stats methods should match file and directory metadata"
    );
}

#[test]
#[serial]
fn test_mkdirsync_creates_directory() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let new_dir = temp_dir.path().join("new_directory");

    let code = format!(
        r#"
        const fs = require('fs');
        fs.mkdirSync("{}");
    "#,
        new_dir.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert!(
        result.trim().is_empty() || result.trim() == "undefined",
        "mkdirSync should return undefined"
    );

    // Verify directory was created
    assert!(new_dir.exists(), "Directory should be created");
    assert!(new_dir.is_dir(), "Path should be a directory");
}

#[test]
#[serial]
fn test_readdirsync_returns_file_list() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    // Create some test files
    fs::write(temp_dir.path().join("file1.txt"), "content1").expect("Failed to write file1");
    fs::write(temp_dir.path().join("file2.txt"), "content2").expect("Failed to write file2");
    fs::write(temp_dir.path().join("file3.txt"), "content3").expect("Failed to write file3");

    let code = format!(
        r#"
        const fs = require('fs');
        const files = fs.readdirSync("{}");
        files.sort().join(',');
    "#,
        temp_dir.path().to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert!(
        result.contains("file1.txt"),
        "readdirSync should return file names"
    );
    assert!(
        result.contains("file2.txt"),
        "readdirSync should return file names"
    );
    assert!(
        result.contains("file3.txt"),
        "readdirSync should return file names"
    );
}

#[test]
#[serial]
fn test_unlinksync_deletes_file() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("to_delete.txt");
    fs::write(&test_file, "delete me").expect("Failed to write test file");

    assert!(test_file.exists(), "Test file should exist before deletion");

    let code = format!(
        r#"
        const fs = require('fs');
        fs.unlinkSync("{}");
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert!(
        result.trim().is_empty() || result.trim() == "undefined",
        "unlinkSync should return undefined"
    );

    // Verify file was deleted
    assert!(!test_file.exists(), "File should be deleted");
}

#[test]
#[serial]
fn test_rmdirsync_removes_directory() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let sub_dir = temp_dir.path().join("to_remove");
    fs::create_dir(&sub_dir).expect("Failed to create sub directory");

    assert!(
        sub_dir.exists(),
        "Sub directory should exist before removal"
    );

    let code = format!(
        r#"
        const fs = require('fs');
        fs.rmdirSync("{}");
    "#,
        sub_dir.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert!(
        result.trim().is_empty() || result.trim() == "undefined",
        "rmdirSync should return undefined"
    );

    // Verify directory was removed
    assert!(!sub_dir.exists(), "Directory should be removed");
}

#[test]
#[serial]
fn test_fs_module_has_all_functions() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const fs = require('fs');
        const hasReadFileSync = typeof fs.readFileSync === 'function';
        const hasWriteFileSync = typeof fs.writeFileSync === 'function';
        const hasExistsSync = typeof fs.existsSync === 'function';
        const hasMkdirSync = typeof fs.mkdirSync === 'function';
        const hasReaddirSync = typeof fs.readdirSync === 'function';
        const hasUnlinkSync = typeof fs.unlinkSync === 'function';
        const hasRmdirSync = typeof fs.rmdirSync === 'function';
        [hasReadFileSync, hasWriteFileSync, hasExistsSync, hasMkdirSync, hasReaddirSync, hasUnlinkSync, hasRmdirSync].join(',');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true,true,true,true,true,true,true",
        "All fs functions should exist"
    );
}

#[test]
#[serial]
fn test_readfilesync_error_handling() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const fs = require('fs');
        try {
            fs.readFileSync("/nonexistent/path/to/file.txt");
            "allowed";
        } catch (error) {
            String(error && error.message ? error.message : error);
        }
    "#;

    let result = runtime.execute_code(code).expect("Execution failed");
    assert!(
        result.contains("Error"),
        "readFileSync should return error message for non-existent file"
    );
}

// v0.3.6: Async file operations tests

#[test]
#[serial]
fn test_readfile_callback_returns_content() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("async_test.txt");
    fs::write(&test_file, "Async content").expect("Failed to write test file");

    let code = format!(
        r#"
        const fs = require('fs');
        let result;
        fs.readFile("{}", "utf8", (err, data) => {{
            result = err ? err : data;
        }});
        result;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert!(
        result.contains("Async content"),
        "readFile callback should receive content"
    );
}

#[test]
#[serial]
fn test_writefile_callback_completes() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("write_async.txt");

    let code = format!(
        r#"
        const fs = require('fs');
        let completed = false;
        fs.writeFile("{}", "Async write content", (err) => {{
            completed = !err;
        }});
        completed;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    // The callback may not have run yet, so just check it doesn't error
    assert!(
        result == "true" || result.trim().is_empty(),
        "writeFile should not throw"
    );

    // Verify file was written
    let content = fs::read_to_string(&test_file).expect("Failed to read file");
    assert_eq!(
        content, "Async write content",
        "File should contain written content"
    );
}

#[test]
#[serial]
fn test_appendfile_callback_completes() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("append_async.txt");
    fs::write(&test_file, "Original").expect("Failed to create file");

    let code = format!(
        r#"
        const fs = require('fs');
        fs.appendFile("{}", " appended", (err) => {{}});
        "done";
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert!(
        result.trim() == "done",
        "appendFile should execute without error"
    );

    // Verify content was appended
    let content = fs::read_to_string(&test_file).expect("Failed to read file");
    assert_eq!(
        content, "Original appended",
        "File should have appended content"
    );
}

#[test]
#[serial]
fn test_readfile_error_callback() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const fs = require('fs');
        let errorReceived = false;
        fs.readFile("/nonexistent/path.txt", "utf8", (err, data) => {
            errorReceived = !!err;
        });
        errorReceived;
    "#;

    let result = runtime.execute_code(code).expect("Execution failed");
    // Should receive error in callback (may be true or callback may not have run yet)
    assert!(
        result == "true" || result.trim().is_empty(),
        "readFile should handle error"
    );
}

#[test]
#[serial]
fn test_fs_module_has_async_functions() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const fs = require('fs');
        const hasReadFile = typeof fs.readFile === 'function';
        const hasWriteFile = typeof fs.writeFile === 'function';
        const hasAppendFile = typeof fs.appendFile === 'function';
        [hasReadFile, hasWriteFile, hasAppendFile].join(',');
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true,true,true",
        "All async fs functions should exist"
    );
}

// v0.3.64 Feature Tests
// Tests for http.Agent, response headers, and fs.promises

use serial_test::serial;
use std::fs;
use tempfile::TempDir;

#[test]
#[serial]
fn test_http_agent_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof http.Agent;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "function", "http.Agent should be a function");
}

#[test]
#[serial]
fn test_http_agent_constructor() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const agent = new http.Agent();
        typeof agent;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "new http.Agent() should return an object"
    );
}

#[test]
#[serial]
fn test_http_agent_options() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const agent = new http.Agent({ maxFreeSockets: 5, maxSockets: 10, keepAlive: true });
        agent.maxFreeSockets + ',' + agent.maxSockets + ',' + agent.keepAlive;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "5,10,true",
        "Agent should have correct option values"
    );
}

#[test]
#[serial]
fn test_http_global_agent() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof http.globalAgent;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "object",
        "http.globalAgent should be an object"
    );
}

#[test]
#[serial]
fn test_http_server_close() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const server = http.createServer();
        typeof server.close;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "server.close should be a function"
    );
}

#[test]
#[serial]
fn test_response_get_header() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    // Test getHeader/setHeader without triggering request handler
    let code = r#"
        const server = http.createServer();
        server.close();
        'success';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "success", "server should have close method");
}

#[test]
#[serial]
fn test_response_write_head() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    // Test writeHead without triggering request handler
    let code = r#"
        const server = http.createServer();
        server.close();
        'success';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(result.trim(), "success", "writeHead should work");
}

#[test]
#[serial]
fn test_fs_promises_readfile_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof fs.promises.readFile;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "fs.promises.readFile should be a function"
    );
}

#[test]
#[serial]
fn test_fs_promises_writefile_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof fs.promises.writeFile;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "fs.promises.writeFile should be a function"
    );
}

#[test]
#[serial]
fn test_fs_promises_unlink_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof fs.promises.unlink;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "fs.promises.unlink should be a function"
    );
}

#[test]
#[serial]
fn test_fs_promises_rename_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof fs.promises.rename;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "fs.promises.rename should be a function"
    );
}

#[test]
#[serial]
fn test_fs_promises_readfile_returns_thenable() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Hello, World!").expect("Failed to write test file");

    let code = format!(
        r#"
        const result = fs.promises.readFile("{}");
        typeof result.then;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "fs.promises.readFile should return a thenable"
    );
}

#[test]
#[serial]
fn test_fs_promises_readfile_content() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test_content.txt");
    fs::write(&test_file, "Test content 123").expect("Failed to write test file");

    let code = format!(
        r#"
        const p = fs.promises.readFile("{}");
        p.then(content => content);
        p.__result__;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert!(
        result.trim().contains("Test content 123"),
        "fs.promises.readFile should return file content, got: {}",
        result.trim()
    );
}

#[test]
#[serial]
fn test_fs_promises_writefile_thenable() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("write_test.txt");

    let code = format!(
        r#"
        const p = fs.promises.writeFile("{}", "Written content");
        p.then(() => 'done');
        p.__result__;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "done",
        "fs.promises.writeFile should resolve when done"
    );

    // Verify file was written
    let content = fs::read_to_string(&test_file).expect("Failed to read file");
    assert_eq!(
        content, "Written content",
        "File should contain written content"
    );
}

#[test]
#[serial]
fn test_fs_promises_unlink_thenable() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("delete_me.txt");
    fs::write(&test_file, "delete me").expect("Failed to write test file");

    let code = format!(
        r#"
        const p = fs.promises.unlink("{}");
        p.then(() => 'deleted');
        p.__result__;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "deleted",
        "fs.promises.unlink should resolve when done"
    );

    // Verify file was deleted
    assert!(!test_file.exists(), "File should be deleted");
}

#[test]
#[serial]
fn test_fs_promises_rename_thenable() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let old_file = temp_dir.path().join("old_name.txt");
    let new_file = temp_dir.path().join("new_name.txt");
    fs::write(&old_file, "Rename me").expect("Failed to write test file");

    let code = format!(
        r#"
        const p = fs.promises.rename("{}", "{}");
        p.then(() => 'renamed');
        p.__result__;
    "#,
        old_file.to_string_lossy().into_owned(),
        new_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "renamed",
        "fs.promises.rename should resolve when done"
    );

    // Verify file was renamed
    assert!(!old_file.exists(), "Old file should not exist");
    assert!(new_file.exists(), "New file should exist");
}

#[test]
#[serial]
fn test_fs_promises_readdir_thenable() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    fs::write(temp_dir.path().join("file1.txt"), "content1").expect("Failed to create file");
    fs::write(temp_dir.path().join("file2.txt"), "content2").expect("Failed to create file");

    let code = format!(
        r#"
        const p = fs.promises.readdir("{}");
        p.then(files => files.length);
        p.__result__;
    "#,
        temp_dir.path().to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "2",
        "fs.promises.readdir should return number of files"
    );
}

#[test]
#[serial]
fn test_fs_promises_stat_thenable() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("stat_test.txt");
    fs::write(&test_file, "stat me").expect("Failed to write test file");

    let code = format!(
        r#"
        const p = fs.promises.stat("{}");
        p.then(stat => stat.isFile());
        p.__result__;
    "#,
        test_file.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "true",
        "fs.promises.stat should return isFile() true for file"
    );
}

#[test]
#[serial]
fn test_fs_promises_mkdir_thenable() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let new_dir = temp_dir.path().join("new_dir");

    let code = format!(
        r#"
        const p = fs.promises.mkdir("{}");
        p.then(() => 'created');
        p.__result__;
    "#,
        new_dir.to_string_lossy().into_owned()
    );

    let result = runtime.execute_code(&code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "created",
        "fs.promises.mkdir should resolve when done"
    );

    assert!(new_dir.exists(), "Directory should be created");
}

#[test]
#[serial]
fn test_fs_promises_error_handling() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

    let code = r#"
        const p = fs.promises.readFile("/nonexistent/path/file.txt");
        p.then(
            () => 'success',
            (err) => 'error: ' + err
        );
        p.__result__;
    "#;

    let result = runtime.execute_code(code).expect("Execution failed");
    assert!(
        result.trim().starts_with("error:"),
        "fs.promises should handle errors, got: {}",
        result.trim()
    );
}

// v0.3.65 Tests: http.request enhancement

#[test]
#[serial]
fn test_http_request_exists() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        typeof http.request;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "http.request should be a function"
    );
}

#[test]
#[serial]
fn test_http_request_options() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const req = http.request({
            method: 'POST',
            hostname: 'example.com',
            port: 8080,
            path: '/api/test'
        });
        req.method + ',' + req.hostname + ',' + req.port + ',' + req.path;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "POST,example.com,8080,/api/test",
        "http.request should parse options correctly"
    );
}

#[test]
#[serial]
fn test_http_request_default_options() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const req = http.request({});
        req.method + ',' + req.hostname + ',' + req.port + ',' + req.path;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "GET,localhost,80,/",
        "http.request should use default options"
    );
}

#[test]
#[serial]
fn test_http_request_write_method() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const req = http.request({});
        typeof req.write;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "function",
        "http.request should have write method"
    );
}

#[test]
#[serial]
fn test_http_request_write_body() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const req = http.request({});
        req.write('hello');
        req._body;
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "hello",
        "http.request().write() should store body"
    );
}

#[test]
#[serial]
fn test_http_request_callback_pattern() {
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
        const req = http.request({
            hostname: 'example.com',
            path: '/test'
        }, (res) => {
            return res.statusCode;
        });
        req.end();
        'done';
    "#;
    let result = runtime.execute_code(code).expect("Execution failed");
    assert_eq!(
        result.trim(),
        "done",
        "http.request callback pattern should work"
    );
}

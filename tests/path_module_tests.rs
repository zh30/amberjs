// Tests for path module (v0.3.32)
// Tests for path.join, path.dirname, path.basename, path.extname, path.resolve
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_path_module_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof require('path')");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_path_join_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof require('path').join");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_path_join_single_arg() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').join("foo")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "foo");
}

#[test]
#[serial]
fn test_path_join_multiple_args() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').join("foo", "bar", "baz")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "foo/bar/baz");
}

#[test]
#[serial]
fn test_path_join_with_slashes() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').join("foo/bar", "baz/qux")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "foo/bar/baz/qux");
}

#[test]
#[serial]
fn test_path_join_empty_args() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').join("")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    // Node normalizes an all-empty join to the current directory.
    assert_eq!(result.unwrap().trim(), ".");
}

#[test]
#[serial]
fn test_path_join_no_args() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').join()"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), ".");
}

#[test]
#[serial]
fn test_path_dirname_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof require('path').dirname");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_path_dirname_basic() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').dirname("/foo/bar/baz")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "/foo/bar");
}

#[test]
#[serial]
fn test_path_dirname_file() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').dirname("file.txt")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    // Result should not be "file.txt"
    let binding = result.unwrap();
    let output = binding.trim();
    assert_ne!(output, "file.txt");
}

#[test]
#[serial]
fn test_path_dirname_root() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').dirname("/")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "/");
}

#[test]
#[serial]
fn test_path_dirname_empty() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').dirname("")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), ".");
}

#[test]
#[serial]
fn test_path_basename_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof require('path').basename");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_path_basename_basic() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').basename("/foo/bar/baz.txt")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "baz.txt");
}

#[test]
#[serial]
fn test_path_basename_no_ext() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').basename("/foo/bar/baz", ".txt")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "baz");
}

#[test]
#[serial]
fn test_path_basename_root() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').basename("/")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    // The root has no trailing component.
    assert_eq!(result.unwrap().trim(), "");
}

#[test]
#[serial]
fn test_path_extname_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof require('path').extname");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_path_extname_basic() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').extname("file.txt")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), ".txt");
}

#[test]
#[serial]
fn test_path_extname_no_ext() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').extname("file")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "");
}

#[test]
#[serial]
fn test_path_extname_multiple_dots() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').extname("file.min.js")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), ".js");
}

#[test]
#[serial]
fn test_path_extname_dotfile() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').extname(".gitignore")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    // .gitignore has no extension (hidden files return empty string)
    assert_eq!(result.unwrap().trim(), "");
}

#[test]
#[serial]
fn test_path_resolve_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof require('path').resolve");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_path_resolve_no_args() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"typeof require('path').resolve()"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    // Should return string type
    assert_eq!(result.unwrap().trim(), "string");
}

#[test]
#[serial]
fn test_path_resolve_single_arg() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').resolve("test.txt")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // Should include test.txt
    assert!(output.ends_with("test.txt") || output.contains("test.txt"));
}

#[test]
#[serial]
fn test_path_resolve_multiple_args() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').resolve("foo", "bar", "baz")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // Should include all parts
    assert!(output.ends_with("foo/bar/baz") || output.ends_with("baz"));
}

#[test]
#[serial]
fn test_path_resolve_absolute_last() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').resolve("/absolute", "path")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // Should be /absolute/path
    assert!(output.starts_with("/absolute"));
}

#[test]
#[serial]
fn test_path_resolve_parent_dir() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').resolve("/a/b", "../c")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // Should resolve .. correctly
    assert!(output.contains("/c"));
}

#[test]
#[serial]
fn test_path_resolve_current_dir() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"require('path').resolve(".", "file")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // Should handle . correctly
    assert!(output.ends_with("file"));
}

#[test]
#[serial]
fn test_path_sep_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("require('path').sep");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "/");
}

#[test]
#[serial]
fn test_path_module_all_functions() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        (function() {
            const path = require('path');
            // Check that all functions exist and are functions
            return typeof path.join === 'function' &&
                   typeof path.dirname === 'function' &&
                   typeof path.basename === 'function' &&
                   typeof path.extname === 'function' &&
                   typeof path.resolve === 'function' &&
                   path.sep === '/';
        })()
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

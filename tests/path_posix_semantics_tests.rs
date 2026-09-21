//! Exact POSIX semantics for the path module.
//!
//! `path_module_tests.rs` asserts loosely (`ends_with(..) || ends_with(..)`),
//! which let real divergences pass: `resolve` appended its segments in reverse,
//! `normalize` moved the leading separator to the end of absolute paths, and
//! `extname` treated a dotfile's leading dot as an extension. Every expectation
//! below is the value Node reports on POSIX.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn eval(code: &str) -> String {
    let mut runtime = MinimalRuntime::new().unwrap();
    runtime
        .execute_code(code)
        .unwrap_or_else(|error| panic!("{code} should evaluate: {error}"))
        .trim()
        .to_string()
}

fn cwd() -> String {
    std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string()
}

#[test]
#[serial]
fn resolve_keeps_left_to_right_segment_order() {
    assert_eq!(
        eval(r#"require('path').resolve("foo", "bar", "baz")"#),
        format!("{}/foo/bar/baz", cwd())
    );
}

#[test]
#[serial]
fn resolve_preserves_leading_separator_of_absolute_segment() {
    assert_eq!(
        eval(r#"require('path').resolve("/absolute", "path")"#),
        "/absolute/path"
    );
}

#[test]
#[serial]
fn resolve_restarts_at_the_rightmost_absolute_segment() {
    assert_eq!(
        eval(r#"require('path').resolve("/first", "/second", "tail")"#),
        "/second/tail"
    );
}

#[test]
#[serial]
fn resolve_collapses_parent_segments() {
    assert_eq!(eval(r#"require('path').resolve("/a/b", "../c")"#), "/a/c");
    assert_eq!(eval(r#"require('path').resolve("/a/b/c", "../..")"#), "/a");
}

#[test]
#[serial]
fn resolve_cannot_climb_above_the_root() {
    assert_eq!(eval(r#"require('path').resolve("/a", "../../..")"#), "/");
}

#[test]
#[serial]
fn resolve_anchors_relative_segments_to_the_cwd() {
    assert_eq!(
        eval(r#"require('path').resolve(".", "file")"#),
        format!("{}/file", cwd())
    );
    assert_eq!(eval(r#"require('path').resolve()"#), cwd());
}

#[test]
#[serial]
fn resolve_drops_trailing_separators() {
    assert_eq!(
        eval(r#"require('path').resolve("/a/b/")"#),
        "/a/b",
        "resolve never reports a trailing separator"
    );
}

#[test]
#[serial]
fn resolve_ignores_empty_segments() {
    assert_eq!(
        eval(r#"require('path').resolve("/a", "", "b")"#),
        "/a/b",
        "an empty segment must not reset resolution"
    );
}

#[test]
#[serial]
fn normalize_keeps_the_root_in_front() {
    assert_eq!(eval(r#"require('path').normalize("/a/b/../c")"#), "/a/c");
    assert_eq!(eval(r#"require('path').normalize("/a//b/./c")"#), "/a/b/c");
    assert_eq!(eval(r#"require('path').normalize("/")"#), "/");
    assert_eq!(eval(r#"require('path').normalize("/..")"#), "/");
}

#[test]
#[serial]
fn normalize_keeps_leading_parents_of_relative_paths() {
    assert_eq!(eval(r#"require('path').normalize("../a")"#), "../a");
    assert_eq!(eval(r#"require('path').normalize("a/../../b")"#), "../b");
}

#[test]
#[serial]
fn normalize_preserves_a_trailing_separator() {
    assert_eq!(eval(r#"require('path').normalize("/a/b/")"#), "/a/b/");
    assert_eq!(eval(r#"require('path').normalize("")"#), ".");
}

#[test]
#[serial]
fn join_keeps_the_root_in_front() {
    assert_eq!(eval(r#"require('path').join("/a", "b")"#), "/a/b");
    assert_eq!(eval(r#"require('path').join("/a", "..", "b")"#), "/b");
}

#[test]
#[serial]
fn extname_treats_a_leading_dot_as_a_dotfile() {
    assert_eq!(eval(r#"require('path').extname(".gitignore")"#), "");
    assert_eq!(eval(r#"require('path').extname("/etc/.bashrc")"#), "");
    assert_eq!(eval(r#"require('path').extname("..")"#), "");
    assert_eq!(eval(r#"require('path').extname(".")"#), "");
}

#[test]
#[serial]
fn extname_reads_the_last_dot_of_the_basename() {
    assert_eq!(eval(r#"require('path').extname("file.min.js")"#), ".js");
    assert_eq!(eval(r#"require('path').extname("..a")"#), ".a");
    assert_eq!(eval(r#"require('path').extname("a.")"#), ".");
    assert_eq!(
        eval(r#"require('path').extname("/a.b/file")"#),
        "",
        "a dot in a parent directory is not the basename's extension"
    );
}

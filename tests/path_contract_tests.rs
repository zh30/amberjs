//! Pins docs/PATH_CONTRACT.md — POSIX Node `path` Stable surface.
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn eval(code: &str) -> String {
    let mut runtime = MinimalRuntime::new().expect("runtime");
    runtime
        .execute_code(code)
        .unwrap_or_else(|error| panic!("{code} should evaluate: {error}"))
        .trim()
        .to_string()
}

fn cwd() -> String {
    std::env::current_dir()
        .expect("cwd")
        .to_string_lossy()
        .to_string()
}

#[test]
#[serial]
fn require_and_esm_reach_the_same_posix_surface() {
    assert_eq!(
        eval(
            r#"
            const path = require('path');
            const nodePath = require('node:path');
            [
              path === nodePath,
              path.sep,
              path.delimiter,
              typeof path.join,
              typeof path.resolve,
              typeof path.normalize,
              typeof path.dirname,
              typeof path.basename,
              typeof path.extname,
              typeof path.relative,
              typeof path.isAbsolute,
              typeof path.parse,
              typeof path.format,
              typeof path.posix,
              typeof path.win32
            ].join('|');
            "#
        ),
        "true|/|:|function|function|function|function|function|function|function|function|function|function|object|object"
    );
}

#[test]
#[serial]
fn join_normalize_dirname_basename_extname() {
    assert_eq!(
        eval(
            r#"
            const path = require('path');
            [
              path.join('a', 'b', 'c'),
              path.join('/a', 'b'),
              path.join('/a', '..', 'b'),
              path.join(),
              path.join(''),
              path.normalize('/a/b/../c'),
              path.normalize('/a//b/./c'),
              path.normalize('/'),
              path.normalize('/..'),
              path.normalize('../a'),
              path.normalize('a/../../b'),
              path.normalize('/a/b/'),
              path.normalize(''),
              path.dirname('/foo/bar/baz'),
              path.dirname('/'),
              path.dirname('file.txt'),
              path.dirname(''),
              path.basename('/foo/bar/baz.txt'),
              path.basename('/foo/bar/baz', '.txt'),
              path.basename('/'),
              path.extname('file.min.js'),
              path.extname('.gitignore'),
              path.extname('/etc/.bashrc'),
              path.extname('..'),
              path.extname('..a'),
              path.extname('a.'),
              path.extname('/a.b/file')
            ].join('|');
            "#
        ),
        "a/b/c|/a/b|/b|.|.|/a/c|/a/b/c|/|/|../a|../b|/a/b/|.|/foo/bar|/|.|.|baz.txt|baz||.js||||.a|.|"
    );
}

#[test]
#[serial]
fn resolve_relative_is_absolute_parse_format() {
    let cwd = cwd();
    assert_eq!(
        eval(
            r#"
            const path = require('path');
            const parsed = path.parse('/home/user/dir/file.txt');
            [
              path.resolve('foo', 'bar', 'baz'),
              path.resolve('/absolute', 'path'),
              path.resolve('/first', '/second', 'tail'),
              path.resolve('/a/b', '../c'),
              path.resolve('/a/b/c', '../..'),
              path.resolve('/a', '../../..'),
              path.resolve('.', 'file'),
              path.resolve(),
              path.resolve('/a/b/'),
              path.resolve('/a', '', 'b'),
              path.relative('/data/orandea/test/aaa', '/data/orandea/impl/bbb'),
              path.relative('/same', '/same'),
              path.isAbsolute('/foo/bar'),
              path.isAbsolute('qux/'),
              parsed.root,
              parsed.dir,
              parsed.base,
              parsed.ext,
              parsed.name,
              path.format(parsed)
            ].join('|');
            "#
        ),
        format!(
            "{cwd}/foo/bar/baz|/absolute/path|/second/tail|/a/c|/a|/|{cwd}/file|{cwd}|/a/b|/a/b|../../impl/bbb|.|true|false|/|/home/user/dir|file.txt|.txt|file|/home/user/dir/file.txt"
        )
    );
}

#[test]
#[serial]
fn posix_matches_path_and_win32_only_changes_sep_delimiter() {
    assert_eq!(
        eval(
            r#"
            const path = require('path');
            JSON.stringify({
              posixSep: path.posix.sep,
              posixDelimiter: path.posix.delimiter,
              posixJoinSame: path.posix.join === path.join,
              posixResolveSame: path.posix.resolve === path.resolve,
              posixNormalizeSame: path.posix.normalize === path.normalize,
              win32Sep: path.win32.sep,
              win32Delimiter: path.win32.delimiter,
              win32JoinSame: path.win32.join === path.join,
              win32ResolveSame: path.win32.resolve === path.resolve,
              win32NormalizeSame: path.win32.normalize === path.normalize,
              win32Join: path.win32.join('a', 'b'),
              win32AbsUnix: path.win32.isAbsolute('/foo'),
              win32AbsDrive: path.win32.isAbsolute('C:\\foo')
            });
            "#
        ),
        r#"{"posixSep":"/","posixDelimiter":":","posixJoinSame":true,"posixResolveSame":true,"posixNormalizeSame":true,"win32Sep":"\\","win32Delimiter":";","win32JoinSame":true,"win32ResolveSame":true,"win32NormalizeSame":true,"win32Join":"a/b","win32AbsUnix":true,"win32AbsDrive":false}"#
    );
}

#[test]
#[serial]
fn parse_leading_dot_basename_sets_name_and_ext_unlike_extname() {
    // Amber parse: when the only `.` opens the basename, ext equals base, so
    // name falls back to base too. extname still treats that as a dotfile.
    assert_eq!(
        eval(
            r#"
            const path = require('path');
            const p = path.parse('/a/.gitignore');
            [path.extname('/a/.gitignore'), p.ext, p.name, p.base].join('|');
            "#
        ),
        "|.gitignore|.gitignore|.gitignore"
    );
}

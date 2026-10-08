//! Pins docs/NODE_URL_CONTRACT.md — Node `require('url')` file-URL Stable subset.
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
fn require_and_node_prefix_share_file_helpers_and_g10_ctors() {
    assert_eq!(
        eval(
            r#"
            const url = require('url');
            const nodeUrl = require('node:url');
            [
              typeof url.fileURLToPath,
              typeof url.pathToFileURL,
              typeof url.URL,
              typeof url.URLSearchParams,
              url.URL === globalThis.URL,
              url.URLSearchParams === globalThis.URLSearchParams,
              url.fileURLToPath === nodeUrl.fileURLToPath,
              url.pathToFileURL === nodeUrl.pathToFileURL
            ].join('|');
            "#
        ),
        "function|function|function|function|true|true|true|true"
    );
}

#[test]
#[serial]
fn path_to_file_url_returns_url_instance_with_encoded_href() {
    assert_eq!(
        eval(
            r#"
            const { pathToFileURL } = require('url');
            const u = pathToFileURL('/tmp/foo bar');
            const hash = pathToFileURL('/tmp/foo#bar');
            const q = pathToFileURL('/tmp/foo?x=1');
            [
              u instanceof URL,
              u.href,
              hash.href,
              q.href,
              u.protocol
            ].join('|');
            "#
        ),
        "true|file:///tmp/foo%20bar|file:///tmp/foo%23bar|file:///tmp/foo%3Fx=1|file:"
    );
}

#[test]
#[serial]
fn path_to_file_url_resolves_relative_against_cwd() {
    let cwd = cwd();
    let expected_href = format!(
        "file://{}",
        amberjs::nodejs_core::url::encode_file_url_path(&format!(
            "{}/rel-file-url-probe",
            cwd.trim_end_matches('/')
        ))
    );
    assert_eq!(
        eval(
            r#"
            const { pathToFileURL } = require('url');
            pathToFileURL('rel-file-url-probe').href;
            "#
        ),
        expected_href
    );
}

#[test]
#[serial]
fn file_url_to_path_decodes_and_accepts_url_instance() {
    assert_eq!(
        eval(
            r#"
            const { fileURLToPath, pathToFileURL } = require('url');
            const fromString = fileURLToPath('file:///tmp/foo%20bar');
            const fromUrl = fileURLToPath(pathToFileURL('/tmp/café'));
            const localhost = fileURLToPath('file://localhost/tmp/x');
            const root = fileURLToPath('file:///');
            [
              fromString,
              fromUrl,
              localhost,
              root
            ].join('|');
            "#
        ),
        "/tmp/foo bar|/tmp/café|/tmp/x|/"
    );
}

#[test]
#[serial]
fn file_url_helpers_throw_coded_errors() {
    assert_eq!(
        eval(
            r#"
            const { fileURLToPath, pathToFileURL } = require('url');
            function code(fn) {
              try { fn(); return 'none'; }
              catch (e) { return (e && e.code) || e.name || 'err'; }
            }
            [
              code(() => fileURLToPath('http://example.com')),
              code(() => fileURLToPath('file://example.com/tmp')),
              code(() => fileURLToPath('not-a-url')),
              code(() => fileURLToPath({ href: 'file:///tmp/x' })),
              code(() => pathToFileURL(null)),
              code(() => pathToFileURL(123))
            ].join('|');
            "#
        ),
        "ERR_INVALID_URL_SCHEME|ERR_INVALID_FILE_URL_HOST|ERR_INVALID_URL|ERR_INVALID_ARG_TYPE|ERR_INVALID_ARG_TYPE|ERR_INVALID_ARG_TYPE"
    );
}

#[test]
#[serial]
fn legacy_parse_format_resolve_are_not_on_require_surface() {
    assert_eq!(
        eval(
            r#"
            const url = require('url');
            [
              typeof url.parse,
              typeof url.format,
              typeof url.resolve
            ].join('|');
            "#
        ),
        "undefined|undefined|undefined"
    );
}

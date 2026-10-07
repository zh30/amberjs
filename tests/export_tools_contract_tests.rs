// Pins the Stable `amber run --export-tools` contract in docs/EXPORT_TOOLS_CONTRACT.md.

use amberjs::agent::EXPORT_TOOLS_ERROR_PREFIX;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn amber_path() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

fn combined(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn run_amber(dir: &Path, args: &[&str]) -> std::process::Output {
    let mut child = Command::new(amber_path())
        .current_dir(dir)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn amber");
    let started = Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(20) {
            let _ = child.kill();
            let output = child.wait_with_output().expect("wait after kill");
            panic!(
                "amber timed out after 20s. args: {args:?} output: {}",
                combined(&output)
            );
        }
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().expect("wait amber"),
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(err) => panic!("try_wait failed: {err}"),
        }
    }
}

fn assert_failure(output: &std::process::Output, needles: &[&str]) {
    let text = combined(output);
    assert!(
        !output.status.success(),
        "expected export-tools to fail. output: {text}"
    );
    assert!(
        text.contains(EXPORT_TOOLS_ERROR_PREFIX),
        "failure must use {EXPORT_TOOLS_ERROR_PREFIX}. output: {text}"
    );
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("\"tools\""),
        "failure must not print a tools array. output: {text}"
    );
    for needle in needles {
        assert!(
            text.contains(needle),
            "failure must contain {needle:?}. output: {text}"
        );
    }
}

fn assert_tools(output: &std::process::Output, names: &[&str]) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "expected export-tools to succeed. stdout: {stdout} stderr: {stderr}"
    );
    assert!(
        !stderr.contains(EXPORT_TOOLS_ERROR_PREFIX),
        "success must not print {EXPORT_TOOLS_ERROR_PREFIX}. stderr: {stderr}"
    );
    assert!(
        stdout.ends_with('\n'),
        "success stdout must end with a newline. stdout: {stdout}"
    );
    let value: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    let pretty = serde_json::to_string_pretty(&value).expect("pretty");
    assert_eq!(
        stdout,
        format!("{pretty}\n"),
        "stdout must be pretty-printed JSON"
    );
    let tools = value["tools"].as_array().expect("tools array");
    let got: Vec<&str> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(got, names, "tool names. stdout: {stdout}");
    value
}

fn write(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(&path, body).expect("write fixture");
    path
}

#[test]
fn example_echo_manifest_schema() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = run_amber(
        root,
        &[
            "run",
            "--sandbox",
            "--export-tools",
            "examples/agent/echo_tool.ts",
        ],
    );
    let value = assert_tools(&output, &["echo"]);
    assert_eq!(
        value["tools"][0]["description"],
        "Echo the input text back to the host"
    );
    assert_eq!(value["tools"][0]["inputSchema"]["type"], "object");
    assert_eq!(
        value["tools"][0]["inputSchema"]["properties"]["text"]["type"],
        "string"
    );
}

#[test]
fn manifest_replaces_source_and_omitted_fields_default() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "tool.js",
        "export function fromSource() { return 1; }\n",
    );
    write(
        dir.path(),
        "tools.json",
        r#"{"tools":[{"name":"echo","extra":true}],"ignored":1}"#,
    );
    let output = run_amber(
        dir.path(),
        &["run", "--export-tools", "tool.js", "ignored-arg"],
    );
    let value = assert_tools(&output, &["echo"]);
    assert_eq!(value["tools"][0]["description"], "");
    assert_eq!(
        value["tools"][0]["inputSchema"],
        serde_json::json!({"type": "object"})
    );
}

#[test]
fn source_scan_jsdoc_types_and_comment_rules() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "tool.ts",
        "/**\r\n * Add numbers\r\n * @param {number} a\r\n */\r\nexport function add(a: number) {\r\n  return a;\r\n}\r\n\r\n// Echo text\r\n\r\nexport async function echo(input: { text?: string }) {\r\n  return input;\r\n}\r\n",
    );
    let output = run_amber(dir.path(), &["run", "--export-tools", "tool.ts"]);
    let value = assert_tools(&output, &["add", "echo"]);
    assert_eq!(value["tools"][0]["description"], "Add numbers");
    assert_eq!(
        value["tools"][0]["inputSchema"],
        serde_json::json!({"type": "object"})
    );
    assert!(value["tools"][0]["inputSchema"].get("properties").is_none());
    assert_eq!(value["tools"][1]["description"], "Echo text");
    assert_eq!(
        value["tools"][1]["inputSchema"],
        serde_json::json!({"type": "object"})
    );
}

#[test]
fn source_scan_skips_unrecognized_exports_and_block_comments() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "tool.js",
        r#"
// export function commentedLine() { return 1; }
export const hidden = () => 1;
export default function hiddenDefault() { return 1; }
/*
export function hiddenBlock() { return 1; }
*/
/**
 * First
 */
export function shown() { return 1; }
/**
 * Second
 */
export function shown() { return 2; }
export async function alsoShown() { return 3; }
"#,
    );
    let output = run_amber(dir.path(), &["run", "--export-tools", "tool.js"]);
    let value = assert_tools(&output, &["shown", "alsoShown"]);
    assert_eq!(value["tools"][0]["description"], "First");
    assert_eq!(
        value["tools"][1]["description"],
        "Exported function `alsoShown`"
    );
}

#[test]
fn does_not_execute_the_module() {
    let dir = tempfile::tempdir().expect("tempdir");
    let marker = dir.path().join("side_effect.txt");
    let marker_text = marker.display().to_string().replace('\\', "\\\\");
    write(
        dir.path(),
        "tool.js",
        &format!(
            "const fs = require('fs');\nfs.writeFileSync('{marker_text}', 'ran');\nconsole.log('EXECUTED');\nexport function echo(args) {{ return args; }}\n"
        ),
    );
    let output = run_amber(dir.path(), &["run", "--export-tools", "tool.js"]);
    assert_tools(&output, &["echo"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("EXECUTED"));
    assert!(!marker.exists(), "export-tools must not run the module");
}

#[test]
fn sandbox_allows_entry_and_manifest_and_audits_reads_only() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "tool.js",
        "export function echo() { return 1; }\n",
    );
    write(
        dir.path(),
        "tools.json",
        r#"{"tools":[{"name":"echo","description":"from manifest"}]}"#,
    );
    write(
        dir.path(),
        "policy.json",
        r#"{"permissions":{"deny_fs":true,"allow_read":["other.txt"]}}"#,
    );
    let output = run_amber(
        dir.path(),
        &[
            "run",
            "--sandbox",
            "--permission-policy",
            "policy.json",
            "--audit-log",
            "audit.jsonl",
            "--export-tools",
            "tool.js",
        ],
    );
    let value = assert_tools(&output, &["echo"]);
    assert_eq!(value["tools"][0]["description"], "from manifest");
    let audit = fs::read_to_string(dir.path().join("audit.jsonl")).expect("audit log");
    let lines: Vec<&str> = audit.lines().filter(|line| !line.is_empty()).collect();
    assert_eq!(lines.len(), 2, "audit log: {audit}");
    assert!(audit.contains("tool.js"), "audit log: {audit}");
    assert!(audit.contains("tools.json"), "audit log: {audit}");
    assert!(
        audit.contains("\"decision\":\"Allow\""),
        "audit log: {audit}"
    );
    assert!(!audit.contains("tool:"), "audit log: {audit}");
    assert!(
        !audit.contains("\"decision\":\"Deny\""),
        "audit log: {audit}"
    );
}

#[test]
fn deny_fs_without_sandbox_is_permission_denied() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "tool.js",
        "export function echo() { return 1; }\n",
    );
    let output = run_amber(
        dir.path(),
        &["run", "--deny-fs", "--export-tools", "tool.js"],
    );
    assert_failure(&output, &["permission denied"]);
}

#[test]
fn policy_allow_read_without_sandbox() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "tool.js",
        "export function echo() { return 1; }\n",
    );
    write(
        dir.path(),
        "deny.json",
        r#"{"permissions":{"deny_fs":true}}"#,
    );
    let denied = run_amber(
        dir.path(),
        &[
            "run",
            "--permission-policy",
            "deny.json",
            "--export-tools",
            "tool.js",
        ],
    );
    assert_failure(&denied, &["permission denied"]);

    write(
        dir.path(),
        "allow.json",
        r#"{"permissions":{"deny_fs":true,"allow_read":["tool.js"]}}"#,
    );
    let allowed = run_amber(
        dir.path(),
        &[
            "run",
            "--permission-policy",
            "allow.json",
            "--export-tools",
            "tool.js",
        ],
    );
    assert_tools(&allowed, &["echo"]);
}

#[test]
fn manifest_is_next_to_the_entry_not_the_working_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "tools.json",
        r#"{"tools":[{"name":"fromCwd"}]}"#,
    );
    write(
        dir.path(),
        "sub/tool.js",
        "export function fromSource() { return 1; }\n",
    );
    write(
        dir.path(),
        "sub/tools.json",
        r#"{"tools":[{"name":"fromSubdir","description":"beside entry"}]}"#,
    );
    let output = run_amber(dir.path(), &["run", "--export-tools", "sub/tool.js"]);
    let value = assert_tools(&output, &["fromSubdir"]);
    assert_eq!(value["tools"][0]["description"], "beside entry");
}

#[test]
fn directory_named_tools_json_is_not_a_manifest() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::create_dir(dir.path().join("tools.json")).expect("mkdir tools.json");
    write(
        dir.path(),
        "tool.js",
        "export function fromSource() { return 1; }\n",
    );
    let output = run_amber(dir.path(), &["run", "--export-tools", "tool.js"]);
    assert_tools(&output, &["fromSource"]);
}

#[test]
fn ignored_flags_do_not_execute_or_hang() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "tool.js",
        "export function echo() { return 1; }\n",
    );
    write(
        dir.path(),
        "preload.js",
        "throw new Error('PRELOAD_RAN');\n",
    );
    let output = run_amber(
        dir.path(),
        &[
            "run",
            "--watch",
            "--inspect-brk",
            "--preload",
            "preload.js",
            "--export-tools",
            "tool.js",
        ],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_tools(&output, &["echo"]);
    assert!(!stdout.contains("PRELOAD_RAN"));
    assert!(!stdout.contains("Watch mode"));
}

#[test]
fn missing_entry_does_not_run_package_script() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "package.json",
        r#"{"name":"fixture","scripts":{"build":"echo SHOULD_NOT_RUN"}}"#,
    );
    let output = run_amber(dir.path(), &["run", "--export-tools", "build"]);
    assert_failure(&output, &["entry file not found:"]);
    assert!(!combined(&output).contains("SHOULD_NOT_RUN"));
}

#[test]
fn directory_entry_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::create_dir(dir.path().join("subdir")).expect("mkdir");
    let output = run_amber(dir.path(), &["run", "--export-tools", "subdir"]);
    assert_failure(&output, &["entry must be a file:"]);
}

#[test]
fn manifest_failures() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "tool.js",
        "export function echo() { return 1; }\n",
    );

    write(dir.path(), "tools.json", "not-json");
    assert_failure(
        &run_amber(dir.path(), &["run", "--export-tools", "tool.js"]),
        &["Failed to parse tools manifest"],
    );

    write(dir.path(), "tools.json", r#"{"tools":[]}"#);
    assert_failure(
        &run_amber(dir.path(), &["run", "--export-tools", "tool.js"]),
        &["empty tools array"],
    );

    write(dir.path(), "tools.json", r#"{"tools":[{"name":""}]}"#);
    assert_failure(
        &run_amber(dir.path(), &["run", "--export-tools", "tool.js"]),
        &["tool name must be non-empty"],
    );

    write(
        dir.path(),
        "tools.json",
        r#"{"tools":[{"name":"echo"},{"name":"echo"}]}"#,
    );
    assert_failure(
        &run_amber(dir.path(), &["run", "--export-tools", "tool.js"]),
        &["duplicate tool name"],
    );

    write(
        dir.path(),
        "tools.json",
        r#"{"tools":[{"name":"echo","inputSchema":[]}]}"#,
    );
    assert_failure(
        &run_amber(dir.path(), &["run", "--export-tools", "tool.js"]),
        &["must be a JSON object"],
    );
}

#[test]
fn no_exports_without_manifest() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "tool.js", "const value = 1;\n");
    let output = run_amber(dir.path(), &["run", "--export-tools", "tool.js"]);
    assert_failure(&output, &["no exported functions found"]);
}

#[test]
fn policy_file_failures() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "tool.js",
        "export function echo() { return 1; }\n",
    );
    let missing = run_amber(
        dir.path(),
        &[
            "run",
            "--permission-policy",
            "missing-policy.json",
            "--export-tools",
            "tool.js",
        ],
    );
    assert_failure(&missing, &["Failed to read permission policy"]);

    write(dir.path(), "policy.json", "not-json");
    let invalid = run_amber(
        dir.path(),
        &[
            "run",
            "--permission-policy",
            "policy.json",
            "--export-tools",
            "tool.js",
        ],
    );
    assert_failure(&invalid, &["Failed to parse permission policy"]);
}

#[test]
fn clap_missing_file_is_not_the_contract_prefix() {
    let output = Command::new(amber_path())
        .args(["run", "--export-tools"])
        .output()
        .expect("spawn amber");
    let text = combined(&output);
    assert!(!output.status.success(), "clap should fail. output: {text}");
    assert!(
        !text.contains(EXPORT_TOOLS_ERROR_PREFIX),
        "clap usage must not use the contract prefix. output: {text}"
    );
}

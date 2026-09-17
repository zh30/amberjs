use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn bee_path() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

fn js_string(value: &Path) -> String {
    serde_json::to_string(&value.to_string_lossy().to_string()).expect("path should encode as JSON")
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("failed to read JSON fixture"))
        .expect("JSON fixture should be valid")
}

#[test]
fn create_help_does_not_panic() {
    let output = Command::new(bee_path())
        .args(["create", "--help"])
        .output()
        .expect("failed to execute bee create --help");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "bee create --help should exit successfully. stderr: {stderr}"
    );
    assert!(
        !stderr.to_lowercase().contains("panic"),
        "bee create --help should not panic. stderr: {stderr}"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Usage:"),
        "help output should include a usage line. stdout: {stdout}"
    );
}

#[test]
fn create_accepts_legacy_template_first_order() {
    let dir = tempdir().expect("failed to create tempdir");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .args(["create", "ts", "my-ts-app"])
        .output()
        .expect("failed to execute bee create");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee create should accept the legacy template-first order. output: {combined}"
    );
    assert!(
        dir.path().join("my-ts-app/index.ts").is_file(),
        "legacy order should create a TypeScript project named my-ts-app. output: {combined}"
    );
    assert!(
        !dir.path().join("ts").exists(),
        "legacy order must not create a project directory named after the template. output: {combined}"
    );
}

#[test]
fn init_deny_fs_blocks_project_creation() {
    let dir = tempdir().expect("failed to create tempdir");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .args(["init", "--deny-fs", "blocked-app"])
        .output()
        .expect("failed to execute bee init");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied project directory write should fail bee init. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("FileSystem")
            && combined.contains("Write"),
        "bee init should report broker file write denial. output: {combined}"
    );
    assert!(
        !dir.path().join("blocked-app").exists(),
        "denied bee init must not create project directory. output: {combined}"
    );
}

#[test]
fn create_deny_fs_blocks_project_creation() {
    let dir = tempdir().expect("failed to create tempdir");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .args(["create", "--deny-fs", "blocked-app", "js"])
        .output()
        .expect("failed to execute bee create");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied project directory write should fail bee create. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("FileSystem")
            && combined.contains("Write"),
        "bee create should report broker file write denial. output: {combined}"
    );
    assert!(
        !dir.path().join("blocked-app").exists(),
        "denied bee create must not create project directory. output: {combined}"
    );
}

#[test]
fn add_deny_net_fails_before_package_json_update() {
    let dir = tempdir().expect("failed to create tempdir");
    let package_json_path = dir.path().join("package.json");
    std::fs::write(
        &package_json_path,
        r#"{
  "name": "add-policy-fixture",
  "version": "1.0.0",
  "dependencies": {}
}"#,
    )
    .expect("failed to write package.json");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .args(["add", "--deny-net", "left-pad@1.3.0"])
        .output()
        .expect("failed to execute bee add");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied registry network access should fail bee add. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("Network")
            && combined.contains("Connect"),
        "bee add should report broker network denial. output: {combined}"
    );
    assert!(
        read_json(&package_json_path)["dependencies"]
            .get("left-pad")
            .is_none(),
        "denied bee add must not update package.json. output: {combined}"
    );
}

#[test]
fn add_denied_lockfile_write_fails_before_network_access() {
    let dir = tempdir().expect("failed to create tempdir");
    let package_json_path = dir.path().join("package.json");
    let cache_path = dir.path().join(".beejs_cache");
    let node_modules_path = dir.path().join("node_modules");
    std::fs::create_dir_all(&cache_path).expect("failed to create cache dir");
    std::fs::create_dir_all(&node_modules_path).expect("failed to create node_modules dir");
    let cache_path = cache_path
        .canonicalize()
        .expect("failed to canonicalize cache");
    let node_modules_path = node_modules_path
        .canonicalize()
        .expect("failed to canonicalize node_modules");
    let package_target_path = node_modules_path.join("left-pad");
    std::fs::write(
        &package_json_path,
        r#"{
  "name": "add-lock-policy-fixture",
  "version": "1.0.0",
  "dependencies": {}
}"#,
    )
    .expect("failed to write package.json");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .arg("add")
        .arg("--deny-net")
        .arg("--deny-fs")
        .arg("--allow-read")
        .arg(&package_json_path)
        .arg("--allow-write")
        .arg(&package_json_path)
        .arg("--allow-write")
        .arg(&cache_path)
        .arg("--allow-write")
        .arg(&node_modules_path)
        .arg("--allow-write")
        .arg(&package_target_path)
        .arg("left-pad@1.3.0")
        .output()
        .expect("failed to execute bee add");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied package-lock write should fail bee add. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("FileSystem")
            && combined.contains("Write")
            && combined.contains("package-lock.json"),
        "bee add should report denied package-lock write before network denial. output: {combined}"
    );
    assert!(
        read_json(&package_json_path)["dependencies"]
            .get("left-pad")
            .is_none(),
        "denied bee add must not update package.json. output: {combined}"
    );
    assert!(
        !package_target_path.exists(),
        "denied bee add must not create package target. output: {combined}"
    );
}

#[test]
fn install_deny_net_fails_before_lockfile_generation() {
    let dir = tempdir().expect("failed to create tempdir");
    std::fs::write(
        dir.path().join("package.json"),
        r#"{
  "name": "install-policy-fixture",
  "version": "1.0.0",
  "dependencies": {
    "left-pad": "1.3.0"
  }
}"#,
    )
    .expect("failed to write package.json");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .args(["install", "--deny-net"])
        .output()
        .expect("failed to execute bee install");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied registry network access should fail bee install. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("Network")
            && combined.contains("Connect"),
        "bee install should report broker network denial. output: {combined}"
    );
    assert!(
        !dir.path().join("package-lock.json").exists(),
        "denied bee install must not generate package-lock.json. output: {combined}"
    );
}

#[test]
fn install_denied_lockfile_write_fails_before_creating_node_modules() {
    let dir = tempdir().expect("failed to create tempdir");
    let package_json_path = dir.path().join("package.json");
    std::fs::write(
        &package_json_path,
        r#"{
  "name": "install-lock-policy-fixture",
  "version": "1.0.0",
  "dependencies": {}
}"#,
    )
    .expect("failed to write package.json");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .arg("install")
        .arg("--deny-fs")
        .arg("--allow-read")
        .arg("package.json")
        .arg("--allow-write")
        .arg(".beejs_cache")
        .arg("--allow-write")
        .arg("node_modules")
        .output()
        .expect("failed to execute bee install");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied package-lock write should fail bee install. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("FileSystem")
            && combined.contains("Write")
            && combined.contains("package-lock.json"),
        "bee install should report denied package-lock write. output: {combined}"
    );
    assert!(
        !dir.path().join("node_modules").exists(),
        "denied lockfile write must fail before creating node_modules. output: {combined}"
    );
}

#[test]
fn install_frozen_lockfile_mismatch_fails_before_creating_node_modules() {
    let dir = tempdir().expect("failed to create tempdir");
    std::fs::write(
        dir.path().join("package.json"),
        r#"{
  "name": "install-frozen-fixture",
  "version": "1.0.0",
  "dependencies": {
    "left-pad": "^2.0.0"
  }
}"#,
    )
    .expect("failed to write package.json");
    std::fs::write(
        dir.path().join("package-lock.json"),
        r#"{
  "name": "install-frozen-fixture",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "requires": true,
  "dependencies": {
    "left-pad": {
      "version": "1.3.0",
      "resolved": "https://registry.npmjs.org/left-pad/-/left-pad-1.3.0.tgz",
      "integrity": "sha512-test",
      "dev": false
    }
  }
}"#,
    )
    .expect("failed to write package-lock.json");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .args(["install", "--frozen-lockfile"])
        .output()
        .expect("failed to execute bee install --frozen-lockfile");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "frozen lockfile mismatch should fail bee install. output: {combined}"
    );
    assert!(
        combined.contains("frozen lockfile") && combined.contains("left-pad"),
        "frozen lockfile error should identify the mismatched package. output: {combined}"
    );
    assert!(
        !dir.path().join("node_modules").exists(),
        "frozen lockfile mismatch must fail before creating node_modules. output: {combined}"
    );
    assert!(
        !dir.path().join(".beejs_cache").exists(),
        "frozen lockfile mismatch must fail before creating package cache. output: {combined}"
    );
}

#[test]
fn install_frozen_lockfile_does_not_rewrite_package_lock() {
    let dir = tempdir().expect("failed to create tempdir");
    std::fs::write(
        dir.path().join("package.json"),
        r#"{
  "name": "install-frozen-readonly-fixture",
  "version": "1.0.0",
  "dependencies": {}
}"#,
    )
    .expect("failed to write package.json");
    let lock_path = dir.path().join("package-lock.json");
    std::fs::write(
        &lock_path,
        r#"{
  "name": "install-frozen-readonly-fixture",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "requires": true,
  "dependencies": {}
}"#,
    )
    .expect("failed to write package-lock.json");
    let original_lock = std::fs::read_to_string(&lock_path).expect("failed to read lockfile");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .arg("install")
        .arg("--frozen-lockfile")
        .arg("--deny-fs")
        .arg("--allow-read")
        .arg("package.json")
        .arg("--allow-read")
        .arg("package-lock.json")
        .arg("--allow-write")
        .arg(".beejs_cache")
        .arg("--allow-write")
        .arg("node_modules")
        .output()
        .expect("failed to execute bee install --frozen-lockfile");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "frozen lockfile install should not require package-lock write permission. output: {combined}"
    );
    assert!(
        !combined.contains("permission denied"),
        "frozen lockfile install should avoid denied package-lock writes. output: {combined}"
    );
    assert_eq!(
        std::fs::read_to_string(&lock_path).expect("failed to reread lockfile"),
        original_lock,
        "frozen lockfile install must not rewrite package-lock.json"
    );
}

#[test]
fn prune_deny_fs_fails_before_scanning_node_modules() {
    let dir = tempdir().expect("failed to create tempdir");
    let node_modules = dir.path().join("node_modules");
    std::fs::create_dir_all(node_modules.join("unused")).expect("failed to create node_modules");
    std::fs::write(
        dir.path().join("package.json"),
        r#"{
  "name": "prune-policy-fixture",
  "version": "1.0.0",
  "dependencies": {}
}"#,
    )
    .expect("failed to write package.json");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .args([
            "prune",
            "--deny-fs",
            "--allow-read",
            "package.json",
            "--allow-write",
            ".beejs_cache",
            "--allow-write",
            "node_modules",
        ])
        .output()
        .expect("failed to execute bee prune");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied node_modules read should fail bee prune. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("FileSystem")
            && combined.contains("Read"),
        "bee prune should report broker file read denial. output: {combined}"
    );
    assert!(
        node_modules.join("unused").is_dir(),
        "denied bee prune must not remove packages. output: {combined}"
    );
}

#[test]
fn prune_deny_fs_fails_before_absent_node_modules_noop() {
    let dir = tempdir().expect("failed to create tempdir");
    std::fs::write(
        dir.path().join("package.json"),
        r#"{
  "name": "prune-absent-policy-fixture",
  "version": "1.0.0",
  "dependencies": {}
}"#,
    )
    .expect("failed to write package.json");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .args(["prune", "--deny-fs"])
        .output()
        .expect("failed to execute bee prune");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied node_modules read should fail bee prune before noop. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("FileSystem")
            && combined.contains("Read"),
        "bee prune should report broker file read denial. output: {combined}"
    );
    assert!(
        !combined.contains("No node_modules directory found - nothing to prune"),
        "denied bee prune must not report a successful noop. output: {combined}"
    );
}

#[test]
fn bunx_deny_net_fails_before_package_execution() {
    let dir = tempdir().expect("failed to create tempdir");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .args(["bunx", "--deny-net", "left-pad@1.3.0"])
        .output()
        .expect("failed to execute bee bunx");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied registry network access should fail bee bunx. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("Network")
            && combined.contains("Connect"),
        "bee bunx should report broker network denial. output: {combined}"
    );
}

#[test]
fn bunx_deny_run_fails_before_package_installation() {
    let dir = tempdir().expect("failed to create tempdir");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .args(["bunx", "--deny-run", "left-pad@1.3.0"])
        .output()
        .expect("failed to execute bee bunx");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied process execution should fail bee bunx. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("Process")
            && combined.contains("Execute"),
        "bee bunx should report broker process execution denial. output: {combined}"
    );
    assert!(
        !dir.path().join("node_modules").exists(),
        "denied bee bunx must not create installation directories. output: {combined}"
    );
}

#[test]
fn serve_deny_net_blocks_server_configuration() {
    let dir = tempdir().expect("failed to create tempdir");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .args(["serve", "--deny-net", "--host", "127.0.0.1", "--port", "0"])
        .output()
        .expect("failed to execute bee serve");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied server network access should fail bee serve. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("Network")
            && combined.contains("Listen"),
        "bee serve should report broker network denial. output: {combined}"
    );
    assert!(
        !combined.contains("server configured"),
        "denied bee serve must not report successful server configuration. output: {combined}"
    );
}

#[test]
fn upgrade_deny_net_fails_without_rewriting_package_json() {
    let dir = tempdir().expect("failed to create tempdir");
    let package_json_path = dir.path().join("package.json");
    let original_package_json = r#"{
  "name": "upgrade-policy-fixture",
  "version": "1.0.0",
  "dependencies": {
    "left-pad": "1.3.0"
  }
}"#;
    std::fs::write(&package_json_path, original_package_json)
        .expect("failed to write package.json");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .args(["upgrade", "--deny-net", "left-pad"])
        .output()
        .expect("failed to execute bee upgrade");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied registry network access should fail bee upgrade. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("Network")
            && combined.contains("Connect"),
        "bee upgrade should report broker network denial. output: {combined}"
    );
    assert_eq!(
        std::fs::read_to_string(&package_json_path).expect("failed to read package.json"),
        original_package_json,
        "denied bee upgrade must not rewrite package.json. output: {combined}"
    );
}

#[test]
fn upgrade_denied_package_json_write_fails_before_creating_node_modules() {
    let dir = tempdir().expect("failed to create tempdir");
    let package_json_path = dir.path().join("package.json");
    let original_package_json = r#"{
  "name": "upgrade-write-policy-fixture",
  "version": "1.0.0",
  "dependencies": {}
}"#;
    std::fs::write(&package_json_path, original_package_json)
        .expect("failed to write package.json");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .arg("upgrade")
        .arg("--deny-fs")
        .arg("--allow-read")
        .arg("package.json")
        .arg("--allow-write")
        .arg(".beejs_cache")
        .arg("--allow-write")
        .arg("node_modules")
        .arg("--allow-write")
        .arg("package-lock.json")
        .output()
        .expect("failed to execute bee upgrade");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied package.json write should fail bee upgrade. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("FileSystem")
            && combined.contains("Write")
            && combined.contains("package.json"),
        "bee upgrade should report denied package.json write. output: {combined}"
    );
    assert_eq!(
        std::fs::read_to_string(&package_json_path).expect("failed to read package.json"),
        original_package_json,
        "denied bee upgrade must not rewrite package.json. output: {combined}"
    );
    assert!(
        !dir.path().join("node_modules").exists(),
        "denied package.json write must fail before creating node_modules. output: {combined}"
    );
}

#[test]
fn bundle_deny_fs_blocks_entry_read() {
    let dir = tempdir().expect("failed to create tempdir");
    let entry = dir.path().join("entry.js");
    let outfile = dir.path().join("bundle.js");
    std::fs::write(&entry, "console.log('bundle-input');").expect("failed to write entry");

    let output = Command::new(bee_path())
        .arg("bundle")
        .arg("--deny-fs")
        .arg(&entry)
        .arg("--outfile")
        .arg(&outfile)
        .output()
        .expect("failed to execute bee bundle");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied entry read should fail bee bundle. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("FileSystem")
            && combined.contains("Read"),
        "bee bundle should report broker file read denial. output: {combined}"
    );
    assert!(
        !outfile.exists(),
        "denied bee bundle must not create output. output: {combined}"
    );
}

#[test]
fn bundle_deny_fs_blocks_output_write() {
    let dir = tempdir().expect("failed to create tempdir");
    let entry = dir.path().join("entry.js");
    let outfile = dir.path().join("bundle.js");
    std::fs::write(&entry, "console.log('bundle-input');").expect("failed to write entry");

    let output = Command::new(bee_path())
        .arg("bundle")
        .arg("--deny-fs")
        .arg("--allow-read")
        .arg(&entry)
        .arg(&entry)
        .arg("--outfile")
        .arg(&outfile)
        .output()
        .expect("failed to execute bee bundle");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied bundle output write should fail. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("FileSystem")
            && combined.contains("Write"),
        "bee bundle should report broker file write denial. output: {combined}"
    );
    assert!(
        !outfile.exists(),
        "denied bee bundle must not create output. output: {combined}"
    );
}

#[test]
fn remove_permission_policy_denies_package_json_write() {
    let dir = tempdir().expect("failed to create tempdir");
    let package_json_path = dir.path().join("package.json");
    let policy_path = dir.path().join("bee.policy.json");
    std::fs::write(
        &package_json_path,
        r#"{
  "name": "remove-policy-fixture",
  "version": "1.0.0",
  "dependencies": {
    "left-pad": "1.3.0"
  }
}"#,
    )
    .expect("failed to write package.json");
    std::fs::write(
        &policy_path,
        r#"{
  "permissions": {
    "deny_fs": true,
    "allow_read": ["package.json"]
  }
}"#,
    )
    .expect("failed to write permission policy");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .arg("remove")
        .arg("--permission-policy")
        .arg(&policy_path)
        .arg("left-pad")
        .output()
        .expect("failed to execute bee remove");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied package.json write should fail. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("FileSystem")
            && combined.contains("Write"),
        "denied write should report broker permission details. output: {combined}"
    );

    let package_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&package_json_path).expect("failed to read package.json"),
    )
    .expect("package.json should remain valid JSON");
    assert!(
        package_json["dependencies"].get("left-pad").is_some(),
        "denied remove must not mutate package.json. output: {combined}"
    );
}

#[test]
fn test_command_executes_test_callbacks_and_reports_assertion_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("failing_assertion.test.js");
    std::fs::write(
        &test_file,
        r#"
test("fails inside the test callback", () => {
  expect(1).toBe(2);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    assert!(
        !output.status.success(),
        "bee test should return non-zero for a failing assertion"
    );

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Expected 1 to be 2"),
        "bee test should report the assertion failure, not a missing test global. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "bee test must not report success for a failing test. output: {combined}"
    );
}

#[test]
fn test_command_fails_when_file_registers_no_tests() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("empty.test.js");
    std::fs::write(
        &test_file,
        r#"
const collected = [];
collected.push("loaded but no tests");
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail when an explicit test file registers no tests. output: {combined}"
    );
    assert!(
        combined.contains("No tests found in test file"),
        "empty test file should report a no-tests diagnostic. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "empty test file must not report success. output: {combined}"
    );
}

#[test]
fn test_command_counts_todo_file_tests_as_skipped() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("todo.test.js");
    std::fs::write(
        &test_file,
        r#"
test.todo("planned root behavior");
it.todo("planned alias behavior");
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should treat todo tests as skipped. output: {combined}"
    );
    assert!(
        combined.contains("0 passed, 0 failed, 2 skipped"),
        "todo tests should be counted as skipped. output: {combined}"
    );
}

#[test]
fn test_command_supports_failing_file_tests() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("failing_expected.test.js");
    std::fs::write(
        &test_file,
        r#"
test.failing("expected assertion failure passes", () => {
  expect(1).toBe(2);
});

it.failing("expected async rejection passes", async () => {
  throw new Error("planned failure");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should treat expected failures as passed. output: {combined}"
    );
    assert!(
        combined.contains("2 passed, 0 failed, 0 skipped"),
        "expected failing tests should count as passed. output: {combined}"
    );
}

#[test]
fn test_command_reports_failing_file_test_that_unexpectedly_passes() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("failing_unexpected_pass.test.js");
    std::fs::write(
        &test_file,
        r#"
test.failing("unexpected pass should fail", () => {
  expect(1).toBe(1);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail when a failing test unexpectedly passes. output: {combined}"
    );
    assert!(
        combined.contains("Expected failing test to fail"),
        "unexpectedly passing failing test should explain the inversion. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "unexpectedly passing failing test must not report success. output: {combined}"
    );
}

#[test]
fn test_command_supports_failing_each_file_tests() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("failing_each.test.js");
    std::fs::write(
        &test_file,
        r#"
test.failing.each([
  [1, 2],
  [2, 3],
])("planned mismatch %i vs %i", (actual, expected) => {
  expect(actual).toBe(expected);
});

it.failing.each([
  { value: "bee" },
])("planned throw $value", ({ value }) => {
  throw new Error(value);
});

it.concurrent.failing.each`
  value
  ${"async bee"}
`("planned async throw $value", async ({ value }) => {
  throw new Error(value);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should accept failing.each APIs and treat expected failures as passed. output: {combined}"
    );
    assert!(
        combined.contains("4 passed, 0 failed, 0 skipped"),
        "all failing.each rows should count as passed. output: {combined}"
    );
}

#[test]
fn test_command_reports_failing_each_row_that_unexpectedly_passes() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("failing_each_unexpected_pass.test.js");
    std::fs::write(
        &test_file,
        r#"
test.failing.each([
  [1],
])("unexpected pass row %#", (value) => {
  expect(value).toBe(1);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail when a failing.each row unexpectedly passes. output: {combined}"
    );
    assert!(
        combined.contains("Expected failing test to fail"),
        "unexpectedly passing failing.each row should explain the inversion. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed failing.each row must not report success. output: {combined}"
    );
}

#[test]
fn test_command_supports_each_table_tests_and_describes() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("each_table.test.js");
    std::fs::write(
        &test_file,
        r#"
const seen = [];

test.each([
  [1, 2, 3],
  [2, 3, 5]
])("adds %i + %i = %i", (left, right, expected) => {
  seen.push(`${left}+${right}`);
  expect(left + right).toBe(expected);
});

describe.each([
  ["runtime", 2],
  ["tests", 3]
])("%s suite", (name, count) => {
  test("receives row arguments", () => {
    seen.push(`${name}:${count}`);
    expect(name).toEqual(expect.any(String));
    expect(count).toBeGreaterThan(1);
  });
});

test("observes all expanded rows", () => {
  expect(seen).toEqual(["1+2", "2+3", "runtime:2", "tests:3"]);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support test.each and describe.each table expansion. output: {combined}"
    );
    assert!(
        combined.contains("5 passed, 0 failed, 0 skipped"),
        "each table should expand into the expected tests. output: {combined}"
    );
}

#[test]
fn test_command_reports_each_table_row_failure_name() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("each_table_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test.each([
  [1, 2, 4]
])("adds %i + %i = %i", (left, right, expected) => {
  expect(left + right).toBe(expected);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail the expanded table row. output: {combined}"
    );
    assert!(
        combined.contains("adds 1 + 2 = 4"),
        "each row failure should include the expanded test name. output: {combined}"
    );
    assert!(
        combined.contains("Expected 3 to be 4"),
        "each row failure should include the assertion mismatch. output: {combined}"
    );
}

#[test]
fn test_command_supports_each_tagged_template_tables() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("each_template_table.test.js");
    std::fs::write(
        &test_file,
        r#"
const seen = [];

test.each`
  left | right | expected
  ${1} | ${2}  | ${3}
  ${2} | ${3}  | ${5}
`("adds $left + $right = $expected (#$#)", ({ left, right, expected }) => {
  seen.push(`${left}+${right}`);
  expect(left + right).toBe(expected);
});

describe.each`
  name       | count
  ${"bee"}   | ${3}
  ${"tests"} | ${5}
`("$name suite", ({ name, count }) => {
  test("receives tagged row object", () => {
    seen.push(`${name}:${count}`);
    expect(name).toEqual(expect.any(String));
    expect(count).toBeGreaterThan(2);
  });
});

test("observes all tagged rows", () => {
  expect(seen).toEqual(["1+2", "2+3", "bee:3", "tests:5"]);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support tagged-template each tables. output: {combined}"
    );
    assert!(
        combined.contains("5 passed, 0 failed, 0 skipped"),
        "tagged each table should expand into the expected tests. output: {combined}"
    );
}

#[test]
fn test_command_reports_each_tagged_template_row_failure_name() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("each_template_table_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test.each`
  left | right | expected
  ${1} | ${2}  | ${4}
`("adds $left + $right = $expected", ({ left, right, expected }) => {
  expect(left + right).toBe(expected);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail the expanded tagged table row. output: {combined}"
    );
    assert!(
        combined.contains("adds 1 + 2 = 4"),
        "tagged each row failure should include the expanded test name. output: {combined}"
    );
    assert!(
        combined.contains("Expected 3 to be 4"),
        "tagged each row failure should include the assertion mismatch. output: {combined}"
    );
}

#[test]
fn test_command_supports_existing_file_snapshots() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("snapshot.test.js");
    std::fs::write(
        &test_file,
        r#"
test("matches stored object snapshot", () => {
  expect({ name: "beejs", features: ["runtime", "tests"] }).toMatchSnapshot();
});
"#,
    )
    .expect("failed to write test file");
    let snapshots_dir = dir.path().join("__snapshots__");
    std::fs::create_dir_all(&snapshots_dir).expect("failed to create snapshots dir");
    std::fs::write(
        snapshots_dir.join("snapshot.test.js.snap"),
        r#"exports[`matches stored object snapshot 1`] = `
{
  "name": "beejs",
  "features": [
    "runtime",
    "tests"
  ]
}
`;
"#,
    )
    .expect("failed to write snapshot file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should pass when toMatchSnapshot matches an existing snapshot. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "snapshot test should count as passed. output: {combined}"
    );
}

#[test]
fn test_command_fails_missing_file_snapshot() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("missing_snapshot.test.js");
    std::fs::write(
        &test_file,
        r#"
test("needs snapshot", () => {
  expect("beejs").toMatchSnapshot();
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail when a required snapshot is missing. output: {combined}"
    );
    assert!(
        combined.contains("Snapshot not found for needs snapshot 1"),
        "missing snapshot failure should name the missing snapshot key. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "missing snapshot must not report success. output: {combined}"
    );
}

#[test]
fn test_command_update_snapshots_creates_missing_file_snapshot() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("update_snapshot.test.js");
    std::fs::write(
        &test_file,
        r#"
test("writes snapshot", () => {
  expect({ name: "beejs", mode: "update" }).toMatchSnapshot();
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg("--update-snapshots")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test --update-snapshots should create a missing snapshot and pass. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "updated snapshot test should count as passed. output: {combined}"
    );
    let snapshot_file = dir
        .path()
        .join("__snapshots__")
        .join("update_snapshot.test.js.snap");
    let snapshot =
        std::fs::read_to_string(&snapshot_file).expect("update mode should create a snapshot file");
    assert!(
        snapshot.contains("exports[`writes snapshot 1`]"),
        "snapshot file should contain the generated snapshot key. content: {snapshot}"
    );
    assert!(
        snapshot.contains(r#""mode": "update""#),
        "snapshot file should contain serialized received data. content: {snapshot}"
    );
}

#[test]
fn test_command_update_snapshots_respects_write_permission() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("denied_update_snapshot.test.js");
    std::fs::write(
        &test_file,
        r#"
test("blocked snapshot", () => {
  expect("beejs").toMatchSnapshot();
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg("--deny-fs")
        .arg("--allow-read")
        .arg(&test_file)
        .arg("--update-snapshots")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "snapshot update should fail when snapshot write is denied. output: {combined}"
    );
    assert!(
        combined.contains("permission denied"),
        "denied snapshot write should report permission denied. output: {combined}"
    );
    assert!(
        !dir.path().join("__snapshots__").exists(),
        "denied snapshot update must not create a snapshots directory"
    );
}

#[test]
fn test_command_supports_inline_snapshots() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("inline_snapshot.test.js");
    std::fs::write(
        &test_file,
        r#"
test("matches inline snapshot", () => {
  expect({ name: "beejs", mode: "inline" }).toMatchInlineSnapshot(`
{
  "name": "beejs",
  "mode": "inline"
}
`);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should pass matching inline snapshots. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "inline snapshot test should count as passed. output: {combined}"
    );
}

#[test]
fn test_command_reports_inline_snapshot_mismatch() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("inline_snapshot_mismatch.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports inline mismatch", () => {
  expect({ name: "beejs" }).toMatchInlineSnapshot(`"wrong"`);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail mismatched inline snapshots. output: {combined}"
    );
    assert!(
        combined.contains("Inline snapshot mismatch"),
        "inline snapshot mismatch should report a clear diagnostic. output: {combined}"
    );
    assert!(
        combined.contains("Received:"),
        "inline snapshot mismatch should include received data. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "mismatched inline snapshot must not report success. output: {combined}"
    );
}

#[test]
fn test_command_update_snapshots_creates_inline_snapshot() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("inline_snapshot_update.test.js");
    std::fs::write(
        &test_file,
        r#"
test("writes inline snapshot", () => {
  expect({ name: "beejs", mode: "inline-update" }).toMatchInlineSnapshot();
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg("--update-snapshots")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test --update-snapshots should write a missing inline snapshot and pass. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "updated inline snapshot test should count as passed. output: {combined}"
    );

    let source = std::fs::read_to_string(&test_file).expect("test file should remain readable");
    assert!(
        source.contains("toMatchInlineSnapshot(`"),
        "source should receive an inline snapshot literal. source: {source}"
    );
    assert!(
        source.contains(r#""mode": "inline-update""#),
        "inline snapshot should contain serialized received data. source: {source}"
    );
    assert!(
        !source.contains("toMatchInlineSnapshot();"),
        "source should no longer contain an empty inline snapshot call. source: {source}"
    );
}

#[test]
fn test_command_update_snapshots_rewrites_inline_snapshot_mismatch() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("inline_snapshot_rewrite.test.js");
    std::fs::write(
        &test_file,
        r#"
test("rewrites inline snapshot", () => {
  expect({ name: "beejs", mode: "rewritten" }).toMatchInlineSnapshot(`"old"`);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg("--update-snapshots")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test --update-snapshots should rewrite mismatched inline snapshots and pass. output: {combined}"
    );

    let source = std::fs::read_to_string(&test_file).expect("test file should remain readable");
    assert!(
        source.contains(r#""mode": "rewritten""#),
        "rewritten inline snapshot should contain received data. source: {source}"
    );
    assert!(
        !source.contains("\"old\""),
        "old inline snapshot value should be replaced. source: {source}"
    );
}

#[test]
fn test_command_update_inline_snapshot_respects_source_write_permission() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("inline_snapshot_denied.test.js");
    let original_source = r#"
test("blocked inline snapshot", () => {
  expect("beejs").toMatchInlineSnapshot();
});
"#;
    std::fs::write(&test_file, original_source).expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg("--deny-fs")
        .arg("--allow-read")
        .arg(&test_file)
        .arg("--update-snapshots")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "inline snapshot update should fail when source write is denied. output: {combined}"
    );
    assert!(
        combined.contains("permission denied"),
        "denied source write should report permission denied. output: {combined}"
    );

    let source = std::fs::read_to_string(&test_file).expect("test file should remain readable");
    assert_eq!(
        source, original_source,
        "denied inline snapshot update must not modify source"
    );
}

#[test]
fn run_file_resolves_commonjs_relative_to_script_directory() {
    let dir = tempdir().expect("failed to create tempdir");
    let app_dir = dir.path().join("app");
    let lib_dir = app_dir.join("lib");
    std::fs::create_dir_all(&lib_dir).expect("failed to create lib directory");
    std::fs::write(lib_dir.join("index.js"), "exports.value = 314;")
        .expect("failed to write lib module");
    let main_file = app_dir.join("main.js");
    std::fs::write(&main_file, "require('./lib').value;").expect("failed to write main file");

    let output = Command::new(bee_path())
        .arg("run")
        .arg(&main_file)
        .output()
        .expect("failed to execute bee run");

    assert!(
        output.status.success(),
        "bee run should resolve relative require from script dir. stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "314");
}

#[test]
fn run_typescript_type_errors_do_not_block_execution() {
    let dir = tempdir().expect("failed to create tempdir");
    let script = dir.path().join("diagnostic_error.ts");
    std::fs::write(
        &script,
        r#"
function answer(): string {
  return 1;
}
console.log("TYPE_DIAGNOSTIC_SCRIPT_RAN", answer());
"#,
    )
    .expect("failed to write TypeScript script");

    let output = Command::new(bee_path())
        .arg("run")
        .arg(&script)
        .output()
        .expect("failed to execute bee run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(
        output.status.success(),
        "oxc is transpile-only: type mismatches must not fail bee run. output: {combined}"
    );
    assert!(
        stdout.contains("TYPE_DIAGNOSTIC_SCRIPT_RAN"),
        "transpiled JS should still execute. stdout: {stdout}"
    );
}

#[test]
fn run_typescript_runtime_error_reports_original_source_location() {
    let dir = tempdir().expect("failed to create tempdir");
    let script = dir.path().join("runtime_error.ts");
    std::fs::write(
        &script,
        r#"const label: string = "before";
function explode(): void {
  const detail: string = label;
  throw new Error("TS_SOURCE_MAP_RUNTIME_ERROR:" + detail);
}
explode();
"#,
    )
    .expect("failed to write TypeScript script");

    let output = Command::new(bee_path())
        .arg("run")
        .arg(&script)
        .output()
        .expect("failed to execute bee run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(
        !output.status.success(),
        "bee run should fail for the thrown TypeScript runtime error. output: {combined}"
    );
    assert!(
        combined.contains("TS_SOURCE_MAP_RUNTIME_ERROR:before"),
        "runtime error message should be reported. output: {combined}"
    );
    assert!(
        combined.contains(&script.to_string_lossy().to_string()),
        "runtime error should mention original TypeScript file path. output: {combined}"
    );
    assert!(
        combined.contains(":4:"),
        "runtime error should point at original TypeScript throw line 4. output: {combined}"
    );
}

#[test]
fn run_tsx_top_level_await_settles_before_exit() {
    let dir = tempdir().expect("failed to create tempdir");
    let script = dir.path().join("top_level_await.tsx");
    std::fs::write(
        &script,
        r#"
const answer: number = await Promise.resolve(42);
console.log("TSX_TYPED_TLA_OK", answer);
"#,
    )
    .expect("failed to write TSX script");

    let output = Command::new(bee_path())
        .arg("run")
        .arg(&script)
        .output()
        .expect("failed to execute bee run");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "tsx top-level await should run before process exit. output: {combined}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "TSX_TYPED_TLA_OK 42"
    );
}

#[test]
fn test_command_reports_async_assertion_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("failing_async_assertion.test.js");
    std::fs::write(
        &test_file,
        r#"
test("fails after an awaited microtask", async () => {
  await Promise.resolve();
  expect(1).toBe(2);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    assert!(
        !output.status.success(),
        "bee test should return non-zero for an async assertion failure"
    );

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Expected 1 to be 2"),
        "bee test should report async assertion failures. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "bee test must not report success for a failing async test. output: {combined}"
    );
}

#[test]
fn test_command_supports_done_callback_async_file_tests() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("done_callback.test.js");
    std::fs::write(
        &test_file,
        r#"
let completed = false;

test("passes through done callback", (done) => {
  setTimeout(() => {
    try {
      expect("beejs").toContain("bee");
      completed = true;
      done();
    } catch (error) {
      done(error);
    }
  }, 1);
});

test("runs after done completed", () => {
  expect(completed).toBe(true);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .args(["test", "--timeout", "1"])
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should wait for done callback completion. output: {combined}"
    );
    assert!(
        combined.contains("2 passed, 0 failed, 0 skipped"),
        "done callback test should pass after callback completion. output: {combined}"
    );
}

#[test]
fn test_command_reports_done_callback_error() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("done_callback_error.test.js");
    std::fs::write(
        &test_file,
        r#"
test("fails through done callback", (done) => {
  setTimeout(() => {
    done(new Error("done callback failed"));
  }, 1);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .args(["test", "--timeout", "1"])
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail when done receives an error. output: {combined}"
    );
    assert!(
        combined.contains("done callback failed"),
        "done callback error should be reported. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed done callback must not report success. output: {combined}"
    );
}

#[test]
fn test_command_reports_done_callback_returned_promise_conflict() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("done_callback_promise_conflict.test.js");
    std::fs::write(
        &test_file,
        r#"
test("does not mix done and promise", (done) => {
  return Promise.resolve().then(() => done());
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .args(["test", "--timeout", "1"])
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail when a done callback test returns a Promise. output: {combined}"
    );
    assert!(
        combined.contains("cannot both use done callback and return a Promise"),
        "done/promise conflict should report a clear error. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "done/promise conflict must not report success. output: {combined}"
    );
}

#[test]
fn test_command_reports_async_assertion_failure_in_mjs_file_mode() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("failing_async_assertion.test.mjs");
    std::fs::write(
        &test_file,
        r#"
test("fails after an awaited microtask in mjs", async () => {
  await Promise.resolve();
  expect(1).toBe(2);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should return non-zero for a failing async mjs test. output: {combined}"
    );
    assert!(
        combined.contains("Expected 1 to be 2"),
        "bee test should report async mjs assertion failures. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "bee test must not report success for a failing async mjs test. output: {combined}"
    );
}

#[test]
fn test_command_supports_expect_assertion_guards() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("expect_assertion_guards.test.js");
    std::fs::write(
        &test_file,
        r#"
test("counts explicit assertions", async () => {
  expect.assertions(3);
  expect("beejs runtime").toContain("runtime");
  await Promise.resolve("bee").then((value) => {
    expect(value).toBe("bee");
  });
  expect({ ok: true }).toEqual({ ok: true });
});

test("requires at least one assertion", () => {
  expect.hasAssertions();
  expect("bee").toBeDefined();
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support expect assertion guards. output: {combined}"
    );
    assert!(
        combined.contains("2 passed, 0 failed, 0 skipped"),
        "expect assertion guard tests should pass. output: {combined}"
    );
}

#[test]
fn test_command_reports_expect_assertions_mismatch() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("expect_assertions_mismatch.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports assertion count mismatch", () => {
  expect.assertions(2);
  expect("bee").toBe("bee");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail an expect.assertions mismatch. output: {combined}"
    );
    assert!(
        combined.contains("Expected 2 assertions") && combined.contains("but 1"),
        "expect.assertions mismatch should report expected and actual counts. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed expect.assertions mismatch must not report success. output: {combined}"
    );
}

#[test]
fn test_command_reports_expect_has_assertions_missing() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("expect_has_assertions_missing.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports missing assertions", () => {
  expect.hasAssertions();
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail when expect.hasAssertions sees no assertions. output: {combined}"
    );
    assert!(
        combined.contains("Expected at least one assertion"),
        "expect.hasAssertions mismatch should explain the missing assertion. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed expect.hasAssertions mismatch must not report success. output: {combined}"
    );
}

#[test]
fn test_command_supports_core_file_matchers() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("core_matchers.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports core matchers", () => {
  expect(null).toBeNull();
  expect(undefined).toBeUndefined();
  expect("bee").toBeDefined();
  expect([1, 2, 3]).toContain(2);
  expect("beejs runtime").toContain("runtime");
  expect([1, 2, 3]).toHaveLength(3);
  expect("bee").toHaveLength(3);
  expect("bee-123").toMatch(/bee-\d+/);
  expect("bee-123").toMatch("123");
  expect(1).not.toBe(2);
  expect({ value: 1 }).not.toEqual({ value: 2 });
  expect("bee").not.toContain("ant");
  expect(false).not.toBeTruthy();
  expect(true).not.toBeFalsy();
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support core expect matchers. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "core matcher test should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_map_and_set_equality_matchers() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("map_set_equality_matchers.test.js");
    std::fs::write(
        &test_file,
        r#"
test("compares Map and Set contents", () => {
  expect(new Map([
    ["mode", "bee"],
    ["meta", { runtime: true }]
  ])).toEqual(new Map([
    ["mode", "bee"],
    ["meta", { runtime: true }]
  ]));
  expect(new Map([["mode", "bee"]])).not.toEqual(new Map([["mode", "ant"]]));
  expect(new Set(["runtime", "v8"])).toEqual(new Set(["runtime", "v8"]));
  expect(new Set(["runtime"])).not.toEqual(new Set(["docs"]));

  expect(new Map([["mode", "bee"]])).toStrictEqual(new Map([["mode", "bee"]]));
  expect(new Map([["mode", "bee"]])).not.toStrictEqual(new Map([["mode", "ant"]]));
  expect(new Set(["runtime", "v8"])).toStrictEqual(new Set(["runtime", "v8"]));
  expect(new Set(["runtime"])).not.toStrictEqual(new Set(["docs"]));
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support Map and Set equality matchers. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "Map and Set equality matcher test should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_to_contain_equal_file_matcher() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("to_contain_equal_matcher.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports toContainEqual matcher", () => {
  const items = [
    { id: "bee", meta: { tags: ["runtime", "v8"] } },
    { id: "ant", meta: { tags: ["legacy"] } }
  ];

  expect(items).toContainEqual({ id: "bee", meta: { tags: ["runtime", "v8"] } });
  expect(items).toContainEqual(expect.objectContaining({
    id: "ant",
    meta: expect.objectContaining({ tags: expect.arrayContaining(["legacy"]) })
  }));
  expect([[1, { nested: true }], [2]]).toContainEqual([1, { nested: true }]);
  expect(items).not.toContainEqual({ id: "wasp", meta: { tags: ["runtime"] } });
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support toContainEqual matcher. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "toContainEqual matcher test should pass. output: {combined}"
    );
}

#[test]
fn test_command_reports_to_contain_equal_matcher_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("to_contain_equal_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports toContainEqual mismatch", () => {
  expect([{ id: "bee" }]).toContainEqual({ id: "ant" });
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail a toContainEqual mismatch. output: {combined}"
    );
    assert!(
        combined.contains("to contain equal"),
        "toContainEqual mismatch should explain the matcher failure. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed toContainEqual matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_supports_expect_extend_custom_matcher() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("expect_extend_matcher.test.js");
    std::fs::write(
        &test_file,
        r#"
expect.extend({
  toBeEven(received) {
    const pass = typeof received === "number" && received % 2 === 0;
    return {
      pass,
      message() {
        return this.isNot
          ? `expected ${received} not to be even`
          : `expected ${received} to be even`;
      },
    };
  },
});

test("supports custom matcher from expect.extend", () => {
  expect.assertions(2);
  expect(4).toBeEven();
  expect(3).not.toBeEven();
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support expect.extend custom matchers. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "custom matcher test should pass. output: {combined}"
    );
}

#[test]
fn test_command_reports_expect_extend_custom_matcher_failure_message() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("expect_extend_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
expect.extend({
  toBeEven(received) {
    const pass = typeof received === "number" && received % 2 === 0;
    return {
      pass,
      message() {
        return this.isNot
          ? `expected ${received} not to be even`
          : `expected ${received} to be even`;
      },
    };
  },
});

test("reports custom matcher failure", () => {
  expect(3).toBeEven();
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail a custom matcher mismatch. output: {combined}"
    );
    assert!(
        combined.contains("expected 3 to be even"),
        "custom matcher failure should use matcher message. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed custom matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_reports_expect_extend_custom_not_matcher_failure_message() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("expect_extend_not_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
expect.extend({
  toBeEven(received) {
    const pass = typeof received === "number" && received % 2 === 0;
    return {
      pass,
      message() {
        return this.isNot
          ? `expected ${received} not to be even`
          : `expected ${received} to be even`;
      },
    };
  },
});

test("reports custom negated matcher failure", () => {
  expect(4).not.toBeEven();
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail a negated custom matcher mismatch. output: {combined}"
    );
    assert!(
        combined.contains("expected 4 not to be even"),
        "negated custom matcher failure should use matcher message. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed negated custom matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_supports_strict_equal_file_matcher() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("strict_equal_matcher.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports strict equality matcher", () => {
  class Payload {
    constructor(value) {
      this.value = value;
    }
  }

  expect(new Payload(7)).toStrictEqual(new Payload(7));
  expect(new Payload(7)).not.toStrictEqual({ value: 7 });
  expect([, "bee"]).not.toStrictEqual([undefined, "bee"]);
  expect({ nested: [{ value: 1 }] }).toStrictEqual({ nested: [{ value: 1 }] });
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support strict equality matcher. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "strict equality matcher test should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_asymmetric_file_matchers() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("asymmetric_matchers.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports asymmetric matchers", () => {
  const payload = {
    id: "bee-123",
    profile: {
      name: "Ada",
      tags: ["runtime", "tests", "v8"]
    }
  };
  const mock = jest.fn();
  mock({ type: "event", payload });

  expect(payload).toEqual(expect.objectContaining({
    id: expect.stringContaining("bee"),
    profile: expect.objectContaining({
      name: expect.any(String),
      tags: expect.arrayContaining(["runtime", expect.stringContaining("v")])
    })
  }));
  expect(payload).toHaveProperty("profile.name", expect.any(String));
  expect(payload.profile.tags).toContain(expect.stringContaining("tes"));
  expect(mock).toHaveBeenCalledWith(expect.objectContaining({
    type: "event",
    payload: expect.objectContaining({ id: expect.anything() })
  }));
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support asymmetric expect matchers. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "asymmetric matcher test should pass. output: {combined}"
    );
}

#[test]
fn test_command_reports_asymmetric_matcher_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("asymmetric_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports asymmetric matcher mismatch", () => {
  expect({ id: "bee-123" }).toEqual(expect.objectContaining({
    id: expect.stringContaining("ant")
  }));
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail an asymmetric matcher mismatch. output: {combined}"
    );
    assert!(
        combined.contains("to equal"),
        "asymmetric matcher mismatch should explain the matcher failure. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed asymmetric matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_supports_string_matching_and_inverted_asymmetric_matchers() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("inverted_asymmetric_matchers.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports stringMatching and inverted asymmetric matchers", () => {
  const payload = {
    id: "bee-123",
    tags: ["runtime", "tests"],
    meta: { owner: "Ada" }
  };
  const mock = jest.fn();
  mock("bee-123", payload);

  expect(payload).toEqual(expect.objectContaining({
    id: expect.stringMatching(/^bee-\d+$/),
    tags: expect.not.arrayContaining(["legacy"]),
    meta: expect.not.objectContaining({ owner: expect.stringMatching(/Grace/) })
  }));
  expect(payload.id).toEqual(expect.stringMatching("bee-"));
  expect(payload.id).toEqual(expect.not.stringContaining("ant"));
  expect(mock).toHaveBeenCalledWith(expect.stringMatching(/bee/), expect.not.objectContaining({
    id: expect.stringMatching(/^ant/)
  }));
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support stringMatching and expect.not asymmetric matchers. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "inverted asymmetric matcher test should pass. output: {combined}"
    );
}

#[test]
fn test_command_reports_inverted_asymmetric_matcher_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir
        .path()
        .join("inverted_asymmetric_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports inverted asymmetric mismatch", () => {
  expect(["runtime", "tests"]).toEqual(expect.not.arrayContaining(["tests"]));
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail an inverted asymmetric matcher mismatch. output: {combined}"
    );
    assert!(
        combined.contains("to equal"),
        "inverted asymmetric matcher mismatch should explain the failure. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed inverted asymmetric matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_reports_strict_equal_matcher_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("strict_equal_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports strict equality mismatch", () => {
  expect([, "bee"]).toStrictEqual([undefined, "bee"]);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail a strict equality mismatch. output: {combined}"
    );
    assert!(
        combined.contains("to strictly equal"),
        "strict equality mismatch should explain the matcher failure. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed strict equality matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_supports_numeric_file_matchers() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("numeric_matchers.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports numeric matchers", () => {
  expect(10).toBeGreaterThan(5);
  expect(3).toBeLessThan(7);
  expect(10).toBeGreaterThanOrEqual(10);
  expect(5).toBeLessThanOrEqual(5);
  expect(Number.NaN).toBeNaN();
  expect(5).not.toBeGreaterThan(10);
  expect(10).not.toBeLessThan(5);
  expect(9).not.toBeGreaterThanOrEqual(10);
  expect(11).not.toBeLessThanOrEqual(10);
  expect(42).not.toBeNaN();
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support numeric expect matchers. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "numeric matcher test should pass. output: {combined}"
    );
}

#[test]
fn test_command_reports_to_be_nan_matcher_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("to_be_nan_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports toBeNaN matcher failure", () => {
  expect(42).toBeNaN();
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail a toBeNaN matcher assertion. output: {combined}"
    );
    assert!(
        combined.contains("Expected 42 to be NaN"),
        "toBeNaN matcher failure should explain the mismatch. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed toBeNaN matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_reports_numeric_matcher_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("numeric_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports numeric matcher failure", () => {
  expect(3).toBeGreaterThan(5);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail a numeric matcher assertion. output: {combined}"
    );
    assert!(
        combined.contains("Expected 3 to be greater than 5"),
        "numeric matcher failure should explain the mismatch. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed numeric matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_supports_close_to_matchers() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("close_to_matchers.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports close numeric matchers", () => {
  expect(0.1 + 0.2).toBeCloseTo(0.3, 5);
  expect(3.14159).toBeCloseTo(3.14, 2);
  expect(3.14159).not.toBeCloseTo(3.2, 2);
  expect({ total: 0.1 + 0.2 }).toEqual({ total: expect.closeTo(0.3, 5) });
  expect([0.3333]).toContain(expect.closeTo(1 / 3, 3));
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support close-to expect matchers. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "close-to matcher test should pass. output: {combined}"
    );
}

#[test]
fn test_command_reports_close_to_matcher_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("close_to_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports close numeric mismatch", () => {
  expect(0.31).toBeCloseTo(0.3, 2);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail a close-to matcher assertion. output: {combined}"
    );
    assert!(
        combined.contains("to be close to"),
        "close-to matcher failure should explain the mismatch. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed close-to matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_supports_to_throw_expected_file_matchers() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("to_throw_expected.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports toThrow expected variants", () => {
  expect(() => { throw new Error("boom goes bee"); }).toThrow("goes");
  expect(() => { throw new Error("boom goes bee"); }).toThrow(/boom .* bee/);
  expect(() => { throw new TypeError("typed boom"); }).toThrow(TypeError);
  expect(() => { throw new Error("plain boom"); }).not.toThrow(TypeError);
  expect(() => {}).not.toThrow("boom");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support toThrow expected variants. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "toThrow expected matcher test should pass. output: {combined}"
    );
}

#[test]
fn test_command_reports_to_throw_expected_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("to_throw_expected_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports toThrow expected mismatch", () => {
  expect(() => { throw new Error("actual message"); }).toThrow("expected message");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail a toThrow expected mismatch. output: {combined}"
    );
    assert!(
        combined.contains("Expected function to throw an error matching \"expected message\""),
        "toThrow expected failure should explain the mismatch. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed toThrow expected matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_supports_resolves_and_rejects_file_matchers() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("async_matchers.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports resolves and rejects", async () => {
  await expect(Promise.resolve(42)).resolves.toBe(42);
  await expect(Promise.resolve("beejs runtime")).resolves.toContain("runtime");
  await expect(Promise.resolve(5)).resolves.not.toBeGreaterThan(10);
  await expect(Promise.reject(new Error("boom goes bee"))).rejects.toThrow("goes bee");
  await expect(Promise.reject(new TypeError("typed boom"))).rejects.toThrow(TypeError);
  await expect(Promise.reject("plain reason")).rejects.toMatch("plain");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support resolves/rejects matcher chains. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "resolves/rejects matcher test should pass. output: {combined}"
    );
}

#[test]
fn test_command_reports_resolves_rejects_state_mismatch() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("async_matcher_state_mismatch.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports resolves mismatch", async () => {
  await expect(Promise.reject(new Error("boom"))).resolves.toBe(1);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail when a resolves chain receives a rejection. output: {combined}"
    );
    assert!(
        combined.contains("Expected promise to resolve, but it rejected with Error: boom"),
        "resolves state mismatch should explain promise state. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed resolves matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_supports_object_file_matchers() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("object_matchers.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports object matchers", () => {
  class CustomError extends Error {}
  const error = new CustomError("typed");
  const payload = {
    profile: {
      name: "Ada",
      nickname: undefined,
      stats: { score: 7, extra: true }
    },
    tags: ["runtime"]
  };

  expect(payload).toHaveProperty("profile.stats.score", 7);
  expect(payload).toHaveProperty("profile.nickname");
  expect({ items: [{ id: "bee" }] }).toHaveProperty(["items", 0, "id"], "bee");
  expect(error).toBeInstanceOf(CustomError);
  expect([1, 2]).toBeInstanceOf(Array);
  expect(payload).toMatchObject({ profile: { stats: { score: 7 } }, tags: ["runtime"] });
  expect(payload).not.toHaveProperty("profile.age");
  expect(payload).not.toMatchObject({ profile: { stats: { score: 9 } } });
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support object matcher chains. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "object matcher test should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_to_have_property_bracket_string_paths() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("property_bracket_paths.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports bracket string property paths", () => {
  const payload = {
    items: [
      { id: "bee", meta: { tags: ["runtime"] } }
    ],
    matrix: [[1, 2], [3, 4]],
    records: {
      "a.b": { value: 5 }
    }
  };

  expect(payload).toHaveProperty("items[0].id", "bee");
  expect(payload).toHaveProperty("items[0].meta.tags[0]", "runtime");
  expect(payload).toHaveProperty("matrix[1][0]", 3);
  expect(payload).toHaveProperty('records["a.b"].value', 5);
  expect(payload).not.toHaveProperty("items[1].id");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support bracket notation in toHaveProperty string paths. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "bracket property path test should pass. output: {combined}"
    );
}

#[test]
fn test_command_reports_object_matcher_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("object_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports object matcher mismatch", () => {
  expect({ profile: { name: "Ada" } }).toHaveProperty("profile.age", 42);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail an object matcher assertion. output: {combined}"
    );
    assert!(
        combined.contains(
            r#"Expected {"profile":{"name":"Ada"}} to have property "profile.age" with value 42"#
        ),
        "object matcher failure should explain the mismatch. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed object matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_reports_negated_matcher_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("negated_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports negated matcher failure", () => {
  expect(["bee"]).not.toContain("bee");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail a negated matcher assertion. output: {combined}"
    );
    assert!(
        combined.contains("Expected [\"bee\"] not to contain \"bee\""),
        "negated matcher failure should explain the mismatch. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed negated matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_fn_mock_tracking() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("mock_tracking.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports jest.fn tracking and implementations", () => {
  const mock = jest.fn((name) => `hello ${name}`)
    .mockReturnValueOnce("first")
    .mockImplementationOnce((name) => `second ${name}`);

  expect(mock("bee")).toBe("first");
  expect(mock("js")).toBe("second js");
  expect(mock("runtime")).toBe("hello runtime");

  expect(mock).toHaveBeenCalled();
  expect(mock).toHaveBeenCalledTimes(3);
  expect(mock).toHaveBeenCalledWith("bee");
  expect(mock).toHaveBeenCalledWith("js");
  expect(mock.mock.calls).toEqual([["bee"], ["js"], ["runtime"]]);
  expect(mock.mock.results.map((result) => result.value)).toEqual(["first", "second js", "hello runtime"]);

  mock.mockClear();
  expect(mock).not.toHaveBeenCalled();
  expect(mock.mock.calls).toHaveLength(0);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support jest.fn mock tracking. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "mock tracking test should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_fn_nth_and_last_call_matchers() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("mock_nth_last_call_matchers.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports nth and last call matchers", () => {
  const mock = jest.fn();
  mock("first", { id: 1 });
  mock("second", { id: 2 });
  mock("third", { id: 3, tags: ["runtime", "v8"] });

  expect(mock).toHaveBeenNthCalledWith(1, "first", { id: 1 });
  expect(mock).toHaveBeenNthCalledWith(3, "third", expect.objectContaining({
    id: 3,
    tags: expect.arrayContaining(["runtime"])
  }));
  expect(mock).toHaveBeenLastCalledWith("third", expect.objectContaining({ id: 3 }));
  expect(mock).not.toHaveBeenNthCalledWith(2, "first", expect.anything());
  expect(mock).not.toHaveBeenLastCalledWith("second", expect.anything());
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support nth and last mock call matchers. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "nth and last mock matcher test should pass. output: {combined}"
    );
}

#[test]
fn test_command_reports_nth_mock_call_matcher_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("mock_nth_call_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports nth call mismatch", () => {
  const mock = jest.fn();
  mock("first");
  mock("second");
  expect(mock).toHaveBeenNthCalledWith(2, "first");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail a nth mock call mismatch. output: {combined}"
    );
    assert!(
        combined.contains("Expected mock nth call 2"),
        "nth mock call mismatch should identify the call index. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed nth mock call matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_reports_last_mock_call_matcher_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("mock_last_call_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports last call mismatch", () => {
  const mock = jest.fn();
  mock("first");
  mock("second");
  expect(mock).toHaveBeenLastCalledWith("first");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail a last mock call mismatch. output: {combined}"
    );
    assert!(
        combined.contains("Expected mock last call"),
        "last mock call mismatch should identify the last call expectation. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed last mock call matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_fn_return_matchers() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("mock_return_matchers.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports mock return matchers", () => {
  const mock = jest.fn()
    .mockReturnValueOnce({ id: 1 })
    .mockImplementationOnce(() => {
      throw new Error("boom");
    })
    .mockReturnValueOnce({ id: 3, tags: ["runtime", "v8"] })
    .mockReturnValue({ id: 4 });

  expect(mock("a")).toEqual({ id: 1 });
  try {
    mock("b");
  } catch (error) {
    expect(error.message).toBe("boom");
  }
  expect(mock("c")).toEqual({ id: 3, tags: ["runtime", "v8"] });
  expect(mock("d")).toEqual({ id: 4 });

  expect(mock).toHaveReturned();
  expect(mock).toHaveReturnedTimes(3);
  expect(mock).toHaveReturnedWith(expect.objectContaining({ id: 3 }));
  expect(mock).toHaveLastReturnedWith({ id: 4 });
  expect(mock).toHaveNthReturnedWith(1, { id: 1 });
  expect(mock).toHaveNthReturnedWith(3, expect.objectContaining({
    tags: expect.arrayContaining(["runtime"])
  }));
  expect(mock).not.toHaveNthReturnedWith(2, expect.anything());
  expect(mock).not.toHaveReturnedWith({ id: 2 });
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support mock return matchers. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "mock return matcher test should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_fn_mock_return_this() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("mock_return_this.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports mockReturnThis for chainable methods", () => {
  const api = {
    step: jest.fn()
      .mockReturnValueOnce("first")
      .mockReturnThis()
      .mockName("step")
  };

  expect(api.step("once")).toBe("first");
  expect(api.step("chain")).toBe(api);
  expect(api.step).toHaveBeenCalledWith("chain");
  expect(api.step.mock.contexts).toEqual([api, api]);
  expect(api.step.getMockName()).toBe("step");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support jest.fn().mockReturnThis(). output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "mockReturnThis test should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_fn_with_implementation() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("mock_with_implementation.test.js");
    std::fs::write(
        &test_file,
        r#"
test("temporarily overrides mock implementation", async () => {
  const mock = jest.fn((value) => `base:${value}`);

  const syncResult = mock.withImplementation(
    (value) => `temp:${value}`,
    () => {
      expect(mock("sync")).toBe("temp:sync");
      return "sync result";
    }
  );

  expect(syncResult).toBe("sync result");
  expect(mock("after-sync")).toBe("base:after-sync");

  await mock.withImplementation(
    (value) => `async:${value}`,
    async () => {
      expect(mock("inside-async")).toBe("async:inside-async");
      await Promise.resolve();
      expect(mock("after-await")).toBe("async:after-await");
    }
  );

  expect(mock("after-async")).toBe("base:after-async");
  expect(mock.mock.calls).toEqual([
    ["sync"],
    ["after-sync"],
    ["inside-async"],
    ["after-await"],
    ["after-async"]
  ]);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support jest.fn().withImplementation(). output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "withImplementation test should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_fn_get_mock_implementation() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("mock_get_implementation.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reads the current mock implementation", () => {
  const initialImplementation = (value) => `base:${value}`;
  const mock = jest.fn(initialImplementation);

  expect(mock.getMockImplementation()).toBe(initialImplementation);
  expect(mock.getMockImplementation()("one")).toBe("base:one");

  mock.mockReturnValue("steady");
  expect(mock.getMockImplementation()("ignored")).toBe("steady");

  mock.mockReset();
  expect(mock.getMockImplementation()).toBeUndefined();
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support jest.fn().getMockImplementation(). output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "getMockImplementation test should pass. output: {combined}"
    );
}

#[test]
fn test_command_reports_mock_return_times_matcher_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("mock_return_times_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports return count mismatch", () => {
  const mock = jest.fn()
    .mockReturnValueOnce("first")
    .mockImplementationOnce(() => {
      throw new Error("boom");
    });
  mock();
  try {
    mock();
  } catch (error) {}
  expect(mock).toHaveReturnedTimes(2);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail a mock return count mismatch. output: {combined}"
    );
    assert!(
        combined.contains("Expected mock to have returned 2 times"),
        "mock return count mismatch should explain expected count. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed mock return count matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_reports_nth_mock_return_matcher_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("mock_nth_return_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports nth return mismatch", () => {
  const mock = jest.fn()
    .mockReturnValueOnce("first")
    .mockImplementationOnce(() => {
      throw new Error("boom");
    });
  mock();
  try {
    mock();
  } catch (error) {}
  expect(mock).toHaveNthReturnedWith(2, "second");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail a nth mock return mismatch. output: {combined}"
    );
    assert!(
        combined.contains("Expected mock nth return 2"),
        "nth mock return mismatch should identify the return index. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed nth mock return matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_fn_mock_matcher_aliases() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("mock_matcher_aliases.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports Jest mock matcher aliases", () => {
  const mock = jest.fn()
    .mockReturnValueOnce("first result")
    .mockReturnValueOnce("second result")
    .mockReturnValue("third result");

  expect(mock("first")).toBe("first result");
  expect(mock("second")).toBe("second result");
  expect(mock("third")).toBe("third result");

  expect(mock).toBeCalled();
  expect(mock).toBeCalledTimes(3);
  expect(mock).toBeCalledWith("second");
  expect(mock).nthCalledWith(1, "first");
  expect(mock).lastCalledWith("third");
  expect(mock).toReturn();
  expect(mock).toReturnTimes(3);
  expect(mock).toReturnWith("second result");
  expect(mock).nthReturnedWith(1, "first result");
  expect(mock).lastReturnedWith("third result");
  expect(mock).not.toBeCalledWith("missing");
  expect(mock).not.nthReturnedWith(2, "first result");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support Jest mock matcher aliases. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "mock matcher alias test should pass. output: {combined}"
    );
}

#[test]
fn test_command_reports_jest_mock_alias_matcher_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("mock_alias_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports Jest alias matcher mismatch", () => {
  const mock = jest.fn();
  mock("first");
  expect(mock).lastCalledWith("second");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail a Jest mock alias mismatch. output: {combined}"
    );
    assert!(
        combined.contains("Expected mock last call"),
        "Jest mock alias mismatch should use the underlying matcher diagnostic. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed Jest mock alias matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_fn_mock_name_and_last_call() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("mock_name_last_call.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports Jest mock names and lastCall metadata", () => {
  const mock = jest.fn((value) => value);

  expect(mock.getMockName()).toBe("jest.fn()");
  expect(mock.mock.lastCall).toBeUndefined();
  expect(mock.mockName("fetchBee")).toBe(mock);
  expect(mock.getMockName()).toBe("fetchBee");

  expect(mock("first")).toBe("first");
  expect(mock("second", { ok: true })).toBe("second");
  expect(mock.mock.lastCall).toEqual(["second", { ok: true }]);

  mock.mockClear();
  expect(mock.mock.lastCall).toBeUndefined();
  expect(mock.getMockName()).toBe("fetchBee");

  mock("after-clear");
  expect(mock.mock.lastCall).toEqual(["after-clear"]);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support Jest mock names and lastCall metadata. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "mock name and lastCall test should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_is_mock_function_and_contexts() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("mock_contexts.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports Jest mock function detection and contexts", () => {
  const plain = () => {};
  const firstContext = { label: "first" };
  const secondContext = { label: "second" };
  const mock = jest.fn(function (value) {
    return `${this.label}:${value}`;
  });

  expect(jest.isMockFunction(mock)).toBe(true);
  expect(jest.isMockFunction(plain)).toBe(false);
  expect(jest.isMockFunction({ _isMockFunction: true })).toBe(false);
  expect(mock.mock.contexts).toEqual([]);

  expect(mock.call(firstContext, "a")).toBe("first:a");
  expect(mock.apply(secondContext, ["b"])).toBe("second:b");
  expect(mock.mock.contexts).toEqual([firstContext, secondContext]);

  mock.mockClear();
  expect(mock.mock.contexts).toEqual([]);

  mock.call(firstContext, "c");
  expect(mock.mock.contexts).toEqual([firstContext]);

  mock.mockReset();
  expect(mock.mock.contexts).toEqual([]);
  expect(jest.isMockFunction(mock)).toBe(true);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support Jest mock function detection and contexts. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "mock function detection and contexts test should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_spy_on_and_restore_all_mocks() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("spy_on_restore.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports Jest spyOn and restore helpers", () => {
  const service = {
    calls: [],
    greet(name) {
      this.calls.push(name);
      return `hello ${name}`;
    },
  };

  const greetSpy = jest.spyOn(service, "greet");
  expect(jest.isMockFunction(service.greet)).toBe(true);
  expect(service.greet("Ada")).toBe("hello Ada");
  expect(greetSpy).toHaveBeenCalledWith("Ada");
  expect(greetSpy.mock.contexts).toEqual([service]);

  greetSpy.mockImplementation(function (name) {
    return `mock ${this.calls.length}:${name}`;
  });
  expect(service.greet("Bee")).toBe("mock 1:Bee");
  expect(service.calls).toEqual(["Ada"]);

  greetSpy.mockRestore();
  expect(jest.isMockFunction(service.greet)).toBe(false);
  expect(service.greet("Grace")).toBe("hello Grace");
  expect(service.calls).toEqual(["Ada", "Grace"]);

  const first = { value() { return "first"; } };
  const second = { value() { return "second"; } };
  jest.spyOn(first, "value").mockReturnValue("mock first");
  jest.spyOn(second, "value").mockReturnValue("mock second");

  expect(first.value()).toBe("mock first");
  expect(second.value()).toBe("mock second");
  jest.restoreAllMocks();
  expect(first.value()).toBe("first");
  expect(second.value()).toBe("second");
  expect(jest.isMockFunction(first.value)).toBe(false);
  expect(jest.isMockFunction(second.value)).toBe(false);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support Jest spyOn and restore helpers. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "spyOn and restore helper test should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_spy_on_getter_accessors() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("spy_on_getter.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports Jest spyOn getter accessors", () => {
  const service = {
    calls: [],
    get value() {
      this.calls.push("get");
      return `real:${this.calls.length}`;
    },
  };
  const originalDescriptor = Object.getOwnPropertyDescriptor(service, "value");

  const spy = jest.spyOn(service, "value", "get");
  expect(jest.isMockFunction(spy)).toBe(true);
  expect(service.value).toBe("real:1");
  expect(spy).toHaveBeenCalledTimes(1);
  expect(spy.mock.contexts.length).toBe(1);
  expect(spy.mock.contexts[0]).toBe(service);

  spy.mockReturnValue("mocked");
  expect(service.value).toBe("mocked");
  expect(spy).toHaveBeenCalledTimes(2);

  spy.mockRestore();
  expect(service.value).toBe("real:2");
  expect(Object.getOwnPropertyDescriptor(service, "value").get).toBe(originalDescriptor.get);
});

test("restoreAllMocks restores getter spies", () => {
  const config = {
    get mode() {
      return "real";
    },
  };

  jest.spyOn(config, "mode", "get").mockReturnValue("mock");
  expect(config.mode).toBe("mock");
  jest.restoreAllMocks();
  expect(config.mode).toBe("real");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support Jest spyOn getter accessors. output: {combined}"
    );
    assert!(
        combined.contains("2 passed, 0 failed, 0 skipped"),
        "spyOn getter accessor tests should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_spy_on_setter_accessors() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("spy_on_setter.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports Jest spyOn setter accessors", () => {
  const service = {
    calls: [],
    current: "initial",
    set value(next) {
      this.calls.push(`real:${next}`);
      this.current = next;
    },
    get value() {
      return this.current;
    },
  };
  const originalDescriptor = Object.getOwnPropertyDescriptor(service, "value");

  const spy = jest.spyOn(service, "value", "set");
  expect(jest.isMockFunction(spy)).toBe(true);
  service.value = "first";
  expect(service.value).toBe("first");
  expect(service.calls).toEqual(["real:first"]);
  expect(spy).toHaveBeenCalledWith("first");
  expect(spy.mock.contexts.length).toBe(1);
  expect(spy.mock.contexts[0]).toBe(service);

  spy.mockImplementation(function (next) {
    this.calls.push(`mock:${next}`);
    this.current = "mocked";
  });
  service.value = "second";
  expect(service.value).toBe("mocked");
  expect(service.calls).toEqual(["real:first", "mock:second"]);
  expect(spy).toHaveBeenCalledTimes(2);

  spy.mockRestore();
  service.value = "third";
  expect(service.value).toBe("third");
  expect(service.calls).toEqual(["real:first", "mock:second", "real:third"]);
  expect(Object.getOwnPropertyDescriptor(service, "value").set).toBe(originalDescriptor.set);
});

test("restoreAllMocks restores setter spies", () => {
  const config = {
    mode: "real",
    set value(next) {
      this.mode = next;
    },
  };

  jest.spyOn(config, "value", "set").mockImplementation(function (next) {
    this.mode = `mock:${next}`;
  });
  config.value = "test";
  expect(config.mode).toBe("mock:test");
  jest.restoreAllMocks();
  config.value = "prod";
  expect(config.mode).toBe("prod");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support Jest spyOn setter accessors. output: {combined}"
    );
    assert!(
        combined.contains("2 passed, 0 failed, 0 skipped"),
        "spyOn setter accessor tests should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_replace_property_and_restore_all_mocks() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("replace_property.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports replaceProperty and restore helpers", () => {
  const config = { mode: "real", env: { HOSTNAME: "original" } };

  const replacedMode = jest.replaceProperty(config, "mode", "mocked");
  expect(config.mode).toBe("mocked");
  expect(replacedMode.replaceValue("test")).toBe(replacedMode);
  expect(config.mode).toBe("test");
  replacedMode.restore();
  expect(config.mode).toBe("real");

  jest.replaceProperty(config.env, "HOSTNAME", "localhost");
  expect(config.env.HOSTNAME).toBe("localhost");
  jest.restoreAllMocks();
  expect(config.env.HOSTNAME).toBe("original");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support jest.replaceProperty and restoreAllMocks. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "replaceProperty restore test should pass. output: {combined}"
    );
}

#[test]
fn test_command_reports_jest_replace_property_missing_property() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("replace_property_missing.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports missing property replacement", () => {
  expect(() => jest.replaceProperty({}, "missing", 1)).toThrow("Property missing does not exist");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should report replaceProperty missing property through toThrow. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "replaceProperty missing property assertion should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_fn_async_mock_helpers() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("async_mock_helpers.test.js");
    std::fs::write(
        &test_file,
        r#"
test("supports async jest.fn helpers", async () => {
  const mock = jest.fn()
    .mockResolvedValueOnce("first")
    .mockRejectedValueOnce(new Error("boom"))
    .mockResolvedValue("steady");

  await expect(mock("a")).resolves.toBe("first");
  await expect(mock("b")).rejects.toThrow("boom");
  await expect(mock("c")).resolves.toBe("steady");
  await expect(mock("d")).resolves.toBe("steady");

  expect(mock).toHaveBeenCalledTimes(4);
  expect(mock.mock.calls).toEqual([["a"], ["b"], ["c"], ["d"]]);

  mock.mockReset().mockRejectedValue("blocked");
  await expect(mock("e")).rejects.toBe("blocked");
  expect(mock).toHaveBeenCalledWith("e");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support async jest.fn mock helpers. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "async mock helper test should pass. output: {combined}"
    );
}

#[test]
fn test_command_exposes_jest_fn_to_required_helpers() {
    let dir = tempdir().expect("failed to create tempdir");
    let helper_file = dir.path().join("mock_helper.js");
    std::fs::write(
        &helper_file,
        r#"
exports.createMock = function createMock() {
  return globalThis.jest.fn((value) => value + 1);
};
"#,
    )
    .expect("failed to write helper file");
    let test_file = dir.path().join("mock_helper.test.js");
    std::fs::write(
        &test_file,
        r#"
const { createMock } = require("./mock_helper");

test("required helper can create jest mocks", () => {
  const mock = createMock();
  expect(mock(41)).toBe(42);
  expect(mock).toHaveBeenCalledWith(41);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should expose jest.fn to required helper modules. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "helper mock test should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_reset_modules_for_required_helpers() {
    let dir = tempdir().expect("failed to create tempdir");
    std::fs::write(
        dir.path().join("counter.js"),
        r#"
globalThis.__counterLoads = (globalThis.__counterLoads || 0) + 1;
module.exports = { loads: globalThis.__counterLoads };
"#,
    )
    .expect("failed to write helper module");
    let test_file = dir.path().join("reset_modules.test.js");
    std::fs::write(
        &test_file,
        r#"
test("resetModules clears required helper cache", () => {
  const first = require("./counter");
  const second = require("./counter");
  expect(second).toBe(first);
  expect(first.loads).toBe(1);

  expect(jest.resetModules()).toBe(jest);
  const third = require("./counter");
  expect(third).not.toBe(first);
  expect(third.loads).toBe(2);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support jest.resetModules for required helpers. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "resetModules helper cache test should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_isolate_modules_for_required_helpers() {
    let dir = tempdir().expect("failed to create tempdir");
    std::fs::write(
        dir.path().join("counter.js"),
        r#"
globalThis.__isolateLoads = (globalThis.__isolateLoads || 0) + 1;
module.exports = { loads: globalThis.__isolateLoads };
"#,
    )
    .expect("failed to write helper module");
    std::fs::write(
        dir.path().join("esm_counter.mjs"),
        r#"
globalThis.__esmIsolateLoads = (globalThis.__esmIsolateLoads || 0) + 1;
export const loads = globalThis.__esmIsolateLoads;
"#,
    )
    .expect("failed to write ESM helper module");
    let test_file = dir.path().join("isolate_modules.test.js");
    std::fs::write(
        &test_file,
        r#"
test("isolateModules uses a sandboxed helper cache", () => {
  const outerFirst = require("./counter");
  const outerSecond = require("./counter");
  expect(outerSecond).toBe(outerFirst);
  expect(outerFirst.loads).toBe(1);

  let isolated;
  jest.isolateModules(() => {
    isolated = require("./counter");
    expect(isolated).not.toBe(outerFirst);
    expect(isolated.loads).toBe(2);
    expect(require("./counter")).toBe(isolated);
  });

  const outerAfter = require("./counter");
  expect(outerAfter).toBe(outerFirst);
  expect(outerAfter.loads).toBe(1);
});

test("isolateModules restores the outer cache after callback errors", () => {
  const outer = require("./counter");
  expect(() => jest.isolateModules(() => {
    require("./counter");
    throw new Error("planned isolate failure");
  })).toThrow("planned isolate failure");
  expect(require("./counter")).toBe(outer);
});

test("isolateModules also sandboxes the ESM namespace bridge cache", () => {
  const outerFirst = require("./esm_counter.mjs");
  const outerSecond = require("./esm_counter.mjs");
  expect(outerSecond).toBe(outerFirst);
  expect(outerFirst.loads).toBe(1);

  let isolated;
  jest.isolateModules(() => {
    isolated = require("./esm_counter.mjs");
    expect(isolated).not.toBe(outerFirst);
    expect(isolated.loads).toBe(2);
    expect(require("./esm_counter.mjs")).toBe(isolated);
  });

  const outerAfter = require("./esm_counter.mjs");
  expect(outerAfter).toBe(outerFirst);
  expect(outerAfter.loads).toBe(1);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support jest.isolateModules for required helpers. output: {combined}"
    );
    assert!(
        combined.contains("3 passed, 0 failed, 0 skipped"),
        "isolateModules helper cache tests should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_isolate_modules_async_for_required_helpers() {
    let dir = tempdir().expect("failed to create tempdir");
    std::fs::write(
        dir.path().join("counter.js"),
        r#"
globalThis.__asyncIsolateLoads = (globalThis.__asyncIsolateLoads || 0) + 1;
module.exports = { loads: globalThis.__asyncIsolateLoads };
"#,
    )
    .expect("failed to write helper module");
    let test_file = dir.path().join("isolate_modules_async.test.js");
    std::fs::write(
        &test_file,
        r#"
test("isolateModulesAsync restores the outer cache after an awaited callback", async () => {
  const outerFirst = require("./counter");
  const outerSecond = require("./counter");
  expect(outerSecond).toBe(outerFirst);
  expect(outerFirst.loads).toBe(1);

  let isolated;
  const returned = await jest.isolateModulesAsync(async () => {
    await Promise.resolve();
    isolated = require("./counter");
    expect(isolated).not.toBe(outerFirst);
    expect(isolated.loads).toBe(2);
    expect(require("./counter")).toBe(isolated);
  });

  expect(returned).toBe(jest);
  expect(require("./counter")).toBe(outerFirst);
});

test("isolateModulesAsync restores the outer cache after callback rejection", async () => {
  const outer = require("./counter");
  let caught = false;
  try {
    await jest.isolateModulesAsync(async () => {
      require("./counter");
      await Promise.resolve();
      throw new Error("planned async isolate failure");
    });
  } catch (error) {
    caught = true;
    expect(error.message).toBe("planned async isolate failure");
  }

  expect(caught).toBe(true);
  expect(require("./counter")).toBe(outer);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support jest.isolateModulesAsync for required helpers. output: {combined}"
    );
    assert!(
        combined.contains("2 passed, 0 failed, 0 skipped"),
        "isolateModulesAsync helper cache tests should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_do_mock_and_require_actual_for_required_helpers() {
    let dir = tempdir().expect("failed to create tempdir");
    std::fs::write(
        dir.path().join("config.js"),
        r#"
globalThis.__configLoads = (globalThis.__configLoads || 0) + 1;
module.exports = { mode: "real", loads: globalThis.__configLoads };
"#,
    )
    .expect("failed to write config helper module");
    std::fs::write(
        dir.path().join("reader.js"),
        r#"
exports.readMode = function readMode() {
  return require("./config").mode;
};
"#,
    )
    .expect("failed to write reader helper module");
    let test_file = dir.path().join("do_mock.test.js");
    std::fs::write(
        &test_file,
        r#"
test("doMock returns factory exports while requireActual bypasses the mock", () => {
  let factoryCalls = 0;
  expect(jest.doMock("./config", () => {
    factoryCalls += 1;
    const actual = jest.requireActual("./config");
    return { mode: "mock", actualMode: actual.mode, actualLoads: actual.loads };
  })).toBe(jest);

  expect(factoryCalls).toBe(0);
  const first = require("./config");
  const second = require("./config");
  expect(second).toBe(first);
  expect(factoryCalls).toBe(1);
  expect(first).toEqual({ mode: "mock", actualMode: "real", actualLoads: 1 });

  const actual = jest.requireActual("./config");
  expect(actual).not.toBe(first);
  expect(actual.mode).toBe("real");
  expect(actual.loads).toBe(1);
});

test("doMock is visible to helper modules with relative require", () => {
  jest.resetModules();
  jest.doMock("./config", () => ({ mode: "mocked for reader" }));
  const reader = require("./reader");
  expect(reader.readMode()).toBe("mocked for reader");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support jest.doMock and jest.requireActual for required helpers. output: {combined}"
    );
    assert!(
        combined.contains("2 passed, 0 failed, 0 skipped"),
        "doMock and requireActual helper tests should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_require_mock_and_unmock_for_required_helpers() {
    let dir = tempdir().expect("failed to create tempdir");
    std::fs::write(
        dir.path().join("config.js"),
        r#"
globalThis.__requireMockConfigLoads = (globalThis.__requireMockConfigLoads || 0) + 1;
module.exports = { mode: "real", loads: globalThis.__requireMockConfigLoads };
"#,
    )
    .expect("failed to write config helper module");
    let test_file = dir.path().join("require_mock.test.js");
    std::fs::write(
        &test_file,
        r#"
test("requireMock materializes the registered mock and shares it with require", () => {
  let factoryCalls = 0;
  jest.doMock("./config", () => {
    factoryCalls += 1;
    return { mode: "mock", calls: factoryCalls };
  });

  expect(factoryCalls).toBe(0);
  const fromRequireMock = jest.requireMock("./config");
  expect(fromRequireMock).toEqual({ mode: "mock", calls: 1 });
  expect(factoryCalls).toBe(1);
  expect(require("./config")).toBe(fromRequireMock);
  expect(jest.requireMock("./config")).toBe(fromRequireMock);
  expect(factoryCalls).toBe(1);
});

test("unmock and dontMock remove explicit module mocks", () => {
  jest.resetModules();
  expect(jest.doMock("./config", () => ({ mode: "mock once" }))).toBe(jest);
  expect(require("./config").mode).toBe("mock once");

  expect(jest.unmock("./config")).toBe(jest);
  const actualAfterUnmock = require("./config");
  expect(actualAfterUnmock.mode).toBe("real");
  expect(actualAfterUnmock.loads).toBe(1);

  jest.resetModules();
  jest.doMock("./config", () => ({ mode: "mock twice" }));
  expect(require("./config").mode).toBe("mock twice");
  expect(jest.dontMock("./config")).toBe(jest);
  const actualAfterDontMock = require("./config");
  expect(actualAfterDontMock.mode).toBe("real");
  expect(actualAfterDontMock.loads).toBe(2);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support jest.requireMock and jest.unmock for required helpers. output: {combined}"
    );
    assert!(
        combined.contains("2 passed, 0 failed, 0 skipped"),
        "requireMock and unmock helper tests should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_mock_factory_for_required_helpers() {
    let dir = tempdir().expect("failed to create tempdir");
    std::fs::write(
        dir.path().join("config.js"),
        r#"
module.exports = { mode: "real" };
"#,
    )
    .expect("failed to write config helper module");
    std::fs::write(
        dir.path().join("reader.js"),
        r#"
exports.readMode = function readMode() {
  return require("./config").mode;
};
"#,
    )
    .expect("failed to write reader helper module");
    let test_file = dir.path().join("mock_factory.test.js");
    std::fs::write(
        &test_file,
        r#"
test("mock registers an explicit factory before require", () => {
  expect(jest.mock("./config", () => ({ mode: "mock factory" }))).toBe(jest);
  expect(require("./config").mode).toBe("mock factory");
  expect(require("./reader").readMode()).toBe("mock factory");
  expect(jest.requireActual("./config").mode).toBe("real");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support jest.mock explicit factories for required helpers. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "jest.mock factory helper test should pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_set_mock_for_required_helpers() {
    let dir = tempdir().expect("failed to create tempdir");
    std::fs::write(
        dir.path().join("config.js"),
        r#"
module.exports = { mode: "real" };
"#,
    )
    .expect("failed to write config helper module");
    let test_file = dir.path().join("set_mock.test.js");
    std::fs::write(
        &test_file,
        r#"
test("setMock registers explicit module exports", () => {
  const exportsObject = { mode: "manual mock" };
  expect(jest.setMock("./config", exportsObject)).toBe(jest);
  expect(jest.requireMock("./config")).toBe(exportsObject);
  expect(require("./config")).toBe(exportsObject);

  exportsObject.mode = "mutated mock";
  expect(require("./config").mode).toBe("mutated mock");

  jest.unmock("./config");
  expect(require("./config").mode).toBe("real");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should support jest.setMock for required helpers. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "jest.setMock helper test should pass. output: {combined}"
    );
}

#[test]
fn test_command_reports_mock_matcher_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("mock_matcher_failure.test.js");
    std::fs::write(
        &test_file,
        r#"
test("reports mock matcher failure", () => {
  const mock = jest.fn();
  mock("bee");
  expect(mock).not.toHaveBeenCalledWith("bee");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail a mock matcher assertion. output: {combined}"
    );
    assert!(
        combined.contains("Expected mock not to have been called with [\"bee\"]"),
        "mock matcher failure should explain the mismatch. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed mock matcher must not report success. output: {combined}"
    );
}

#[test]
fn test_command_runs_before_each_and_after_each_around_each_file_test() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("hooks.test.js");
    std::fs::write(
        &test_file,
        r#"
let events = [];

beforeEach(() => {
  events.push("before");
});

afterEach(() => {
  events.push("after");
});

test("first", () => {
  events.push("first");
  expect(events.join(",")).toBe("before,first");
});

test("second", () => {
  events.push("second");
  expect(events.join(",")).toBe("before,first,after,before,second");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should run beforeEach/afterEach around each file test. output: {combined}"
    );
    assert!(
        combined.contains("2 passed, 0 failed, 0 skipped"),
        "hooked tests should both pass. output: {combined}"
    );
}

#[test]
fn test_command_supports_done_callback_file_hooks() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("done_callback_hooks.test.js");
    std::fs::write(
        &test_file,
        r#"
let events = [];

beforeAll((done) => {
  setTimeout(() => {
    events.push("beforeAll");
    done();
  }, 1);
});

beforeEach((done) => {
  setTimeout(() => {
    events.push("beforeEach");
    done();
  }, 1);
});

afterEach((done) => {
  setTimeout(() => {
    events.push("afterEach");
    done();
  }, 1);
});

afterAll((done) => {
  setTimeout(() => {
    try {
      events.push("afterAll");
      expect(events.join("|")).toBe("beforeAll|beforeEach|test|afterEach|afterAll");
      done();
    } catch (error) {
      done(error);
    }
  }, 1);
});

test("waits for hook done callbacks", () => {
  events.push("test");
  expect(events.join("|")).toBe("beforeAll|beforeEach|test");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .args(["test", "--timeout", "1"])
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should wait for lifecycle hook done callbacks. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "done callback hook test should pass after hooks complete. output: {combined}"
    );
}

#[test]
fn test_command_reports_done_callback_hook_error() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("done_callback_hook_error.test.js");
    std::fs::write(
        &test_file,
        r#"
beforeEach((done) => {
  done(new Error("hook done failed"));
});

test("does not run after failing hook", () => {
  throw new Error("test body should not run");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .args(["test", "--timeout", "1"])
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail when a lifecycle hook done callback receives an error. output: {combined}"
    );
    assert!(
        combined.contains("hook done failed"),
        "hook done callback error should be reported. output: {combined}"
    );
    assert!(
        !combined.contains("test body should not run"),
        "test body should not run after a failing beforeEach hook. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "failed hook done callback must not report success. output: {combined}"
    );
}

#[test]
fn test_command_reports_done_callback_hook_promise_conflict() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir
        .path()
        .join("done_callback_hook_promise_conflict.test.js");
    std::fs::write(
        &test_file,
        r#"
beforeEach((done) => {
  return Promise.resolve().then(() => done());
});

test("does not mix hook done and promise", () => {
  expect(true).toBe(true);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .args(["test", "--timeout", "1"])
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail when a hook mixes done callback and Promise. output: {combined}"
    );
    assert!(
        combined.contains("cannot both use done callback and return a Promise"),
        "hook done/promise conflict should be reported. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "hook done/promise conflict must not report success. output: {combined}"
    );
}

#[test]
fn test_command_runs_describe_scoped_each_hooks_in_jest_order() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("describe_hooks.test.js");
    std::fs::write(
        &test_file,
        r#"
let events = [];

beforeEach(() => events.push("outer before"));
afterEach(() => events.push("outer after"));

describe("suite", () => {
  beforeEach(() => events.push("inner before"));
  afterEach(() => events.push("inner after"));

  test("inner test", () => {
    events.push("test");
    expect(events.join("|")).toBe("outer before|inner before|test");
  });
});

test("observes previous hook order", () => {
  expect(events.join("|")).toBe("outer before|inner before|test|inner after|outer after|outer before");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should run outer/inner hooks in Jest order. output: {combined}"
    );
}

#[test]
fn test_command_describe_skip_skips_file_suite_tests_and_hooks() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("describe_skip.test.js");
    std::fs::write(
        &test_file,
        r#"
describe.skip("skipped suite", () => {
  beforeAll(() => {
    throw new Error("skipped beforeAll should not run");
  });

  afterAll(() => {
    throw new Error("skipped afterAll should not run");
  });

  test("skipped test", () => {
    throw new Error("skipped test should not run");
  });
});

test("outside test still runs", () => {
  console.log("ran outside");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "describe.skip should skip suite tests and hooks without failing. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 1 skipped"),
        "describe.skip should count skipped suite tests. output: {combined}"
    );
    assert!(
        combined.contains("ran outside"),
        "tests outside describe.skip should still run. output: {combined}"
    );
    assert!(
        !combined.contains("skipped test should not run")
            && !combined.contains("skipped beforeAll should not run")
            && !combined.contains("skipped afterAll should not run"),
        "describe.skip must not execute skipped suite bodies or hooks. output: {combined}"
    );
}

#[test]
fn test_command_describe_only_runs_only_file_suite_tests() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("describe_only.test.js");
    std::fs::write(
        &test_file,
        r#"
describe.only("focused suite", () => {
  test("focused test", () => {
    console.log("ran focused");
  });
});

test("outside test", () => {
  throw new Error("outside test should not run");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "describe.only should focus suite tests. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 1 skipped"),
        "describe.only should skip tests outside the focused suite. output: {combined}"
    );
    assert!(
        combined.contains("ran focused"),
        "focused suite test should run. output: {combined}"
    );
    assert!(
        !combined.contains("outside test should not run"),
        "tests outside describe.only should not execute. output: {combined}"
    );
}

#[test]
fn test_command_describe_skip_suppresses_nested_only() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("describe_skip_only.test.js");
    std::fs::write(
        &test_file,
        r#"
describe.skip("skipped suite", () => {
  test.only("nested only should be ignored", () => {
    throw new Error("nested only in skipped suite should not run");
  });
});

test("outside normal test", () => {
  console.log("ran normal outside skipped only");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "test.only inside describe.skip should not focus the file. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 1 skipped"),
        "skipped only test should not suppress outside normal tests. output: {combined}"
    );
    assert!(
        combined.contains("ran normal outside skipped only"),
        "outside normal test should run. output: {combined}"
    );
    assert!(
        !combined.contains("nested only in skipped suite should not run"),
        "nested test.only inside describe.skip must not execute. output: {combined}"
    );
}

#[test]
fn test_command_runs_after_each_after_failed_file_test() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("failed_after_each.test.js");
    std::fs::write(
        &test_file,
        r#"
let cleaned = false;

afterEach(() => {
  cleaned = true;
});

test("fails but still runs cleanup", () => {
  throw new Error("intentional failure");
});

test("sees cleanup from failed test", () => {
  if (!cleaned) {
    throw new Error("cleanup missing");
  }
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should still fail because the first test fails. output: {combined}"
    );
    assert!(
        combined.contains("intentional failure"),
        "original failure should be reported. output: {combined}"
    );
    assert!(
        !combined.contains("cleanup missing"),
        "afterEach should run even after a failing test. output: {combined}"
    );
}

#[test]
fn test_command_runs_before_all_and_after_all_once_for_file_tests() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("all_hooks.test.js");
    std::fs::write(
        &test_file,
        r#"
let events = [];

beforeAll(() => {
  events.push("beforeAll");
});

afterAll(() => {
  events.push("afterAll");
  expect(events.join("|")).toBe("beforeAll|first|second|afterAll");
});

test("first", () => {
  events.push("first");
  expect(events.join("|")).toBe("beforeAll|first");
});

test("second", () => {
  events.push("second");
  expect(events.join("|")).toBe("beforeAll|first|second");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should run beforeAll/afterAll once around file tests. output: {combined}"
    );
    assert!(
        combined.contains("2 passed, 0 failed, 0 skipped"),
        "file-level all hooks should not create extra tests. output: {combined}"
    );
}

#[test]
fn test_command_before_all_failure_blocks_remaining_suite_tests() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("before_all_failure_blocks_suite.test.js");
    std::fs::write(
        &test_file,
        r#"
describe("suite", () => {
  beforeAll(() => {
    console.log("suite setup started");
    throw new Error("setup failed");
  });

  afterAll(() => {
    console.log("inner cleanup ran");
  });

  test("blocked first", () => {
    console.log("first should not run");
  });

  test("blocked second", () => {
    console.log("second should not run");
  });
});

test("outer continues", () => {
  console.log("outer continued");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail because beforeAll failed. output: {combined}"
    );
    assert!(
        combined.contains("suite beforeAll: setup failed"),
        "beforeAll failure should identify the failed suite. output: {combined}"
    );
    assert!(
        combined.contains("inner cleanup ran"),
        "afterAll should run for the started failed suite. output: {combined}"
    );
    assert!(
        combined.contains("outer continued"),
        "unrelated tests outside the failed suite should still run. output: {combined}"
    );
    assert!(
        !combined.contains("first should not run") && !combined.contains("second should not run"),
        "tests in a suite with failed beforeAll must not execute. output: {combined}"
    );
}

#[test]
fn test_command_runs_describe_scoped_all_hooks_before_following_file_tests() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("describe_all_hooks.test.js");
    std::fs::write(
        &test_file,
        r#"
let events = [];

beforeAll(() => events.push("outer beforeAll"));
afterAll(() => {
  events.push("outer afterAll");
  expect(events.join("|")).toBe(
    "outer beforeAll|inner beforeAll|inner test|inner afterAll|outer test|outer afterAll"
  );
});

describe("suite", () => {
  beforeAll(() => events.push("inner beforeAll"));
  afterAll(() => {
    events.push("inner afterAll");
    expect(events.join("|")).toBe("outer beforeAll|inner beforeAll|inner test|inner afterAll");
  });

  test("inner test", () => {
    events.push("inner test");
    expect(events.join("|")).toBe("outer beforeAll|inner beforeAll|inner test");
  });
});

test("outer test after suite", () => {
  events.push("outer test");
  expect(events.join("|")).toBe("outer beforeAll|inner beforeAll|inner test|inner afterAll|outer test");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should run describe-scoped beforeAll/afterAll in suite order. output: {combined}"
    );
}

#[test]
fn test_command_runs_after_all_after_failed_file_test() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("failed_after_all.test.js");
    std::fs::write(
        &test_file,
        r#"
let cleaned = false;

afterAll(() => {
  cleaned = true;
});

test("fails but still runs final cleanup", () => {
  throw new Error("intentional failure");
});

test("sees final cleanup after all tests", () => {
  expect(cleaned).toBe(false);
});

afterAll(() => {
  if (!cleaned) {
    throw new Error("final cleanup missing");
  }
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should still fail because the first test fails. output: {combined}"
    );
    assert!(
        combined.contains("intentional failure"),
        "original failure should be reported. output: {combined}"
    );
    assert!(
        !combined.contains("final cleanup missing"),
        "afterAll should run even after a failing test. output: {combined}"
    );
}

#[test]
fn run_passes_script_args_to_process_argv() {
    let dir = tempdir().expect("failed to create tempdir");
    let script = dir.path().join("argv.js");
    std::fs::write(
        &script,
        r#"
console.log(JSON.stringify(process.argv.slice(2)));
"#,
    )
    .expect("failed to write script");

    let output = Command::new(bee_path())
        .arg("run")
        .arg(&script)
        .args(["alpha", "beta"])
        .output()
        .expect("failed to execute bee run");

    assert!(
        output.status.success(),
        "bee run should exit successfully. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        r#"["alpha","beta"]"#
    );
}

#[test]
fn run_accepts_double_dash_before_script_args() {
    let dir = tempdir().expect("failed to create tempdir");
    let script = dir.path().join("argv_dash.js");
    std::fs::write(
        &script,
        r#"
console.log(JSON.stringify(process.argv.slice(2)));
"#,
    )
    .expect("failed to write script");

    let output = Command::new(bee_path())
        .arg("run")
        .arg(&script)
        .args(["--", "alpha", "--flag", "value"])
        .output()
        .expect("failed to execute bee run");

    assert!(
        output.status.success(),
        "bee run should accept -- before script args. stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        r#"["alpha","--flag","value"]"#
    );
}

#[test]
fn run_preload_file_resolves_relative_require_from_preload_dir() {
    let dir = tempdir().expect("failed to create tempdir");
    let preload_dir = dir.path().join("preload");
    let app_dir = dir.path().join("app");
    std::fs::create_dir_all(&preload_dir).expect("failed to create preload dir");
    std::fs::create_dir_all(&app_dir).expect("failed to create app dir");

    std::fs::write(
        preload_dir.join("helper.js"),
        "module.exports = { value: 'from-preload-helper' };",
    )
    .expect("failed to write preload helper");
    let preload = preload_dir.join("setup.js");
    std::fs::write(
        &preload,
        "globalThis.__preloadValue = require('./helper').value;",
    )
    .expect("failed to write preload file");
    let main = app_dir.join("main.js");
    std::fs::write(
        &main,
        "console.log(globalThis.__preloadValue || 'missing-preload');",
    )
    .expect("failed to write main script");

    let output = Command::new(bee_path())
        .arg("run")
        .arg("--preload")
        .arg(&preload)
        .arg(&main)
        .output()
        .expect("failed to execute bee run");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee run with preload should exit successfully. output: {combined}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "from-preload-helper",
        "preload file should resolve relative require from its own directory. output: {combined}"
    );
}

#[test]
fn run_deny_fs_fails_closed_when_preload_read_is_denied() {
    let dir = tempdir().expect("failed to create tempdir");
    let preload_dir = dir.path().join("preload");
    let app_dir = dir.path().join("app");
    std::fs::create_dir_all(&preload_dir).expect("failed to create preload dir");
    std::fs::create_dir_all(&app_dir).expect("failed to create app dir");

    let preload = preload_dir.join("setup.js");
    std::fs::write(&preload, "globalThis.__preloadRan = true;")
        .expect("failed to write preload file");
    let main = app_dir.join("main.js");
    std::fs::write(&main, "console.log('MAIN_RAN');").expect("failed to write main script");

    let output = Command::new(bee_path())
        .args(["run", "--deny-fs", "--allow-read"])
        .arg(&main)
        .arg("--preload")
        .arg(&preload)
        .arg(&main)
        .output()
        .expect("failed to execute bee run");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "denied preload read should fail bee run. output: {combined}"
    );
    assert!(
        combined.contains("permission denied"),
        "denied preload should report broker denial. output: {combined}"
    );
    assert!(
        !combined.contains("MAIN_RAN"),
        "main script must not execute after preload denial. output: {combined}"
    );
}

#[test]
fn run_preload_helper_sees_require_main_as_entry_module() {
    let dir = tempdir().expect("failed to create tempdir");
    let preload_dir = dir.path().join("preload");
    let app_dir = dir.path().join("app");
    std::fs::create_dir_all(&preload_dir).expect("failed to create preload dir");
    std::fs::create_dir_all(&app_dir).expect("failed to create app dir");

    std::fs::write(
        preload_dir.join("helper.js"),
        r#"
module.exports = {
  isHelperMain: require.main === module,
  mainFilename: require.main && require.main.filename,
};
"#,
    )
    .expect("failed to write preload helper");
    let preload = preload_dir.join("setup.js");
    std::fs::write(
        &preload,
        "globalThis.__preloadMainInfo = require('./helper');",
    )
    .expect("failed to write preload file");
    let main = app_dir.join("main.js");
    std::fs::write(
        &main,
        r#"
const preloadInfo = globalThis.__preloadMainInfo;
console.log(JSON.stringify({
  entryMain: require.main === module,
  helperMain: preloadInfo.isHelperMain,
  helperSawEntry: typeof preloadInfo.mainFilename === "string" &&
    preloadInfo.mainFilename.endsWith("main.js"),
}));
"#,
    )
    .expect("failed to write main script");

    let output = Command::new(bee_path())
        .arg("run")
        .arg("--preload")
        .arg(&preload)
        .arg(&main)
        .output()
        .expect("failed to execute bee run");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee run with preload should expose require.main. output: {combined}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        r#"{"entryMain":true,"helperMain":false,"helperSawEntry":true}"#,
        "preload helper should see entry module as require.main. output: {combined}"
    );
}

#[test]
fn test_command_sets_process_argv_for_test_file() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("argv.test.js");
    std::fs::write(
        &test_file,
        r#"
test("has test file in process argv", () => {
  if (!process.argv[1] || !process.argv[1].endsWith("argv.test.js")) {
    throw new Error(`unexpected argv: ${JSON.stringify(process.argv)}`);
  }
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    assert!(
        output.status.success(),
        "bee test should set process.argv for test files. stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_command_test_name_pattern_runs_only_matching_file_tests() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("name_pattern.test.js");
    std::fs::write(
        &test_file,
        r#"
test("selected case", () => {
  console.log("ran selected");
});

test("unmatched case", () => {
  throw new Error("unmatched test should not run");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .args(["test", "--test-name-pattern", "selected"])
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "--test-name-pattern should skip non-matching tests. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 1 skipped"),
        "--test-name-pattern should report one skipped test. output: {combined}"
    );
    assert!(
        combined.contains("ran selected"),
        "matching test should run. output: {combined}"
    );
    assert!(
        !combined.contains("unmatched test should not run"),
        "non-matching test should not execute. output: {combined}"
    );
}

#[test]
fn test_command_test_name_pattern_uses_regex_for_file_tests() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("name_pattern_regex.test.js");
    std::fs::write(
        &test_file,
        r#"
test("selected 42", () => {
  console.log("ran regex selected");
});

test("selected suffix", () => {
  throw new Error("substring-only matcher should not run this test");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .args(["test", "--test-name-pattern", "^selected \\d+$"])
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "--test-name-pattern should treat patterns as regex. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 1 skipped"),
        "regex name pattern should run only the numeric selected test. output: {combined}"
    );
    assert!(
        combined.contains("ran regex selected"),
        "regex-matching test should run. output: {combined}"
    );
    assert!(
        !combined.contains("substring-only matcher should not run this test"),
        "non-regex match should not execute. output: {combined}"
    );
}

#[test]
fn test_command_test_only_uses_regex_for_file_tests() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("only_pattern_regex.test.js");
    std::fs::write(
        &test_file,
        r#"
test("critical alpha", () => {
  console.log("ran critical alpha");
});

test("critical gamma", () => {
  throw new Error("non-matching only regex should not run");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .args(["test", "--test-only", "critical (alpha|beta)$"])
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "--test-only should treat patterns as regex. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 1 skipped"),
        "regex test-only should run only the alpha test. output: {combined}"
    );
    assert!(
        combined.contains("ran critical alpha"),
        "regex-matching only test should run. output: {combined}"
    );
    assert!(
        !combined.contains("non-matching only regex should not run"),
        "test outside the only regex should not execute. output: {combined}"
    );
}

#[test]
fn test_command_test_skip_skips_matching_file_tests() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("skip_pattern.test.js");
    std::fs::write(
        &test_file,
        r#"
test("fast case", () => {
  console.log("ran fast");
});

test("slow case", () => {
  throw new Error("slow test should not run");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .args(["test", "--test-skip", "slow"])
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "--test-skip should skip matching tests. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 1 skipped"),
        "--test-skip should report one skipped test. output: {combined}"
    );
    assert!(
        combined.contains("ran fast"),
        "non-skipped test should run. output: {combined}"
    );
    assert!(
        !combined.contains("slow test should not run"),
        "skipped test should not execute. output: {combined}"
    );
}

#[test]
fn test_command_test_skip_uses_regex_for_file_tests() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("skip_pattern_regex.test.js");
    std::fs::write(
        &test_file,
        r#"
test("fast case", () => {
  console.log("ran regex fast");
});

test("slow 42", () => {
  throw new Error("regex-skipped test should not run");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .args(["test", "--test-skip", "^slow \\d+$"])
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "--test-skip should treat patterns as regex. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 1 skipped"),
        "regex test-skip should skip only the slow numeric test. output: {combined}"
    );
    assert!(
        combined.contains("ran regex fast"),
        "non-skipped test should run. output: {combined}"
    );
    assert!(
        !combined.contains("regex-skipped test should not run"),
        "regex-skipped test should not execute. output: {combined}"
    );
}

#[test]
fn test_command_bail_stops_after_first_file_test_failure() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("bail.test.js");
    std::fs::write(
        &test_file,
        r#"
test("first failure", () => {
  throw new Error("first failed");
});

test("second should not run", () => {
  console.log("ran after first failure");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .args(["test", "--bail"])
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "--bail should still fail the command on the first failure. output: {combined}"
    );
    assert!(
        combined.contains("first failed"),
        "first failure should be reported. output: {combined}"
    );
    assert!(
        !combined.contains("ran after first failure"),
        "--bail should prevent later tests from executing. output: {combined}"
    );
}

#[test]
fn test_command_timeout_fails_slow_file_promise() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("timeout.test.js");
    std::fs::write(
        &test_file,
        r#"
test("slow promise", () => new Promise((resolve) => {
  setTimeout(resolve, 250);
}));
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .args(["test", "--timeout", "0"])
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "--timeout should fail a slow promise. output: {combined}"
    );
    assert!(
        combined.contains("timed out after 0s"),
        "timeout failure should mention the configured timeout. output: {combined}"
    );
}

#[test]
fn test_command_default_timeout_fails_pending_timer_promise() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("default_timeout.test.js");
    std::fs::write(
        &test_file,
        r#"
test("pending promise must not be treated as passed", () => new Promise((resolve) => {
  setTimeout(resolve, 250);
}));
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test must fail unresolved timer promises instead of printing success. output: {combined}"
    );
    assert!(
        combined.contains("timed out"),
        "default timeout failure should explain the pending async test. output: {combined}"
    );
}

#[test]
fn test_command_timeout_seconds_are_not_capped_to_75ms() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("timeout_seconds.test.js");
    std::fs::write(
        &test_file,
        r#"
test("settles before one second", () => new Promise((resolve) => {
  setTimeout(resolve, 120);
}));
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .args(["test", "--timeout", "1"])
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "--timeout 1 should not be capped to 75ms. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed"),
        "the 120ms test should complete within the configured second. output: {combined}"
    );
}

#[test]
fn test_command_supports_jest_set_timeout_for_file_tests() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("jest_set_timeout.test.js");
    std::fs::write(
        &test_file,
        r#"
jest.setTimeout(1000);

test("settles within Jest timeout", () => new Promise((resolve) => {
  setTimeout(resolve, 120);
}));
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should honor jest.setTimeout for file tests. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 0 skipped"),
        "the 120ms test should complete within jest.setTimeout. output: {combined}"
    );
}

#[test]
fn test_command_reports_jest_set_timeout_expiration() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("jest_set_timeout_expiration.test.js");
    std::fs::write(
        &test_file,
        r#"
jest.setTimeout(1);

test("exceeds Jest timeout", () => new Promise((resolve) => {
  setTimeout(resolve, 50);
}));
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail when jest.setTimeout expires. output: {combined}"
    );
    assert!(
        combined.contains("timed out after"),
        "jest.setTimeout expiration should report timeout. output: {combined}"
    );
    assert!(
        !combined.contains("Tests passed"),
        "expired jest.setTimeout must not report success. output: {combined}"
    );
}

#[test]
fn test_command_supports_concurrent_file_tests_serially() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("concurrent.test.js");
    std::fs::write(
        &test_file,
        r#"
test.concurrent("async concurrent compat", async () => {
  await Promise.resolve();
  expect("settled").toBe("settled");
});

it.concurrent.each([
  [1, 2, 3],
  [2, 3, 5],
])("adds %i and %i", (left, right, total) => {
  expect(left + right).toBe(total);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should accept Jest concurrent test APIs in serial file-mode. output: {combined}"
    );
    assert!(
        combined.contains("3 passed, 0 failed, 0 skipped"),
        "concurrent compatibility tests should be collected and run. output: {combined}"
    );
}

#[test]
fn test_command_supports_concurrent_only_and_skip_file_tests() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("concurrent_only_skip.test.js");
    std::fs::write(
        &test_file,
        r#"
test.concurrent.skip("skipped concurrent", () => {
  throw new Error("skip body should not run");
});

test.concurrent.only("focused concurrent", () => {
  expect(1).toBe(1);
});

test("ordinary test hidden by only", () => {
  throw new Error("only filter should skip this test");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .arg("test")
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "bee test should apply only/skip semantics to concurrent tests. output: {combined}"
    );
    assert!(
        combined.contains("1 passed, 0 failed, 2 skipped"),
        "concurrent only/skip should reuse file-mode filtering. output: {combined}"
    );
}

#[test]
fn test_command_without_file_discovers_and_fails_project_tests() {
    let dir = tempdir().expect("failed to create tempdir");
    std::fs::write(
        dir.path().join("failing.test.js"),
        r#"
test("real project test fails", () => {
  expect(1).toBe(2);
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .arg("test")
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail discovered project tests. output: {combined}"
    );
    assert!(
        combined.contains("Expected 1 to be 2"),
        "discovered test failure should be reported. output: {combined}"
    );
}

#[test]
fn eval_deny_fs_blocks_read_file_sync() {
    let dir = tempdir().expect("failed to create tempdir");
    let secret = dir.path().join("secret.txt");
    std::fs::write(&secret, "secret-value").expect("failed to write secret");

    let code = format!(
        r#"
try {{
  require("fs").readFileSync({}, "utf8");
  console.log("allowed");
}} catch (error) {{
  console.log(String(error && error.message || error));
}}
"#,
        js_string(&secret)
    );

    let output = Command::new(bee_path())
        .args(["eval", "--deny-fs"])
        .arg(code)
        .output()
        .expect("failed to execute bee eval");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "caught permission errors should keep eval process successful. output: {combined}"
    );
    assert!(
        combined.contains("permission denied"),
        "--deny-fs should deny fs.readFileSync. output: {combined}"
    );
    assert!(
        !combined.contains("secret-value") && !combined.contains("allowed"),
        "denied read must not leak the file contents. output: {combined}"
    );
}

#[test]
fn eval_deny_fs_allows_explicit_read_exception() {
    let dir = tempdir().expect("failed to create tempdir");
    let secret = dir.path().join("secret.txt");
    std::fs::write(&secret, "secret-value").expect("failed to write secret");

    let code = format!(
        r#"require("fs").readFileSync({}, "utf8");"#,
        js_string(&secret)
    );

    let output = Command::new(bee_path())
        .arg("eval")
        .arg("--deny-fs")
        .arg("--allow-read")
        .arg(&secret)
        .arg(code)
        .output()
        .expect("failed to execute bee eval");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "--allow-read should allow the exact path after --deny-fs. output: {combined}"
    );
    assert!(
        combined.contains("secret-value"),
        "allowed read should print the file contents. output: {combined}"
    );
}

#[test]
fn eval_deny_fs_blocks_write_file_sync() {
    let dir = tempdir().expect("failed to create tempdir");
    let output_file = dir.path().join("created.txt");

    let code = format!(
        r#"
try {{
  require("fs").writeFileSync({}, "created");
  console.log("allowed");
}} catch (error) {{
  console.log(String(error && error.message || error));
}}
"#,
        js_string(&output_file)
    );

    let output = Command::new(bee_path())
        .args(["eval", "--deny-fs"])
        .arg(code)
        .output()
        .expect("failed to execute bee eval");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "caught permission errors should keep eval process successful. output: {combined}"
    );
    assert!(
        combined.contains("permission denied"),
        "--deny-fs should deny fs.writeFileSync. output: {combined}"
    );
    assert!(
        !output_file.exists(),
        "denied write must not create the target file. output: {combined}"
    );
}

#[test]
fn eval_deny_fs_allows_explicit_write_exception() {
    let dir = tempdir().expect("failed to create tempdir");
    let output_file = dir.path().join("created.txt");

    let code = format!(
        r#"require("fs").writeFileSync({}, "created"); "done";"#,
        js_string(&output_file)
    );

    let output = Command::new(bee_path())
        .arg("eval")
        .arg("--deny-fs")
        .arg("--allow-write")
        .arg(&output_file)
        .arg(code)
        .output()
        .expect("failed to execute bee eval");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "--allow-write should allow the exact path after --deny-fs. output: {combined}"
    );
    assert_eq!(
        std::fs::read_to_string(&output_file).expect("allowed write should create the file"),
        "created"
    );
}

#[test]
fn eval_deny_net_blocks_websocket_constructor() {
    let code = r#"
try {
  new WebSocket("wss://blocked.example/socket");
  console.log("allowed");
} catch (error) {
  console.log(String(error && error.message || error));
}
"done";
"#;

    let output = Command::new(bee_path())
        .args(["eval", "--deny-net"])
        .arg(code)
        .output()
        .expect("failed to execute bee eval");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "caught network permission errors should keep eval successful. output: {combined}"
    );
    assert!(
        combined.contains("permission denied"),
        "--deny-net should deny WebSocket network connect. output: {combined}"
    );
    assert!(
        !combined.contains("allowed"),
        "denied network connect must not construct the WebSocket. output: {combined}"
    );
}

#[test]
fn eval_deny_net_allows_explicit_host_exception() {
    let code = r#"
const results = [];
try {
  new WebSocket("wss://allowed.example/socket");
  results.push("allowed-ok");
} catch (error) {
  results.push(String(error && error.message || error));
}
try {
  new WebSocket("wss://blocked.example/socket");
  results.push("blocked-allowed");
} catch (error) {
  results.push(String(error && error.message || error).includes("permission denied") ? "blocked-denied" : "blocked-other");
}
results.join("|");
"#;

    let output = Command::new(bee_path())
        .arg("eval")
        .arg("--deny-net")
        .arg("--allow-net")
        .arg("allowed.example")
        .arg(code)
        .output()
        .expect("failed to execute bee eval");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "--allow-net host should keep eval successful. output: {combined}"
    );
    assert!(
        combined.contains("allowed-ok|blocked-denied"),
        "--allow-net host should allow only that host. output: {combined}"
    );
}

#[test]
fn eval_allow_net_host_does_not_allow_http_server_listen() {
    let code = r#"
const http = require("http");
const server = http.createServer();
try {
  server.listen(0, "127.0.0.1");
  console.log(`allowed:${server.listening}`);
} catch (error) {
  console.log(`${String(error && error.message || error)}:${server.listening}`);
}
"done";
"#;

    let output = Command::new(bee_path())
        .arg("eval")
        .arg("--deny-net")
        .arg("--allow-net")
        .arg("127.0.0.1")
        .arg(code)
        .output()
        .expect("failed to execute bee eval");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "caught listen permission errors should keep eval successful. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("Network")
            && combined.contains("Listen"),
        "--allow-net host should not grant HTTP server listen permission. output: {combined}"
    );
    assert!(
        combined.contains(":false"),
        "denied listen must not mark the server as listening. output: {combined}"
    );
    assert!(
        !combined.contains("allowed:true"),
        "HTTP server listen must not be allowed by outbound --allow-net host. output: {combined}"
    );
}

#[test]
fn eval_deny_net_allows_explicit_listen_exception() {
    let code = r#"
const http = require("http");
const server = http.createServer();
try {
  server.listen(0, "127.0.0.1");
  console.log(`allowed:${server.listening}`);
} catch (error) {
  console.log(`${String(error && error.message || error)}:${server.listening}`);
}
"done";
"#;

    let output = Command::new(bee_path())
        .arg("eval")
        .arg("--deny-net")
        .arg("--allow-listen")
        .arg("127.0.0.1")
        .arg(code)
        .output()
        .expect("failed to execute bee eval");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "--allow-listen host should keep eval successful. output: {combined}"
    );
    assert!(
        combined.contains("allowed:true"),
        "--allow-listen host should grant HTTP server listen permission. output: {combined}"
    );
    assert!(
        !combined.contains("permission denied"),
        "explicit listen permission should not be denied. output: {combined}"
    );
}

#[test]
fn eval_deny_net_allows_exact_url_exception_only() {
    let code = r#"
const results = [];
try {
  new WebSocket("wss://allowed.example/socket");
  results.push("exact-ok");
} catch (error) {
  results.push(String(error && error.message || error));
}
try {
  new WebSocket("wss://allowed.example/other");
  results.push("other-allowed");
} catch (error) {
  results.push(String(error && error.message || error).includes("permission denied") ? "other-denied" : "other-other");
}
results.join("|");
"#;

    let output = Command::new(bee_path())
        .arg("eval")
        .arg("--deny-net")
        .arg("--allow-net")
        .arg("wss://allowed.example/socket")
        .arg(code)
        .output()
        .expect("failed to execute bee eval");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "--allow-net URL should keep eval successful. output: {combined}"
    );
    assert!(
        combined.contains("exact-ok|other-denied"),
        "--allow-net URL should allow only the exact URL. output: {combined}"
    );
}

#[test]
fn eval_permission_policy_denies_fs_and_allows_relative_read_path() {
    let dir = tempdir().expect("failed to create tempdir");
    let allowed_file = dir.path().join("allowed.txt");
    let blocked_file = dir.path().join("blocked.txt");
    let policy_file = dir.path().join("bee.policy.json");
    std::fs::write(&allowed_file, "allowed").expect("failed to write allowed file");
    std::fs::write(&blocked_file, "blocked").expect("failed to write blocked file");
    std::fs::write(
        &policy_file,
        r#"{
  "permissions": {
    "deny_fs": true,
    "allow_read": ["allowed.txt"]
  }
}"#,
    )
    .expect("failed to write permission policy");

    let code = format!(
        r#"
const fs = require("fs");
const results = [];
try {{
  results.push(fs.readFileSync({}, "utf8"));
}} catch (error) {{
  results.push("allowed-denied");
}}
try {{
  fs.readFileSync({}, "utf8");
  results.push("blocked-allowed");
}} catch (error) {{
  results.push(String(error && error.message || error).includes("permission denied") ? "blocked-denied" : "blocked-other");
}}
results.join("|");
"#,
        js_string(&allowed_file),
        js_string(&blocked_file)
    );

    let output = Command::new(bee_path())
        .arg("eval")
        .arg("--policy")
        .arg(&policy_file)
        .arg(code)
        .output()
        .expect("failed to execute bee eval");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "permission policy should allow the listed read path and deny the rest. output: {combined}"
    );
    assert!(
        combined.contains("allowed|blocked-denied"),
        "policy allow_read should resolve relative to policy file directory. output: {combined}"
    );
}

#[test]
fn eval_permission_policy_denies_environment_except_allow_list() {
    let dir = tempdir().expect("failed to create tempdir");
    let policy_file = dir.path().join("bee.policy.json");
    std::fs::write(
        &policy_file,
        r#"{
  "permissions": {
    "deny_env": true,
    "allow_env": ["BEEJS_POLICY_PUBLIC"]
  }
}"#,
    )
    .expect("failed to write permission policy");

    let code = r#"
const secret = process.env.BEEJS_POLICY_SECRET === undefined ? "secret-denied" : "secret-visible";
const publicValue = process.env.BEEJS_POLICY_PUBLIC === "visible" ? "public-visible" : String(process.env.BEEJS_POLICY_PUBLIC);
secret + "|" + publicValue;
"#;

    let output = Command::new(bee_path())
        .arg("eval")
        .arg("--policy")
        .arg(&policy_file)
        .arg(code)
        .env("BEEJS_POLICY_SECRET", "hidden")
        .env("BEEJS_POLICY_PUBLIC", "visible")
        .output()
        .expect("failed to execute bee eval");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "permission policy should filter environment variables without failing eval. output: {combined}"
    );
    assert!(
        combined.contains("secret-denied|public-visible"),
        "policy allow_env should expose only listed variables. output: {combined}"
    );
}

#[test]
fn eval_deny_env_allows_explicit_environment_exception() {
    let code = r#"
const secret = process.env.BEEJS_CLI_SECRET === undefined ? "secret-denied" : "secret-visible";
const publicValue = process.env.BEEJS_CLI_PUBLIC === "visible" ? "public-visible" : String(process.env.BEEJS_CLI_PUBLIC);
secret + "|" + publicValue;
"#;

    let output = Command::new(bee_path())
        .arg("eval")
        .arg("--deny-env")
        .arg("--allow-env")
        .arg("BEEJS_CLI_PUBLIC")
        .arg(code)
        .env("BEEJS_CLI_SECRET", "hidden")
        .env("BEEJS_CLI_PUBLIC", "visible")
        .output()
        .expect("failed to execute bee eval");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "--allow-env should keep eval successful. output: {combined}"
    );
    assert!(
        combined.contains("secret-denied|public-visible"),
        "--allow-env should expose only the listed environment variable. output: {combined}"
    );
}

#[test]
fn eval_deny_run_allows_explicit_child_process_command() {
    let code = r#"
const childProcess = require("child_process");
const results = [];
try {
  childProcess.exec("allowed-command");
  results.push("allowed-ok");
} catch (error) {
  results.push("allowed-denied");
}
try {
  childProcess.exec("blocked-command");
  results.push("blocked-allowed");
} catch (error) {
  results.push(String(error && error.message || error).includes("permission denied") ? "blocked-denied" : "blocked-other");
}
results.join("|");
"#;

    let output = Command::new(bee_path())
        .arg("eval")
        .arg("--deny-run")
        .arg("--allow-run")
        .arg("allowed-command")
        .arg(code)
        .output()
        .expect("failed to execute bee eval");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "--allow-run should keep eval successful. output: {combined}"
    );
    assert!(
        combined.contains("allowed-ok|blocked-denied"),
        "--allow-run should allow only the listed child_process command. output: {combined}"
    );
}

#[test]
fn run_deny_fs_blocks_script_fs_read() {
    let dir = tempdir().expect("failed to create tempdir");
    let secret = dir.path().join("secret.txt");
    let script = dir.path().join("read-secret.js");
    std::fs::write(&secret, "secret-value").expect("failed to write secret");
    std::fs::write(
        &script,
        format!(
            r#"
try {{
  console.log(require("fs").readFileSync({}, "utf8"));
}} catch (error) {{
  console.log(String(error && error.message || error));
}}
"#,
            js_string(&secret)
        ),
    )
    .expect("failed to write script");

    let output = Command::new(bee_path())
        .args(["run", "--deny-fs", "--allow-read"])
        .arg(&script)
        .arg(&script)
        .output()
        .expect("failed to execute bee run");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "caught permission errors should keep run process successful. output: {combined}"
    );
    assert!(
        combined.contains("permission denied"),
        "--deny-fs should deny fs.readFileSync from scripts. output: {combined}"
    );
    assert!(
        !combined.contains("secret-value"),
        "denied read must not leak the file contents. output: {combined}"
    );
}

#[test]
fn run_deny_fs_blocks_target_file_read() {
    let dir = tempdir().expect("failed to create tempdir");
    let script = dir.path().join("target-secret.js");
    std::fs::write(&script, "console.log('TARGET_FILE_RAN');").expect("failed to write script");

    let output = Command::new(bee_path())
        .args(["run", "--deny-fs"])
        .arg(&script)
        .output()
        .expect("failed to execute bee run");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "--deny-fs should deny reading the target script itself. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("FileSystem")
            && combined.contains("Read"),
        "target script read should report broker denial. output: {combined}"
    );
    assert!(
        !combined.contains("TARGET_FILE_RAN"),
        "denied target script must not execute. output: {combined}"
    );
}

#[test]
fn run_watch_deny_fs_fails_before_starting_watcher() {
    let dir = tempdir().expect("failed to create tempdir");
    let script = dir.path().join("watch-target.js");
    std::fs::write(&script, "console.log('WATCH_TARGET_RAN');").expect("failed to write script");

    let output = Command::new(bee_path())
        .args(["run", "--deny-fs", "--watch"])
        .arg(&script)
        .output()
        .expect("failed to execute bee run --watch");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "--deny-fs should deny watch target before startup. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("FileSystem")
            && combined.contains("Read"),
        "watch target read should report broker denial. output: {combined}"
    );
    assert!(
        !combined.contains("WATCH_TARGET_RAN"),
        "denied watch target must not execute. output: {combined}"
    );
    assert!(
        !combined.contains("Watch mode enabled"),
        "watch mode should not report startup before denied target read is rejected. output: {combined}"
    );
    assert!(
        !combined.contains("Watching for changes"),
        "watcher must not start before denied target read is rejected. output: {combined}"
    );
    assert!(
        !combined.contains("WebSocket server ready"),
        "watch websocket must not be reported ready after denied target read. output: {combined}"
    );
}

#[test]
fn run_deny_fs_blocks_relative_require_module_file() {
    let dir = tempdir().expect("failed to create tempdir");
    let lib = dir.path().join("lib.js");
    let script = dir.path().join("main.js");
    std::fs::write(&lib, "module.exports = { value: 7 };").expect("failed to write module");
    std::fs::write(&script, "require('./lib').value;").expect("failed to write script");

    let output = Command::new(bee_path())
        .args(["run", "--deny-fs"])
        .arg(&script)
        .output()
        .expect("failed to execute bee run");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "uncaught denied module load should fail bee run. output: {combined}"
    );
    assert!(
        combined.contains("permission denied"),
        "--deny-fs should deny CommonJS module file reads. output: {combined}"
    );
}

#[test]
fn debug_deny_fs_blocks_debug_target_file_read() {
    let dir = tempdir().expect("failed to create tempdir");
    let script = dir.path().join("debug-secret.js");
    std::fs::write(&script, "console.log('debug-secret-value');").expect("failed to write script");

    let output = Command::new(bee_path())
        .args(["debug", "--deny-fs"])
        .arg(&script)
        .output()
        .expect("failed to execute bee debug");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee debug should fail when target file read is denied. output: {combined}"
    );
    assert!(
        combined.contains("permission denied"),
        "--deny-fs should deny debug target file reads. output: {combined}"
    );
    assert!(
        !combined.contains("debug-secret-value"),
        "denied debug target read must not print file contents. output: {combined}"
    );
}

#[test]
fn test_command_deny_fs_blocks_target_file_read() {
    let dir = tempdir().expect("failed to create tempdir");
    let test_file = dir.path().join("target-read.test.js");
    std::fs::write(
        &test_file,
        r#"
test("target file should not run", () => {
  console.log("TARGET_TEST_FILE_RAN");
});
"#,
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .args(["test", "--deny-fs"])
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "--deny-fs should deny reading the target test file itself. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("FileSystem")
            && combined.contains("Read"),
        "target test file read should report broker denial. output: {combined}"
    );
    assert!(
        !combined.contains("TARGET_TEST_FILE_RAN"),
        "denied target test file must not execute. output: {combined}"
    );
}

#[test]
fn test_command_deny_fs_blocks_discovery_root_read() {
    let dir = tempdir().expect("failed to create tempdir");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .args(["test", "--deny-fs"])
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "--deny-fs should deny reading the discovery root before running builtin tests. output: {combined}"
    );
    assert!(
        combined.contains("permission denied")
            && combined.contains("FileSystem")
            && combined.contains("Read"),
        "discovery root read should report broker denial. output: {combined}"
    );
    assert!(
        !combined.contains("Test Summary: 8 passed"),
        "denied discovery must not fall back to builtin tests. output: {combined}"
    );
}

#[test]
fn test_command_deny_fs_blocks_file_test_fs_read() {
    let dir = tempdir().expect("failed to create tempdir");
    let secret = dir.path().join("secret.txt");
    let test_file = dir.path().join("permission.test.js");
    std::fs::write(&secret, "secret-value").expect("failed to write secret");
    std::fs::write(
        &test_file,
        format!(
            r#"
test("fs read is denied", () => {{
  require("fs").readFileSync({}, "utf8");
}});
"#,
            js_string(&secret)
        ),
    )
    .expect("failed to write test file");

    let output = Command::new(bee_path())
        .args(["test", "--deny-fs", "--allow-read"])
        .arg(&test_file)
        .arg(&test_file)
        .output()
        .expect("failed to execute bee test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "bee test should fail when a test body hits denied fs. output: {combined}"
    );
    assert!(
        combined.contains("permission denied"),
        "--deny-fs should deny fs reads inside bee test callbacks. output: {combined}"
    );
}

#[test]
fn run_hello_server_answers_http_without_manual_pump() {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::{Duration, Instant};

    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").expect("reserve a free port");
        listener.local_addr().expect("local addr").port()
    };

    let mut child = Command::new(bee_path())
        .args(["run", "examples/http/hello_server.js"])
        .env("PORT", port.to_string())
        .env("HOST", "127.0.0.1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn bee run hello_server");

    let started = Instant::now();
    let mut response = None;
    while started.elapsed() < Duration::from_secs(15) {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(mut stream) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                if stream
                    .write_all(
                        format!(
                            "GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
                        )
                        .as_bytes(),
                    )
                    .is_ok()
                {
                    let mut body = String::new();
                    if stream.read_to_string(&mut body).is_ok() && body.contains("hello") {
                        response = Some(body);
                        break;
                    }
                }
            }
            Err(_) => thread::sleep(Duration::from_millis(100)),
        }
    }

    let _ = child.kill();
    let _ = child.wait();

    let body =
        response.expect("bee run hello_server.js should answer HTTP without a test-side pump");
    assert!(
        body.contains("200") && body.contains("hello"),
        "hello_server should return 200 hello. response: {body}"
    );
}

#[test]
fn run_sandbox_denies_fs_read_outside_allow_prefix() {
    let dir = tempdir().expect("failed to create tempdir");
    let jail = dir.path().join("workspace");
    std::fs::create_dir_all(&jail).expect("failed to create jail");
    std::fs::write(jail.join("ok.txt"), "ok").expect("failed to write jail file");
    std::fs::write(dir.path().join("secret.txt"), "nope").expect("failed to write secret");
    let script = dir.path().join("tool.js");
    std::fs::write(
        &script,
        r#"
const fs = require("fs");
const path = require("path");
const results = [];
try {
  results.push(fs.readFileSync(path.join(__dirname, "workspace", "ok.txt"), "utf8").trim());
} catch (error) {
  results.push("jail-denied");
}
try {
  fs.readFileSync(path.join(__dirname, "secret.txt"), "utf8");
  results.push("secret-allowed");
} catch (error) {
  results.push(String(error && error.message || error).includes("permission denied") ? "secret-denied" : "secret-other");
}
console.log(results.join("|"));
"#,
    )
    .expect("failed to write script");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .args([
            "run",
            "--sandbox",
            "--allow-read",
            jail.to_str().unwrap(),
            script.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute bee run --sandbox");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "sandbox prefix jail should keep the process successful. output: {combined}"
    );
    assert!(
        combined.contains("ok|secret-denied"),
        "sandbox should allow the jail prefix and deny sibling paths. output: {combined}"
    );
}

#[test]
fn run_sandbox_writes_audit_jsonl_without_secret_values() {
    let dir = tempdir().expect("failed to create tempdir");
    let audit = dir.path().join("audit.jsonl");
    let script = dir.path().join("probe.js");
    std::fs::write(dir.path().join("secret.txt"), "classified-value")
        .expect("failed to write secret");
    std::fs::write(
        &script,
        r#"
try { require("fs").readFileSync(require("path").join(__dirname, "secret.txt"), "utf8"); }
catch (error) { console.log("denied"); }
"#,
    )
    .expect("failed to write script");

    let output = Command::new(bee_path())
        .current_dir(dir.path())
        .args([
            "run",
            "--sandbox",
            "--audit-log",
            audit.to_str().unwrap(),
            script.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute bee run --audit-log");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "denied read should stay catchable. output: {combined}"
    );
    let log = std::fs::read_to_string(&audit).expect("audit log should exist");
    assert!(
        log.contains("\"decision\":\"Deny\"") && log.contains("FileSystem"),
        "audit log should record a filesystem deny. log: {log}"
    );
    assert!(
        !log.contains("classified-value"),
        "audit log must not include file contents. log: {log}"
    );
}

#[test]
fn run_export_tools_prints_echo_schema() {
    let output = Command::new(bee_path())
        .args([
            "run",
            "--sandbox",
            "--export-tools",
            "examples/agent/echo_tool.ts",
        ])
        .output()
        .expect("failed to execute bee run --export-tools");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "export-tools should succeed. stderr: {stderr}"
    );
    assert!(
        stdout.contains("\"name\": \"echo\"") || stdout.contains("\"name\":\"echo\""),
        "export-tools should print the echo tool. stdout: {stdout}"
    );
}

#[test]
fn session_jsonrpc_calls_echo_tool() {
    let mut child = Command::new(bee_path())
        .args(["session", "--sandbox", "examples/agent/echo_tool.ts"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn bee session");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        use std::io::Write;
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"tools/list"}}"#).unwrap();
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"echo","arguments":{{"text":"hi"}}}}}}"#
        )
        .unwrap();
    }
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("session should exit");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "bee session should exit successfully. stderr: {stderr} stdout: {stdout}"
    );
    assert!(
        stdout.contains("echo"),
        "tools/list should mention echo. stdout: {stdout}"
    );
    assert!(
        stdout.contains("hi"),
        "tools/call should echo the text. stdout: {stdout}"
    );
}

#[test]
fn session_isolate_per_call_echoes_twice() {
    let mut child = Command::new(bee_path())
        .args([
            "session",
            "--sandbox",
            "--isolate-per-call",
            "examples/agent/echo_tool.ts",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn bee session --isolate-per-call");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        use std::io::Write;
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"echo","arguments":{{"text":"one"}}}}}}"#
        )
        .unwrap();
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"echo","arguments":{{"text":"two"}}}}}}"#
        )
        .unwrap();
    }
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("session should exit");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "isolate-per-call session should succeed. stderr: {stderr} stdout: {stdout}"
    );
    assert!(
        stdout.contains("one") && stdout.contains("two"),
        "both isolate-per-call tool results should appear. stdout: {stdout}"
    );
}

#[test]
fn mcp_initialize_lists_echo_tool() {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    })
    .to_string();
    let framed = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);

    let mut child = Command::new(bee_path())
        .args(["mcp", "--sandbox", "examples/agent/echo_tool.ts"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn bee mcp");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        use std::io::Write;
        stdin.write_all(framed.as_bytes()).unwrap();
    }
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("mcp should exit");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "bee mcp initialize should succeed. stderr: {stderr} stdout: {stdout}"
    );
    assert!(
        stdout.contains("beejs") && stdout.contains("protocolVersion"),
        "MCP initialize should return server info. stdout: {stdout}"
    );
}

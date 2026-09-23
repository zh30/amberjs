// Pins the Stable `amber install` contract in docs/INSTALL_CONTRACT.md.
// CI runs this target explicitly so the written contract cannot silently regress.

use amberjs::package_manager::{
    validate_locked_dependency_dist, verify_package_tarball, LockedDependency, INSTALL_ERROR_PREFIX,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

const PAYLOAD: &[u8] = b"amber-install-contract-v1\n";
const SHA512: &str = "sha512-khPyLQE8NbFexb/KWd9WmobGXOYz0jEQDltjab4VOKkE95A2DvyobOKWDdCJblw1cEiZWp1BgYSuUURY7KZ/jg==";
const SHA384: &str = "sha384-6t4g7O44R6pYgLFCnoig5hQGiqJKO+Or9PTWY8nUOXYMI9nZwJm0Fl/FB5JLqrsh";
const SHA256: &str = "sha256-sbTL0/X4w6kn93CGgoZyAbxXtBo7wuByW2wwrOJJMc0=";
const SHA1_SRI: &str = "sha1-Vhqj56/KHSBLVOe+x0nXxgI6Cqk=";
const SHA1_HEX: &str = "561aa3e7afca1d204b54e7bec749d7c6023a0aa9";
const SHA512_OTHER: &str =
    "sha512-4lrDhF+MvhKAGi36WonUxV3EeQDztu3Jqe5ZDzwrkxL2ZdADnJOCi3tY8zlQvIF6CVWpxQAKjT4oBWnwh0XKaA==";

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

fn run_install(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(amber_path())
        .current_dir(dir)
        .arg("install")
        .args(args)
        .output()
        .expect("failed to execute amber install")
}

fn assert_stable_error(output: &std::process::Output, needles: &[&str]) {
    let text = combined(output);
    assert!(
        !output.status.success(),
        "expected amber install to fail. output: {text}"
    );
    assert!(
        text.contains(INSTALL_ERROR_PREFIX),
        "failure must use {INSTALL_ERROR_PREFIX}. output: {text}"
    );
    for needle in needles {
        assert!(
            text.contains(needle),
            "failure must contain {needle:?}. output: {text}"
        );
    }
}

fn assert_success(output: &std::process::Output, needles: &[&str]) {
    let text = combined(output);
    assert!(
        output.status.success(),
        "expected amber install to succeed. output: {text}"
    );
    assert!(
        !text.contains(INSTALL_ERROR_PREFIX),
        "success must not print {INSTALL_ERROR_PREFIX}. output: {text}"
    );
    for needle in needles {
        assert!(
            text.contains(needle),
            "success must contain {needle:?}. output: {text}"
        );
    }
}

fn write_package(dir: &Path, body: &str) {
    fs::write(dir.join("package.json"), body).expect("write package.json");
}

fn lock_pinning(version: &str, lockfile_version: u32) -> String {
    format!(
        r#"{{
  "name": "fixture",
  "version": "1.0.0",
  "lockfileVersion": {lockfile_version},
  "requires": true,
  "dependencies": {{
    "left-pad": {{
      "version": "{version}",
      "resolved": "https://registry.npmjs.org/left-pad/-/left-pad-{version}.tgz",
      "integrity": "sha512-test"
    }}
  }}
}}"#,
    )
}

fn project_requesting(section: &str, requested: &str) -> String {
    format!(
        r#"{{
  "name": "fixture",
  "version": "1.0.0",
  "{section}": {{
    "left-pad": "{requested}"
  }}
}}"#,
    )
}

#[test]
fn contract_help_names_the_subset() {
    let output = Command::new(amber_path())
        .args(["install", "--help"])
        .output()
        .expect("amber install --help");
    let text = combined(&output);
    assert!(output.status.success(), "{text}");
    assert!(text.contains("frozen-lockfile"), "{text}");
    assert!(
        text.contains("Not an npm, yarn, or pnpm replacement"),
        "{text}"
    );
}

#[test]
fn contract_empty_install_writes_lock_and_skips_lifecycle() {
    let dir = tempdir().expect("tempdir");
    write_package(
        dir.path(),
        r#"{
  "name": "empty-contract",
  "version": "1.2.3",
  "scripts": {
    "preinstall": "exit 1",
    "install": "exit 1",
    "postinstall": "exit 1"
  }
}"#,
    );
    let output = run_install(dir.path(), &[]);
    assert_success(
        &output,
        &["Installed 0 dependencies", "Generated package-lock.json"],
    );
    let lock: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(dir.path().join("package-lock.json")).expect("read lock"),
    )
    .expect("parse generated lock");
    assert_eq!(lock["lockfileVersion"], 3);
    assert_eq!(lock["name"], "empty-contract");
    assert_eq!(lock["version"], "1.2.3");
    assert!(lock["dependencies"].is_object());
    assert!(lock.get("packages").is_none());
    assert!(dir.path().join("node_modules").is_dir());
}

#[test]
fn contract_frozen_empty_lock_is_not_rewritten() {
    let dir = tempdir().expect("tempdir");
    write_package(
        dir.path(),
        r#"{ "name": "frozen-empty", "version": "1.0.0", "dependencies": {} }"#,
    );
    let lock_path = dir.path().join("package-lock.json");
    let original = "{\n  \"name\": \"frozen-empty\",\n  \"version\": \"1.0.0\",\n  \"lockfileVersion\": 3,\n  \"dependencies\": {}\n}\n";
    fs::write(&lock_path, original).expect("write lock");
    let output = run_install(dir.path(), &["--frozen-lockfile"]);
    assert_success(&output, &["Verified frozen package-lock.json"]);
    let text = combined(&output);
    assert!(
        !text.contains("Generated package-lock.json"),
        "frozen install must not claim a lock rewrite. output: {text}"
    );
    assert_eq!(fs::read_to_string(&lock_path).expect("reread"), original);
}

#[test]
fn contract_ignored_inputs_do_not_install_or_run_scripts() {
    let dir = tempdir().expect("tempdir");
    write_package(
        dir.path(),
        r#"{
  "name": "ignored-inputs",
  "version": "1.0.0",
  "peerDependencies": { "react": "^18.0.0" },
  "scripts": { "preinstall": "exit 1", "install": "exit 1", "postinstall": "exit 1" },
  "workspaces": ["packages/*"]
}"#,
    );
    fs::write(dir.path().join("yarn.lock"), "not a yarn lock {{{").expect("yarn.lock");
    fs::write(dir.path().join("pnpm-lock.yaml"), ":\n - [").expect("pnpm-lock");
    let output = run_install(dir.path(), &["--deny-net"]);
    assert_success(&output, &["Installed 0 dependencies"]);
    assert!(!dir.path().join("node_modules").join("react").exists());
    assert!(!dir.path().join("packages").exists());
}

#[test]
fn contract_missing_package_json() {
    let dir = tempdir().expect("tempdir");
    let output = run_install(dir.path(), &[]);
    assert_stable_error(&output, &["package.json not found"]);
    assert!(!dir.path().join("node_modules").exists());
    assert!(!dir.path().join("package-lock.json").exists());
}

#[test]
fn contract_invalid_package_json() {
    let dir = tempdir().expect("tempdir");
    write_package(dir.path(), "{");
    let output = run_install(dir.path(), &[]);
    assert_stable_error(&output, &["Failed to parse package.json"]);
    assert!(!dir.path().join("node_modules").exists());
}

#[test]
fn contract_package_json_requires_name() {
    let dir = tempdir().expect("tempdir");
    write_package(dir.path(), r#"{ "version": "1.0.0" }"#);
    let output = run_install(dir.path(), &[]);
    assert_stable_error(&output, &["Failed to parse package.json"]);
    assert!(
        !dir.path().join("package-lock.json").exists(),
        "a package.json that fails the typed parser must not write a lock"
    );
}

#[test]
fn contract_frozen_lockfile_required() {
    let dir = tempdir().expect("tempdir");
    write_package(
        dir.path(),
        r#"{ "name": "needs-lock", "version": "1.0.0", "dependencies": {} }"#,
    );
    let output = run_install(dir.path(), &["--frozen-lockfile"]);
    assert_stable_error(
        &output,
        &["frozen lockfile requires package-lock.json to exist"],
    );
    assert!(!dir.path().join("node_modules").exists());
    assert!(!dir.path().join(".amberjs_cache").exists());
}

#[test]
fn contract_frozen_non_string_dependency() {
    let dir = tempdir().expect("tempdir");
    write_package(
        dir.path(),
        r#"{
  "name": "bad-dep",
  "version": "1.0.0",
  "dependencies": { "left-pad": 1 }
}"#,
    );
    fs::write(
        dir.path().join("package-lock.json"),
        lock_pinning("1.3.0", 3),
    )
    .expect("lock");
    let output = run_install(dir.path(), &["--frozen-lockfile"]);
    assert_stable_error(
        &output,
        &[
            "frozen lockfile cannot validate non-string dependency",
            "left-pad",
        ],
    );
    assert!(!dir.path().join("node_modules").exists());
}

#[test]
fn contract_frozen_version_mismatch_fails_before_directories() {
    let dir = tempdir().expect("tempdir");
    write_package(dir.path(), &project_requesting("dependencies", "^2.0.0"));
    let lock_path = dir.path().join("package-lock.json");
    let original = lock_pinning("1.3.0", 3);
    fs::write(&lock_path, &original).expect("lock");
    let output = run_install(dir.path(), &["--frozen-lockfile"]);
    assert_stable_error(
        &output,
        &[
            "frozen lockfile mismatch for package",
            "left-pad",
            "package.json requests",
            "^2.0.0",
        ],
    );
    assert!(!dir.path().join("node_modules").exists());
    assert!(!dir.path().join(".amberjs_cache").exists());
    assert_eq!(fs::read_to_string(&lock_path).expect("reread"), original);
}

#[test]
fn contract_frozen_dev_dependency_mismatch() {
    let dir = tempdir().expect("tempdir");
    write_package(dir.path(), &project_requesting("devDependencies", "^2.0.0"));
    fs::write(
        dir.path().join("package-lock.json"),
        lock_pinning("1.3.0", 3),
    )
    .expect("lock");
    let output = run_install(dir.path(), &["--frozen-lockfile"]);
    assert_stable_error(
        &output,
        &["frozen lockfile mismatch for package", "left-pad"],
    );
    assert!(!dir.path().join("node_modules").exists());
}

#[test]
fn contract_frozen_optional_dependency_mismatch_is_fail_closed() {
    let dir = tempdir().expect("tempdir");
    write_package(
        dir.path(),
        &project_requesting("optionalDependencies", "^2.0.0"),
    );
    fs::write(
        dir.path().join("package-lock.json"),
        lock_pinning("1.3.0", 3),
    )
    .expect("lock");
    let output = run_install(dir.path(), &["--frozen-lockfile"]);
    assert_stable_error(
        &output,
        &["frozen lockfile mismatch for package", "left-pad"],
    );
    assert!(!dir.path().join("node_modules").exists());
}

#[test]
fn contract_frozen_packages_map_is_not_a_pin() {
    let dir = tempdir().expect("tempdir");
    write_package(dir.path(), &project_requesting("dependencies", "1.3.0"));
    fs::write(
        dir.path().join("package-lock.json"),
        r#"{
  "name": "fixture",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "packages": {
    "node_modules/left-pad": { "version": "1.3.0" }
  }
}"#,
    )
    .expect("packages-only lock");
    let output = run_install(dir.path(), &["--frozen-lockfile"]);
    assert_stable_error(
        &output,
        &[
            "frozen lockfile mismatch for package",
            "left-pad",
            "missing from package-lock.json",
        ],
    );
    assert!(!dir.path().join("node_modules").exists());
}

#[test]
fn contract_frozen_lockfile_version_1_still_checks_dependencies() {
    let dir = tempdir().expect("tempdir");
    write_package(dir.path(), &project_requesting("dependencies", "^2.0.0"));
    fs::write(
        dir.path().join("package-lock.json"),
        lock_pinning("1.3.0", 1),
    )
    .expect("lock");
    let output = run_install(dir.path(), &["--frozen-lockfile"]);
    assert_stable_error(
        &output,
        &["frozen lockfile mismatch for package", "left-pad"],
    );
    let text = combined(&output);
    assert!(
        !text.contains("Unsupported lockfile"),
        "lockfileVersion must not hide a dependency mismatch. output: {text}"
    );
}

#[test]
fn contract_lock_version_mismatch_does_not_unpack_or_rewrite() {
    let dir = tempdir().expect("tempdir");
    write_package(dir.path(), &project_requesting("dependencies", "^2.0.0"));
    let lock_path = dir.path().join("package-lock.json");
    let original = lock_pinning("1.3.0", 3);
    fs::write(&lock_path, &original).expect("lock");
    let output = run_install(dir.path(), &[]);
    assert_stable_error(
        &output,
        &[
            "Failed to install dependencies",
            "package-lock.json version mismatch for package",
            "left-pad",
        ],
    );
    assert!(!dir.path().join("node_modules").join("left-pad").exists());
    assert_eq!(fs::read_to_string(&lock_path).expect("reread"), original);
}

#[test]
fn contract_dev_dependency_lock_mismatch_fails() {
    let dir = tempdir().expect("tempdir");
    write_package(dir.path(), &project_requesting("devDependencies", "^2.0.0"));
    fs::write(
        dir.path().join("package-lock.json"),
        lock_pinning("1.3.0", 3),
    )
    .expect("lock");
    let output = run_install(dir.path(), &[]);
    assert_stable_error(
        &output,
        &["package-lock.json version mismatch for package", "left-pad"],
    );
}

#[test]
fn contract_optional_lock_mismatch_does_not_fail_install() {
    let dir = tempdir().expect("tempdir");
    write_package(
        dir.path(),
        &project_requesting("optionalDependencies", "^2.0.0"),
    );
    fs::write(
        dir.path().join("package-lock.json"),
        lock_pinning("1.3.0", 3),
    )
    .expect("lock");
    let output = run_install(dir.path(), &["--deny-net"]);
    assert_success(&output, &["Installed 0 dependencies"]);
    assert!(!dir.path().join("node_modules").join("left-pad").exists());
}

#[test]
fn contract_denied_network_uses_stable_prefix_and_skips_lock() {
    let dir = tempdir().expect("tempdir");
    write_package(dir.path(), &project_requesting("dependencies", "1.3.0"));
    let output = run_install(dir.path(), &["--deny-net"]);
    assert_stable_error(&output, &["permission denied", "Network", "Connect"]);
    assert!(!dir.path().join("package-lock.json").exists());
    assert!(!dir.path().join("node_modules").join("left-pad").exists());
}

#[test]
fn contract_tarball_sri_algorithms_match() {
    let dir = tempdir().expect("tempdir");
    let tarball = dir.path().join("pkg.tgz");
    fs::write(&tarball, PAYLOAD).expect("tarball");
    for integrity in [SHA512, SHA384, SHA256, SHA1_SRI] {
        verify_package_tarball(&tarball, Some(integrity), None)
            .unwrap_or_else(|err| panic!("{integrity} should match: {err}"));
    }
    verify_package_tarball(&tarball, None, Some(SHA1_HEX))
        .unwrap_or_else(|err| panic!("shasum should match: {err}"));
}

#[test]
fn contract_tarball_integrity_mismatch() {
    let dir = tempdir().expect("tempdir");
    let tarball = dir.path().join("pkg.tgz");
    fs::write(&tarball, PAYLOAD).expect("tarball");
    let err = verify_package_tarball(&tarball, Some(SHA512_OTHER), Some(SHA1_HEX))
        .expect_err("wrong sha512 must fail even if shasum would match");
    let text = err.to_string();
    assert!(
        text.contains("Package integrity mismatch"),
        "integrity failure body drifted: {text}"
    );
}

#[test]
fn contract_tarball_shasum_mismatch() {
    let dir = tempdir().expect("tempdir");
    let tarball = dir.path().join("pkg.tgz");
    fs::write(&tarball, PAYLOAD).expect("tarball");
    let err = verify_package_tarball(
        &tarball,
        None,
        Some("0000000000000000000000000000000000000000"),
    )
    .expect_err("wrong shasum");
    let text = err.to_string();
    assert!(
        text.contains("Package shasum mismatch"),
        "shasum failure body drifted: {text}"
    );
}

#[test]
fn contract_tarball_missing_integrity_is_refused() {
    let dir = tempdir().expect("tempdir");
    let tarball = dir.path().join("pkg.tgz");
    fs::write(&tarball, PAYLOAD).expect("tarball");
    let err = verify_package_tarball(&tarball, None, None).expect_err("untrusted tarball");
    let text = err.to_string();
    assert!(
        text.contains("refusing untrusted tarball"),
        "missing integrity body drifted: {text}"
    );
}

#[test]
fn contract_tarball_unsupported_algorithm() {
    let dir = tempdir().expect("tempdir");
    let tarball = dir.path().join("pkg.tgz");
    fs::write(&tarball, PAYLOAD).expect("tarball");
    let err = verify_package_tarball(&tarball, Some("blake3-aaaa"), None).expect_err("blake3");
    let text = err.to_string();
    assert!(
        text.contains("Unsupported package integrity algorithm"),
        "unsupported algorithm body drifted: {text}"
    );
}

fn locked_left_pad(integrity: &str, resolved: &str) -> LockedDependency {
    LockedDependency {
        version: "1.3.0".to_string(),
        resolved: Some(resolved.to_string()),
        integrity: Some(integrity.to_string()),
        dev: Some(false),
        dependencies: None,
    }
}

#[test]
fn contract_lock_metadata_integrity_and_resolved_mismatch() {
    let resolved = "https://registry.npmjs.org/left-pad/-/left-pad-1.3.0.tgz";
    let locked = locked_left_pad("sha512-locked", resolved);
    let integrity_err = validate_locked_dependency_dist(
        "left-pad",
        "1.3.0",
        Some(&locked),
        resolved,
        Some("sha512-registry"),
    )
    .expect_err("integrity strings differ");
    let integrity_text = integrity_err.to_string();
    assert!(
        integrity_text.contains("package-lock.json integrity mismatch for package")
            && integrity_text.contains("left-pad"),
        "{integrity_text}"
    );

    let resolved_locked =
        locked_left_pad("sha512-same", "https://example.invalid/left-pad-1.3.0.tgz");
    let resolved_err = validate_locked_dependency_dist(
        "left-pad",
        "1.3.0",
        Some(&resolved_locked),
        resolved,
        Some("sha512-same"),
    )
    .expect_err("resolved URL differs");
    let resolved_text = resolved_err.to_string();
    assert!(
        resolved_text.contains("package-lock.json resolved mismatch for package")
            && resolved_text.contains("left-pad"),
        "{resolved_text}"
    );

    let ok = locked_left_pad("sha512-same", resolved);
    validate_locked_dependency_dist(
        "left-pad",
        "1.3.0",
        Some(&ok),
        resolved,
        Some("sha512-same"),
    )
    .expect("matching lock metadata");
}

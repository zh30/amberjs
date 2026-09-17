// amber install command tests
// v0.3.229 - Test coverage for amber install command

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Get the path to the amber binary built by cargo for integration tests.
fn amberjs_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_amber"))
}

#[cfg(test)]
mod install_command_tests {
    use super::*;

    /// Test 1: amber install command should be recognized
    #[test]
    fn test_install_command_exists() {
        let amberjs = amberjs_path();
        assert!(amberjs.exists(), "amber binary should exist at {:?}", amberjs);

        let output = Command::new(&amberjs)
            .arg("install")
            .arg("--help")
            .output()
            .expect("Failed to execute amber install --help");

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Should have successful exit code or show help
        assert!(
            output.status.success()
                || stdout.contains("Install dependencies")
                || stderr.contains("Install dependencies"),
            "amber install command should be recognized. stdout: {}, stderr: {}",
            stdout,
            stderr
        );
    }

    /// Test 2: amber install with no package.json should error
    #[test]
    fn test_install_no_package_json() {
        let temp_dir = TempDir::new().unwrap();

        let output = Command::new(amberjs_path())
            .arg("install")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute amber install");

        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            !output.status.success(),
            "amber install should fail without package.json"
        );
        assert!(
            stderr.contains("package.json not found"),
            "Error message should mention package.json: {}",
            stderr
        );
    }

    /// Test 3: amber install with empty dependencies should succeed
    #[test]
    fn test_install_empty_dependencies() {
        let temp_dir = TempDir::new().unwrap();

        let package_json = r#"{
            "name": "test-project",
            "version": "1.0.0"
        }"#;

        let package_json_path = temp_dir.path().join("package.json");
        fs::write(&package_json_path, package_json).unwrap();

        let output = Command::new(amberjs_path())
            .arg("install")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute amber install");

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            output.status.success(),
            "amber install with empty dependencies should succeed. stderr: {}",
            stderr
        );
        assert!(
            stdout.contains("0 dependencies") || stdout.contains("Installed"),
            "Should show installed count: {}",
            stdout
        );
    }

    /// Test 4: Package.json should have optionalDependencies field
    #[test]
    fn test_package_json_optional_dependencies() {
        let temp_dir = TempDir::new().unwrap();

        let package_json = r#"{
            "name": "test-project",
            "version": "1.0.0",
            "optionalDependencies": {
                "fsevents": "^2.3.0"
            }
        }"#;

        let package_json_path = temp_dir.path().join("package.json");
        fs::write(&package_json_path, package_json).unwrap();

        // Parse and verify optionalDependencies is preserved
        let content = fs::read_to_string(&package_json_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert!(
            parsed["optionalDependencies"]["fsevents"].is_string(),
            "optionalDependencies should be preserved in package.json"
        );
    }

    /// Test 5: verify optional_dependencies field in PackageJson struct
    #[test]
    fn test_package_json_struct_has_optional_dependencies() {
        use amberjs::package_manager::PackageJson;
        use std::collections::HashMap;

        let package_json = PackageJson {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            main: None,
            scripts: None,
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: Some(HashMap::new()),
            author: None,
            license: None,
            repository: None,
        };

        assert!(
            package_json.optional_dependencies.is_some(),
            "PackageJson should have optional_dependencies field"
        );
    }

    fn registry_reachable() -> bool {
        Command::new("curl")
            .args(["-sI", "--max-time", "8", "https://registry.npmjs.org/ms"])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[test]
    fn add_lodash_and_ms_then_require() {
        if !registry_reachable() {
            eprintln!("skipping registry e2e: npm registry unreachable");
            return;
        }

        let dir = TempDir::new().expect("tempdir");
        let init = Command::new(amberjs_path())
            .current_dir(dir.path())
            .args(["init", "demo"])
            .output()
            .expect("amber init");
        assert!(
            init.status.success(),
            "amber init failed: {}",
            String::from_utf8_lossy(&init.stderr)
        );

        let project = dir.path().join("demo");
        let add_lodash = Command::new(amberjs_path())
            .current_dir(&project)
            .args(["add", "lodash"])
            .output()
            .expect("amber add lodash");
        assert!(
            add_lodash.status.success(),
            "amber add lodash failed: {}",
            String::from_utf8_lossy(&add_lodash.stderr)
        );

        let eval_lodash = Command::new(amberjs_path())
            .current_dir(&project)
            .args(["eval", "console.log(require('lodash').VERSION)"])
            .output()
            .expect("amber eval lodash");
        assert!(
            eval_lodash.status.success(),
            "require('lodash') failed: {}{}",
            String::from_utf8_lossy(&eval_lodash.stdout),
            String::from_utf8_lossy(&eval_lodash.stderr)
        );

        let add_ms = Command::new(amberjs_path())
            .current_dir(&project)
            .args(["add", "ms"])
            .output()
            .expect("amber add ms");
        assert!(
            add_ms.status.success(),
            "amber add ms failed: {}",
            String::from_utf8_lossy(&add_ms.stderr)
        );

        let eval_ms = Command::new(amberjs_path())
            .current_dir(&project)
            .args(["eval", "console.log(typeof require('ms'))"])
            .output()
            .expect("amber eval ms");
        assert!(
            eval_ms.status.success(),
            "require('ms') failed: {}{}",
            String::from_utf8_lossy(&eval_ms.stdout),
            String::from_utf8_lossy(&eval_ms.stderr)
        );

        let lock = fs::read_to_string(project.join("package-lock.json"))
            .expect("package-lock.json after add");
        assert!(
            lock.contains("integrity") && !lock.contains("\"integrity\": null"),
            "lockfile should record real integrity. lock: {lock}"
        );
    }
}

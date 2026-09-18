// amber prune command tests
// v0.3.230 - Test coverage for amber prune command

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Get the path to the amber binary built by cargo for integration tests.
fn amberjs_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_amber"))
}

#[cfg(test)]
mod prune_command_tests {
    use super::*;

    /// Test 1: amber prune command should be recognized
    #[test]
    fn test_prune_command_exists() {
        let amberjs = amberjs_path();
        assert!(
            amberjs.exists(),
            "amber binary should exist at {:?}",
            amberjs
        );

        let output = Command::new(&amberjs)
            .arg("prune")
            .arg("--help")
            .output()
            .expect("Failed to execute amber prune --help");

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Should have successful exit code or show help
        assert!(
            output.status.success()
                || stdout.contains("Remove unused dependencies")
                || stderr.contains("Remove unused dependencies"),
            "amber prune command should be recognized. stdout: {}, stderr: {}",
            stdout,
            stderr
        );
    }

    /// Test 2: amber prune with no package.json should error
    #[test]
    fn test_prune_no_package_json() {
        let temp_dir = TempDir::new().unwrap();

        let output = Command::new(amberjs_path())
            .arg("prune")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute amber prune");

        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            !output.status.success(),
            "amber prune should fail without package.json"
        );
        assert!(
            stderr.contains("package.json not found"),
            "Error message should mention package.json: {}",
            stderr
        );
    }

    /// Test 3: amber prune with no node_modules should succeed
    #[test]
    fn test_prune_no_node_modules() {
        let temp_dir = TempDir::new().unwrap();

        let package_json = r#"{
            "name": "test-project",
            "version": "1.0.0"
        }"#;

        let package_json_path = temp_dir.path().join("package.json");
        fs::write(&package_json_path, package_json).unwrap();

        let output = Command::new(amberjs_path())
            .arg("prune")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute amber prune");

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            output.status.success(),
            "amber prune with no node_modules should succeed. stderr: {}",
            stderr
        );
        assert!(
            stdout.contains("nothing to prune") || stdout.contains("clean"),
            "Should indicate nothing to prune: {}",
            stdout
        );
    }

    /// Test 4: amber prune should preserve declared dependencies
    #[test]
    fn test_prune_preserves_declared_deps() {
        let temp_dir = TempDir::new().unwrap();

        let package_json = r#"{
            "name": "test-project",
            "version": "1.0.0",
            "dependencies": {
                "lodash": "4.17.21"
            }
        }"#;

        let package_json_path = temp_dir.path().join("package.json");
        fs::write(&package_json_path, package_json).unwrap();

        // Create node_modules with lodash (declared)
        let node_modules = temp_dir.path().join("node_modules");
        fs::create_dir_all(&node_modules).unwrap();
        let lodash = node_modules.join("lodash");
        fs::create_dir_all(&lodash).unwrap();
        fs::write(
            lodash.join("package.json"),
            r#"{"name":"lodash","version":"4.17.21"}"#,
        )
        .unwrap();

        let output = Command::new(amberjs_path())
            .arg("prune")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute amber prune");

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            output.status.success(),
            "amber prune should succeed. stderr: {}",
            stderr
        );
        // lodash should NOT be removed since it's declared
        assert!(
            !stdout.contains("lodash"),
            "Should not remove declared dependency lodash: {}",
            stdout
        );
    }

    /// Test 5: amber prune should remove undeclared packages
    #[test]
    fn test_prune_removes_undeclared() {
        let temp_dir = TempDir::new().unwrap();

        let package_json = r#"{
            "name": "test-project",
            "version": "1.0.0",
            "dependencies": {
                "lodash": "4.17.21"
            }
        }"#;

        let package_json_path = temp_dir.path().join("package.json");
        fs::write(&package_json_path, package_json).unwrap();

        // Create node_modules with both lodash (declared) and undeclared package
        let node_modules = temp_dir.path().join("node_modules");
        fs::create_dir_all(&node_modules).unwrap();

        // lodash (declared - should remain)
        let lodash = node_modules.join("lodash");
        fs::create_dir_all(&lodash).unwrap();
        fs::write(
            lodash.join("package.json"),
            r#"{"name":"lodash","version":"4.17.21"}"#,
        )
        .unwrap();

        // undeclared-package (not declared - should be removed)
        let undeclared = node_modules.join("undeclared-package");
        fs::create_dir_all(&undeclared).unwrap();
        fs::write(
            undeclared.join("package.json"),
            r#"{"name":"undeclared-package","version":"1.0.0"}"#,
        )
        .unwrap();

        let output = Command::new(amberjs_path())
            .arg("prune")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute amber prune");

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            output.status.success(),
            "amber prune should succeed. stderr: {}",
            stderr
        );
        // undeclared-package should be removed
        assert!(
            stdout.contains("undeclared-package"),
            "Should remove undeclared package: {}",
            stdout
        );
    }

    /// Test 6: verify prune method in PackageManager
    #[test]
    fn test_package_manager_prune() {
        use amberjs::package_manager::{PackageJson, PackageManager, PackageManagerConfig};

        let temp_dir = TempDir::new().unwrap();
        let config = PackageManagerConfig {
            cache_dir: temp_dir.path().join("cache"),
            node_modules_dir: temp_dir.path().join("node_modules"),
            ..Default::default()
        };

        let pm = PackageManager::new(config).unwrap();

        // Create node_modules with a package
        let node_modules = temp_dir.path().join("node_modules");
        fs::create_dir_all(&node_modules).unwrap();

        let undeclared = node_modules.join("undeclared-pkg");
        fs::create_dir_all(&undeclared).unwrap();
        fs::write(
            undeclared.join("package.json"),
            r#"{"name":"undeclared-pkg","version":"1.0.0"}"#,
        )
        .unwrap();

        // Create package.json with no dependencies
        let package_json = PackageJson {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            main: None,
            scripts: None,
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            author: None,
            license: None,
            repository: None,
        };

        let removed = pm.prune(&package_json).unwrap();

        assert!(
            removed.contains(&"undeclared-pkg".to_string()),
            "Should have removed undeclared-pkg: {:?}",
            removed
        );
    }

    /// Test 7: prune handles scoped packages correctly
    #[test]
    fn test_prune_scoped_packages() {
        let temp_dir = TempDir::new().unwrap();

        let package_json = r#"{
            "name": "test-project",
            "version": "1.0.0",
            "dependencies": {
                "@babel/core": "7.0.0"
            }
        }"#;

        let package_json_path = temp_dir.path().join("package.json");
        fs::write(&package_json_path, package_json).unwrap();

        // Create node_modules with @babel/core (declared) and @other/pkg (not declared)
        let node_modules = temp_dir.path().join("node_modules");
        fs::create_dir_all(&node_modules).unwrap();

        // @babel/core (declared - should remain)
        let babel_core = node_modules.join("@babel").join("core");
        fs::create_dir_all(&babel_core).unwrap();
        fs::write(
            babel_core.join("package.json"),
            r#"{"name":"@babel/core","version":"7.0.0"}"#,
        )
        .unwrap();

        // @other/pkg (not declared - should be removed)
        let other_pkg = node_modules.join("@other").join("pkg");
        fs::create_dir_all(&other_pkg).unwrap();
        fs::write(
            other_pkg.join("package.json"),
            r#"{"name":"@other/pkg","version":"1.0.0"}"#,
        )
        .unwrap();

        let output = Command::new(amberjs_path())
            .arg("prune")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute amber prune");

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            output.status.success(),
            "amber prune should succeed. stderr: {}",
            stderr
        );
        // @other/pkg should be removed
        assert!(
            stdout.contains("@other/pkg"),
            "Should remove undeclared scoped package: {}",
            stdout
        );
    }
}

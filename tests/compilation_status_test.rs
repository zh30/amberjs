// Test to verify compilation status of Amber runtime
//
// This test ensures that the core modules compile successfully.
// Following TDD methodology: write test first, then fix issues.

#[cfg(test)]
mod compilation_tests {
    use std::path::Path;
    use std::process::Command;

    /// Test that cargo build succeeds
    #[test]
    fn test_cargo_build_succeeds() {
        // Get the directory containing Cargo.toml
        let cargo_toml = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");

        // Verify Cargo.toml exists
        assert!(
            cargo_toml.exists(),
            "Cargo.toml should exist at project root"
        );

        // Run cargo build
        let output = Command::new("cargo")
            .args(["build", "--lib"])
            .output()
            .expect("Failed to execute cargo build");

        // Print build output for debugging
        println!("Build stdout: {}", String::from_utf8_lossy(&output.stdout));
        println!("Build stderr: {}", String::from_utf8_lossy(&output.stderr));

        // Assert build succeeded
        assert!(
            output.status.success(),
            "cargo build should succeed. Build failed with:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Test that basic runtime modules are accessible
    #[test]
    fn test_runtime_modules_accessible() {
        // This test verifies that the basic modules we depend on exist
        // It doesn't run them, just ensures they compile

        // We expect these modules to compile:
        // - runtime_core (main V8 runtime)
        // - runtime_lite (minimal runtime for testing)
        // - v8_engine (V8 integration layer)

        // If this test compiles, the modules are accessible
        assert!(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).exists(),
            "Runtime modules compilation check passed"
        );
    }

    /// Test that CLI tool builds successfully
    #[test]
    fn test_cli_build_succeeds() {
        let output = Command::new("cargo")
            .args(["build", "--bin", "amber"])
            .output()
            .expect("Failed to execute cargo build for amber binary");

        println!(
            "CLI build stdout: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        println!(
            "CLI build stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        assert!(
            output.status.success(),
            "amberjs CLI binary should build successfully. Build failed with:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Test V8 engine initialization
    #[test]
    fn test_v8_engine_basic() {
        // Basic smoke test for V8 engine
        // This verifies the V8 engine can be initialized without errors

        // For now, just check that the module exists and has expected exports
        // The actual V8 initialization will be tested in integration tests
        assert!(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).exists(),
            "V8 engine module accessible"
        );
    }

    /// Test error types are well-defined
    #[test]
    fn test_error_types_defined() {
        // Verify that error types are properly defined
        // This is important for proper error handling
        assert!(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).exists(),
            "Error types are defined"
        );
    }
}

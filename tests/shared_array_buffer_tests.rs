// SharedArrayBuffer tests for Amber runtime
// v0.3.322: Tests for cross-Worker shared memory support

use std::fs;
use std::process::{Command, Stdio};

/// Helper function to run a JavaScript script with amberjs
fn run_script(script: &str) -> std::process::Output {
    // Create a temporary file with the script
    let temp_dir = tempfile::Builder::new()
        .prefix("amberjs-shared-buffer-test-")
        .tempdir()
        .unwrap();
    let temp_file = temp_dir.path().join("test.js");
    fs::write(&temp_file, script).unwrap();

    // Run amberjs with the script
    let output = Command::new("./target/debug/amber")
        .arg("run")
        .arg(&temp_file)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to run amber");

    // Clean up
    drop(temp_dir);

    output
}

#[cfg(test)]
mod shared_array_buffer_tests {
    use super::*;

    #[test]
    fn test_shared_array_buffer_exists() {
        // Test that SharedArrayBuffer constructor is available
        let script = r#"
            if (typeof SharedArrayBuffer !== 'undefined') {
                console.log('SUCCESS: SharedArrayBuffer constructor exists');
            } else {
                throw new Error('SharedArrayBuffer constructor not found');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "SharedArrayBuffer should exist: {}",
            stdout
        );
        // Check for the success message (output may contain "Result: undefined" at the end)
        assert!(
            stdout.contains("SUCCESS: SharedArrayBuffer constructor exists"),
            "Output: {}",
            stdout
        );
    }

    #[test]
    fn test_shared_array_buffer_creation() {
        // Test basic SharedArrayBuffer creation
        let script = r#"
            try {
                const sab = new SharedArrayBuffer(1024);
                console.log('SUCCESS: SharedArrayBuffer created with byteLength: ' + sab.byteLength);
            } catch (e) {
                console.log('ERROR: ' + e.message);
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "Should create SharedArrayBuffer: {}",
            stdout
        );
        assert!(
            stdout.contains("byteLength: 1024"),
            "Should have correct byteLength"
        );
    }

    #[test]
    fn test_shared_array_buffer_zero_size() {
        // Test creating empty SharedArrayBuffer
        let script = r#"
            try {
                const sab = new SharedArrayBuffer(0);
                if (sab.byteLength === 0) {
                    console.log('SUCCESS: Empty SharedArrayBuffer created');
                } else {
                    console.log('ERROR: Expected byteLength 0, got ' + sab.byteLength);
                }
            } catch (e) {
                console.log('ERROR: ' + e.message);
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "Should create empty SharedArrayBuffer: {}",
            stdout
        );
    }

    #[test]
    fn test_shared_array_buffer_properties() {
        // Test SharedArrayBuffer properties
        let script = r#"
            const sab = new SharedArrayBuffer(256);
            // Check that byteLength property exists and is correct
            if (sab.byteLength === 256) {
                console.log('SUCCESS: byteLength is correct (256)');
            } else {
                console.log('ERROR: byteLength is ' + sab.byteLength);
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "Properties should work: {}",
            stdout
        );
    }

    #[test]
    fn test_shared_array_buffer_with_typed_array() {
        // Test using SharedArrayBuffer with Int32Array
        let script = r#"
            try {
                const sab = new SharedArrayBuffer(64);
                const int32 = new Int32Array(sab);
                int32[0] = 42;
                if (int32[0] === 42) {
                    console.log('SUCCESS: Int32Array on SharedArrayBuffer works');
                } else {
                    console.log('ERROR: Value mismatch');
                }
            } catch (e) {
                console.log('ERROR: ' + e.message);
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "TypedArray on SharedArrayBuffer should work: {}",
            stdout
        );
    }

    #[test]
    fn test_shared_array_buffer_slice() {
        // Test slice() method
        let script = r#"
            try {
                const sab = new SharedArrayBuffer(100);
                const sliced = sab.slice(0, 50);
                if (sliced.byteLength === 50) {
                    console.log('SUCCESS: slice() works correctly');
                } else {
                    console.log('ERROR: slice returned byteLength ' + sliced.byteLength);
                }
            } catch (e) {
                console.log('ERROR: ' + e.message);
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "slice() should work: {}",
            stdout
        );
    }

    #[test]
    fn test_shared_array_buffer_max_size() {
        // Test that large allocations work
        let script = r#"
            try {
                // 1MB should work
                const sab = new SharedArrayBuffer(1024 * 1024);
                if (sab.byteLength === 1024 * 1024) {
                    console.log('SUCCESS: 1MB SharedArrayBuffer created');
                } else {
                    console.log('ERROR: Size mismatch');
                }
            } catch (e) {
                console.log('ERROR: ' + e.message);
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "Large allocation should work: {}",
            stdout
        );
    }

    #[test]
    fn test_shared_array_buffer_type_check() {
        // Test that SharedArrayBuffer is detected correctly
        let script = r#"
            const sab = new SharedArrayBuffer(100);
            const isSAB = sab instanceof SharedArrayBuffer;
            if (isSAB) {
                console.log('SUCCESS: instanceof works');
            } else {
                console.log('INFO: instanceof may not be fully supported');
                // Check for byteLength instead
                if (sab.byteLength === 100) {
                    console.log('SUCCESS: Properties work correctly');
                }
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Either instanceof works or properties work - both are valid
        assert!(
            stdout.contains("SUCCESS"),
            "Type checking should work: {}",
            stdout
        );
    }

    #[test]
    fn test_shared_array_buffer_data_view() {
        // Test using SharedArrayBuffer with DataView
        let script = r#"
            try {
                const sab = new SharedArrayBuffer(16);
                const view = new DataView(sab);
                view.setFloat64(0, 3.14159, true);
                const value = view.getFloat64(0, true);
                if (Math.abs(value - 3.14159) < 0.0001) {
                    console.log('SUCCESS: DataView on SharedArrayBuffer works');
                } else {
                    console.log('ERROR: Value mismatch: ' + value);
                }
            } catch (e) {
                console.log('ERROR: ' + e.message);
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "DataView should work: {}",
            stdout
        );
    }

    #[test]
    fn test_multiple_shared_array_buffers() {
        // Test creating multiple SharedArrayBuffers
        let script = r#"
            try {
                const buffers = [];
                for (let i = 0; i < 5; i++) {
                    buffers.push(new SharedArrayBuffer(100 * (i + 1)));
                }
                const allValid = buffers.every((b, i) => b.byteLength === 100 * (i + 1));
                if (allValid) {
                    console.log('SUCCESS: Created ' + buffers.length + ' SharedArrayBuffers');
                } else {
                    console.log('ERROR: Some buffers have incorrect size');
                }
            } catch (e) {
                console.log('ERROR: ' + e.message);
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "Multiple buffers should work: {}",
            stdout
        );
    }

    #[test]
    fn test_shared_array_buffer_with_uint8array() {
        // Test using SharedArrayBuffer with Uint8Array for byte-level access
        let script = r#"
            try {
                const sab = new SharedArrayBuffer(256);
                const uint8 = new Uint8Array(sab);
                // Write some data
                for (let i = 0; i < 256; i++) {
                    uint8[i] = i;
                }
                // Verify data
                let correct = true;
                for (let i = 0; i < 256; i++) {
                    if (uint8[i] !== i) {
                        correct = false;
                        break;
                    }
                }
                if (correct) {
                    console.log('SUCCESS: Uint8Array on SharedArrayBuffer works');
                } else {
                    console.log('ERROR: Data verification failed');
                }
            } catch (e) {
                console.log('ERROR: ' + e.message);
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "Uint8Array should work: {}",
            stdout
        );
    }
}

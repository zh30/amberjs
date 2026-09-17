// ArrayBuffer Transfer API Tests for Amber
// v0.3.311: Tests for zero-copy ArrayBuffer detach and transfer operations
// Critical for AI workloads that need to pass large buffers efficiently

#[cfg(test)]
mod array_buffer_transfer_tests {
    use std::path::PathBuf;
    use std::process::Command;

    fn amberjs_path() -> PathBuf {
        PathBuf::from(
            std::env::var("CARGO_BIN_EXE_amber").unwrap_or_else(|_| "./target/debug/amber".to_string()),
        )
    }

    /// Test 1: Basic ArrayBuffer creation
    #[test]
    fn test_array_buffer_creation() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                const buffer = new ArrayBuffer(16);
                console.log('byteLength:', buffer.byteLength);
                buffer.byteLength === 16
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("byteLength: 16"),
            "Expected byteLength to be 16. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("true"),
            "Expected test to pass. Got: {}",
            stdout
        );
    }

    /// Test 2: transferToAttached basic functionality
    #[test]
    fn test_transfer_to_attached_basic() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                const buffer = new ArrayBuffer(32);
                const result = transferToAttached(buffer);
                console.log('transferred length:', result);
                console.log('original byteLength:', buffer.byteLength);
                result === 32 && buffer.byteLength === 0
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("transferred length: 32"),
            "Expected transferred length 32. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("original byteLength: 0"),
            "Expected byteLength 0 after transfer. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("true"),
            "Expected test to pass. Got: {}",
            stdout
        );
    }

    /// Test 3: transferToAttached with zero-sized buffer - throws error as considered already detached
    #[test]
    fn test_transfer_zero_sized_buffer() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                const buffer = new ArrayBuffer(0);
                try {
                    transferToAttached(buffer);
                    false;
                } catch(e) {
                    e.message.includes('already detached')
                }
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Zero-sized buffers are considered already detached
        assert!(
            stdout.contains("true"),
            "Zero buffer should throw 'already detached'. Got: {}",
            stdout
        );
    }

    /// Test 4: detachArrayBuffer basic functionality
    #[test]
    fn test_detach_array_buffer() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                const buffer = new ArrayBuffer(64);
                detachArrayBuffer(buffer);
                buffer.byteLength === 0
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected byteLength to be 0 after detach. Got: {}",
            stdout
        );
    }

    /// Test 5: transferToAttached with invalid argument
    #[test]
    fn test_transfer_invalid_argument() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                try {
                    transferToAttached('not a buffer');
                    false;
                } catch(e) {
                    e.message.includes('ArrayBuffer')
                }
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected error for non-ArrayBuffer argument. Got: {}",
            stdout
        );
    }

    /// Test 6: detachArrayBuffer with invalid argument
    #[test]
    fn test_detach_invalid_argument() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                try {
                    detachArrayBuffer(123);
                    false;
                } catch(e) {
                    e.message.includes('ArrayBuffer')
                }
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected error for non-ArrayBuffer argument. Got: {}",
            stdout
        );
    }

    /// Test 7: Accessing detached buffer throws error
    #[test]
    fn test_access_detached_buffer_throws() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                const buffer = new ArrayBuffer(16);
                transferToAttached(buffer);
                try {
                    new Uint8Array(buffer);
                    false;
                } catch(e) {
                    e.message.includes('ArrayBuffer') || e.message.includes('detached')
                }
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected error when accessing detached buffer. Got: {}",
            stdout
        );
    }

    /// Test 8: Multiple buffers can be transferred independently
    #[test]
    fn test_multiple_buffer_transfer() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                const buf1 = new ArrayBuffer(100);
                const buf2 = new ArrayBuffer(200);
                const result1 = transferToAttached(buf1);
                const result2 = transferToAttached(buf2);
                result1 === 100 && result2 === 200 && buf1.byteLength === 0 && buf2.byteLength === 0
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected multiple buffers to transfer independently. Got: {}",
            stdout
        );
    }

    /// Test 9: Large buffer transfer (AI workload simulation)
    #[test]
    fn test_large_buffer_transfer() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                // Simulate a large AI tensor buffer (10MB)
                const largeBuffer = new ArrayBuffer(10 * 1024 * 1024);
                const transferred = transferToAttached(largeBuffer);
                console.log('Large buffer transferred:', transferred);
                transferred === 10485760 && largeBuffer.byteLength === 0
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Large buffer transferred: 10485760"),
            "Expected 10MB transfer. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("true"),
            "Expected test to pass. Got: {}",
            stdout
        );
    }

    /// Test 10: structuredClone still works for deep cloning (transfer not yet implemented in structuredClone)
    #[test]
    fn test_structured_clone_deep_clone() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const buffer = new ArrayBuffer(50);
                const original = { data: buffer, name: 'test' };
                const cloned = structuredClone(original);
                console.log('original buffer byteLength:', original.data.byteLength);
                console.log('cloned buffer byteLength:', cloned.data.byteLength);
                console.log('are they same buffer:', original.data === cloned.data);
                // structuredClone creates a copy, not a transfer
                original.data.byteLength === 50 && cloned.data.byteLength === 50 && original.data !== cloned.data
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("original buffer byteLength: 50"),
            "Original should still have data (copy, not transfer). Got: {}",
            stdout
        );
        assert!(
            stdout.contains("cloned buffer byteLength: 50"),
            "Clone should have copy of data. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("are they same buffer: false"),
            "Clone should be different object. Got: {}",
            stdout
        );
    }

    /// Test 11: transferFromAttached basic functionality
    #[test]
    fn test_transfer_from_attached() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                const buffer = new ArrayBuffer(42);
                transferToAttached(buffer);
                // Note: transferFromAttached is for receiving transferred buffers
                // In single context, the buffer is already attached after detach
                const result = transferFromAttached(buffer);
                console.log('transferFromAttached returned buffer:', result instanceof ArrayBuffer);
                result instanceof ArrayBuffer
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected transferFromAttached to return buffer. Got: {}",
            stdout
        );
    }
}

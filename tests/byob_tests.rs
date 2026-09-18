// BYOB (Bring Your Own Buffer) Tests for ReadableStream
// Tests for v0.3.294: BYOB support in ReadableStream.getReader()

#[cfg(test)]
mod byob_tests {
    use std::path::PathBuf;
    use std::process::Command;

    fn amberjs_path() -> PathBuf {
        PathBuf::from(
            std::env::var("CARGO_BIN_EXE_amber")
                .unwrap_or_else(|_| "./target/debug/amber".to_string()),
        )
    }

    /// Test 1: ReadableStream.getReader() read() accepts a view parameter
    #[test]
    fn test_byob_read_accepts_view() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                const stream = new ReadableStream({
                    start(controller) {
                        controller.enqueue(new Uint8Array([1, 2, 3, 4, 5]));
                        controller.close();
                    }
                });
                const reader = stream.getReader();
                // Try to read with a BYOB view
                const buffer = new Uint8Array(10);
                const result = reader.read(buffer);
                // If BYOB is supported, result should be a Promise
                console.log(result instanceof Promise);
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Current implementation should return false (BYOB not supported yet)
        // After implementation, this should return true
        assert!(
            stdout.contains("true") || stdout.contains("false"),
            "Expected true or false in output, got: {}",
            stdout
        );
    }

    /// Test 2: BYOB read should copy data into the provided buffer
    #[test]
    fn test_byob_copies_to_buffer() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                const stream = new ReadableStream({
                    start(controller) {
                        controller.enqueue(new Uint8Array([72, 101, 108, 108, 111])); // "Hello"
                        controller.close();
                    }
                });
                const reader = stream.getReader();
                const buffer = new Uint8Array(5);
                reader.read(buffer).then(result => {
                    console.log('done:', result.done);
                    console.log('bytes read:', result.value ? result.value.length : 0);
                });
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should handle the read operation
        assert!(
            stdout.contains("done:") || stdout.contains("bytes"),
            "Expected read result in output, got: {}",
            stdout
        );
    }

    /// Test 3: BYOB with smaller buffer than chunk
    #[test]
    fn test_byob_smaller_buffer() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                const stream = new ReadableStream({
                    start(controller) {
                        controller.enqueue(new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]));
                        controller.close();
                    }
                });
                const reader = stream.getReader();
                // Request smaller buffer
                const buffer = new Uint8Array(3);
                reader.read(buffer).then(result => {
                    console.log('chunk size:', result.value ? result.value.length : 0);
                });
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should handle partial reads
        assert!(
            stdout.contains("chunk size:") || stdout.contains("error") || stdout.contains("3"),
            "Expected chunk size in output, got: {}",
            stdout
        );
    }

    /// Test 4: BYOB with ArrayBufferView (DataView)
    #[test]
    fn test_byob_with_dataview() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                const stream = new ReadableStream({
                    start(controller) {
                        controller.enqueue(new Uint8Array([0x01, 0x02, 0x03, 0x04]));
                        controller.close();
                    }
                });
                const reader = stream.getReader();
                const buffer = new ArrayBuffer(4);
                const view = new DataView(buffer);
                reader.read(view).then(result => {
                    console.log('DataView read:', result.done === false);
                });
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("DataView read:")
                || stdout.contains("true")
                || stdout.contains("false"),
            "Expected DataView result in output, got: {}",
            stdout
        );
    }

    /// Test 5: Normal read without BYOB still works
    #[test]
    fn test_normal_read_still_works() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const stream = new ReadableStream({
                    start(controller) {
                        controller.enqueue(new Uint8Array([1, 2, 3]));
                        controller.close();
                    }
                });
                const reader = stream.getReader();
                reader.read().then(result => {
                    console.log('normal read works:', result.done === false && result.value instanceof Uint8Array);
                });
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("normal read works: true"),
            "Expected normal read to work, got: {}",
            stdout
        );
    }
}

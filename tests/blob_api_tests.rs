// Blob API Tests for Beejs
// Tests for v0.3.305: Blob/File API implementation for binary data handling
// Enables efficient binary data handling for AI workloads

#[cfg(test)]
mod blob_api_tests {
    use std::path::PathBuf;
    use std::process::Command;

    fn beejs_path() -> PathBuf {
        PathBuf::from(
            std::env::var("CARGO_BIN_EXE_amber")
                .unwrap_or_else(|_| "./target/debug/bee".to_string()),
        )
    }

    /// Test 1: Blob constructor with string parts
    #[test]
    fn test_blob_constructor_string() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const blob = new Blob(['Hello, World!'], { type: 'text/plain' });
                console.log('blob.size:', blob.size);
                console.log('blob.type:', blob.type);
                console.log('blob instanceof Blob:', blob instanceof Blob);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("blob.size: 13"),
            "Expected blob.size 13. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("blob.type: text/plain"),
            "Expected blob.type text/plain. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("blob instanceof Blob: true"),
            "Expected Blob instance. Got: {}",
            stdout
        );
    }

    /// Test 2: Blob constructor with empty parts
    #[test]
    fn test_blob_constructor_empty() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const blob = new Blob([], { type: 'application/octet-stream' });
                console.log('empty blob size:', blob.size);
                console.log('empty blob type:', blob.type);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("empty blob size: 0"),
            "Expected empty blob size 0. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("empty blob type: application/octet-stream"),
            "Expected octet-stream type. Got: {}",
            stdout
        );
    }

    /// Test 3: Blob constructor with multiple parts
    #[test]
    fn test_blob_constructor_multiple_parts() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const blob = new Blob(['Hello', ' ', 'World'], { type: 'text/plain' });
                console.log('multi part size:', blob.size);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("multi part size: 11"),
            "Expected size 11 (Hello + space + World). Got: {}",
            stdout
        );
    }

    /// Test 4: Blob.text() method
    #[test]
    fn test_blob_text() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const blob = new Blob(['Hello, Beejs!'], { type: 'text/plain' });
                const text = blob.text();
                console.log('blob text:', text);
                console.log('text correct:', text === 'Hello, Beejs!');
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("blob text: Hello, Beejs!"),
            "Expected correct text. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("text correct: true"),
            "Expected text to match. Got: {}",
            stdout
        );
    }

    /// Test 5: Blob.slice() method
    #[test]
    fn test_blob_slice() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const blob = new Blob(['Hello, World!'], { type: 'text/plain' });
                const sliced = blob.slice(0, 5);
                console.log('sliced size:', sliced.size);
                console.log('sliced type:', sliced.type);
                console.log('sliced text:', sliced.text());
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("sliced size: 5"),
            "Expected sliced size 5. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("sliced text: Hello"),
            "Expected sliced text 'Hello'. Got: {}",
            stdout
        );
    }

    /// Test 6: Blob.slice() with negative start
    #[test]
    fn test_blob_slice_negative_start() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const blob = new Blob(['Hello, World!'], { type: 'text/plain' });
                const sliced = blob.slice(-6, 12); // Last 6 chars
                console.log('slice from end size:', sliced.size);
                console.log('slice from end text:', sliced.text());
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("slice from end size: 5"),
            "Expected size 5. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("slice from end text: World"),
            "Expected text 'World'. Got: {}",
            stdout
        );
    }

    /// Test 7: Blob.slice() with content type
    #[test]
    fn test_blob_slice_with_content_type() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const blob = new Blob(['Hello, World!'], { type: 'text/plain' });
                const sliced = blob.slice(0, 5, 'text/html');
                console.log('sliced with type:', sliced.type);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("sliced with type: text/html"),
            "Expected content type override. Got: {}",
            stdout
        );
    }

    /// Test 8: Blob.stream() method
    #[test]
    fn test_blob_stream() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const blob = new Blob(['Hello, World!'], { type: 'text/plain' });
                const stream = blob.stream();
                console.log('stream exists:', typeof stream === 'object');
                console.log('stream.getReader exists:', typeof stream.getReader === 'function');
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("stream exists: true"),
            "Expected stream to exist. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("stream.getReader exists: true"),
            "Expected getReader method. Got: {}",
            stdout
        );
    }

    /// Test 9: File constructor
    #[test]
    fn test_file_constructor() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const file = new File(['file content here'], 'test.txt', { type: 'text/plain' });
                console.log('file.name:', file.name);
                console.log('file.size:', file.size);
                console.log('file.type:', file.type);
                console.log('file instanceof File:', file instanceof File);
                console.log('file instanceof Blob:', file instanceof Blob);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("file.name: test.txt"),
            "Expected file name test.txt. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("file.size: 17"),
            "Expected file size 17. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("file.type: text/plain"),
            "Expected file type text/plain. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("file instanceof File: true"),
            "Expected File instance. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("file instanceof Blob: true"),
            "Expected File to be Blob instance. Got: {}",
            stdout
        );
    }

    /// Test 10: File with lastModified
    #[test]
    fn test_file_with_last_modified() {
        let output = Command::new(beejs_path())
            .args(["eval", r#"
                const file = new File(['content'], 'doc.txt', { type: 'text/plain', lastModified: 1234567890 });
                console.log('file.name:', file.name);
            "#])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("file.name: doc.txt"),
            "Expected file name doc.txt. Got: {}",
            stdout
        );
    }

    /// Test 11: Blob with Unicode content
    #[test]
    fn test_blob_unicode() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const blob = new Blob(['Hello, 世界! 🐝'], { type: 'text/plain;charset=utf-8' });
                console.log('unicode blob size:', blob.size);
                const text = blob.text();
                console.log('unicode text:', text);
                console.log('contains emoji:', text.includes('🐝'));
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("contains emoji: true"),
            "Expected emoji in text. Got: {}",
            stdout
        );
    }

    /// Test 12: Blob with binary data
    #[test]
    fn test_blob_binary() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                // Create blob with bytes that include null and control characters
                const bytes = [0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x00, 0x01];
                const chars = String.fromCharCode.apply(null, bytes);
                const blob = new Blob([chars], { type: 'application/octet-stream' });
                console.log('binary blob size:', blob.size);
                console.log('contains null byte:', blob.size === 7);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("binary blob size: 7"),
            "Expected size 7. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("contains null byte: true"),
            "Expected null byte preserved. Got: {}",
            stdout
        );
    }

    /// Test 13: Blob methods are on prototype
    #[test]
    fn test_blob_methods_on_prototype() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const blob = new Blob(['test'], { type: 'text/plain' });
                console.log('text is function:', typeof blob.text === 'function');
                console.log('slice is function:', typeof blob.slice === 'function');
                console.log('stream is function:', typeof blob.stream === 'function');
                console.log('arrayBuffer is function:', typeof blob.arrayBuffer === 'function');
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("text is function: true"),
            "Expected text method. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("slice is function: true"),
            "Expected slice method. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("stream is function: true"),
            "Expected stream method. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("arrayBuffer is function: true"),
            "Expected arrayBuffer method. Got: {}",
            stdout
        );
    }

    /// Test 13b: Blob.arrayBuffer() returns stored bytes
    #[test]
    fn test_blob_array_buffer_returns_bytes() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const blob = new Blob(['abc']);
                const bytes = Array.from(new Uint8Array(blob.arrayBuffer())).join(',');
                console.log('arrayBuffer bytes:', bytes);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("arrayBuffer bytes: 97,98,99"),
            "Expected Blob.arrayBuffer() to expose stored bytes. Got: {}",
            stdout
        );
    }

    /// Test 13c: Blob.arrayBuffer() preserves non-UTF-8 bytes
    #[test]
    fn test_blob_array_buffer_preserves_uint8array_bytes() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const blob = new Blob([new Uint8Array([0, 255, 65])]);
                const bytes = Array.from(new Uint8Array(blob.arrayBuffer())).join(',');
                console.log('arrayBuffer binary bytes:', bytes);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("arrayBuffer binary bytes: 0,255,65"),
            "Expected Blob.arrayBuffer() to preserve binary bytes. Got: {}",
            stdout
        );
    }

    /// Test 14: File inherits all Blob methods
    #[test]
    fn test_file_inherits_blob_methods() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const file = new File(['file content'], 'test.txt', { type: 'text/plain' });
                console.log('file.text method:', typeof file.text === 'function');
                console.log('file.slice method:', typeof file.slice === 'function');
                console.log('file.stream method:', typeof file.stream === 'function');
                const sliced = file.slice(0, 4);
                console.log('file.slice result type:', sliced instanceof File);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("file.text method: true"),
            "Expected file.text method. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("file.slice method: true"),
            "Expected file.slice method. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("file.slice result type: true"),
            "Expected slice to return File. Got: {}",
            stdout
        );
    }

    /// Test 15: Blob.stream() with ReadableStream
    #[test]
    fn test_blob_stream_readable() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const blob = new Blob(['Hello, Stream!'], { type: 'text/plain' });
                const stream = blob.stream();
                // Verify it's a proper ReadableStream with getReader
                const reader = stream.getReader();
                console.log('reader exists:', typeof reader === 'object');
                console.log('reader.read exists:', typeof reader.read === 'function');
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("reader exists: true"),
            "Expected reader to exist. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("reader.read exists: true"),
            "Expected read method. Got: {}",
            stdout
        );
    }
}

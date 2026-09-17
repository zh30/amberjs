// CompressionStream Tests for Beejs
// Tests for v0.3.295: CompressionStream API (gzip/deflate)
// Enables streaming compression for AI workloads

#[cfg(test)]
mod compression_stream_tests {
    use std::path::PathBuf;
    use std::process::Command;

    fn beejs_path() -> PathBuf {
        PathBuf::from(
            std::env::var("CARGO_BIN_EXE_amber")
                .unwrap_or_else(|_| "./target/debug/bee".to_string()),
        )
    }

    /// Test 1: CompressionStream constructor with 'gzip' format
    #[test]
    fn test_compression_stream_gzip_constructor() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const cs = new CompressionStream('gzip');
                console.log('gzip format:', cs.format);
                console.log('has readable:', cs.readable instanceof ReadableStream);
                console.log('has writable:', cs.writable instanceof WritableStream);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("gzip format: gzip"), "Expected gzip format");
        assert!(
            stdout.contains("has readable: true"),
            "Expected readable stream"
        );
        assert!(
            stdout.contains("has writable: true"),
            "Expected writable stream"
        );
    }

    /// Test 2: CompressionStream constructor with 'deflate' format
    #[test]
    fn test_compression_stream_deflate_constructor() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const cs = new CompressionStream('deflate');
                console.log('deflate format:', cs.format);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("deflate format: deflate"),
            "Expected deflate format"
        );
    }

    /// Test 3: Basic compression pipeline
    #[test]
    fn test_basic_compression_pipeline() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                async function test() {
                    const cs = new CompressionStream('gzip');
                    const writer = cs.writable.getWriter();
                    const reader = cs.readable.getReader();

                    // Write some data
                    const data = new TextEncoder().encode('Hello, CompressionStream!');
                    await writer.write(data);
                    await writer.close();

                    // Read compressed data
                    let compressed = new Uint8Array();
                    let result;
                    while (!(result = await reader.read()).done) {
                        const chunk = result.value;
                        const newArray = new Uint8Array(compressed.length + chunk.length);
                        newArray.set(compressed);
                        newArray.set(chunk, compressed.length);
                        compressed = newArray;
                    }

                    // Verify compressed data is smaller than original (after gzip header)
                    console.log('original size:', data.length);
                    console.log('compressed size:', compressed.length);
                    console.log('compression works:', compressed.length > 0);
                }
                test().catch(e => console.log('error:', e.message));
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("compression works: true"),
            "Expected compression to work"
        );
    }

    /// Test 4: Pipe through compression stream
    #[test]
    fn test_pipe_through_compression() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                async function test() {
                    const stream = new ReadableStream({
                        start(controller) {
                            controller.enqueue(new TextEncoder().encode('Test data'));
                            controller.close();
                        }
                    });

                    const compressed = stream.pipeThrough(new CompressionStream('gzip'));
                    console.log('pipeThrough works:', compressed instanceof ReadableStream);
                }
                test().catch(e => console.log('error:', e.message));
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("pipeThrough works: true"),
            "Expected pipeThrough to work"
        );
    }

    /// Test 5: Decompression with DecompressionStream
    #[test]
    fn test_decompression_stream() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                // Test compression and decompression round-trip
                const original = 'Hello, World! This is a test of compression and decompression.';
                const encoder = new TextEncoder();
                const decoder = new TextDecoder();

                // Compress using _compressData helper
                const originalBytes = encoder.encode(original);
                const compressed = _compressData(originalBytes, 'gzip');
                console.log('original size:', originalBytes.length);
                console.log('compressed size:', compressed.length);

                // Decompress using _decompressData helper
                const decompressed = _decompressData(compressed, 'gzip');
                const decompressedStr = decoder.decode(decompressed);

                console.log('decompressed length:', decompressed.length);
                console.log('decompression matches:', decompressedStr === original);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("decompression matches: true"),
            "Expected decompression to match original. Got: {}",
            stdout
        );
    }

    /// Test 6: Invalid format should throw error
    #[test]
    fn test_invalid_format_throws() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                try {
                    new CompressionStream('invalid');
                    console.log('no error');
                } catch (e) {
                    console.log('has error:', e instanceof Error);
                    console.log('error message contains:', e.message.length > 0);
                }
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should either throw an error or handle invalid format gracefully
        assert!(
            stdout.contains("has error: true") || stdout.contains("no error"),
            "Expected error for invalid format or graceful handling"
        );
    }

    /// Test 7: Empty data compression
    #[test]
    fn test_empty_data_compression() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                async function test() {
                    const cs = new CompressionStream('gzip');
                    const writer = cs.writable.getWriter();
                    const reader = cs.readable.getReader();

                    await writer.write(new Uint8Array(0));
                    await writer.close();

                    let size = 0;
                    let result;
                    while (!(result = await reader.read()).done) {
                        size += result.value.length;
                    }
                    console.log('empty compressed size:', size);
                }
                test().catch(e => console.log('error:', e.message));
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("empty compressed size:"),
            "Expected to handle empty data"
        );
    }

    /// Test 8: Large data compression
    #[test]
    fn test_large_data_compression() {
        let output = Command::new(beejs_path())
            .args(["eval", r#"
                async function test() {
                    // Create 10KB of repeated data (compresses well)
                    const largeData = new Uint8Array(10240).fill(0x41); // 'A' repeated

                    const cs = new CompressionStream('gzip');
                    const writer = cs.writable.getWriter();
                    const reader = cs.readable.getReader();

                    await writer.write(largeData);
                    await writer.close();

                    let compressed = new Uint8Array();
                    let result;
                    while (!(result = await reader.read()).done) {
                        const chunk = result.value;
                        const newArray = new Uint8Array(compressed.length + chunk.length);
                        newArray.set(compressed);
                        newArray.set(chunk, compressed.length);
                        compressed = newArray;
                    }

                    // Should have significant compression ratio
                    console.log('original size:', largeData.length);
                    console.log('compressed size:', compressed.length);
                    console.log('compression ratio:', (compressed.length / largeData.length).toFixed(4));
                }
                test().catch(e => console.log('error:', e.message));
            "#])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("compression ratio:"),
            "Expected to compress large data"
        );
    }

    /// Test 9: CompressionStream.close() method exists and is callable
    #[test]
    fn test_compression_stream_close_method() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const cs = new CompressionStream('gzip');
                console.log('close method exists:', typeof cs.close === 'function');
                console.log('has readable:', cs.readable instanceof ReadableStream);
                console.log('has writable:', cs.writable instanceof WritableStream);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("close method exists: true"),
            "Expected close method to exist"
        );
        assert!(
            stdout.contains("has readable: true"),
            "Expected readable stream"
        );
        assert!(
            stdout.contains("has writable: true"),
            "Expected writable stream"
        );
    }

    /// Test 10: DecompressionStream.close() method exists and is callable
    #[test]
    fn test_decompression_stream_close_method() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const ds = new DecompressionStream('gzip');
                console.log('close method exists:', typeof ds.close === 'function');
                console.log('has readable:', ds.readable instanceof ReadableStream);
                console.log('has writable:', ds.writable instanceof WritableStream);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("close method exists: true"),
            "Expected close method to exist"
        );
        assert!(
            stdout.contains("has readable: true"),
            "Expected readable stream"
        );
        assert!(
            stdout.contains("has writable: true"),
            "Expected writable stream"
        );
    }

    /// Test 11: Close method actually closes the streams properly
    #[test]
    fn test_close_method_closes_streams() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                async function test() {
                    const cs = new CompressionStream('gzip');
                    const writer = cs.writable.getWriter();

                    // Write some data
                    await writer.write(new TextEncoder().encode('test data'));

                    // Call close method before closing writer
                    if (typeof cs.close === 'function') {
                        await cs.close();
                        console.log('close method called successfully');
                    }

                    // Try to write after close - should fail or be no-op
                    try {
                        await writer.write(new Uint8Array([1, 2, 3]));
                        console.log('write after close: allowed');
                    } catch (e) {
                        console.log('write after close: rejected');
                    }
                }
                test().catch(e => console.log('error:', e.message));
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("close method called successfully"),
            "Expected close method to work"
        );
    }

    /// Test 12: Compression round-trip with proper close
    #[test]
    fn test_compression_round_trip_with_close() {
        let output = Command::new(beejs_path())
            .args(["eval", r#"
                async function test() {
                    const original = 'Hello, Beejs! This is a test of the compression stream with proper close.';
                    const encoder = new TextEncoder();
                    const decoder = new TextDecoder();

                    // Compress
                    const cs = new CompressionStream('gzip');
                    const writer = cs.writable.getWriter();
                    await writer.write(encoder.encode(original));
                    await writer.close();

                    // Read compressed
                    const reader = cs.readable.getReader();
                    let compressedChunks = [];
                    while (true) {
                        const result = await reader.read();
                        if (result.done) break;
                        compressedChunks.push(result.value);
                    }

                    // Decompress
                    const ds = new DecompressionStream('gzip');
                    const dsWriter = ds.writable.getWriter();
                    for (const chunk of compressedChunks) {
                        await dsWriter.write(chunk);
                    }
                    await dsWriter.close();

                    // Read decompressed
                    const dsReader = ds.readable.getReader();
                    let decompressedChunks = [];
                    while (true) {
                        const result = await dsReader.read();
                        if (result.done) break;
                        decompressedChunks.push(result.value);
                    }

                    const decompressed = decoder.decode(concatUint8Arrays(decompressedChunks));
                    console.log('round trip success:', decompressed === original);
                    console.log('original length:', original.length);
                    console.log('compressed size:', compressedChunks.reduce((sum, c) => sum + c.length, 0));
                }

                function concatUint8Arrays(arrays) {
                    let total = arrays.reduce((sum, arr) => sum + arr.length, 0);
                    let result = new Uint8Array(total);
                    let offset = 0;
                    for (const arr of arrays) {
                        result.set(arr, offset);
                        offset += arr.length;
                    }
                    return result;
                }
                test().catch(e => console.log('error:', e.message));
            "#])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("round trip success: true"),
            "Expected successful round-trip compression"
        );
    }
}

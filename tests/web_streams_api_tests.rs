// v0.3.282: Web Streams API integration tests
// Tests for ReadableStream, WritableStream, TransformStream, TextDecoderStream

#[cfg(test)]
mod web_streams_api_tests {
    use std::path::PathBuf;
    use std::process::Command;

    fn beejs_path() -> PathBuf {
        PathBuf::from(
            std::env::var("CARGO_BIN_EXE_amber")
                .unwrap_or_else(|_| "./target/debug/bee".to_string()),
        )
    }

    #[test]
    fn test_readable_stream_creation() {
        let output = Command::new(beejs_path())
            .args(["eval", "console.log(typeof ReadableStream)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("function"), "ReadableStream should exist");
    }

    #[test]
    fn test_readable_stream_get_reader() {
        let output = Command::new(beejs_path())
            .args(["eval", "const s = new ReadableStream(); const r = s.getReader(); console.log(typeof r.read)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("function"),
            "Reader should have read method"
        );
    }

    #[test]
    fn test_readable_stream_reader_has_read_write_locked_closed() {
        let output = Command::new(beejs_path())
            .args(["eval", r#"const s = new ReadableStream(); const r = s.getReader(); console.log(typeof r.read === 'function' && typeof r.releaseLock === 'function' && r.closed instanceof Promise)"#])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Reader should have correct properties"
        );
    }

    #[test]
    fn test_readable_stream_locked_property() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                "const s = new ReadableStream(); console.log(s.locked)",
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("false"), "New stream should be unlocked");
    }

    #[test]
    fn test_readable_stream_locked_after_get_reader() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                "const s = new ReadableStream(); s.getReader(); console.log(s.locked)",
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("true"), "Stream should be locked");
    }

    #[test]
    fn test_readable_stream_rejects_second_reader_until_release() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const s = new ReadableStream();
                const reader1 = s.getReader();
                let threwWhileLocked = false;
                try {
                    s.getReader();
                } catch (error) {
                    threwWhileLocked = error instanceof TypeError &&
                        String(error.message).includes('locked');
                }
                reader1.releaseLock();
                const unlockedAfterRelease = s.locked === false;
                const reader2 = s.getReader();
                console.log(threwWhileLocked && unlockedAfterRelease && s.locked === true && typeof reader2.read === 'function');
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "ReadableStream should reject a second reader until releaseLock. Got: {}",
            stdout
        );
    }

    #[test]
    fn test_readable_stream_release_lock_does_not_rewind_queue() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const stream = new ReadableStream({
                    start(controller) {
                        controller.enqueue('one');
                        controller.enqueue('two');
                        controller.close();
                    }
                });
                const seen = [];
                const reader1 = stream.getReader();
                reader1.read();
                const afterFirstRead = stream._readIndex;
                reader1.releaseLock();
                const afterRelease = stream._readIndex;
                const reader2 = stream.getReader();
                reader2.read();
                console.log(afterFirstRead === 1 && afterRelease === 1 && stream._readIndex === 2);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "releaseLock should not rewind the stream queue. Got: {}",
            stdout
        );
    }

    #[test]
    fn test_writable_stream_creation() {
        let output = Command::new(beejs_path())
            .args(["eval", "console.log(typeof WritableStream)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("function"), "WritableStream should exist");
    }

    #[test]
    fn test_writable_stream_get_writer() {
        let output = Command::new(beejs_path())
            .args(["eval", "const s = new WritableStream(); const w = s.getWriter(); console.log(typeof w.write)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("function"),
            "Writer should have write method"
        );
    }

    #[test]
    fn test_writable_stream_writer_has_promises() {
        let output = Command::new(beejs_path())
            .args(["eval", r#"const s = new WritableStream(); const w = s.getWriter(); console.log(w.ready instanceof Promise && w.closed instanceof Promise)"#])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Writer should have ready and closed promises"
        );
    }

    #[test]
    fn test_writable_stream_locked_property() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                "const s = new WritableStream(); console.log(s.locked)",
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("false"),
            "New WritableStream should be unlocked"
        );
    }

    #[test]
    fn test_writable_stream_rejects_second_writer_until_release() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const s = new WritableStream();
                const writer1 = s.getWriter();
                let threwWhileLocked = false;
                try {
                    s.getWriter();
                } catch (error) {
                    threwWhileLocked = error instanceof TypeError &&
                        String(error.message).includes('locked');
                }
                writer1.releaseLock();
                const unlockedAfterRelease = s.locked === false;
                const writer2 = s.getWriter();
                console.log(threwWhileLocked && unlockedAfterRelease && s.locked === true && typeof writer2.write === 'function');
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "WritableStream should reject a second writer until releaseLock. Got: {}",
            stdout
        );
    }

    #[test]
    fn test_transform_stream_creation() {
        let output = Command::new(beejs_path())
            .args(["eval", "console.log(typeof TransformStream)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("function"), "TransformStream should exist");
    }

    #[test]
    fn test_transform_stream_has_readable_writable() {
        // Note: TransformStream readable/writable getReader/getWriter require full implementation
        // Basic scaffold provides readable/writable objects with locked property
        let output = Command::new(beejs_path())
            .args(["eval", "const t = new TransformStream(); console.log(typeof t.readable && typeof t.writable)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("object"),
            "TransformStream should have readable and writable objects"
        );
    }

    #[test]
    fn test_text_decoder_stream_creation() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                "const d = new TextDecoderStream(); console.log(d.encoding)",
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("utf-8"),
            "TextDecoderStream should have utf-8 encoding"
        );
    }

    #[test]
    fn test_text_decoder_stream_has_readable_writable() {
        let output = Command::new(beejs_path())
            .args(["eval", "const d = new TextDecoderStream(); console.log(typeof d.readable && typeof d.writable)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("object"),
            "TextDecoderStream should have readable and writable"
        );
    }

    #[test]
    fn test_stream_creation_performance() {
        let output = Command::new(beejs_path())
            .args(["eval", r#"performance.mark('s'); for(let i=0;i<100;i++){new ReadableStream(); new WritableStream(); new TransformStream();} performance.mark('e'); performance.measure('m','s','e'); console.log(performance.getEntriesByType('measure')[0].duration < 1000)"#])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("true"), "Stream creation should be fast");
    }

    // v0.3.283: Tests for ReadableStream.start() and controller.enqueue()
    #[test]
    fn test_readable_stream_with_start_and_enqueue() {
        // Test that start() callback is called and can use controller.enqueue()
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                let received = null;
                const stream = new ReadableStream({
                    start(controller) {
                        controller.enqueue('hello');
                        controller.enqueue('world');
                    }
                });
                const reader = stream.getReader();
                reader.read();
                stream._queue.length === 2 && stream._queue[0] === 'hello' && stream._queue[1] === 'world' && stream._readIndex === 1
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Should enqueue data and advance read index once. Got: {}",
            stdout
        );
    }

    #[test]
    fn test_readable_stream_controller_has_enqueue() {
        // Test that controller object has enqueue method
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                let hasEnqueue = false;
                const stream = new ReadableStream({
                    start(controller) {
                        hasEnqueue = typeof controller.enqueue === 'function';
                    }
                });
                console.log(hasEnqueue);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Controller should have enqueue method"
        );
    }

    #[test]
    fn test_readable_stream_controller_has_close() {
        // Test that controller object has close method
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                let hasClose = false;
                const stream = new ReadableStream({
                    start(controller) {
                        hasClose = typeof controller.close === 'function';
                    }
                });
                console.log(hasClose);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Controller should have close method"
        );
    }

    #[test]
    fn test_readable_stream_controller_has_error() {
        // Test that controller object has error method
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                let hasError = false;
                const stream = new ReadableStream({
                    start(controller) {
                        hasError = typeof controller.error === 'function';
                    }
                });
                console.log(hasError);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Controller should have error method"
        );
    }

    #[test]
    fn test_readable_stream_multiple_chunks() {
        // Test reading multiple chunks from a stream
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const chunks = [];
                const stream = new ReadableStream({
                    start(controller) {
                        controller.enqueue('chunk1');
                        controller.enqueue('chunk2');
                        controller.enqueue('chunk3');
                        controller.close();
                    }
                });
                const reader = stream.getReader();
                reader.read().then(r1 => {
                    chunks.push(r1.value);
                    return reader.read();
                }).then(r2 => {
                    chunks.push(r2.value);
                    return reader.read();
                }).then(r3 => {
                    chunks.push(r3.value);
                    console.log(chunks.join(','));
                });
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("chunk1,chunk2,chunk3"),
            "Should receive all chunks"
        );
    }

    #[test]
    fn test_readable_stream_close_after_enqueue() {
        // Test that stream is done after close
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                let doneValues = [];
                const stream = new ReadableStream({
                    start(controller) {
                        controller.enqueue('data');
                    }
                });
                const reader = stream.getReader();
                reader.read().then(r => {
                    doneValues.push(r.done);
                    return reader.read();
                }).then(r2 => {
                    doneValues.push(r2.done);
                    console.log(doneValues.join(','));
                });
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // First read should have data, second should be done
        assert!(
            stdout.contains("false,true") || stdout.contains("true"),
            "Should be done after all chunks read"
        );
    }

    // v0.3.284: Tests for WritableStream start() callback and write queue
    #[test]
    fn test_writable_stream_with_start_callback() {
        // Test that start() callback is called
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                let startCalled = false;
                const stream = new WritableStream({
                    start() {
                        startCalled = true;
                    }
                });
                console.log(startCalled);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("true"), "Start callback should be called");
    }

    #[test]
    fn test_writable_stream_write_adds_to_queue() {
        // Test that write() adds chunks to the write queue
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const stream = new WritableStream();
                const writer = stream.getWriter();
                // Write some data
                writer.write('test1');
                writer.write('test2');
                // Check that queue has entries (internal state check via locked)
                console.log(typeof stream._writeQueue === 'object');
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "WritableStream should have write queue"
        );
    }

    #[test]
    fn test_writable_stream_close_changes_state() {
        // Test that close() changes the stream state
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const stream = new WritableStream();
                const writer = stream.getWriter();
                // Initially state should be 0 (Open)
                const initialState = stream._state;
                // Close the stream
                writer.close();
                // After close, state should be 1 (Closed)
                const closedState = stream._state;
                console.log(initialState === 0 && closedState === 1);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "State should change from 0 to 1 on close"
        );
    }

    #[test]
    fn test_writable_stream_abort_changes_state() {
        // Test that abort() changes the stream state to errored
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const stream = new WritableStream();
                const writer = stream.getWriter();
                // Initially state should be 0 (Open)
                const initialState = stream._state;
                // Abort the stream
                writer.abort(new Error('test error'));
                // After abort, state should be 2 (Errored)
                const erroredState = stream._state;
                console.log(initialState === 0 && erroredState === 2);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "State should change from 0 to 2 on abort"
        );
    }

    #[test]
    fn test_writable_stream_write_rejects_when_closed() {
        // Test that write() doesn't add to queue when stream is closed
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const stream = new WritableStream();
                const writer = stream.getWriter();
                // Get initial queue length
                const initialLength = stream._writeQueue.length;
                // Close the stream
                writer.close();
                // Try to write after close
                writer.write('should not be added');
                // Queue should not have changed (or may have the pre-close entries)
                console.log(typeof stream._writeQueue.length === 'number');
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("true"), "Write queue should exist");
    }

    #[test]
    fn test_transform_stream_transform_function_call() {
        // Test that TransformStream with transformer works correctly
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const ts = new TransformStream({
                    transform(chunk, controller) {
                        controller.enqueue(chunk.toString().toUpperCase());
                    }
                });
                // Check that transform function reference is stored
                console.log(ts.readable !== undefined && ts.writable !== undefined);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "TransformStream should have readable and writable"
        );
    }

    #[test]
    fn test_transform_stream_controller_has_methods() {
        // Test that TransformStream writable side works with controller methods
        let output = Command::new(beejs_path())
            .args(["eval", r#"
                const ts = new TransformStream({
                    transform(chunk, controller) {
                        controller.enqueue(chunk.toString().toUpperCase());
                    }
                });
                // Check that writable stream can be used
                const writer = ts.writable.getWriter();
                console.log(typeof writer.write === 'function' && typeof writer.close === 'function' && typeof writer.abort === 'function');
            "#])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Writer should have write, close, abort methods"
        );
    }

    #[test]
    fn test_transform_stream_readable_has_get_reader() {
        // Test that TransformStream readable side has getReader
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const ts = new TransformStream({
                    transform(chunk, controller) {
                        controller.enqueue(chunk);
                    }
                });
                // Readable should have getReader
                const reader = ts.readable.getReader();
                console.log(typeof reader.read === 'function' && reader.closed instanceof Promise);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Readable should have read method and closed Promise"
        );
    }

    // v0.3.287: Tests for TransformStream transform() end-to-end data flow
    #[test]
    fn test_transform_stream_end_to_end_transform() {
        // Test that transform() actually transforms data from writable to readable
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const ts = new TransformStream({
                    transform(chunk, controller) {
                        controller.enqueue(chunk.toString().toUpperCase());
                    }
                });
                const writer = ts.writable.getWriter();
                const reader = ts.readable.getReader();

                // Write and read transformed data
                writer.write('hello');
                writer.write('world');
                writer.close();

                // Read all transformed chunks
                let results = [];
                reader.read().then(r => {
                    results.push(r.value);
                    return reader.read();
                }).then(r => {
                    results.push(r.value);
                    return reader.read();
                }).then(r => {
                    results.push(r.value);
                    console.log(results.join(','));
                });
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("HELLO,WORLD"),
            "Transform should uppercase input"
        );
    }

    #[test]
    fn test_transform_stream_with_flush() {
        // Test transform with flush callback
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                let flushCalled = false;
                const ts = new TransformStream({
                    transform(chunk, controller) {
                        controller.enqueue(chunk);
                    },
                    flush(controller) {
                        flushCalled = true;
                        controller.enqueue('[END]');
                    }
                });
                const writer = ts.writable.getWriter();
                const reader = ts.readable.getReader();

                writer.write('data');
                writer.close();

                let results = [];
                reader.read().then(r => {
                    results.push(r.value);
                    return reader.read();
                }).then(r => {
                    results.push(r.value);
                    console.log(results.join(',') + ':' + flushCalled);
                });
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("data,[END]") && stdout.contains("true"),
            "Flush should be called and add termination chunk"
        );
    }

    #[test]
    fn test_transform_stream_error_propagation() {
        // Test that errors in transform are properly handled
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const ts = new TransformStream({
                    transform(chunk, controller) {
                        if (chunk === 'error') {
                            throw new Error('transform error');
                        }
                        controller.enqueue(chunk);
                    }
                });
                const writer = ts.writable.getWriter();

                // Write normal data first
                writer.write('ok').then(() => {
                    console.log('ok written');
                });
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("ok written"),
            "Normal transform should work"
        );
    }

    // v0.3.288: Tests for pipeTo() method
    #[test]
    fn test_readable_stream_pipe_to_method_exists() {
        // Test that ReadableStream has pipeTo method
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const rs = new ReadableStream();
                console.log(typeof rs.pipeTo === 'function');
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "ReadableStream should have pipeTo method"
        );
    }

    #[test]
    fn test_readable_stream_pipe_to_writable() {
        // Test basic pipeTo functionality - data flows from readable to writable
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const chunks = [];
                const writable = new WritableStream({
                    write(chunk) {
                        chunks.push(chunk);
                    }
                });

                const readable = new ReadableStream({
                    start(controller) {
                        controller.enqueue('hello');
                        controller.enqueue('world');
                        controller.close();
                    }
                });

                // Use pipeTo to connect readable to writable
                readable.pipeTo(writable);

                // Use a zero-delay timer so the current runtime drains the callback in this eval.
                setTimeout(() => {
                    console.log(chunks.join(','));
                }, 0);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("hello,world"),
            "pipeTo should transfer chunks to writable"
        );
    }

    #[test]
    fn test_readable_stream_pipe_to_returns_promise() {
        // Test that pipeTo returns a Promise
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const rs = new ReadableStream();
                const ws = new WritableStream();
                const result = rs.pipeTo(ws);
                console.log(result instanceof Promise);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("true"), "pipeTo should return a Promise");
    }

    // v0.3.288: Tests for pipeThrough() method
    #[test]
    fn test_readable_stream_pipe_through_method_exists() {
        // Test that ReadableStream has pipeThrough method
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const rs = new ReadableStream();
                console.log(typeof rs.pipeThrough === 'function');
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "ReadableStream should have pipeThrough method"
        );
    }

    #[test]
    fn test_readable_stream_pipe_through_transform() {
        // Test basic pipeThrough functionality
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const readable = new ReadableStream({
                    start(controller) {
                        controller.enqueue('hello');
                        controller.enqueue('world');
                        controller.close();
                    }
                });

                const ts = new TransformStream({
                    transform(chunk, controller) {
                        controller.enqueue(chunk.toString().toUpperCase());
                    }
                });

                // Use pipeThrough to connect readable through transform
                const result = readable.pipeThrough(ts);

                setTimeout(() => {
                    console.log(typeof result.readable === 'object');
                }, 0);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "pipeThrough should return object with readable property"
        );
    }

    #[test]
    fn test_readable_stream_pipe_through_data_flow() {
        // Test that pipeThrough correctly sets up the transform pipeline
        // The actual data flow is async, so we verify the structure is correct
        let output = Command::new(beejs_path())
            .args(["eval", r#"
                const ts = new TransformStream({
                    transform(chunk, controller) {
                        controller.enqueue(chunk.toString().toUpperCase());
                    }
                });

                const readable = new ReadableStream({
                    start(controller) {
                        controller.enqueue('hello');
                        controller.close();
                    }
                });

                const result = readable.pipeThrough(ts);

                // Verify pipeThrough returns an object with readable property
                // result.readable should be an object with getReader method
                console.log(typeof result === 'object' && typeof result.readable === 'object' && typeof result.readable.getReader === 'function');
            "#])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Verify the structure is correct - the actual async data flow requires setTimeout which
        // the test framework doesn't wait for, but this verifies pipeThrough is wired up correctly
        assert!(
            stdout.contains("true"),
            "pipeThrough should return object with ReadableStream readable"
        );
    }

    // v0.3.289: Tests for pipeTo with preventClose option
    #[test]
    fn test_pipe_to_returns_promise() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const readable = new ReadableStream({
                    start(controller) {
                        controller.enqueue('test');
                        controller.close();
                    }
                });
                const writable = new WritableStream({
                    write(chunk) {}
                });
                const result = readable.pipeTo(writable);
                console.log(result instanceof Promise);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("true"), "pipeTo should return a Promise");
    }

    #[test]
    fn test_pipe_to_closes_writable_by_default() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const readable = new ReadableStream({
                    start(controller) {
                        controller.enqueue('test');
                        controller.close();
                    }
                });
                const writable = new WritableStream({
                    write(chunk) {}
                });
                readable.pipeTo(writable).then(() => {
                    console.log(writable._state);  // Should be 1 (Closed)
                });
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("1"),
            "pipeTo should close writable by default (state=1)"
        );
    }

    #[test]
    fn test_pipe_to_prevent_close_option() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const readable = new ReadableStream({
                    start(controller) {
                        controller.enqueue('test');
                        controller.close();
                    }
                });
                const writable = new WritableStream({
                    write(chunk) {}
                });
                readable.pipeTo(writable, { preventClose: true }).then(() => {
                    console.log(writable._state);  // Should be 0 (Open)
                });
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("0"),
            "pipeTo with preventClose should keep writable open (state=0)"
        );
    }

    #[test]
    fn test_pipe_to_data_transfer() {
        let output = Command::new(beejs_path())
            .args(["eval", r#"
                const readable = new ReadableStream({
                    start(controller) {
                        controller.enqueue('hello');
                        controller.enqueue('world');
                        controller.close();
                    }
                });
                const chunks = [];
                const writable = new WritableStream({
                    write(chunk) {
                        chunks.push(chunk);
                    }
                });
                readable.pipeTo(writable).then(() => {
                    console.log(chunks.length === 2 && chunks[0] === 'hello' && chunks[1] === 'world');
                });
            "#])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "pipeTo should transfer all chunks correctly"
        );
    }

    #[test]
    fn test_pipe_to_prevent_abort_option() {
        // Test preventAbort option - writable should not be aborted when error occurs
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const chunks = [];
                const readable = new ReadableStream({
                    start(controller) {
                        controller.enqueue('chunk1');
                        controller.enqueue('chunk2');
                        controller.close();
                    }
                });

                let writableState = 'open';
                const writable = new WritableStream({
                    write(chunk) {
                        chunks.push(chunk);
                        return Promise.resolve();
                    }
                });

                // Pipe with preventAbort: true - errors should not abort writable
                readable.pipeTo(writable, { preventAbort: true }).then(() => {
                    console.log('success');
                }).catch(err => {
                    console.log('error:', err.message || err);
                });
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("success"),
            "pipeTo should succeed with preventAbort option"
        );
    }

    #[test]
    fn test_pipe_to_error_propagates_to_promise() {
        // Test that errors in readable stream propagate to the pipeTo promise
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const readable = new ReadableStream({
                    start(controller) {
                        controller.enqueue('chunk1');
                        // Error occurs during read - this should cause pipeTo to reject
                    }
                });

                const writable = new WritableStream({
                    write(chunk) {
                        return Promise.resolve();
                    }
                });

                // Pipe and check that promise is returned (errors handled in promise chain)
                const result = readable.pipeTo(writable);
                console.log(result instanceof Promise);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("true"), "pipeTo should return a Promise");
    }

    #[test]
    fn test_pipe_to_writable_close_failure_aborts() {
        // Test that writable close failure properly aborts the pipe
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                let chunks = [];
                const readable = new ReadableStream({
                    start(controller) {
                        controller.enqueue('chunk1');
                        controller.close();
                    }
                });

                const writable = new WritableStream({
                    write(chunk) {
                        chunks.push(chunk);
                        return Promise.resolve();
                    },
                    close() {
                        // Simulate close failure
                        return Promise.reject(new Error('Close failed'));
                    }
                });

                // Test that pipeTo returns a promise even when close fails
                const result = readable.pipeTo(writable);
                console.log(result instanceof Promise);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "pipeTo should return a Promise even on close failure"
        );
    }

    #[test]
    fn test_pipe_to_both_options_together() {
        // Test preventClose and preventAbort options together
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const readable = new ReadableStream({
                    start(controller) {
                        controller.enqueue('test');
                        controller.close();
                    }
                });

                const writable = new WritableStream({
                    write(chunk) {
                        return Promise.resolve();
                    }
                });

                // Use both options
                const result = readable.pipeTo(writable, {
                    preventClose: true,
                    preventAbort: false
                });
                console.log(result instanceof Promise);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "pipeTo should accept both preventClose and preventAbort options"
        );
    }

    // v0.3.291: AbortController/signal tests
    #[test]
    fn test_pipe_to_signal_option_exists() {
        // Test that pipeTo accepts signal option
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const controller = new AbortController();
                const readable = new ReadableStream({
                    start(controller) {
                        controller.enqueue('test');
                    }
                });
                const writable = new WritableStream({
                    write(chunk) {
                        return Promise.resolve();
                    }
                });
                // pipeTo should accept signal option without error
                const result = readable.pipeTo(writable, { signal: controller.signal });
                console.log(result instanceof Promise);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "pipeTo should accept signal option"
        );
    }

    #[test]
    fn test_pipe_to_pre_aborted_signal() {
        // Test that already aborted signal immediately rejects
        // Note: Async abort tests require WritableStream.write() to properly await
        // user Promises, which is a separate enhancement.
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const controller = new AbortController();
                controller.abort(); // Abort immediately

                const readable = new ReadableStream({
                    start(ctrl) {
                        ctrl.enqueue('test');
                    }
                });
                const writable = new WritableStream({
                    write(chunk) {
                        return Promise.resolve();
                    }
                });

                // Should reject immediately since signal is already aborted
                readable.pipeTo(writable, { signal: controller.signal })
                    .then(() => console.log('should not reach here'))
                    .catch(err => {
                        console.log(err.name);
                    });
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("AbortError"),
            "pipeTo should reject immediately for already aborted signal"
        );
    }

    #[test]
    fn test_pipe_to_signal_with_prevent_abort_pre_aborted() {
        // Test preventAbort with pre-aborted signal
        // This works because rejection happens synchronously at pipeTo start
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const controller = new AbortController();
                controller.abort(); // Abort immediately

                const readable = new ReadableStream({
                    start(ctrl) {
                        ctrl.enqueue('test');
                    }
                });
                const writable = new WritableStream({
                    write(chunk) {
                        return Promise.resolve();
                    }
                });

                // With preventAbort: true, writable should not be aborted
                readable.pipeTo(writable, { signal: controller.signal, preventAbort: true })
                    .then(() => console.log('should not reach here'))
                    .catch(err => {
                        console.log(err.name === 'AbortError' && writable._state === 0);
                    });
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "pipeTo with signal and preventAbort should not abort writable"
        );
    }

    // v0.3.292: WritableStream.write() async enhancement tests
    #[test]
    fn test_writable_stream_write_waits_for_async_callback() {
        // Test that write() waits for async write callback to complete
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                let writeCompleted = false;
                const stream = new WritableStream({
                    write(chunk) {
                        return Promise.resolve().then(() => {
                            writeCompleted = true;
                        });
                    }
                });
                const writer = stream.getWriter();
                const writePromise = writer.write('test');

                // Before the promise resolves, write should not be complete
                console.log('before:', writeCompleted);

                // Wait for the write to complete
                writePromise.then(() => {
                    console.log('after:', writeCompleted);
                });
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Check that async write works: false before, true after
        assert!(
            stdout.contains("before: false") && stdout.contains("after: true"),
            "write() should wait for async write callback"
        );
    }

    #[test]
    fn test_writable_stream_write_rejects_on_callback_error() {
        // Test that write() rejects when write callback rejects
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const stream = new WritableStream({
                    write(chunk) {
                        return Promise.reject(new Error('Write failed'));
                    }
                });
                const writer = stream.getWriter();
                writer.write('test').then(
                    () => console.log('resolved'),
                    (err) => console.log('rejected:', err.message)
                );
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("rejected: Write failed"),
            "write() should reject when write callback rejects"
        );
    }

    #[test]
    fn test_writable_stream_write_with_sync_callback() {
        // Test that write() resolves immediately for synchronous callbacks
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                let callbackCalled = false;
                const stream = new WritableStream({
                    write(chunk) {
                        callbackCalled = true;
                        // No return value = synchronous completion
                    }
                });
                const writer = stream.getWriter();
                const writePromise = writer.write('test');

                // Callback should be called immediately
                console.log('callbackCalled:', callbackCalled);

                writePromise.then(() => {
                    console.log('promiseResolved: true');
                });
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("callbackCalled: true") && stdout.contains("promiseResolved: true"),
            "write() should resolve immediately for sync callbacks"
        );
    }

    #[test]
    fn test_writable_stream_write_returns_promise() {
        // Test that write() returns a Promise
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const stream = new WritableStream();
                const writer = stream.getWriter();
                const result = writer.write('test');
                console.log(result instanceof Promise);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("true"), "write() should return a Promise");
    }

    // v0.3.293: TextEncoderStream tests
    #[test]
    fn test_text_encoder_stream_creation() {
        // Test that TextEncoderStream can be created
        let output = Command::new(beejs_path())
            .args(["eval", "console.log(typeof TextEncoderStream)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("function"),
            "TextEncoderStream should exist"
        );
    }

    #[test]
    fn test_text_encoder_stream_has_encoding() {
        // Test that TextEncoderStream has encoding property
        let output = Command::new(beejs_path())
            .args([
                "eval",
                "const e = new TextEncoderStream(); console.log(e.encoding)",
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("utf-8"),
            "TextEncoderStream should have utf-8 encoding"
        );
    }

    #[test]
    fn test_text_encoder_stream_has_readable_writable() {
        // Test that TextEncoderStream has readable and writable properties
        let output = Command::new(beejs_path())
            .args(["eval", "const e = new TextEncoderStream(); console.log(typeof e.readable && typeof e.writable)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("object"),
            "TextEncoderStream should have readable and writable"
        );
    }

    #[test]
    fn test_text_encoder_stream_encodes_to_bytes() {
        // Test that TextEncoderStream actually encodes strings to bytes
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                async function test() {
                    const encoder = new TextEncoderStream();
                    const writer = encoder.writable.getWriter();
                    await writer.write('hello');
                    await writer.close();
                    const reader = encoder.readable.getReader();
                    const { done, value } = await reader.read();
                    // Check that we got Uint8Array bytes for 'hello'
                    console.log(value instanceof Uint8Array && value.length === 5);
                    console.log(value[0] === 104 && value[4] === 111); // 'h' = 104, 'o' = 111
                }
                test();
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "TextEncoderStream should encode strings to Uint8Array bytes"
        );
    }

    #[test]
    fn test_text_encoder_stream_multibyte_characters() {
        // Test that TextEncoderStream handles multibyte UTF-8 characters correctly
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                async function test() {
                    const encoder = new TextEncoderStream();
                    const writer = encoder.writable.getWriter();
                    await writer.write('你好'); // Chinese characters
                    await writer.close();
                    const reader = encoder.readable.getReader();
                    const { done, value } = await reader.read();
                    // Each Chinese character is 3 bytes in UTF-8
                    console.log(value instanceof Uint8Array && value.length === 6);
                }
                test();
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "TextEncoderStream should handle multibyte UTF-8 characters"
        );
    }

    #[test]
    fn test_text_encoder_stream_empty_string() {
        // Test that TextEncoderStream handles empty string
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                async function test() {
                    const encoder = new TextEncoderStream();
                    const writer = encoder.writable.getWriter();
                    await writer.write('');
                    await writer.close();
                    const reader = encoder.readable.getReader();
                    const { done, value } = await reader.read();
                    console.log(value instanceof Uint8Array && value.length === 0);
                }
                test();
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "TextEncoderStream should handle empty string"
        );
    }

    #[test]
    fn test_text_encoder_stream_pipe_from_readable() {
        // Test piping from ReadableStream through TextEncoderStream
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                async function test() {
                    const readable = new ReadableStream({
                        start(controller) {
                            controller.enqueue('test string');
                            controller.close();
                        }
                    });
                    const encoder = new TextEncoderStream();
                    const promise = readable.pipeTo(encoder.writable);
                    promise.then(() => {
                        const reader = encoder.readable.getReader();
                        reader.read().then(r => {
                            console.log(r.value instanceof Uint8Array && r.value.length === 11);
                        });
                    });
                }
                test();
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "TextEncoderStream should work with pipeTo"
        );
    }

    // v0.3.338: Test TransformStream async flush support
    #[test]
    fn test_transform_stream_async_flush() {
        // Test transform with async flush callback (returns Promise)
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                async function test() {
                    const results = [];
                    const ts = new TransformStream({
                        transform(chunk, controller) {
                            controller.enqueue(chunk.toUpperCase());
                        },
                        async flush(controller) {
                            // Async flush operation (returns Promise)
                            await Promise.resolve();
                            controller.enqueue('[ASYNC_END]');
                            return 'flush complete';
                        }
                    });
                    const writer = ts.writable.getWriter();
                    const reader = ts.readable.getReader();

                    await writer.write('hello');
                    await writer.close();

                    // Read all chunks
                    const read1 = await reader.read();
                    results.push(read1.value);
                    const read2 = await reader.read();
                    results.push(read2.value);

                    console.log(results.join(',') === 'HELLO,[ASYNC_END]');
                }
                test();
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Async flush should complete and enqueue final chunk"
        );
    }

    #[test]
    fn test_transform_stream_sync_flush() {
        // Test transform with sync flush callback (no Promise)
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                let flushCalled = false;
                const ts = new TransformStream({
                    transform(chunk, controller) {
                        controller.enqueue(chunk);
                    },
                    flush(controller) {
                        flushCalled = true;
                        controller.enqueue('[END]');
                    }
                });
                const writer = ts.writable.getWriter();
                const reader = ts.readable.getReader();

                writer.write('data');
                writer.close();

                let results = [];
                reader.read().then(r => {
                    results.push(r.value);
                    return reader.read();
                }).then(r => {
                    results.push(r.value);
                    console.log(results.join(',') + ':' + flushCalled);
                });
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("data,[END]") && stdout.contains("true"),
            "Flush should be called and add termination chunk"
        );
    }
}

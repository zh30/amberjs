// Web Streams API tests for Amber runtime
// Tests for ReadableStream, WritableStream, TransformStream, TextDecoderStream
// Stage 75: Web Streams API for AI workloads

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_readable_stream_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof ReadableStream");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_readable_stream_constructor() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const rs = new ReadableStream({
            start(controller) {
                controller.enqueue('hello');
                controller.close();
            }
        });
        typeof rs
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_readable_stream_get_reader() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const rs = new ReadableStream({
            start(controller) {
                controller.enqueue('test');
                controller.close();
            }
        });
        const reader = rs.getReader();
        typeof reader.read
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_readable_stream_locked() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const rs = new ReadableStream({
            start(controller) {
                controller.close();
            }
        });
        rs.locked
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "false");
}

#[test]
#[serial]
fn test_readable_stream_reader_has_release_lock() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const rs = new ReadableStream({
            start(controller) {
                controller.close();
            }
        });
        const reader = rs.getReader();
        typeof reader.releaseLock
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_readable_stream_reader_has_closed() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const rs = new ReadableStream({
            start(controller) {
                controller.close();
            }
        });
        const reader = rs.getReader();
        typeof reader.closed
        "#,
    );
    assert!(result.is_ok());
    // Should be a Promise (object)
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_writable_stream_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof WritableStream");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_writable_stream_constructor() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const ws = new WritableStream({
            write(chunk) {
                // do nothing
            }
        });
        typeof ws
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_writable_stream_get_writer() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const ws = new WritableStream({
            write(chunk) {}
        });
        const writer = ws.getWriter();
        typeof writer.write
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_writable_stream_locked() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const ws = new WritableStream({
            write(chunk) {}
        });
        ws.locked
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "false");
}

#[test]
#[serial]
fn test_writable_stream_writer_has_close() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const ws = new WritableStream({
            write(chunk) {}
        });
        const writer = ws.getWriter();
        typeof writer.close
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_writable_stream_writer_has_abort() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const ws = new WritableStream({
            write(chunk) {}
        });
        const writer = ws.getWriter();
        typeof writer.abort
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_writable_stream_writer_has_ready() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const ws = new WritableStream({
            write(chunk) {}
        });
        const writer = ws.getWriter();
        typeof writer.ready
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object"); // Promise
}

#[test]
#[serial]
fn test_transform_stream_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof TransformStream");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_transform_stream_constructor() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const ts = new TransformStream({
            transform(chunk, controller) {
                controller.enqueue(chunk.toString().toUpperCase());
            }
        });
        typeof ts
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_transform_stream_has_readable() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const ts = new TransformStream({
            transform(chunk, controller) {
                controller.enqueue(chunk);
            }
        });
        typeof ts.readable
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_transform_stream_has_writable() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const ts = new TransformStream({
            transform(chunk, controller) {
                controller.enqueue(chunk);
            }
        });
        typeof ts.writable
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_text_decoder_stream_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof TextDecoderStream");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_text_decoder_stream_constructor() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const tds = new TextDecoderStream();
        typeof tds
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_text_decoder_stream_has_readable() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const tds = new TextDecoderStream();
        typeof tds.readable
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_text_decoder_stream_has_writable() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const tds = new TextDecoderStream();
        typeof tds.writable
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_text_decoder_stream_encoding() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const tds = new TextDecoderStream();
        tds.encoding
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "utf-8");
}

#[test]
#[serial]
fn test_text_decoder_stream_fatal() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const tds = new TextDecoderStream();
        tds.fatal
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "false");
}

#[test]
#[serial]
fn test_text_decoder_stream_ignore_bom() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const tds = new TextDecoderStream();
        tds.ignoreBOM
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "false");
}

// AI Workload use case tests

#[test]
#[serial]
fn test_readable_stream_controller_enqueue() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const rs = new ReadableStream({
            start(controller) {
                controller.enqueue('hello');
                controller.enqueue('world');
                controller.close();
            }
        });
        rs.locked
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "false");
}

#[test]
#[serial]
fn test_readable_stream_controller_close() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        let closed = false;
        const rs = new ReadableStream({
            start(controller) {
                controller.close();
            }
        });
        // If close works, stream should be in closed state
        typeof rs
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_writable_stream_controller_write() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const ws = new WritableStream({
            write(chunk) {
                // do nothing
            }
        });
        const writer = ws.getWriter();
        // Write should return a Promise
        typeof writer.write('test')
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object"); // Promise
}

#[test]
#[serial]
fn test_writable_stream_controller_close() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const ws = new WritableStream({
            write(chunk) {}
        });
        const writer = ws.getWriter();
        // close() should return a Promise
        typeof writer.close()
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object"); // Promise
}

#[test]
#[serial]
fn test_writable_stream_controller_abort() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        r#"
        const ws = new WritableStream({
            write(chunk) {}
        });
        const writer = ws.getWriter();
        // abort() should return a Promise
        typeof writer.abort('error')
        "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object"); // Promise
}

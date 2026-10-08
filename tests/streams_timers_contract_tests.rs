// Pins the Stable streams, compression, timers, and performance contract
// in docs/STREAMS_TIMERS_CONTRACT.md.

use amberjs::nodejs_core::performance::clear_performance_entries;
use amberjs::nodejs_core::timers::{clear_all_async_timers, clear_all_timers};
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn runtime() -> MinimalRuntime {
    let mut runtime = MinimalRuntime::new().expect("runtime");
    runtime.set_timer_drain_limit_ms(500);
    runtime
}

fn run(code: &str) -> String {
    runtime()
        .execute_code(code)
        .unwrap_or_else(|err| panic!("execution failed: {err}"))
        .trim()
        .to_string()
}

fn reset_timers() {
    clear_all_timers();
    clear_all_async_timers();
}

#[test]
#[serial]
fn readable_stream_reads_enqueued_chunks_and_locks() {
    let output = run(r#"
        const stream = new ReadableStream({
            start(controller) {
                controller.enqueue('a');
                controller.enqueue('b');
                controller.close();
            }
        });
        const reader = stream.getReader();
        let locked = '';
        try { stream.getReader(); } catch (e) { locked = e.message; }
        Promise.all([reader.read(), reader.read(), reader.read()]).then((parts) => {
            return parts.map((part) => part.done ? 'done' : part.value).join(',') + '|' + locked;
        });
        "#);
    assert_eq!(output, "a,b,done|ReadableStream is locked");
}

#[test]
#[serial]
fn readable_stream_from_and_pipe_to() {
    let output = run(r#"
        const collected = [];
        const writable = new WritableStream({
            write(chunk) { collected.push(chunk); }
        });
        ReadableStream.from(['p', 'q']).pipeTo(writable).then(() => collected.join(''));
        "#);
    assert_eq!(output, "pq");
}

#[test]
#[serial]
fn writable_stream_rejects_a_second_writer_and_ignores_writes_after_close() {
    let output = run(r#"
        const seen = [];
        const stream = new WritableStream({
            write(chunk) { seen.push(chunk); }
        });
        const writer = stream.getWriter();
        let locked = '';
        try { stream.getWriter(); } catch (e) { locked = e.message; }
        writer.write('one').then(() => writer.close()).then(() => writer.write('two')).then(() => {
            return seen.join(',') + '|' + locked;
        });
        "#);
    assert_eq!(output, "one|WritableStream is locked");
}

#[test]
#[serial]
fn transform_stream_maps_chunks_and_flushes() {
    let output = run(r#"
        const ts = new TransformStream({
            transform(chunk, controller) {
                controller.enqueue(String(chunk).toUpperCase());
            },
            flush(controller) {
                controller.enqueue('END');
            }
        });
        const writer = ts.writable.getWriter();
        const reader = ts.readable.getReader();
        writer.write('ab');
        writer.close();
        Promise.all([reader.read(), reader.read(), reader.read()]).then((parts) => {
            return [
                ts.readable instanceof ReadableStream,
                parts.map((part) => part.done ? 'done' : part.value).join(',')
            ].join('|');
        });
        "#);
    assert_eq!(output, "true|AB,END,done");
}

#[test]
#[serial]
fn queuing_strategies_size_chunks_and_do_not_gate_enqueue() {
    let output = run(r#"
        let missing = '';
        try { new CountQueuingStrategy(); } catch (e) { missing = e instanceof TypeError; }
        const count = new CountQueuingStrategy({ highWaterMark: 0 });
        const bytes = new ByteLengthQueuingStrategy({ highWaterMark: 1 });
        const stream = new ReadableStream({
            start(controller) {
                controller.enqueue('still-queued');
                controller.close();
            }
        }, count);
        stream.getReader().read().then((part) => {
            return [
                missing,
                count.size(),
                bytes.size(new Uint8Array([1, 2, 3])),
                bytes.size({}),
                part.value
            ].join('|');
        });
        "#);
    assert_eq!(output, "true|1|3|0|still-queued");
}

#[test]
#[serial]
fn compression_round_trip_gzip_deflate_and_raw() {
    let output = run(r#"
        function textOf(bytes) {
            let text = '';
            for (let i = 0; i < bytes.length; i++) text += String.fromCharCode(bytes[i]);
            return text;
        }
        async function roundTrip(format) {
            const input = new TextEncoder().encode('Hello, streams ' + format);
            const cs = new CompressionStream(format);
            const writer = cs.writable.getWriter();
            const reader = cs.readable.getReader();
            await writer.write(input);
            const compressed = (await reader.read()).value;
            const ds = new DecompressionStream(format);
            const outWriter = ds.writable.getWriter();
            const outReader = ds.readable.getReader();
            await outWriter.write(compressed);
            const output = (await outReader.read()).value;
            return {
                format: cs.format,
                first: compressed[0],
                second: compressed[1],
                text: textOf(output)
            };
        }
        Promise.all(['gzip', 'deflate', 'deflate-raw'].map(roundTrip)).then((rows) => {
            return rows.map((row) => row.format + ':' + row.first + ':' + row.second + ':' + row.text).join('\n');
        });
        "#);
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 3, "got {output}");
    assert!(
        lines[0].starts_with("gzip:31:139:Hello, streams gzip"),
        "gzip member: {output}"
    );
    assert!(
        lines[1].starts_with("deflate:120:"),
        "zlib header: {output}"
    );
    assert!(
        lines[1].ends_with("Hello, streams deflate"),
        "deflate text: {output}"
    );
    assert!(lines[2].starts_with("deflate-raw:"), "raw format: {output}");
    assert!(
        lines[2].ends_with("Hello, streams deflate-raw"),
        "raw text: {output}"
    );
    assert!(
        !lines[1].starts_with("deflate:31:139:"),
        "deflate must not be a gzip member: {output}"
    );
}

#[test]
#[serial]
fn compression_rejects_an_unknown_format() {
    let output = run(r#"
        let name = '';
        let message = '';
        try {
            new CompressionStream('brotli');
        } catch (e) {
            name = e.name;
            message = e.message;
        }
        name + '|' + message;
        "#);
    assert!(output.starts_with("RangeError|"), "got {output}");
    assert!(output.contains("unsupported format"), "got {output}");
}

#[test]
#[serial]
fn fetch_response_body_stays_outside_readable_stream() {
    let output = run(r#"
        const response = new Response('ab');
        const body = response.body;
        const reader = body.getReader();
        const first = reader.read();
        let text = '';
        const bytes = first.value;
        if (bytes && typeof bytes.length === 'number') {
            for (let i = 0; i < bytes.length; i++) text += String.fromCharCode(bytes[i]);
        }
        let locked = '';
        try { body.getReader(); } catch (e) { locked = e.message; }
        [
            body instanceof ReadableStream,
            typeof body.pipeTo,
            first.done,
            text,
            locked
        ].join('|');
        "#);
    assert_eq!(
        output, "false|undefined|false|ab|body stream is locked",
        "fetch body changed: {output}"
    );
}

#[test]
#[serial]
fn timers_run_microtasks_before_zero_delay_and_clear_cancels() {
    reset_timers();
    let output = run(r#"
        new Promise((resolve) => {
            const order = [];
            order.push('sync');
            queueMicrotask(() => order.push('micro'));
            Promise.resolve().then(() => order.push('then'));
            let cleared = 0;
            const cancelled = setTimeout(() => { cleared = 1; }, 40);
            clearTimeout(cancelled);
            setTimeout((label) => {
                order.push(label);
                order.push('cleared=' + cleared);
                resolve(order.join(','));
            }, 0, 'timeout');
        });
        "#);
    assert_eq!(output, "sync,micro,then,timeout,cleared=0");
}

#[test]
#[serial]
fn set_interval_repeats_until_clear_interval() {
    reset_timers();
    let mut runtime = runtime();
    runtime.set_timer_drain_limit_ms(400);
    let output = runtime
        .execute_code(
            r#"
            new Promise((resolve) => {
                let n = 0;
                const id = setInterval(() => {
                    n += 1;
                    if (n === 2) {
                        clearInterval(id);
                        resolve('ticks:' + n + ':' + typeof id + ':' + typeof id._timerId);
                    }
                }, 20);
            });
            "#,
        )
        .unwrap_or_else(|err| panic!("interval failed: {err}"));
    assert_eq!(output.trim(), "ticks:2:object:number");
}

#[test]
#[serial]
fn timer_callbacks_must_be_functions() {
    reset_timers();
    let err = runtime()
        .execute_code("setTimeout(1, 0);")
        .expect_err("non-function callback");
    let message = err.to_string();
    assert!(
        message.contains("callback must be a function"),
        "got {message}"
    );

    let err = runtime()
        .execute_code("queueMicrotask('nope');")
        .expect_err("non-function microtask");
    let message = err.to_string();
    assert!(
        message.contains("callback must be a function"),
        "got {message}"
    );
}

#[test]
#[serial]
fn performance_now_marks_and_measures() {
    clear_performance_entries();
    let output = run(r#"
        const first = performance.now();
        const second = performance.now();
        performance.mark('streams-start');
        performance.mark('streams-end');
        performance.measure('streams-span', 'streams-start', 'streams-end');
        const span = performance.getEntriesByName('streams-span')[0];
        const marks = performance.getEntriesByType('mark').filter((entry) => entry.name === 'streams-start');
        let empty = '';
        try { performance.mark(''); } catch (e) { empty = e instanceof TypeError; }
        performance.clearMarks();
        const afterClear = performance.getEntriesByType('mark').length;
        [
            typeof first,
            first >= 0,
            second >= first,
            performance.timeOrigin > 1700000000000,
            span.entryType,
            span.duration >= 0,
            marks.length,
            empty,
            afterClear,
            typeof Performance,
            typeof performance.toJSON().now
        ].join('|');
        "#);
    assert_eq!(
        output,
        "number|true|true|true|measure|true|1|true|0|undefined|number"
    );
}

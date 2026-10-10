# Web streams, compression, timers, and performance contract

This is the user-facing contract for the Stable subset of Web streams, `CompressionStream` / `DecompressionStream`, the global timers, and `performance` in the default Amber runtime. It is derived from `src/web_api/streams.rs`, `src/web_api/streams_fast.js`, `src/web_api/compression.rs`, `src/nodejs_core/timers.rs`, `src/nodejs_core/performance.rs`, `src/runtime_minimal.rs`, and `tests/streams_timers_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

These globals are reachable from `amber run` / `amber eval`. The rest of `src/web_api/` stays Preview. This is not the WHATWG Streams, Compression Streams, HTML timers, or Performance Timeline standards.

`fetch`'s `response.body` is unchanged. It is the pull reader in [`docs/FETCH_CONTRACT.md`](FETCH_CONTRACT.md). It is not `instanceof ReadableStream`, it has no `pipeTo`, and a second `getReader()` throws `body stream is locked`. `read()` on that reader returns `{ done, value }` synchronously.

## ReadableStream

`ReadableStream` is the constructor installed by `setup_streams_api`. `ReadableStream.from(iterable)` enqueues each value from a sync iterable or an async iterable, then closes.

- `start(controller)` runs during construction. `controller.enqueue(chunk)` and `controller.close()` record chunks already produced.
- `getReader()` locks the stream. A second `getReader()` throws `TypeError` (`ReadableStream is locked`). `releaseLock()` clears that lock and does not rewind the queue.
- `read()` returns a Promise of `{ done, value }`. A chunk queued before the read resolves `{ done: false, value }`. After those chunks are consumed, or when the queue is empty, `read()` resolves `{ done: true }`. It does not wait for a later `enqueue`.
- `pipeTo(writable)` locks the readable, writes already queued chunks, and resolves. `pipeThrough(pair)` returns `pair.readable` after starting `pipeTo(pair.writable)`.
- `Symbol.asyncIterator` reads with `getReader()` until `done`.

## WritableStream

`WritableStream` accepts an underlying sink.

- `start(controller)` runs during construction.
- `getWriter()` locks the stream. A second `getWriter()` throws `TypeError` (`WritableStream is locked`).
- `write(chunk)` returns a Promise. While the stream is open it calls the sink `write` with that chunk. After `writer.close()`, a later `write` does not call the sink.
- `writer.close()` marks the writable closed and resolves. It does not call a sink `close` function.

## TransformStream

`TransformStream` has `readable` and `writable`.

- `transform(chunk, controller)` runs on `writer.write(chunk)`. `controller.enqueue` is what the readable side yields.
- `flush(controller)` runs when `writer.close()` is called, before the readable side reports closed.
- `readable instanceof ReadableStream` is true. Its `getReader()` reads the transform queue, not a second independent stream.

## Queuing strategies

`CountQueuingStrategy` and `ByteLengthQueuingStrategy` are constructors.

- Both require `options.highWaterMark`. Omitting it throws `TypeError`.
- `CountQueuingStrategy.prototype.size()` returns `1`.
- `ByteLengthQueuingStrategy.prototype.size(chunk)` returns `chunk.byteLength`, or `0` when the chunk has no `byteLength`.
- Passing either strategy as the second argument to `ReadableStream` or `WritableStream` does not change enqueue, `read()`, or `desiredSize`. The constructors do not apply it as backpressure.

## CompressionStream and DecompressionStream

Formats are `gzip`, `deflate`, and `deflate-raw`. Any other string throws `RangeError` whose message contains `unsupported format`. The `format` property is the lowercased name.

- `gzip` is a gzip member (RFC 1952). The first two bytes are `0x1f`, `0x8b`.
- `deflate` is a zlib member (RFC 1950), not raw DEFLATE and not gzip.
- `deflate-raw` is one raw DEFLATE member (RFC 1951).
- `writable.getWriter().write(chunk)` compresses or decompresses that chunk as one complete member and enqueues the result on `readable`. A string chunk is UTF-8 encoded first. `Uint8Array` chunks are used as bytes.
- A following `read()` returns that member. Empty input still produces a gzip or zlib header member; a failed decode enqueues nothing.
- Each write is its own member. This is not one compressed stream continued across writes, and a single write that concatenates several members is not decoded as a multi-member sequence.
- `close()` on the compression or decompression object calls `writable.close()`.

`_compressData` and `_decompressData` are the same codecs. They are helpers for this implementation, not a second public API.

## Timers

`setTimeout`, `setInterval`, `clearTimeout`, `clearInterval`, and `queueMicrotask` are the globals installed by `src/runtime_minimal.rs` from `src/nodejs_core/timers.rs`. `src/web_api/timers.rs` is not installed on this path and is not this contract.

- A non-function callback throws `TypeError` (`callback must be a function`) for `setTimeout`, `setInterval`, and `queueMicrotask`.
- `setTimeout` and `setInterval` return a timer object with a numeric `_timerId`. `clearTimeout` and `clearInterval` accept that object or a numeric id. Clearing a timer removes its callback.
- `setTimeout(fn, 0)` does not run inside the current turn. `queueMicrotask` and an already queued `Promise` reaction run before that timer. The order in one turn is synchronous code, then `queueMicrotask` callbacks in call order, then other microtasks queued before the checkpoint, then the zero-delay timer.
- `setTimeout(fn, delay)` with `delay > 0` runs after at least that many milliseconds, on a later event-loop turn. The runtime keeps draining ref'd timers until they fire or are cleared.
- `setInterval` repeats until `clearInterval`. A requested delay of `0` is scheduled as `1` millisecond. Extra arguments after the delay are passed to the callback.
- `queueMicrotask` queues a V8 microtask. It does not run the callback before the current turn returns.

`setImmediate` is installed next to these timers and is outside this contract.

## performance

`globalThis.performance` is the object installed from `src/nodejs_core/performance.rs` (re-exported by `src/web_api/performance.rs`). There is no `Performance` constructor.

- `performance.now()` returns milliseconds since the process clock started. It is a finite number `>= 0`. A later call is `>=` an earlier call in the same process. With `--freeze-time`, `now()` stays `0`.
- `performance.timeOrigin` is the Unix epoch in milliseconds captured when the clock was created, or the frozen time when `--freeze-time` is set.
- `performance.mark(name)` records a mark. An empty name throws `TypeError`.
- `performance.measure(name, startMark, endMark)` records a measure whose `duration` is the end mark minus the start mark. A missing mark uses `0` for the start and `now()` for the end.
- `getEntries()`, `getEntriesByName(name)`, and `getEntriesByType(type)` return arrays of `{ name, entryType, startTime, duration }`. `entryType` is `"mark"` or `"measure"`.
- `clearMarks()` drops marks. `clearMeasures()` drops measures.
- `toJSON()` returns `{ now, timeOrigin }`.

Marks and measures are process-global. This is not a per-document timeline, and it does not implement `PerformanceObserver`.

## Non-goals

- A `read()` that stays pending until a later `enqueue`.
- Byte-stream controllers, BYOB readers as a stable promise, or backpressure from queuing strategies.
- `writer.close()` invoking the sink `close` callback.
- One compression context continued across `write` calls, `deflate` as raw DEFLATE, or a multi-member gzip buffer decoded in one `write`.
- `response.body` as a `ReadableStream`. See [`docs/FETCH_CONTRACT.md`](FETCH_CONTRACT.md).
- `setImmediate` and `require('timers')`.
- Node `require('stream')`. That module has its own Stable subset contract in [`docs/NODE_STREAM_CONTRACT.md`](NODE_STREAM_CONTRACT.md) (**G24**). This page does not graduate it.
- A `Performance` constructor, `PerformanceObserver`, or resource timing.
- Node `zlib` (`require('zlib')`). That module has its own Stable sync contract in [`docs/ZLIB_CONTRACT.md`](ZLIB_CONTRACT.md) and async `gzip` / `gunzip` contract in [`docs/ZLIB_ASYNC_CONTRACT.md`](ZLIB_ASYNC_CONTRACT.md). This page does not graduate it.

## Tests

```bash
cargo test --test streams_timers_contract_tests -- --test-threads=1
```

CI runs that command as `web streams timers Stable contract`.

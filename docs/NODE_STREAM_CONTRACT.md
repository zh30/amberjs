# Node `stream` contract

This is the user-facing contract for the Stable subset of Node `stream` in Amber. It is derived from `src/nodejs_core/stream.rs`, `src/runtime_minimal.rs` (`require('stream')` / `require('node:stream')` return `globalThis.stream` installed by `setup_stream_api`), `tests/node_stream_contract_tests.rs`, and `tests/stream_transform_contract_tests.rs` (**tentative G71** rows). Historical `docs/STAGE_*` reports are not part of this contract.

This is **not** the Stable Web streams surface in [`docs/STREAMS_TIMERS_CONTRACT.md`](STREAMS_TIMERS_CONTRACT.md) (G13). G13 does not graduate `require('stream')`. This page is an independent Node module contract (**G24**; path G17 through crypto G23 and additive G8 `existsSync` are Stable on `main`).

Methods, constructors, and helpers that are not listed in the Stable surface table are outside this contract. Stub or incomplete exports that remain on the module object (`Duplex`, `Readable.from`, `finished`, and friends) stay **Preview**. They are Non-goals here; they are **not** Stable “limits”.

## Stable surface

| Call / constructor | Behavior |
| :--- | :--- |
| `require('stream')` / `require('node:stream')` | Same object as `globalThis.stream` after `setup_stream_api`. |
| `Readable` | `new stream.Readable({ read })` or `{ _read }`. In flowing mode (`on('data')` or `pipe`), `push(chunk)` delivers that chunk to the `data` listener. `push(null)` ends the readable side and, when piped, ends the destination. |
| `Readable.prototype.pipe(dest)` | Pipes flowing `data` into `dest.write`. Returns `dest`. When the source ends, calls `dest.end()` when present. |
| `Writable` | `new stream.Writable({ write })` or `{ _write }`. `write(chunk[, encoding][, callback])` invokes the user `_write` / `write` with `(chunk, encoding, callback)` and returns a boolean. `end()` marks the writable finished and fires `finish` after the write path settles. |
| `PassThrough` / `passThrough` | Same constructor under both names. `write(chunk)` emits `data` with that chunk and, when `pipe(dest)` was called, also calls `dest.write(chunk)`. `pipe(dest)` stores the destination and returns `dest`. `end()` fires `finish` then `end` listeners and cascades `dest.end()` down the pipe chain (**tentative G71**). |
| `Transform` (**tentative G71**) | `new stream.Transform({ transform })` or `{ _transform }`, and `{ flush }` / `{ _flush }` (both spellings accepted). Writes enter `_transform(chunk, encoding, callback)`; `callback(err)` fires the write-side callback, `callback(err, output)` additionally pushes `output` to the readable side. `push(chunk)` emits `data` when flowing; `push(null)` ends the readable side and fires the `end` listener. `end([chunk][, enc][, cb])` writes the final chunk, runs `flush(flushCb)` when defined (`flushCb(out)` pushes `out` then finishes), then fires `finish`. Without a user `transform`, the stream is identity (chunk passes through). |
| `Transform.prototype.pipe` / `unpipe` (**tentative G71**) | `pipe(dest)` registers `data`/`end` forwarders (`data` → `dest.write(chunk)`; `end` → `dest.end()`), sets flowing, and returns `dest`. `unpipe()` clears the forwarders and `_pipeDest`. Works into another `Transform` (transform→transform chains) or a `Writable`. |
| `stream.pipeline(src, ...throughs, dest[, cb])` (**tentative G71**) | Chains `pipe` across the arguments and attaches a completion listener to the last stream's `finish` (`_events.finish` for Transform/PassThrough-shaped streams, the Writable `finish` slot for Writables). The callback runs once after the destination finishes. |

## Limits

These are real behaviors of the Stable surface above, not stand-ins for missing APIs:

- `Writable.write` / PassThrough `write` always return `true` in the current host. There is no `drain` event and no high-water-mark backpressure on this contract.
- Stream `on(event, listener)` keeps a single listener per event name. `Transform` / `PassThrough` listeners live in `this._events[name]` (an event-named property would collide with method slots like `end`); `Writable` uses the property-slot model. This is not full multi-listener `EventEmitter` fan-out.
- `unpipe` clears the single `data` / `end` listener slots — it cannot distinguish pipe forwarders from user listeners on those names.
- Paused-mode buffering is outside this contract: without a `data` listener or `pipe`, `push` does not queue chunks for a later `read()`.
- Chunk values are passed through as given (typically strings in the contract tests). Encoding pipelines and `objectMode` are not pinned.
- `pipeline` accepts the callback form only; `require('stream/promises').pipeline` Promise settlement stays outside Stable.
- Transform `write(chunk, cb)` normalizes `(chunk[, encoding][, callback])` like `Writable` — a second-argument function is treated as the callback.

## Non-goals

Outside this contract (Preview or unimplemented). Do **not** treat these as Stable limits of a stub:

- `Duplex` as a user-constructed class (Transform covers the chained read/write path)
- `require('stream/promises').pipeline` and `finished` Promise settlement
- `destroy` / `_destroy`, `cork` / `uncork`, `error` event propagation across a pipe chain
- Real backpressure (`needDrain`, false from `write` / `push` when over HWM)
- `Readable.from`, `Duplex.from`
- `toWeb` / `fromWeb`, `compose`, `isReadable` / `isWritable`, default high-water-mark helpers
- WHATWG `ReadableStream` / `WritableStream` / `TransformStream` (see G13)

## Reachability

The CLI `amber` binary installs `globalThis.stream` from `src/runtime_minimal.rs` via `nodejs_core::stream::setup_stream_api`. `require('stream')` / `require('node:stream')` resolve that object through the CommonJS builtin path. Library users reach the installer through `amberjs::nodejs_core::stream::setup_stream_api` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.

## Tests

```bash
cargo test --test node_stream_contract_tests -- --test-threads=1
cargo test --test stream_transform_contract_tests -- --test-threads=1
```

CI runs those commands as `node stream Stable contract` and `node stream transform Stable contract` (**tentative G71**).

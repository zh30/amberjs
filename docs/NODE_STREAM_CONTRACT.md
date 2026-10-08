# Node `stream` contract

This is the user-facing contract for the Stable subset of Node `stream` in Amber. It is derived from `src/nodejs_core/stream.rs`, `src/runtime_minimal.rs` (`require('stream')` / `require('node:stream')` return `globalThis.stream` installed by `setup_stream_api`), and `tests/node_stream_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

This is **not** the Stable Web streams surface in [`docs/STREAMS_TIMERS_CONTRACT.md`](STREAMS_TIMERS_CONTRACT.md) (G13). G13 does not graduate `require('stream')`. This page is an independent Node module contract (**G24**; path G17 through crypto G23 and additive G8 `existsSync` are Stable on `main`; CommonJS `require` remains last in wave-2 after child_process→url→dns).

Methods, constructors, and helpers that are not listed in the Stable surface table are outside this contract. Stub or incomplete exports that remain on the module object (`Transform`, `Duplex`, `pipeline`, and friends) stay **Preview**. They are Non-goals here; they are **not** Stable “limits”.

## Stable surface

| Call / constructor | Behavior |
| :--- | :--- |
| `require('stream')` / `require('node:stream')` | Same object as `globalThis.stream` after `setup_stream_api`. |
| `Readable` | `new stream.Readable({ read })` or `{ _read }`. In flowing mode (`on('data')` or `pipe`), `push(chunk)` delivers that chunk to the `data` listener. `push(null)` ends the readable side and, when piped, ends the destination. |
| `Readable.prototype.pipe(dest)` | Pipes flowing `data` into `dest.write`. Returns `dest`. When the source ends, calls `dest.end()` when present. |
| `Writable` | `new stream.Writable({ write })` or `{ _write }`. `write(chunk[, encoding][, callback])` invokes the user `_write` / `write` with `(chunk, encoding, callback)` and returns a boolean. `end()` marks the writable finished and fires `finish` after the write path settles. |
| `PassThrough` / `passThrough` | Same constructor under both names. `write(chunk)` emits `data` with that chunk and, when `pipe(dest)` was called, also calls `dest.write(chunk)`. `pipe(dest)` stores the destination and returns `dest`. |

## Limits

These are real behaviors of the Stable surface above, not stand-ins for missing APIs:

- `Writable.write` / PassThrough `write` always return `true` in the current host. There is no `drain` event and no high-water-mark backpressure on this contract.
- Stream `on(event, listener)` keeps a single listener per event name (property overwrite). It is not full multi-listener `EventEmitter` fan-out for those events.
- Paused-mode buffering is outside this contract: without a `data` listener or `pipe`, `push` does not queue chunks for a later `read()`.
- Chunk values are passed through as given (typically strings in the contract tests). Encoding pipelines and `objectMode` are not pinned.
- PassThrough `end()` does not call `dest.end()` and does not fire `finish` on the pipe destination. Only Readable→Writable `pipe` ends the destination when the source pushes `null`.

## Non-goals

Outside this contract (Preview or unimplemented). Do **not** treat these as Stable limits of a stub:

- `Transform` and `Duplex` (including their `pipe` / `unpipe` methods)
- `stream.pipeline` and `require('stream/promises').pipeline` completion callbacks / Promise settlement
- `unpipe`, `destroy` / `_destroy`, `cork` / `uncork`
- Real backpressure (`needDrain`, false from `write` / `push` when over HWM)
- `Readable.from`, `Duplex.from`, `finished` Promise/error paths as Stable promises
- `toWeb` / `fromWeb`, `compose`, `isReadable` / `isWritable`, default high-water-mark helpers
- Transform `callback(null, chunk)` auto-push semantics
- WHATWG `ReadableStream` / `WritableStream` / `TransformStream` (see G13)

## Reachability

The CLI `amber` binary installs `globalThis.stream` from `src/runtime_minimal.rs` via `nodejs_core::stream::setup_stream_api`. `require('stream')` / `require('node:stream')` resolve that object through the CommonJS builtin path. Library users reach the installer through `amberjs::nodejs_core::stream::setup_stream_api` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.

## Tests

```bash
cargo test --test node_stream_contract_tests -- --test-threads=1
```

CI runs that command as `node stream Stable contract`.

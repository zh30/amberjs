# Node `process.stdin` contract

This is the user-facing contract for the Stable `process.stdin` slice in Amber (**tentative G70**). It is derived from `src/nodejs_core/process.rs` (stdin reader thread + pump), `src/runtime_minimal.rs` (stdin pump wiring), and `tests/process_stdin_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

The broader `process` surface stays covered by [`docs/PROCESS_CONTRACT.md`](PROCESS_CONTRACT.md); this page owns only the `process.stdin` readable-stream slice. `readline.createInterface({ input: process.stdin })` stays governed by [`docs/READLINE_CONTRACT.md`](READLINE_CONTRACT.md) — G70 is what makes that contract's host stdin reads real.

**Honesty:** `process.stdin.read()` previously existed but always returned `null` and no `'data'` / `'end'` was ever delivered. Do **not** graduate a stubbed `read() → null`, an absent `'data'` emit, or a fabricated `'end'`.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `process.stdin.fd` | Always `0`. |
| `process.stdin.isTTY` | `true` when fd 0 is a real terminal at startup, `false` when stdin is a pipe/file/closed. Reads the real host TTY state. |
| `process.stdin.read()` | Non-blocking pop of the next buffered chunk as a **UTF-8 string**, or `null` when the queue is empty (and at EOF). Calling `read()` starts the host reader — data written to fd 0 before the first call is not lost — but does **not** keep the event loop alive on its own (a bare `read()` never blocks process exit). |
| `process.stdin.on('data', fn)` / `once` / `addListener` | Attaching a `'data'` listener starts the reader and switches stdin to flowing mode: each host chunk is delivered to `fn` as a string on a later event-loop pump turn. |
| `process.stdin.on('readable', fn)` | Starts the reader without flowing: the first `'readable'` emits when buffered data is available; `fn` then drains via `read()`. Re-signals when the queue becomes non-empty again after being fully drained. |
| `process.stdin.on('end', fn)` | Emitted once after host EOF when the buffered queue has been fully delivered/drained, while stdin holds liveness. |
| `process.stdin.resume()` | Switches to flowing mode, holds the event loop alive, and starts the reader. Returns `process.stdin` (chainable). |
| `process.stdin.pause()` | Stops `'data'` delivery and releases the event-loop keep-alive (acts as `unref`): a paused stdin does not hold the process open. Buffered bytes are kept. Returns `process.stdin`. |
| `process.stdin.unref()` / `ref()` | Toggle whether an active (started) stdin keeps the event loop alive. `ref()` on a never-started stdin is a no-op. Both return `process.stdin`. |
| `process.stdin.setEncoding('utf8')` | Accepted (chunks are already strings). Also accepts `'utf-8'`, `null`, `undefined` — all case-insensitive on the `utf8`/`utf-8` spelling. Any other value throws `TypeError`. Returns `process.stdin`. |
| `process.stdin instanceof` / `.on` / `.once` / `.emit` / `.listenerCount` | `process.stdin` is an EventEmitter (prototype set to `EventEmitter.prototype` at install); the standard emitter methods from G-contract events are available. |

Wire proof required by this contract: piping bytes into `amber eval` with a `'data'` listener delivers the real bytes as a string chunk, and host EOF produces exactly one `'end'` emit. With a `'readable'` listener, `'readable'` fires and `read()` returns the real bytes, not `null`.

## Limits

These are real behaviors of the Stable stdin surface, not cover for missing reads:

- Chunks are always **UTF-8 strings** (invalid bytes replaced via lossy decoding). There are no Buffer chunks on stdin — `setEncoding` exists only to validate the encoding argument, not to switch modes.
- Event delivery is isolate-thread pump based (same model as G45 net / G64 fs.watch). A `'data'` listener's callback runs on a later pump turn — never synchronously inside `on()` — and tests must let the event loop pump.
- Chunk boundaries follow host `read()` granularity (≤ 8 KiB per chunk), not line boundaries or Node's exact segmentation. Concatenate chunks if you need the full input.
- `pause()` deliberately doubles as the liveness release (Node's `unref` semantics): pausing a flowing stdin lets the process exit once queued `'end'` work is gone or nothing remains pinned. Resume with `resume()` to keep reading.
- `'end'` is delivered only while stdin holds liveness when EOF arrives — stdin paused/unref'd at EOF means the process may exit before `'end'`.
- One stdin exists per process: the reader starts on first use (`read()`, `resume()`, `'data'`/`'readable'` listener) and cannot be restarted after EOF.
- `process.stdin` state is **process-global**: multiple runtimes/isolates inside one host process share the single fd-0 queue — a second runtime cannot start an independent stdin after EOF, and runtimes can consume each other's buffered input. Scripts that need stdin should own it in the main runtime (the CLI model).
- Mixing an active `process.stdin` reader with `readline.question` in the same process splits fd-0 bytes between two consumers (the reader re-locks per host read rather than starving readline outright). Pin stdin to one consumer — Node has the same "two readers on one stream" hazard in spirit.
- `isTTY` reflects the real fd-0 terminal state; a TTY stdin with a `'data'` listener keeps the process alive until `pause()`/`unref()` — matching Node, not a bug.
- `'close'`, `'error'`, `'timeout'` events, `read(0)`/size arguments, `isRaw`/`setRawMode`, and `iter`/`Symbol.asyncIterator` are not pinned by this contract.

## Non-goals

Outside this contract (Preview / unimplemented). Do **not** treat these as Stable Limits of dishonest behavior:

- Buffer chunks / `setEncoding` for non-UTF-8 encodings (`'ascii'`, `'base64'`, `'hex'`, ...)
- `process.stdin.setRawMode` / TTY raw input
- `for await (const chunk of process.stdin)` async iteration
- `process.stdin.pipe()` / interop with `stream.Writable`
- Backpressure / `highWaterMark` knobs, `_read` internals
- `process.stdout`/`process.stderr` stream semantics beyond G22 (writable events, `.write()` chunking, `.end()`)
- A second stdin reader after EOF, or `fd` reassignment

## Reachability

`process.stdin` is installed on the CLI path in `nodejs_core::process::setup_process_api`; the EventEmitter prototype + auto-resume wrappers are attached by `nodejs_core::process::install_stdin_event_emitter`, which `install_core_apis` calls immediately after `setup_events_api` (the EE class does not exist earlier). The reader thread and pump live in `src/nodejs_core/process.rs` and are driven from the runtime's event-loop pump in `src/runtime_minimal.rs`. The contract is not feature-gated.

## Tests

```bash
cargo test --test process_stdin_contract_tests -- --test-threads=1
```

CI step: `node process stdin Stable contract`.

## Honesty rule

A `read()` that always returns `null`, a `'data'`/`'end'` that never fires, or a fabricated `'end'` listed as a Stable Limit must never graduate. Limits describe pump delivery, UTF-8-only chunks, chunk boundaries, and pause-as-unref semantics only.

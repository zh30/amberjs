# Node `events` (EventEmitter) contract

This is the user-facing contract for the Stable Node `EventEmitter` surface in Amber. It is derived from the JavaScript bootstrap in `src/nodejs_core/events.rs` (installed by `runtime_minimal::install_core_apis`), `require('events')` / `require('node:events')` in `src/runtime_minimal.rs`, and `tests/node_events_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

This is **not** the Stable web `Event` / `EventTarget` / `CustomEvent` contract in [`EVENTS_CHANNELS_CONTRACT.md`](EVENTS_CHANNELS_CONTRACT.md) (G12). Node `EventEmitter` and web `EventTarget` are separate APIs.

The rest of `src/nodejs_core/` stays Preview, including Node `url`, `crypto`, `buffer`, `path`, `os`, streams, HTTP, and networking.

## Reachability

| Path | Shape |
| :--- | :--- |
| `require('events')` / `require('node:events')` | The `EventEmitter` constructor function itself. |
| `events.EventEmitter` / destructuring `{ EventEmitter }` | The same constructor (`EventEmitter.EventEmitter === EventEmitter`). |
| `import eventsDefault, { EventEmitter } from 'events'` / `'node:events'` | Default and named export are that constructor. |
| `globalThis.events` / `globalThis.EventEmitter` | Same constructor after install. |

`new EventEmitter()` creates an emitter with a private `_events` map. Listeners are per instance.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `on` / `addListener` | Appends a listener. Returns `this`. A non-function listener throws `TypeError` (`The "listener" argument must be of type Function`). |
| `prependListener` | Inserts at the front of that event's list. Returns `this`. |
| `once` / `prependOnceListener` | Registers a wrapper that removes itself after the first emit. `listeners(type)` returns the original function; `rawListeners(type)` returns the wrapper. |
| `emit` | Calls listeners in registration order (prepended first). Returns `true` when at least one listener ran, otherwise `false`. Arguments are forwarded. |
| `off` / `removeListener` | Removes one matching listener (identity or wrapped `once` listener). Returns `this`. |
| `removeAllListeners([type])` | Clears one event name, or every event when `type` is omitted. Returns `this`. |
| `listeners` / `rawListeners` | New arrays. `listeners` unwraps `once` wrappers; `rawListeners` does not. |
| `listenerCount(type)` | Number of listeners for that name on this emitter. |
| `eventNames()` | Own keys of `_events` when any listeners remain. |
| `setMaxListeners(n)` / `getMaxListeners()` | Per-emitter cap. Default when unset is `EventEmitter.defaultMaxListeners` (`10`). `n` must be a non-negative number; otherwise `RangeError`. `0` means no warning threshold. |
| `EventEmitter.listenerCount(emitter, type)` | Delegates to `emitter.listenerCount(type)`. |
| `EventEmitter.once(emitter, type[, options])` | Returns a `Promise` that resolves to the argument array from the next `type` emit. Rejects on the emitter's `error` (unless `type` is `'error'`). Optional `options.signal` (`AbortSignal`): already aborted or later `abort` rejects with `Error('This operation was aborted')`. |

## Emit and meta-events

- Emitting `'error'` with no listeners throws: an `Error` argument is rethrown; otherwise `Error('Unhandled error. …')` with `.context` set to the argument.
- When a `newListener` listener is registered, each later `on` / `once` / `prepend*` emits `'newListener'` with `(type, listener)` before the new listener is stored.
- When a `removeListener` listener is registered, a successful `removeListener` / `off` emits `'removeListener'` with `(type, listener)`.
- Exceeding `getMaxListeners()` (when greater than `0`) prints a `MaxListenersExceededWarning` string through `console.warn`. It does not throw and does not use `process.emitWarning`.

## Non-goals

- Web `EventTarget`, `Event`, `CustomEvent`, `AbortController` as the emitter model (G12).
- `events.on` async iterator, `captureRejectionSymbol`, `AsyncResource` integration, or Domain.
- Cross-isolate or worker-shared emitters.
- Node's `process.emitWarning` pipeline for max-listeners warnings.
- Full Node `events` module parity beyond the table above.

## Tests

```bash
cargo test --test node_events_contract_tests -- --test-threads=1
```

CI runs that command as `node events Stable contract`, next to the other Stable contract steps.

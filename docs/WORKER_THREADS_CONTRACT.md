# Node `worker_threads` contract

This is the user-facing contract for the Stable `worker_threads` module slice in Amber (**tentative G75**). It is derived from `src/nodejs_core/worker_threads.rs` (module shim), `src/web_api/worker_host.rs` (`WorkerHost` — real OS thread + fresh V8 isolate per worker, main-side `Worker` class, and the worker-isolate bootstrap), and `tests/worker_threads_contract_tests.rs`. The web-side `globalThis.Worker` / service-worker registration surface stays on [`docs/WORKERS_CONTRACT.md`](WORKERS_CONTRACT.md) (**G16**); this page owns the Node module bag.

**Honesty:** `require('worker_threads')` returns the real bag `globalThis.worker_threads` — not a shape-only object. Workers are real OS threads running fresh V8 isolates (not a same-isolate simulation), and messages really cross between them.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `require('worker_threads')` / `require('node:worker_threads')` | Both return the same `globalThis.worker_threads` object. |
| `worker_threads.isMainThread` | `true` on the main thread, `false` inside a worker isolate. |
| `worker_threads.parentPort` | `null` on the main thread; inside a worker, an emitter with `postMessage(data)` and `on('message', fn)` / `close()`. |
| `worker_threads.workerData` | `undefined` on the main thread; inside a worker, the `workerData` option passed to `new Worker`, carried through `JSON.stringify`. |
| `worker_threads.threadId` | `0` on the main thread; a positive integer inside a worker (global allocation across workers). |
| `new Worker(filename, options?)` | Spawns a real OS thread running a fresh V8 isolate over `filename` (resolved as a script file path). Throws synchronously when the script cannot be loaded. `options.eval` treats `filename` as source text. `options.workerData` is JSON-stringified into the worker's `workerData`. `options.type` accepts only `'classic'` — anything else throws `TypeError`. |
| `worker.postMessage(data)` | `JSON.stringify`s `data` and delivers it to the worker's `parentPort` `'message'` listeners (or `globalThis` message/`onmessage` handlers). Values `undefined`, functions, and circular objects throw inside `postMessage`. Posting to a terminated worker throws. |
| `worker.on('message' \| 'error' \| 'exit')`, `worker.onmessage` / `worker.onerror` | `'message'` receives each `parentPort.postMessage` from the worker (JSON-decoded). `'error'` receives an `Error` when the worker's script throws uncaught. `'exit'` receives the exit code (`0` after `terminate()`, `1` after an uncaught worker exception). `onmessage` is invoked with `{ data }`. |
| `worker.terminate()` | Synchronously stops the worker thread, emits `'exit'` with code `0`, and returns `undefined` (not a Promise). |
| `worker.ref()` / `worker.unref()` | Toggle whether the worker keeps the runtime's event loop alive; return `worker` for chaining. |
| `worker.threadId` | The worker's positive integer id. |
| `MessageChannel` / `MessagePort` | Present on the bag as a bare emitter pair (`port1` / `port2`); no real port plumbing — see Limits. |
| Worker-side `setTimeout` / `clearTimeout` | Work inside the worker isolate on the worker's own timer queue. |

Wire proof required by this contract: `new Worker` spawns a real thread whose isolate can `require('worker_threads')` and `require('events')`, see `workerData`, and round-trip messages both directions; an uncaught worker exception surfaces as `'error'` + `'exit' 1`; `terminate()` emits `'exit' 0` synchronously.

## Limits

These are real behaviors of the Stable slice, not cover for missing machinery:

- Messages travel as **`JSON.stringify` payloads** — this is not structured clone. `undefined`, functions, `Symbol`s, `BigInt`, `Map`/`Set`, `ArrayBuffer`/`Transferable`, and circular graphs either throw at `postMessage` or lose fidelity; there is no `transferList`.
- `terminate()` is **synchronous** and returns `undefined`, not Node's `Promise<number>`.
- `workerData` is carried by `JSON.stringify` — non-JSON values degrade or throw inside the worker's parse.
- A ref'd worker keeps the main runtime alive (Node-parity): `unref()` or `terminate()` it to let the process/event loop finish.
- Inside the worker isolate `require` is a **whitelist**: only `worker_threads` / `node:worker_threads` and `events` / `node:events` resolve; any other specifier throws.
- Inside the worker isolate `Worker` exists but constructing it throws `"Nested Worker is not supported"`.
- `MessageChannel` / `MessagePort` are exposed only as a bare two-emitter pair; there is no port transfer, `postMessage` between ports, or `receiveMessageOnPort`.
- Host-side messages queue until the worker's loop dispatches them; a message dispatched while no `'message'` listener is registered on `parentPort` is dropped (fire-and-forget `emit`, no replay).
- `worker.onmessage` receives `{ data }` shaped like a `MessageEvent`; it is not an `Event` instance.
- Worker scripts see a minimal global surface (`require` whitelist, timers, `postMessage`, `addEventListener('message')`, `onmessage`, `self`, console) — not the full main-isolate API set.

## Non-goals

`getEnvironmentData` / `setEnvironmentData`, `SHARE_ENV`, `resourceLimits`, `stdout` / `stderr` / `stdin` worker streams, `markAsUncloneable` / `markAsUntransferable`, `moveMessagePortToContext`, `receiveMessageOnPort`, `BroadcastChannel` (module export), `isMarkedAsUntransferable`, `options.type === 'module'`, `importScripts`-style loaders, Worker `options.argv` / `options.env` / `options.execArgv`, Atomics/`SharedArrayBuffer` sharing, and `Worker` inside workers stay outside this contract.

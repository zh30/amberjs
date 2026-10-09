# Worker and service worker contract

This is the user-facing contract for Stable dedicated `Worker` and service worker registration in the default Amber runtime. It is derived from `src/web_api/worker_host.rs`, `src/web_api/service_worker.rs`, and `tests/workers_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

Both kinds of worker are reachable from `amber run` / `amber eval`. `Worker` is `globalThis.Worker`. Service worker registration is `navigator.serviceWorker.register`. `require('worker_threads')` uses the same host. The rest of `src/web_api/` stays Preview. This is not the Web Workers standard or a browser service worker.

## Dedicated `Worker`

`new Worker(url)` reads a script file or a `data:` URL and runs it on its own thread and V8 isolate. `new Worker(source, { eval: true })` runs `source` the same way. `workerData` on that options object is delivered to `require('worker_threads').workerData` inside the worker.

- The worker global has `self`, `postMessage`, and `onmessage`. Assigning `self.onmessage` or calling `addEventListener('message', ...)` during the first turn keeps the thread alive.
- `worker.postMessage(value)` delivers `{ data: value }` to that listener. `postMessage` inside the worker delivers `{ data: value }` to `worker.onmessage` and to `worker.addEventListener('message', ...)` on the page.
- Values are JSON. Objects, arrays, strings, numbers, booleans, and `null` round-trip. Functions, symbols, `bigint`, and `undefined` do not. A circular value throws `TypeError` from `JSON.stringify`.
- `worker.terminate()` stops the thread. A later `worker.postMessage` throws `Cannot postMessage to a terminated worker`.
- A script that never listens for `message` during its first turn posts whatever it sends and then exits.
- `{ type: 'module' }` throws `TypeError` (`Worker type "module" is not supported`) before the script is loaded. `type` omitted or `"classic"` is a classic script.
- An unreadable path throws, and the message names the path. The constructor does not return a worker that never runs.

`SharedWorker` is not defined.

## Service worker registration

`navigator.serviceWorker.register(scriptURL, options)` returns a promise. A missing script rejects with the same path error as `Worker`. The promise does not resolve to an empty registration.

A readable script runs on a worker isolate:

| Step | What runs |
| :--- | :--- |
| Script evaluation | The script may call `self.addEventListener`. |
| `install` | Listeners for `'install'` run. The event object has `type` and `waitUntil(promise)`. |
| `waitUntil` during install | `register`'s promise stays pending until every `waitUntil` promise settles. A rejection fails registration (same error path as a script throw). |
| After install, without `self.skipWaiting()` | The promise resolves after install `waitUntil` settles. `registration.waiting.state` is `"installed"`. `registration.active` is `null`. |
| `self.skipWaiting()` during install | An `activate` event follows (after install `waitUntil` settles). Activate also supports `waitUntil`. `registration.active.state` is `"activated"` when the promise resolves. |
| `self.clients.claim()` during activate | `navigator.serviceWorker.controller` is that same active worker object when the promise resolves. |

`registration.scope` is `options.scope` when that option is a string. Otherwise it is `"/"` for a `data:` URL and the parent directory of a file script (trailing slash kept). A non-string or empty scope throws `TypeError` before the script loads. Scope is not matched against `fetch`.

`registration.active.postMessage(value)` and `controller.postMessage(value)` deliver a `'message'` event in the service worker. `event.data` is the JSON value. `event.source.postMessage(value)` delivers `{ data: value }` to `navigator.serviceWorker.onmessage` and to `addEventListener('message', ...)` on `navigator.serviceWorker`.

`registration.unregister()` stops the thread and resolves `true`. `postMessage` on that worker then throws.

`{ type: 'module' }` throws `TypeError` (`ServiceWorker type "module" is not supported`).

`navigator.serviceWorker.ready` resolves with the registration after the first worker reaches `"activated"`. It stays pending when the worker stops at `"installed"`.

## Non-goals

- `SharedWorker`.
- Module workers (`{ type: 'module' }`).
- Structured clone, transfer lists, and `SharedArrayBuffer` messaging.
- Fetch interception is **not this Stable contract**. A G16 registration that has no `fetch` listener does not wrap `fetch`; `fetch` keeps its identity (`tests/workers_contract_tests.rs`). Intercept (`FetchEvent.respondWith`) is **G35** ([`SW_FETCH_CONTRACT.md`](SW_FETCH_CONTRACT.md)).
- Cache API and Push API. Cache / CacheStorage is **Stable** under [`CACHE_CONTRACT.md`](CACHE_CONTRACT.md) (**G34**) and is not this contract. Push subscription still rejects.
- Browser-complete `ExtendableEvent` prototype chain / `instanceof` parity. Install/activate events are plain objects with `type` and `waitUntil`.
- `importScripts`, nested `Worker` inside a worker, and the page's DOM inside the worker isolate.

## Tests

```bash
cargo test --test workers_contract_tests -- --test-threads=1
```

CI runs that command as `web workers Stable contract`, next to the other Stable contract steps.

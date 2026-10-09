# Web service-worker `fetch` intercept contract (`FetchEvent.respondWith`)

This is the user-facing contract for Stable service-worker fetch interception in the default Amber runtime. It is derived from `src/web_api/service_worker.rs` (registration wrapper + fetch bridge), `src/web_api/cache_storage.rs` when used from `respondWith(caches.match(...))`, and `tests/service_worker_fetch_intercept_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

Interception is reachable from `amber run` / `amber eval` after `navigator.serviceWorker.register` activates a worker that registered a `'fetch'` listener. Registration, install/activate, and `waitUntil` stay **G16** ([`WORKERS_CONTRACT.md`](WORKERS_CONTRACT.md)). Cache / CacheStorage used from `respondWith(caches.match(...))` is **Stable** (**G34**) under [`CACHE_CONTRACT.md`](CACHE_CONTRACT.md). Network `fetch` without intercept stays **G9** ([`FETCH_CONTRACT.md`](FETCH_CONTRACT.md)).

**Numbering:** This contract is **G35**. Cache / CacheStorage is **G34**. Do not renumber G9–G16, G8 / G17–G28, G29–G34. G16 remains the registration contract; this page owns intercept only.

## Stable surface

### When intercept is armed

After registration resolves with an **activated** worker (`skipWaiting` + activate, or equivalent) **and** the worker script registered at least one `'fetch'` listener during evaluation, the page’s `globalThis.fetch` is replaced with an intercepting wrapper (`fetch !==` the prior G9 function).

| Step | Behavior |
| :--- | :--- |
| Page `fetch(input, init)` while armed | Returns a **Promise**. The request is snapshotted (URL, method, headers, string body) and posted to the worker as a JSON control frame. |
| Worker `'fetch'` listener | Receives a plain event: `type === "fetch"`, `request` / `requestUrl`, and `respondWith(promise)`. |
| `event.respondWith(responseOrPromise)` | Must be called at most once; argument required. Settles to a Response-like object (status, statusText, headers, body as text across the isolate). The page Promise resolves to a reconstructed `Response`. |
| Listener without `respondWith` | Worker signals pass-through; the page Promise resolves to the **native G9** `fetch(input, init)` result (network). |
| `respondWith(caches.match(...))` | Allowed; uses the Stable Cache / CacheStorage store (**G34**, same process Mutex). |

`new FetchEvent(type, init)` exists on the page isolate with `respondWith` for constructor/shape tests; live intercept uses the worker-host event object described above.

### G9 identity when there is no fetch listener (mandatory)

If the activated service worker has **no** `'fetch'` listener:

- Page `fetch` is **not** wrapped (`fetch` remains the G9 function identity).
- `fetch(url)` keeps G9 behavior: **synchronous `Response` return** for the network hop (not a Promise wrapper from intercept).
- The request hits the network as G9 specifies.

Pinned by `tests/service_worker_fetch_intercept_tests.rs` (`no_fetch_listener_keeps_g9_fetch_and_hits_network`) and by G16’s non-intercept registration tests.

## Limits

These limits are part of the Stable contract:

- **Activated + listener only.** Waiting / installed (non-activated) workers do not intercept. Unregister restores non-intercepting `fetch` when no other activated fetch-capable registration remains.
- **No scope matching.** `registration.scope` is recorded (G16) but is **not** matched against the request URL for intercept.
- **Cross-isolate JSON / text.** Request and response cross the worker as JSON control messages; response body is transferred as text. This is not structured clone of a live `Response` / `ReadableStream`.
- **Intercepted `fetch` returns a Promise.** Only the no-listener path keeps G9’s sync `Response`. Callers must `Promise.resolve(fetch(...))` or `await` when a fetch listener may be present.
- **Not navigation / CORS / opaque.** No navigation preload, no CORS special-case, no opaque filtered responses as Stable behavior.
- Double `respondWith` or missing argument throws `TypeError` in the worker. Settling to a non-Response rejects the page Promise with an ordinary `Error`.

## Non-goals

- Expanding G16 into fetch intercept (G16 stays registration / messaging / install-activate `waitUntil`).
- Browser service-worker scope matching, foreign fetch, or Clients API beyond G16 `claim` / `controller`.
- Inventing CORS, opaque responses, or navigation interception as Stable Limits while unimplemented.
- Push, Notification, Payment, SharedWorker, or module workers.
- Rewriting G9 network semantics when intercept is not armed.

## Tests

```bash
cargo test --test service_worker_fetch_intercept_tests -- --test-threads=1
```

CI runs that command as `web SW fetch Stable contract`, next to the other Stable contract steps.

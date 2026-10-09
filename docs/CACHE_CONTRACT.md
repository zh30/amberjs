# Web Cache / CacheStorage contract (`caches`)

This is the user-facing contract for Stable Cache / CacheStorage in the default Amber runtime. It is derived from `src/web_api/cache_storage.rs` (`setup_cache_api`, installed from `setup_service_worker_api` and worker isolates via `worker_host`) and `tests/cache_storage_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`caches` is reachable from `amber run` / `amber eval` through `src/runtime_minimal.rs`. The backend is a **process-wide in-memory `Mutex` map** of named caches holding Request URL/method plus Response status/headers/body. This is not HTTP cache, not disk persistence, and not browser origin-partitioned Cache Storage.

**Numbering:** This contract is **G34**. Cascade id **G33** is Background Sync (PR #183; not yet on `main` — do not treat as Stable here). Do not renumber G9–G16, G8 / G17–G28, G29–G32. Service-worker fetch intercept that calls `caches.match` is **G35** (not this page); registration without intercept stays **G16** ([`WORKERS_CONTRACT.md`](WORKERS_CONTRACT.md)).

## Stable surface

### `CacheStorage` (`globalThis.caches`)

| Call | Behavior |
| :--- | :--- |
| `caches.open(name)` | Requires a non-empty name (ToString). Creates the named cache if missing. Returns a Promise of a `Cache` object bound to that name. |
| `caches.has(name)` | Promise → `true` if the name exists in the store. |
| `caches.delete(name)` | Removes the named cache; Promise → whether it existed. |
| `caches.keys()` | Promise → string array of cache names in creation order. |

There is **no** `caches.match(...)` on CacheStorage in this contract.

### `Cache` (from `caches.open`)

| Call | Behavior |
| :--- | :--- |
| `cache.put(request, response)` | Stores a snapshot for a URL string or Request-like `{ url, method }` (fragment stripped; default method `GET`). Only **GET** or **HEAD** methods are stored. Response status/headers/body are copied (`206` rejected). Replaces any prior entry that matches the same URL + method (GET/HEAD treated as matching each other). Promise → `undefined`. |
| `cache.match(request)` | Looks up by URL + method (GET↔HEAD). Promise → reconstructed `Response`, or `undefined` on miss. |
| `cache.delete(request)` | Removes a matching entry; Promise → whether one was removed. |
| `cache.keys()` | Promise → array of reconstructed Request (or URL string fallback) for stored entries. |
| `cache.add(request)` | `fetch(request)` then `put` if `response.ok`; otherwise rejects. Requires `fetch`. |
| `cache.addAll(requests)` | Iterable of requests; fetches all, then `put`s in order; rejects if any response is not ok. |

Round-trip (CLI): after `await cache.put(url, new Response(body, init))`, `await cache.match(url)` yields a Response whose text/status/headers match the stored snapshot for ordinary string bodies.

## Limits

These limits are part of the Stable contract:

- **In-process `Mutex` only.** One process-wide backend. Not origin-partitioned, not quota-backed, not persistent across process exit. Worker isolates that install `setup_cache_api` share the same store.
- **Not HTTP cache.** No freshness / TTL, no `Vary`, no conditional revalidation, no stale-while-revalidate.
- **No match options.** `ignoreSearch`, `ignoreMethod`, `ignoreVary`, and CacheStorage-level `match` are not implemented (not present as Stable APIs).
- **Request key is URL + method.** Fragments are stripped. Query strings are part of the URL string as stored; they are not normalized beyond that.
- **Body snapshot at `put`.** The Response body is consumed into bytes for storage; later `match` reconstructs a new Response. This is not a streaming cache.
- Rejects use ordinary `TypeError` messages (empty name, bad request, non-GET/HEAD `put`, `206`, missing fetch for `add` / `addAll`), not `DOMException` / `QuotaExceededError`.

## Non-goals

- Persistent or origin-partitioned Cache Storage.
- HTTP cache semantics (freshness, Vary, revalidation).
- `CacheStorage.match`, `matchAll`, or `ignore*` option bags.
- Push, Notification, Payment, SharedWorker, or module workers.
- Rewriting G9 `fetch` ([`FETCH_CONTRACT.md`](FETCH_CONTRACT.md)) or G16 registration ([`WORKERS_CONTRACT.md`](WORKERS_CONTRACT.md)).
- Service-worker fetch intercept itself (G35).

## Tests

```bash
cargo test --test cache_storage_tests -- --test-threads=1
```

CI runs that command as `web cache Stable contract`, next to the other Stable contract steps.

# Web Background Sync contract (CLI `registration.sync`)

This is the user-facing contract for Stable Background Sync on the default Amber CLI path. It is derived from `src/web_api/background_sync.rs` (`setup_background_sync_api`) and `tests/background_sync_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`registration.sync` and `SyncEvent` are reachable from `amber run` / `amber eval` through `src/runtime_minimal.rs`. **`register` queues a tag and dispatches a real `SyncEvent` on the same CLI page isolate** — not on a service-worker isolate. This is not the browser Background Sync API, not Periodic Background Sync, and not a network-offline scheduler.

The CLI installs/updates `globalThis.registration.sync` without requiring `navigator.serviceWorker.register`. That name shape coincides with browser `ServiceWorkerRegistration.sync` by design for CLI scripts; it is not a claim that sync is delivered into a worker.

**Numbering:** This contract is **G33** after `DOMParser` (**G32**, [`DOMPARSER_CONTRACT.md`](DOMPARSER_CONTRACT.md)). Do not renumber G9–G16, G8 / G17–G28, G29–G32. Service worker registration stays **G16** ([`WORKERS_CONTRACT.md`](WORKERS_CONTRACT.md)); this page does not deliver sync into the worker.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `registration.sync.register(tag)` | Requires a non-empty tag (ToString). Queues the tag if not already queued. Returns a Promise that resolves to `undefined`. After settle, schedules a fire (prefer `setTimeout(fn, 0)`, else a microtask) that dispatches `SyncEvent` on **this isolate**. Duplicate tags while still queued do not schedule a second fire. Missing / empty tag rejects with `TypeError`. |
| `registration.sync.getTags()` | Returns a Promise that resolves to a string array of currently queued tags (removed after the matching event fires). |
| `new SyncEvent(type, init)` | Returns a plain object with SyncEvent-shaped own properties (not `instanceof Event` / `ExtendableEvent`). |
| Fired `SyncEvent` | After `register`, listeners see `type === "sync"`, `tag` equal to the registered tag, `lastChance === false`, `isTrusted === true`, and `waitUntil(promise)`. |

### How the fired event is delivered

On the CLI page isolate, dispatch calls (in order, when present):

1. `globalThis.onsync(event)`
2. `self.onsync(event)` when `self` is an object distinct from `globalThis`
3. `globalThis.dispatchEvent(event)` when that function exists

### `SyncEvent` fields / `waitUntil`

| Field / method | Behavior |
| :--- | :--- |
| `type` | Constructor: first argument ToString, or `"sync"` if omitted. Fired events use `"sync"`. |
| `tag` | From `init.tag`, or `"default-sync"` when omitted. Fired events use the registered tag. |
| `lastChance` | From `init.lastChance` boolean; fired events use `false`. |
| `bubbles` / `cancelable` / `isTrusted` / `timeStamp` | Own data properties (`bubbles` false, `cancelable` true; constructor `isTrusted` false; fired `isTrusted` true). |
| `waitUntil(promise)` | Requires an argument. A Promise increments a process pending counter until it settles (fulfill or reject); `MinimalRuntime` / `amber run` keep the script alive while `has_pending_wait_until()` is true. Non-promise arguments resolve an empty path without that keep-alive. |

`typeof SyncEvent === "function"`. `registration` is installed/updated on `globalThis` with a `sync` manager object exposing `register` and `getTags`.

## Limits

These limits are part of the Stable contract:

- **Same-isolate CLI fire only.** The `SyncEvent` runs on the page / CLI isolate that called `register`. It is **not** delivered into a service-worker isolate, and it does not require `navigator.serviceWorker.register`.
- **Not Periodic Background Sync.** There is no `periodicSync`, no interval, and no OS background task.
- **Not a network-offline scheduler.** Tags are not held until connectivity returns; fire is scheduled on the local event loop after `register` settles.
- **Process-local tag queue.** Queued tags live in a process `Mutex<Vec<String>>`. A new `MinimalRuntime` does not invent browser origin partitioning.
- Constructed / fired events are plain objects with `waitUntil`, not a browser `ExtendableEvent` prototype chain / `instanceof` parity.
- Rejects and `waitUntil` arity errors use ordinary `TypeError`, not `DOMException`.

## Non-goals

- Browser Background Sync (service-worker `sync` event, permission, replay after offline).
- Periodic Background Sync.
- Push, Notification, Payment, SharedWorker, or module workers.
- Rewriting G16 install/activate `waitUntil` ([`WORKERS_CONTRACT.md`](WORKERS_CONTRACT.md)).
- Graduating Cache or service-worker fetch intercept in this contract.

## Tests

```bash
cargo test --test background_sync_tests -- --test-threads=1
```

CI runs that command as `web background sync Stable contract`, next to the other Stable contract steps.

# Node `perf_hooks` contract

This is the user-facing contract for the Stable `perf_hooks` module slice in Amber (**tentative G74**). It is derived from `src/nodejs_core/performance.rs` (entry store + observer notification), `src/nodejs_core/perf_hooks.rs` (`PERF_OBSERVER_JS` shared snippet), `src/runtime_minimal.rs` (module bag wiring), and `tests/perf_hooks_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

The `performance` **object** (its `now` / `mark` / `measure` / `getEntries*` semantics) stays governed by [`docs/STREAMS_TIMERS_CONTRACT.md`](STREAMS_TIMERS_CONTRACT.md) (**G13**); this page owns only the `require('perf_hooks')` module surface layered on top — like G42 `require('timers')` layered over G13's global timers.

**Honesty:** `require('perf_hooks')` previously returned a bag containing only `{ performance }`; the `PerformanceObserver` class on the other install path was a stub whose `observe` / `takeRecords` / `disconnect` did nothing. Do **not** graduate a `PerformanceObserver` that never receives records, a `takeRecords()` that always returns `[]`, or a `timerify` that never records.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `require('perf_hooks')` / `require('node:perf_hooks')` | Returns the module bag `{ performance, PerformanceObserver }`. Both specifiers return the same object (`globalThis.perf_hooks`). |
| `perf_hooks.performance` | **Identical** to `globalThis.performance` (`===`). Every G13-pinned behavior (`now`, `timeOrigin`, `mark`, `measure`, `getEntries`, `getEntriesByName`, `getEntriesByType`, `clearMarks`, `clearMeasures`, `toJSON`) applies unchanged. |
| `performance.timerify(fn)` | Returns a wrapped function whose `name` is `"timerified <fn.name>"` (`"timerified anonymous"` for anonymous functions). Each call runs `fn` with the same `this`/args, records a `{ name: fn.name \|\| 'anonymous', entryType: 'function', startTime, duration }` entry **even when `fn` throws** (the error propagates), and returns `fn`'s result. A non-function argument throws `TypeError`. |
| `new PerformanceObserver(callback)` | `callback` must be a function — anything else throws `TypeError`. The observer is inert until `observe()` is called. |
| `observer.observe({ entryTypes })` | `entryTypes` must be a non-empty array whose members are all of `'mark'` / `'measure'` / `'function'` — a missing option, non-array, empty array, or unknown type string throws `TypeError`. Registers the observer; entries recorded afterwards buffer into the observer. |
| `observer.takeRecords()` | Synchronously drains the buffered records and returns them as an array of `{ name, entryType, startTime, duration }` objects. Records consumed this way are **not** delivered again to the callback. |
| `observer.disconnect()` | Unregisters the observer and clears its buffer; no further records or callbacks. |
| Observer callback | Invoked on the microtask queue (Node's async-notification model): `callback(list, observer)` where `list` exposes `getEntries()`, `getEntriesByType(type)`, and `getEntriesByName(name)` returning the buffered records, and `observer` is the observer itself. Records from multiple marks/measures/timerified calls in one turn batch into a single delivery. |

Wire proof required by this contract: `mark()`/`measure()`/timerified calls on `ph.performance` land in an observing `PerformanceObserver`'s buffer — retrievable both via `takeRecords()` and via the asynchronously delivered `list` — with real `entryType`/`startTime`/`duration` values, and `timerify`'s wrapper records on throw as well as on return.

## Limits

These are real behaviors of the Stable slice, not cover for missing machinery:

- Observer delivery rides the **microtask queue** (`queueMicrotask`), same as Node's "task after the current one" notification. It is not synchronous with `mark()`/`measure()`; observers needing the records immediately use `takeRecords()`.
- Entries and marks are **process-global** (`PERFORMANCE_STATE`), matching G13's pinned behavior — there is no per-context or per-observer timeline.
- `entryType` strings are exactly `'mark'`, `'measure'`, `'function'`. `getEntriesByType`/`observe({entryTypes})` recognize that set only.
- `measure`'s forgiving mark resolution stays the **G13 semantics**: a missing start-mark name resolves to `0`, a missing end-mark name to `now()` — it does not throw Node's `ERR_INVALID_PERFORMANCE_MARK`.
- `mark()` and `measure()` return `undefined` (G13-pinned), not the created entry objects.
- Entry objects are plain objects with `name`/`entryType`/`startTime`/`duration`; there is no `PerformanceEntry`/`PerformanceMark` prototype and no `detail` field on the delivered shape.
- Observer `entryTypes` matching is exact-string, case-sensitive; duplicates inside `entryTypes` are accepted and deduped on read.
- The callback's `list` is a minimal entry-list shape (`getEntries`/`getEntriesByType`/`getEntriesByName` only) — not Node's full `PerformanceObserverEntryList` prototype.
- A record dropped into a buffer whose observer never flushes stays buffered until `takeRecords()`/`disconnect()`.
- The `'function'` entry log is bounded: it retains at most 10_000 entries (oldest dropped) — `clearMarks`/`clearMeasures` never touch them and `setResourceTimingBufferSize` is a Non-goal, so this cap stands in for a long-running runtime's retention control. Marks and measures keep G13's unbounded retention.

## Non-goals

`monitorEventLoopDelay`, `createHistogram`, `eventLoopUtilization`, `histogram`, `constants`, `performance.eventLoopUtilization`, `performance.nodeTiming`, `performance.setResourceTimingBufferSize`, `performance.clearResourceTimings`, `performance.markResourceTiming`, `observe({ type, buffered })` single-entry-type mode, `Performance` constructor, and resource timing stay outside this contract. `require('performance')` as a module specifier is not pinned here.

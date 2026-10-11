# Node `async_hooks` narrow contract (`AsyncResource` + `AsyncLocalStorage`)

This is the user-facing contract for the Stable narrow subset of Node `async_hooks` in Amber (**tentative G57**). It is derived from `src/nodejs_core/async_hooks.rs`, CLI install via `setup_async_hooks_api` / `require('async_hooks')`, `tests/async_hooks_contract_tests.rs`, and conformance `tests/conformance/fixtures/async_hooks.js`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('async_hooks')` and `require('node:async_hooks')` reach the same object installed on `globalThis.async_hooks` by `setup_async_hooks_api` on the CLI path (`runtime_minimal`).

This is **not** full Node `async_hooks`. Only the calls below are Stable.

## Stable surface

### `AsyncResource`

| Call | Behavior |
| :--- | :--- |
| `new AsyncResource(type[, triggerAsyncId])` | Creates a resource. `type` defaults to `'UNKNOWN'` when omitted/falsy. Assigns a positive integer `asyncId` (monotonic counter starting at 1). `triggerAsyncId` defaults to `0`. |
| `asyncId()` | Returns the resource's assigned positive integer id. |
| `triggerAsyncId()` | Returns the constructor `triggerAsyncId` (number; default `0`). |
| `runInAsyncScope(fn[, thisArg, ...args])` | Invokes `fn.apply(thisArg, args)` synchronously and returns its result. Throws if `fn` throws. |
| `emitDestroy()` | Marks the resource destroyed; returns `this`. |
| `bind(fn[, type, thisArg])` | Returns a function that, when called, runs `fn` via `runInAsyncScope` with the bound `thisArg` (or the caller's `this`). |
| `AsyncResource.bind(fn[, type, thisArg])` | Static helper: constructs an `AsyncResource` (`type` or `fn.name` or `'bound'`) and returns its `bind` wrapper. |

### `AsyncLocalStorage`

| Call | Behavior |
| :--- | :--- |
| `new AsyncLocalStorage()` | Creates an enabled store instance (`getStore()` is `undefined` until `run` / `enterWith`). |
| `getStore()` | Returns the current store value while enabled; `undefined` when disabled or outside `run` / after restore. |
| `run(store, callback, ...args)` | Sets the store for the duration of `callback(...args)`, restores the previous store afterward. Nested `run` stacks correctly (inner finishes → outer store restored). If `callback` returns a thenable, the previous store is restored in `Promise.finally`. Sync throw restores the previous store and rethrows. |
| `exit(callback, ...args)` | Temporarily clears the store (`undefined`) for `callback(...args)`, then restores. Thenable / throw restore semantics match `run`. |
| `enterWith(store)` | Sets the store for the remainder of the current synchronous continuation while enabled (no auto-restore). |
| `disable()` | Clears the store, marks the instance disabled (`getStore()` → `undefined`), and removes it from `AsyncLocalStorage.snapshot` capture. |
| `AsyncLocalStorage.snapshot()` | Captures currently enabled instances and their stores; returns a function that restores those stores for a callback (thenable / throw restore like `run`). |

## Limits

- `runInAsyncScope` is a synchronous apply helper. It does **not** register or fire Node-style async lifecycle hooks.
- Store propagation for thenables uses `Promise.resolve(res).finally(...)`. This is not V8 `promiseHook` integration and is not claimed to cover every async boundary Node covers.
- `executionAsyncId`, `triggerAsyncId` (module-level), `executionAsyncResource`, and `createHook` may exist on the require object for shape compatibility, but they are **outside** this Stable contract (see Non-goals).

## Non-goals

These are outside Stable. Do **not** document them as Stable Limits fiction for missing or stub APIs:

- Real `createHook` async resource tracking (`init` / `before` / `after` / `destroy` / `promiseResolve` callbacks that fire across awaits, timers, I/O)
- V8 `promiseHook` integration / Node-parity async context propagation across every Promise and microtask edge
- Claiming module-level `executionAsyncId` / `triggerAsyncId` / `executionAsyncResource` as Stable (present values are not Node async-id tracking)
- Full Node `async_hooks` parity, `asyncWrapProviders`, or inspector async stacks product

## Reachability

The CLI `amber` binary installs this module on the CommonJS builtin path used by `runtime_minimal`. The contract is not feature-gated.

## Tests

```bash
cargo test --test async_hooks_contract_tests -- --test-threads=1
```

CI runs that command as `node async_hooks Stable contract`.

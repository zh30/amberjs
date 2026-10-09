# Web `ErrorEvent` contract

This is the user-facing contract for Stable `ErrorEvent` and the global `onerror` hook in the default Amber runtime. It is derived from `src/web_api/error_event.rs`, `src/runtime_minimal.rs`, `src/web_api/websocket.rs`, and `tests/error_event_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`ErrorEvent` is reachable from `amber run` / `amber eval` as `globalThis.ErrorEvent`. The WebSocket client (G15) builds error payloads with the same shape via `create_error_event_object`. This is not the HTML document error pipeline, and it is not a full browser `window` model.

**Provisional numbering:** This contract is **provisional G30** after `atob` / `btoa` (**G29**, [`ATOB_BTOA_CONTRACT.md`](ATOB_BTOA_CONTRACT.md)). Do not renumber G9–G16, G8 / G17–G28, or G29. Do not rewrite the atob/btoa contract here.

## `ErrorEvent` constructor

`typeof ErrorEvent === "function"`. `new ErrorEvent(type, init)` returns a plain object with ErrorEvent-shaped own properties.

| Field | Behavior |
| :--- | :--- |
| `type` | Always the string `"error"`. The first argument does not become `type`. |
| `message` | From `init.message` when it is a string. If the first argument is a string other than `"error"` and `init.message` is absent, that first argument is used as `message`. Otherwise `""`. |
| `filename` | From `init.filename` when it is a string; otherwise `""`. |
| `lineno` | From `init.lineno` when it is a number; otherwise `0`. |
| `colno` | From `init.colno` when it is a number; otherwise `0`. |
| `error` | From `init.error` when present and not `null` / `undefined`; otherwise `null`. |
| `bubbles` | `false`. |
| `cancelable` | `true`. |
| `composed` | `false`. |
| `defaultPrevented` | `false`. |
| `isTrusted` | `false`. |

These are ordinary own data properties. They are not getters, and they are not read-only.

## Global `onerror`

`globalThis.onerror` starts as a function that returns `false`. Assigning a function replaces it. `globalThis.window` is an alias of `globalThis` so `window.onerror` is the same binding.

When `MinimalRuntime` / `amber run` hits an uncaught exception, the runtime calls `onerror(message, filename, lineno, colno, error)` if it is a function. If that call returns `true`, the exception is treated as handled and is not re-thrown to the caller. Arguments: `message` is a string, `filename` / `lineno` / `colno` may be empty or zero for script throws, and `error` is the thrown value when available.

## Limits

These limits are part of the Stable contract:

- **Not `instanceof Event` or `instanceof ErrorEvent`.** Constructed objects are plain objects with the fields above. They do not carry `Event.prototype` methods (`preventDefault`, `stopPropagation`, …).
- **Not a document `window` error pipeline.** There is no bubbling through a DOM tree, no `error` event on `EventTarget` for script errors, and no resource-load error targeting.
- **`type` is fixed to `"error"`.** Passing another first-argument string does not create a differently typed event; it may only affect `message` as above.
- **Default `onerror` is a function, not `null`.**
- **Worker host `onerror` EventTarget parity** beyond the dedicated-worker surface already in [`WORKERS_CONTRACT.md`](WORKERS_CONTRACT.md) is outside this contract. WebSocket `onerror` payloads use this object's field shape (G15); listener semantics stay the WebSocket contract.

## Non-goals

- HTML living-standard `ErrorEvent` prototype chain and readonly IDL attributes.
- `reportError`, `unhandledrejection`, or Promise rejection tracking.
- Rewriting G29 `atob` / `btoa` ([`ATOB_BTOA_CONTRACT.md`](ATOB_BTOA_CONTRACT.md)).
- `ExtendableEvent`, Cache, Push, SharedWorker, or other Preview `src/web_api/` surfaces.

## Tests

```bash
cargo test --test error_event_contract_tests -- --test-threads=1
```

CI runs that command as `web error event Stable contract`, next to the other Stable contract steps.

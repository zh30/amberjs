# Node `timers/promises` contract

This is the user-facing contract for the Stable Node `require('timers/promises')` / `require('node:timers/promises')` surface in Amber (**G50**). It is derived from the CommonJS builtin install path in `src/runtime_minimal.rs` (JS Promise wrappers over `globalThis.setTimeout` / `clearTimeout`) and `tests/timers_promises_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

This page does **not** replace:

- G13 Web streams / global timers / `performance` ([`docs/STREAMS_TIMERS_CONTRACT.md`](STREAMS_TIMERS_CONTRACT.md))
- G42 Node `require('timers')` callback module bag ([`docs/TIMERS_MODULE_CONTRACT.md`](TIMERS_MODULE_CONTRACT.md))

This is not full Node `timers/promises`.

## Stable surface

| Export | Behavior |
| :--- | :--- |
| `setTimeout(delay = 0, value, options)` | Returns a `Promise` that resolves to `value` after `delay` milliseconds via `globalThis.setTimeout`. Zero delay does not resolve inside the current sync turn (microtasks run first). |
| `setImmediate(value, options)` | Equivalent to `setTimeout(0, value, options)` — returns a `Promise` that resolves to `value` on a later timer turn. |
| `default` | Object `{ setTimeout, setImmediate }` whose members are the same functions as the named exports of that require result. |

`require('timers/promises')` and `require('node:timers/promises')` both expose the same Stable export names. Each successful `require` builds a fresh export object (function identity is **not** shared across require calls, and is **not** the G42 `require('timers').setTimeout` callback identity).

### AbortSignal

When `options` is present and `options.signal` is an `AbortSignal`:

- If `signal.aborted` is already true at call time, the Promise rejects immediately with `signal.reason`, or with `Error('The operation was aborted')` when `reason` is absent / falsy.
- Otherwise an `abort` listener clears the underlying timer and rejects with the same reason rule.

`options` fields other than `signal` are ignored.

## Limits

- Scheduling uses today's installed `globalThis.setTimeout` / `clearTimeout` (same event-loop drain as G13 / G42).
- Require results are not cached; do not pin cross-require function identity.
- Abort rejection uses ordinary `Error` (or the caller-supplied `reason`), not a Node `AbortError` / `DOMException` code pin.

## Non-goals

These are outside Stable. Do **not** document them as Stable Limits fiction for missing APIs:

- `scheduler.wait` / `scheduler.yield` (or any `scheduler` export)
- Promise `setInterval` / async-iterable interval helpers
- Full Node options bags beyond `AbortSignal` (`ref` / `unref` product behavior, and other unused options fields)
- Graduating under G13 or G42 contract pages
- Claiming `timers.promises` as a property of the G42 `require('timers')` object

## Reachability

The CLI `amber` binary resolves `timers/promises` / `node:timers/promises` through the CommonJS builtin path in `src/runtime_minimal.rs`. The wrappers call the global timers installed by `nodejs_core::timers::setup_timers_api`. The contract is not feature-gated.

## Tests

```bash
cargo test --test timers_promises_contract_tests -- --test-threads=1
```

CI runs that command as `node timers/promises Stable contract`.

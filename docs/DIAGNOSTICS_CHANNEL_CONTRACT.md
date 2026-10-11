# Node `diagnostics_channel` contract

This is the user-facing contract for the Stable subset of Node `diagnostics_channel` in Amber (**tentative G56**). It is derived from `src/nodejs_core/diagnostics_channel.rs`, CLI install via `setup_diagnostics_channel_api` / `require('diagnostics_channel')`, `tests/diagnostics_channel_contract_tests.rs`, and conformance `tests/conformance/fixtures/diagnostics_channel.js`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('diagnostics_channel')` and `require('node:diagnostics_channel')` reach the same object installed on `globalThis.diagnostics_channel`.

This is not full Node `diagnostics_channel`. Only the calls below are Stable.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `channel(name)` | Returns a `Channel` for a string or symbol `name`. The same name returns the same channel object within this isolate. A non-string / non-symbol name throws `TypeError` (`Channel name must be a string or symbol`). |
| `channel.hasSubscribers` | `true` when at least one subscriber is registered; otherwise `false`. |
| `channel.subscribe(fn)` | Registers `fn` if it is a function and not already registered. Non-functions are ignored. |
| `channel.unsubscribe(fn)` | Removes a previously registered `fn`. |
| `channel.publish(message)` | Invokes each subscriber synchronously as `fn(message, channel.name)`. Subscriber throws are deferred (`process.nextTick` when available, else `setTimeout(..., 0)`). |
| `hasSubscribers(name)` | Module helper: `true` when `channel(name)` currently has subscribers. |
| `subscribe(name, fn)` / `unsubscribe(name, fn)` | Module helpers that forward to `channel(name).subscribe` / `.unsubscribe`. |
| `tracingChannel(name)` | With a string name, returns a `TracingChannel` whose `start` / `end` / `error` are `channel(\`${name}:start\`)` / `:end` / `:error`. |
| `tracingChannel.traceSync(fn, context, thisArg, ...args)` | Publishes `context` on `start`, runs `fn.apply(thisArg, args)`, stores `context.result` on success, publishes `context` on `error` then rethrows on failure, and always publishes `context` on `end` in `finally`. Returns the sync result. |

Conformance pin: `tests/conformance/fixtures/diagnostics_channel.js` (PASS on tip) covers channel pub/sub and `tracingChannel.traceSync` returning a sync sum.

## Limits

- Channel registry and subscribers live in the **current isolate** only (the JS Map created by the bootstrap script for that context).
- `publish` delivers on the calling turn; there is no async fan-out queue beyond deferred subscriber errors.
- `tracingChannel(name)` also wires `asyncStart` / `asyncEnd` channel names in the bootstrap; those names are not part of this Stable pin beyond existing for the object shape.

## Non-goals

These are outside Stable. Do **not** document them as Stable Limits fiction:

- **Process-wide / cross-isolate tracing product** (subscribers do not span Worker / service-worker / other isolates; there is no process-wide diagnostics bus)
- Claiming Node inspector / OpenTelemetry / APM integration as this page
- Stable promises for `tracePromise` / `traceCallback` (may exist in bootstrap code; not this contract)
- Graduating `async_hooks` (separate surface; inventory G57)
- Full Node `diagnostics_channel` parity (every optional TracingChannel option bag, store, bindStore, …)

## Reachability

`nodejs_core::diagnostics_channel::setup_diagnostics_channel_api` installs `globalThis.diagnostics_channel` on the CLI path (`runtime_minimal`). `require('diagnostics_channel')` / `require('node:diagnostics_channel')` resolve that object. The contract is not feature-gated.

## Tests

```bash
cargo test --test diagnostics_channel_contract_tests -- --test-threads=1
```

CI runs that command as `node diagnostics_channel Stable contract`.

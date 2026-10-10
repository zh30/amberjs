# Node `timers` module contract

This is the user-facing contract for the Stable Node `require('timers')` / `require('node:timers')` surface in Amber (**G42**). It is derived from `src/nodejs_core/timers.rs` (`setup_timers_api`), the CLI CommonJS builtin path in `src/runtime_minimal.rs`, and `tests/timers_module_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('timers')` and `require('node:timers')` return the **same** object installed on `globalThis.timers`. Each Stable export is the **same function identity** as the matching `globalThis` timer API installed by the same `setup_timers_api` call.

This page does **not** replace or expand the G13 Web streams / global timers / `performance` contract in [`docs/STREAMS_TIMERS_CONTRACT.md`](STREAMS_TIMERS_CONTRACT.md). G13 pins global `setTimeout` / `setInterval` / `clearTimeout` / `clearInterval` / `queueMicrotask` (and streams / compression / `performance`). This page graduates the **Node module bag** plus `setImmediate` / `clearImmediate` as module exports.

This is not full Node `timers`.

## Stable surface

| Export | Behavior |
| :--- | :--- |
| `setTimeout` | Same function as `globalThis.setTimeout`. Non-function callback throws `TypeError` (`callback must be a function`). Returns a timer object with numeric `_timerId`. Extra arguments after the delay are passed to the callback. `setTimeout(fn, 0)` does not run inside the current turn (microtasks run first). |
| `clearTimeout` | Same function as `globalThis.clearTimeout` (and as `clearInterval` / `clearImmediate`). Accepts the timer object or a numeric id. |
| `setInterval` | Same function as `globalThis.setInterval`. Non-function callback throws `TypeError`. Returns a timer object with `_timerId`. A requested delay of `0` is scheduled as `1` millisecond. Repeats until cleared. |
| `clearInterval` | Same clear function as `clearTimeout`. |
| `setImmediate` | Same function as `globalThis.setImmediate`. Non-function callback throws `TypeError` (`setImmediate: callback must be a function`). Returns a timer object with `_timerId`. Schedules the callback for the next event-loop immediate phase (not the current sync turn). |
| `clearImmediate` | Same clear function as `clearTimeout`. |

Identity pins this contract claims:

- `require('timers') === require('node:timers')` (or equivalent shared export object)
- `require('timers').setTimeout === globalThis.setTimeout` (and likewise for the other five Stable exports)

## Limits

- Scheduling behavior matches today's installed globals (interval `0` → `1ms`; zero-delay `setTimeout` is not same-turn; returned handles carry `_timerId` and clear-by-object-or-number).
- Not full Node `timers` and not browser `Window` timers.
- Global `ref` / `unref` helpers and `queueMicrotask` stay installed on `globalThis` but are **not** Stable exports of this module object.

## Non-goals

- Expanding or renumbering **G13** / claiming the module under [`docs/STREAMS_TIMERS_CONTRACT.md`](STREAMS_TIMERS_CONTRACT.md)
- `require('timers/promises')` / `timers.promises` as Stable under this G#
- Legacy `active` / `enroll` / `unenroll`
- Exporting `Timeout` / `Immediate` constructors as Node classes
- Claiming `queueMicrotask` as a `timers` module export
- Using `src/web_api/timers.rs` as the CLI install path

## Reachability

The CLI `amber` binary installs `globalThis.timers` from `src/runtime_minimal.rs` via `nodejs_core::timers::setup_timers_api`. `require('timers')` / `require('node:timers')` resolve that object through the CommonJS builtin path. Library users reach the installer through `amberjs::nodejs_core::timers::setup_timers_api`. The contract is not feature-gated.

## Tests

```bash
cargo test --test timers_module_contract_tests -- --test-threads=1
```

CI runs that command as `node timers Stable contract`.

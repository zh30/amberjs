# Amber test262 scorecard

- Generated: 2026-10-22T07:54Z
- test262 pin: `d53ebc8b1931d9f1541e43d59650776e4b766248`
- Amber: `./target/release/amber`
- Filter: `built-ins`
- Elapsed: 197.7s

| Status | Count |
|--------|------:|
| PASS | 34030 |
| FAIL | 524 |
| SKIP | 12787 |
| TOTAL | 47341 |

**Gate: FAIL** (PASS=34030 FAIL=524 SKIP=12787; filtered path=built-ins (not full-suite gate))

## Sample failures

| Test | Mode | Reason |
|------|------|--------|
| `built-ins/Array/prototype/concat/arg-length-near-integer-limit.js` | non-strict | Error: [Amber Error] UNKNOWN_ERROR: [].concat(spreadableHasPoisonedIndex) throws a Test262Error exception Expected a Tes |
| `built-ins/Array/prototype/concat/arg-length-near-integer-limit.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: [].concat(spreadableHasPoisonedIndex) throws a Test262Error exception Expected a Tes |
| `built-ins/Atomics/notify/bigint/notify-all-on-loc.js` | non-strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/bigint/notify-all-on-loc.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/count-defaults-to-infinity-missing.js` | non-strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/count-defaults-to-infinity-missing.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/count-defaults-to-infinity-undefined.js` | non-strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/count-defaults-to-infinity-undefined.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/negative-count.js` | non-strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/negative-count.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/notify-all-on-loc.js` | non-strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/notify-all-on-loc.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/notify-all.js` | non-strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/notify-all.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/notify-in-order-one-time.js` | non-strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/notify-in-order-one-time.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/notify-in-order.js` | non-strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/notify-in-order.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/notify-nan.js` | non-strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/notify-nan.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/notify-one.js` | non-strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/notify-one.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/notify-renotify-noop.js` | non-strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/notify-renotify-noop.js` | strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |
| `built-ins/Atomics/notify/notify-two.js` | non-strict | Error: [Amber Error] UNKNOWN_ERROR: $262.agent is not implemented on Amber yet |

## Sample skips

| Test | Mode | Reason |
|------|------|--------|
| `built-ins/AbstractModuleSource/length.js` | module | unsupported-feature:source-phase-imports |
| `built-ins/AbstractModuleSource/name.js` | module | unsupported-feature:source-phase-imports |
| `built-ins/AbstractModuleSource/proto.js` | module | unsupported-feature:source-phase-imports |
| `built-ins/AbstractModuleSource/prototype.js` | module | unsupported-feature:source-phase-imports |
| `built-ins/AbstractModuleSource/prototype/Symbol.toStringTag.js` | module | unsupported-feature:source-phase-imports |
| `built-ins/AbstractModuleSource/prototype/constructor.js` | module | unsupported-feature:source-phase-imports |
| `built-ins/AbstractModuleSource/prototype/proto.js` | module | unsupported-feature:source-phase-imports |
| `built-ins/AbstractModuleSource/throw-from-constructor.js` | module | unsupported-feature:source-phase-imports |
| `built-ins/Array/prototype/at/coerced-index-resize.js` | non-strict | unsupported-feature:resizable-arraybuffer |
| `built-ins/Array/prototype/at/coerced-index-resize.js` | strict | unsupported-feature:resizable-arraybuffer |
| `built-ins/Array/prototype/at/typed-array-resizable-buffer.js` | non-strict | unsupported-feature:resizable-arraybuffer |
| `built-ins/Array/prototype/at/typed-array-resizable-buffer.js` | strict | unsupported-feature:resizable-arraybuffer |
| `built-ins/Array/prototype/copyWithin/resizable-buffer.js` | non-strict | unsupported-feature:resizable-arraybuffer |
| `built-ins/Array/prototype/copyWithin/resizable-buffer.js` | strict | unsupported-feature:resizable-arraybuffer |
| `built-ins/Array/prototype/entries/resizable-buffer-grow-mid-iteration.js` | non-strict | unsupported-feature:resizable-arraybuffer |

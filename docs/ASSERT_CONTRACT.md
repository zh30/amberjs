# Node `assert` contract

This is the user-facing contract for a **narrow** Stable subset of Node `assert` in Amber. It is derived from `src/nodejs_core/assert.rs` (`setup_assert_api`), `src/runtime_minimal.rs` (`install_core_apis` → `setup_assert_api`; CJS `require('assert')` / `require('assert/strict')` / `require('node:assert')` / `require('node:assert/strict')`), and `tests/assert_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

This is **not** full Node `assert`. Methods that are not listed under Stable surface are outside the contract.

**Numbering:** This contract is **G40** after Node `querystring` (**G38**) and Node `string_decoder` (**G39**). Do not assign G38/G39 to assert.

## Stable surface

| Call / field | Behavior |
| :--- | :--- |
| `require('assert')` / `require('node:assert')` | Same callable function object installed by `setup_assert_api` on `globalThis.assert`. |
| `require('assert/strict')` / `require('node:assert/strict')` | Same object as `require('assert')` today (not a separate strict-only module). |
| `assert(value[, message])` (callable) | Same as `assert.ok`. Throws when `value` is not truthy. |
| `assert.ok(value[, message])` | Throws an Error when `value` is falsy: `null`, `undefined`, `false`, `0`, `NaN`, or `""`. Other values pass. Optional `message` becomes the Error message (default `"Assertion failed"`). |
| `assert.strictEqual(actual, expected)` | Passes when `actual` and `expected` are strictly equal (`===`). On failure throws an Error whose message contains `strictly equal` and string forms of both sides. |
| `assert.deepStrictEqual(actual, expected)` | Passes when values are strictly equal, or when `JSON.stringify` of both sides is strictly equal (plain objects / arrays). On failure throws an Error whose message contains `deeply equal`. |
| `assert.throws(fn)` | `fn` must be a function. Invokes `fn` with no arguments; passes if the call throws. If `fn` does not throw, throws an Error whose message contains `Missing expected exception`. A non-function throws with message containing `requires a function`. |
| `assert.fail([message])` | Always throws. Optional `message` (stringified) is the Error message; default `"Failed"`. |
| `assert.ifError(err)` | Passes when `err` is `null` or `undefined`. Otherwise rethrows `err` as the exception value (does not wrap it). |

## Limits

These are real behaviors of the Stable surface, not cover for missing Node APIs:

- `assert/strict` is the **same object** as `assert` (including non-strict-named helpers that exist on the object). There is no separate strict-only export.
- `deepStrictEqual` compares via `JSON.stringify` when values are not `===`. Property order, `undefined` object values, `Date`, `Map` / `Set`, circular structures, and other non-JSON-faithful values are not Node deep-equal parity.
- `throws` only checks that a function threw. Error class / RegExp / validator second arguments are not pinned.
- Failure messages are Amber-shaped strings (often with `AssertionError [ERR_ASSERTION]` text). They are not a full Node `AssertionError` class with `actual` / `expected` / `operator` / `code` fields.
- ESM `import` from `'assert'` / `'node:assert'` is not wired in `normalized_esm_builtin_name` today — only CJS `require` is on this contract.

## Non-goals

Outside this contract (Preview / unimplemented). Do **not** treat these as Stable Limits of dishonest behavior:

- **`assert.match` / `assert.doesNotMatch`** — not installed.
- **`assert.rejects` / `assert.doesNotReject`** — not installed; no async assertion surface.
- **`assert.equal` coercion quirks** — present on the object but **not Stable**. Today's implementation is coercion-shaped (number epsilon / stringification), not Node `==` parity. Do not graduate it under Limits fiction.
- **`assert.deepEqual` alias** — today aliases the same function as `deepStrictEqual`. That alias is **not Stable**; only `deepStrictEqual` is pinned.
- `assert.doesNotThrow`, `assert.notEqual`, `assert.notStrictEqual`, `assert.notDeepEqual`, `assert.notDeepStrictEqual`, `partialDeepStrictEqual`, and the rest of Node `assert`.
- Promoting `equal` / `deepEqual` by listing incomplete coercion or alias behavior as Stable Limits.

## Reachability

The CLI `amber` binary installs `globalThis.assert` from `src/runtime_minimal.rs` via `nodejs_core::assert::setup_assert_api`. `require('assert')`, `require('assert/strict')`, `require('node:assert')`, and `require('node:assert/strict')` return that object through the CommonJS builtin arm. Library users reach the installer through `amberjs::nodejs_core::assert::setup_assert_api` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.

## Tests

```bash
cargo test --test assert_contract_tests -- --test-threads=1
```

CI step: `node assert Stable contract`.

## Honesty rule

`equal` coercion quirks and the `deepEqual` → `deepStrictEqual` alias are **Non-goals**. Limits describe JSON deep-compare and same-object `assert/strict` only. Graduating full Node `assert` as a bag is forbidden.

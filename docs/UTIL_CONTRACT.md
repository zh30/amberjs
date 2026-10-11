# Node `util` contract

This is the user-facing contract for the Stable basics subset of Node `util` in Amber. It is derived from `src/nodejs_core/util.rs` (Rust install plus the JS overlay that runs at the end of `setup_util_api`), `src/runtime_minimal.rs` (`require('util')` / `require('node:util')` return `globalThis.util`), and `tests/util_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('util')` and `require('node:util')` reach the same object as `globalThis.util`.

This is not full Node `util`. Methods that are not listed here are outside the contract. Additive **G59** pins real V8 `util.types.isArrayBuffer` and `util.types.isRegExp` (the previous always-`false` Rust stubs are gone).

The rest of `src/nodejs_core/` stays Preview except surfaces with their own Stable contracts (for example the Node `fs` contract and the tiny `process` basics contract).

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `format(fmt, ...args)` | Substitutes `%s`, `%d`, `%i`, and `%f` from following arguments (string / number / boolean / null / undefined stringified; other values become `"[Object]"`). `%%` becomes a single `%`. `%j` appends the literal `"[Object]"` and consumes one argument. Remaining arguments after the format string are appended, separated by spaces. |
| `inspect(value)` | Returns a string. `null` → `"null"`, `undefined` → `"undefined"`, strings are single-quoted, numbers and booleans are their usual string forms, arrays are `"Array(n)"` (elements are not expanded), plain objects are `"{ k: v, ... }"` from own enumerable keys (string values quoted; nested objects are not deeply inspected). Options (`depth`, `colors`, `showHidden`, and the rest) are ignored when present. |
| `promisify(fn)` | Returns a function that returns a `Promise`. The wrapped call appends a Node-style `(err, ...values)` callback. Rejection uses `err`. Resolution uses `values[0]` when there is at most one value, otherwise the `values` array. A non-function throws `TypeError` whose message contains `must be of type function`. |
| `promisify.custom` | The symbol `Symbol.for('nodejs.util.promisify.custom')`. When `fn[promisify.custom]` is set, `promisify(fn)` returns that value. |
| `callbackify(asyncFn)` | Returns a function whose last argument must be a callback `(err, value)`. Settling the promise calls `cb(null, ret)` or `cb(rej)`. A non-function original, or a call whose last argument is not a function, throws `TypeError`. |
| `inherits(ctor, superCtor)` | Sets `ctor.super_ = superCtor` and links `ctor.prototype` to `superCtor.prototype` (`Object.create` when `ctor.prototype` is missing, otherwise `Object.setPrototypeOf`). Empty or missing constructors, or a super without `prototype`, throw `TypeError`. |
| `isArray` / `isBoolean` / `isNull` / `isNumber` / `isString` / `isUndefined` / `isObject` / `isFunction` | Legacy predicates. `isObject` is true for objects that are not `null`. `isUndefined` is the camelCase helper from the JS overlay. |

### `util.types` (Stable members only)

| Call | Behavior |
| :--- | :--- |
| `isDate` | `true` for `Date` instances. |
| `isMap` / `isSet` / `isWeakMap` / `isWeakSet` | `instanceof` those constructors. |
| `isPromise` | `instanceof Promise`, or a thenable with a `then` function. |
| `isTypedArray` | `ArrayBuffer.isView(v) && !(v instanceof DataView)`. |
| `isAnyArrayBuffer` | `ArrayBuffer` or `SharedArrayBuffer` when that global exists. |
| `isArrayBuffer` | `true` when the value is a V8 `ArrayBuffer` (`Value::is_array_buffer`). `false` for `SharedArrayBuffer`, TypedArray / `DataView` views, plain objects, and primitives. Omitted argument is `false`. |
| `isRegExp` | `true` when the value is a V8 `RegExp` (`Value::is_reg_exp`). `false` for RegExp-like plain objects, strings, and other types. Omitted argument is `false`. |
| `isDataView` | `instanceof DataView`. |
| `isNativeError` | `instanceof Error`. |
| `isArgumentsObject` | `Object.prototype.toString` is `[object Arguments]`. |
| `isBooleanObject` / `isNumberObject` / `isStringObject` / `isSymbolObject` | Boxed primitives via `toString` tag / `instanceof` as implemented in the overlay. |
| `isBoxedPrimitive` | Boxed `Number` / `String` / `Boolean` / `Symbol` / `BigInt`. |
| `isAsyncFunction` / `isGeneratorFunction` | Constructor name `AsyncFunction` / `GeneratorFunction`. |

## Limits

These are real behaviors of the Stable surface, not cover for missing APIs:

- `isArrayBuffer` does **not** treat `SharedArrayBuffer` as an `ArrayBuffer` (use `isAnyArrayBuffer`).
- `isArrayBuffer` / `isRegExp` use V8 type tags, not duck-typing (`{ buffer: … }` / `{ test() {} }` stay `false`).
- Shallow `inspect` does not honor `depth`, colors, or custom inspect symbols.

## Non-goals

- Full Node `util` (`parseArgs`, `styleText`, `transferableAbortController`, `TextEncoder` / `TextDecoder` re-exports, and the rest).
- Deep `inspect` with `depth`, custom inspect symbols, or colored output.
- `deprecate` and `debuglog` (present for experiments; not pinned here).
- Matching every Node format specifier or inspect edge case.
- Every other `util.types.*` helper beyond the Stable members table above.

## Reachability

The CLI `amber` binary installs `globalThis.util` from `src/runtime_minimal.rs` via `nodejs_core::util::setup_util_api`. Library users reach the same setup through `amberjs::nodejs_core` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.

## Tests

```bash
cargo test --test util_contract_tests -- --test-threads=1
```

CI runs that command as `node util Stable contract`.

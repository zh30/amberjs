# Node `querystring` contract

This is the user-facing contract for the Stable subset of Node `querystring` in Amber. It is derived from `src/nodejs_core/querystring.rs`, CLI install via `setup_querystring_api` / `require('querystring')`, `tests/querystring_contract_tests.rs`, and conformance `tests/conformance/fixtures/querystring_basics.js`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('querystring')` and `require('node:querystring')` reach the same object installed by `setup_querystring_api`.

This is not full Node `querystring`. Only the calls below are Stable.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `parse(str)` | Splits `str` on `&` / `=` pairs. Keys and values are percent-decoded (`%XX` and `+` → space). Repeated keys become a string array (order preserved). Empty values are `""`. Missing / non-string input is coerced via the install path’s string default. |
| `stringify(obj)` | Own enumerable string keys of a plain object become `key=value` pairs joined by `&`. Array values emit one pair per element (same key repeated). Values are percent-encoded (spaces → `%20`, reserved bytes encoded). Booleans stringify as `true` / `false`. |
| `escape(str)` | Percent-encodes a string for query use (spaces → `%20`). |
| `unescape(str)` | Percent-decodes `%XX` sequences only. A literal `+` stays `+` (unlike `parse`, which maps `+` → space). |

## Limits

- Separators are fixed `&` / `=` on this Stable pin. The install path may accept extra positional `sep` / `eq` arguments, but they are **not** Stable (do not rely on them; do not invent an options bag).
- Nested object / `qs` library shapes are not produced by `parse` and not claimed by `stringify`.
- `maxKeys` and decode/encode option functions are outside this contract.
- `unescape` is not a full `parse`-component decoder: no `+` → space.

## Non-goals

- Full Node `querystring` / `URLSearchParams` parity.
- Claiming `maxKeys`, nested `qs` objects, or decode/encode option bags as Stable Limits fiction.
- `querystring.parse(str, sep, eq, options)` options-object bags.
- Graduating other `src/nodejs_core/` modules.

## Reachability

The CLI `amber` binary installs this module on the CommonJS builtin path used by `runtime_minimal`. The contract is not feature-gated. Pinning tests: `tests/querystring_contract_tests.rs` and CI step `node querystring Stable contract`.

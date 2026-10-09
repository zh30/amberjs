# Web `atob` / `btoa` contract

This is the user-facing contract for Stable `atob` and `btoa` in the default Amber runtime. It is derived from `src/web_api/encoding.rs` (`atob_callback`, `btoa_callback`, installed by `setup_encoding_api`) and `tests/atob_btoa_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

These globals are reachable from `amber run` / `amber eval` through `src/runtime_minimal.rs`. `URL`, `URLSearchParams`, `TextEncoder`, `TextDecoder`, and `structuredClone` stay the G10 contract. Every other `src/web_api/` module stays on its own Stable or Preview surface. This is not the full HTML Living Standard `WindowOrWorkerGlobalScope` base64 algorithm.

**Numbering:** This contract is **G29** after CommonJS `require` (**provisional G28**). G9–G16 (web) and G8 / G17–G28 (`nodejs_core`) stay intact. Stream stays **G24**.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `btoa(data)` | Converts `data` with `ToString` (except when the argument is omitted or `undefined`; see below). Every UTF-16 code unit must be a Latin-1 byte (`code point ≤ 255`). Those bytes are encoded with the standard Base64 alphabet and padding (`=`). Returns the Base64 string. Empty string encodes to `""`. |
| `atob(data)` | Converts `data` with `ToString` (except when the argument is omitted or `undefined`). Decodes standard Base64 (alphabet `A–Za–z0–9+/`, `=` padding) into bytes and returns a string where each byte is a Latin-1 character (`String.fromCharCode(byte)`). Empty string decodes to `""`. |

Round-trip: for any Latin-1 string `s`, `atob(btoa(s)) === s`.

## Errors

| Condition | Result |
| :--- | :--- |
| `btoa()` or `btoa(undefined)` | Throws `Error` whose message is `btoa: input is required`. |
| `atob()` or `atob(undefined)` | Throws `Error` whose message is `atob: input is required`. |
| `btoa` string with a code point `> 255` | Throws `Error` whose message contains `Latin-1`. |
| `atob` string that is not valid standard Base64 | Throws `Error` whose message contains `invalid base64`. |

Other values (`null`, numbers, objects) are stringified with `ToString` before encode/decode (`btoa(null)` encodes `"null"`; `btoa(123)` encodes `"123"`). `atob(null)` therefore decodes the Base64 string `"null"` (three Latin-1 bytes), and does not throw.

## Limits

- Errors are ordinary `Error` objects, not `DOMException` / `InvalidCharacterError`.
- Decode is strict standard Base64. Whitespace inside or after the payload is not stripped (HTML ForgivingBase64 is not implemented). Missing padding that the `base64` crate rejects fails as invalid Base64.
- An omitted or `undefined` argument throws instead of encoding/decoding the string `"undefined"`.
- Not a general binary codec API: there is no `Uint8Array` overload; binary data is carried as Latin-1 strings only.

## Non-goals

- HTML ForgivingBase64 whitespace and padding rules.
- `DOMException` error types.
- Base64url (`-` / `_`) alphabets.
- Graduating other encoding helpers beyond G10 and this page.

## Tests

```bash
cargo test --test atob_btoa_contract_tests -- --test-threads=1
```

CI runs that command as `web atob btoa Stable contract`, next to the other Stable contract steps.

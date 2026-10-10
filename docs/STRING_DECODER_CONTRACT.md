# Node `string_decoder` contract

This is the user-facing contract for the Stable UTF-8 `StringDecoder` subset of Node `string_decoder` in Amber. It is derived from `src/nodejs_core/string_decoder.rs` (`setup_string_decoder_api`), `src/runtime_minimal.rs` (`install_core_apis` and the CommonJS builtin arm for `string_decoder`), and `tests/string_decoder_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('string_decoder')`, `require('node:string_decoder')`, and `globalThis.string_decoder` reach the same object installed by `setup_string_decoder_api`. The object exposes `StringDecoder` and `default` (the same constructor).

This is **not** full Node `string_decoder`. Stable behavior is UTF-8 decode with incomplete multibyte buffering on `write` / `end`, plus string passthrough and Buffer / `Uint8Array` byte paths. Non-UTF-8 encodings are **not claimed**.

**Provisional numbering:** This contract is **G39** after Node `querystring` **G38**. Do not take G38.

## Stable surface

| Call / field | Behavior |
| :--- | :--- |
| `require('string_decoder')` / `require('node:string_decoder')` | Same object as `globalThis.string_decoder` / `globalThis.__string_decoder`. |
| `string_decoder.StringDecoder` / `string_decoder.default` | Constructor. `default === StringDecoder`. |
| `new StringDecoder([encoding])` | Default encoding is `'utf8'`. The string `'utf-8'` is normalized to `'utf8'`. Other encodings may be stored on the instance but are **outside** this Stable pin (see Limits / Non-goals). |
| `decoder.write(buffer)` | Decodes bytes and returns a string. A **string** argument is returned as-is (passthrough). `Uint8Array`, Buffer-like views (`buffer.buffer` is an `ArrayBuffer`), and plain byte arrays are accepted. Incomplete trailing UTF-8 sequences are held and not emitted until a later `write` or `end` completes them. |
| `decoder.end([buffer])` | Optional final `write(buffer)`, then flushes any buffered incomplete UTF-8 bytes through the decoder and clears the hold buffer. Returns the concatenated string. |

## Limits

These are real behaviors of the Stable UTF-8 surface:

- **UTF-8 primary.** Incomplete multibyte hold/flush is implemented for `encoding === 'utf8'` only. That is the contracted path (default constructor and `'utf8'` / `'utf-8'`).
- **Buffer / `Uint8Array` path.** Byte inputs are copied into a JS number array before decode. Views with an `ArrayBuffer` use `byteOffset` / `byteLength` when present.
- **String passthrough.** `write(string)` returns the string without re-encoding.
- Unsupported input shapes (neither string, `Uint8Array`, ArrayBuffer view, nor array) yield `''` from `write` (no throw).
- Empty byte input yields `''`.
- The decoder uses the host `TextDecoder` for the final UTF-8 decode step on complete byte runs.

## Non-goals

Outside this contract (Preview / unimplemented). Do **not** treat these as Stable Limits fiction:

- Non-UTF-8 Node `StringDecoder` encodings (`base64`, `hex`, `utf16le`, `ucs2`, `latin1` / `binary` / `ascii`, …) as a Stable promise.
- Full Node `string_decoder` parity (fill / leftover APIs beyond `write` / `end`, encoding option bags, streaming edge cases not listed above).
- Graduating other Preview builtins by loading this name through `require` (G28 loads the name; this page graduates only the UTF-8 decoder surface).

## Reachability

The CLI `amber` binary installs `globalThis.string_decoder` from `src/runtime_minimal.rs` via `nodejs_core::string_decoder::setup_string_decoder_api`. `require('string_decoder')` / `require('node:string_decoder')` return that object through the CommonJS builtin arm. Library users reach the installer through `amberjs::nodejs_core::string_decoder::setup_string_decoder_api` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.

## Tests

```bash
cargo test --test string_decoder_contract_tests -- --test-threads=1
```

CI step: `node string_decoder Stable contract`.

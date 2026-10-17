# Node `string_decoder` contract

This is the user-facing contract for the Stable `StringDecoder` subset of Node `string_decoder` in Amber. It is derived from `src/nodejs_core/string_decoder.rs` (`setup_string_decoder_api`), `src/runtime_minimal.rs` (`install_core_apis` and the CommonJS builtin arm for `string_decoder`), and `tests/string_decoder_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('string_decoder')`, `require('node:string_decoder')`, and `globalThis.string_decoder` reach the same object installed by `setup_string_decoder_api`. The object exposes `StringDecoder` and `default` (the same constructor).

This is **not** full Node `string_decoder`. Stable behavior is incremental `write` / `end` decode (or encode-to-string for `base64` / `hex`) with incomplete-unit buffering, plus string passthrough and Buffer / `Uint8Array` byte paths.

**Numbering:** UTF-8 `StringDecoder` graduated as **G39**. Additive multi-encoding support (`latin1` / `ascii` / `binary`, `base64`, `hex`, `utf16le` / `ucs2`) graduates as **G68**. Do not renumber G1–G67.

## Stable surface

| Call / field | Behavior |
| :--- | :--- |
| `require('string_decoder')` / `require('node:string_decoder')` | Same object as `globalThis.string_decoder` / `globalThis.__string_decoder`. |
| `string_decoder.StringDecoder` / `string_decoder.default` | Constructor. `default === StringDecoder`. |
| `new StringDecoder([encoding])` | Default encoding is `'utf8'`. Normalization: `'utf-8'` → `'utf8'`; `'ucs2'` / `'utf-16le'` → `'utf16le'`; `'binary'` → `'latin1'`. Supported encodings: `utf8`, `utf16le`, `latin1`, `ascii`, `base64`, `hex`. Unsupported encodings throw `TypeError` with `code === 'ERR_UNKNOWN_ENCODING'`. |
| `decoder.write(buffer)` | Decodes (or encodes-to-string) bytes and returns a string. A **string** argument is returned as-is (passthrough). `Uint8Array`, Buffer-like views (`buffer.buffer` is an `ArrayBuffer`), and plain byte arrays are accepted. Incomplete trailing units are held and not emitted until a later `write` or `end` completes them (per encoding). |
| `decoder.end([buffer])` | Optional final `write(buffer)`, then flushes held bytes per encoding rules and clears the hold buffer. Returns the concatenated string. |

### Encoding rules (G39 + G68)

| Encoding | Incomplete hold on `write` | `end` flush |
| :--- | :--- | :--- |
| `utf8` (**G39**) | Trailing incomplete multibyte UTF-8 sequence (1–3 lead/continuation bytes). | Decode held bytes through UTF-8. |
| `utf16le` / `ucs2` (**G68**) | Odd trailing byte; also a trailing high surrogate waiting for a low surrogate. | Emit complete code units (including a lone high surrogate); discard a lone odd trailing byte. |
| `base64` (**G68**) | Remainder when byte length mod 3 ≠ 0. | Encode remainder with `=` padding (`TQ==` / `TWE=` style). |
| `hex` (**G68**) | None (each byte → two hex chars). | N/A (nothing held). |
| `latin1` / `binary` (**G68**) | None (each byte → `String.fromCharCode(b & 0xff)`). | N/A. |
| `ascii` (**G68**) | None (each byte → `String.fromCharCode(b & 0x7f)`). | N/A. |

## Limits

These are real behaviors of the Stable surface:

- **Incomplete-unit buffering** is encoding-specific as in the table above. Hex / latin1 / ascii do not buffer across writes.
- **Buffer / `Uint8Array` path.** Byte inputs are copied into a JS number array before decode. Views with an `ArrayBuffer` use `byteOffset` / `byteLength` when present.
- **String passthrough.** `write(string)` returns the string without re-encoding.
- Unsupported input shapes (neither string, `Uint8Array`, ArrayBuffer view, nor array) yield `''` from `write` (no throw).
- Empty byte input yields `''`.
- UTF-8 complete runs use the host `TextDecoder('utf8')`. Other encodings use real in-module codecs (not stubs); they do **not** require `TextDecoder` support for `latin1` / `base64` / `hex` / `utf-16le`.
- `ascii` masks with `0x7f` (Node-shaped); `latin1` keeps the full byte.

## Non-goals

Outside this contract (Preview / unimplemented). Do **not** treat these as Stable Limits fiction:

- `base64url` and any encoding not listed in the Stable surface table.
- Full Node `string_decoder` parity (`fill` / leftover APIs beyond `write` / `end`, encoding option bags, every Node streaming edge case).
- Graduating other Preview builtins by loading this name through `require` (G28 loads the name; this page graduates only the decoder surface above).
- Fixing `Buffer.prototype.toString('utf16le')` host gaps (StringDecoder utf16le is self-contained).

## Reachability

The CLI `amber` binary installs `globalThis.string_decoder` from `src/runtime_minimal.rs` via `nodejs_core::string_decoder::setup_string_decoder_api`. `require('string_decoder')` / `require('node:string_decoder')` return that object through the CommonJS builtin arm. Library users reach the installer through `amberjs::nodejs_core::string_decoder::setup_string_decoder_api` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.

## Tests

```bash
cargo test --test string_decoder_contract_tests -- --test-threads=1
```

CI step: `node string_decoder Stable contract`.

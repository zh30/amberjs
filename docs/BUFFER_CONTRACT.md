# Node `buffer` / `Buffer` contract

This is the user-facing contract for the Stable Node `Buffer` surface in Amber. It is derived from `setup_buffer_module` in `src/runtime_minimal.rs` and `tests/buffer_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`globalThis.Buffer`, `require('buffer').Buffer`, and `require('node:buffer').Buffer` are the same constructor installed by `install_core_apis`. The `require('buffer')` object also exposes `default.Buffer` as that constructor.

The CLI path does **not** use `src/nodejs_core/buffer.rs`. That file is orphaned for the default `amber` binary and is not this contract.

This is not full Node `buffer`. Methods that are not listed here are outside the contract.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `Buffer.from(string[, encoding])` | Encodes the string. Default encoding is `utf8`. Supported encodings: `utf8` / `utf-8` / `utf8mb4`, `hex`, `base64`, `base64url`, `latin1` / `ascii` / `binary`. Unknown encodings fall back to UTF-8 bytes. Small UTF-8 strings may allocate from the pool (see Pool). |
| `Buffer.from(ArrayBuffer[, byteOffset[, length]])` | A `Uint8Array` view over that buffer (shared memory), with Buffer prototype. |
| `Buffer.from(TypedArray \| DataView)` | Copies the view's bytes into a new buffer. |
| `Buffer.from(array)` | Copies each element's low 8 bits. |
| `Buffer.from(number)` | Allocates that many bytes (not a Node string of that length). |
| `Buffer.alloc(size[, fill[, encoding]])` | Allocates `size` bytes. Omitted or `0` fill yields zeros. A number fill repeats that byte. A string fill uses `encoding` (default `utf8`) and tiles the encoded bytes. |
| `Buffer.allocUnsafe(size)` | Allocates `size` bytes. Sizes below `Buffer.poolSize >>> 1` may reuse the pool and are **not** zero-filled. Larger sizes are a fresh `ArrayBuffer`. |
| `Buffer.allocUnsafeSlow(size)` | Always a fresh allocation of `size` bytes (no pool). |
| `Buffer.concat(list[, totalLength])` | Copies list members (Buffer / TypedArray / ArrayBuffer) into one buffer. Omitted `totalLength` sums member lengths. A smaller `totalLength` truncates; a larger one zero-pads the tail. |
| `Buffer.byteLength(value[, encoding])` | For strings, the encoded byte length (same encodings as `from`). For `ArrayBuffer` / TypedArray, `byteLength`. |
| `Buffer.isBuffer(value)` | True when `value._isBuffer === true`, or when `value` is an object whose `.buffer` is an `ArrayBuffer`. Plain `Uint8Array` therefore counts as a buffer under this check. |
| `buf.length` | Byte length (`Uint8Array` length). |
| `buf[i]` | Byte at index `i` (TypedArray indexing). |
| `buf.toString([encoding[, start[, end]]])` | Decodes the slice `[start, end)`. Default encoding `utf8`. Same encoding names as `from`. Invalid UTF-8 uses lossy replacement. |
| `buf.write(string[, offset[, length]][, encoding])` | Writes encoded bytes at `offset` (default `0`). Optional numeric `length` caps bytes written. Returns the byte count written. |
| `buf.slice([start[, end]])` | Same as `buf.subarray`: a view over the same underlying `ArrayBuffer` (shared memory), not a copy. Negative indexes count from the end. |
| `buf.subarray([start[, end]])` | View over the same memory. |
| `Buffer.poolSize` | `8192`. |

Instances are `Uint8Array` views with `Buffer.prototype` (via an internal `FastBuffer` subclass). `buf instanceof Buffer` uses `Buffer.isBuffer`. `Buffer.prototype` sits on `Uint8Array.prototype`.

## Pool

`Buffer.poolSize` is `8192`. A shared `ArrayBuffer` of that size backs small `allocUnsafe` and small UTF-8 `Buffer.from(string)` allocations. The next offset is rounded up to a multiple of 8. When the remaining pool space is too small, a new pool is created.

Because pooled `allocUnsafe` reuses prior bytes, reading before writing is undefined. Do not treat `allocUnsafe` as zero-filled.

## Encodings

| Name | Encode (`from` / `write` / `alloc` fill) | Decode (`toString`) |
| :--- | :--- | :--- |
| `utf8`, `utf-8`, `utf8mb4` | UTF-8 bytes | UTF-8; invalid sequences are lossy |
| `hex` | `hex::decode`; on failure, UTF-8 bytes of the input string | lowercase hex |
| `base64` | standard Base64; on failure, UTF-8 bytes of the input | standard Base64 |
| `base64url` | URL-safe Base64 without padding | URL-safe Base64 without padding |
| `latin1`, `ascii`, `binary` | one byte per string code unit (truncated) | one char per byte |

## Non-goals

- Node `Buffer.compare`, `equals`, `includes`, `lastIndexOf`, `readUInt*`, `writeUInt*`, `swap*`, `toJSON` as a Stable pin.
- `Buffer.alloc` security guarantees beyond zero-fill for omitted/`0` fill (pooled `allocUnsafe` is intentionally uninitialized).
- The orphaned `src/nodejs_core/buffer.rs` / `nodejs_core::setup_nodejs_core_apis` Buffer path.
- Full Node.js `buffer` module parity (`SlowBuffer` as a separate export, `INSPECT_MAX_BYTES`, transcoder streams).

`copy`, `fill`, and `indexOf` exist on the prototype for other modules but are not part of this Stable pin.

## Reachability

The CLI `amber` binary installs this constructor from `src/runtime_minimal.rs` (`setup_buffer_module` inside `install_core_apis`). `require('buffer')` returns that global. The contract is not feature-gated.

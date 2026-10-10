# Node `zlib` async `gzip` / `gunzip` contract

This is the user-facing contract for the Stable async callback slice of Node `zlib` in Amber (**tentative G48**). It is derived from `src/nodejs_core/zlib.rs`, `src/runtime_minimal.rs`, and `tests/zlib_async_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

The six sync methods remain the G21 contract in [`docs/ZLIB_CONTRACT.md`](ZLIB_CONTRACT.md). This page does not replace G21. Web `CompressionStream` / `DecompressionStream` stay G13 ([`docs/STREAMS_TIMERS_CONTRACT.md`](STREAMS_TIMERS_CONTRACT.md)).

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `gzip(input, callback)` | Compresses `input` as one gzip member (RFC 1952). Invokes `callback(null, buffer)` on a later `process.nextTick` turn. The buffer is a Node `Buffer`; the first two bytes are `0x1f`, `0x8b`. Empty input still produces a gzip header member. |
| `gunzip(input, callback)` | Decompresses one gzip member. Invokes `callback(null, buffer)` on a later `process.nextTick` turn. The buffer UTF-8 bytes match the original when the input was produced by `gzip` / `gzipSync`. |

Accepted `input` forms match G21: string (UTF-8 bytes), `ArrayBuffer`, `TypedArray` / `Uint8Array`, and objects that expose a `buffer` `ArrayBuffer` (including `Buffer`). Any other value throws `TypeError` whose message contains `invalid input` **synchronously** (callback is not called).

`callback` must be a function at argument index `1`. A non-function throws `TypeError` whose message contains `callback` **synchronously**.

The arity is exactly two arguments: `(input, callback)`. An options bag or extra arguments throw `TypeError` whose message contains `options` **synchronously**.

Codec failures (corrupt or truncated compressed bytes) invoke `callback(err)` on a later `process.nextTick` turn. `err` is an `Error` (not a `TypeError`) whose message contains the method name and `failed`.

Round-trip pin: after `gzip(s, cb)` succeeds, `gunzip(compressed, cb2)` yields a `Buffer` whose `toString('utf8')` equals the original non-empty UTF-8 string `s`.

`gzip` / `gunzip` return `undefined` on the calling turn. The callback does not run inside the call.

## Limits

- Codec work runs on the isolate thread at the next `process.nextTick` turn (same delivery model as G8 async `fs`). This is not a libuv threadpool.
- Only `gzip` and `gunzip` callbacks are Stable. Other async zlib names stay outside this contract.

## Non-goals

These are outside Stable. Do **not** document them as Stable Limits fiction for missing APIs:

- `deflate` / `inflate` / `deflateRaw` / `inflateRaw` callback or Promise APIs
- Promise-returning `zlib.promises` / util.promisify special-casing beyond ordinary callback shape
- Options bags (`level`, `windowBits`, `dictionary`, flush controls)
- `createGzip`, `createGunzip`, and the rest of the streaming `create*` family
- `brotliCompress` / `brotliDecompress` / sync brotli, and any brotli surface
- `zlib.constants`, `Z_*` export constants, and `zlib.codes`
- Binding Node `stream` / `pipeline` to zlib codecs
- Web `CompressionStream` / `DecompressionStream` (G13)

## Reachability

Same installer as G21: `nodejs_core::zlib::setup_zlib_api` on the CLI path installs `globalThis.zlib`. `require('zlib')` / `require('node:zlib')` resolve that object. The contract is not feature-gated.

## Tests

```bash
cargo test --test zlib_async_contract_tests -- --test-threads=1
```

CI runs that command as `node zlib async Stable contract`.

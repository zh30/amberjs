# Node `zlib` sync contract

This is the user-facing contract for the Stable sync subset of Node `zlib` in Amber. It is derived from `src/nodejs_core/zlib.rs`, `src/runtime_minimal.rs`, and `tests/zlib_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('zlib')` and `require('node:zlib')` return the same object installed on `globalThis.zlib` by `setup_zlib_api`. This is not full Node `zlib`. It is also not the Stable Web `CompressionStream` / `DecompressionStream` surface in [`docs/STREAMS_TIMERS_CONTRACT.md`](STREAMS_TIMERS_CONTRACT.md) (G13). G13 does not graduate `require('zlib')`.

Methods that are not listed here are outside this sync contract. Async `gzip` / `gunzip` callbacks are a separate Stable surface under [`docs/ZLIB_ASYNC_CONTRACT.md`](ZLIB_ASYNC_CONTRACT.md) (tentative G48). Streaming zlib (`createGzip` and friends), brotli, and `zlib.constants` stay Non-goals of both contracts (not Stable Limits fiction).

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `gzipSync(input)` | Compresses `input` as one gzip member (RFC 1952). Returns a `Buffer`. The first two bytes are `0x1f`, `0x8b`. Empty input still produces a gzip header member. |
| `gunzipSync(input)` | Decompresses one gzip member. Returns a `Buffer` whose UTF-8 bytes match the original when the input was produced by `gzipSync`. |
| `deflateSync(input)` | Compresses `input` as one zlib member (RFC 1950), not raw DEFLATE and not gzip. Returns a `Buffer`. |
| `inflateSync(input)` | Decompresses one zlib member. Returns a `Buffer`. |
| `deflateRawSync(input)` | Compresses `input` as one raw DEFLATE member (RFC 1951). Returns a `Buffer`. |
| `inflateRawSync(input)` | Decompresses one raw DEFLATE member. Returns a `Buffer`. |

Accepted `input` forms: string (UTF-8 bytes), `ArrayBuffer`, `TypedArray` / `Uint8Array`, and objects that expose a `buffer` `ArrayBuffer` (including `Buffer`). Any other value throws `TypeError` whose message contains `invalid input`.

Codec failures (corrupt or truncated compressed bytes) throw an `Error` whose message contains the method name and `failed`.

A second options argument is not part of this contract. Level, window bits, dictionary, and flush controls are not pinned.

Round-trips that this contract pins:

- `gunzipSync(gzipSync(s)).toString('utf8') === s` for a non-empty UTF-8 string `s`
- `inflateSync(deflateSync(s)).toString('utf8') === s`
- `inflateRawSync(deflateRawSync(s)).toString('utf8') === s`

`Buffer.isBuffer` is true for every successful return value from the six methods above.

## Non-goals

- `deflate` / `inflate` / `deflateRaw` / `inflateRaw` callback or Promise APIs (async `gzip` / `gunzip` are G48, not this page)
- `createGzip`, `createGunzip`, `createDeflate`, `createInflate`, `createDeflateRaw`, `createInflateRaw`, `createBrotliCompress`, `createBrotliDecompress`
- `brotliCompressSync`, `brotliDecompressSync`, and any brotli surface
- `zlib.constants`, `Z_*` export constants, and `zlib.codes`
- Streaming Node zlib, `pipeline` into zlib, or binding Node streams to these codecs
- Web `CompressionStream` / `DecompressionStream` (see [`docs/STREAMS_TIMERS_CONTRACT.md`](STREAMS_TIMERS_CONTRACT.md))
- Node `require('stream')` (stays Preview; G13 does not graduate it)

## Reachability

The CLI `amber` binary installs `globalThis.zlib` from `src/runtime_minimal.rs` via `nodejs_core::zlib::setup_zlib_api`. `require('zlib')` / `require('node:zlib')` resolve that object through the CommonJS builtin path. Library users reach the installer through `amberjs::nodejs_core::zlib::setup_zlib_api` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.

## Tests

```bash
cargo test --test zlib_contract_tests -- --test-threads=1
```

CI runs that command as `node zlib Stable contract`.

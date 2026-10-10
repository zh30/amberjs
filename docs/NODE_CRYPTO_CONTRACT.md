# Node `crypto` contract

This is the user-facing contract for the Stable **subset** of Node `crypto` in Amber (**G23**; path G17, events G18, buffer G19, os G20, zlib G21, util+process G22, and additive G8 `existsSync` are already Stable on `main`). It is derived from `src/nodejs_core/crypto.rs` (`createHash`, `createHmac`, `randomBytes`, `randomBytesSync`), `src/runtime_minimal.rs` (`timingSafeEqual` on the same global object), `require('crypto')` / `node:crypto` wiring, and `tests/node_crypto_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('crypto')`, `require('node:crypto')`, and `import` from `'crypto'` / `'node:crypto'` reach the same object as `globalThis.crypto` for the methods named below. Named ESM exports include those methods. This is **not** the Stable Web Crypto contract in [`docs/WEB_CRYPTO_CONTRACT.md`](WEB_CRYPTO_CONTRACT.md) (G14: `crypto.subtle`, `crypto.getRandomValues`, and `CryptoKey`).

This is not full Node `crypto`. Methods that are not listed here — including password `createCipher` / `createDecipher`, AES-GCM / CTR / CFB / OFB / ECB cipher modes, `pbkdf2`, `scrypt`, `createSign` / `createVerify`, `generateKeyPair`, KeyObjects, `hkdf`, Diffie-Hellman, and RSA encrypt helpers — are outside **this** contract even when present on the object. Do not treat their presence as a Stable promise. The narrow AES-*-CBC `createCipheriv` / `createDecipheriv` carve is a separate Stable page (**G43**, [`docs/NODE_CRYPTO_CIPHERIV_CONTRACT.md`](NODE_CRYPTO_CIPHERIV_CONTRACT.md)).

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `createHash(algorithm)` | Returns a hash object. Algorithms: `md5`, `sha1`, `sha256`, `sha384`, `sha512`, `blake3` (and the common aliases `SHA-256`, `sha-384`, …). Unknown algorithms throw `TypeError` whose message contains `Unsupported hash algorithm`. |
| `hash.update(data[, encoding])` | Accepts a string (default UTF-8), `Buffer` / `Uint8Array` / `ArrayBuffer`, or a string with encoding `utf8`, `hex`, `base64`, `base64url`, `latin1`, or `binary`. Returns the same hash object. |
| `hash.digest([encoding])` | Omitting encoding returns a `Uint8Array`. `hex`, `base64`, `base64url`, `latin1`, and `binary` return strings. A second `digest` on the same hash throws. |
| `hash.copy()` | Returns a new hash with the same algorithm and the data updated so far. Further `update` on either object does not affect the other. |
| `createHmac(algorithm, key)` | Same hash algorithm set as `createHash`. `key` is a string or byte view (optional string encoding as the third argument to `createHmac` is accepted when present). Unknown algorithms throw `TypeError` whose message contains `Unsupported HMAC algorithm`. |
| `hmac.update` / `hmac.digest` | Same input and output encodings as hash. A second `digest` without encoding returns an empty `Uint8Array`; with an encoding it returns an empty string. There is no `hmac.copy`. |
| `randomBytes(size[, callback])` | Sync when called without a callback: returns a `Uint8Array` of `size` bytes. With a callback, the callback is invoked on the same turn as `(null, uint8Array)` and the call also returns that `Uint8Array`. `size` must be an integer `0` … `2147483647`; otherwise a `RangeError` (or `TypeError` when the size is not a number) is thrown. |
| `randomBytesSync(size)` | Same bytes and size rules as sync `randomBytes`. |
| `randomUUID()` | Lowercase UUID version 4 string (36 characters). Same function as `globalThis.crypto.randomUUID` from the Web Crypto Stable contract. `--seed` does not change it. |
| `timingSafeEqual(a, b)` | Constant-time compare of two equal-length `Buffer` / TypedArray / `ArrayBuffer` values. Equal lengths and equal bytes → `true`; equal lengths and different bytes → `false`. Unequal lengths throw `TypeError` whose message mentions length. |

Known digests (hex):

| Call | Digest |
| :--- | :--- |
| `createHash('md5').update('hello').digest('hex')` | `5d41402abc4b2a76b9719d911017c592` |
| `createHash('sha256').update('hello').digest('hex')` | `2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824` |
| `createHash('sha384').update('hello').digest('hex')` | `59e1748777448c69de6b800d7a33bbfb9ff1b463e44354c3553bcdb9c666fa90125a3c79f90397bdf5f6a13de828684f` |
| `createHash('sha512').update('hello').digest('hex')` | `9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca72323c3d99ba5c11d7c7acc6e14b8c5da0c4663475c2e5c3adef46f73bcdec043` |
| `createHmac('sha256', 'key').update('The quick brown fox jumps over the lazy dog').digest('hex')` | `f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8` |

## Determinism

When `amber run --seed <u64>` is set, `randomBytes` / `randomBytesSync` fill from the deterministic PRNG (same family as `crypto.getRandomValues`). `randomUUID` stays OS-random and is not reproducible under `--seed`.

## Limits

- Return values of `randomBytes` and bare `digest()` are `Uint8Array`, not Node `Buffer` instances (`constructor.name` is `Uint8Array`). They remain usable where TypedArrays are accepted.
- `randomBytes(size, cb)` runs `cb` synchronously and still returns the bytes. It is not a libuv threadpool job and does not return `undefined`.
- `timingSafeEqual` is installed on the shared crypto object from the runtime path, not from a separate method table in `nodejs_core/crypto.rs`.
- KDF, sign/verify, keypair, and KeyObject APIs on the same object stay Preview.
- AES-*-CBC `createCipheriv` / `createDecipheriv` is Stable under **G43** ([`docs/NODE_CRYPTO_CIPHERIV_CONTRACT.md`](NODE_CRYPTO_CIPHERIV_CONTRACT.md)); password `createCipher` / `createDecipher`, AES-GCM, and other cipher modes stay outside G23 and outside that carve unless pinned there.
- Web `crypto.subtle` and `crypto.getRandomValues` stay the G14 contract only.

## Non-goals

- Full Node `crypto` parity.
- Expanding this page to password `createCipher`, AES-GCM, CTR/CFB/OFB/ECB, RSA, scrypt, pbkdf2, or `createSign` by listing them as “limits”.
- Claiming Web Crypto algorithms or `CryptoKey` through `require('crypto')`.
- Streaming Hash / Hmac as Node `stream.Transform` subclasses.

## Reachability

The CLI `amber` binary installs this object from `src/runtime_minimal.rs` (`setup_legacy_web_apis` then `nodejs_core::crypto::setup_crypto_api`, then Web Crypto for `subtle` / `randomUUID` / `getRandomValues`). Library users reach `setup_crypto_api` through `amberjs::nodejs_core`. The contract is not feature-gated.

## Tests

```bash
cargo test --test node_crypto_contract_tests -- --test-threads=1
```

CI runs that command as `node crypto Stable contract`, next to the other Stable contract steps.

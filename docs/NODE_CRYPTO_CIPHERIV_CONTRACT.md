# Node `crypto` cipheriv CBC contract

This is the user-facing contract for a **narrow** Stable carve of Node `crypto` in Amber: **`createCipheriv` / `createDecipheriv` for AES-*-CBC only** (**G43**). It is derived from `src/nodejs_core/crypto.rs` (`create_cipheriv_callback`, `create_decipheriv_callback`, CBC path in `cipher_update_callback` / `cipher_final_callback`), `src/runtime_minimal.rs` (crypto install on the CLI path), `require('crypto')` / `node:crypto` wiring, and `tests/crypto_cipheriv_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

This is **additive** to the G23 hash/HMAC/`randomBytes`/`timingSafeEqual` subset in [`docs/NODE_CRYPTO_CONTRACT.md`](NODE_CRYPTO_CONTRACT.md). It is **not** Web Crypto (G14). It is **not** full Node `crypto`.

**Honesty:** Password-based `createCipher` / `createDecipher`, and AES-GCM (including `setAAD` / `getAuthTag` / `setAuthTag`), must **never** be graduated by renaming incomplete or bag-wide behavior as Stable Limits. CTR / CFB / OFB / ECB on the same object stay outside this carve even when callable.

## Stable surface

| Call / field | Behavior |
| :--- | :--- |
| `require('crypto')` / `require('node:crypto')` | Same object as `globalThis.crypto` for the methods named here and in G23. |
| `createCipheriv(algorithm, key, iv)` | Returns a cipher object when `algorithm` is an AES-CBC name below, `key` has the required byte length, and `iv` is exactly 16 bytes. |
| `createDecipheriv(algorithm, key, iv)` | Same algorithm / key / IV rules; returns a decipher object. |
| Algorithms | `aes-128-cbc`, `aes-192-cbc`, `aes-256-cbc`, plus Node bare aliases `aes128` / `aes192` / `aes256` (normalized to the matching `aes-*-cbc`). Case-insensitive. |
| Key length | AES-128 → 16 bytes; AES-192 → 24 bytes; AES-256 → 32 bytes. Wrong length throws with message containing `invalid key length` and `code === 'ERR_CRYPTO_INVALID_KEYLEN'`. |
| IV length | Exactly 16 bytes for CBC. Wrong length throws with message containing `invalid iv length` and `code === 'ERR_CRYPTO_INVALID_IV'`. |
| Key / IV input | Hex string (even length, ASCII hex digits only → decoded) or `Buffer` / `Uint8Array` byte view. Non-hex strings are taken as UTF-8 raw bytes. |
| Unknown algorithm | Throws with message containing `unsupported algorithm` and `code === 'ERR_CRYPTO_UNKNOWN_CIPHER'`. |
| `cipher.update(data[, inputEncoding][, outputEncoding])` | Accepts string (default UTF-8) or byte view; optional encodings include `utf8`, `hex`, `base64` (and related Node-shaped aliases already used by G23 hash paths). Returns ciphertext chunk (string when `outputEncoding` is set, else a byte view). Split updates keep CBC chaining state. |
| `cipher.final([outputEncoding])` | Flushes the last CBC block (PKCS#7 padding by default). A second `final` / `update` after finalize throws. |
| `decipher.update` / `decipher.final` | Inverse of encrypt; decrypts Node/OpenSSL-compatible CBC ciphertext including split-update timing. |
| `setAutoPadding(enabled)` | When called with `false` before `final`, encrypt omits the extra PKCS#7 padding block for block-aligned plaintext (Node-shaped). Default is padding on. Returns the cipher/decipher object. |

Known vector (hex), AES-128-CBC:

| Field | Value |
| :--- | :--- |
| key | `00112233445566778899aabbccddeeff` |
| iv | `0102030405060708090a0b0c0d0e0f10` |
| plaintext (two 16-byte blocks) | `000102030405060708090a0b0c0d0e0f` + `101112131415161718191a1b1c1d1e1f` |
| ciphertext (`update`+`update`+`final` hex) | `da1dca49b61ef24bdd0e15e681c8a1ba4a8588657b946e13ed4f5f6a3cc66cf5b04e433e26a6a25da21cdeedc9d34611` |

## Limits

These are real behaviors of the Stable CBC carve, not cover for other cipher modes:

- Return chunks without an output encoding are TypedArray / byte-view shaped (same family as G23 digests), not necessarily Node `Buffer` instances.
- Hex-string key/IV decoding is Amber's even-length ASCII-hex heuristic; it is not a separate `encoding` argument on `createCipheriv`.
- This page does not pin cipher objects as Node `stream.Transform` subclasses, `pipe`, or event emitters.
- GCM auth-tag helpers may exist on non-CBC objects; they are **not** this contract.

## Non-goals

Outside this contract (Preview / DEFER). Do **not** treat these as Stable Limits of bag-wide or dishonest behavior:

- **Password-based `createCipher` / `createDecipher`** (EVP_BytesToKey / password→key derivation). Never graduate by listing password KDF quirks as Limits.
- **AES-GCM** (`aes-*-gcm`, `setAAD`, `getAuthTag`, `setAuthTag`) — even when Preview tests round-trip. Never claim GCM-as-Limits fiction on this page.
- **AES-CTR / CFB / OFB / ECB** and other OpenSSL names accepted by `get_cipher` today — callable Preview only; not Stable.
- Expanding G23 to “all crypto methods on the object”.
- KDF (`pbkdf2`, `scrypt`, `hkdf`), `createSign` / `createVerify`, `generateKeyPair`, KeyObjects, Diffie-Hellman, RSA encrypt helpers.
- Web Crypto `crypto.subtle` / `CryptoKey` (G14 only).
- Graduating http client / Agent / `https` / `http2` / SharedWorker / Push by renaming stubs as Limits (unrelated; never paper over).

## Reachability

The CLI `amber` binary installs the crypto object from `src/runtime_minimal.rs` via `nodejs_core::crypto::setup_crypto_api`. `require('crypto')` / `require('node:crypto')` return that object. Library users reach the installer through `amberjs::nodejs_core`. The contract is not feature-gated.

## Tests

```bash
cargo test --test crypto_cipheriv_contract_tests -- --test-threads=1
```

CI step: `node crypto cipheriv CBC Stable contract`.

Broader Preview coverage (CTR/GCM/etc.) remains in `tests/crypto_cipheriv_tests.rs` and is **not** this Stable pin.

## Honesty rule

Password `createCipher` and AES-GCM must stay Non-goals. Limits describe CBC key/IV/update/final quirks only. Graduating “cipher” as a bag is forbidden.

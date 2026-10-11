# Node `crypto` cipheriv CTR contract

This is the user-facing contract for a **narrow** Stable carve of Node `crypto` in Amber: **`createCipheriv` / `createDecipheriv` for AES-*-CTR only** (**G52**). It is derived from `src/nodejs_core/crypto.rs` (`create_cipheriv_callback`, `create_decipheriv_callback`, CTR path in `cipher_update_callback` / `cipher_final_callback` via `is_ctr_cipher`), `src/runtime_minimal.rs` (crypto install on the CLI path), `require('crypto')` / `node:crypto` wiring, and `tests/crypto_cipheriv_ctr_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

This is **additive** to the G23 hash/HMAC/`randomBytes`/`timingSafeEqual` subset in [`docs/NODE_CRYPTO_CONTRACT.md`](NODE_CRYPTO_CONTRACT.md), to the G43 AES-*-CBC carve in [`docs/NODE_CRYPTO_CIPHERIV_CONTRACT.md`](NODE_CRYPTO_CIPHERIV_CONTRACT.md), and to the AES-*-GCM carve (**G51**, [`docs/NODE_CRYPTO_CIPHERIV_GCM_CONTRACT.md`](NODE_CRYPTO_CIPHERIV_GCM_CONTRACT.md)). It is **not** Web Crypto (G14). It is **not** full Node `crypto`.

**Honesty:** Password-based `createCipher` / `createDecipher`, and other cipher modes on this page (CBC/GCM/CFB/OFB/ECB as *this* Stable pin), must **never** be graduated by renaming incomplete or bag-wide behavior as Stable Limits. CBC stays **G43**; GCM stays **G51**; CFB/OFB/ECB stay outside Stable even when callable.

## Stable surface

| Call / field | Behavior |
| :--- | :--- |
| `require('crypto')` / `require('node:crypto')` | Same object as `globalThis.crypto` for the methods named here, in G23, and in the G43/G51 cipheriv carves. |
| `createCipheriv(algorithm, key, iv)` | Returns a cipher object when `algorithm` is an AES-CTR name below, `key` has the required byte length, and `iv` is exactly 16 bytes (initial counter block). |
| `createDecipheriv(algorithm, key, iv)` | Same algorithm / key / IV rules; returns a decipher object. |
| Algorithms | `aes-128-ctr`, `aes-192-ctr`, `aes-256-ctr`, plus Amber aliases `aes128-ctr` / `aes192-ctr` / `aes256-ctr`. Case-insensitive. (Bare Node CBC aliases `aes128` / `aes192` / `aes256` remain the G43 CBC path, not this page.) |
| Key length | AES-128 → 16 bytes; AES-192 → 24 bytes; AES-256 → 32 bytes. Wrong length throws with message containing `invalid key length` and `code === 'ERR_CRYPTO_INVALID_KEYLEN'`. |
| IV length | Exactly 16 bytes for AES-CTR. Wrong length throws with message containing `invalid iv length` and `code === 'ERR_CRYPTO_INVALID_IV'`. |
| Key / IV input | Hex string (even length, ASCII hex digits only → decoded) or `Buffer` / `Uint8Array` byte view. Non-hex strings are taken as UTF-8 raw bytes. |
| Unknown algorithm | Throws with message containing `unsupported algorithm` and `code === 'ERR_CRYPTO_UNKNOWN_CIPHER'`. |
| `cipher.update(data[, inputEncoding][, outputEncoding])` | Accepts string (default UTF-8) or byte view; optional encodings include `utf8`, `hex`, `base64` (and related Node-shaped aliases already used by G23 hash paths). Returns ciphertext chunk (string when `outputEncoding` is set, else a byte view). Split updates keep CTR counter state; CTR emits keystream output on `update` (not deferred until `final` only). |
| `cipher.final([outputEncoding])` | Flushes any remaining CTR output (often empty when all plaintext was already emitted by `update`). A second `final` / `update` after finalize throws. CTR is a stream cipher — there is no PKCS#7 padding block. |
| `decipher.update` / `decipher.final` | Inverse of encrypt; decrypts Node/OpenSSL-compatible CTR ciphertext including split-update timing. |

Known vector (hex), AES-128-CTR:

| Field | Value |
| :--- | :--- |
| key | `00112233445566778899aabbccddeeff` |
| iv (counter) | `0102030405060708090a0b0c0d0e0f10` |
| plaintext (split) | `00` then `010203` |
| ciphertext chunks (`update`+`update`+`final` hex) | `bf` + `642bbd` + `` (empty `final`) |

## Limits

These are real behaviors of the Stable CTR carve, not cover for other cipher modes:

- Return chunks without an output encoding are TypedArray / byte-view shaped (same family as G23 digests), not necessarily Node `Buffer` instances.
- Hex-string key/IV decoding is Amber's even-length ASCII-hex heuristic; it is not a separate `encoding` argument on `createCipheriv`.
- This page does not pin cipher objects as Node `stream.Transform` subclasses, `pipe`, or event emitters.
- `setAutoPadding` is not part of this CTR pin (CTR has no block padding). GCM auth-tag helpers are **not** this contract.
- Amber's `aes128-ctr` / `aes192-ctr` / `aes256-ctr` aliases are Stable here; upstream Node may reject those spellings — the OpenSSL `aes-*-ctr` names are the cross-runtime pin.

## Non-goals

Outside this contract (Preview / DEFER / other Stable pages). Do **not** treat these as Stable Limits of bag-wide or dishonest behavior on *this* page:

- **Password-based `createCipher` / `createDecipher`** (EVP_BytesToKey / password→key derivation). Never graduate by listing password KDF quirks as Limits.
- **AES-*-CBC** — Stable under **G43** ([`docs/NODE_CRYPTO_CIPHERIV_CONTRACT.md`](NODE_CRYPTO_CIPHERIV_CONTRACT.md)); not re-pinned here.
- **AES-*-GCM** (`aes-*-gcm`, `setAAD`, `getAuthTag`, `setAuthTag`) — Stable under **G51** ([`docs/NODE_CRYPTO_CIPHERIV_GCM_CONTRACT.md`](NODE_CRYPTO_CIPHERIV_GCM_CONTRACT.md)); not re-pinned here. Never claim GCM-as-Limits fiction on this page.
- **AES-CFB / OFB / ECB** and other OpenSSL names accepted by `get_cipher` today — callable Preview only; not Stable.
- Expanding G23 to “all crypto methods on the object”.
- KDF (`pbkdf2`, `scrypt`, `hkdf`), `createSign` / `createVerify`, `generateKeyPair`, KeyObjects, Diffie-Hellman, RSA encrypt helpers.
- Web Crypto `crypto.subtle` / `CryptoKey` (G14 only).
- Graduating http client / Agent / `https` / `http2` / SharedWorker / Push by renaming stubs as Limits (unrelated; never paper over).

## Reachability

The CLI `amber` binary installs the crypto object from `src/runtime_minimal.rs` via `nodejs_core::crypto::setup_crypto_api`. `require('crypto')` / `require('node:crypto')` return that object. Library users reach the installer through `amberjs::nodejs_core`. The contract is not feature-gated.

## Tests

```bash
cargo test --test crypto_cipheriv_ctr_contract_tests -- --test-threads=1
```

CI step: `node crypto cipheriv CTR Stable contract`.

Broader Preview coverage (CBC/GCM/etc.) remains in `tests/crypto_cipheriv_tests.rs` and the G43/G51 contract suites; those are **not** this Stable pin.

## Honesty rule

Password `createCipher` and other modes (CBC/GCM/CFB/OFB/ECB) must stay Non-goals of *this* page. Limits describe CTR key/IV/update/final quirks only. Graduating “cipher” as a bag is forbidden.

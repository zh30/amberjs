# Node `crypto` cipheriv AES-GCM contract

This is the user-facing contract for a **narrow** Stable carve of Node `crypto` in Amber: **`createCipheriv` / `createDecipheriv` for AES-*-GCM** plus **`getAuthTag` / `setAuthTag` / `setAAD`** (**G51**). It is derived from `src/nodejs_core/crypto.rs` (`create_cipheriv_callback`, `create_decipheriv_callback`, GCM helpers `set_gcm_methods` / `apply_gcm_aad` / auth-tag get/set, GCM path in `cipher_update_callback` / `cipher_final_callback`), `src/runtime_minimal.rs` (crypto install on the CLI path), `require('crypto')` / `node:crypto` wiring, and `tests/crypto_cipheriv_gcm_contract_tests.rs`. Broader Preview coverage remains in `tests/crypto_cipheriv_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

This is **additive** to G23 ([`docs/NODE_CRYPTO_CONTRACT.md`](NODE_CRYPTO_CONTRACT.md)) and to the G43 AES-*-CBC carve ([`docs/NODE_CRYPTO_CIPHERIV_CONTRACT.md`](NODE_CRYPTO_CIPHERIV_CONTRACT.md)). It is **not** Web Crypto (G14). It is **not** full Node `crypto`.

**Honesty:** Password-based `createCipher` / `createDecipher`, and AES-CTR / CFB / OFB / ECB on the same object, must **never** be graduated by renaming incomplete or bag-wide behavior as Stable Limits on this page.

## Stable surface

| Call / field | Behavior |
| :--- | :--- |
| `require('crypto')` / `require('node:crypto')` | Same object as `globalThis.crypto` for the methods named here, in G23, and in G43. |
| `createCipheriv(algorithm, key, iv)` | Returns a cipher object when `algorithm` is an AES-GCM name below, `key` has the required byte length, and `iv` is exactly 12 bytes. |
| `createDecipheriv(algorithm, key, iv)` | Same algorithm / key / IV rules; returns a decipher object. |
| Algorithms | `aes-128-gcm`, `aes-192-gcm`, `aes-256-gcm`, plus hyphen aliases `aes128-gcm` / `aes192-gcm` / `aes256-gcm`. Case-insensitive. |
| Key length | AES-128 → 16 bytes; AES-192 → 24 bytes; AES-256 → 32 bytes. Wrong length throws with message containing `invalid key length` and `code === 'ERR_CRYPTO_INVALID_KEYLEN'`. |
| IV length | Exactly **12 bytes** for GCM. Wrong length throws with message containing `invalid iv length` and `code === 'ERR_CRYPTO_INVALID_IV'`. |
| Key / IV input | Hex string (even length, ASCII hex digits only → decoded) or `Buffer` / `Uint8Array` byte view. Non-hex strings are taken as UTF-8 raw bytes. |
| Unknown algorithm | Throws with message containing `unsupported algorithm` and `code === 'ERR_CRYPTO_UNKNOWN_CIPHER'`. |
| `cipher.update(data[, inputEncoding][, outputEncoding])` | Accepts string (default UTF-8) or byte view; optional encodings include `utf8`, `hex`, `base64` (and related Node-shaped aliases already used by G23 hash paths). Returns ciphertext chunk (string when `outputEncoding` is set, else a byte view). |
| `cipher.final([outputEncoding])` | Completes GCM encryption and materializes the 16-byte auth tag for `getAuthTag`. A second `final` / `update` after finalize throws. |
| `cipher.getAuthTag()` | After successful encrypt `final`, returns the 16-byte authentication tag as a byte view. Calling before `final` throws (message contains `auth tag` / `before final`). |
| `cipher.setAAD(aad[, encoding])` | Optional additional authenticated data before `final`. Returns the cipher object. May be called more than once (bytes accumulate). Calling after `final` throws. |
| `decipher.setAuthTag(tag)` | Required before decrypt `final`. Tag length must be `1`…`16` bytes; empty or `> 16` throws (`invalid auth tag length`). Returns the decipher object. Calling after `final` throws. |
| `decipher.setAAD(aad[, encoding])` | Same AAD rules as encrypt; AAD must match the encrypt side for auth to succeed. |
| `decipher.update` / `decipher.final` | Inverse of encrypt; decrypts Node/OpenSSL-compatible GCM ciphertext. Missing auth tag before `final` throws (`auth tag is required for AES-GCM`). Wrong tag fails closed on `final` (throw; message mentions auth / final / bad decrypt). |

Known vector (hex), AES-128-GCM:

| Field | Value |
| :--- | :--- |
| key | `000102030405060708090a0b0c0d0e0f` |
| iv | `101112131415161718191a1b` (12 bytes) |
| aad | `feedfacedeadbeef` |
| plaintext | `00112233445566778899aabbccddeeff` |
| ciphertext (`update`+`final` hex) | `c43f219c4b1ad0989f44f74e0bfa05c1` |
| auth tag (`getAuthTag` hex) | `8904c9157db4cc2579d0d2226f069e32` |

## Limits

These are real behaviors of the Stable GCM carve, not cover for other cipher modes:

- Return chunks and auth tags without an output encoding are TypedArray / byte-view shaped (same family as G23 digests), not necessarily Node `Buffer` instances.
- Hex-string key/IV decoding is Amber's even-length ASCII-hex heuristic; it is not a separate `encoding` argument on `createCipheriv`.
- This page does not pin cipher objects as Node `stream.Transform` subclasses, `pipe`, or event emitters.
- Encrypt always produces a **16-byte** auth tag (`getAuthTag`); shorter tags may be accepted on decrypt via `setAuthTag` (OpenSSL-shaped `1`…`16`), but the known vector and Stable pin use the full 16-byte tag.
- GCM ignores PKCS#7-style auto-padding (padding is not applied on the GCM path).

## Non-goals

Outside this contract (Preview / DEFER). Do **not** treat these as Stable Limits of bag-wide or dishonest behavior:

- **Password-based `createCipher` / `createDecipher`** (EVP_BytesToKey / password→key derivation). Never graduate by listing password KDF quirks as Limits.
- **AES-CTR / CFB / OFB / ECB** and other OpenSSL names accepted by `get_cipher` today — callable Preview only; not this page (CTR may graduate separately later).
- Expanding G23 / G43 to “all crypto methods on the object”.
- KDF (`pbkdf2`, `scrypt`, `hkdf`), `createSign` / `createVerify`, `generateKeyPair`, KeyObjects, Diffie-Hellman, RSA encrypt helpers.
- Web Crypto `crypto.subtle` / `CryptoKey` (G14 only).
- Graduating http client / Agent / `https` / `http2` / SharedWorker / Push by renaming stubs as Limits (unrelated; never paper over).

AES-*-CBC remains the separate **G43** contract ([`docs/NODE_CRYPTO_CIPHERIV_CONTRACT.md`](NODE_CRYPTO_CIPHERIV_CONTRACT.md)).

## Reachability

The CLI `amber` binary installs the crypto object from `src/runtime_minimal.rs` via `nodejs_core::crypto::setup_crypto_api`. `require('crypto')` / `require('node:crypto')` return that object. Library users reach the installer through `amberjs::nodejs_core`. The contract is not feature-gated.

## Tests

```bash
cargo test --test crypto_cipheriv_gcm_contract_tests -- --test-threads=1
```

CI step: `node crypto cipheriv GCM Stable contract`.

Broader Preview coverage (CTR/CFB/OFB/ECB/etc.) remains in `tests/crypto_cipheriv_tests.rs` and is **not** this Stable pin. CBC stays on `tests/crypto_cipheriv_contract_tests.rs` (**G43**).

## Honesty rule

Password `createCipher` and non-GCM modes must stay Non-goals of **this** page. Limits describe GCM key/IV/AAD/auth-tag/update/final quirks only. Graduating “cipher” as a bag is forbidden.

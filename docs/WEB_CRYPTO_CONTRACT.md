# Web Crypto contract

This is the user-facing contract for Stable Web Crypto in the default Amber runtime. It is derived from `src/web_api/crypto.rs` and `tests/web_crypto_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

The surface is `globalThis.crypto.subtle`, `crypto.randomUUID`, `crypto.getRandomValues`, and the `CryptoKey` objects those methods return. It is reachable from `amber run` and `amber eval`. Fetch, URL / encoding / `structuredClone`, `Blob` / `File` / `FormData`, events / abort / channels, and streams / timers / performance stay on their own Stable contracts. The rest of `src/web_api/` stays Preview. The Node `crypto` subset in [`docs/NODE_CRYPTO_CONTRACT.md`](NODE_CRYPTO_CONTRACT.md) (`createHash`, `createHmac`, `randomBytes`, `randomUUID`, `timingSafeEqual`) is a separate Stable contract on the same global object; other Node `crypto` methods stay Preview. This is not the Web Crypto standard.

## `crypto.randomUUID` and `crypto.getRandomValues`

`crypto.randomUUID()` returns a lowercase UUID version 4 string: 36 characters, hyphens in the usual places, the version nibble is `4`, and the variant nibble is `8`, `9`, `a`, or `b`. It uses OS randomness. `--seed` does not change it.

`crypto.getRandomValues(view)` fills an integer TypedArray (`Uint8Array`, `Uint16Array`, `Uint32Array`, `Int8Array`, `Int16Array`, `Int32Array`, `Uint8ClampedArray`, `BigInt64Array`, `BigUint64Array`). It returns the same view and writes only that view's byte window. A float TypedArray throws `TypeError`. A view longer than 65536 bytes throws `RangeError`. `--seed` fills the view from the deterministic PRNG.

## `subtle.digest`

Algorithms: `SHA-1`, `SHA-256`, `SHA-384`, `SHA-512`. The algorithm is a string (`SHA-256`, `sha-256`, `SHA256`, `sha256`, and the same spellings for the other three) or `{ name }`.

`data` must be an `ArrayBuffer` or a TypedArray. The promise resolves to a `Uint8Array` of the digest (`SHA-1` is 20 bytes, `SHA-256` is 32, `SHA-384` is 48, `SHA-512` is 64). An unknown algorithm throws `Error` (`Unsupported hash algorithm`) before a promise is returned.

## `CryptoKey`

A key is a plain object. There is no `CryptoKey` constructor, and `instanceof CryptoKey` is not part of this contract.

| Field | Value |
| :--- | :--- |
| `type` | `secret`, `public`, or `private` |
| `extractable` | the boolean passed to `generateKey` / `importKey` |
| `algorithm.name` | the name string that was passed, except `Ed25519` and `Ed448`, which are stored in that canonical spelling |
| `algorithm.length` | AES keys, in bits |
| `algorithm.hash.name` | HMAC, RSA-OAEP, and RSASSA-PKCS1-v1_5 (`SHA-1`, `SHA-256`, `SHA-384`, or `SHA-512`) |
| `algorithm.namedCurve` | ECDSA and ECDH (`P-256`, `P-384`, or `P-521`) |
| `usages` | the usage strings stored on that key |

Key bytes are not a visible property. Operations compare algorithm names without regard to case. A usage the algorithm does not allow throws. A key whose usages do not include the operation throws.

Successful `subtle` operations return a Promise. Most validation failures throw before that promise exists. `exportKey` of a key with `extractable: false` rejects the promise with `exportKey: key is not extractable`.

## Import and export

`importKey` accepts format `raw`, `jwk`, `spki`, or `pkcs8`. Any other format throws.

| Key | Import | Export |
| :--- | :--- | :--- |
| HMAC, AES-GCM, AES-CBC, AES-CTR, AES-KW | `raw` bytes, or JWK `kty: "oct"` | `raw` bytes, or JWK `kty: "oct"` with `k`, `alg`, `key_ops`, and `ext: true` |
| PBKDF2 | `raw` password bytes | `raw` password bytes when extractable |
| Ed25519, Ed448 | `raw` public key, JWK `kty: "OKP"`, `spki` public, `pkcs8` private | the same. `raw` / `spki` reject a private key. `pkcs8` rejects a public key |
| ECDSA, ECDH | not supported | `raw` public key is the uncompressed point (`0x04` ‖ X ‖ Y). Private `raw`, and every `jwk` / `spki` / `pkcs8` export, reject |
| RSA-OAEP, RSASSA-PKCS1-v1_5 | not supported | `raw` and `jwk` reject |

AES `raw` keys are 16, 24, or 32 bytes. An `oct` JWK `alg`, when present, must match the table below. `key_ops`, when present, must include every requested usage. `ext: false` cannot be imported with `extractable: true`. `k`, `x`, and `d` are base64url without padding.

JWK `alg` written by export:

| Key | `alg` |
| :--- | :--- |
| HMAC-SHA-1 / 256 / 384 / 512 | `HS1`, `HS256`, `HS384`, `HS512` |
| AES-GCM | `A128GCM`, `A192GCM`, `A256GCM` |
| AES-CBC | `A128CBC`, `A192CBC`, `A256CBC` |
| AES-CTR | `A128CTR`, `A192CTR`, `A256CTR` |
| AES-KW | `A128KW`, `A192KW`, `A256KW` |
| Ed25519, Ed448 | `Ed25519`, `Ed448`, with `crv` set to the same name |

`HS1` is the label this runtime writes for HMAC-SHA-1. It is not an IANA JWK `alg`.

## `generateKey`

| Algorithm | Result |
| :--- | :--- |
| HMAC | One `secret` key. The key is always 64 bytes. A `length` field is ignored. `hash` is `SHA-1`, `SHA-256`, `SHA-384`, or `SHA-512`. |
| AES-GCM, AES-CBC, AES-CTR, AES-KW | One `secret` key. `length` is required and is `128`, `192`, or `256`. |
| RSA-OAEP | `{ publicKey, privateKey }` from `modulusLength`, `publicExponent` (`Uint8Array`), and `hash`. Public usages are `encrypt` and `wrapKey`. Private usages are `decrypt` and `unwrapKey`. |
| RSASSA-PKCS1-v1_5 | `{ publicKey, privateKey }`. Private usage is `sign`. Public usage is `verify`. |
| Ed25519, Ed448 | `{ publicKey, privateKey }`. |
| ECDSA, ECDH | `{ publicKey, privateKey }` on `namedCurve` `P-256`, `P-384`, or `P-521`. An omitted curve is `P-256`. |

## Encrypt, decrypt, sign, and verify

Byte results of `encrypt`, `decrypt`, and `sign` are `ArrayBuffer`s. `verify` resolves to a boolean.

| Algorithm | Operation |
| :--- | :--- |
| AES-GCM | `iv` is 12 bytes. `tagLength`, when set, must be `128`. `additionalData`, when it is a BufferSource, is the AAD. The result is ciphertext followed by a 16-byte tag. A bad tag or a different AAD throws. 128- and 256-bit keys use ring. 192-bit keys use OpenSSL AES-192-GCM. |
| AES-CBC | `iv` is 16 bytes. PKCS#7 padding. 128-, 192-, and 256-bit keys. |
| AES-CTR | `counter` is 16 bytes. `length` is an integer from 1 to 128. A message that would exhaust that counter space throws. 128-, 192-, and 256-bit keys. |
| RSA-OAEP | Public key encrypts. Private key decrypts. The hash is the key's hash. `label`, when set, must be a BufferSource and must match on decrypt. |
| HMAC | Signs and verifies with the key's hash. A wrong signature resolves `false`. |
| Ed25519, Ed448 | No hash parameter. Signatures are 64 and 114 bytes. A private key signs. A public key verifies. A wrong message resolves `false`. |
| ECDSA | Hash comes from the operation (`SHA-256` when omitted). The signature is raw `r ‖ s`: 64 bytes on P-256, 96 on P-384, 132 on P-521. |
| RSASSA-PKCS1-v1_5 | PKCS#1 v1.5 with the key's hash. A wrong signature resolves `false`. |

AES-CBC and AES-CTR without the required `iv` or `counter`, and AES-GCM without a 12-byte `iv`, throw. `encrypt` / `decrypt` of any other algorithm throws `not implemented`. `sign` of an RSA algorithm other than RSASSA-PKCS1-v1_5 throws `not implemented`.

## Derive, wrap, and unwrap

`deriveBits` and `deriveKey` accept PBKDF2 and ECDH.

- PBKDF2 `hash` is `SHA-256`, `SHA-384`, or `SHA-512`. An omitted `salt` is 16 zero bytes. An omitted `iterations` is `100000`. `deriveBits` length is in bits and is rounded up to a whole number of bytes; a missing length is 256. `deriveKey` reads `length` from the derived algorithm (default 256) and returns a `secret` key.
- ECDH reads `algorithm.public`. `deriveBits` returns the shared secret. Two peers on the same curve derive the same bytes.

`wrapKey` and `unwrapKey` accept AES-GCM and AES-KW, formats `raw` and `jwk`. The key being wrapped must be extractable. The wrapping key needs usage `wrapKey`; the unwrapping key needs `unwrapKey`. AES-GCM wrap uses a 12-byte `iv`, empty AAD, and a 16-byte tag. AES-KW is RFC 3394: the payload is at least 16 bytes and a multiple of 8. A 32-byte key wraps to 40 bytes. A JWK wrap encrypts the JSON object (`kty` `oct` or `OKP`), not the raw internal bytes. AES-KW is not an `encrypt` / `decrypt` algorithm.

## Non-goals

- HKDF, RSA-PSS, X25519, and any digest other than SHA-1/256/384/512.
- AES-GCM nonces other than 12 bytes, or a tag other than 16 bytes.
- Importing RSA, ECDSA, or ECDH keys. Exporting those keys as JWK, SPKI, or PKCS#8.
- A `CryptoKey` constructor.
- Graduating Node `crypto` beyond [`docs/NODE_CRYPTO_CONTRACT.md`](NODE_CRYPTO_CONTRACT.md). `createCipheriv`, `pbkdf2`, `scrypt`, sign/verify, and KeyObjects stay Preview.
- String inputs to `digest`, `encrypt`, `sign`, or `verify`. The data must be an `ArrayBuffer` or a TypedArray.
- `--seed` changing `randomUUID`.
- Turning every failure into a rejected promise. Callers must handle both thrown exceptions and rejections.

## Tests

```bash
cargo test --test web_crypto_contract_tests -- --test-threads=1
```

CI runs that command as `web crypto Stable contract`, next to the other Stable contract steps.

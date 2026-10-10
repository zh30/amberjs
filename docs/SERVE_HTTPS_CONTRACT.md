# `amber serve --https` contract

This is the user-facing contract for Stable `amber serve --https`. It is derived from `src/main.rs`, `src/https_serve.rs`, and the tests named below. Historical `docs/STAGE_*` reports are not part of this contract.

`amber serve --https` terminates TLS with rustls and serves one HTTP/1.1 request per connection. It is not a general-purpose web server, and it is not the Node.js `https` module.

Plain `amber serve` without `--https` is Stable under its own contract: [`docs/SERVE_HTTP_CONTRACT.md`](SERVE_HTTP_CONTRACT.md) (G41).

## Command

```bash
amber serve --https --cert CERT.pem --key KEY.pem [--host HOST] [--port PORT] [SCRIPT]
```

- `--https` requires both `--cert` and `--key`. Each path must be a regular file of at most 1 MiB.
- `--cert` without `--https`, or `--key` without `--https`, is a clap usage error. That error is not the `error: amber serve:` prefix.
- Default `--host` is `localhost`. Default `--port` is `3000`. Port `0` asks the OS for an ephemeral port. The listening line prints the bound address.
- Permission checks run before PEM parsing. `--deny-net` (and any other listen denial) prints the permission broker's `permission denied` diagnostic and exits non-zero. It does not print `Listening`.
- Every other contracted failure below exits **2**, prints `error: amber serve:` on stderr, and does not print `Starting Amber Web Server` or `Listening`.
- The process validates PEM and, when a script is selected, loads that script **before** it binds. A bad certificate or a script that fails to load never listens.

## TLS material

| Input | Rule |
| :--- | :--- |
| Certificate PEM | One or more X.509 certificates. The leaf is first, then intermediates. |
| Private key PEM | The first key block wins. Accepted formats: PKCS#8 (`BEGIN PRIVATE KEY`), PKCS#1 RSA (`BEGIN RSA PRIVATE KEY`), SEC1 EC (`BEGIN EC PRIVATE KEY`). |
| Match | rustls must accept the leaf and the key together. A mismatch exits 2 with `invalid TLS material`. |

Encrypted keys (`BEGIN ENCRYPTED PRIVATE KEY`, `Proc-Type: 4,ENCRYPTED`, or `DEK-Info:`) exit 2 with `encrypted private keys are not supported`. There is no passphrase flag.

The rustls server uses `with_safe_defaults` (TLS 1.2 and TLS 1.3), requests no client certificate, and advertises ALPN `http/1.1` only. A client that offers only `h2` fails the handshake. A client that offers `http/1.1`, or that offers no ALPN, can connect and must still speak HTTP/1.1.

## What is served

| Invocation | Behavior |
| :--- | :--- |
| No script argument, and none of `app.ts`, `app.js`, `server.ts`, `server.js`, `index.ts`, `index.js` exists in the working directory | Every **valid** HTTP/1.1 request gets `200` `application/json`: `{"runtime":"amberjs","ok":true,"version":"<crate version>"}\n`. The method and path are not interpreted. |
| Script argument, or the first existing file in that list | The file is executed as the fetch handler below. |

The health document is not a readiness probe framework. It is the default document when no script was selected.

## Fetch handler

The script runs once at startup inside a CommonJS-shaped wrapper (`module`, `exports`). The handler is chosen in this order:

1. `module.exports` when it is a function, or when it has `default` or `fetch`.
2. A script-local `function fetch` (it shadows the runtime client).
3. Otherwise `404` with body `Not Found: No fetch handler exported`.

`globalThis.fetchHandler` is also accepted by the bridge. The runtime's global `fetch` **client** is not used as the server handler.

Each request calls that handler with a `Request` whose URL is `https://<bound-address><target>`, whose method is the request method, and whose body is the request body interpreted as lossy UTF-8 (invalid bytes become U+FFFD). The handler's `Response` status, string headers, and text body are written back. A thrown handler becomes `500` with `Internal Server Error:` plus the message.

Response rules on the wire:

- The reason phrase comes from the status code (`200 OK`, `201 Created`, `404 Not Found`, `500 Internal Server Error`, and the phrases listed below). A non-200 response is not labeled `OK`.
- `Content-Length` is the UTF-8 byte length of the body. A handler-supplied content-length is replaced.
- `Connection: close` is always sent. Keep-alive is not honored. After the response bytes are flushed, the server sends a TLS `close_notify` and shuts down the TCP write side.
- Header names must be tokens. Header values containing CR, LF, or NUL are dropped, not rewritten.
- `HEAD` sends the same headers and `Content-Length` as the handler body, and no body bytes.
- A missing handler `status` defaults to 200. A status outside 100–599 becomes 500.

## HTTP/1.1 limits

One request per connection. Bytes after the body are discarded. The server reads the headers, then reads exactly `Content-Length` bytes if that header is present. There is no request body when `Content-Length` is absent.

| Limit | Value |
| :--- | :--- |
| Header block, including the final `\r\n\r\n` | 16 KiB (`16384` bytes). One byte over is rejected. |
| Header fields, including `Host` | 64. One more is rejected. |
| `Content-Length` | At most 1 MiB (`1048576` bytes). |
| PEM file | At most 1 MiB per file. |
| Socket read/write timeout | 10 seconds per accepted connection. |

Requests use CRLF. A LF-only header block is a bad request. The target must be origin-form (`/path` plus an optional query). `Host` is required exactly once. Methods are 1–20 ASCII uppercase letters.

| Request | Status | Body contains |
| :--- | :--- | :--- |
| Not HTTP/1.1 (`HTTP/1.0`, `HTTP/2.0`, including the `PRI * HTTP/2.0` preface) | 505 | `HTTP/1.1 required` |
| Malformed request line, lowercase method, NUL, obs-fold, or LF-only framing | 400 | `bad request` |
| Target is not origin-form | 400 | `origin-form target required` |
| Missing `Host` | 400 | `host header required` |
| More than one `Host` | 400 | `invalid host header` |
| Missing, duplicate, or non-decimal `Content-Length`, or `Content-Length` together with `Transfer-Encoding` | 400 | `invalid content-length` |
| Body shorter than `Content-Length` | 400 | `incomplete request body` |
| Any `Transfer-Encoding` | 501 | `transfer-encoding is not supported` |
| Any `Expect` | 417 | `expectation failed` |
| Headers larger than 16 KiB or more than 64 fields | 431 | `request headers too large` |
| `Content-Length` above 1 MiB | 413 | `request body too large` |
| Read timeout after the TLS handshake | 408 | `request timeout` |

A timeout during the TLS handshake closes the connection without an HTTP response. Handshake failures and accept errors do not exit the process. The accept loop runs until the process is stopped.

`Content-Length` above 1 MiB is rejected before the body is buffered. A number that does not fit in `u64` is `400`, not `413`.

## Stable failure diagnostics

Exit 2 and stderr contains `error: amber serve:`.

| Case | Body contains |
| :--- | :--- |
| `--https` without `--cert` | `--cert` |
| `--https` without `--key` | `--key` |
| Certificate path does not exist | `TLS certificate not found` |
| Key path does not exist | `TLS private key not found` |
| Certificate or key path is not a regular file | `is not a file` |
| Certificate PEM has no certificate | `no certificate found in PEM` |
| Key PEM has no private key | `no private key found in PEM` |
| Encrypted private key | `encrypted private keys are not supported` |
| PEM file larger than 1 MiB | `exceeds` |
| Key does not match the certificate, or rustls rejects the pair | `invalid TLS material` |
| Script cannot be read or fails before listen | `failed to load script` |
| Bind fails | `failed to bind` |

## Explicitly out of scope

- Plain `amber serve` without `--https` (Stable under G41 [`docs/SERVE_HTTP_CONTRACT.md`](SERVE_HTTP_CONTRACT.md); not this TLS contract)
- HTTP/2, HTTP/3, keep-alive, pipelining, and chunked bodies
- WebSocket, static files, directories, and reverse proxying
- Client certificates, SNI certificate selection, and encrypted or password-protected keys
- Streaming or binary-preserving request and response bodies
- More than one request per connection
- The Node.js `https` / `tls` compatibility modules (`src/nodejs_core/` is not this command)

## How this is enforced

```bash
cargo test --lib https_serve -- --test-threads=1
cargo test --test serve_https_contract_tests -- --test-threads=1
cargo test --test cli_serve_https_tests -- --test-threads=1
```

CI runs `serve_https_contract_tests` as its own step. `cargo test` also runs the `https_serve` unit tests and `cli_serve_https_tests`.

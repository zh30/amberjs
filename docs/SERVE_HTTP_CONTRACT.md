# `amber serve` (plain HTTP) contract

This is the user-facing contract for Stable plain `amber serve` (no `--https`) under **G41**. It is derived from `src/main.rs`, `src/https_serve.rs` (`serve_plain_connections`), and the tests named below. Historical `docs/STAGE_*` reports are not part of this contract.

Plain `amber serve` accepts cleartext TCP, speaks one HTTP/1.1 request per connection with the same framing limits as G7, and serves either a JSON health document or a script's fetch-handler bridge. It is not a general-purpose web server, not TLS, and not the Node.js `http` / `https` modules.

`amber serve --https` stays its own Stable contract: [`docs/SERVE_HTTPS_CONTRACT.md`](SERVE_HTTPS_CONTRACT.md) (G7). The two commands share HTTP/1.1 framing helpers; they do not merge into one product promise.

## Command

```bash
amber serve [--host HOST] [--port PORT] [SCRIPT]
```

- Default `--host` is `localhost`. Default `--port` is `3000`. Port `0` asks the OS for an ephemeral port. The listening line prints the bound address.
- Permission checks run before bind. `--deny-net` (and any other listen denial) prints the permission broker's `permission denied` diagnostic and exits non-zero. It does not print `Listening`.
- Every other contracted failure below exits **2**, prints `error: amber serve:` on stderr, and does not print `Starting Amber Web Server` or `Listening`.
- The process loads a selected script **before** it binds. A script that fails to load never listens.
- `--https`, `--cert`, and `--key` are outside this contract (see G7). `--cert` / `--key` without `--https` remain clap usage errors and do not use the `error: amber serve:` prefix.

## What is served

| Invocation | Behavior |
| :--- | :--- |
| No script argument, and none of `app.ts`, `app.js`, `server.ts`, `server.js`, `index.ts`, `index.js` exists in the working directory | Every **valid** HTTP/1.1 request gets `200` `application/json`: `{"runtime":"amberjs","ok":true,"version":"<crate version>"}\n`. The method and path are not interpreted. |
| Script argument, or the first existing file in that list | The file is executed as the fetch handler below. |

The health document is not a readiness probe framework. It is the default document when no script was selected.

## Fetch handler

The script runs once at startup inside a CommonJS-shaped wrapper (`module`, `exports`). Handler resolution matches G7 (`FETCH_BRIDGE` + `wrap_user_script`):

1. `module.exports` when it is a function, or when it has `default` or `fetch`.
2. A script-local `function fetch` (it shadows the runtime client; the global `fetch` client is not used as the server handler).
3. `globalThis.fetchHandler` when it is a function.
4. Otherwise `404` with body `Not Found: No fetch handler exported`.

Each request calls that handler with a `Request` whose URL is `http://<bound-address><target>`, whose method is the request method, and whose body is the request body interpreted as lossy UTF-8 (invalid bytes become U+FFFD). The handler's `Response` status, string headers, and text body are written back. A thrown handler becomes `500` with `Internal Server Error:` plus the message.

Response rules on the wire (same as G7 without TLS):

- The reason phrase comes from the status code (`200 OK`, `201 Created`, `404 Not Found`, `500 Internal Server Error`, and the phrases listed in the limits table). A non-200 response is not labeled `OK`.
- `Content-Length` is the UTF-8 byte length of the body. A handler-supplied content-length is replaced.
- `Connection: close` is always sent. Keep-alive is not honored. After the response bytes are flushed, the server shuts down the TCP write side.
- Header names must be tokens. Header values containing CR, LF, or NUL are dropped, not rewritten.
- `HEAD` sends the same headers and `Content-Length` as the handler body, and no body bytes.
- A missing handler `status` defaults to 200. A status outside 100–599 becomes 500.

## HTTP/1.1 limits

Framing, rejection statuses, and numeric caps are the same cleartext HTTP/1.1 stack used by G7 (without rustls). One request per connection. Bytes after the body are discarded. The server reads the headers, then reads exactly `Content-Length` bytes if that header is present. There is no request body when `Content-Length` is absent.

| Limit | Value |
| :--- | :--- |
| Header block, including the final `\r\n\r\n` | 16 KiB (`16384` bytes). One byte over is rejected. |
| Header fields, including `Host` | 64. One more is rejected. |
| `Content-Length` | At most 1 MiB (`1048576` bytes). |
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
| Read timeout | 408 | `request timeout` |

Accept errors do not exit the process. The accept loop runs until the process is stopped.

`Content-Length` above 1 MiB is rejected before the body is buffered. A number that does not fit in `u64` is `400`, not `413`.

## Stable failure diagnostics

Exit 2 and stderr contains `error: amber serve:`.

| Case | Body contains |
| :--- | :--- |
| Script cannot be read or fails before listen | `failed to load script` |
| Bind fails | `failed to bind` |

## Explicitly out of scope (Non-goals)

These are not Stable Limits fiction; they are outside this product surface:

- `amber serve --https` / TLS / PEM / ALPN (Stable under G7)
- Node.js `http.createServer` (Stable under G37 for its own contract), Node `https` / `http2` / `net` listen
- HTTP/2, HTTP/3, keep-alive, pipelining, and chunked bodies
- WebSocket, static files, directories, and reverse proxying
- Streaming or binary-preserving request and response bodies
- More than one request per connection
- Claiming tiny_http keep-alive or framing behavior (this path does not use `tiny_http`)

## How this is enforced

```bash
cargo test --lib https_serve -- --test-threads=1
cargo test --test serve_http_contract_tests -- --test-threads=1
```

CI runs `serve_http_contract_tests` as the step `amber serve Stable contract`. `cargo test` also runs the `https_serve` unit tests (shared framing).

# Node `http` server contract

This is the user-facing contract for a **narrow** Stable subset of Node `http` in Amber: **server listen / bind / accept / request→handler / buffered response only**. It is derived from `src/nodejs_core/http.rs` (`setup_http_api`, `http_server_listen_callback`, `run_http_server`, response buffering / `generate_http_response_v2`), `src/runtime_minimal.rs` (`install_core_apis` → `setup_http_api`; CJS `require('http')`), and `tests/http_server_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

This is **not** full Node `http`. It is **not** the CLI `amber serve --https` surface (G7, [`docs/SERVE_HTTPS_CONTRACT.md`](SERVE_HTTPS_CONTRACT.md)). It is **not** Node `https`, `http2`, or `net`.

**Honesty:** `http.request` / `http.get` fail-open to status `200` on error, Agent/pool mocks, and the thin `https` wrap are **Non-goals**. They must never be graduated by renaming those lies as Stable Limits.

## Stable surface

| Call / field | Behavior |
| :--- | :--- |
| `require('http')` / `require('node:http')` | Same object installed by `setup_http_api` on `globalThis.http`. |
| `http.createServer([options,] handler)` | Returns a server object. When `handler` (or a function `options`) is a function, it is registered as the `'request'` listener and stored for dispatch (`globalThis._httpServerRequestHandler`). |
| `server.listen(port[, host][, cb])` | Real `TcpListener::bind` + Tokio accept loop (`run_http_server`). Default host when omitted is `"0.0.0.0"`. Default port when omitted / unparsable is `3000`. Port `0` asks the OS for an ephemeral port; `server.address().port` reflects the bound port. Emits `'listening'` and invokes `cb` when provided (via `once('listening', …)`). Returns `server`. |
| `server.listen({ port, host }[, cb])` | Same bind/accept path. Options default: `port` → `3000`, `host` → `"0.0.0.0"`. |
| `server.address()` | After a successful listen: `{ port, address, family }` where `address` is the host string used to bind and `family` is `"IPv6"` when that host looks like IPv6, otherwise `"IPv4"`. |
| `server.close()` | Clears the listening flag / stops the accept loop registration for that host+port; sets `listening` to `false`; returns `server`. |
| `server.listening` | Boolean; `true` after listen succeeds on the JS object, `false` after `close` or listen permission denial. |
| Request → handler | Accepted HTTP/1.1 connection is parsed and dispatched to the request handler as `(req, res)`. `req` exposes at least `method`, `url` / `path`, `headers`, and body delivery via `'data'` / `'end'` (body string when present). |
| `res.setHeader(name, value)` / `getHeader` / `hasHeader` / `removeHeader` | Mutate the response header map before the buffered write. |
| `res.writeHead(statusCode[, statusMessage][, headers])` | Sets `statusCode` (and optional reason / headers). Does not flush bytes to the wire by itself. |
| `res.write(chunk)` | Appends `String(chunk)` to an in-memory body buffer. Returns `true`. Does **not** write a chunked frame to the socket. |
| `res.end([chunk])` | Optional final chunk append, then one HTTP/1.1 response is written with `Content-Length` equal to the buffered body byte length (plus caller headers / status). |
| Missing dispatcher / handler | If no request handler is registered, or the accept-path dispatcher is unavailable, when a request arrives the server responds **503** with a plain body (fail-closed). It does not invent a fake 200. |

Keep-alive: the accept path can serve more than one HTTP/1.1 request on the same connection when keep-alive is negotiated (`Connection: keep-alive` / HTTP/1.1 defaults). That is part of this server surface; it is not an Agent pool claim.

## Limits

These are real behaviors of the Stable server surface, not cover for client/`https` stubs:

- Response bodies are **fully buffered** until `end`. `write` only concatenates into `_body`. There is no contracted chunked transfer encoding or Node `stream.Writable` backpressure on `ServerResponse`.
- `req.socket` / `connection` may be a placeholder object (fixed remote address shape) rather than a live Node `net.Socket` API.
- Listen starts the accept thread and briefly waits (~5 ms) before advertising the bound port; callers should still use `address()` / the listening callback, not assume sync readiness of peers.
- Bind failures log and clear `listening`; they are not a fully Node-shaped `error` event contract on this page.
- Permission broker: listen checks Network/Listen for the host+port. Denial throws before bind (message contains `permission denied`) and leaves `listening === false`.
- ESM `import` from `'http'` / `'node:http'` is **not** wired in `normalized_esm_builtin_name` today — only CJS `require` is on this contract.
- Express-shaped helpers on `ServerResponse` (`status`, `json`, `send`, …) may exist on the object; they are **not** pinned Stable here. Only the buffered `writeHead` / `setHeader` / `write` / `end` path is.

## Non-goals

Outside this contract (Preview / DEFER / unimplemented). Do **not** treat these as Stable Limits of dishonest behavior:

- **`http.request` / `http.get` client** — today fail-open: transport errors can surface as status `200` / empty body. Never graduate that as a Limit.
- **`http.Agent` / connection pool** — counters and mock `createConnection` (`"[Socket connected]"`); no real pooled `TcpStream`.
- **Node `https` module** — thin wrap copying `http` client/Agent plus optional TLS PEM fields; inherits client fail-open. Not G7 CLI `amber serve --https`.
- **`http2`** — JS stub constructors; no framing.
- **`require('net')` server `listen`** — flag-only (no bind/accept). Separate NEEDS_IMPL; not this page.
- Chunked / true streaming wire writes, Trailers, `CONNECT`, upgrade-as-Stable (WebSocket upgrade path on the same listener exists but is uncontracted).
- Full Node `IncomingMessage` / `ServerResponse` stream compatibility, `http.Server` EventEmitter parity beyond listen/request/close as exercised above.
- Promoting client, Agent, `https`, or `http2` by listing them under Limits.

## Reachability

The CLI `amber` binary installs `globalThis.http` from `src/runtime_minimal.rs` via `nodejs_core::http::setup_http_api`. `require('http')` / `require('node:http')` return that object through the CommonJS builtin arm. Library users reach the installer through `amberjs::nodejs_core::http::setup_http_api` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.

## Tests

```bash
cargo test --test http_server_contract_tests -- --test-threads=1
```

CI step: `node http server Stable contract`.

## Honesty rule

Fail-open client `200`, Agent mocks, and `https` thin wrap are **Non-goals**. Limits describe buffered server I/O and listen quirks only. Graduating “HTTP” as a bag is forbidden.

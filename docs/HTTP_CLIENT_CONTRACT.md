# Node `http` client contract

This is the user-facing contract for a **narrow** Stable subset of the Node `http` **client** in Amber: **`http.request` / `http.get` over real HTTP/1.1 with a buffered response delivered as `IncomingMessage`-shaped events**. It is derived from `src/nodejs_core/http.rs` (`http_request_callback`, `http_get_callback`, `http_req_end_callback`, `install_http_event_methods`, `install_http_res_pipe`), `src/nodejs_core/tcp_async.rs` (`sync_http_request`, `parse_http_response`), and `tests/http_client_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

This is **not** full Node `http` client. It is **not** Node `https` client, `http2`, or the server surface — the server has its own page, [`docs/HTTP_SERVER_CONTRACT.md`](HTTP_SERVER_CONTRACT.md) (G37).

**Honesty:** before G72 the client was fail-open — transport errors surfaced as a fabricated `200 OK` with an empty body, and request headers were silently dropped. Both are fixed on this page: failures surface as an `'error'` event; there is **no fake response path**.

## Stable surface

| Call / field | Behavior |
| :--- | :--- |
| `http.request(options[, cb])` / `http.request(url[, cb])` | Builds a `ClientRequest` object. `options`: `{ method?, hostname?, port?, path?, headers? }`. `url` accepts `'http://host[:port][/path]'` strings. `cb(res)` is stored as the response callback. The request is **not** sent until `req.end()`. |
| `http.get(options[, cb])` / `http.get(url[, cb])` | Same as `request` with `method` fixed to `GET` and the request **sent immediately** — Node semantics. Because the client is synchronous-buffered, the whole response sequence (see Ordering) runs inside the `get()` call. |
| `req.write(chunk)` | Appends `String(chunk)` to the request body buffer. |
| `req.end([chunk][, cb])` | Optional final chunk append, then sends the request over a real `TcpStream` and waits for the full response (10 s timeout). `cb` runs once after the request completes (see Ordering). |
| `req.on('response', fn)` / `req.once(...)` | Registers the `'response'` listener into `req._events` (single-slot model, same convention as `stream` contract listeners — not an `EventEmitter` multiplexer). Fires with the `res` object when a response arrives. |
| `req.on('error', fn)` | Registers the `'error'` listener. Fires with `{ name: 'Error', message }` when the transport fails (connect refused, timeout, malformed response). **Never** produces a fabricated `200`. |
| `res.statusCode` / `res.statusMessage` | Status line of the real response. |
| `res.headers` | Plain object of response headers with **lowercased names** (Node-normalized). |
| `res.body` | Full response body string (kept for compatibility). |
| `res.on('data', fn)` / `res.on('end', fn)` | IncomingMessage events. `'data'` fires once with the whole body as a string chunk, then `'end'` fires once. When the body is empty, only `'end'` fires. |
| `res.setEncoding('utf8')` | Stores the encoding marker; `'data'` chunks are always strings on this page. Returns `res`. |
| `res.pipe(dest)` | Wires `'data' → dest.write(chunk, 'utf8', noop)` and `'end' → dest.end()` — same forwarder model as the `stream` pipe contract (G24/G71). Returns `dest`. |

Ordering inside `req.end(...)`: user end-callback → `req` `'response'` event → stored response callback → `res` `'data'` → `res` `'end'`. For `http.get`, all of that happens inside `get()`.

Non-UTF-8 bodies: `'data'` delivers the string `"[binary data]"`; `res.body` holds the same marker. Binary-safe body APIs are Preview.

## Limits

- **Synchronous buffered model**: the request blocks `end()` (or `get()`) until the response completes or times out (10 s). `'data'` is a single chunk — there is no incremental streaming, no `AbortSignal`, no mid-flight `req.abort()`.
- `http.get` sends immediately: a `'response'` listener attached **after** `get()` returns misses the event (the response was already delivered). Use the `cb` argument, or `http.request` when you need to attach listeners first.
- `req.end(cb)`'s `cb` is called after the request completes (buffered model has no distinct `'finish'` timing); it is not Node's `'finish'` event.
- An unhandled `'error'` logs to stderr instead of throwing (there is no `'error'` throw contract yet).
- `Connection: close` is always sent; keep-alive / response `Agent` pooling across requests is not contracted (the `_poolStats` field is informational only).
- `https://` URLs and the `https` module are **fail-closed Preview** — an `https://` URL string marks the request and `end()` emits `'error'` (`"https:// URLs stay Preview"`); it never silently downgrades to `http` or to the default `localhost:80` target.
- Responses are capped at **64 MiB**; exceeding it fails with `'error'` instead of exhausting memory.
- Request header names/values containing `\r` or `\n` are rejected (`'error'`) — no CRLF header injection.
- A malformed status line (not `HTTP/…`) or a connection that errors/times out before the response is complete (per `Content-Length`, or before EOF when no `Content-Length`) is `'error'`, never a delivered response.
- A client request to a G37 `http` server **in the same runtime** cannot complete: the synchronous read blocks the V8 dispatcher that would dispatch the server-side request, so it times out into `'error'`. Use a separate process or a Rust/JS bridge instead.
- Chunked / `Transfer-Encoding` responses, trailers, `CONNECT`, redirect following, `ClientRequest#abort`, `socket`, and ESM `import 'http'` are outside this page.

## Non-goals

Outside this contract (Preview / DEFER / unimplemented):

- `http.Agent` / connection pooling behavior, `maxSockets`, `keepAlive` agents — counters exist but are not a real pooled `TcpStream` contract.
- `https.request` / `https.get` — TLS is not implemented; the thin `https` wrapper inherits nothing from this page.
- Streaming/`IncomingMessage` as a real `stream.Readable` (beyond `'data'`/`'end'`/`pipe`/`setEncoding` above), `res.pause`/`resume`, backpressure.
- Fetch-style redirects, cookies, compression negotiation, request timeouts as a JS option (`timeout:` option, `req.setTimeout`).
- Web API `fetch()` — a separate surface (`src/web_api/`).

## Tests

`cargo test --test http_client_contract_tests -- --test-threads=1` — real wire tests against a Rust-side `TcpListener` server: status/headers parse + `'data'`/`'end'`, custom request headers + body on the wire, `'response'` event + `res.pipe` to a `Writable`, `'error'` on connection refused (asserts no fake 200), object-form options + `end(cb)` ordering, `http.get` auto-send.

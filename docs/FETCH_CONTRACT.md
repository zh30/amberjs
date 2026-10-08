# Web `fetch` contract

This is the user-facing contract for Stable Web `fetch` in the default Amber runtime. It is derived from `src/web_api/fetch.rs`, `src/web_api/abort.rs`, and `tests/fetch_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`fetch` is reachable from `amber run` / `amber eval` as `globalThis.fetch`, together with `Request`, `Response`, and `Headers`. The rest of `src/web_api/` stays Preview. This is not the Fetch standard, and it is not Node or browser compatibility.

## Response body

`response.body` is a pull reader, not a string of the whole payload.

- `response.body.getReader()` returns a reader. A second call throws `body stream is locked`. Calling `getReader()` sets `bodyUsed`.
- `reader.read()` returns `{ done, value }` synchronously. `value` is a `Uint8Array` of the next bytes already received, plus at most one further socket read. It does not wait for `Content-Length` to finish.
- `text()`, `json()`, `arrayBuffer()`, and `blob()` are synchronous. They read the unread remainder and therefore buffer at the moment they are called. A second consume throws `Response body already consumed`.
- `fetch()` itself returns after response headers. Bytes that happened to arrive in the same packet as the headers are kept. The rest stays on the connection until a read.
- `response.body` is not `instanceof ReadableStream`. It has no `pipeTo` or `pipeThrough`.

An empty `integrity` leaves the final body unread. A non-empty `integrity` reads that hop's body completely so the digest can be checked before `fetch` returns.

## Headers

Response headers keep repeated names. Two `Set-Cookie` fields stay two fields.

- `headers.getSetCookie()` returns each `Set-Cookie` value, in order.
- `headers.get(name)` joins every value for that name with `", "`.
- `headers.append` is how a fetched response is filled. `headers.set` still replaces one name.

Request headers are a single map. Sending two request headers with the same name is not part of this contract.

## Redirect

`redirect` is `follow` (default), `manual`, or `error`. Anything else throws `TypeError` (`Invalid redirect mode`) before connect.

| Mode | Result |
| :--- | :--- |
| `follow` | `301`, `302`, `303`, `307`, and `308` with a `Location` are followed, up to 20 hops. `301`/`302` rewrite a non-GET/HEAD method to GET and drop the body. `303` rewrites everything except HEAD to GET and drops the body. `307`/`308` keep the method and body. The final response has `redirected === true` after at least one hop. The next URL is resolved against the current URL. |
| `manual` | The 3xx is returned. `status` is the redirect status, `redirected` is false, and `Location` is readable. The redirect target is not requested. This is not a browser opaque-redirect (`status` is not forced to 0). |
| `error` | A redirect status rejects with `redirect not allowed`. |

A loop past 20 hops rejects with `Too many redirects`. A cross-origin hop drops `Authorization`, `Cookie`, `Cookie2`, and `Proxy-Authorization` from the next request.

## `AbortSignal` and `integrity` on every hop

Both are read from `fetch(input, init)` and, when present, from a `Request`. They are applied again on every redirect hop, not only the first request.

- Before each hop, and between body chunks of that hop, an aborted signal rejects with `The operation was aborted`. `controller.abort()` before `fetch` prevents the first connection. An abort that becomes visible while a hop is in flight (including `abort_in_flight_fetch` from another thread) stops the loop before the next hop.
- Abort does not cancel a connect syscall that is already blocked, and it does not apply to `read()` / `text()` after `fetch` has already returned.
- `integrity` is Subresource Integrity metadata: whitespace-separated `sha256-`, `sha384-`, or `sha512-` tokens, base64 (standard alphabet). One matching token succeeds. A supported token that does not match rejects with `integrity mismatch`. No supported token rejects with `Unsupported integrity algorithm`.
- The digest is the body of that hop. A redirect body that does not match rejects before the next request. The final body is checked the same way. Empty `integrity` skips the check.

## CORS

Amber has no document origin.

| `mode` | Behavior |
| :--- | :--- |
| omitted | Not filtered. Custom response headers and every `Set-Cookie` stay visible. `Request` defaults its `mode` property to `"cors"` but that default is not treated as an explicit mode. |
| `"cors"` | Response headers are filtered to the CORS-safelisted names (`cache-control`, `content-language`, `content-length`, `content-type`, `expires`, `last-modified`, `pragma`), `access-control-*`, and names listed in `Access-Control-Expose-Headers` (`*` exposes the rest). `Set-Cookie` and `Set-Cookie2` are always removed. `response.type` is `"cors"`. The request still succeeds when `Access-Control-Allow-Origin` is absent. There is no preflight. |
| `"no-cors"` | Opaque result: `type` is `"opaque"`, `status` is 0, headers are empty, the body is empty, and `url` is empty. |
| `"same-origin"` | Allowed, and not filtered. It does not reject a cross-origin URL. |
| anything else | `TypeError` (`Invalid request mode`) before connect. |

## Non-goals

- Cookie jar, credentialed CORS, or CORS preflight.
- HTTP cache (`cache` is stored on `Request` and not implemented here).
- Upload streaming. Request bodies are buffered.
- `ReadableStream` piping on `response.body`.
- Promises from `text()` / `json()` / `arrayBuffer()` / `blob()`.
- Browser `opaqueredirect`.
- Cancelling an in-progress TCP connect, or aborting a body read after `fetch` returns.

## Tests

```bash
cargo test --test fetch_contract_tests -- --test-threads=1
```

CI runs that command as `web fetch Stable contract`, next to the other Stable contract steps.

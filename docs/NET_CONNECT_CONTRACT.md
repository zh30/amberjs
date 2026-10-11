# Node `net.connect` client contract

This is the user-visible contract for a **narrow** Stable subset of Node `net` **client** sockets in Amber: **`net.connect` / `createConnection` → real TCP connect → `'connect'` → `'data'` / `write` / `end`**. It is derived from `src/nodejs_core/net.rs` (`net_connect_callback`, `pump_pending_socket_events_in_scope`, client Socket EventEmitter bootstrap), `src/nodejs_core/tcp_async.rs` (`connect`, `start_reading`, `sync_write`, `shutdown_write`), `src/runtime_minimal.rs` (socket event pump + keep-alive), and `tests/net_connect_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

This is **not** full Node `net`. It is **not** the G45 `net` **server** listen/accept surface (separate page). It is **not** Node `http` client / Agent / `https` / `http2` (those stay lasting DEFER / Non-goals — never graduate fail-open client fiction here).

**Honesty:** G45 listed `net.connect` client rewrite as a Non-goal. This page graduates only the narrow connect + `'connect'` / `'data'` / `write` / `end` pin. Do **not** graduate TLS, unix sockets, full duplex stream parity, or http client by listing them as Limits.

## Stable surface

| Call / field | Behavior |
| :--- | :--- |
| `require('net')` / `require('node:net')` | Same object installed by `setup_net_api` on `globalThis.net` (shared with G45 server). |
| `net.connect(port[, host][, connectListener])` | Real TCP connect to `host` (default `"localhost"`) and `port`. Returns a Socket whose prototype is EventEmitter-backed. When `connectListener` is a function, it is registered as `once('connect', …)`. |
| `net.connect(options[, connectListener])` | Same path. Options: `port`, `host` (default `"localhost"`), optional `connectTimeout` (seconds; default **10** when omitted / `0`). |
| `net.createConnection(...)` | Alias of `net.connect` (same wrapped function shape). |
| `'connect'` | Emitted after a successful TCP connect (via the runtime socket-event pump). `connecting` becomes `false`. |
| `'data'` | Emitted with a UTF-8 lossy string chunk when bytes arrive on the TCP stream (background `try_read` → buffer → pump). |
| `socket.write(chunk)` | Writes `String(chunk)` bytes on the live TCP stream. Returns `true`. |
| `socket.end([chunk])` | Optional final `write`, then half-closes the write side (`shutdown`). Returns `socket`. |
| `socket.destroy()` | Stops reading, closes the stream, unregisters keep-alive / pump registry entry. |
| Address fields | After successful connect: live `remoteAddress` / `remotePort` / `localAddress` / `localPort` / `remoteFamily` from the OS peer/local addrs. |
| Permission denial | Connect checks Network/Connect for `tcp://host:port`. Denial throws before connect (message contains `permission denied` / `permission`). |

Wire proof required by this contract: a peer TCP listener accepts the Amber client; the client sees `'connect'`, `write`s bytes the peer receives, and receives peer bytes on `'data'`.

## Limits

These are real behaviors of the Stable client surface, not cover for missing emit:

- TCP connect is **synchronous on the isolate thread** (`block_on` with timeout) before `connect` returns; `'connect'` is delivered on the next socket-event pump after listeners are attached (including `connectListener`).
- `'data'` chunks are UTF-8 lossy strings (same string write path), not `Buffer` objects.
- Failed connect (timeout / refused) returns a Socket with `connecting === true` and does **not** enqueue `'connect'`; a full Node-shaped `'error'` event contract is not pinned here.
- Open client sockets keep CLI `amber run` keep-alive until `destroy()`, write-ended + read-finished, or process exit.
- ESM `import` from `'net'` / `'node:net'` is **not** wired — only CJS `require` is on this contract.
- `new net.Socket()` without `connect` is not pinned.

## Non-goals

Outside this contract (Preview / DEFER / unimplemented). Do **not** treat these as Stable Limits of dishonest behavior:

- Node `http.request` / `http.get` client, `http.Agent` / pool mocks, Node `https`, `http2` (lasting DEFER; fail-open client must never become Stable Limits).
- TLS / `tls.connect`, unix domain sockets (`connect(path)` / `{ path }` throws `TypeError` naming unix sockets).
- Full Node `net.Socket` duplex stream parity (`pause` / `resume` backpressure, cork/uncork, `setNoDelay` contract, `'end'` / `'close'` / `'timeout'` event product pins, `Buffer` data chunks).
- Promoting server listen (G45) or http framing by expanding this page.

## Reachability

The CLI `amber` binary installs `globalThis.net` from `src/runtime_minimal.rs` via `nodejs_core::net::setup_net_api`. `require('net')` / `require('node:net')` return that object through the CommonJS builtin arm. Library users reach the installer through `amberjs::nodejs_core::net::setup_net_api`. The contract is not feature-gated.

## Tests

```bash
cargo test --test net_connect_contract_tests -- --test-threads=1
```

CI step: `node net connect Stable contract`.

## Honesty rule

A Socket that only sets a string `connect` property (or a single-listener stub `on` without TCP `'data'` delivery) must never be graduated as Stable Limits. Limits describe sync connect timing and UTF-8 string chunks only.

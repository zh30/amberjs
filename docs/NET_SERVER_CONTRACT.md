# Node `net` server contract

This is the user-visible contract for a **narrow** Stable subset of Node `net` in Amber: **`createServer` → real `listen` / bind / accept → `'connection'` with a live Socket handle → `socket.write` bytes**. It is derived from `src/nodejs_core/net.rs` (`setup_net_api`, `server_listen_callback`, `run_net_server`, `pump_pending_net_connections_in_scope`), `src/nodejs_core/tcp_async.rs` (`adopt_stream`, `sync_write`), `src/runtime_minimal.rs` (net pump + keep-alive), and `tests/net_server_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

This is **not** full Node `net`. It is **not** the G37 `http` server (which has its own accept path). It is **not** TLS / `tls` / unix domain sockets / cluster.

**Honesty:** The previous flag-only `listen` (set `listening` + fabricate `address` without `TcpListener::bind`) is gone for this surface. Do **not** graduate keep-alive, full Socket EventEmitter parity, or client `net.connect` rewrite by listing missing pieces as Limits.

## Stable surface

| Call / field | Behavior |
| :--- | :--- |
| `require('net')` / `require('node:net')` | Same object installed by `setup_net_api` on `globalThis.net`. |
| `net.createServer([options,] connectionListener)` | Returns a server object whose prototype is EventEmitter-backed. When `connectionListener` (or a function `options`) is a function, it is registered as a `'connection'` listener. |
| `server.listen(port[, host][, cb])` | Real `TcpListener::bind` + Tokio accept loop (`run_net_server`). Default host when omitted is `"0.0.0.0"`. Default / omitted port is `0` (OS ephemeral). Emits `'listening'` and invokes `cb` when provided (via `once('listening', …)`). Returns `server`. |
| `server.listen({ port, host }[, cb])` | Same bind/accept path. Options default: `port` → `0`, `host` → `"0.0.0.0"`. |
| `server.listen(cb)` | Same as `listen(0, cb)` (ephemeral port). |
| `server.address()` | After a successful listen: `{ port, address, family }` where `address` is the host string used to bind, `port` is the OS-bound port (including ephemeral), and `family` is `"IPv6"` when that host looks like IPv6, otherwise `"IPv4"`. Before listen / after close: `null`. |
| `server.close()` | Clears the listening flag / stops the accept loop registration for that server id; sets `listening` to `false`; emits `'close'`; returns `server`. |
| `server.listening` | Boolean; `true` after listen succeeds on the JS object, `false` after `close` or listen permission / bind failure. |
| `server.on('connection', …)` / createServer listener | Accept delivers a Socket with live `_handleId`, real `remoteAddress` / `remotePort` / `localAddress` / `localPort` / `remoteFamily`. |
| `socket.write(chunk)` | Writes `String(chunk)` bytes on the accepted TCP stream (shared net Tokio runtime). Returns `true`. |
| Permission denial | Listen checks Network/Listen for the host+port. Denial throws before bind (message contains `permission denied` / `permission`) and leaves `listening === false`. |

Wire proof required by this contract: a peer `TcpStream::connect` to the bound port receives bytes written by the `'connection'` handler via `socket.write`.

## Limits

These are real behaviors of the Stable server surface, not cover for missing emit / bind:

- Listen starts the accept thread and briefly waits (~5 ms) before advertising the bound port; callers should still use `address()` / the listening callback.
- Bind failures clear `listening` and throw from `listen` when detected in that window; they are not a full Node-shaped `error` event contract on this page.
- Accepted Socket `'data'` / `'end'` EventEmitter delivery, `pause` / `resume` backpressure, and `socket.end` half-close are **not** pinned here. The Stable pin is accept → `'connection'` → `socket.write` (and address fields).
- `server.getConnections` may report `0`; it is not a live connection counter.
- ESM `import` from `'net'` / `'node:net'` is **not** wired in `normalized_esm_builtin_name` today — only CJS `require` is on this contract.
- `new net.Server(...)` without going through `createServer` is not pinned (use `createServer`).

## Non-goals

Outside this contract (Preview / DEFER / unimplemented). Do **not** treat these as Stable Limits of dishonest behavior:

- Full Node `net.Socket` duplex stream parity (`'data'` pump, cork/uncork, `setNoDelay` contract, etc.).
- `net.connect` / `createConnection` client rewrite (existing Preview client path stays Preview).
- TLS / `tls.createServer`, unix domain sockets, `server.listen({ fd })`, `cluster` / share handles.
- HTTP framing (use G37 `http`), keep-alive HTTP semantics, WebSocket upgrade.
- Promoting client connect, TLS, or unix sockets by listing them under Limits.

## Reachability

The CLI `amber` binary installs `globalThis.net` from `src/runtime_minimal.rs` via `nodejs_core::net::setup_net_api`. `require('net')` / `require('node:net')` return that object through the CommonJS builtin arm. Library users reach the installer through `amberjs::nodejs_core::net::setup_net_api`. The contract is not feature-gated.

## Tests

```bash
cargo test --test net_server_contract_tests -- --test-threads=1
```

CI step: `node net server Stable contract`.

## Honesty rule

Flag-only listen (no bind / no accept / no-op `on`/`emit`) must never be graduated as Stable Limits. Limits describe bind timing and the narrow write pin only.

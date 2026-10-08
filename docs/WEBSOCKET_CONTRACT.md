# Web `WebSocket` contract

This is the user-facing contract for the Stable WebSocket **client** in the default Amber runtime. It is derived from `src/web_api/websocket.rs`, `src/runtime_minimal.rs`, and `tests/websocket_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`WebSocket` is reachable from `amber run` / `amber eval` as `globalThis.WebSocket`. The rest of `src/web_api/` stays Preview. This is not the WebSocket standard, and it is not a WebSocket server. `src/server/websocket.rs` is not part of the default build and is not this contract.

## Connect, send, message, close, error

`new WebSocket(url)` starts a `ws://` handshake on a background task. `readyState` is `0` (`CONNECTING`) until the handshake finishes.

| Event | When | What the handler sees |
| :--- | :--- | :--- |
| `onopen` | The handshake completed. `readyState` is `1` (`OPEN`). | `{ type: "open" }` |
| `onmessage` | A data frame arrived. | `{ type: "message", data }`. Text is a string. Binary is an `ArrayBuffer` (the default `binaryType` is `"arraybuffer"`). |
| `onerror` | The connect, read, or send failed. | An `ErrorEvent` with `type === "error"` and `message`. A refused connect uses `Connection failed: ...`. |
| `onclose` | The socket is finished. `readyState` is `3` (`CLOSED`). | `{ type: "close", code, reason, wasClean }`. `wasClean` is true only for code `1000`. |

A refused connect fires `onerror` and then `onclose` with code `1006` and `wasClean === false`. No close frame is sent on the wire for that path.

`send(string)` queues a text frame. `send(ArrayBuffer)` and `send(Uint8Array)` queue a binary frame. `send` while `readyState` is not `OPEN` throws `Error` whose message contains `not open`. `bufferedAmount` stays `0`; the contract does not track queued bytes.

`close(code, reason)` sends a close frame and moves to `CLOSING` (`2`). The peer's close frame becomes `onclose`. If the peer does not answer within 2 seconds, `onclose` still runs with the code and reason passed to `close`. `close()` with no arguments uses code `1000` and an empty reason. Calling `close` again while `CLOSING` or `CLOSED` does nothing.

`addEventListener(type, fn)` stores that function as `onopen` / `onmessage` / `onclose` / `onerror`. A later assignment replaces the previous one. `removeEventListener(type)` clears that handler.

`ws instanceof WebSocket` is true. `WebSocket.CONNECTING`, `OPEN`, `CLOSING`, and `CLOSED` are `0`, `1`, `2`, and `3`, and the same values are on the prototype (`ws.OPEN === 1`).

The runtime delivers these handlers on its event loop. An unfinished socket from the current script keeps `amber run` / `execute_code` alive until `onclose`. A later script does not inherit the previous script's sockets.

## Protocol limits

These limits are part of the Stable contract. The tests pin them against a local socket.

- **No `permessage-deflate`.** The client does not offer `Sec-WebSocket-Extensions`. `ws.extensions` stays `""`. A server that requires compression is outside this contract.
- **No subprotocols.** `new WebSocket(url, protocols)` throws `TypeError` (`WebSocket protocols are not supported`) before connect. `ws.protocol` stays `""`.
- **Ping and pong are not message events.** The client answers a ping with a pong and does not call `onmessage` for either frame.
- **`binaryType = "blob"` is not part of this contract.** Binary data is delivered as an `ArrayBuffer`.
- **`wss://` is not pinned.** The constructor accepts a `wss://` URL. Certificate checks, ALPN, and TLS failure text are not part of this contract. The pinned tests use `ws://` on `127.0.0.1`.
- **One handler per event name.** This is not a full `EventTarget` listener list.
- **No WebSocket server** in this contract.

A non-`ws` / non-`wss` URL throws `Error` (`Invalid WebSocket URL`) before connect. `WebSocket(...)` without `new` throws `TypeError`.

## Non-goals

- Browser `EventTarget` dispatch (capture, `stopPropagation`, multiple listeners).
- `permessage-deflate` and any other extension.
- Subprotocol negotiation.
- A tracked `bufferedAmount`.
- Fragmented messages beyond what tokio-tungstenite 0.21 reassembles.
- A listening WebSocket server, including hot-reload sockets used by `amber run --watch`.

## Tests

```bash
cargo test --test websocket_contract_tests -- --test-threads=1
```

CI runs that command as `web websocket Stable contract`, next to the other Stable contract steps.

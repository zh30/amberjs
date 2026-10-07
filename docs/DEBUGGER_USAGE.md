# Amber inspector

The public inspector is `amber run --inspect` / `amber run --inspect-brk`, not `amber debug`.
`amber debug <file>` still exists as an experimental extra-diagnostics command; it is not the CDP attach path.

The Stable contract is [`INSPECT_CONTRACT.md`](INSPECT_CONTRACT.md). Default CDP port is **9229**, bound to `127.0.0.1` only.

## Commands

```bash
amber run --inspect script.js
amber run --inspect-brk script.js
amber run --inspect-brk --inspect-port 9229 app.ts
```

`--inspect` starts the CDP HTTP/WebSocket agent and runs the script.
`--inspect-brk` does the same but **does not execute user code** until the client sends `Runtime.runIfWaitingForDebugger` or `Debugger.resume`.

Discovery:

```text
GET http://127.0.0.1:9229/json/version
GET http://127.0.0.1:9229/json/list
ws://127.0.0.1:9229/ws
```

`/json/version` reports `Browser: Amber/<package version>` and `Protocol-Version: 1.3`.
`Runtime.evaluate` runs on the V8 isolate (for example `1+1` → `2`).
Other CDP methods, including `Debugger.enable`, stepping, and `Debugger.setBreakpoint`, return JSON-RPC `-32601`.

`Runtime.evaluate` is serviced while `--inspect-brk` is paused, and after the user script yields to the event loop. It does not preempt a synchronous JavaScript turn: the `v8` 152.2.0 interrupt callback must not reenter the isolate. There is no scope chain and no breakpoint list. This is not Chrome DevTools and not a VS Code Node debug adapter.

## Attach

1. `amber run --inspect-brk --inspect-port 9229 app.js`
2. `GET /json/version`, then connect to `ws://127.0.0.1:9229/ws`
3. Optional: `Runtime.evaluate` while paused
4. Send `Runtime.runIfWaitingForDebugger` or `Debugger.resume`

A client that expects the Node inspector handshake (`Debugger.enable`, breakpoints, scopes) is outside this contract.

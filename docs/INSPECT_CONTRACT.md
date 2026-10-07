# `amber run --inspect` contract

This is the user-facing contract for Stable `amber run --inspect` and `amber run --inspect-brk`. It is derived from `src/main.rs`, `src/tooling/inspector.rs`, and `tests/inspect_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

The listener is a small CDP subset on the process's V8 isolate. It is not Chrome DevTools, not the VS Code Node debug adapter, and not `v8::inspector`.

## Command

```bash
amber run --inspect <file>
amber run --inspect-brk <file>
amber run --inspect-brk --inspect-port 9229 <file>
```

- The listener is `127.0.0.1` only. There is no host flag.
- `--inspect-port` defaults to `9229`. `0` is rejected. There is no ephemeral-port mode.
- `--inspect-brk` implies the same listener as `--inspect` and also pauses before the user script. Passing both flags pauses.
- `--preload` / `--require` run before that pause, on the same isolate. The user script does not.
- Exit status is non-zero on every contracted failure below. Those failures print a stderr line that starts with `error: amber run:`.
- On failure the user script does not run.

## Discovery

| Request | Response |
| :--- | :--- |
| `GET /json/version` | `200` JSON. `Browser` is `Amber/<package version>`. `Protocol-Version` is `1.3`. |
| `GET /json/list` and `GET /json` | `200` JSON array with one target. `webSocketDebuggerUrl` is `ws://127.0.0.1:<port>/ws`. |
| Any other HTTP path | `404`. |
| `GET /ws` with `Upgrade: websocket` | The CDP socket. |

Stdout includes `Debugger listening on ws://127.0.0.1:<port>/ws` after the port binds.

`devtoolsFrontendUrl` is present for clients that read it. Opening that URL does not provide breakpoints, scope variables, or a DevTools session.

## WebSocket methods

| Method | Behavior |
| :--- | :--- |
| `Runtime.evaluate` | Indirect `eval` of `params.expression` on the user isolate. Reply `result` is a remote object. |
| `Runtime.runIfWaitingForDebugger` | Releases `--inspect-brk`. Also emits `Debugger.resumed`. |
| `Debugger.resume` | Same release as `Runtime.runIfWaitingForDebugger`. |

Any other method that carries an `id` gets a JSON-RPC error:

```json
{"id": 1, "error": {"code": -32601, "message": "Amber inspector does not support this method"}}
```

`Debugger.stepOver`, `Debugger.stepInto`, `Debugger.stepOut`, `Debugger.setBreakpoint`, and `Debugger.enable` are in that error set. They do not change isolate state and do not resume the user script.

### `Runtime.evaluate` values

| Value | `result` |
| :--- | :--- |
| finite number | `type: "number"` and numeric `value` |
| boolean | `type: "boolean"` and `value` |
| string | `type: "string"` and `value` |
| `undefined` | `type: "undefined"` and no `value` |
| `null` | `type: "object"`, `subtype: "null"` |
| other object | `type` plus `description` (`String(value)`). No `objectId`, no property walk. |
| thrown exception | `exceptionDetails.text` is the exception string. `result` has no `value`. |

The expression runs on the same isolate and context as the user script. Assignments made while paused are visible to the user script after resume. Preload assignments are visible to `Runtime.evaluate` during the pause.

### When evaluate runs

`Runtime.evaluate` is serviced on the isolate thread only while that thread is free:

- During the `--inspect-brk` pause, before the user script.
- After the user script yields to the host event loop (a ref'd timer or a listening HTTP server).

It does not preempt a synchronous JavaScript turn. The `v8` 152.2.0 interrupt callback must not reenter the isolate, so a tight loop never reaches the evaluate pump.

If the isolate thread does not service the call within 20 seconds, the reply is an error (`code` `-32000`, message `Runtime.evaluate was not serviced on the isolate thread`), not a successful `undefined`.

`--inspect` does not pause. The user script starts without a debugger message. The socket does not emit `Debugger.paused`. A short script can exit before a client attaches; use `--inspect-brk` to evaluate first.

`--inspect-brk` emits one `Debugger.paused` event on connect (`reason` `Break on start`, empty `scopeChain`) and does not run the user script until `Runtime.runIfWaitingForDebugger` or `Debugger.resume`.

## Contracted failures

| Condition | stderr contains |
| :--- | :--- |
| Port already bound | `failed to bind inspector` |
| `--inspect-port 0` | `--inspect-port` |
| `--watch` together with either inspect flag | `cannot be combined with --watch` |
| `--workers` greater than 1, or `AMBER_WORKERS` greater than 1, together with either inspect flag | `cannot be combined with --workers` |

## Non-goals

- Chrome DevTools, `chrome://inspect` debugging, and the VS Code Node debug adapter.
- Breakpoints, stepping, stack traces, and scope variables.
- Blackboxing, heap snapshots, the profiler domain, and `Runtime.compileScript`.
- Listening on anything other than `127.0.0.1`.
- Preempting synchronous JavaScript.
- A remote-object mirror (`objectId`, `Runtime.getProperties`).

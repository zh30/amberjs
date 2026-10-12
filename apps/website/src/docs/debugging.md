---
title: "Debugging"
nav: "Debugging"
subtitle: "A small Chrome DevTools Protocol endpoint for evaluating code in a running script."
status: "stable"
id: "debugging"
---

`amber run --inspect` and `--inspect-brk` are **Stable** (G5) for the CDP subset in [`docs/INSPECT_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/INSPECT_CONTRACT.md).

## Usage

```bash
amber run --inspect app.js
amber run --inspect-brk app.js            # wait for the client before running
amber run --inspect --inspect-port 9300 app.js
```

The listener binds to `127.0.0.1` (default port 9229; port 0 is rejected). It answers:

- `GET /json/version` and `GET /json/list`.
- `Runtime.evaluate` on the user isolate.
- `Runtime.runIfWaitingForDebugger` and `Debugger.resume`, which release `--inspect-brk`.

## Limits

This is **not** Chrome DevTools or the V8 inspector.

- There are no breakpoints, stepping, or scope inspection.
- Other CDP methods return JSON-RPC error `-32601`.
- `Runtime.evaluate` only runs while the isolate thread is free: during the pause or between event-loop tasks. It does not interrupt a synchronous turn.
- Results carry a `description` only.
- `--watch` and `--workers` greater than 1 are rejected with `--inspect`.

## Other tools

`amber debug`, `amber lsp`, and `amber profile` exist in the binary but are not in the Stable scope. See [Experimental](/docs/experimental).

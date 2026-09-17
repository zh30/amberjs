# Amber debugger (v1.9.1)

The public inspector is `amber run --inspect` / `amber run --inspect-brk`, not `amber debug`.
`amber debug <file>` still exists as an experimental extra-diagnostics command; it is not the Chrome DevTools / VS Code attach path.

Default CDP port is **9229**.

## Commands

```bash
amber run --inspect script.js
amber run --inspect-brk script.js
amber run --inspect-brk --inspect-port 9229 app.ts
```

`--inspect` starts the CDP HTTP/WebSocket agent and runs the script.
`--inspect-brk` does the same but **does not execute user code** until DevTools sends `Runtime.runIfWaitingForDebugger` or `Debugger.resume`.

Discovery:

```text
GET http://127.0.0.1:9229/json/version
GET http://127.0.0.1:9229/json/list
ws://127.0.0.1:9229/ws
```

`/json/version` reports `Browser: Amber/<package version>`.
`Runtime.evaluate` runs on the V8 isolate (for example `1+1` → `2`).
This is Preview: rusty_v8 0.22 does not expose a full `v8::inspector` session, so line breakpoints and scope walking are not Chrome-complete.

## Chrome DevTools

1. `amber run --inspect-brk --inspect-port 9229 app.js`
2. Open `devtools://devtools/bundled/js_app.html?ws=127.0.0.1:9229/ws`
3. Resume execution when ready.

## VS Code attach

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "node",
      "request": "launch",
      "name": "Debug with amber",
      "runtimeExecutable": "amber",
      "runtimeArgs": ["run", "--inspect-brk", "--inspect-port", "9229"],
      "args": ["${file}"],
      "port": 9229
    },
    {
      "type": "node",
      "request": "attach",
      "name": "Attach to amber --inspect",
      "port": 9229
    }
  ]
}
```

The in-repo extension (`tools/vscode-extension`) launches the same `amber run --inspect-brk --inspect-port` command.

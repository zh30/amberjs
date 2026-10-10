# Node `child_process` async contract

This is the user-facing contract for the Stable **narrow async** subset of Node `child_process` in Amber (**tentative G47**). It is derived from `src/nodejs_core/child_process.rs`, `src/runtime_minimal.rs`, and `tests/child_process_async_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('child_process')`, `require('node:child_process')`, and ESM `import` of those names reach the same object installed by `setup_child_process_api`. Named ESM exports include `exec` and `execFile`.

The sync surface (`execSync` / `spawnSync`) stays **G25** ([`docs/CHILD_PROCESS_CONTRACT.md`](CHILD_PROCESS_CONTRACT.md)). This page does not renumber G25.

This is not full Node `child_process`. Only the async calls listed below are Stable under G47.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `exec(command[, options], callback)` | Starts `command` on a **host worker thread** through the host shell (`sh -c` on Unix, `cmd /C` on Windows). Returns a pending child stub **before** the process exits (does not block the isolate on `.output()`). When the process exits, invokes `callback(error, stdout, stderr)` on a **later** event-loop turn. `stdout` / `stderr` are UTF-8 lossy strings. On exit status 0, `error` is `null`; otherwise `error` is an `Error` whose message includes the exit code and whose `code` property is that exit code. `options` (when present between command and callback) is accepted for call-shape compatibility and is otherwise ignored. |
| `execFile(file[, args][, options], callback)` | Same scheduling and callback shape as `exec`, but runs `file` with `args` (string array) via `std::process::Command` (no shell). |

## Return value

Both calls return a plain object immediately with:

- `pid` number `0` (not a real OS pid)
- `killed` boolean `false`
- `exitCode` / `signal` `null` at return time
- `stdout` / `stderr` `undefined` at return time
- chainable `on(...)` that does **not** fire exit/close synchronously

Stable callers must use the callback, not `on('exit')` / streaming stdio, for completion.

## Failures

- Under `--sandbox` without an allowing `--allow-run` / permission policy for that argv0, both calls throw **synchronously** before a host thread starts. The thrown value is an `Error` whose message reports the permission denial.
- A non-zero exit delivers a non-null `error` to the callback (does not throw on the isolate).
- A spawn/exec OS error on the worker thread is delivered as a non-zero exit-shaped callback (`error` non-null, stderr text from the OS error).

## Non-goals

- `spawn` with live `stdin` / `stdout` / `stderr` pipes, streaming `ChildProcess`, or `ChildProcess` EventEmitter exit/close parity. Preview `spawn` may still sync-block today and is **not** this contract.
- `fork`, IPC, detached processes, real `pid`, signals, `kill`, `timeout`, `maxBuffer`, `cwd`, `env`, `uid` / `gid`, `shell`, `input`, `stdio`, `windowsHide`, and encoding option bags.
- Same-turn callback delivery (the pre-G47 Preview lie). Stable async must not block the isolate on `Command::output()` inside the `exec` / `execFile` call.
- Graduating Preview `spawn` by documenting sync-block as a Stable Limit.

## Reachability

The CLI `amber` binary installs this object from `src/runtime_minimal.rs`. Library users reach it through `amberjs::nodejs_core::child_process::setup_child_process_api` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.

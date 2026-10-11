# Node `child_process.spawn` async contract

This is the user-facing contract for the Stable **narrow async `spawn`** subset of Node `child_process` in Amber (**tentative G60**). It is derived from `src/nodejs_core/child_process.rs`, `src/runtime_minimal.rs`, and `tests/child_process_spawn_async_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('child_process')`, `require('node:child_process')`, and ESM `import` of those names reach the same object installed by `setup_child_process_api`. Named ESM exports include `spawn`.

Sync `execSync` / `spawnSync` stay **G25** ([`docs/CHILD_PROCESS_CONTRACT.md`](CHILD_PROCESS_CONTRACT.md)). Narrow async `exec` / `execFile` stay **G47** ([`docs/CHILD_PROCESS_ASYNC_CONTRACT.md`](CHILD_PROCESS_ASYNC_CONTRACT.md)). This page does not renumber those graduations.

This is not full Node `child_process`. Only the async `spawn` call listed below is Stable under G60.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `spawn(command[, args][, options], callback?)` | Starts `command` with `args` (string array) via `std::process::Command` (no shell) on a **host worker thread**. Returns a pending child stub **before** the process exits (does not block the isolate on `.output()`). When the process exits, if a function `callback` was supplied (as the last argument after `command`, after `args`, or after an ignored `options` object), invokes `callback(error, stdout, stderr)` on a **later** event-loop turn. `stdout` / `stderr` are UTF-8 lossy strings captured after exit (not live pipes). On exit status 0, `error` is `null`; otherwise `error` is an `Error` whose message includes the exit code and whose `code` property is that exit code. `options` (when present between args and callback) is accepted for call-shape compatibility and is otherwise ignored. |

## Return value

`spawn` returns a plain object immediately with:

- `pid` number `0` (not a real OS pid)
- `killed` boolean `false`
- `exitCode` / `signal` `null` at return time
- `stdout` / `stderr` `undefined` at return time
- chainable `on(...)` that does **not** fire exit/close synchronously

Stable callers must use the optional callback for completion. `on('exit')` / streaming stdio are not this contract.

## Failures

- Under `--sandbox` without an allowing `--allow-run` / permission policy for that argv0, `spawn` throws **synchronously** before a host thread starts. The thrown value is an `Error` whose message reports the permission denial.
- A non-zero exit delivers a non-null `error` to the callback (does not throw on the isolate).
- A spawn OS error on the worker thread is delivered as a non-zero exit-shaped callback (`error` non-null, stderr text from the OS error).

## Limits

These are true quirks of the shipped Stable `spawn` surface (not missing APIs dressed as Limits):

- The immediate return stub always has `pid === 0` (not a real OS pid), `killed === false`, `exitCode` / `signal` `null`, and `stdout` / `stderr` `undefined` at return time.
- `options` (when present) is accepted for call-shape compatibility and is otherwise ignored — no `cwd`, `env`, `timeout`, `maxBuffer`, `encoding`, `stdio`, or other option bags are honored.
- Stdout/stderr delivered to the callback are UTF-8 lossy strings captured **after** the process exits (buffered post-exit), not live pipe streams.
- When a `callback` is supplied, it runs on a **later** event-loop turn after the worker finishes — never same-turn inside the `spawn` call, and never via sync-block `Command::output()` on the isolate.
- Chainable `on(...)` on the stub does **not** fire `exit` / `close` (or any other child event) as part of this Stable contract; completion is the optional callback only.

## Non-goals

These are outside Stable. Do **not** document them as Stable Limits fiction for unfinished surfaces:

- Live `stdin` / `stdout` / `stderr` pipes, streaming `ChildProcess`, or `ChildProcess` EventEmitter exit/close parity.
- `fork`, IPC, detached processes, real `pid`, signals, `kill`, and honored option bags (`timeout`, `maxBuffer`, `cwd`, `env`, `uid` / `gid`, `shell`, `input`, `stdio`, `windowsHide`, encoding).
- Reintroducing same-turn callback delivery or sync-block `Command::output()` inside the `spawn` call (the pre-G60 Preview lie).
- Graduating pipes / `fork` by documenting absence as Stable Limits.

## Reachability

The CLI `amber` binary installs this object from `src/runtime_minimal.rs`. Library users reach it through `amberjs::nodejs_core::child_process::setup_child_process_api` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.

# Node `child_process` contract

This is the user-facing contract for the Stable sync subset of Node `child_process` in Amber. It is derived from `src/nodejs_core/child_process.rs`, `src/runtime_minimal.rs`, and `tests/child_process_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('child_process')`, `require('node:child_process')`, and ESM `import` of those names reach the same object installed by `setup_child_process_api`. The default ESM export is that object. Named ESM exports include `execSync` and `spawnSync`.

This is not full Node `child_process`. Only `execSync` and `spawnSync` are Stable on this page. Narrow async `exec` / `execFile` (host-thread + later-turn callback) is Stable under **tentative G47** ([`docs/CHILD_PROCESS_ASYNC_CONTRACT.md`](CHILD_PROCESS_ASYNC_CONTRACT.md)). `spawn`, `fork`, streaming stdio, and detached processes stay outside both contracts (Preview / Non-goals).

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `execSync(command[, options])` | Runs `command` through the host shell (`sh -c` on Unix, `cmd /C` on Windows). Blocks the isolate until the process exits. On exit status 0, returns stdout. Without `options.encoding`, or with `encoding: "buffer"`, the return value is a `Buffer` (or an `ArrayBuffer` when `Buffer` is unavailable). Any other `options.encoding` string returns a UTF-8 lossy string. |
| `spawnSync(command[, args][, options])` | Runs `command` with `args` (string array) via `std::process::Command` (no shell). Blocks until exit. Returns an object with `status` (exit code number, or `0` when the OS status has no code), `signal` (`null`), `pid` (`0`), `stdout`, `stderr`, `output` (`[null, stdout, stderr]`), and `error` (`undefined` on success). Encoding for `stdout` / `stderr` follows the same `options.encoding` rules as `execSync`. When `args` is omitted and the second argument is an options object, that object is treated as `options` and `args` is empty. |

## Failures

- Non-zero exit from `execSync` throws an `Error` whose message starts with `Command failed:`, with `status` set to the exit code and `stdout` / `stderr` encoded like a successful return.
- A spawn/exec OS error from `execSync` (command not found, etc.) throws an `Error` whose message starts with `Command failed:`.
- A spawn OS error from `spawnSync` does not throw. The result has `status === 1`, empty-string `stdout`, `stderr` set to the OS error text, and `error` set to an `Error` with that message.
- Under `--sandbox` without an allowing `--allow-run` / permission policy for that argv0, both `execSync` and `spawnSync` throw before the process starts. The thrown value is an `Error` whose message reports the permission denial.

## Non-goals

- Narrow async `exec` / `execFile` — see **G47** [`docs/CHILD_PROCESS_ASYNC_CONTRACT.md`](CHILD_PROCESS_ASYNC_CONTRACT.md) (not this sync page).
- `spawn`, `fork`, and any streaming `ChildProcess` with live `stdin` / `stdout` / `stderr` pipes.
- Real `pid` values, `killed`, signals, `timeout`, `maxBuffer`, `cwd`, `env`, `uid` / `gid`, `shell`, `input`, `stdio`, and `windowsHide`.
- Detached processes, IPC, or `child_process.fork` module workers.
- Node's async callback scheduling on this page. This contract is sync-only.

## Reachability

The CLI `amber` binary installs this object from `src/runtime_minimal.rs`. Library users reach it through `amberjs::nodejs_core::child_process::setup_child_process_api` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.

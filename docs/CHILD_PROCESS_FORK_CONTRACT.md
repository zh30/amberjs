# Node `child_process.fork` contract

This is the user-facing contract for the Stable **narrow `fork`** subset of Node `child_process` in Amber (**G67**). It is derived from `src/nodejs_core/child_process_fork.rs`, `src/nodejs_core/child_process.rs`, `src/runtime_minimal.rs`, and `tests/child_process_fork_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('child_process')`, `require('node:child_process')`, and ESM `import` of those names reach the same object installed by `setup_child_process_api`. Named ESM exports include `fork`.

Sync `execSync` / `spawnSync` stay **G25** ([`docs/CHILD_PROCESS_CONTRACT.md`](CHILD_PROCESS_CONTRACT.md)). Narrow async `exec` / `execFile` stay **G47** ([`docs/CHILD_PROCESS_ASYNC_CONTRACT.md`](CHILD_PROCESS_ASYNC_CONTRACT.md)). This page does not renumber G25 or G47.

This is not full Node `child_process.fork`. Only the IPC slice listed below is Stable under G67.

## Stable surface

| Call / field | Behavior |
| :--- | :--- |
| `fork(modulePath[, args][, options])` | Spawns **this machine's** `amber` executable as `amber run <modulePath> …` (real OS process). Establishes a localhost TCP IPC channel via env `AMBER_FORK_CHANNEL`. Returns a child object **before** the child exits. `args` (string array) are forwarded after the module path. `options` (when present) is accepted for call-shape compatibility and otherwise ignored. |
| `child.pid` | Real OS pid of the spawned process (not the G25/G47 placeholder `0`). |
| `child.connected` | `true` after return while the IPC socket is open; set `false` after disconnect or child exit. |
| `child.send(message)` | JSON-serializes `message` (`JSON.stringify`) and writes a length-prefixed IPC frame. Returns `true` when the write is accepted, `false` when not connected. Non-JSON-serializable values throw. |
| `child.on('message', listener)` | Registers a listener. Delivered payloads are `JSON.parse` of the child's sent value, on a later event-loop turn. |
| `child.on('exit' \| 'close', listener)` | Fired once when the OS process exits, with `(code, null)`. |
| `child.on('disconnect', listener)` | Fired when the IPC socket closes before or with exit. |
| `child.disconnect()` | Drops the parent IPC writer and marks `connected` false. |
| Child `process.send(message)` | Installed only when `AMBER_FORK_CHANNEL` is set (forked worker bootstrap). Same JSON IPC rules as `child.send`. |
| Child `process.on('message', listener)` | **Real** for `'message'` only in a forked child. Other `process.on` event names stay chainable no-ops (G22/G44 honesty — not graduated here). |
| Child `process.connected` | `true` while the IPC socket to the parent is open. |

## IPC framing

- Transport: TCP to `127.0.0.1` (ephemeral parent listen port).
- Frame: 4-byte big-endian length + UTF-8 JSON body `{"v":1,"t":"msg","d":<json>}`.
- Values are JSON-only (not structured clone, not Node native handles).

## Failures

- Under `--sandbox` without an allowing `--allow-run` / permission policy for the module path argv0, `fork` throws **synchronously** before spawn. The thrown value is an `Error` whose message reports the permission denial.
- Empty / missing `modulePath` throws synchronously.
- Listen / spawn OS failures throw synchronously with an `Error` whose message starts with `fork:`.

## Limits

- Messages are JSON-serializable only.
- `options` bags (`cwd`, `env`, `execPath`, `execArgv`, `silent`, `stdio`, `serialization`, `uid` / `gid`, …) are not applied.
- General `process.on` / `process.off` outside forked-child `'message'` stay Preview no-ops and are **not** Stable Limits fiction for the rest of `process`.

## Non-goals

- Full Node `ChildProcess` EventEmitter / streaming stdio pipes (`stdin` / `stdout` / `stderr` as streams).
- `silent`, custom `execPath` / `execArgv`, `serialization: 'advanced'`, native handle passing.
- Graduating no-op `process.on` / `process.off` for non-`message` events.
- Preview `spawn` pipes (separate from this fork carve; see G47 Non-goals / future spawn work).

## Reachability

The CLI `amber` binary installs this object from `src/runtime_minimal.rs`. Library users reach it through `amberjs::nodejs_core::child_process::setup_child_process_api` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.

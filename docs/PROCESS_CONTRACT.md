# Node `process` basics contract

This is the user-facing contract for a **tiny** Stable slice of the global `process` object in Amber. It is derived from `src/nodejs_core/process.rs`, `src/runtime_minimal.rs` (`require('process')` / `require('node:process')` return the global `process`), and `tests/process_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

Full Node `process` is too wide for one graduation: stdio, `hrtime`, `memoryUsage`, signals, `on`/`off`, `dlopen`, `kill`, `umask`, `uptime`, and argv shaping stay Preview. This page pins only the basics that are real today.

## Stable surface

| Call / field | Behavior |
| :--- | :--- |
| `process.nextTick(fn, ...args)` | Queues `fn` on the process next-tick queue. A non-function throws `TypeError` whose message contains `callback must be a function`. Extra arguments are passed through. Queued callbacks run on a later turn of the isolate event loop, **before** Promise microtasks scheduled on the same turn. Nested `nextTick` calls scheduled during a batch run in a later batch of the same drain. |
| `process.env` | An object of environment string values visible to the process (subject to the permission broker). Reading a present key returns a string. Assigning a value stores `String(value)`. |
| `process.cwd()` | Returns the current working directory as a string. |
| `process.pid` | A number: the host OS process id, greater than 0. |
| `process.platform` | `"linux"`, `"darwin"`, `"win32"`, or `"unknown"` for the host build target. |
| `require('process')` / `require('node:process')` | The same object as `globalThis.process`. |

## Non-goals / still Preview

Do **not** treat these as Stable under this contract (even when a property exists):

- `argv` / `execArgv` / `execPath` / `title` / `version` / `versions` / `release` / `features`
- `stdout` / `stderr` / `stdin`
- `hrtime` / `uptime` / `memoryUsage` / `cpuUsage` (several are estimated or epoch-based, not Node-faithful)
- `on` / `off` / `removeListener` / `setMaxListeners` / `getMaxListeners` (`on`/`off` do not register uncaught handlers)
- `exit` / `exitCode` / `abort` / `kill` / `chdir` / `umask` / `config`
- `dlopen` (N-API hello loader stays Experimental)
- Matching Node EventEmitter `process` or libuv process semantics beyond `nextTick`

## Reachability

The CLI `amber` binary installs `globalThis.process` from `src/runtime_minimal.rs` via `nodejs_core::process::setup_process_api`. Library users reach the same setup through `amberjs::nodejs_core`. The contract is not feature-gated.

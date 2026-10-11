# Node `process` basics contract

This is the user-facing contract for a **tiny** Stable slice of the global `process` object in Amber. It is derived from `src/nodejs_core/process.rs`, `src/runtime_minimal.rs` (`require('process')` / `require('node:process')` return the global `process`), and `tests/process_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

Full Node `process` is too wide for one graduation: stdin, stream Writable APIs beyond the G44 carve, `memoryUsage`, signals, `on`/`off`, `dlopen`, `kill`, `umask`, `uptime`, and argv shaping stay Preview. This page pins the G22 basics, the additive **G44** `stdout` / `stderr` **`write`** carve, and the additive **G53** `hrtime` / `hrtime.bigint` carve.

## Stable surface

| Call / field | Behavior |
| :--- | :--- |
| `process.nextTick(fn, ...args)` | Queues `fn` on the process next-tick queue. A non-function throws `TypeError` whose message contains `callback must be a function`. Extra arguments are passed through. Queued callbacks run on a later turn of the isolate event loop, **before** Promise microtasks scheduled on the same turn. Nested `nextTick` calls scheduled during a batch run in a later batch of the same drain. |
| `process.env` | An object of environment string values visible to the process (subject to the permission broker). Reading a present key returns a string. Assigning a value stores `String(value)`. |
| `process.cwd()` | Returns the current working directory as a string. |
| `process.pid` | A number: the host OS process id, greater than 0. |
| `process.platform` | `"linux"`, `"darwin"`, `"win32"`, or `"unknown"` for the host build target. |
| `require('process')` / `require('node:process')` | The same object as `globalThis.process`. |
| `process.stdout.write(chunk)` | Writes the V8 `ToString` of `chunk` to the host process **stdout** and flushes. Returns `true`. `null` / `undefined` write an empty string. Extra arguments (encoding, callback) are ignored. |
| `process.stderr.write(chunk)` | Same as `stdout.write`, but writes to the host process **stderr**. |
| `process.hrtime()` | Returns a length-2 array `[seconds, nanoseconds]` of non-negative numbers from the host wall clock as nanoseconds since the Unix epoch, split into whole seconds and the remainder. Values are an **arbitrary past** relative to a Node monotonic clock — do **not** claim Node `hrtime` clock parity or process-start-relative timestamps. |
| `process.hrtime(previous)` | Same absolute epoch-based `[seconds, nanoseconds]` as a no-arg call. The `previous` argument is **ignored** (no previous-value diff overload). Callers must not treat the return as a Node-shaped elapsed delta. |
| `process.hrtime.bigint()` | Returns a `bigint` of nanoseconds since the Unix epoch (same wall-clock source as `hrtime()`). Extra arguments are ignored. Not a Node monotonic bigint clock. |

## Honest Limits (G53)

- **No previous-value diff:** `process.hrtime([sec, nsec])` does not subtract a prior tuple. Arguments are ignored; the return is always the current absolute epoch split.
- **Epoch-based arbitrary past:** Both `hrtime()` and `hrtime.bigint()` use `SystemTime` since `UNIX_EPOCH`. Magnitudes look like wall-clock epoch seconds / nanos, not Node's typical process-relative values. Do not claim Node clock parity.
- This carve does **not** graduate `uptime`, `memoryUsage`, `cpuUsage`, or any other process timing API.

## Non-goals / still Preview

Do **not** treat these as Stable under this contract (even when a property exists):

- `argv` / `execArgv` / `execPath` / `title` / `version` / `versions` / `release` / `features`
- `stdin` (including `stdin.read`)
- Full stdio stream APIs beyond the `write` carve above: `pipe`, `end`, `cork` / `uncork`, `destroy`, backpressure / `drain`, encoding / callback overloads of `write`
- `stdout` / `stderr` fields other than `write` (`fd`, `isTTY`, `columns`, `rows`, and any other property)
- `uptime` / `memoryUsage` / `cpuUsage` (estimated or otherwise; not this carve)
- Node-faithful `hrtime(previous)` elapsed-diff semantics and a process-start monotonic clock (deliberately outside Stable; see Honest Limits)
- `on` / `off` / `removeListener` / `setMaxListeners` / `getMaxListeners` (**never** Stable: `on`/`off` do not register uncaught handlers — do not graduate as Limits fiction)
- `exit` / `exitCode` / `abort` / `kill` / `chdir` / `umask` / `config`
- `dlopen` (N-API hello loader stays Experimental)
- Matching Node EventEmitter `process` or libuv process semantics beyond `nextTick`, the `write` carve, and the `hrtime` carve

## Reachability

The CLI `amber` binary installs `globalThis.process` from `src/runtime_minimal.rs` via `nodejs_core::process::setup_process_api`. Library users reach the same setup through `amberjs::nodejs_core`. The contract is not feature-gated.

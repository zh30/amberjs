# `amber` watch / hot reload contract

This is the user-facing contract for Stable `amber run --watch` and `amber test --watch`. It is derived from `src/main.rs`, `src/watcher.rs`, `src/watcher_websocket.rs`, and the executable tests in `tests/watch_contract_tests.rs` (plus `tests/hot_reload_tests.rs` and `src/watcher.rs` / `src/watcher_websocket.rs` unit tests). Historical `docs/STAGE_*` reports are not part of this contract.

Watch mode re-executes a finished script, or re-runs an already discovered test set, when a watched file changes. It is not in-place hot module replacement, and it does not inject a reload client into a page.

The watcher and WebSocket server are part of the default build (`pub mod watcher` and `pub mod watcher_websocket` in `src/lib.rs`). They are not behind a Cargo feature.

## `amber run --watch`

```bash
amber run --watch [--debounce <ms>] [--websocket-port <port>] [-r|--preload <module>] [--require <module>] <file> [args...]
```

- `<file>` must be an existing file. A missing path or a directory fails before the watch banner with `error: amber watch:` and `entry must be a file:`.
- `--deny-fs` (or any broker denial of `FileSystem`/`Read` on the entry) fails before `Watch mode enabled`, before a watcher starts, and before `WebSocket server ready`. The script does not run.
- `--debounce` defaults to `100` milliseconds and is the quiet period before a batch of filesystem events is delivered.
- The watch root is the entry file's parent directory (the current directory when the entry has no parent). The watch is recursive.
- `--websocket-port` defaults to `9999` and binds `127.0.0.1` only. `0` asks the OS for an ephemeral port. The ready line prints the bound address: `WebSocket server ready on ws://<addr>`.
- The listener is bound before the ready line and before the entry runs. Bind failure (port in use, or `Network`/`Listen` denied) prints `error: amber watch:`, stops the file watcher, and does not print the ready line or execute the entry.
- `--deny-net` denies that listen unless `--allow-listen` allows the host (`127.0.0.1`) or the exact `ws://127.0.0.1:<port>` URL checked before bind (`ws://127.0.0.1:<configured port>`, including port `0`).
- On a watched change, the original entry is executed again from scratch in a new isolate, including `--preload` / `--require` modules. Script arguments are passed as `process.argv`. A sibling script change under the watch root also re-runs the entry.
- The terminal is cleared (ANSI) before the reload log. The log includes the notify change label (`created`, `modified`, `removed`, or `renamed`) and `Reloaded in <ms>ms`.
- A script exception is printed (`❌ Error:`) and watching continues. The process does not exit. An initial read or preload failure exits non-zero with `error: amber watch:`. A later read or preload failure is printed (`❌ Reload failed:`) and watching continues.
- WebSocket clients receive one text frame on connect: `event_type` `status`, `message` `connected`. A text frame `ping` is answered with `pong`. Reload frames are JSON:

```json
{"event_type":"reload","file_path":"<path>","change_type":"modified","timestamp":0,"message":null}
```

`change_type` is the label above. No connected client is still success. `stop()` stops accepting; the port can be bound again after the accept loop observes shutdown.

## `amber test --watch`

```bash
amber test --watch [file-or-directory]
amber test --parallel
```

- `amber test --parallel` exits `2` and prints `not supported`, including when `--watch` is also passed. It does not discover or run tests. V8 isolates are not shared across threads.
- Without `--watch`, a passing file or directory exits `0`. A failing file exits `1`. `--bail` without `--watch` still exits `1` on the first failing file.
- With `--watch`, tests run once, then the process stays up. A failing file, directory, or discovery run does not exit before the watch loop. `--bail` with `--watch` stops the rest of that initial round and then watches.
- Single file: watch the file's parent directory. Any watched script change re-runs that file. Failures are printed and the process stays up.
- Directory: watch that directory. A watched change re-runs the files discovered at start (`🔄 File changed: ... Re-running tests...`).
- No path: watch the current directory after zero-argument discovery (the Stable discovery exclusions still apply: `manual`, `node_modules`, `__snapshots__`, plus `.git`, `target`, and `dist`). A watched change re-runs that same set (`Re-running discovered tests...`).
- Test watch debounce is `200` ms. Banners contain `Watching for changes` and `Ctrl+C to quit`. The banner is printed only after `HotReloader::watch` has armed the OS watcher (same readiness idea as the run `--watch` WebSocket ready line).
- `--coverage` still writes the lcov report from the initial run. In discovery mode it is written before the watch loop.

## What is watched

Extensions, case-insensitive: `js`, `ts`, `mjs`, `cjs`, `jsx`, `tsx`.

Any path component named `node_modules`, `.git`, `dist`, `build`, `target`, or `.amberjs-cache` is ignored.

Notify event kinds map as:

| Notify kind | `change_type` |
| :--- | :--- |
| `Create` | `created` |
| `Remove` | `removed` |
| `Modify(Name)` | `renamed` |
| other `Modify`, or `Any` | `modified` |
| `Access`, `Other` | ignored |

A create followed by a write in the same debounce window stays `created`. A remove followed by a create of the same path becomes `modified`. One event is emitted per path per quiet period, in path order. The operating system may report a create-plus-write as only `modified`; both labels trigger a reload. `Access` never reloads.

`HotReloader::watch` creates the OS watcher on the calling thread. A missing path or a backend error returns `Err` and leaves `is_running()` false. `FileSystem`/`Read` is checked on the watch root and on each counted file before the watcher starts, and again for each delivered path. A denied path is not delivered. `files_watched` is the count of matching files after that scan (`0` when startup fails).

A notify rescan/overflow with no concrete paths emits one `modified` event for the watch root so the entry or tests run again.

## Non-goals

- Not in-place HMR. No module accept/dispose, no preserved isolate state, no browser script injection. `amber serve` is unrelated.
- The entry or test run is not interrupted. A script that stays inside the runtime (listening HTTP server, pending timers) is not restarted when a file changes, because the watch loop waits for that execution to return.
- `--workers`, `--timeout`, and `--max-memory` are not applied to watch re-executions. `amber run --inspect` / `--inspect-brk` cannot be combined with `--watch`; the process exits before watch starts.
- A `package.json` script name (`amber run --watch <script>`) runs that shell script once and does not enter watch mode.
- Test watch does not rediscover files added after startup. If discovery finds nothing, the built-in snippet suite runs and `--watch` is not entered.
- Rename labels follow the OS notify backend. Clients must reload on every `change_type`.

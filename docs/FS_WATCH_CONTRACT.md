# Node `fs.watch` / `fs.watchFile` contract

This is the user-facing contract for the Stable `fs.watch` / `fs.watchFile` slice in Amber (**G64**). It is derived from `src/nodejs_core/fs_watch.rs`, `src/runtime_minimal.rs` (fs-watch pump), and `tests/fs_watch_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

The broader Node `fs` byte I/O surface stays G8 ([`docs/FS_CONTRACT.md`](FS_CONTRACT.md)). This page does **not** replace G8. CLI `amber run --watch` stays G6 ([`docs/WATCH_CONTRACT.md`](WATCH_CONTRACT.md)) and is a separate process re-exec product.

**Honesty:** `fs.watch` / `fs.watchFile` were absent (`typeof fs.watch === "undefined"`). Do **not** graduate a no-op listener, a timer that fabricates `'change'`, or absence-as-Limits fiction.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `require('fs').watch(path[, options][, listener])` | Starts a real OS file watcher via the `notify` recommended backend (inotify on Linux, FSEvents on macOS, ReadDirectoryChangesW on Windows). Returns an `FSWatcher` whose prototype is EventEmitter-backed (`on` / `once` / `emit`). |
| `watcher.on('change', listener)` / watch `listener` | Delivers `(eventType, filename)` where `eventType` is `"rename"` or `"change"`. `filename` is a string basename (or relative name under a directory watch) when the backend supplies a path, otherwise `null`. Delivery runs on a later event-loop pump turn — not inside the `watch()` call. |
| `watcher.close()` | Stops the native watcher and drops the registry entry. Further filesystem mutations do not emit on that watcher. |
| `fs.watchFile(path[, options], listener)` | Starts a real host `stat` poller. `listener` must be a function (`TypeError` otherwise). Invokes `listener(curr, prev)` when size or mtime changes. `curr` / `prev` expose at least `size` and `mtimeMs` numbers. Returns `undefined`. |
| `options.interval` (watchFile) | Poll period in milliseconds. Default `100`. Values `≤ 0` fall back to the default. |
| `options.recursive` (watch) | When `true` and `path` is a directory, watches with `RecursiveMode::Recursive`. When `false` or omitted, non-recursive. |
| `fs.unwatchFile(path)` | Stops all `watchFile` pollers for that path. |
| Missing path (`watch`) | Throws `Error` with `code === "ENOENT"`, `syscall === "watch"`, `path`, and negative `errno` **synchronously**. |
| Permission denial | Under the permission broker, denied FileSystem/Read throws `TypeError` whose message contains `permission denied` before any watcher starts. |

Wire proof required by this contract: after `fs.watch` on an existing file, a host write that mutates the file content produces at least one `'change'` delivery observable via the pump. After `fs.watchFile` with a short `interval`, a host write produces at least one `listener(curr, prev)` call where `curr.size` reflects the new length.

## Limits

These are real behaviors of the Stable watch surface, not cover for missing emit:

- Event delivery is isolate-thread pump based (same model as G45 net accept). Callers / tests must allow the runtime to pump; this is not a libuv threadpool.
- `eventType` mapping follows notify kinds: create/remove/rename-name → `"rename"`; content/metadata modify → `"change"`. Access-only events are ignored.
- Backend coalescing and platform timing mean more than one `'change'` may fire for a single write; the Stable pin is **at least one** delivery, not exact Node event counts.
- `recursive: true` depends on the host `notify` backend. On some platforms recursive directory watches are emulated by walking the tree; very large trees may miss late-created nested paths until re-watch (documented platform gap — not stubbed as success).
- `watchFile` is **polling** (`stat`), not inotify. That matches Node's `watchFile` model. Interval scheduling is best-effort on a background thread.
- `filename` may be the watched file's basename even when watching a single file (Node-compatible shape).
- `ref` / `unref` on `FSWatcher` are not pinned.
- ESM `import { watch } from 'fs'` named export for these three is not separately pinned beyond the shared `fs` object G8 already installs.

## Non-goals

Outside this contract (Preview / DEFER / unimplemented). Do **not** treat these as Stable Limits of dishonest behavior:

- `fs.glob`, `fs.cp`, `fs.opendir`, `fs.statfs`
- Promise-based `fs.promises.watch` async iterator
- Exact Node event ordering / encoding option (`buffer` filenames)
- `FSWatcher` `'error'` event product contract beyond stopping a failed backend
- Fabricating change events without OS observation
- Graduating absence of `watch` as a G8 Limits bullet (removed; this page owns the surface)

## Reachability

Installed on the CLI path after EventEmitter setup: `nodejs_core::fs_watch::install_fs_watch_api` mutates `globalThis.fs`. `require('fs')` / `require('node:fs')` return that object. The contract is not feature-gated.

## Tests

```bash
cargo test --test fs_watch_contract_tests -- --test-threads=1
```

CI step: `node fs watch Stable contract`.

## Honesty rule

No-op `watch` / timer-fabricated `'change'` / “not implemented” listed as Stable Limits must never graduate. Limits describe pump delivery, backend coalescing, and recursive platform gaps only.

---
title: "Node.js compatibility"
nav: "Node.js APIs"
subtitle: "Which Node modules are Stable, what each contract covers, and what stays out."
status: "stable"
id: "node-compat"
---

Amber is not a drop-in Node.js replacement. Each module below is Stable only for the methods named in its contract. Everything else in `src/nodejs_core/` is Preview. For a searchable view, use the [compatibility page](/compatibility).

## Modules and loading

| API | Id | Notes |
| :--- | :--- | :--- |
| CommonJS `require`, `module.exports`, `require.resolve`, `createRequire` | G28 (provisional) | `node_modules` walk, `"exports"` / `"imports"` / `"main"`, JSON, TS, and ESM via `require`. No `require.cache`; resolve errors have no `code`. |
| `node:` and bare builtin names | G28 | Loading a builtin does not make its whole API Stable. |

## File system and paths

| API | Id | Notes |
| :--- | :--- | :--- |
| `fs`, `fs/promises` | G8, G77 | Byte read and write, `mkdir`, `stat`, `copyFile`, `rm`, `realpath`, fds, `chmod`, `access`, `existsSync`, `readdir` dirents, system errors. `promises.open` returns a `FileHandle`. Async work runs on the isolate thread at the next tick. |
| `fs.watch`, `fs.watchFile` | G64 (tentative) | Real OS watcher and stat polling. No `fs.promises.watch`. |
| `fs.cp`, `fs.opendir`, `fs.statfs` | G76 (tentative) | Recursive copy, a `Dir` snapshot, and Unix `statfs`. |
| `path` | G17 | POSIX semantics. `path.win32` only changes `sep` and `delimiter`. |

`fs.glob` is not implemented.

## Core utilities

| API | Id | Notes |
| :--- | :--- | :--- |
| `events` (`EventEmitter`) | G18 | No `events.on` async iterator. |
| `buffer` / `Buffer` | G19 | `from`, `alloc`, `concat`, `byteLength`, `toString`, `slice`. `compare` and `readUInt*` are outside the pin. |
| `util` | G22, G59 | `format`, shallow `inspect`, `promisify`, `callbackify`, `inherits`, listed `util.types`. |
| `process` | G22, G44, G53, G54, G70 | `nextTick`, `env`, `cwd`, `pid`, `platform`, `stdout.write`, `hrtime`, `memoryUsage`, and `stdin` UTF-8 string chunks. Heap fields are estimates; `process.on` / `off` are no-ops and not Stable. |
| `os` | G20, G62, G63 | Host info, `hostname`, real `networkInterfaces`. |
| `querystring` | G38 | Fixed `&` and `=` separators. |
| `string_decoder` | G39, G68 | UTF-8 plus latin1, ascii, base64, hex, utf16le. No `base64url`. |
| `assert` | G40, G69 | `ok`, `strictEqual`, `deepStrictEqual`, `throws`, `match`, `rejects`, and a few more. Loose `equal` semantics are not pinned. |
| `timers`, `timers/promises` | G42, G50 | Same functions as the globals; Promise `setTimeout` and `setImmediate` with `AbortSignal`. |
| `tty.isatty` | G49 | Always `false` on Windows today. |
| `vm` | G55, G66 (tentative) | `runInThisContext` and `runInNewContext`. Not full Contextify. |
| `vm.createContext` | G73 | `createContext` / `isContext` markers and `runInContext`. Persistent context reuse stays out. |
| `diagnostics_channel` | G56 (tentative) | `channel` pub/sub and `tracingChannel.traceSync`. |
| `async_hooks` | G57 (tentative) | `AsyncResource` and `AsyncLocalStorage`. No `createHook`. |
| `readline` | G46 (tentative) | `question()` on host stdin and stdout only. |

## Crypto and compression

| API | Id | Notes |
| :--- | :--- | :--- |
| `crypto` hashes and random | G23 | `createHash`, `createHmac` (md5, sha1, sha256, sha384, sha512, blake3), `randomBytes`, `randomUUID`, `timingSafeEqual`. Returns `Uint8Array`. |
| `crypto` AES ciphers | G43, G51, G52 | `createCipheriv` / `createDecipheriv` for AES-CBC, AES-GCM, and AES-CTR. |
| `zlib` | G21, G48, G58 | Sync and callback gzip, deflate, and raw deflate. |

KDF, sign and verify, key pairs, `KeyObject`, brotli, and streaming `createGzip` are not Stable.

## Processes and networking

| API | Id | Notes |
| :--- | :--- | :--- |
| `child_process` sync | G25 | `execSync`, `spawnSync`. `pid` is `0`. |
| `child_process` async | G47, G60, G67 | `exec`, `execFile`, `spawn` with captured output, and `fork` with JSON IPC. No live stdio pipes. |
| `http` server | G37 (tentative) | `createServer`, `listen`, buffered responses, keep-alive. |
| `http` client | G72 | `request` / `get`, one buffered HTTP/1.1 body. No `https`, Agent, or chunked streaming. |
| `perf_hooks` | G74 | `performance`, `PerformanceObserver`, `timerify`. |
| `worker_threads` | G75 | Real OS thread, JSON `postMessage`. No `transferList` or nested workers. |
| `net` server and client | G45, G61 (tentative) | `createServer` listen and accept; `net.connect` client. |
| `dns` | G27, G65 | `lookup`, `resolve*`, `reverse`, `getServers` from the OS resolver. |
| `url` | G26 | POSIX `fileURLToPath` and `pathToFileURL`. Legacy `parse` is outside. |
| `stream` | G24, G71 | `Readable`, `Writable`, `PassThrough`, `Transform`, and callback `pipeline`. No `Duplex` or `stream/promises`. |

## Not supported

These fail or reject instead of pretending to work:

- `http.Agent`, the `https` module, and `http2`.
- Streaming `zlib.createGzip` family, brotli, and `zlib.constants`.
- Node native addons beyond the Experimental N-API hello loader.

## Conformance

The Node conformance scorecard in `tests/conformance/` is 56/56 on main. See [Stability levels](/docs/stability) for what that number does and does not mean.

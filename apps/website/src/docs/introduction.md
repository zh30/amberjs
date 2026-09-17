---
title: "Overview"
subtitle: "A JavaScript/TypeScript runtime in Rust and V8. One binary: amber."
group: "Getting Started"
id: "introduction"
---

## What is Amber?

**Amber** is a JavaScript and TypeScript runtime built with **Rust** and **Google V8**. It ships as a single executable named `amber`.

It is built for **agent tools and sandboxed scripts**: run TypeScript without `tsc`, run Jest-style tests, host MCP/JSON-RPC tools, and do in-process tensors via `amber:ai`. It is **not** a drop-in Node.js replacement.

Use Amber when you want:

- **One binary** for `run` / `eval` / `test` / `repl` / `mcp`
- **TypeScript without `tsc`** (oxc transpile-only)
- **Capability sandbox** for agent tools (`--sandbox`, `--seed`, `--freeze-time`)
- **In-process tensors** via `amber:ai` (no Python sidecar)

Closest cousins: Deno (V8 + Rust, permissioned) and Bun (all-in-one CLI). Amber keeps V8, adds an agent/MCP host, and labels every command Stable / Preview / Experimental.

---

## Architecture

```text
+-------------------------------------------------------------------+
|  Application  —  JS / TS  ·  tests  ·  MCP tools  ·  HTTP fetch   |
+-------------------------------------------------------------------+
|  Runtime APIs                                                     |
|    Node compat (fs, http, buffer, …)                              |
|    Web APIs (fetch, Streams, URL, Web Crypto)                     |
|    amber:ai (Tensor, LLM, AgentPipeline)                            |
+-------------------------------------------------------------------+
|  Engine  —  V8 isolate  ·  oxc TS/TSX  ·  Tokio I/O               |
+-------------------------------------------------------------------+
|  Host  —  capability broker  ·  snapshots  ·  Wasm backing stores |
+-------------------------------------------------------------------+
```

---

## What is in v1.16.0

| Area | Status | Notes |
| :--- | :--- | :--- |
| `amber run` / `eval` / `repl` | **Stable** | JS always; TS/TSX via oxc (Preview contract) |
| `amber test` | **Stable** | Jest-style `describe` / `test` / `expect` |
| `amber:ai` | **Stable** | Tensor / LLM / AgentPipeline in-process |
| `--sandbox` / MCP / session | **Preview** | Default-deny I/O, seed, freeze-time |
| `amber serve` | **Preview** | WinterCG `fetch` handler; `--https` is rustls HTTP/1.1 |
| `amber bundle` / `amber compile` | **Preview** | oxc graph bundle; SEA trailer `AMBER_STANDALONE` |
| `amber:wasm` | **Preview** | Zero-copy Memory / ArrayBuffer |
| Package manager (`init`/`install`/`x`) | **Experimental** | Lightweight; not npm-complete |
| Node API surface | **Preview** | Per-API. Conformance 5.0 is **55/55** |

The only user-facing capability map is [Current Scope](https://github.com/zh30/amberjs/blob/main/docs/CURRENT_SCOPE.md) in the repo. Historical stage reports are not product promises.

---

## Comparison

| | Amber 1.16.0 | Node.js | Bun | Deno |
| :--- | :--- | :--- | :--- | :--- |
| Engine | V8 + Rust | V8 + C++ | JSC + Zig | V8 + Rust |
| TypeScript | oxc, transpile-only | loaders / `tsc` | built-in | built-in |
| Secure defaults | opt-in `--sandbox` | none | none | permission flags |
| Node API | incremental Preview | native | drop-in goal | compat layer |
| Test runner | built-in `amber test` | external | `bun test` | `deno test` |
| Native AI | `amber:ai` | — | — | — |
| Conformance scorecard | 55/55 fixtures | native | high | high |

Coverage is **per-API**, not “Node compatible.” Modules that exist today include `fs`, `path`, `os`, `url`, `buffer`, `events`, `stream`, `crypto`, `http`, `http2`, `net`, `child_process` (`execSync` / `spawnSync`), `zlib`, `util`, `worker_threads`. Web: `fetch`, Streams, Web Crypto, URL, `Worker`.

---

## Performance

Public numbers belong in the repo `benchmarks/` directory with hardware, command, and a correctness check. On Apple M2 Max (2026-09-16, Amber vs Node v22.22.3 vs Bun 1.4.1):

- URL + URLSearchParams (20k): Amber **7.28 ms**, Node 12.54 ms
- fetch 100 sequential GETs: Amber **6.51 ms**, Node 17.00 ms
- ReadableStream 5k chunks: Amber **0.92 ms**, Node 1.16 ms
- Express 5.x: Amber **~68k req/s**, Node ~19k req/s
- CLI `eval 1+1` mean: Amber **18.47 ms**, Node 27.53 ms, Bun 8.03 ms

Those are one machine, one commit. They are not SLAs. Bun still wins several microbenches and cold start.

---

## Next

1. [Install](/docs/installation)
2. [Quick start](/docs/quick-start)
3. [CLI reference](/docs/cli-usage)

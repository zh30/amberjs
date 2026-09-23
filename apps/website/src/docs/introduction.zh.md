---
title: "概览"
subtitle: "Rust + V8 的 JavaScript/TypeScript 运行时。一个二进制：amber。"
group: "开始"
id: "introduction"
---

## Amber 是什么？

**Amber** 是用 **Rust** 和 **Google V8** 构建的 JavaScript / TypeScript 运行时，发布形态是单一可执行文件 `amber`。

它面向 **Agent 工具与沙箱脚本**：不用 `tsc` 就能跑 TypeScript，内置 Jest 风格测试，通过 MCP / JSON-RPC 托管工具，并用 `amber:ai` 做进程内张量。它 **不是** Node.js 的即插即用替代品。

适合这些场景：

- **一个二进制** 覆盖 `run` / `eval` / `test` / `repl` / `mcp`
- **不用 `tsc` 的 TypeScript**（oxc 只做类型擦除）
- **能力沙箱**（`--sandbox`、`--seed`、`--freeze-time`）
- **进程内张量** `amber:ai`（不需要 Python sidecar）

最近的参照物：Deno（V8 + Rust，权限模型）和 Bun（一体化 CLI）。Amber 保留 V8，加上 Agent / MCP 宿主，并且每个命令都标了 Stable / Preview / Experimental。

---

## 架构

```text
+-------------------------------------------------------------------+
|  应用层  —  JS / TS  ·  测试  ·  MCP 工具  ·  HTTP fetch          |
+-------------------------------------------------------------------+
|  运行时 API                                                       |
|    Node 兼容（fs, http, buffer, …）                               |
|    Web API（fetch, Streams, URL, Web Crypto）                     |
|    amber:ai（Tensor, LLM, AgentPipeline）                           |
+-------------------------------------------------------------------+
|  引擎  —  V8 isolate  ·  oxc TS/TSX  ·  Tokio I/O                 |
+-------------------------------------------------------------------+
|  宿主  —  能力经纪  ·  快照  ·  Wasm backing store                |
+-------------------------------------------------------------------+
```

---

## v1.16.0 里有什么

| 能力 | 状态 | 说明 |
| :--- | :--- | :--- |
| `amber run` / `eval` / `repl` | **Stable** | JS 始终可用；TS/TSX 走 oxc（契约仍是 Preview） |
| `amber test` | **Stable** | Jest 风格 `describe` / `test` / `expect` |
| `amber:ai` | **Stable** | 进程内 Tensor / LLM / AgentPipeline |
| `--sandbox` / MCP / session | **Preview** | 默认拒绝 I/O，可冻结时钟与 PRNG |
| `amber serve` | **Preview** | WinterCG `fetch` 处理器；`--https` 是 rustls HTTP/1.1 |
| `amber bundle` | **Stable** | 本地 JS/TS/JSON 图 → 单个 IIFE。限制见 [Current Scope](https://github.com/zh30/amberjs/blob/main/docs/CURRENT_SCOPE.md) |
| `amber compile` | **Stable** | 宿主 SEA。Trailer `AMBER_STANDALONE`。不是 pkg/nexe/Bun |
| `amber:wasm` | **Preview** | Memory / ArrayBuffer 零拷贝 |
| `amber install` | **Stable** | 直接安装 `package.json` 依赖并核对 lock 的 `dependencies`。不是 npm/yarn/pnpm。`add` / `init` / `x` 仍是 Experimental |
| Node API | **Preview** | 按 API 计。Conformance 5.0 为 **55/55** |

用户侧能力边界以仓库里的 [Current Scope](https://github.com/zh30/amberjs/blob/main/docs/CURRENT_SCOPE.md) 为准。历史阶段报告不是产品承诺。

---

## 对比

| | Amber 1.16.0 | Node.js | Bun | Deno |
| :--- | :--- | :--- | :--- | :--- |
| 引擎 | V8 + Rust | V8 + C++ | JSC + Zig | V8 + Rust |
| TypeScript | oxc，仅转译 | loaders / `tsc` | 内置 | 内置 |
| 安全默认 | 可选 `--sandbox` | 无 | 无 | 权限开关 |
| Node API | 增量 Preview | 原生 | 目标 drop-in | 兼容层 |
| 测试 | 内置 `amber test` | 外部 | `bun test` | `deno test` |
| 原生 AI | `amber:ai` | — | — | — |
| 符合度记分牌 | 55/55 fixtures | 原生 | 高 | 高 |

覆盖是 **按 API** 的，不是「兼容 Node」。当前存在的模块包括 `fs`、`path`、`os`、`url`、`buffer`、`events`、`stream`、`crypto`、`http`、`http2`、`net`、`child_process`（`execSync` / `spawnSync`）、`zlib`、`util`、`worker_threads`。Web：`fetch`、Streams、Web Crypto、URL、`Worker`。

---

## 性能

公开数字必须能在仓库 `benchmarks/` 里复现（硬件、命令、正确性检查）。Apple M2 Max（2026-09-16，Amber vs Node v22.22.3 vs Bun 1.4.1）：

- URL + URLSearchParams（20k）：Amber **7.28 ms**，Node 12.54 ms
- fetch 连续 100 次 GET：Amber **6.51 ms**，Node 17.00 ms
- ReadableStream 5k chunks：Amber **0.92 ms**，Node 1.16 ms
- Express 5.x：Amber **约 68k req/s**，Node 约 19k req/s
- CLI `eval 1+1` 均值：Amber **18.47 ms**，Node 27.53 ms，Bun 8.03 ms

这是一台机器、一次提交，不是 SLA。Bun 在若干微基准和冷启动上仍然更快。

---

## 接下来

1. [安装](/docs/installation)
2. [快速开始](/docs/quick-start)
3. [CLI 参考](/docs/cli-usage)

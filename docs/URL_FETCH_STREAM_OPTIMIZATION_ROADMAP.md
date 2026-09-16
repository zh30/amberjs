# URL / Fetch / ReadableStream 专项性能突破与 RSI 优化路线图

> **目标**: 针对全景基准 2.0 暴露的三项短板，采用递归自我改进 (RSI) 逐项突破。每项独立分支、独立基准、独立合并。
>
> **基线环境**: Beejs 1.15.0 vs Node.js v22.22.3 vs Bun 1.4.1，Apple M2 Max。数据来自 `benchmarks/COMPREHENSIVE_BENCHMARK_REPORT.md`（提交 `289e192c`）。

---

## 待办清单

| 任务编号 | 优化维度 | 对标 | 初始差距 | 核心突破思路 | 分支 | 实测与状态 |
| --- | --- | --- | --- | --- | --- | --- |
| **TASK-URL** | WHATWG `URL` / `URLSearchParams` 构造与查询串吞吐 | Node 12.38 ms | Beejs 234.21 ms（慢 **19.6x**） | 去掉 BeeURL 二次包装；用 JIT 友好的 `url_fast.js` 解析绝对 URL | `feat/rsi-url-native` | **已完成**：8.17 ms，比 Node 快 **1.51x**（相对基线提升 28.7x）。Conformance 55/55 |
| **TASK-FETCH** | `fetch()` 串行客户端 GET | Node 18.19 ms / 100 req | Beejs 1451.62 ms（慢 **43.7x**） | 字符串 GET 走线程局部 HTTP/1.1 keep-alive，避免 Tokio `block_on` | `feat/rsi-fetch-fastpath` | **已完成**：8.22 ms，比 Node 快 **2.21x**（相对基线提升 176x）。Conformance 55/55 |
| **TASK-STREAM** | `ReadableStream` 生产/消费 5k chunk | Node 1.28 ms | Beejs 4.53 ms（慢 **4.4x**） | 默认 enqueue/read 热路径改 JS 实现，避免每 chunk 一次 Rust FFI | `feat/rsi-stream-js-hotpath` | **已完成**：0.95 ms，比 Node 快 **1.35x**（相对基线提升 4.8x）。Conformance 55/55 |

---

## 根因（源码事实，不是猜测）

### TASK-URL

- `src/nodejs_core/url.rs` 与 `src/web_api/url.rs` 都在启动时 `global.set("URL", FunctionTemplate)`，覆盖 V8 15.2 自带的 WHATWG URL。
- 每次 `new URL()` 都跨 JS→Rust：`to_rust_string_lossy` + 手写/`Url::parse` + 十余次 `v8::String::new` / `obj.set`。
- 基准 `16. URL & URLSearchParams (20k)` 正打在这条 FFI 热路径上。Node/Bun 走 ada / V8 原生解析，无逐次 FFI。

### TASK-FETCH

- `src/web_api/fetch.rs` 的 `fetch_callback` 在 V8 回调里 `get_fetch_runtime().block_on(execute_fetch(...))`，再同步拼完整 Response/Headers。
- 每次请求：`permissions::check_global_permission` 克隆 URL、reqwest 组包、2 worker Tokio runtime 从非 runtime 线程 `block_on`。
- 100 次 `await fetch(localhost)` 变成 100 次停顿式阻塞，约 14.5 ms/req vs Node undici ~0.3 ms/req。

### TASK-STREAM

- `src/web_api/streams.rs` 用 `FunctionTemplate` 实现 `ReadableStream` / `getReader` / `read`。
- 基准 `20. ReadableStream produce & consume (5k chunks)` 对每个 chunk `enqueue` + `reader.read()`，即约 1 万次 Rust 回调，无法被 Maglev 内联。

---

## RSI 运作规范

对每一项严格按下列循环：

1. `git checkout -b feat/rsi-<item>`
2. 记录改动前微基准（Beejs / Node / Bun）
3. 针对 FFI、block_on、隐藏类、多余分配设计突破方案
4. 落地实现；`tests/conformance` 保持 55/55
5. 复测对应 workload，必须有实质性提升（至少明显缩小与 Node 的倍数差）
6. 提交并以 `--no-ff` 合入 `main`，在本表填写实测
7. 启动下一项

验收门槛：

- **TASK-URL**: 20k URL+searchParams 进入 Node 同量级（目标 **< 25 ms**，相对 Node 慢于 2x 以内）
- **TASK-FETCH**: 100 次串行 localhost GET 目标 **< 80 ms**（相对 Node 慢于 3x 以内）
- **TASK-STREAM**: 5k chunk 目标 **< 1.8 ms**（相对 Node 慢于 1.5x 以内）

---

## 验证命令

```bash
# URL
./target/release/bee run benchmarks/comprehensive_bench.js   # 看第 16 项
node benchmarks/comprehensive_bench.js

# Fetch + SQLite（需 Phase 5 本地 HTTP）
python3 benchmarks/run_comprehensive_benchmark.py

# Stream
# 同一 comprehensive_bench.js 第 20 项

./tests/conformance/run_conformance.sh
```

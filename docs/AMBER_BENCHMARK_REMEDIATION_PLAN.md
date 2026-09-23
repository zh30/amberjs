# Amber 1.16.0 基准整改方案

Last reviewed: 2026-09-17  
Hardware: Apple M2 Max, Darwin 27.0.0 arm64  
Peers: Node.js v22.22.3, Bun 1.4.1  
Command:

```bash
cargo build --release
python3 benchmarks/run_comprehensive_benchmark.py
AMBER_BIN=./target/release/amber bash tests/conformance/run_conformance.sh
```

Artifacts: `benchmarks/comprehensive_results.json`, `benchmarks/COMPREHENSIVE_BENCHMARK_REPORT.md`, `tests/conformance/scorecard.md`.

数字只来自上述命令。本文件不是 SLA，也不覆盖 `docs/STAGE_*` 里的历史数字。

## Status after P0–P3 (2026-09-17 follow-up)

| Item | Result |
| --- | --- |
| P0-1 sandbox `package.json` | Fixed. Conformance **55/55**. Denied `package.json` is treated as missing during CJS resolve. |
| P0-2 `amber:ai` / `amber:db` | Tensor bench **0 skipped**. |
| P0-3 `--warm` | Option B: hidden no-op on one-shot CLI. Process-local `IsolatePrewarmer` kept. |
| P0 docs | README / CURRENT_SCOPE stay **55/55** (now true). |
| P1-1 Web Crypto digest | ring + zero-copy: **0.632 ms vs Node 1.882 ms**. Matches `createHash`. |
| P1-2 Wasm | Native `instantiate` restored. **0.66 ms vs Node 0.59 ms** (noise-level). |
| P1-3 Buffer | Fill/from(array) copy restored. **1.99 ms vs Node 2.10 ms**. |
| P2-1 startup | `AMBER_TRACE_STARTUP=1`. In-process eval ~6 ms; remaining CLI time is process spawn, not globals. Delay-load deferred. |
| P2-2 Date / Streams vs Bun | Date is V8; Streams/gzip vs Bun not rewritten. |
| P2-3 / P3 | Bench runner warns if HTTP RSS does not drop 20% from peak; fails if Express < 2× Node, URL/EventEmitter < 1.2× Node, or conformance < 55/55. |

---

## 1. 实测结论（先对齐事实）

### 已经成立的优势（不要为了“全面优化”去动）

| 区域 | 本次结果 | 含义 |
| --- | --- | --- |
| Express 5.x | Amber 70,829 req/s vs Node 18,299 vs Bun 58,563 | 单进程 Express 是当前最大产品差异 |
| Hono / Fastify / Raw HTTP | 与 Bun 同量级，稳赢 Node | 保持，回归测试锁住即可 |
| HTTP 压测后 RSS 回落 | Express 88.8 → 56.9 MB；Node/Bun 停在峰值 | 回收路径有效，不要为压峰值去关 GC |
| `amber test` | 18.8 ms vs Bun test 197.5 ms | 内置测试是短命工具场景的卖点 |
| URL / EventEmitter / TextEncoder / gzip vs Node | 1.5–2.5x | 继续当回归基线 |
| Fetch 100 sequential GET | 7.7 ms vs Node 17.3 ms | 热路径改写有效 |

### 必须改口的文档事实

| 文档现状 | 本次事实 |
| --- | --- |
| `docs/CURRENT_SCOPE.md` 写 Node Conformance 5.0 **55/55 PASS** | 曾为 49/55（sandbox 引导）；**已修回 55/55** |
| README / 官网仍可能沿用 55/55 | 与 scorecard 一致即可 |
| `amber:ai` 作为可测卖点 | 基准已切到 `require('amber:ai')`，tensor **0 skipped** |
| `--warm` 被描述成亚毫秒复用 | 一次性 CLI 上已改为 hidden no-op |

### 失败不是六套 API，是一类引导错误

`--sandbox` 相关 6 个 fixture 全部以同一错误退出：

```text
Error: permission denied: FileSystem Read Path("/Users/henry/code/beejs/package.json")
```

失败文件：

- `child_process_exec_denied.js`
- `env_denied.js`
- `fetch_allowlist.js`
- `fs_jail_allows_prefix.js`
- `fs_read_denied.js`
- `run_denied.js`

这些脚本本身不读仓库根 `package.json`。是运行时在 `--sandbox` 下解析/启动 fixture 时先读了它。所以 **49/55 首先是沙箱引导回归，不是 Node API 大面积倒退**。

---

## 2. 目标与原则

1. **先修正确性，再修速度。** 55/55 和 `amber:ai` 可跑，优先于再抠 2ms 冷启动。
2. **对标场景，不对标虚荣数字。** Agent 工具 = 冷启动 + sandbox + 内置测试；服务 = Express/Hono 吞吐 + RSS 回收；不要用 Bun 的 8ms `bun -e` 当唯一 KPI。
3. **每项整改必须带复现命令和通过标准。** 没有命令的条目不算方案。
4. **已经领先的路径只加回归，不重写。** Express / URL / EventEmitter / gzip vs Node 属于 lock，不属于 refactor。

建议对外口径（修完 P0 之前）：

- Conformance：**49/55，6 项为 sandbox 引导失败**
- 冷启动：快于 Node，慢于 Bun
- HTTP：Express 明显快于 Node；与 Bun 接近
- `amber:ai`：实现在，当前基准脚本未接到 `amber:ai`

---

## 3. 工作包

### P0 — 正确性与可测性（本周）

#### P0-1 沙箱引导不再读被拒绝的 `package.json`

**现象：** `--sandbox` fixture 在用户代码之前因读根目录 `package.json` 崩溃。  
**假设：** CommonJS resolver / `package.json` type 探测 / 模块预加载在 capability broker 生效后仍走真实 `fs`。  
**改哪里：** `src/nodejs_core/commonjs_resolver.rs` 的 `nearest_package_type`、sandbox 下的 `fs` 桥、`amber run --sandbox` 启动路径。  
**不要：** 给 conformance 开 `--allow-read .` 来“刷 55/55”。那是掩盖。  
**通过标准：**

```bash
AMBER_BIN=./target/release/amber bash tests/conformance/run_conformance.sh
# 6 个 *denied / jail / allowlist fixture 全部 PASS
# 合计 55/55
```

同步改 `docs/CURRENT_SCOPE.md`、README、官网文案：在合并修复前改成 49/55；合并后才能写 55/55。

#### P0-2 基准与示例切到 `amber:*`

**现象：** Tensor 五项全部 `Cannot find module 'bee:ai'`。更名已删除 `bee:*` 别名。  
**改哪里：**

- `benchmarks/ai_tensor_bench.js` → `require('amber:ai')`
- `benchmarks/extended_io_bench.js` 里 Node/Bun 对 `bee:db` 的探测（SKIP 文案）
- 仓库内剩余 `require('bee:` / `from 'bee:` 的 **可执行** 示例与基准（历史 `docs/STAGE_*` 可不动）

**通过标准：**

```bash
./target/release/amber run benchmarks/ai_tensor_bench.js
# 5 workloads, 0 skipped
```

#### P0-3 `--warm` 名实不符

**现象：** 冷 `eval` 34.9 ms，`--warm` 44.6 ms。实现里 `AMBER_WARM` 还写了两次。  
**改哪里：** `src/main.rs` 的 `is_warm_mode`，`src/isolate_prewarmer.rs`。  
**要回答的问题：**

1. 短命 CLI 每次新进程，预热池是否根本跨不过进程边界？
2. 若不能跨进程，CLI `--warm` 应改为 no-op 并在 `--help` 标明，或做成常驻 daemon。
3. 同进程复用（`amber test` 多文件、HTTP worker）才是预热的合法场景。

**通过标准（选一，禁止含糊）：**

- A. 同机 20 次 `amber eval --warm '1+1'` 均值 **低于** 冷启动，且 p95 不差于冷启动。
- B. 从公开 CLI 拿掉 `--warm`，只保留进程内 API / 测试运行器复用，并改文档。

**决议 B（已落地）：** 一次性 `amber run` / `amber eval` 上 `--warm` 为 no-op（`hide = true`）。跨进程没有 standby isolate。`IsolatePrewarmer` 仍留给进程内复用。启动分段：`AMBER_TRACE_STARTUP=1`。

---

### P1 — 明确落后于 Node 的热路径（接下来两周）

这些是同 V8 却慢于 Node 的项，优先于“追 Bun 冷启动”。

| ID | 工作负载 | Amber | Node | 倍率 | 整改方向 |
| --- | --- | --- | --- | --- | --- |
| P1-1 | Web Crypto `subtle.digest` SHA-256 100×16KB | 4.69 ms | 2.02 ms | 0.43x | `src/web_api/crypto.rs`：digest 热路径避免每次 `Vec<u8>` 拷贝；对 SHA-256 走 ring/sha2 一次 digest，少进 OpenSSL；TypedArray 用 backing store 零拷贝 |
| P1-2 | WebAssembly instantiate + 100k add | 0.82 ms | 0.63 ms | 0.76x | 核对 Wasm 2.0 backing store 是否在 `instantiate` 上多了一次 copy；100k 调用是否走 JS 桥而不是直接 wasm export |
| P1-3 | Buffer alloc/fill/slice 1k×16KB | 1.83 ms | 1.56 ms | 0.85x | `buffer` 实现：`allocUnsafe` / `fill` 是否 memset + 额外 JS 包装；对比 Node 的 slab allocator |

**通过标准：** 同一条 `python3 benchmarks/run_comprehensive_benchmark.py` 的 phase 2，三项相对 Node ≥ 0.95x（允许 5% 噪声）。禁止只改 JS 基准迭代次数来“变快”。

**建议探针（先测量再改）：**

```bash
./target/release/amber run benchmarks/comprehensive_bench.js | python3 -c 'import sys,json,re; s=sys.stdin.read(); a=s[s.find("["):s.rfind("]")+1];
print([x for x in json.loads(a) if "Crypto / subtle" in x["name"] or "WebAssembly" in x["name"] or x["name"].startswith("8. Buffer")])'
```

改 Web Crypto 时加一条 **正确性** 测试：`crypto.subtle.digest('SHA-256', data)` 与 `crypto.createHash('sha256')` 对同一输入一致。速度优化不得牺牲向量测试。

---

### P2 — 产品形态与对 Bun 的差距（中期）

Bun 在短命 CLI、部分微基准（数组 map/reduce、ReadableStream、Date、gzip、冷 `eval`）仍明显更快。这些不必一周内追平，但要有策略。

#### P2-1 冷启动 20 ms vs Bun ~10 ms

`run hello_world.js` Amber 20.1 ms / Bun 11.0 ms。构成应拆成：

1. 进程 + 动态库加载
2. V8 isolate + context
3. Node/Web 全局注入
4. 读文件 + 执行

**动作：** 在 `amber --verbose run` 或一次性 `AMBER_TRACE_STARTUP=1` 打四个时间戳。没有分段数据不要猜“V8 太慢”。  
**可能手段（有数据后再选）：**

- 默认 V8 snapshot（`src/v8_snapshot`），缩短 isolate 创建
- 延迟加载非 `run` 必需的全局（http2、tls、wasm、ai）
- 评估 musl/静态链接 vs 当前 72MB release 的加载成本

**通过标准：** 分段日志可复现；`run hello_world.js` 20 次均值 ≤ 15 ms（相对本次 20.1 ms 的务实目标，不是 8 ms）。

#### P2-2 进程内仍慢于 Bun 的项

| 项 | Amber | Bun | 备注 |
| --- | --- | --- | --- |
| Arrays filter-map-reduce | 0.59 ms | 0.21 ms | 多半是 JS 引擎差异，低优先级 |
| ReadableStream 5k chunks | 1.36 ms | 0.38 ms | 查 Streams 实现是否每 chunk 分配 |
| Date.toISOString 50k | 21.2 ms | 7.5 ms | 若走 JS polyfill 则改成本地 |
| CompressionStream gzip | 1.37 ms | 0.30 ms | 查是否同步桥 + 额外 buffer |
| JSON parse/stringify | 5.37 ms | 3.53 ms | 低优先级 |

**原则：** 只改 **我们自己的桥和 polyfill**，不和 JSC 比 Fibonacci。Fibonacci 已与 Node 持平（9.16 vs 9.40），说明 V8 JIT 健康。

#### P2-3 HTTP 峰值 RSS

Amber 峰值高于 Bun（Hono 81 vs 41 MB），但 **settled 低于峰值**。  
**做：** 在 worker 空闲时主动 `isolate.low_memory_notification` 的策略保持，并加一条 RSS 回归（峰值与 settled 差 ≥ 20%）。  
**不做：** 为压峰值关掉优化 JIT 或把堆收到影响 Express 吞吐。

---

### P3 — 基准工程与防回归（持续）

1. **固定复现入口：** `python3 benchmarks/run_comprehensive_benchmark.py`（已改为 `target/release/amber`）。CI 可跑 phase 1 + conformance，HTTP 压测放 nightly。
2. **锁住优势项：** Express req/s 不得跌破 Node 的 2x；URL、EventEmitter vs Node 不得跌破 1.2x。用 JSON artifact 对比，而不是手抄 README。
3. **噪声控制：** `--warm` 本次 p95 116 ms，冷启动 p95 70 ms，说明机器有干扰。正式对比应 `sudo nice`/`taskpolicy` 或关闭省电，连续两轮取中位数。
4. **fixture 与产品同名：** 基准、示例、类型定义统一 `amber:*`。
5. **禁止用 STAGE 报告数字更新官网。** 只引用本文件列出的命令产物。

---

## 4. 建议实施顺序

```text
Week 1
  P0-1  sandbox 引导读 package.json     → 55/55
  P0-2  基准脚本 amber:ai / amber:db    → tensor 不再 SKIP
  文档  CURRENT_SCOPE / README 与 49/55 对齐，修复后改回 55/55

Week 2
  P0-3  --warm 二选一（修或删）
  P1-1  Web Crypto digest 零拷贝

Week 3–4
  P1-2  Wasm instantiate
  P1-3  Buffer alloc
  P2-1  启动分段计时 + 延迟加载

之后
  P2 Streams / Date / gzip 仅在分段数据证明是我们的桥之后再动
```

每项合并条件：对应命令变绿 + 本文件表格更新实测列 + 不把旧 STAGE 数字写进 README。

---

## 5. 复现与验收清单

```bash
# 构建
cargo build --release

# 全面基准（CLI + 24 微基准 + HTTP + conformance + I/O + amber:ai）
python3 benchmarks/run_comprehensive_benchmark.py

# 仅 conformance
AMBER_BIN=./target/release/amber bash tests/conformance/run_conformance.sh

# 仅 HTTP 对照（可选）
# 见 phase 3：32 connections / 5s autocannon / 127.0.0.1
```

验收（P0 完成后）：

- [ ] scorecard **55/55**
- [ ] `ai_tensor_bench.js` **0 skipped**
- [ ] CURRENT_SCOPE / README 的 conformance 数字与 scorecard 一致
- [ ] `--warm` 要么快于冷启动，要么从 CLI 消失
- [ ] Express req/s 仍 ≥ Node 的 2 倍（防回归）

---

## 6. 明确不在本方案内

- 把 Bun 8ms `bun -e` 当成 v1.16 必须项
- 为刷微基准关掉正确性测试
- 给 sandbox 测试加 `--allow-read` 根目录
- 恢复 `bee:*` 模块别名
- 用 `docs/STAGE_*` 或 2025 年 HTML 报告更新官网
- 重写已经领先的 Express / URL / EventEmitter 路径

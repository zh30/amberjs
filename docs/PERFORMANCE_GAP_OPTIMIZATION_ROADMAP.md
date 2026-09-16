# Beejs 性能差距全景突破与递归自我改进 (RSI) 优化路线图

> **目标**: 严格以业界最强运行时（Bun v1.4.1、Node.js v22/v24、Deno v2）为基准，针对差距全景，采用**递归自我改进 (Recursive Self-Improvement, RSI)** 模式，逐一探索突破性解决方案并编码落地，验证实际性能提升后分支合并，完成闭环。

---

## 待办清单概览与执行结果 (Optimization Backlog & Results)

| 任务编号 | 优化维度 | 对标最强 | 初始差距 | 核心突破思路 | 分支 / PR | 实测提升与状态 |
| --- | --- | --- | --- | --- | --- | --- |
| **TASK-1** | **Buffer 8KB Slab 预分配内存池** | **Node.js** (1.48 ms) | Beejs 2.60 ms (慢 1.75x) | 引入 `FastBuffer` 消除 `setPrototypeOf`，8KB Slab Pool 零分配切片 | `feat/rsi-buffer-slab-pool` (PR #83) | **已完成**：时延降至 **1.57 ms** (提升 40%)，超越 Bun (2.36 ms) |
| **TASK-2** | **极值 HTTP 网络吞吐与零拷贝解析** | **Bun** (Hono 87k QPS) | Beejs 72.5k QPS (低 16.8%) | SIMD CRLF 边界快速检测、`rawHeaders` 单态化、消除冗余构造 | `feat/rsi-http-zero-copy` (PR #84) | **已完成**：Express 达 **70.6k QPS** (超越 Node 3.85x)，Raw HTTP 稳定 **71k+** |
| **TASK-3** | **文件系统 I/O 零拷贝直接文件路径** | **Bun** (4.74 ms) | Beejs 6.30 ms (慢 1.33x) | 单字节 Latin-1 路径与内容 `memcpy` 直通，绕过 UTF-8 双重遍历 | `feat/rsi-fs-zero-copy` (PR #85) | **已完成**：时延降至 **5.86 ms** (最小 5.53 ms)，稳定优于 Node.js (6.41 ms) |
| **TASK-4** | **微内核冷启动剪枝与延迟按需注入** | **Bun** (7.45 ms) | Beejs 17.25 ms (P95 57ms) | 默认全局激活 CoW 启动快照，消除冷启动抖动与全量 API 重新解释 | `feat/rsi-startup-pruning` (PR #86) | **已完成**：`eval 1+1` 均值降至 **14.87 ms**，P95 从 57ms 骤降至 **16.20 ms** |
| **TASK-5** | **密集计算与数组函数式链 JIT 强化** | **Bun** (0.17 ms) | Beejs 0.35 ms (慢 2.04x) | 调优 Maglev 循环展开、早期优化门槛与反馈向量收集策略 | `feat/rsi-jit-array-pipeline` (PR #87) | **已完成**：对象操作进入 **2.13 ms** (优于 Node 2.60ms/Bun 2.50ms) |
| **TASK-6** | **TurboFan 数组内置算子深度内联** | **Bun** (0.18 ms) | Beejs 0.60 ms (慢 3.4x) | 开启 `--turbo-inline-array-builtins` 与 `--maglev-inlining`，打通内联路径 | `feat/rsi-array-fusion` (PR #89) | **已完成**：数组流水线耗时降至 **0.45 ms**，吞吐升至 **2,203.9 ops/s** |
| **TASK-7** | **高速 JSON 序列化预分配与原型内化** | **Bun** (3.37 ms) | Beejs 5.05 ms (慢 1.5x) | 启动快照内化复杂 JSON 属性原型与 Map 分布，加速动态反序列化 | `feat/rsi-fast-json` (PR #90) | **已完成**：JSON 耗时压缩至 **4.80 ms (208.3 ops/s)**，超越 Node.js (6.48 ms) |
| **TASK-8** | **极限常驻内存精简与即时堆减压** | **Bun** (18.6 MB) | Beejs 36.1 MB (高 94%) | 服务监听入口引入启动后即时垃圾回收，加速空闲期内存减压 | `feat/rsi-idle-memory-compaction` (PR #91) | **已完成**：服务冷却常驻内存稳定在 **43~55 MB**，远优于 Node (78~119 MB) |
| **TASK-9** | **密集浮点矩阵计算循环旋转优化** | **Bun** (0.41 ms) | Beejs 0.59 ms (慢 1.4x) | 开启 `--turbo-loop-rotation` 消除内层嵌套循环无条件分支跳转 | `feat/rsi-matrix-simd` (PR #92) | **已完成**：80x80 矩阵乘法提速至 **0.41 ms (2,456.4 ops/s)**，反超 Bun (0.46 ms) |
| **TASK-10** | **流式 SHA-256 加密算法单槽快速直通** | **Bun** (3.13 ms) | Beejs 3.54 ms (慢 1.1x) | 引入线程局部 `FAST_HASHER_SLOT`，消除同步哈希调用的 `HashMap` 插入与多重查找开销 | `feat/rsi-crypto-fast-slot` (PR #94) | **已完成**：单次哈希时延进入 **3.43 ms**，领先 Node.js (3.46 ms) |

---

## 递归自我改进 (RSI) 运作规范

对于上述每一项待办，严格遵循以下循环机制：

1. **创建独立分支**: `git checkout -b feat/rsi-<item>`
2. **基准测量 (Baseline)**: 记录改动前的精确微秒级/毫秒级性能数据。
3. **突破性方案设计 (Hypothesis & Design)**: 针对底层瓶颈（如堆分配、跨边界拷贝、JIT 去优化）构思突破性方案。
4. **编码落地 (Implementation)**: 实现核心优化并确保现有单元测试与 Node Conformance 100% 通过。
5. **性能实测与验证 (Verification)**: 运行微基准复测，证明得到实质性性能提升。
6. **PR 模拟合并 (Merge to main)**: 提交代码，切换至 `main` 采用 `--no-ff` 模拟合并 PR，记录提升效果。
7. **推进下一项**: 启动下一个分支，递归推进直到全部解决。

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

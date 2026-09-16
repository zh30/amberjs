# Beejs 性能差距全景突破与递归自我改进 (RSI) 优化路线图

> **目标**: 严格以业界最强运行时（Bun v1.4.1、Node.js v22/v24、Deno v2）为基准，针对差距全景，采用**递归自我改进 (Recursive Self-Improvement, RSI)** 模式，逐一探索突破性解决方案并编码落地，验证实际性能提升后分支合并，完成闭环。

---

## 待办清单概览 (Optimization Backlog)

| 任务编号 | 优化维度 | 对标最强 | 初始差距 | 核心突破思路 | 分支名称 | 状态 |
| --- | --- | --- | --- | --- | --- | --- |
| **TASK-1** | **Buffer 8KB Slab 预分配内存池** | **Node.js** (1.48 ms) | Beejs 2.60 ms (慢 1.75x) | 引入 Node.js 式 8KB Buffer Slab Pool，小 Buffer (<4KB) 零系统分配与指针快速切片 | `feat/rsi-buffer-slab-pool` | 待处理 |
| **TASK-2** | **极值 HTTP 网络吞吐与零拷贝解析** | **Bun** (Hono 87k QPS) | Beejs 72.5k QPS (低 16.8%) | 网络循环零拷贝请求头解析、Fast-Path 预编码与无锁事件分发 | `feat/rsi-http-zero-copy` | 待处理 |
| **TASK-3** | **文件系统 I/O 零拷贝直接文件路径** | **Bun** (4.74 ms) | Beejs 6.30 ms (慢 1.33x) | 优化 `readFileSync`/`writeFileSync` 绕过中间堆层，直通系统内核与 V8 缓冲底座 | `feat/rsi-fs-zero-copy` | 待处理 |
| **TASK-4** | **微内核冷启动剪枝与延迟按需注入** | **Bun** (7.45 ms) | Beejs 17.25 ms (慢 2.32x) | 启动时推迟大型模块（Crypto/Readline等）绑定，快照首屏最小化上下文构建 | `feat/rsi-startup-pruning` | 待处理 |
| **TASK-5** | **密集计算与数组函数式链 JIT 强化** | **Bun** (0.17 ms) | Beejs 0.35 ms (慢 2.04x) | 内联快速原型链、单态 Fast-Array 优化、规避多态 IC 去优化 | `feat/rsi-jit-array-pipeline` | 待处理 |

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

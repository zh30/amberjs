---
title: "Agent 确定性沙箱与资源硬配额 (Deterministic Sandbox & Quotas)"
subtitle: "CPU Watchdog 硬中断、物理堆配额、伪随机数重放与时间戳冻结"
group: "Agent 与高级特性"
id: "agent-sandbox"
---

在运行由自主 AI Agent 实时生成的代码，或在多租户云端沙箱中执行不可信任务时，仅仅限制网络或文件是不够的：恶意脚本或 Agent 幻觉可能会写出死循环（如 `while(true){}`）导致整个 CPU 跑满挂起，或者发生急速内存泄漏导致整机 OOM。

Beejs 构建了专为 Agent 设计的**确定性执行沙箱与硬件级资源配额系统**。

---

## 1. 四大约束维度一览

| 参数标志 | 约束维度 | 底层核心机制 |
| :--- | :--- | :--- |
| **`--timeout <MS>`** | **CPU 超时硬中断** | 独立后台 Watchdog 线程 + `IsolateHandle::terminate_execution()` |
| **`--max-memory <MB>`** | **物理堆内存配额** | V8 `ResourceConstraints` 硬限制老年代堆上限 |
| **`--seed <U64>`** | **确定性随机数回放** | Mulberry32 确定性伪随机发生器接管 `Math.random()` |
| **`--freeze-time <TS>`** | **时间戳绝对冻结** | 接管 `Date.now()`，时间完全恒定 |

---

## 2. 详细使用与技术原理

### 2.1 CPU 超时 Watchdog 强行中断 (`--timeout`)
在传统 JavaScript 引擎中，一旦执行进入纯 CPU 密集死循环，事件循环机制将完全失效。Beejs 采用 **Isolate 级外部看门狗监控架构**：

```bash
$ bee run --timeout 2000 infinite_loop.ts
```

当执行超过 2000 毫秒后，独立的 Watchdog 线程将向 V8 虚拟机发送硬件中断信号，安全强制展开调用栈并退出：
```text
Error: Execution timed out after 2000ms
```

### 2.2 物理堆内存硬配额 (`--max-memory`)
防止 Agent 脚本无节制创建超大数组或缓冲区挤爆宿主服务器内存：

```bash
# 严格限制 V8 堆内存不能超过 128 MB
$ bee run --max-memory 128 mem_heavy_task.ts
```

### 2.3 确定性随机回放 (`--seed`)
在评估 Agent 决策树、A/B 测试或进行复杂模拟时，随机性不可控会导致结果无法复现。通过 `--seed` 注入种子：

```bash
$ bee run --seed 123456789 simulation.ts
```

无论在何时、何种硬件平台上重新运行该命令，`Math.random()` 产生的伪随机数序列永远 100% 绝对一致。

### 2.4 时间戳冻结 (`--freeze-time`)
在生成快照或评估与时间相关的逻辑时，可以通过该选项锁定系统时间戳：

```bash
# 锁定到 2026-01-01 00:00:00 UTC (1767225600000 ms)
$ bee run --freeze-time 1767225600000 test_date.ts
```

脚本内调用 `Date.now()` 将恒定返回指定的毫秒数，彻底消除测试时区和当前时钟的副作用。

---

## 3. 典型 Agent 生产沙箱启动范式

在生产 Agent 调度器中，推荐使用如下全套沙箱标志启动用户任务：

```bash
$ bee run \
    --sandbox \
    --timeout 3000 \
    --max-memory 256 \
    --seed 42 \
    --allow-read ./workspace \
    --allow-write ./workspace/output \
    agent_task.ts
```

**达成的安全防护**：
1. **死循环免疫**：3 秒内强制中断；
2. **OOM 免疫**：堆使用限制在 256MB 以内；
3. **确定性重演**：随机种子固定，每次模拟决策轨迹一致；
4. **I/O 目录完全封锁**：仅允许读写 `./workspace`，严禁越权访问宿主敏感文件与外网连接。

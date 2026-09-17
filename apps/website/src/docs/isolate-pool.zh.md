---
title: "多租户高密度 IsolatePool (bee:pool)"
subtitle: "专为多 Agent 隔离与 Serverless 微任务设计的高密度独立 V8 执行池"
group: "Core Systems"
id: "isolate-pool"
---

## 1. 架构定位与核心价值

在多智能体（Multi-Agent）调度、微服务或 Serverless 边缘计算场景中，系统往往需要高频执行第三方或多租户提供的 JavaScript/TypeScript 代码，要求兼顾**绝对的运行时隔离、极低的常驻内存以及亚毫秒级的任务分发延迟**。

传统方案若通过多进程（Child Process）或容器隔离，往往需要几十兆内存与数百毫秒冷启动。**Beejs v1.4.0 推出了全新的 `bee:pool` 模块**，在 Rust 内核层面实现线程级 V8 `IsolatePool`。

### 核心亮点
- **真正的多租户隔离**：每个工作线程独占独立的 V8 堆空间与垃圾回收器，单个租户的全局变量污染、原型修改或内存异常绝不会影响其他任务。
- **工作线程预热池化**：支持设置最小预热 Isolates，常驻线程复用，彻底规避脚本运行前的冷启动开销。
- **严格的任务超时控制**：支持为每个任务设定精确毫秒级超时，防止死循环或恶意任务耗尽资源。
- **实时运行度量**：随时查询活跃任务数、成功数、失败数与创建总数。

---

## 2. 快速上手：并发隔离执行

通过 `bee:pool` 导入 `IsolatePool`：

```typescript
import { IsolatePool } from 'bee:pool';

// 创建线程隔离池：预热 2 个隔离实例，最高允许 8 个并发实例
const pool = new IsolatePool({
  minIsolates: 2,
  maxIsolates: 8,
  maxMemoryMb: 128,
  timeoutMs: 5000,
});

// 并发在独立的 V8 堆中执行任务
const task1 = pool.run("30 * 40");
const task2 = pool.run("JSON.stringify({ agent: 'bee', isolated: true })");

const [res1, res2] = await Promise.all([task1, task2]);

console.log('任务 1 结果:', res1); // 1200
console.log('任务 2 结果:', res2); // { agent: 'bee', isolated: true }

// 实时获取执行统计
const stats = pool.stats();
console.log('池运行指标:', stats);
// { active: 0, tasksCompleted: 2, tasksFailed: 0, totalCreated: 2 }

// 完成后优雅销毁隔离池
pool.destroy();
```

---

## 3. 配置参数详解

| 参数项 | 类型 | 默认值 | 说明 |
| :--- | :--- | :--- | :--- |
| `minIsolates` | `number` | `1` | 启动时预热的工作 Isolates 线程数 |
| `maxIsolates` | `number` | `8` | 允许动态扩展的最大工作 Isolates 线程数 |
| `maxMemoryMb` | `number` | `128` | 每个 Isolate 堆内存最大限额（MB） |
| `timeoutMs` | `number` | `10000` | 默认单个任务的最大执行超时时间（毫秒） |

---

## 4. 多 Agent 动态工具执行场景

在自动化 Agent 执行动态工具或用户自定义脚本时：

```typescript
import { IsolatePool } from 'bee:pool';

const agentPool = new IsolatePool({ minIsolates: 4, timeoutMs: 3000 });

async function executeAgentTool(code: string, contextData: any) {
  const runner = `
    const ctx = ${JSON.stringify(contextData)};
    (() => {
      ${code}
    })()
  `;

  try {
    return await agentPool.run(runner);
  } catch (err) {
    console.error('Agent 隔离执行违规:', err);
    throw err;
  }
}
```

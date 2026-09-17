---
title: "多 Agent 消息总线与 PubSub 编排网格 (bee:bus)"
subtitle: "进程内高吞吐消息路由、主题通配符分发、请求-响应 RPC 模式与死信队列"
group: "Agent 与高级特性"
id: "agent-bus"
---

在构建复杂的自治多 Agent 体系（如规划者-执行者-审查者三元组、Swarm 蜂群网络及流水线协作图）时，各 Agent 间需要低耦合、异步解耦且具备确定性保证的通信基础底座。传统的直接函数调用会形成高度耦合，极易引发级联阻塞与单点崩溃。

**Beejs v1.8.0 正式引入原生多 Agent 消息总线与 PubSub 编排网格 (`bee:bus`)**。该模块采用纯 Rust 内核驱动，零外部网络及进程开销，提供主题通配符路由（`*`、`#`）、基于关联 ID 的异步请求-响应（RPC）模式、优先级排序分发、中间件拦截管线以及死信队列（DLQ）机制。

---

## 1. 主题通配符与层级路由机制

`bee:bus` 采用标准点分隔层级主题规范：
- **精准匹配**：`agent.planner.task` 仅响应精确发布至该主题的消息。
- **单段通配符 (`*`)**：`agent.*.task` 匹配 `agent.planner.task` 与 `agent.critic.task`，但不匹配 `agent.planner.sub.task`。
- **多段通配符 (`#`)**：`agent.#` 匹配 `agent` 命名空间下的所有深层子主题。

```typescript
import { subscribe, publish, topicMatches } from 'bee:bus';

// 订阅所有规划与评估 Agent 的响应
subscribe('agent.*.response', (message) => {
  console.log(`[${message.topic}] 来自 ${message.id}:`, message.payload);
});

// 订阅所有审计日志事件
subscribe('audit.#', (message) => {
  console.log(`审计追踪 [${message.topic}]:`, message.payload);
});

// 发布消息
publish('agent.planner.response', {
  planId: 'plan_101',
  steps: ['拉取财报', '计算毛利率', '生成汇报图表']
});
```

---

## 2. 异步请求-响应 (Request-Reply RPC 模式)

`bee:bus` 内建开箱即用的双向请求响应语义。请求方在临时生成的私有关联主题上等待响应，内置精确超时熔断保护：

```typescript
import { request, subscribe, reply } from 'bee:bus';

// 1. 服务提供方注册监听并回复
subscribe('service.calculator.add', (msg) => {
  const { a, b } = msg.payload;
  reply(msg, { result: a + b });
});

// 2. 消费方发起异步 RPC 请求并等待返回
async function run() {
  const res = await request('service.calculator.add', { a: 15, b: 27 }, { timeoutMs: 3000 });
  console.log('计算结果:', res.result); // 42
}
```

---

## 3. 优先级队列、中间件与死信队列 (DLQ)

```typescript
import { createBus } from 'bee:bus';

const bus = createBus();

// 1. 挂载分布式链路追踪中间件
bus.use((msg, next) => {
  msg.headers = msg.headers || {};
  msg.headers['x-trace-id'] = `trace_${Date.now()}`;
});

// 2. 注册高优先级紧急熔断监听器
bus.subscribe('task.alert', (msg) => {
  console.log('优先执行高优先级回调');
}, { priority: 10 });

// 3. 检查未被任何消费者路由的消息（死信队列）
const deadLetters = bus.getDeadLetters();
console.log('未路由消息数:', deadLetters.length);
```

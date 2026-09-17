---
title: "Multi-Agent Message Bus & PubSub Channel Fabric (bee:bus)"
subtitle: "High-throughput in-process message routing, topic wildcards, request-reply semantics, and DLQ"
group: "Agent & Advanced"
id: "agent-bus"
---

Autonomous multi-agent architectures (such as Planner-Worker-Critic trios, swarms, and collaborative task pipelines) require decoupled, asynchronous, and high-throughput communication. Traditional point-to-point callbacks tightly couple agent implementations and create fragile distributed state.

**Beejs v1.8.0 introduces the Native Multi-Agent Message Bus & PubSub Channel Fabric (`bee:bus`)**. It provides an in-process, zero-dependency event bus with topic wildcard routing (`*`, `#`), asynchronous request-response RPC semantics, priority queues, middleware interceptors, and dead-letter queue (DLQ) support.

---

## 1. Topic Wildcards & Hierarchical Routing

The `bee:bus` routing engine supports dot-delimited hierarchical topics:
- **Exact matching**: `agent.planner.task` matches only `agent.planner.task`.
- **Single-segment wildcard (`*`)**: `agent.*.task` matches `agent.planner.task` and `agent.critic.task`, but not `agent.planner.sub.task`.
- **Multi-segment wildcard (`#`)**: `agent.#` matches all subtopics like `agent.planner.task`, `agent.a.b.c`, and `agent.event`.

```typescript
import { subscribe, publish, topicMatches } from 'bee:bus';

// Subscribe to all agent responses
subscribe('agent.*.response', (message) => {
  console.log(`[${message.topic}] from ${message.id}:`, message.payload);
});

// Subscribe to all audit topics
subscribe('audit.#', (message) => {
  console.log(`Audit log [${message.topic}]:`, message.payload);
});

// Publish a message
publish('agent.planner.response', {
  planId: 'plan_101',
  steps: ['fetch data', 'analyze', 'summarize']
});
```

---

## 2. Asynchronous Request-Reply (RPC Pattern)

`bee:bus` supports bidirectional request-response semantics out of the box. The requester waits for a response on an ephemeral correlation topic with configurable timeout:

```typescript
import { request, subscribe, reply } from 'bee:bus';

// 1. Service provider registers a handler
subscribe('service.calculator.add', (msg) => {
  const { a, b } = msg.payload;
  reply(msg, { result: a + b });
});

// 2. Requester awaits the response
async function run() {
  const res = await request('service.calculator.add', { a: 15, b: 27 }, { timeoutMs: 3000 });
  console.log('Calculation result:', res.result); // 42
}
```

---

## 3. Priority Queues, Middlewares & Dead-Letter Queue (DLQ)

```typescript
import { createBus } from 'bee:bus';

const bus = createBus();

// 1. Tracing & security middleware
bus.use((msg, next) => {
  msg.headers = msg.headers || {};
  msg.headers['x-timestamp'] = String(Date.now());
});

// 2. High-priority listener
bus.subscribe('task.critical', (msg) => {
  console.log('Executed first');
}, { priority: 10 });

// 3. Inspect unhandled messages via Dead-Letter Queue
const deadLetters = bus.getDeadLetters();
console.log('Unrouted messages:', deadLetters.length);
```

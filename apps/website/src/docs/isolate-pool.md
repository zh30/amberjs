---
title: "Multi-Tenant IsolatePool (bee:pool)"
subtitle: "High-density, multi-tenant V8 execution pool designed for Multi-Agent sandboxing and Serverless micro-tasks"
group: "Core Systems"
id: "isolate-pool"
---

## 1. Overview & Architecture

Modern AI agent workflows, microservices, and serverless edge functions require executing untrusted or multi-tenant JavaScript/TypeScript code concurrently with absolute isolation, minimal memory overhead, and millisecond dispatch latency.

While standard child processes or containers require tens of megabytes of memory and hundreds of milliseconds to boot, **Beejs v1.4.0 introduces `bee:pool`**—a thread-isolated, high-density V8 `IsolatePool`.

### Key Advantages
- **True Multi-Tenant Isolation**: Each worker thread encapsulates an independent V8 heap and garbage collector. Global variables or prototypes modified in one task cannot leak to another.
- **Worker Pre-warming**: Pool maintains a pre-allocated pool of worker isolates, eliminating cold-start compilation overhead.
- **Strict Timeouts & Quotas**: Configurable execution timeouts per task prevent infinite loops or Denial-of-Service attacks.
- **Real-Time Operational Metrics**: Inspect active tasks, completed tasks, failed tasks, and total created isolates on the fly.

---

## 2. Using IsolatePool in JavaScript & TypeScript

Import `IsolatePool` from `bee:pool`:

```typescript
import { IsolatePool } from 'bee:pool';

// Initialize a pool with minimum 2 and maximum 8 warm isolates
const pool = new IsolatePool({
  minIsolates: 2,
  maxIsolates: 8,
  maxMemoryMb: 128,
  timeoutMs: 5000,
});

// Run tasks concurrently in isolated V8 heaps
const task1 = pool.run("30 * 40");
const task2 = pool.run("JSON.stringify({ agent: 'bee', isolated: true })");

const [res1, res2] = await Promise.all([task1, task2]);

console.log('Result 1:', res1); // 1200
console.log('Result 2:', res2); // { agent: 'bee', isolated: true }

// Check pool performance metrics
const stats = pool.stats();
console.log('Pool stats:', stats);
// { active: 0, tasksCompleted: 2, tasksFailed: 0, totalCreated: 2 }

// Gracefully destroy the pool when finished
pool.destroy();
```

---

## 3. Configuration Options

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `minIsolates` | `number` | `1` | Number of worker isolates pre-warmed at startup |
| `maxIsolates` | `number` | `8` | Maximum number of concurrent worker threads |
| `maxMemoryMb` | `number` | `128` | Per-isolate maximum memory heap limit in megabytes |
| `timeoutMs` | `number` | `10000` | Default execution timeout in milliseconds per task |

---

## 4. Multi-Agent Sandboxing Scenario

When orchestrating autonomous agents that execute dynamic JavaScript logic:

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
    console.error('Agent execution sandbox violation:', err);
    throw err;
  }
}
```

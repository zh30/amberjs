---
title: "Persistent Key-Value & Durable State Engine (bee:kv)"
subtitle: "Embedded ACID transactional storage, prefix range scanning, and TTL expiration for autonomous Agents"
group: "Agent & Advanced"
id: "kv-store"
---

Autonomous AI Agents require fast, reliable, and durable memory across isolate recycling, worker task restarts, and session boundaries. Traditional databases like SQLite or PostgreSQL are often too heavy for sub-millisecond key-value caching, while in-memory `Map` instances vanish when the process exits.

**Beejs v1.7.0 introduces the Native Persistent Key-Value & Durable State Engine (`bee:kv`)**. Featuring zero external dependencies, it provides an embedded ACID transactional key-value store with in-memory mode, disk-backed Write-Ahead Log (WAL) persistence, automatic TTL expiration, prefix range scanning, and atomic batch operations.

---

## 1. Core Architecture

The `bee:kv` engine operates in dual modes:

```
[ In-Memory Mode ]                [ Disk-Backed Mode ]
KVStore.openMemory()              KVStore.open("./agent.bee-kv")
        |                                       |
        v                                       v
[ Volatile RAM Index ]             [ RAM Index + Append-Only WAL ]
(Blazing fast, transient)          (Automatic replay, durable reload)
                                                |
                                                v (store.compact())
                                   [ Atomic Snapshot Compaction ]
```

### Key Technical Advantages
- **Zero Heavy Dependencies**: Pure Rust implementation without external SQLite or RocksDB shared library bloat.
- **Durable Write-Ahead Logging (WAL)**: Ensures atomic crash-resilience across process restarts.
- **Built-in TTL Index**: Ephemeral keys automatically expire without periodic external cron jobs.
- **Prefix Range Scans**: Sub-millisecond scanning of hierarchical agent keys (e.g. `agent:session:*`).

---

## 2. Basic Usage

### 2.1 Opening a Store

```typescript
import { KVStore } from 'bee:kv';

// Option A: Fast in-memory store
const memStore = KVStore.openMemory();

// Option B: Persistent disk-backed store with automatic WAL replay
const diskStore = KVStore.open('./data/agent_memory.bee-kv');
```

### 2.2 Storing and Querying Data

Any JSON-serializable value (strings, numbers, objects, arrays) can be stored directly:

```typescript
// Set value with optional TTL expiration in milliseconds
diskStore.set('session:101', {
  user: 'Alice',
  model: 'qwen2.5-7b',
  turns: 4
}, { ttlMs: 60000 }); // Expire after 60 seconds

// Retrieve value
const session = diskStore.get('session:101');
console.log(`Current turns: ${session.turns}`);

// Check existence and delete
if (diskStore.has('session:101')) {
  diskStore.delete('session:101');
}
```

---

## 3. Advanced Capabilities

### 3.1 Prefix Range Scanning

Organize agent memories hierarchically and scan them with a single call:

```typescript
diskStore.set('agent:memory:user_goal', 'Build landing page');
diskStore.set('agent:memory:user_budget', 5000);
diskStore.set('agent:memory:deadline', '2026-10-01');
diskStore.set('agent:config:model', 'gpt-4o');

// Scan all keys starting with 'agent:memory:'
const memories = diskStore.scan({ prefix: 'agent:memory:' });
for (const [key, value] of memories) {
  console.log(`${key} =>`, value);
}

// Limit scan results
const limited = diskStore.scan({ prefix: 'agent:memory:', limit: 2 });
```

### 3.2 Atomic Numeric Increments & Batch Transactions

```typescript
// Atomic increments (safe for counters and rate limits)
const visits = diskStore.incr('page_views', 1);
const tokensUsed = diskStore.incr('agent:tokens', 350);

// Atomic batch commit (all-or-nothing)
diskStore.batch([
  { type: 'put', key: 'checkpoint:step', value: 42 },
  { type: 'put', key: 'checkpoint:status', value: 'completed' },
  { type: 'del', key: 'checkpoint:pending_work' }
]);

// Compact storage log to reclaim disk space
diskStore.compact();
```

---

## 4. API Reference

| Method | Parameters | Return Type | Description |
| :--- | :--- | :--- | :--- |
| `KVStore.open(path)` | `path: string` | `KVStore` | Opens or creates a disk-backed store with WAL |
| `KVStore.openMemory()` | None | `KVStore` | Opens an ephemeral in-memory store |
| `get(key)` | `key: string` | `T \| undefined` | Retrieves non-expired value for key |
| `set(key, val, opts?)` | `key, val, { ttlMs? }` | `this` | Stores value with optional TTL |
| `delete(key)` | `key: string` | `boolean` | Deletes key and appends delete tombstone to WAL |
| `has(key)` | `key: string` | `boolean` | Checks if key exists and is unexpired |
| `keys(prefix?)` | `prefix?: string` | `string[]` | Returns sorted array of matching keys |
| `values()` | None | `any[]` | Returns sorted array of values |
| `entries(prefix?)` | `prefix?: string` | `[string, any][]` | Returns key-value pairs |
| `scan(opts?)` | `{ prefix?, limit? }` | `[string, any][]` | Scans key range up to limit |
| `incr(key, delta?)` | `key, delta = 1` | `number` | Atomically increments numeric value |
| `batch(operations)` | `KVBatchOperation[]` | `this` | Executes multiple operations atomically |
| `compact()` | None | `boolean` | Rewrites WAL file with active keys only |
| `flush()` | None | `boolean` | Forces disk sync of pending WAL entries |
| `close()` | None | `void` | Flushes and closes store |

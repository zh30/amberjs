---
title: "Agent State Checkpoint & Time-Travel Snapshotting (bee:checkpoint)"
subtitle: "Immutable state snapshots, deep structural diffing, reasoning branching, and bee:kv persistence"
group: "Agent & Advanced"
id: "agent-checkpoint"
---

Long-running autonomous agent pipelines (e.g. multi-step code generation, financial report analysis, web research workflows) frequently encounter unexpected tool execution errors, hallucinations, or dead-ends. Without snapshotting, an entire workflow must either crash or restart from step one, discarding valuable context and wasting LLM tokens.

**Beejs v1.8.0 introduces the Native Agent State Checkpoint & Time-Travel Snapshot Engine (`bee:checkpoint`)**. It delivers lightweight immutable state snapshotting, deep structural diffing, speculative execution branching (Tree-of-Thought), and native persistence into `bee:kv`.

---

## 1. State Snapshotting & Time-Travel Recovery

Take snapshots of intermediate agent memory and restore prior states upon execution failure:

```typescript
import { createCheckpointManager } from 'bee:checkpoint';

const mgr = createCheckpointManager();

// 1. Save initial plan
mgr.save('step_1', {
  status: 'planning',
  plan: ['download_data', 'aggregate_metrics']
});

// 2. Advance to execution
mgr.save('step_2', {
  status: 'executing',
  plan: ['download_data', 'aggregate_metrics'],
  data: [100, 200, 300]
});

// 3. Step 3 fails unexpectedly!
mgr.save('step_3_failed', {
  status: 'failed',
  error: 'Network timeout during external API fetch'
});

// 4. Rollback agent state cleanly to step_2
const recoveredState = mgr.restore('step_2');
console.log('Recovered agent status:', recoveredState.status); // "executing"
```

---

## 2. Deep Structural State Diffing

Inspect exactly what fields changed between two historical checkpoints:

```typescript
import { diff, save } from 'bee:checkpoint';

const cp1 = save('v1', { title: 'Draft', wordCount: 150, reviewed: false });
const cp2 = save('v2', { title: 'Final Report', wordCount: 320 });

const delta = diff('v1', 'v2');
console.log('Modified:', delta.modified); // { title: { from: 'Draft', to: 'Final Report' }, wordCount: { from: 150, to: 320 } }
console.log('Deleted:', delta.deleted);   // ['reviewed']
```

---

## 3. Speculative Reasoning Branching (Tree-of-Thought)

Fork alternative agent exploration paths from any prior checkpoint without polluting the main execution lineage:

```typescript
import { createCheckpointManager } from 'bee:checkpoint';

const mainMgr = createCheckpointManager();
mainMgr.save('root', { problem: 'Design database schema' });
mainMgr.save('branch_point', { approach: 'SQL vs NoSQL' });

// Fork an exploratory branch for NoSQL
const nosqlBranch = mainMgr.fork('branch_point', 'nosql_exploration');
nosqlBranch.save('eval_mongo', { database: 'MongoDB', score: 82 });

// Main branch continues with SQL
mainMgr.save('eval_postgres', { database: 'PostgreSQL', score: 94 });

console.log('Main checkpoints:', mainMgr.list().length);    // 3
console.log('Branch checkpoints:', nosqlBranch.list().length); // 3
```

---

## 4. Durable Persistence via `bee:kv`

Persist all checkpoints to durable disk Write-Ahead Logs (WAL) via `bee:kv`:

```typescript
import { createCheckpointManager } from 'bee:checkpoint';
import { open } from 'bee:kv';

const kv = open({ path: './data/agent_checkpoints.wal' });
const mgr = createCheckpointManager();

mgr.save('checkpoint_1', { step: 1, memory: 'Persistent context' });

// Flush checkpoints to disk WAL
mgr.persist(kv, 'agent_state:');

// In another session or process, reload checkpoints
const newMgr = createCheckpointManager();
newMgr.restoreFromKV(kv, 'agent_state:');
console.log('Reloaded:', newMgr.get('checkpoint_1').state.memory);
```

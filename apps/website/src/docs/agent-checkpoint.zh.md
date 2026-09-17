---
title: "Agent 状态检查点与时间旅行回退引擎 (bee:checkpoint)"
subtitle: "不可变状态快照、深层结构差异对比、分支推演推倒重来与 bee:kv 持久化集成"
group: "Agent 与高级特性"
id: "agent-checkpoint"
---

在长周期自治 Agent 任务流（如多步骤代码自动重构、研报自动化撰写、多源网页搜索管线）中，Agent 经常会面临外部工具执行失败、网络超时或模型输出偏离预期的死胡同。在缺乏状态检查点机制的环境下，整个执行链条要么彻底崩溃，要么被迫从第一步重新全量执行，不仅白白丢弃宝贵的上下文，更会浪费高昂的模型 Token 成本。

**Beejs v1.8.0 正式引入原生 Agent 状态检查点与时间旅行回退引擎 (`bee:checkpoint`)**。它提供轻量不可变状态快照、深层结构 Diff 对比、投机分支推演（Tree-of-Thought / 思考树探索）以及与 `bee:kv` 持久化引擎的无缝联动。

---

## 1. 状态快照与时间旅行回滚

在关键步骤保存状态，并在后续步骤遭遇失败时毫秒级精确回滚历史上下文：

```typescript
import { createCheckpointManager } from 'bee:checkpoint';

const mgr = createCheckpointManager();

// 1. 保存规划阶段初始状态
mgr.save('step_1', {
  status: 'planning',
  plan: ['下载数据集', '聚合统计指标']
});

// 2. 进入数据处理阶段
mgr.save('step_2', {
  status: 'executing',
  plan: ['下载数据集', '聚合统计指标'],
  data: [100, 200, 300]
});

// 3. 步骤 3 遭遇异常中断
mgr.save('step_3_failed', {
  status: 'failed',
  error: '调用外部 API 超时'
});

// 4. 精确回退状态至 step_2
const recoveredState = mgr.restore('step_2');
console.log('已恢复状态:', recoveredState.status); // "executing"
```

---

## 2. 深层结构差异对比 (`diff`)

精准分析任意两个历史检查点之间的新增、修改与删除字段：

```typescript
import { diff, save } from 'bee:checkpoint';

const cp1 = save('v1', { title: '草稿', wordCount: 150, reviewed: false });
const cp2 = save('v2', { title: '定稿报告', wordCount: 320 });

const delta = diff('v1', 'v2');
console.log('字段变更:', delta.modified); // { title: { from: '草稿', to: '定稿报告' }, wordCount: { from: 150, to: 320 } }
console.log('删除字段:', delta.deleted);   // ['reviewed']
```

---

## 3. 投机推理分支衍生 (Tree-of-Thought)

从任意历史检查点派生出独立的推理探索分支，支持多路径对比搜索，不污染主执行链路：

```typescript
import { createCheckpointManager } from 'bee:checkpoint';

const mainMgr = createCheckpointManager();
mainMgr.save('root', { problem: '设计高并发数据库架构' });
mainMgr.save('branch_point', { approach: 'SQL 还是 NoSQL' });

// 派生独立分支探索 NoSQL
const nosqlBranch = mainMgr.fork('branch_point', 'nosql_exploration');
nosqlBranch.save('eval_mongo', { database: 'MongoDB', score: 82 });

// 主干继续验证关系型 SQL
mainMgr.save('eval_postgres', { database: 'PostgreSQL', score: 94 });

console.log('主分支检查点数:', mainMgr.list().length);    // 3
console.log('派生分支检查点数:', nosqlBranch.list().length); // 3
```

---

## 4. 与 `bee:kv` 持久化集成

无缝将内存状态检查点批量同步至磁盘 WAL 事务日志：

```typescript
import { createCheckpointManager } from 'bee:checkpoint';
import { open } from 'bee:kv';

const kv = open({ path: './data/agent_checkpoints.wal' });
const mgr = createCheckpointManager();

mgr.save('checkpoint_1', { step: 1, memory: '持久化长效记忆' });

// 批量持久化到磁盘 WAL
mgr.persist(kv, 'agent_state:');

// 在全新进程或会话中重新拉取
const newMgr = createCheckpointManager();
newMgr.restoreFromKV(kv, 'agent_state:');
console.log('重新载入数据:', newMgr.get('checkpoint_1').state.memory);
```

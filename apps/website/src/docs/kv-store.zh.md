---
title: "持久化 Key-Value 与 Agent 状态引擎 (bee:kv)"
subtitle: "嵌入式 ACID 事务存储、前缀范围扫描与 TTL 自动过期"
group: "Agent & Advanced"
id: "kv-store"
---

在运行长时间自治的 AI Agent 业务流时，Agent 极其依赖在 Worker 实例重启、Isolate 回收与会话更迭之间保存持久化上下文与内存。传统关系型数据库对于亚毫秒级的会话存取往往过于厚重，而内存中的 `Map` 又无法在进程退出后保留数据。

**Beejs v1.7.0 正式引入原生持久化 Key-Value 与 Agent 状态持久引擎（`bee:kv`）**。完全基于 Rust 原生实现，零外部重型依赖，提供嵌入式 ACID 事务、纯内存/磁盘 WAL 预写日志双模运行、自动 TTL 过期剔除、前缀范围扫描及原子批量提交。

---

## 1. 核心架构与运行模式

`bee:kv` 提供灵活的双模存储架构：

```
[ 纯内存模式 ]                      [ 磁盘持久化模式 ]
KVStore.openMemory()               KVStore.open("./agent.bee-kv")
        |                                       |
        v                                       v
[ 内存瞬态索引 ]                     [ 内存索引 + 预写追加日志 WAL ]
(极致亚毫秒存取，瞬态使用)            (进程崩溃自动重放，跨会话无缝恢复)
                                                |
                                                v (store.compact())
                                    [ 原子快照压缩与日志重写 ]
```

### 核心特性
- **零外部动态库依赖**：纯 Rust 原生嵌入式实现，无外部 SQLite / RocksDB 依赖膨胀。
- **持久化预写日志 (WAL)**：确保所有写操作具备 ACID 崩溃安全性，重启时毫秒级自动重放构建最新内存态。
- **内置 TTL 动态生命周期**：支持为任意键值设置毫秒级过期时长，过期键自动懒剔除或在扫描时自动清退。
- **前缀范围扫描**：支持按层级前缀（如 `agent:session:*`）开展结构化键值探测与分页扫描。

---

## 2. 基础使用指南

### 2.1 打开存储实例

```typescript
import { KVStore } from 'bee:kv';

// 模式 A: 极速纯内存存储
const memStore = KVStore.openMemory();

// 模式 B: 磁盘持久化存储（自动维护 WAL 与恢复）
const diskStore = KVStore.open('./data/agent_memory.bee-kv');
```

### 2.2 存储与查询数据

支持存取任意可 JSON 序列化的数据类型（字符串、数值、对象、数组）：

```typescript
// 写入数据并指定 60 秒后自动过期 (ttlMs: 60000)
diskStore.set('session:101', {
  user: 'Alice',
  model: 'qwen2.5-7b',
  turns: 4
}, { ttlMs: 60000 });

// 读取数据
const session = diskStore.get('session:101');
console.log(`当前交互轮次: ${session.turns}`);

// 检查键存在性与主动删除
if (diskStore.has('session:101')) {
  diskStore.delete('session:101');
}
```

---

## 3. 高级 Agent 场景功能

### 3.1 前缀范围扫描

按照层级结构组织 Agent 记忆与配置，并一键扫描提取：

```typescript
diskStore.set('agent:memory:user_goal', '开发官网落地页');
diskStore.set('agent:memory:user_budget', 5000);
diskStore.set('agent:memory:deadline', '2026-10-01');
diskStore.set('agent:config:model', 'gpt-4o');

// 扫描所有以 'agent:memory:' 开头的键值对
const memories = diskStore.scan({ prefix: 'agent:memory:' });
for (const [key, value] of memories) {
  console.log(`${key} =>`, value);
}

// 限制扫描返回数量
const limited = diskStore.scan({ prefix: 'agent:memory:', limit: 2 });
```

### 3.2 原子数值递增与事务级批量提交

```typescript
// 原子计数递增（线程安全，适合 Agent 轮次、Token 计量与限流）
const visits = diskStore.incr('page_views', 1);
const tokensUsed = diskStore.incr('agent:tokens', 350);

// 原子批量执行（全部成功或失败）
diskStore.batch([
  { type: 'put', key: 'checkpoint:step', value: 42 },
  { type: 'put', key: 'checkpoint:status', value: 'completed' },
  { type: 'del', key: 'checkpoint:pending_work' }
]);

// 压缩存储文件，回收已删除条目占用的磁盘空间
diskStore.compact();
```

---

## 4. API 完整参考

| 方法名称 | 参数列表 | 返回值类型 | 功能说明 |
| :--- | :--- | :--- | :--- |
| `KVStore.open(path)` | `path: string` | `KVStore` | 打开或创建基于磁盘 WAL 的持久化存储实例 |
| `KVStore.openMemory()` | 无 | `KVStore` | 打开纯内存瞬态存储实例 |
| `get(key)` | `key: string` | `T \| undefined` | 获取未过期的键值，若过期或不存在返回 `undefined` |
| `set(key, val, opts?)`| `key, val, { ttlMs? }` | `this` | 写入键值，支持指定 TTL 过期毫秒数 |
| `delete(key)` | `key: string` | `boolean` | 删除指定键并写入 WAL 墓碑记录 |
| `has(key)` | `key: string` | `boolean` | 检查键是否存在且未过期 |
| `keys(prefix?)` | `prefix?: string` | `string[]` | 返回排好序的匹配键名列表 |
| `values()` | 无 | `any[]` | 返回所有有效值的排序列表 |
| `entries(prefix?)` | `prefix?: string` | `[string, any][]` | 返回键值对二维数组 |
| `scan(opts?)` | `{ prefix?, limit? }` | `[string, any][]` | 范围扫描指定前缀并可限制最大返回条数 |
| `incr(key, delta?)` | `key, delta = 1` | `number` | 原子增减数值型条目 |
| `batch(operations)` | `KVBatchOperation[]` | `this` | 原子批量执行一组 `put`/`del` 操作 |
| `compact()` | 无 | `boolean` | 压缩 WAL 文件并清除所有过期历史记录 |
| `flush()` | 无 | `boolean` | 强制将暂存的 WAL 缓冲区刷入磁盘 |
| `close()` | 无 | `void` | 刷盘并关闭存储句柄 |

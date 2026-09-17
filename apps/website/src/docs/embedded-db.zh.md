---
title: "嵌入式数据与向量引擎 (bee:db & bee:vector)"
subtitle: "零配置原生 SQLite 数据库与高维向量相似度检索引擎，为现代与 AI 应用提供全闭环数据底座"
group: "生态与扩展"
id: "embedded-db"
---

## 1. 为什么在运行时内置数据与向量引擎？

在现代 Web 服务与边缘 AI 应用中，数据存储与向量检索是不可或缺的基础设施。传统方案往往存在以下瓶颈：
- **依赖外部进程与 Docker**：微服务或本地开发需要启动独立的 PostgreSQL / Redis / Milvus，配置复杂且占用大量系统资源；
- **CJS/Native 插件兼容陷阱**：在 Node.js 中使用 `better-sqlite3` 常常受制于 `node-gyp` 原生编译环境与多架构交叉编译失败；
- **跨边界数据序列化损耗**：大批量向量数据在 JavaScript 堆与外部 Python / 数据库进程之间来回 JSON 序列化，产生严重的 CPU 与内存开销。

Beejs 在底层直接使用 Rust 打包编译了原生的 **SQLite 3** 引擎与轻量级**高维向量检索（VectorDB）引擎**，通过 `bee:db` 与 `bee:vector` 提供开箱即用、零外部依赖的高性能数据体验。

---

## 2. 嵌入式 SQLite (`bee:db` / `bee:sqlite`)

通过 `import { Database } from 'bee:db'` 或 `require('bee:db')` 即可直接使用。

### 核心特性
- **内存数据库与磁盘持久化**：支持 `:memory:` 快速临时库与常规 `.db` 文件持久化。
- **预编译执行模型**：支持 `.run()` 执行写操作、`.get()` 查询单行、`.all()` 查询全部记录。
- **线程安全句柄管理**：底层采用安全的连接池与线程同步原语，高并发安全。
- **事务与原子回滚**：内置 `db.transaction()`，异常自动回滚，确保 ACID。

### 使用示例

```typescript
import { Database } from 'bee:db';

// 1. 初始化数据库连接（支持文件路径或 ':memory:'）
const db = new Database('app.db');

// 2. 创建表结构
db.run(`
  CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
  )
`);

// 3. 插入数据（支持参数化绑定，防止 SQL 注入）
const insertResult = db.run(
  'INSERT INTO users (username, email) VALUES (?, ?)',
  ['alice', 'alice@beejs.dev']
);
console.log('新插入记录 ID:', insertResult.lastInsertRowid);
console.log('受影响行数:', insertResult.changes);

// 4. 查询单行记录
const user = db.get('SELECT * FROM users WHERE username = ?', ['alice']);
console.log('查询结果:', user);

// 5. 批量查询
const allUsers = db.all('SELECT id, username, email FROM users ORDER BY id DESC');
console.log('全部用户:', allUsers);

// 6. 事务安全提交与自动回滚
db.transaction(() => {
  db.run('INSERT INTO users (username, email) VALUES (?, ?)', ['bob', 'bob@beejs.dev']);
  db.run('INSERT INTO users (username, email) VALUES (?, ?)', ['carol', 'carol@beejs.dev']);
});

// 7. 关闭连接
db.close();
```

---

## 3. 向量检索数据库 (`bee:vector`)

`bee:vector` 为本地 RAG（检索增强生成）、知识库搜索与推荐算法量身定制。支持在本地内存中索引上万条高维嵌入向量，并提供微秒级的最近邻相似度排序。

### 支持的距离度量
- `cosine`（默认）：余弦相似度，取值范围 `[-1.0, 1.0]`，数值越大越相似，专为自然语言与语义向量优化。
- `euclidean`：欧几里得距离，数值越小空间距离越近。
- `dot`：点积相似度，适用于已归一化（Normalized）的 Embedding 向量。

### 使用示例：构建本地语义检索系统

```typescript
import { VectorDB } from 'bee:vector';

// 1. 创建 4 维向量数据库（默认使用 cosine 余弦相似度）
const vdb = new VectorDB(4, 'cosine');

// 2. 插入带有结构化元数据的向量记录
vdb.insert('doc-1', [0.1, 0.8, 0.2, 0.0], { title: 'Beejs 架构概览', tag: 'arch' });
vdb.insert('doc-2', [0.12, 0.79, 0.18, 0.05], { title: 'V8 内存模型解析', tag: 'v8' });
vdb.insert('doc-3', [0.9, 0.1, 0.05, 0.2], { title: 'Docker 容器化指南', tag: 'devops' });

// 3. 执行 Top-K 向量相似度搜索
const queryEmbedding = [0.11, 0.81, 0.19, 0.02];
const topMatches = vdb.search(queryEmbedding, 2);

console.log('最近邻语义搜索结果:');
topMatches.forEach((match, rank) => {
  console.log(`#${rank + 1} ID: ${match.id}, 相似度: ${match.score.toFixed(4)}, 标题: ${match.metadata.title}`);
});

// 4. 元数据更新与记录删除
vdb.delete('doc-3');
console.log('当前索引总数:', vdb.count());

// 5. 序列化导出与加载（可保存到磁盘或缓存）
const serialized = vdb.toJSON();
const restoredVdb = VectorDB.fromJSON(serialized);
console.log('重建后索引记录数:', restoredVdb.count());
```

---

## 4. 最佳实践：打造本地 RAG 知识库微服务

结合 `bee:ai`、`bee:db` 与 `bee:vector`，你可以在 30 行内使用纯 TypeScript 构建一个完全脱离 Python 与外部容器的自包含 AI 问答检索服务：

```typescript
import { Database } from 'bee:db';
import { VectorDB } from 'bee:vector';

const db = new Database('knowledge.db');
const vdb = new VectorDB(128, 'cosine');

// 初始化文档内容与向量索引
export function indexDocument(id: string, content: string, embedding: number[]) {
  db.run('INSERT OR REPLACE INTO documents (id, content) VALUES (?, ?)', [id, content]);
  vdb.insert(id, embedding, { id });
}

export function searchKnowledge(queryEmbedding: number[], topK = 3) {
  const matches = vdb.search(queryEmbedding, topK);
  return matches.map(match => {
    const row = db.get('SELECT content FROM documents WHERE id = ?', [match.id]);
    return {
      id: match.id,
      score: match.score,
      content: row?.content
    };
  });
}
```

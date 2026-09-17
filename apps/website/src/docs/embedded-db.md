---
title: "Embedded Database & Vector Engine (bee:db & bee:vector)"
subtitle: "Zero-config in-process SQLite and high-dimensional vector similarity engine for modern & AI applications"
group: "Ecosystem"
id: "embedded-db"
---

## 1. Why Built-in Database and Vector Engine?

In modern cloud services and edge AI applications, data persistence and vector similarity search are fundamental requirements. Traditional approaches often introduce friction:
- **Heavy External Dependencies**: Starting standalone PostgreSQL, Redis, or Milvus containers requires operational overhead and consumes high memory footprint.
- **Native Addon Pitfalls**: Using packages like `better-sqlite3` in Node.js frequently fails during `node-gyp` builds or cross-platform deployment.
- **Serialization Overhead**: Serializing massive vector embeddings over JSON IPC between JavaScript and Python/database processes hurts throughput.

Beejs bundles native **SQLite 3** and an in-memory **VectorDB** engine directly compiled into the binary via Rust. Through `bee:db` and `bee:vector`, developers get an out-of-the-box, zero-dependency data foundation.

---

## 2. In-Process SQLite (`bee:db` / `bee:sqlite`)

Import the engine using `import { Database } from 'bee:db'` or `require('bee:db')`:

### Key Features
- **In-Memory & File Persistence**: Supports fast `:memory:` temporary databases as well as standard `.db` disk files.
- **Prepared Statement API**: `.run()` for mutations, `.get()` for single-row retrieval, and `.all()` for fetching all records.
- **Thread-Safe Handles**: Backed by thread-safe connection pooling and sync primitives.
- **ACID Transactions**: Built-in `db.transaction()` with automatic rollback on errors.

### Example Usage

```typescript
import { Database } from 'bee:db';

// 1. Initialize connection (file path or ':memory:')
const db = new Database('app.db');

// 2. Create table schema
db.run(`
  CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
  )
`);

// 3. Insert records with parameterized binding
const insertResult = db.run(
  'INSERT INTO users (username, email) VALUES (?, ?)',
  ['alice', 'alice@beejs.dev']
);
console.log('Inserted ID:', insertResult.lastInsertRowid);
console.log('Rows modified:', insertResult.changes);

// 4. Query single record
const user = db.get('SELECT * FROM users WHERE username = ?', ['alice']);
console.log('User query result:', user);

// 5. Query multiple rows
const allUsers = db.all('SELECT id, username, email FROM users ORDER BY id DESC');
console.log('All users:', allUsers);

// 6. Safe transactions with rollback
db.transaction(() => {
  db.run('INSERT INTO users (username, email) VALUES (?, ?)', ['bob', 'bob@beejs.dev']);
  db.run('INSERT INTO users (username, email) VALUES (?, ?)', ['carol', 'carol@beejs.dev']);
});

// 7. Close connection
db.close();
```

---

## 3. Vector Similarity Search (`bee:vector`)

`bee:vector` is tailored for local RAG (Retrieval-Augmented Generation), semantic document search, and recommendation systems. It indexes high-dimensional vectors with sub-millisecond nearest-neighbor search.

### Supported Distance Metrics
- `cosine` (default): Cosine similarity in `[-1.0, 1.0]`, higher score indicates higher semantic similarity.
- `euclidean`: L2 Euclidean distance, smaller score indicates closer spatial distance.
- `dot`: Dot product similarity, optimal for pre-normalized embeddings.

### Example: Semantic Search Engine

```typescript
import { VectorDB } from 'bee:vector';

// 1. Create a 4-dimensional vector database with cosine metric
const vdb = new VectorDB(4, 'cosine');

// 2. Insert records with structured metadata
vdb.insert('doc-1', [0.1, 0.8, 0.2, 0.0], { title: 'Beejs Architecture', tag: 'arch' });
vdb.insert('doc-2', [0.12, 0.79, 0.18, 0.05], { title: 'V8 Memory Management', tag: 'v8' });
vdb.insert('doc-3', [0.9, 0.1, 0.05, 0.2], { title: 'Docker Guide', tag: 'devops' });

// 3. Perform Top-K nearest neighbor search
const queryEmbedding = [0.11, 0.81, 0.19, 0.02];
const topMatches = vdb.search(queryEmbedding, 2);

console.log('Top Semantic Matches:');
topMatches.forEach((match, rank) => {
  console.log(`#${rank + 1} ID: ${match.id}, Score: ${match.score.toFixed(4)}, Title: ${match.metadata.title}`);
});

// 4. Delete vector record
vdb.delete('doc-3');
console.log('Current count:', vdb.count());

// 5. Serialize and restore index
const serialized = vdb.toJSON();
const restoredVdb = VectorDB.fromJSON(serialized);
console.log('Restored count:', restoredVdb.count());
```

---

## 4. Local RAG Service in 30 Lines

Combine `bee:ai`, `bee:db`, and `bee:vector` to build a self-contained question-answering search pipeline without Python or external services:

```typescript
import { Database } from 'bee:db';
import { VectorDB } from 'bee:vector';

const db = new Database('knowledge.db');
const vdb = new VectorDB(128, 'cosine');

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

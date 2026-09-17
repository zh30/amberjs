---
title: "Native Text Embeddings (bee:ai 2.0)"
subtitle: "Sub-millisecond text vectorization and high-dimensional cosine similarity directly in V8 without external dependencies"
group: "Core System"
id: "ai-embeddings"
---

## 1. Why Native Embeddings in the Runtime?

In Large Language Model applications and Retrieval-Augmented Generation (RAG) pipelines, **Text Embedding** is the fundamental operation for semantic retrieval, document deduplication, and intent classification. Traditional JavaScript solutions incur heavy operational penalties:

- **Cloud Embedding APIs**: 50–200ms round-trip latency, costly for real-time stream filtering or high-throughput batching;
- **Python / PyTorch Child Processes**: Hundreds of milliseconds startup latency, high memory overhead, and complex inter-process serialization;
- **Heavyweight Model Files**: Multi-hundred-megabyte weights difficult to deploy in lightweight edge agent containers.

Beejs v1.3.0 introduces a **zero-dependency, native text vectorization engine**:
- **Pure Rust Host Engine**: Combines subword/n-gram hashing, positional weighting, and GELU activation to generate deterministic dense semantic vectors;
- **Hardware-accelerated L2 Normalization**: Outputs unit vectors where the dot product equals exact cosine similarity;
- **Microsecond Latency**: Vectorizes sentences in tens of microseconds—over **1,000x faster** than network round-trips;
- **Native Integration with `bee:vector`**: Outputs standard `Float32Array` objects directly consumed by `VectorDB`.

---

## 2. API Reference & Usage

Import embedding utilities from the `bee:ai` module:

```typescript
import { embed, embedBatch, cosineSimilarity, Tensor } from 'bee:ai';
```

### `embed(text, options?)`

Computes a dense semantic embedding for an input string:

```typescript
// Defaults to 64-dimensional Float32Array
const vec = embed("High performance JavaScript runtime written in Rust");
console.log(vec instanceof Float32Array); // true
console.log(vec.length); // 64

// Custom dimensions: supports 64, 128, 384
const vec128 = embed("Autonomous agent tool orchestration", { dimensions: 128 });
console.log(vec128.length); // 128

// Return as high-performance Tensor instance
const tensor = embed("Deep learning architectures", { asTensor: true });
console.log(tensor instanceof Tensor); // true
```

#### Options (`EmbedOptions`)

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `dimensions` | `number` | `64` | Output embedding dimension (`64`, `128`, `384`) |
| `normalize` | `boolean` | `true` | Apply L2 unit normalization |
| `asTensor` | `boolean` | `false` | Return a `bee:ai` `Tensor` instance instead of `Float32Array` |

---

### `embedBatch(texts, options?)`

Generates embeddings for an array of strings in a single call:

```typescript
const texts = [
  "Beejs runtime architecture",
  "Zero-copy memory mapping",
  "Homemade Italian pasta recipes"
];

const batch = embedBatch(texts, { dimensions: 64 });
console.log(batch.length); // 3
console.log(batch[0] instanceof Float32Array); // true
```

---

### `cosineSimilarity(a, b)`

Computes the cosine similarity between two vector representations in `[-1.0, 1.0]`:

```typescript
const vA = embed("Concurrency performance and throughput tuning");
const vB = embed("Optimizing server concurrency and system throughput");
const vC = embed("How to bake chocolate brownies in an oven");

const simAB = cosineSimilarity(vA, vB);
const simAC = cosineSimilarity(vA, vC);

console.log("Related similarity:", simAB.toFixed(4));   // ~ 0.75+
console.log("Unrelated similarity:", simAC.toFixed(4)); // ~ 0.15-
```

---

## 3. In-Memory RAG with `bee:vector`

Combine `embed` with Beejs's built-in `VectorDB` to build an embedded retrieval pipeline with zero external databases:

```typescript
import { embed } from 'bee:ai';
import { VectorDB } from 'bee:vector';

const db = new VectorDB({ dimensions: 64, metric: 'cosine' });

const docs = [
  { id: '1', text: 'Beejs v1.3.0 ships official Model Context Protocol 2.0.' },
  { id: '2', text: 'Virtual Filesystem provides in-memory Copy-on-Write sandbox.' },
  { id: '3', text: 'Embedded SQLite engine supports transactions and prepared queries.' },
];

for (const doc of docs) {
  db.insert(doc.id, embed(doc.text), { text: doc.text });
}

const query = "How does Beejs safely run untrusted Agent code?";
const results = db.search(embed(query), { topK: 1 });

console.log("Top Match:", results[0].metadata.text);
```

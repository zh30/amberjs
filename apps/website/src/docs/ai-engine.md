---
title: "Native AI Engine (bee:ai)"
subtitle: "Eliminating Python glue layers: direct zero-copy tensors, streaming LLM inference, and agent pipelines in V8"
group: "Core Systems"
id: "ai-engine"
---

## 1. Why Native AI in the Runtime?

In the era of generative AI and autonomous agents, backend development is shifting from traditional CRUD services to **vector similarity search, high-dimensional tensor operations, streaming LLM inference, and agent tool execution**.

JavaScript developers traditionally face frustrating compromises:
- **Subprocess Python IPC**: Communicating with Python via stdio or HTTP incurs serialization overhead, doubles memory footprints, and complicates deployments.
- **Fragile node-gyp Addons**: Native C++ plugins frequently fail to build across architectures, break in container environments, and trigger memory leaks.

Beejs introduces the built-in **`bee:ai`** module: written in Rust at the host layer with SIMD optimizations, interfacing directly with V8 through zero-copy `Float32Array` buffers.

---

## 2. Core Components

Import directly without third-party dependencies:

```typescript
import { Tensor, LLM, AgentPipeline } from 'bee:ai';
```

---

### 1. `Tensor` Zero-Copy Vector Operations

Backed by contiguous `Float32Array` buffers and vectorized CPU instructions:

```typescript
// 1. Create tensors from flat arrays and shape definitions
const a = Tensor.from([1.0, 2.0, 3.0, 4.0], [2, 2]);
const b = Tensor.from([5.0, 6.0, 7.0, 8.0], [2, 2]);

// 2. Hardware-accelerated matrix multiplication (matmul)
const c = Tensor.matmul(a, b);
console.log('Matrix product:', c.toArray(), c.shape);

// 3. Dot product and vector magnitude
const v1 = Tensor.from([1.0, 2.0, 3.0], [3]);
const v2 = Tensor.from([4.0, 5.0, 6.0], [3]);
console.log('Dot product:', Tensor.dot(v1, v2)); // 32.0
console.log('L2 Norm:', Tensor.norm(v1)); // 3.7416...

// 4. Numerically stable Softmax
const logits = Tensor.from([2.0, 1.0, 0.1], [3]);
const probs = Tensor.softmax(logits);
console.log('Normalized probabilities:', probs.toArray());

// 5. Cosine similarity for semantic retrieval
const queryEmbedding = Tensor.from([0.1, 0.8, 0.3], [3]);
const docEmbedding = Tensor.from([0.12, 0.79, 0.31], [3]);
const similarity = Tensor.cosineSimilarity(queryEmbedding, docEmbedding);
console.log('Cosine similarity:', similarity); // 0.999...
```

---

### 2. `LLM` Local Streaming Inference

Load quantized model weights directly for streaming token generation and embeddings:

```typescript
// Load local quantized GGUF weights
const model = await LLM.load('./models/qwen2.5-0.5b-instruct.gguf', {
  threads: 4,
  contextSize: 2048,
});

// 1. Asynchronous token stream (ideal for Server-Sent Events / SSE)
const prompt = "Summarize the key advantages of the Rust language in 3 sentences.";
console.log(`Prompt: ${prompt}\nGenerating: `);

for await (const token of model.generateStream(prompt, { maxTokens: 256, temperature: 0.7 })) {
  process.stdout.write(token);
}
console.log('\n--- Done ---');

// 2. Generate text embeddings
const vector = await model.embed("High performance cloud-native runtime");
console.log(`Vector dimensions: ${vector.length}, first 3:`, vector.slice(0, 3));
```

---

### 3. `AgentPipeline` Autonomous Orchestration

Build deterministic AI agents with structured tool calling and state tracking:

```typescript
// agent_example.ts
import { AgentPipeline } from 'bee:ai';

const agent = new AgentPipeline({
  name: 'DevOps Assistant',
  instructions: 'You are an operations assistant capable of querying cluster health.',
});

// Register tools with JSON Schema parameter definitions
agent.registerTool({
  name: 'query_server_status',
  description: 'Inspect CPU and memory metrics for a server node',
  parameters: {
    type: 'object',
    properties: {
      server_id: { type: 'string', description: 'Server identifier, e.g., srv-01' },
    },
    required: ['server_id'],
  },
  handler: async ({ server_id }) => {
    return {
      server_id,
      cpu_usage: '24.5%',
      memory_free: '8.2 GB',
      healthy: true,
    };
  },
});

// Execute agent decision step
const response = await agent.runStep('Check if server node srv-01 is healthy.');
console.log('Agent Response:', response.content);
```

---

## 3. Deterministic Sandboxing for Testing

AI decision loops frequently display non-deterministic variance that complicates debugging. Beejs provides CLI flags for reproducible agent evaluation:

- **`--seed <UINT64>`**: Seeds the pseudo-random number generator (`Math.random()` and `crypto.getRandomValues()`).
- **`--freeze-time <ISO_STRING>`**: Locks the virtual clock (`Date.now()`, `new Date()`, `performance.now()`).

```bash
# Replay agent runs with pinned random seed and virtual clock
bee run --seed 42 --freeze-time "2026-09-07T08:00:00Z" agent_example.ts
```

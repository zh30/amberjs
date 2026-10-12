---
title: "amber:ai"
nav: "amber:ai"
subtitle: "Typed-array tensors, a built-in text embedding, and an LLM interface for local model files."
status: "stable"
id: "ai-engine"
---

`amber:ai` is in the Stable scope and compiles in the default build. It does not need the `ai` Cargo feature, which is empty. Read the notes on `LLM` below before building on it.

```js
const { Tensor, LLM, AgentPipeline, embed, cosineSimilarity } = require("amber:ai");
```

## Tensor

`Tensor` wraps a `Float32Array` and a shape. Operations do not copy the backing buffer when they can avoid it.

```js
const a = new Tensor([1, 2, 3, 4], [2, 2]);
const b = new Tensor([5, 6, 7, 8], [2, 2]);

a.matmul(b).toArray(); // [[19, 22], [43, 50]]

const v1 = new Tensor([1, 2, 3]);
const v2 = new Tensor([4, 5, 6]);
v1.dot(v2);              // 32
v1.norm();               // 3.7416...
v1.cosineSimilarity(v2); // 0.9746...
new Tensor([2, 1, 0.1]).softmax().toArray();
```

## Embeddings

```js
const vector = embed("hello world");          // Float32Array, 64 dims by default
const other = embed("hello there", { dimensions: 128 });
```

`embed` is a deterministic, hashed-token embedding computed in Rust. It is fast and reproducible, but it is **not** a learned semantic model. Use it for tests, deduplication, or rough lexical similarity. For semantic search, load a real embedding model and feed its output into `Tensor`.

## LLM

```js
const llm = await LLM.load("./models/model.gguf");
const result = await llm.generate("Summarize this file.");

for await (const chunk of llm.generateStream("Write a haiku.")) {
  process.stdout.write(chunk);
}
```

When `LLM.load` can open the model file, generation runs locally through Candle. The default build runs on the CPU. CUDA and Metal need a source build with the `cuda` or `metal` Cargo feature, and those features are not CI-gated.

**Important:** when the model cannot be loaded, `LLM.load` does not throw. It returns an `LLM` backed by a built-in template generator that produces placeholder text, and placeholder JSON when you pass a schema. That output is not model output. Check that your model path loads before you trust a result.

## AgentPipeline

```js
const pipeline = new AgentPipeline({
  model: llm,
  tools: [{ name: "echo", execute: (input) => input }],
});

const step = await pipeline.step("hello");
// { status: "completed", output: "...", historyLength: 2 }
```

`AgentPipeline` records history and holds registered tools. `step()` sends the input to `model.generate()` and stores the reply. It does not plan or call tools by itself; your code decides when to call `tool.execute`.

## Not in scope

The getters `weights`, `tools`, `kv`, `bus`, `grammar`, and `checkpoint` on `amber:ai` point to modules that are not classified in the scope page. Treat them as Experimental.

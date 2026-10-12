---
title: "amber:ai"
nav: "amber:ai"
subtitle: "基于类型化数组的张量、内置文本 embedding，以及加载本地模型文件的 LLM 接口。"
status: "stable"
id: "ai-engine"
---

`amber:ai` 属于 Stable 范围，并在默认构建中编译，不需要 `ai` Cargo feature（该 feature 为空）。在基于它开发之前，请先阅读下文关于 `LLM` 的说明。

```js
const { Tensor, LLM, AgentPipeline, embed, cosineSimilarity } = require("amber:ai");
```

## Tensor

`Tensor` 封装一个 `Float32Array` 和一个 shape。在可以避免时，运算不会复制底层缓冲区。

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

## Embedding

```js
const vector = embed("hello world");          // Float32Array，默认 64 维
const other = embed("hello there", { dimensions: 128 });
```

`embed` 是在 Rust 中计算的确定性哈希 token embedding。它快速且可复现，但**不是**经过训练的语义模型，适合测试、去重或粗略的词面相似度。需要语义检索时，请加载真正的 embedding 模型，再把输出交给 `Tensor`。

## LLM

```js
const llm = await LLM.load("./models/model.gguf");
const result = await llm.generate("Summarize this file.");

for await (const chunk of llm.generateStream("Write a haiku.")) {
  process.stdout.write(chunk);
}
```

当 `LLM.load` 能打开模型文件时，生成通过 Candle 在本地运行。默认构建使用 CPU；CUDA 与 Metal 需要用 `cuda` 或 `metal` Cargo feature 从源码构建，这两个 feature 没有 CI 门禁。

**重要：** 模型无法加载时，`LLM.load` 不会抛错，而是返回一个由内置模板生成器驱动的 `LLM`。它产生占位文本，传入 schema 时产生占位 JSON。这些输出不是模型输出。在信任结果之前，请先确认模型路径能够正常加载。

## AgentPipeline

```js
const pipeline = new AgentPipeline({
  model: llm,
  tools: [{ name: "echo", execute: (input) => input }],
});

const step = await pipeline.step("hello");
// { status: "completed", output: "...", historyLength: 2 }
```

`AgentPipeline` 记录历史并保存已注册的工具。`step()` 把输入交给 `model.generate()` 并保存回复。它不会自行规划或调用工具，何时调用 `tool.execute` 由你的代码决定。

## 不在范围内

`amber:ai` 上的 `weights`、`tools`、`kv`、`bus`、`grammar` 和 `checkpoint` getter 指向范围页中未分级的模块，请视为 Experimental。

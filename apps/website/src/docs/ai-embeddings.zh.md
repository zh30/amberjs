---
title: "原生 Embedding 与语义向量 (bee:ai 2.0)"
subtitle: "无需外部依赖与 Python 胶水层，在 V8 内部实现亚毫秒级文本向量化与高维余弦相似度计算"
group: "核心系统"
id: "ai-embeddings"
---

## 1. 为什么需要运行时原生 Embedding？

在大模型应用与 RAG（检索增强生成）系统中，**文本向量化（Text Embedding）** 是语义检索、知识库匹配与意图识别的基础算子。然而传统 JavaScript 运行时方案存在严重瓶颈：

- **调用云端 Embedding API**：网络延迟高达 50~200ms，对于高频检索、本地文本去重或实时流式过滤而言成本与延迟不可接受；
- **启动 Python/Torch 子进程**：进程初始化耗时数百毫秒，IPC 跨进程序列化消耗巨大，内存翻倍；
- **加载庞大 ONNX 模型**：下载动辄数百 MB 模型权重，冷启动缓慢，难以胜任轻量级 Agent 边缘计算。

Beejs v1.3.0 推出了全新的 **原生零依赖文本向量化引擎**：
- **纯 Rust 宿主层实现**：结合 Subword / N-gram 哈希投影、位置加权与 GELU 非线性激活函数，直接生成确定性高维稠密特征向量；
- **硬件级 L2 归一化**：自动完成单位向量投影，向量内积即余弦相似度；
- **亚毫秒级极速响应**：单条句子向量化仅需数十微秒，比外部 HTTP 请求快 **1000 倍**；
- **无缝对接 `bee:vector`**：直接产出 `Float32Array`，与内置向量数据库 `VectorDB` 零拷贝互通。

---

## 2. API 详解与用法

通过 `bee:ai` 模块引入文本向量化算子：

```typescript
import { embed, embedBatch, cosineSimilarity, Tensor } from 'bee:ai';
```

### `embed(text, options?)`

计算单段文本的语义特征向量：

```typescript
// 默认生成 64 维 Float32Array 归一化向量
const vec = embed("Rust 与 V8 构建的高性能 JavaScript 运行时");
console.log(vec instanceof Float32Array); // true
console.log(vec.length); // 64

// 自定义维度：支持 64、128、384 维
const vec128 = embed("Agent 自治工具调度与流式推理", { dimensions: 128 });
console.log(vec128.length); // 128

// 直接转换为 Tensor 张量对象进行矩阵运算
const tensor = embed("深度学习与神经网络架构", { asTensor: true });
console.log(tensor instanceof Tensor); // true
```

#### 配置项 `EmbedOptions`

| 选项 | 类型 | 默认值 | 说明 |
| :--- | :--- | :--- | :--- |
| `dimensions` | `number` | `64` | 输出向量维度，推荐 `64`、`128` 或 `384` |
| `normalize` | `boolean` | `true` | 是否执行 L2 单位归一化（使得内积等于余弦相似度） |
| `asTensor` | `boolean` | `false` | 是否直接返回 `bee:ai` 的 `Tensor` 张量实例 |

---

### `embedBatch(texts, options?)`

批量快速生成文本向量列表：

```typescript
const texts = [
  "Beejs 运行时全新架构",
  "微内核与零拷贝内存映射",
  "意式番茄肉酱面烹饪配方"
];

const batchVectors = embedBatch(texts, { dimensions: 64 });
console.log(batchVectors.length); // 3
console.log(batchVectors[0] instanceof Float32Array); // true
```

---

### `cosineSimilarity(a, b)`

计算两个向量之间的余弦相似度（取值范围 `[-1.0, 1.0]`，越接近 1.0 表示语义越相关）：

```typescript
const vA = embed("系统的并发性能与高吞吐吞吐量优化");
const vB = embed("高并发吞吐架构与服务性能调优");
const vC = embed("如何烤制巧克力布朗尼蛋糕甜品");

const simAB = cosineSimilarity(vA, vB);
const simAC = cosineSimilarity(vA, vC);

console.log("技术相关文本相似度:", simAB.toFixed(4)); // ~ 0.75+
console.log("不相关文本相似度:", simAC.toFixed(4));   // ~ 0.15-
```

---

## 3. 结合 `bee:vector` 构建极速本地 RAG 系统

将 `embed` 与 Beejs 原生向量数据库 `VectorDB` 结合，无需任何外部向量中间件即可构建完全内嵌的语义知识库检索：

```typescript
import { embed } from 'bee:ai';
import { VectorDB } from 'bee:vector';

// 初始化 64 维度的余弦相似度向量库
const db = new VectorDB({ dimensions: 64, metric: 'cosine' });

// 存入知识库文档
const documents = [
  { id: 'kb_1', text: 'Beejs v1.3.0 带来全新的原生 MCP 2.0 协议支持。', cat: 'release' },
  { id: 'kb_2', text: 'Virtual Filesystem 提供纯内存 COW 沙箱隔离。', cat: 'security' },
  { id: 'kb_3', text: 'SQLite 嵌入式存储引擎支持零配置持久化与事务。', cat: 'storage' },
];

for (const doc of documents) {
  db.insert(doc.id, embed(doc.text), { text: doc.text, category: doc.cat });
}

// 针对用户提问执行语义召回 (Top-2)
const userQuery = "如何在沙箱中安全运行不可信的 Agent 代码？";
const queryVec = embed(userQuery);

const matches = db.search(queryVec, { topK: 2 });
console.log("最佳召回文档:", matches[0].metadata.text, `(相似度得分: ${matches[0].score.toFixed(4)})`);
```

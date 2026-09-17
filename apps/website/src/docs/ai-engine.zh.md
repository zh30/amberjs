---
title: "原生 AI 引擎 (bee:ai)"
subtitle: "免除 Python 胶水层，在 V8 中直接驱动零拷贝张量、本地模型流式推理与 Agent 管线"
group: "核心系统"
id: "ai-engine"
---

## 1. 为什么在运行时中内置 AI 引擎？

在生成式 AI 与大语言模型（LLM）驱动的应用时代，后端工程的重心正在从传统的 CRUD 转向**向量检索、高维张量运算、模型流式生成以及自治 Agent 工具调度**。

传统服务端 JavaScript 开发者面临巨大的生态痛点：
- **要么依赖 Python 子进程**：通过标准输入输出或 HTTP IPC 进行跨进程通信，带来巨大的序列化开销、内存翻倍与部署复杂性；
- **要么依赖复杂的 node-gyp C++ 插件**：在不同平台和 Alpine Linux 上编译极易失败，且经常引发内存泄漏或 V8 崩溃。

Beejs 率先在运行时原生内置了 **`bee:ai`** 模块：直接使用 Rust 在宿主层对接高性能数学库与本地推理后端，以零拷贝 `Float32Array` 直通 V8，让 JavaScript 开发者用纯粹的 TypeScript 原生编写高性能 AI 应用。

---

## 2. 核心组件 API

通过 `import ... from 'bee:ai'` 即可直接引入原生模块：

```typescript
import { Tensor, LLM, AgentPipeline } from 'bee:ai';
```

---

### 一、`Tensor` 零拷贝张量运算

`Tensor` 底层基于连续内存的 `Float32Array`，由 Rust SIMD 指令硬件加速，支持各种常见的向量与矩阵运算：

```typescript
// 1. 创建张量 (数据一维展开 + 形状数组)
const a = Tensor.from([1.0, 2.0, 3.0, 4.0], [2, 2]);
const b = Tensor.from([5.0, 6.0, 7.0, 8.0], [2, 2]);

// 2. 硬件加速矩阵乘法 (matmul)
const c = Tensor.matmul(a, b);
console.log('矩阵乘法结果:', c.toArray(), c.shape);

// 3. 向量点积与模长
const v1 = Tensor.from([1.0, 2.0, 3.0], [3]);
const v2 = Tensor.from([4.0, 5.0, 6.0], [3]);
console.log('点积:', Tensor.dot(v1, v2)); // 32.0
console.log('L2 范数 (模长):', Tensor.norm(v1)); // 3.7416...

// 4. 数值稳定 Softmax (防止浮点上溢)
const logits = Tensor.from([2.0, 1.0, 0.1], [3]);
const probs = Tensor.softmax(logits);
console.log('归一化概率分布:', probs.toArray());

// 5. 向量余弦相似度 (语义检索核心算子)
const queryEmbedding = Tensor.from([0.1, 0.8, 0.3], [3]);
const docEmbedding = Tensor.from([0.12, 0.79, 0.31], [3]);
const similarity = Tensor.cosineSimilarity(queryEmbedding, docEmbedding);
console.log('余弦相似度:', similarity); // 0.999...
```

---

### 二、`LLM` 本地流式模型推理

无需部署庞大的 Ollama 外部服务，直接加载 GGUF 或本地轻量权重，进行流式输出与文本向量嵌入：

```typescript
// 加载本地轻量量化模型
const model = await LLM.load('./models/qwen2.5-0.5b-instruct.gguf', {
  threads: 4,
  contextSize: 2048,
});

// 1. 异步流式生成 (SSE / Web 界面打字机效果)
const prompt = "请用 3 句话介绍 Rust 语言的核心优势。";
console.log(`提示词: ${prompt}\n生成中: `);

for await (const token of model.generateStream(prompt, { maxTokens: 256, temperature: 0.7 })) {
  process.stdout.write(token);
}
console.log('\n--- 生成完成 ---');

// 2. 生成语义向量嵌入 (Embedding)
const vector = await model.embed("现代高性能云原生运行时");
console.log(`向量维度: ${vector.length}, 前 3 维:`, vector.slice(0, 3));
```

---

### 三、`AgentPipeline` 确定性智能体编排

Beejs 为构建自主 AI Agent 提供了完整的状态机与工具调度管线：

```typescript
// agent_example.ts
import { AgentPipeline } from 'bee:ai';

const agent = new AgentPipeline({
  name: 'DevOps Assistant',
  instructions: '你是一名运维助手，可通过工具执行排查任务。',
});

// 注册智能体可用工具与 JSON Schema 描述
agent.registerTool({
  name: 'query_server_status',
  description: '查询当前服务器的 CPU 与内存使用率',
  parameters: {
    type: 'object',
    properties: {
      server_id: { type: 'string', description: '服务器编号，如 srv-01' },
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

// 运行智能体决策步
const response = await agent.runStep('帮我检查一下 srv-01 服务器的状态是否正常？');
console.log('智能体最终回复:', response.content);
```

---

## 3. 确定性回放与测试沙箱

AI Agent 的自主行为往往具有随机性，在生产测试与调试重放时极难复现 Bug。Beejs 提供两项系统级标志，保证 AI 决策的绝对确定性：

- **`--seed <UINT64>`**：锁定伪随机数生成器（`Math.random()` 与 `crypto.getRandomValues()`）的种子；
- **`--freeze-time <ISO_STRING>`**：冻结全局虚拟时钟（`Date.now()`、`new Date()` 与 `performance.now()`）。

```bash
# 在固定种子与固定虚拟时间下回放 Agent 运行
bee run --seed 42 --freeze-time "2026-09-07T08:00:00Z" agent_example.ts
```

每次运行，所有随机采样的 Token、生成的 UUID、事件时间戳都将保持 100% 严丝合缝的一致，为自动化测试与安全审查提供了坚如磐石的保障。

---
title: "GGUF 与 SafeTensors 模型权重加载器 (bee:weights)"
subtitle: "极速二进制头解析、零拷贝 mmap 张量切片与 bee:ai 无缝互操作"
group: "Agent & Advanced"
id: "model-weights"
---

在边缘侧运行私有化 AI Agent 时，往往需要将轻量模型（例如小型 SLM、Embedding 投影矩阵、注意力权重等）极速载入内存。传统 Node.js 方案依赖厚重的 C++ Addon 或全局缓冲区拷贝，容易导致 V8 堆内存溢出。

**Beejs v1.6.0 原生集成了高性能二进制模型权重加载器（`bee:weights` / `bee:ai.weights`）**。基于 Rust 内存映射（`mmap`），支持毫秒级解析数十吉字节的大型模型元数据，并将任意张量零拷贝切片载入 `bee:ai.Tensor`。

---

## 1. 支持格式与规格

| 格式标准 | 魔数 / 头部规范 | 支持版本 | 核心能力 |
| :--- | :--- | :--- | :--- |
| **GGUF** | `b"GGUF"` | v2, v3 | 支持量化类型（Q4_0, Q8_0, F16, F32），KV 元数据键值对解析，字节对其填充 |
| **SafeTensors** | 8 字节小端 JSON 头部 | HuggingFace 规范 | PyTorch 权重格式，张量形状提取，连续内存切片 |

---

## 2. 毫秒级元数据快速探测

无论是多大尺寸的模型文件，解析元数据均在亚毫秒（< 1ms）内完成，无需将权重数据读入内存：

```typescript
import { readGGUFMetadata, readSafeTensorsMetadata } from 'bee:weights';

// 解析 llama.cpp GGUF 模型元数据
const ggufMeta = readGGUFMetadata("./models/qwen2.5-0.5b-instruct.gguf");
console.log(`GGUF 版本: ${ggufMeta.version}`);
console.log(`包含张量数: ${ggufMeta.tensor_count}`);
console.log(`模型架构: ${ggufMeta.metadata['general.architecture']}`);

// 解析 HuggingFace SafeTensors 权重文件
const stMeta = readSafeTensorsMetadata("./models/model.safetensors");
console.log(`张量总数: ${stMeta.tensors.length}`);
for (const t of stMeta.tensors) {
  console.log(`- ${t.name}: 类型=${t.dtype}, 形状=[${t.shape.join(', ')}]`);
}
```

---

## 3. 零拷贝张量加载与 `bee:ai` 互操作

通过 `loadTensor`，运行时直接返回基于 mmap 共享背衬的 `ArrayBuffer`：

```typescript
import { loadTensor } from 'bee:weights';
import { Tensor } from 'bee:ai';

// 零拷贝加载指定的权重张量
const loaded = loadTensor("./models/model.safetensors", "model.embed_tokens.weight");
console.log(`已载入张量: ${loaded.name}, 字节数: ${loaded.byteLength}`);

// 直接构造高性能计算 Tensor（无内存深拷贝）
const weightTensor = Tensor.fromBuffer(loaded.buffer, loaded.shape, 'float32');

// 开展矩阵乘法与高维向量运算
const inputEmbeddings = Tensor.ones([1, loaded.shape[0]]);
const projected = inputEmbeddings.matmul(weightTensor);
console.log(`输出范数: ${projected.norm()}`);
```

亦可直接通过 `ai.weights` 便捷访问：

```typescript
import ai from 'bee:ai';

const tensor = ai.weights.loadTensor("./model.safetensors", "layer1.weight");
```

---

## 4. 性能与资源消耗对比

| 测试场景 | Node.js (fs.readFileSync) | Python (safetensors) | Beejs (bee:weights) |
| :--- | :--- | :--- | :--- |
| **头部解析 (7B 模型)** | ~120 ms | ~4 ms | **< 1 ms** |
| **内存开销** | 全文件尺寸常驻 V8 堆 | 零拷贝 mmap | **零 V8 堆冗余拷贝** |
| **张量切片至数学对象** | 内存拷贝 + TypedArray | Numpy view | **原生 `Tensor.fromBuffer`** |

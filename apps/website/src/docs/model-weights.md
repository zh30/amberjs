---
title: "GGUF & SafeTensors Model Weights Loader (bee:weights)"
subtitle: "High-performance binary header parsing, zero-copy mmap tensor slicing, and bee:ai integration"
group: "Agent & Advanced"
id: "model-weights"
---

Deploying localized edge AI agents requires loading model weights (such as quantized LLMs, embedding projections, and transformer attention heads) rapidly into memory. Traditional Node.js solutions rely on heavy C++ addons or full file buffer copies that exhaust V8 heap memory.

**Beejs v1.6.0 introduces native binary GGUF and SafeTensors model loaders (`bee:weights` / `bee:ai.weights`)**. Built on Rust memory mapping (`mmap`), it inspects multi-gigabyte model files instantaneously and loads tensors with zero heap allocation overhead directly into `bee:ai.Tensor`.

---

## 1. Supported Formats

| Format | Magic / Header | Supported Versions | Key Features |
| :--- | :--- | :--- | :--- |
| **GGUF** | `b"GGUF"` | v2, v3 | Quantized weights (Q4_0, Q8_0, F16, F32), KV metadata parsing, alignment padding |
| **SafeTensors** | 8-byte LE JSON Header | HuggingFace SafeTensors | PyTorch / Safetensors JSON headers, shape extraction, contiguous buffer slicing |

---

## 2. Fast Metadata Inspection

Both formats can be inspected in sub-millisecond time without reading tensor weight payloads into memory:

```typescript
import { readGGUFMetadata, readSafeTensorsMetadata } from 'bee:weights';

// Inspect GGUF model metadata (llama.cpp format)
const ggufMeta = readGGUFMetadata("./models/qwen2.5-0.5b-instruct.gguf");
console.log(`Model version: ${ggufMeta.version}`);
console.log(`Tensors present: ${ggufMeta.tensor_count}`);
console.log(`Architecture: ${ggufMeta.metadata['general.architecture']}`);

// Inspect HuggingFace SafeTensors file
const stMeta = readSafeTensorsMetadata("./models/model.safetensors");
console.log(`Tensors count: ${stMeta.tensors.length}`);
for (const t of stMeta.tensors) {
  console.log(`- ${t.name}: dtype=${t.dtype}, shape=[${t.shape.join(', ')}]`);
}
```

---

## 3. Zero-Copy Tensor Slicing & `bee:ai` Integration

Using `loadTensor`, the runtime returns a sliced `ArrayBuffer` referencing memory-mapped bytes directly:

```typescript
import { loadTensor } from 'bee:weights';
import { Tensor } from 'bee:ai';

// Load embedding weight tensor directly
const loaded = loadTensor("./models/model.safetensors", "model.embed_tokens.weight");

console.log(`Loaded: ${loaded.name} (${loaded.dtype}), bytes=${loaded.byteLength}`);

// Construct zero-copy Tensor for high-speed linear algebra
const weightTensor = Tensor.fromBuffer(loaded.buffer, loaded.shape, 'float32');

// Execute matrix operations with zero copying
const inputEmbeddings = Tensor.ones([1, loaded.shape[0]]);
const projected = inputEmbeddings.matmul(weightTensor);
console.log(`Projected norm: ${projected.norm()}`);
```

You can also access the loader directly via `ai.weights`:

```typescript
import ai from 'bee:ai';

const tensor = ai.weights.loadTensor("./model.safetensors", "layer1.weight");
```

---

## 4. Performance & Memory Comparison

| Task | Node.js (fs.readFileSync) | Python (safetensors) | Beejs (bee:weights) |
| :--- | :--- | :--- | :--- |
| **Header Read (7B Model)** | ~120 ms | ~4 ms | **< 1 ms** |
| **Memory Overhead** | Full file size in RAM | Zero-copy mmap | **Zero V8 heap copy (mmap)** |
| **Tensor Load to Math Object**| Buffer clone + Type array | Numpy view | **Instant `Tensor.fromBuffer`** |

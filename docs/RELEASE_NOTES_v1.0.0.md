# Amber v1.0.0 Release Notes: The First Official Stable Release

> **Release Tag**: `v1.0.0`  
> **Milestone**: 3-Year Roadmap 2026–2029 (Year 1 Alignment: Edge & AI-Native Runtime)  
> **Status**: Production Stable  

---

## 🌟 Executive Summary

We are thrilled to announce **Amber v1.0.0**, the very first official stable release of the Amber JavaScript/TypeScript runtime!

Built with **Rust** and Google's **V8 engine**, Amber is designed from the ground up to be an **AI-Native, Edge-Optimized, and Sovereign-Sandboxed** runtime. Fully aligning with Year 1 of the *Amber 3-Year Technical Topology & Vision Blueprint (2026–2029)*, v1.0.0 delivers first-class native Agentic AI infrastructure, zero-copy `mmap` snapshot cold start, expanded Node.js conformance, and deterministic sandbox replay.

---

## 🚀 Key Highlights & New Capabilities

### 1. Native Agentic AI Engine (`amber:ai`)
Amber v1.0.0 introduces `amber:ai`, an in-engine AI acceleration layer eliminating the overhead of external Python processes or IPC serialization.
- **Zero-Copy Tensor**: Multi-dimensional tensor structures backed by native TypedArrays (`Float32Array`). Native operations include 2D matrix multiplication (`matmul`), dot product (`dot`), L2 norm (`norm`), numerically stable `softmax`, Hadamard arithmetic (`add`, `sub`, `mul`), and cosine similarity (`cosineSimilarity`).
- **Local Streaming LLM (`LLM`)**: Standardized model abstraction with `LLM.load(modelPath, options)`. Supports single-shot `generate()`, token stream generator `for await (const chunk of model.generateStream())`, and semantic vector embeddings via `model.embed()`.
- **Agent Pipeline (`AgentPipeline`)**: Autonomous tool-use execution loops with history tracking, deterministic step scheduling, and sandbox isolation.

```typescript
import { LLM, Tensor, cosineSimilarity } from 'amber:ai';

// 1. Zero-copy Tensor operations
const a = new Tensor([1, 2, 3]);
const b = new Tensor([4, 5, 6]);
console.log('Cosine similarity:', cosineSimilarity(a, b));

// 2. Local streaming LLM inference
const model = await LLM.load('qwen-2.5-7b-quant.gguf', { device: 'metal' });
for await (const chunk of model.generateStream('Explain quantum computing')) {
  process.stdout.write(chunk);
}
```

### 2. V8 Snapshot 2.0 with Memory-Mapped CoW Loading
- Memory-mapped snapshot loading powered by `memmap2::Mmap` directly into V8 isolate startup parameters.
- Achieves sub-millisecond cold start with zero-copy memory page sharing across high-concurrency worker isolates.
- Automated version binding (`v1.0.0`) and self-healing cache management via `amber snapshot [build|status|clean]`.

### 3. Expanded Node.js Conformance (100% PASS)
- Expanded the conformance suite from 45 to 51+ automated fixtures.
- Complete coverage for `amber:ai`, `events` (`listenerCount`, `prependListener`, `eventNames`), `util.promisify` / `util.types`, `string_decoder` streaming UTF-8 chunking, `perf_hooks` metrics, and POSIX path resolution.
- Node.js Conformance Scorecard verified at **100% pass rate**.

### 4. Deterministic Sandbox & Virtual Time (Deterministic Replay)
- Deterministic PRNG seeding via `--seed <u64>` for `Math.random()`, `crypto.getRandomValues()`, and `crypto.randomBytes()`.
- Deterministic virtual clock freezing via `--freeze-time <spec>` for reproducible AI Agent trajectory recording and regression testing.
- Fine-grained permission sandbox `--sandbox` with `--allow-*` allowlists and structured JSONL audit logs (`--audit-log <path>`).

### 5. Multi-Isolate Concurrency & WebAssembly Streaming
- True OS thread-backed V8 isolates for `worker_threads` and `globalThis.Worker` with bi-directional `MessageChannel` messaging.
- Native streaming WebAssembly instantiation (`WebAssembly.compileStreaming`, `WebAssembly.instantiateStreaming`) consuming `Response` and `Promise<Response>` without intermediate buffer string corruption.

---

## 📦 Upgrade Guide

To build and run the official v1.0.0 release:

```bash
cargo build --release
./target/release/amber --version
# amberjs 1.0.0
```

Run an AI script:
```bash
./target/release/amber eval "const { Tensor, cosineSimilarity } = require('amber:ai'); console.log('Similarity:', cosineSimilarity([1, 0], [0, 1]));"
```

Verify snapshot status:
```bash
./target/release/amber snapshot status
```

Run test suite:
```bash
./target/release/amber test
```

---

## 🗺️ Looking Ahead: The Road to 2027–2029

With v1.0.0 reaching stable graduation, Amber enters Year 2 of the 3-Year Roadmap:
- **2027–2028**: Polyglot Bridge (PyO3 / Wasm Component Model matrix), Linux Landlock + eBPF kernel sandbox, native `io_uring` event loop.
- **2028–2029**: Distributed Isolate Mesh (**BeeGrid**) and live cross-node Isolate hot migration.

Thank you to everyone who contributed to this monumental milestone!

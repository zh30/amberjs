---
title: "Comprehensive API Reference"
subtitle: "Complete API directory for Beejs native modules (bee:*), Node.js compatibility, and Web standards"
group: "Reference & Specs"
id: "api-reference"
---

Beejs delivers a unified runtime environment exposing three foundational API tiers:
1. **Beejs Native Modules (`bee:*`)**: Purpose-built subsystems for AI Agent execution, persistent state, streaming grammars, and sandboxing.
2. **Node.js Core Modules (`node:*`)**: 51/51 conformance suites passing for drop-in npm package compatibility.
3. **W3C / WHATWG Web Standards**: Universal browser-compatible primitives (`fetch`, `WebCrypto`, `WebStreams`, `Worker`).

---

## 1. Beejs Native API Reference (`bee:*`)

All native Beejs modules can be imported using the canonical `bee:<module>` specifier or its unqualified short identifier (e.g. `import { open } from 'bee:kv'` or `const { open } = require('kv')`).

```
                              [ Beejs Native Fabric ]
   +------------------------------------+------------------------------------+
   |   Autonomous Agent Subsystems      |   Runtime & Infrastructure         |
   |   ---------------------------      |   ------------------------         |
   |   • bee:ai        (Tensors & LLM)  |   • bee:kv        (ACID State)     |
   |   • bee:bus       (PubSub Fabric)  |   • bee:sandbox   (Micro-Enclaves) |
   |   • bee:grammar   (Stream Repair)  |   • bee:security  (Permissions)    |
   |   • bee:checkpoint(Time-Travel)    |   • bee:db        (SQLite Engine)  |
   |   • bee:tools     (OpenAPI Synth)  |   • bee:vector    (Vector Search)  |
   |   • bee:replay    (Trace Replay)   |   • bee:wasm      (Zero-Copy JIT)  |
   |   • bee:weights   (GGUF Slicing)   |   • bee:ffi       (Native C ABI)   |
   |   • bee:mcp       (MCP 2.0 Client) |   • bee:std       (Std Library)    |
   +------------------------------------+------------------------------------+
```

### 1.1 `bee:ai` — Edge Tensor Computing & Agent Pipelines
```typescript
import { Tensor, LLM, AgentPipeline, embed, embedBatch, generate, generateStream, cosineSimilarity } from 'bee:ai';
```
| Function / Class | Signature | Description |
| :--- | :--- | :--- |
| `Tensor` | `new Tensor(shape: number[], data?: number[] \| Float32Array, dtype?: string)` | N-dimensional tensor supporting `matmul()`, `add()`, `slice()`, `norm()`, and `fromBuffer()`. |
| `LLM` | `new LLM(config?: { model?: string, maxTokens?: number })` | Local or remote inference engine with `generate()` and `generateStream()`. |
| `AgentPipeline` | `new AgentPipeline()` | Multi-step reasoning pipeline supporting `registerTool()`, `registerTools()`, and `step()`. |
| `embed(text: string)` | `Promise<number[]>` | Generates high-density vector embeddings (384/768 dim). |
| `embedBatch(texts: string[])` | `Promise<number[][]>` | Batch generates embeddings with parallel SIMD optimization. |
| `cosineSimilarity(a, b)` | `number` | Calculates cosine similarity between two float vectors. |

---

### 1.2 `bee:bus` — Multi-Agent Message Bus & PubSub Channel Fabric
```typescript
import { createBus, getDefaultBus, subscribe, once, unsubscribe, publish, broadcast, request, reply, use, topicMatches } from 'bee:bus';
```
| Method | Signature | Description |
| :--- | :--- | :--- |
| `subscribe(pattern, handler, opts?)` | `(pattern: string, handler: (msg: Message) => void, opts?: { priority?: number }) => Subscription` | Subscribes to hierarchical topics with `*` and `#` wildcards. |
| `once(pattern, handler, opts?)` | `(pattern: string, handler: (msg) => void, opts?) => Subscription` | Single-shot event subscriber automatically unregistering after first delivery. |
| `publish(topic, payload, opts?)` | `(topic: string, payload: any, opts?: PublishOptions) => Message` | Dispatches message to subscribers sorted by descending priority. |
| `request(topic, payload, opts?)` | `(topic: string, payload: any, opts?: { timeoutMs?: number }) => Promise<any>` | Bidirectional RPC request awaiting reply on correlation topic. |
| `reply(originalMsg, responsePayload)`| `(msg: Message, response: any) => Message` | Replies directly to message's `replyTo` return address. |
| `use(middleware)` | `(middleware: (msg, next) => void) => this` | Registers interceptor for tracing, authentication, or payload validation. |
| `getMetrics()` | `() => BusMetrics` | Returns `{ published_count, delivered_count, dead_letter_count, active_subscriptions }`. |
| `getDeadLetters()` | `() => Message[]` | Inspects unhandled messages routed to the Dead-Letter Queue. |

---

### 1.3 `bee:grammar` — Streaming Structured Output & Token Grammar Engine
```typescript
import { parsePartialJSON, createStreamDecoder, parseSSEChunk, createGrammar, createChoiceGrammar, createRegexGrammar, createJSONGrammar } from 'bee:grammar';
```
| Method | Signature | Description |
| :--- | :--- | :--- |
| `parsePartialJSON(text)` | `(input: string) => any` | Microsecond-speed auto-repair for unclosed strings, open brackets `]`, and open braces `}`. |
| `createStreamDecoder(opts?)` | `(opts?: { onChunk?: (parsed, isComplete) => void }) => StreamDecoder` | Statefully accumulates text chunks and emits incrementally updated JSON snapshots. |
| `parseSSEChunk(chunk)` | `(chunk: string) => SSEMessage[]` | Parses raw Server-Sent Events stream chunks with built-in `.json()` accessor. |
| `createChoiceGrammar(choices)` | `(choices: string[]) => Grammar` | Token grammar restricting LLM generation to fixed string alternatives. |
| `createRegexGrammar(pattern)` | `(pattern: string \| RegExp) => Grammar` | Token grammar enforcing regular expression conformance. |
| `createJSONGrammar(schema?)` | `(schema?: any) => Grammar` | Token grammar verifying progressive valid JSON syntax. |

---

### 1.4 `bee:checkpoint` — Agent State Checkpoint & Time-Travel Snapshotting
```typescript
import { createCheckpointManager, getDefaultManager, save, restore, get, list, diff, fork, clear } from 'bee:checkpoint';
```
| Method | Signature | Description |
| :--- | :--- | :--- |
| `save(idOrOptions, state?, metadata?)`| `(id?: string, state?: any, meta?: object) => Checkpoint` | Captures an immutable deep clone of current agent state linked to lineage tree. |
| `restore(id)` | `(id: string) => any` | Restores agent state to a historical checkpoint for clean fault rollback. |
| `diff(fromId, toId)` | `(fromId: string, toId: string) => StateDiff` | Structural delta identifying `{ added, modified: { from, to }, deleted }`. |
| `fork(fromId, branchName)` | `(fromId: string, branchName: string) => CheckpointManager` | Creates speculative execution branch (Tree-of-Thought) without mutating main branch. |
| `persist(kvStore, prefix?)` | `(kvStore: KVStore, prefix?: string) => number` | Flushes all checkpoints to a durable `bee:kv` Write-Ahead Log. |
| `restoreFromKV(kvStore, prefix?)` | `(kvStore: KVStore, prefix?: string) => number` | Restores complete checkpoint lineage from a `bee:kv` store instance. |

---

### 1.5 `bee:kv` — Persistent Key-Value & Durable State Engine
```typescript
import { open, openInMemory, KVStore } from 'bee:kv';
```
| Method | Signature | Description |
| :--- | :--- | :--- |
| `open(pathOrOptions)` | `(options: string \| { path: string }) => KVStore` | Opens durable disk-backed key-value store with append-only Write-Ahead Log (WAL). |
| `openInMemory()` | `() => KVStore` | Opens ultra-low latency in-memory transient key-value store. |
| `get(key)` | `(key: string) => any` | Reads key; returns `undefined` if key does not exist or has expired. |
| `set(key, value, ttlMs?)` | `(key: string, value: any, ttlMs?: number) => this` | Writes key-value pair with optional automatic millisecond TTL expiration. |
| `delete(key)` | `(key: string) => boolean` | Deletes key from memory and appends tombstone to WAL. |
| `incr(key, delta?)` | `(key: string, delta?: number) => number` | Atomic integer increment operation. |
| `scan(options?)` | `(options?: { prefix?: string, limit?: number }) => Array<{ key, value }>` | High-performance prefix range scanning. |
| `batch(operations)` | `(ops: Array<{ type: 'set' \| 'delete', key, value?, ttlMs? }>) => this` | Executes multiple write operations in a single atomic transaction. |
| `compact()` | `() => boolean` | Prunes expired entries and compacts WAL log file to minimize disk footprint. |

---

### 1.6 `bee:tools` — Agent Tool Auto-Synthesis & OpenAPI Schema Compiler
```typescript
import { compileSchemaTool, fromOpenAPI, parseToolCalls, registerTools, AgentTool } from 'bee:tools';
```
| Method | Signature | Description |
| :--- | :--- | :--- |
| `compileSchemaTool(spec)` | `(spec: ToolDefinition) => AgentTool` | Compiles JSON Schema into validated callable `AgentTool`. |
| `fromOpenAPI(spec, options?)` | `(spec: object \| string, opts?: OpenAPIOptions) => AgentTool[]` | Automatically synthesizes executable tools from OpenAPI 3.x specifications. |
| `parseToolCalls(llmOutput)` | `(llmOutput: string \| object) => ToolCall[]` | Robustly extracts tool calls from JSON, arrays, markdown code blocks, and tags. |
| `registerTools(pipeline, tools)` | `(pipeline: AgentPipeline, tools: AgentTool[]) => AgentPipeline` | Binds synthesized tools directly into an `AgentPipeline`. |

---

### 1.7 `bee:sandbox` — Hardened Micro-Enclaves & Real-time Audit Logging
```typescript
import { createEnclave, startAuditLog, stopAuditLog, getAuditLogPath, isEnabled, enable, disable } from 'bee:sandbox';
```
| Method | Signature | Description |
| :--- | :--- | :--- |
| `createEnclave(policyOrCode, opts?)` | `(policy?: EnclavePolicy) => SandboxEnclave` | Creates a zero-privilege micro-enclave with memory, timeout, and whitelist limits. |
| `startAuditLog(path)` | `(path: string) => boolean` | Streams real-time compliance JSONL audit logs for all security decisions. |
| `stopAuditLog()` | `() => boolean` | Flushes and finalizes active compliance audit log. |
| `getAuditLogPath()` | `() => string \| null` | Queries the currently active audit log file path. |

---

### 1.8 `bee:replay` — Deterministic Agent Replay Engine
```typescript
import { startRecording, stopRecording, loadTrace, step, isRecording, isReplaying, getTraceStats } from 'bee:replay';
```
| Method | Signature | Description |
| :--- | :--- | :--- |
| `startRecording(opts)` | `(opts: { script?: string, outputPath?: string }) => void` | Arms engine to record non-deterministic inputs into `.bee-trace.json`. |
| `stopRecording(path?)` | `(path?: string) => AgentTrace` | Finalizes recording and exports trace file. |
| `loadTrace(traceOrPath)` | `(trace: string \| object) => void` | Loads trace and arms offline deterministic replay mode. |
| `step(name, input, fn)` | `(name: string, input: any, fn: (input) => any) => any` | Records during live run; intercepts and replays cached outputs during replay. |

---

### 1.9 `bee:weights` — Native GGUF & SafeTensors Model Weights Loader
```typescript
import { readGGUFMetadata, readSafeTensorsMetadata, loadTensor } from 'bee:weights';
```
| Method | Signature | Description |
| :--- | :--- | :--- |
| `readGGUFMetadata(filePath)` | `(path: string) => GGUFMetadata` | Sub-millisecond inspection of GGUF v2/v3 tensor headers and KV pairs. |
| `readSafeTensorsMetadata(filePath)`| `(path: string) => SafeTensorsMetadata` | Inspects HuggingFace SafeTensors file headers. |
| `loadTensor(filePath, tensorName)` | `(path: string, name: string) => LoadedTensor` | Memory-maps tensor weights zero-copy into typed `ArrayBuffer`. |

---

### 1.10 `bee:security` & `bee:permissions` — Enterprise Capability Security
```typescript
import { permissions, createSandboxPolicy, attenuate } from 'bee:security';
```
| Method | Signature | Description |
| :--- | :--- | :--- |
| `permissions.query(descriptor)` | `(desc: PermissionDescriptor) => Promise<PermissionStatus>` | Queries whether specific I/O permission is currently granted. |
| `permissions.has(descriptor)` | `(desc: PermissionDescriptor) => boolean` | Synchronous boolean capability inspection. |
| `permissions.revoke(descriptor)`| `(desc: PermissionDescriptor) => boolean` | Drops privileged access dynamically at runtime. |
| `permissions.list()` | `() => PermissionRules` | Dumps currently active allow/deny rule sets. |
| `attenuate(base, restricted)` | `(base: Policy, restricted: Policy) => Policy` | Computes mathematical least-privilege intersection of permissions. |

---

### 1.11 `bee:db` & `bee:vector` — Embedded SQLite & Vector Search
```typescript
import { Database } from 'bee:db';
import { VectorDB } from 'bee:vector';

// SQLite
const db = Database.open('./data.db');
db.exec('CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT)');
const stmt = db.prepare('INSERT INTO users (name) VALUES (?)');
stmt.run('Alice');

// Vector Engine
const vecDb = VectorDB.create({ dimension: 384, metric: 'cosine' });
vecDb.insert('doc-1', embeddingArray, { title: 'Introduction' });
const results = vecDb.search(queryEmbedding, { limit: 5 });
```

---

### 1.12 `bee:std` — Modern Standard Library
```typescript
import { config } from 'bee:std/dotenv';
import { colors, table } from 'bee:std/cli';
import { walkDir, ensureDir } from 'bee:std/fs';
import { uuid, signJwt, verifyJwt } from 'bee:std/crypto';
import { assert, assertEquals } from 'bee:std/assert';

config({ path: '.env' });
console.log(colors.green('Environment loaded successfully'));
```

---

### 1.13 `bee:wasm`, `bee:ffi` & `bee:pool` — Native Interop & Concurrency
```typescript
// WebAssembly 2.0 Shared Memory Bridge (bee:wasm)
import { compile, instantiate, MemoryView } from 'bee:wasm';

// Native C ABI FFI (bee:ffi)
import { dlopen, CString, types } from 'bee:ffi';
const libm = dlopen('libm.dylib', { cos: { args: [types.f64], returns: types.f64 } });

// Multi-Tenant IsolatePool (bee:pool)
import { IsolatePool } from 'bee:pool';
const pool = new IsolatePool({ size: 4, memoryLimitMb: 128 });
const result = await pool.execute('2 + 3');
```

---

## 2. Node.js Core Modules Reference (`node:*`)

Beejs passes 51/51 official Node.js conformance test suites with hardware SIMD acceleration:

| Module | Specifier | Primary APIs | Status |
| :--- | :--- | :--- | :---: |
| **`node:fs`** | `fs`, `node:fs`, `node:fs/promises` | `readFile`, `writeFile`, `stat`, `readdir`, `mkdir`, `rm`, `createReadStream`, `createWriteStream` | ✅ 100% |
| **`node:path`** | `path`, `node:path` | `join`, `resolve`, `dirname`, `basename`, `extname`, `normalize`, `isAbsolute` | ✅ 100% |
| **`node:crypto`** | `crypto`, `node:crypto` | `createHash`, `createHmac`, `randomBytes`, `randomUUID`, `pbkdf2`, AES-GCM/CBC | ✅ 100% |
| **`node:buffer`** | `buffer`, `node:buffer` | `Buffer.from`, `Buffer.alloc`, `Buffer.concat`, `isBuffer`, `toString` (SIMD accelerated) | ✅ 100% |
| **`node:events`** | `events`, `node:events` | `EventEmitter`, `on`, `once`, `emit`, `removeListener`, `listenerCount` | ✅ 100% |
| **`node:stream`** | `stream`, `node:stream` | `Readable`, `Writable`, `Transform`, `pipeline`, `finished` | ✅ 100% |
| **`node:http`** | `http`, `node:http` | `createServer`, `IncomingMessage`, `ServerResponse`, `request`, `get`, Keep-Alive | ✅ 100% |
| **`node:process`**| `process`, `node:process` | `argv`, `env`, `cwd()`, `exit()`, `uptime()`, `memoryUsage()`, `nextTick()` | ✅ 100% |
| **`node:timers`** | `timers`, `node:timers` | `setTimeout`, `clearTimeout`, `setInterval`, `clearInterval`, `setImmediate` | ✅ 100% |
| **`node:url`** | `url`, `node:url` | `URL`, `URLSearchParams`, `fileURLToPath`, `pathToFileURL` | ✅ 100% |
| **`node:dns`** | `dns`, `node:dns` | `lookup`, `resolve`, `resolve4`, `resolve6` asynchronous DNS queries | ✅ 100% |
| **`node:perf_hooks`**| `perf_hooks`, `node:perf_hooks`| `performance.now()`, `PerformanceObserver` | ✅ 100% |

---

## 3. Web Standards API Surface

Universally accessible on `globalThis` without import:

| Web API | Description | Global Access |
| :--- | :--- | :---: |
| **`fetch()`** | Universal network request interface with streaming bodies | ✅ `globalThis.fetch` |
| **`Headers`, `Request`, `Response`** | Fetch API primitives | ✅ `globalThis.*` |
| **`URL`, `URLSearchParams`** | WHATWG URL specification parser | ✅ `globalThis.*` |
| **`WebSocket`** | Standard real-time full-duplex client socket | ✅ `globalThis.WebSocket` |
| **`crypto.subtle` (Web Crypto)** | Cryptography: `digest`, `encrypt`, `decrypt`, `sign`, `verify` | ✅ `globalThis.crypto.subtle` |
| **`ReadableStream`, `WritableStream`** | WHATWG Streams standard for data pipelines | ✅ `globalThis.*` |
| **`CompressionStream`** | Native streaming `gzip` and `deflate` compression | ✅ `globalThis.CompressionStream` |
| **`Blob`, `File`, `FormData`** | Binary and multipart containers | ✅ `globalThis.*` |
| **`structuredClone()`** | Native deep-cloning for complex object graphs | ✅ `globalThis.structuredClone` |
| **`TextEncoder`, `TextDecoder`** | High-performance UTF-8 conversion | ✅ `globalThis.*` |
| **`Worker`** | Multi-threaded Web Worker execution | ✅ `globalThis.Worker` |

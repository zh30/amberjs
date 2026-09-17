---
title: "Beejs 全量 API 核心速查手册"
subtitle: "涵盖 Beejs 原生模块 (bee:*)、Node.js 兼容层与现代 Web 标准的完整 API 字典"
group: "参考与规范"
id: "api-reference"
---

Beejs 运行时提供三层相互贯通的 API 体系结构：
1. **Beejs 原生核心模块 (`bee:*`)**：专为自治 AI Agent、可靠持久状态、流式语法自愈及沙箱安全打造的高性能内置模块。
2. **Node.js 兼容层 (`node:*`)**：100% 通过 51 项 Node.js 官方一致性测试套件，提供对主流 npm 包的开箱即用支持。
3. **W3C / WHATWG Web 平台标准**：与现代浏览器、Cloudflare Workers、Deno 统一的跨端标准接口（`fetch`、`WebCrypto`、`Streams` 等）。

---

## 1. Beejs 原生核心 API 参考 (`bee:*`)

所有 Beejs 原生模块均支持规范协议前缀 `bee:<module>` 或无前缀短名导入（例如 `import { open } from 'bee:kv'` 或 `const { open } = require('kv')`）。

```
                                [ Beejs 原生架构全景 ]
   +------------------------------------+------------------------------------+
   |       自治 AI Agent 核心子系统       |          运行时与系统底座           |
   |   ---------------------------      |   ------------------------         |
   |   • bee:ai        (张量与 LLM)     |   • bee:kv        (ACID 状态存储)  |
   |   • bee:bus       (多 Agent 总线)   |   • bee:sandbox   (加固微飞地)     |
   |   • bee:grammar   (流式语法自愈)    |   • bee:security  (零信任权限控制)  |
   |   • bee:checkpoint(时间旅行快照)    |   • bee:db        (嵌入式 SQLite)  |
   |   • bee:tools     (工具与 OpenAPI)  |   • bee:vector    (向量检索引擎)    |
   |   • bee:replay    (确定性回放)      |   • bee:wasm      (零拷贝 Wasm 2.0)|
   |   • bee:weights   (模型权重加载)    |   • bee:ffi       (原生 C ABI 接口)|
   |   • bee:mcp       (MCP 2.0 协议)   |   • bee:std       (官方现代标准库)  |
   +------------------------------------+------------------------------------+
```

### 1.1 `bee:ai` — 端侧张量计算与 Agent 管道
```typescript
import { Tensor, LLM, AgentPipeline, embed, embedBatch, generate, generateStream, cosineSimilarity } from 'bee:ai';
```
| 方法 / 类型 | 类型签名 | 核心功能说明 |
| :--- | :--- | :--- |
| `Tensor` | `new Tensor(shape: number[], data?: number[] \| Float32Array, dtype?: string)` | 原生多维张量，支持 `matmul()`, `add()`, `slice()`, `norm()` 及零拷贝 `fromBuffer()`。 |
| `LLM` | `new LLM(config?: { model?: string, maxTokens?: number })` | 端侧/远程模型推演引擎，提供 `generate()` 及流式 `generateStream()`。 |
| `AgentPipeline` | `new AgentPipeline()` | 自治多步推理流水线，支持 `registerTool()`, `registerTools()` 及 `step()`。 |
| `embed(text)` | `(text: string) => Promise<number[]>` | 生成 384/768 维高密度文本语义向量嵌入。 |
| `embedBatch(texts)` | `(texts: string[]) => Promise<number[][]>` | 批量多文本并行向量化计算（SIMD 优化加速）。 |
| `cosineSimilarity(a, b)` | `(a: number[], b: number[]) => number` | 高精度计算两个浮点特征向量的余弦相似度。 |

---

### 1.2 `bee:bus` — 多 Agent 消息总线与 PubSub 网格
```typescript
import { createBus, getDefaultBus, subscribe, once, unsubscribe, publish, broadcast, request, reply, use, topicMatches } from 'bee:bus';
```
| 方法 | 类型签名 | 核心功能说明 |
| :--- | :--- | :--- |
| `subscribe(pattern, handler, opts?)` | `(pattern: string, handler: (msg: Message) => void, opts?: { priority?: number }) => Subscription` | 订阅点分隔层级主题，支持单段通配符 `*` 与多段通配符 `#`。 |
| `once(pattern, handler, opts?)` | `(pattern: string, handler: (msg) => void, opts?) => Subscription` | 单次生效监听器，触发一次后自动注销。 |
| `publish(topic, payload, opts?)` | `(topic: string, payload: any, opts?: PublishOptions) => Message` | 发布消息，按订阅者优先级从高到低同步/异步顺序派发。 |
| `request(topic, payload, opts?)` | `(topic: string, payload: any, opts?: { timeoutMs?: number }) => Promise<any>` | 双向请求-响应模式（RPC），在临时关联主题上等待返回，内置超时熔断。 |
| `reply(originalMsg, responsePayload)`| `(msg: Message, response: any) => Message` | 针对带有 `replyTo` 回执地址的消息进行直接回复。 |
| `use(middleware)` | `(middleware: (msg, next) => void) => this` | 注册消息中间件，用于注入 TraceId、鉴权校验或参数清洗。 |
| `getMetrics()` | `() => BusMetrics` | 获取 `{ published_count, delivered_count, dead_letter_count, active_subscriptions }`。 |
| `getDeadLetters()` | `() => Message[]` | 检索因没有活跃订阅者或过期而落入死信队列的消息。 |

---

### 1.3 `bee:grammar` — 流式结构化输出与 Token 语法引擎
```typescript
import { parsePartialJSON, createStreamDecoder, parseSSEChunk, createGrammar, createChoiceGrammar, createRegexGrammar, createJSONGrammar } from 'bee:grammar';
```
| 方法 | 类型签名 | 核心功能说明 |
| :--- | :--- | :--- |
| `parsePartialJSON(text)` | `(input: string) => any` | 微秒级实时自愈大模型未生成完毕的截断 JSON，补齐引号、数组 `]` 与对象 `}`。 |
| `createStreamDecoder(opts?)` | `(opts?: { onChunk?: (parsed, isComplete) => void }) => StreamDecoder` | 增量维护流式缓冲区，数据块到达时即时计算最新结构化快照。 |
| `parseSSEChunk(chunk)` | `(chunk: string) => SSEMessage[]` | 解析标准 Server-Sent Events 事件流，内建 `.json()` 解析器。 |
| `createChoiceGrammar(choices)` | `(choices: string[]) => Grammar` | 构建固定字符串候选项约束语法，提供前缀 `accept()` 判定。 |
| `createRegexGrammar(pattern)` | `(pattern: string \| RegExp) => Grammar` | 构建正则表达式合法性验证语法。 |
| `createJSONGrammar(schema?)` | `(schema?: any) => Grammar` | 构建 JSON 格式与 Schema 逐 Token 校验语法。 |

---

### 1.4 `bee:checkpoint` — Agent 状态检查点与时间旅行回退
```typescript
import { createCheckpointManager, getDefaultManager, save, restore, get, list, diff, fork, clear } from 'bee:checkpoint';
```
| 方法 | 类型签名 | 核心功能说明 |
| :--- | :--- | :--- |
| `save(idOrOptions, state?, metadata?)`| `(id?: string, state?: any, meta?: object) => Checkpoint` | 创建中间推理状态的不可变深拷贝快照，挂载至血缘追溯树。 |
| `restore(id)` | `(id: string) => any` | 提取并回退状态至指定历史检查点，支持出错推倒重来。 |
| `diff(fromId, toId)` | `(fromId: string, toId: string) => StateDiff` | 计算两个检查点之间的深层结构增量 `{ added, modified: { from, to }, deleted }`。 |
| `fork(fromId, branchName)` | `(fromId: string, branchName: string) => CheckpointManager` | 基于某一历史节点派生全新投机探索分支（Tree-of-Thought），不污染主线。 |
| `persist(kvStore, prefix?)` | `(kvStore: KVStore, prefix?: string) => number` | 将内存检查点血缘树批量写入 `bee:kv` 磁盘 WAL 日志。 |
| `restoreFromKV(kvStore, prefix?)` | `(kvStore: KVStore, prefix?: string) => number` | 从 `bee:kv` 持久化存储中重新灌入完整历史检查点。 |

---

### 1.5 `bee:kv` — 持久化键值存储与可靠状态引擎
```typescript
import { open, openInMemory, KVStore } from 'bee:kv';
```
| 方法 | 类型签名 | 核心功能说明 |
| :--- | :--- | :--- |
| `open(pathOrOptions)` | `(options: string \| { path: string }) => KVStore` | 打开带有 WAL 预写日志的磁盘持久化 KV 存储。 |
| `openInMemory()` | `() => KVStore` | 打开极速纯内存瞬态 KV 存储。 |
| `get(key)` | `(key: string) => any` | 读取键值；若不存在或已过期则返回 `undefined`。 |
| `set(key, value, ttlMs?)` | `(key: string, value: any, ttlMs?: number) => this` | 写入键值，支持毫秒级 TTL 自动过期。 |
| `delete(key)` | `(key: string) => boolean` | 删除指定键，同步记录墓碑标记至 WAL。 |
| `incr(key, delta?)` | `(key: string, delta?: number) => number` | 并发安全的原子整数递增操作。 |
| `scan(options?)` | `(options?: { prefix?: string, limit?: number }) => Array<{ key, value }>` | 高性能前缀范围检索扫描。 |
| `batch(operations)` | `(ops: Array<{ type: 'set' \| 'delete', key, value?, ttlMs? }>) => this` | 单事务批量写入/删除操作（原子执行）。 |
| `compact()` | `() => boolean` | 在线合并压缩 WAL 日志，清理失效数据并缩减磁盘体积。 |

---

### 1.6 `bee:tools` — 工具自动合成与 OpenAPI 编译器
```typescript
import { compileSchemaTool, fromOpenAPI, parseToolCalls, registerTools, AgentTool } from 'bee:tools';
```
| 方法 | 类型签名 | 核心功能说明 |
| :--- | :--- | :--- |
| `compileSchemaTool(spec)` | `(spec: ToolDefinition) => AgentTool` | 基于 JSON Schema 参数规范编译强类型校验的 `AgentTool`。 |
| `fromOpenAPI(spec, options?)` | `(spec: object \| string, opts?: OpenAPIOptions) => AgentTool[]` | 解析 OpenAPI 3.x 文档并自动转换为原生 `fetch` 驱动的工具集。 |
| `parseToolCalls(llmOutput)` | `(llmOutput: string \| object) => ToolCall[]` | 智能从裸 JSON、Markdown 代码块中提取结构化工具调用指令。 |
| `registerTools(pipeline, tools)` | `(pipeline: AgentPipeline, tools: AgentTool[]) => AgentPipeline` | 将合成工具批量注入并挂载至 `AgentPipeline`。 |

---

### 1.7 `bee:sandbox` — 加固微飞地与合规审计日志
```typescript
import { createEnclave, startAuditLog, stopAuditLog, getAuditLogPath, isEnabled, enable, disable } from 'bee:sandbox';
```
| 方法 | 类型签名 | 核心功能说明 |
| :--- | :--- | :--- |
| `createEnclave(policyOrCode, opts?)` | `(policy?: EnclavePolicy) => SandboxEnclave` | 构建零特权微飞地，支持硬性超时、内存上限与全局白名单上下文。 |
| `startAuditLog(path)` | `(path: string) => boolean` | 开启合规审计追踪，将所有底层 I/O 判决实时流式落盘为 JSONL。 |
| `stopAuditLog()` | `() => boolean` | 终止审计日志收集并刷新落盘缓冲区。 |
| `getAuditLogPath()` | `() => string \| null` | 查询当前正在运行的审计日志绝对路径。 |

---

### 1.8 `bee:replay` — 确定性 Agent 回放引擎
```typescript
import { startRecording, stopRecording, loadTrace, step, isRecording, isReplaying, getTraceStats } from 'bee:replay';
```
| 方法 | 类型签名 | 核心功能说明 |
| :--- | :--- | :--- |
| `startRecording(opts)` | `(opts: { script?: string, outputPath?: string }) => void` | 进入录制状态，自动拦截时间戳、随机数与单步工具调用出入参。 |
| `stopRecording(path?)` | `(path?: string) => AgentTrace` | 结束录制并将完整轨迹写为 `.bee-trace.json` 文件。 |
| `loadTrace(traceOrPath)` | `(trace: string \| object) => void` | 载入历史执行轨迹并进入离线确定性重放模式。 |
| `step(name, input, fn)` | `(name: string, input: any, fn: (input) => any) => any` | 录制时执行真实逻辑；回放时直接注入历史快照返回值并校验分歧。 |

---

### 1.9 `bee:weights` — 原生 GGUF / SafeTensors 权重加载
```typescript
import { readGGUFMetadata, readSafeTensorsMetadata, loadTensor } from 'bee:weights';
```
| 方法 | 类型签名 | 核心功能说明 |
| :--- | :--- | :--- |
| `readGGUFMetadata(filePath)` | `(path: string) => GGUFMetadata` | 亚毫秒级解析 GGUF (v2/v3) 文件的张量头部与元数据 KV。 |
| `readSafeTensorsMetadata(filePath)`| `(path: string) => SafeTensorsMetadata` | 解析 HuggingFace SafeTensors 文件头部索引。 |
| `loadTensor(filePath, tensorName)` | `(path: string, name: string) => LoadedTensor` | 基于 `mmap` 零拷贝将权重切片映射为 TypedArray，直通 `bee:ai.Tensor`。 |

---

### 1.10 `bee:security` & `bee:permissions` — 企业级能力安全控制
```typescript
import { permissions, createSandboxPolicy, attenuate } from 'bee:security';
```
| 方法 | 类型签名 | 核心功能说明 |
| :--- | :--- | :--- |
| `permissions.query(descriptor)` | `(desc: PermissionDescriptor) => Promise<PermissionStatus>` | 查询指定的文件、网络、环境变量能力是否已授予。 |
| `permissions.has(descriptor)` | `(desc: PermissionDescriptor) => boolean` | 同步布尔检查能力状态。 |
| `permissions.revoke(descriptor)`| `(desc: PermissionDescriptor) => boolean` | 运行时主动永久废除敏感特权。 |
| `permissions.list()` | `() => PermissionRules` | 导出底层所有生效中的放行与阻断规则。 |
| `attenuate(base, restricted)` | `(base: Policy, restricted: Policy) => Policy` | 数学级计算权限交集约束，防止特权下发越界。 |

---

### 1.11 `bee:db` 与 `bee:vector` — 嵌入式数据库与向量底座
```typescript
import { Database } from 'bee:db';
import { VectorDB } from 'bee:vector';

// SQLite
const db = Database.open('./app.db');
db.exec('CREATE TABLE IF NOT EXISTS notes (id INTEGER PRIMARY KEY, title TEXT)');
const stmt = db.prepare('INSERT INTO notes (title) VALUES (?)');
stmt.run('会议纪要');

// 向量数据库
const vecDb = VectorDB.create({ dimension: 384, metric: 'cosine' });
vecDb.insert('note-1', embeddingArray, { category: 'meeting' });
const matches = vecDb.search(queryEmbedding, { limit: 5 });
```

---

### 1.12 `bee:std` — 现代化官方标准库
```typescript
import { config } from 'bee:std/dotenv';
import { colors, table } from 'bee:std/cli';
import { walkDir, ensureDir } from 'bee:std/fs';
import { uuid, signJwt, verifyJwt } from 'bee:std/crypto';
import { assert, assertEquals } from 'bee:std/assert';

config({ path: '.env' });
console.log(colors.cyan('Beejs 标准库环境就绪'));
```

---

### 1.13 `bee:wasm`, `bee:ffi` 与 `bee:pool` — 原生互通与隔离池
```typescript
// WebAssembly 2.0 共享内存桥接 (bee:wasm)
import { compile, instantiate, MemoryView } from 'bee:wasm';

// 原生 C ABI FFI (bee:ffi)
import { dlopen, CString, types } from 'bee:ffi';
const lib = dlopen('libm.dylib', { sin: { args: [types.f64], returns: types.f64 } });

// 多租户隔离池 (bee:pool)
import { IsolatePool } from 'bee:pool';
const pool = new IsolatePool({ size: 8, memoryLimitMb: 128 });
const output = await pool.execute('1 + 1');
```

---

## 2. Node.js 核心兼容模块速查 (`node:*`)

Beejs 在 51 项 Node.js 官方测试套件中达到 100% 满分通过率：

| 核心模块 | 协议前缀规范 | 主要支持 API | 状态 |
| :--- | :--- | :--- | :---: |
| **`node:fs`** | `fs`, `node:fs`, `node:fs/promises` | `readFile`, `writeFile`, `stat`, `readdir`, `mkdir`, `rm`, `createReadStream`, `createWriteStream` | ✅ 100% 满分 |
| **`node:path`** | `path`, `node:path` | `join`, `resolve`, `dirname`, `basename`, `extname`, `normalize`, `isAbsolute` | ✅ 100% 满分 |
| **`node:crypto`** | `crypto`, `node:crypto` | `createHash`, `createHmac`, `randomBytes`, `randomUUID`, `pbkdf2`, AES-GCM/CBC 加解密 | ✅ 100% 满分 |
| **`node:buffer`** | `buffer`, `node:buffer` | `Buffer.from`, `Buffer.alloc`, `Buffer.concat`, `isBuffer`, `toString` (SIMD 硬件加速) | ✅ 100% 满分 |
| **`node:events`** | `events`, `node:events` | `EventEmitter`, `on`, `once`, `emit`, `removeListener`, `listenerCount` | ✅ 100% 满分 |
| **`node:stream`** | `stream`, `node:stream` | `Readable`, `Writable`, `Transform`, `pipeline`, `finished` 及异步可迭代 | ✅ 100% 满分 |
| **`node:http`** | `http`, `node:http` | `createServer`, `IncomingMessage`, `ServerResponse`, `request`, `get`, Keep-Alive | ✅ 100% 满分 |
| **`node:process`**| `process`, `node:process` | `argv`, `env`, `cwd()`, `exit()`, `uptime()`, `memoryUsage()`, `nextTick()` | ✅ 100% 满分 |
| **`node:timers`** | `timers`, `node:timers` | `setTimeout`, `clearTimeout`, `setInterval`, `clearInterval`, `setImmediate` (时间轮优化) | ✅ 100% 满分 |
| **`node:url`** | `url`, `node:url` | `URL`, `URLSearchParams`, `fileURLToPath`, `pathToFileURL` | ✅ 100% 满分 |
| **`node:dns`** | `dns`, `node:dns` | `lookup`, `resolve`, `resolve4`, `resolve6` 异步 DNS 查询 | ✅ 100% 满分 |
| **`node:perf_hooks`**| `perf_hooks`, `node:perf_hooks`| `performance.now()`, `PerformanceObserver` 纳秒精度指标 | ✅ 100% 满分 |

---

## 3. Web 平台标准 API 全景

挂载于 `globalThis` 全局命名空间，无需单独 import：

| Web 标准 API | 规范与能力 | 全局直接可用 |
| :--- | :--- | :---: |
| **`fetch()`** | 标准网络请求，支持流式请求体与流式响应 | ✅ `globalThis.fetch` |
| **`Headers`, `Request`, `Response`** | Fetch API 基础对象 | ✅ 全局可用 |
| **`URL`, `URLSearchParams`** | WHATWG 统一资源定位符与查询参数规范 | ✅ 全局可用 |
| **`WebSocket`** | 现代全双工实时通信客户端 | ✅ 全局可用 |
| **`crypto.subtle` (Web Crypto)** | 密码学标准：`digest`, `encrypt`, `decrypt`, `sign`, `verify` | ✅ `globalThis.crypto.subtle` |
| **`ReadableStream`, `WritableStream`** | WHATWG Web Streams 标准流管道 | ✅ 全局可用 |
| **`CompressionStream`** | 原生 `gzip` 与 `deflate` 数据解压缩 | ✅ 全局可用 |
| **`Blob`, `File`, `FormData`** | 二进制大对象与表单传输格式 | ✅ 全局可用 |
| **`structuredClone()`** | 原生深拷贝（支持 Set、Map、Date、RegExp 与循环引用） | ✅ 全局可用 |
| **`TextEncoder`, `TextDecoder`** | 极速 UTF-8 文本编码转换 | ✅ 全局可用 |
| **`Worker`** | 多线程 Web Worker 原生并发 | ✅ 全局可用 |

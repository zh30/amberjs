# Amber v1.7.0 Release Notes: Persistent KV & Durable State Engine, Agent Tool Auto-Synthesis & OpenAPI Compiler, Hardened Micro-Enclave Sandbox & Real-time Audit Logging

> **Release Tag**: `v1.7.0`  
> **Release Type**: Minor Release (次版本 / 中版本)  
> **Target Commits**: Persistent KV & durable state engine (`amber:kv` / `amber:state`), Agent tool auto-synthesis & OpenAPI 3.x compiler (`amber:tools` / `amber:ai.tools`), hardened micro-enclave sandbox & real-time compliance audit logging engine (`amber:sandbox` & CLI `--audit-log`), TypeScript definitions (`types/amberjs.d.ts`), and 5-language documentation.

---

## 概述 (Overview)

Amber **v1.7.0** 是针对**长周期自治 Agent 状态管理、自动化工具生态集成与生产级合规加固执行**打造的关键里程碑版本。本版本实现了三大企业级核心子系统及完整的全语种开发者支持：

1. **持久化 KV 与可靠状态引擎 (`amber:kv` / `amber:state` / `amber:ai.kv`)**：
   - **嵌入式零外部依赖架构**：原生 Rust 实现，提供零配置、极轻量的持久化与内存键值存储，支持长效 Agent 记忆与会话状态持久化。
   - **WAL 事务日志与崩溃自愈**：通过写入日志 (Write-Ahead Log) 保证崩溃一致性与数据不丢失，并提供在线日志压缩合并 (`compact()`)。
   - **TTL 自动过期机制**：支持键值毫秒级生命周期管理，集成懒剔除与内存周期性主动清理。
   - **前缀范围扫描与原子自增**：支持 `scan({ prefix, limit, reverse })` 进行前缀索引检索，以及 `incr(key, delta)` 提供并发原子计数。
   - **全事务原子批处理 (`batch()`)**：单次原子操作执行批量 `set` 与 `delete`，失败自动回滚。

2. **工具自动合成与 OpenAPI 编译器 (`amber:tools` / `amber:ai.tools`)**：
   - **JSON Schema 校验工具编译器 (`compileSchemaTool`)**：从 JSON Schema 规范自动构建带强类型与参数校验的原生可调用工具对象。
   - **OpenAPI 3.x 规范自动解析与合成 (`fromOpenAPI`)**：一键将标准 OpenAPI 3.x 描述文档转换为可执行工具集合，内建参数解析、路径填充与原生 `fetch` 请求分发。
   - **LLM 工具调用鲁棒解析 (`parseToolCalls`)**：无缝解析大语言模型生成的 Markdown 代码块、混合文本或裸 JSON 工具调用指令。
   - **原生 AgentPipeline 协同集成 (`registerTools`)**：合成的工具直接挂载至 `amber:ai.AgentPipeline`，为自主 Agent 赋能开箱即用的外设调用能力。

3. **加固微飞地沙箱与实时合规审计日志 (`amber:sandbox` & CLI `--audit-log`)**：
   - **微飞地隔离运行 (`createEnclave(policy)`)**：在宿主 V8 进程内构建零特权隔离子执行空间，支持硬性执行超时、堆内存硬配额与受控能力白名单。
   - **实时 JSONL 合规审计日志流 (`startAuditLog` / `stopAuditLog`)**：对所有文件、网络、环境变量的判定操作进行带时间戳的不可篡改审计追踪。
   - **CLI 深度集成**：命令行直接支持 `--audit-log <path>`，为企业安全监管与 SOC2/ISO 合规提供透明审计凭据。

4. **统一类型系统与全球化多语言文档**：
   - `types/amberjs.d.ts` 与 `src/types_export.rs` 完整纳入 `amber:kv`、`amber:tools`、`amber:sandbox` 全量 API 与类型声明。
   - 官方文档站提供 3 个全新深度技术专栏，并全量支持英语、中文、西班牙语、法语、印地语 5 种语言导航。

---

## 模块新特性深度解析

### 1. 持久化 KV 与可靠状态引擎 (`amber:kv`)

```typescript
import { open, openInMemory } from 'amber:kv';

// 1. 打开持久化 KV 存储（含 WAL 磁盘日志）
const db = open({ path: './data/agent_state.wal' });

// 2. 基础写入与 TTL 过期设置
db.set('user:session:1001', { name: 'Alice', role: 'admin' }, 3600_000); // 1小时有效
console.log('User Role:', db.get('user:session:1001')?.role);

// 3. 原子计数器操作
const visitCount = db.incr('stats:visits', 1);

// 4. 前缀扫描（用于检索所有会话状态）
const sessions = db.scan({ prefix: 'user:session:', limit: 50 });
console.log(`Active sessions: ${sessions.length}`);

// 5. 原子事务批处理
db.batch([
  { type: 'set', key: 'tx:1', value: { status: 'committed' } },
  { type: 'delete', key: 'tx:staging' },
]);

// 6. 存储紧缩
db.compact();
```

---

### 2. 工具自动合成与 OpenAPI 编译器 (`amber:tools`)

```typescript
import { compileSchemaTool, fromOpenAPI, parseToolCalls } from 'amber:tools';
import { AgentPipeline } from 'amber:ai';

// 1. 从 JSON Schema 编译校验型工具
const calcTool = compileSchemaTool({
  name: 'calculator',
  description: '执行算术运算',
  parameters: {
    type: 'object',
    required: ['expression'],
    properties: {
      expression: { type: 'string' },
    },
  },
  execute: async ({ expression }) => ({ result: eval(expression) }),
});

// 2. 从 OpenAPI 3.x 规范自动合成 API 工具集
const openapiSpec = {
  openapi: '3.0.0',
  info: { title: 'Weather API', version: '1.0' },
  paths: {
    '/weather/{city}': {
      get: {
        operationId: 'getWeather',
        summary: '查询指定城市天气',
        parameters: [{ name: 'city', in: 'path', required: true, schema: { type: 'string' } }],
      },
    },
  },
};
const weatherTools = fromOpenAPI(openapiSpec, { baseUrl: 'https://api.weather.local' });

// 3. 解析大模型返回的工具调用文本
const llmResponse = '```json\n{"tool": "getWeather", "arguments": {"city": "Tokyo"}}\n```';
const calls = parseToolCalls(llmResponse);

// 4. 注册到 AgentPipeline
const pipeline = new AgentPipeline();
pipeline.registerTools([...weatherTools, calcTool]);
```

---

### 3. 加固微飞地沙箱与合规审计日志 (`amber:sandbox`)

```typescript
import { createEnclave, startAuditLog, stopAuditLog } from 'amber:sandbox';

// 1. 启动审计日志流
startAuditLog('./audit.jsonl');

// 2. 创建受限微飞地执行未信任代码
const enclave = createEnclave({
  allowRead: ['./data/public'],
  allowNet: ['api.github.com'],
  timeoutMs: 1000,
  maxMemoryMb: 64,
});

const result = enclave.run(`
  const a = 10;
  const b = 20;
  a + b;
`);
console.log('Enclave Output:', result); // 30

// 3. 停止审计日志收集
stopAuditLog();
```

通过 CLI 开启全程合规审计追踪：

```bash
$ amber run --audit-log /var/log/amber-audit.jsonl agent.ts
```

---

## 升级与兼容性说明 (Upgrade & Compatibility)

- **完全向后兼容**：`v1.7.0` 保持与 `v1.6.0`、`v1.5.0` 所有已发布内置模块（`amber:replay`、`amber:weights`、`amber:security`、`amber:wasm` 等）的完全向后二进制与 API 兼容。
- **模块 Specifier 说明**：新模块可通过 `amber:kv`、`amber:tools`、`amber:sandbox`，或短名 `kv`、`tools`、`sandbox` 引用。同时可在 `amber:ai.tools` 和 `amber:ai.kv` 下直接使用。

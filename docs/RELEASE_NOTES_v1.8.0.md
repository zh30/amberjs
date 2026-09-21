# Amber v1.8.0 Release Notes: Multi-Agent Message Bus & PubSub Channel Fabric, Streaming Structured JSON & LLM Token Grammar Engine, and Agent State Checkpoint & Time-Travel Snapshotting

> **Release Tag**: `v1.8.0`  
> **Release Type**: Minor Release (次版本 / 中版本)  
> **Target Commits**: Multi-Agent Message Bus & PubSub channel fabric (`amber:bus` / `amber:events`), Streaming structured JSON & token grammar engine (`amber:grammar` / `amber:streaming`), Agent state checkpoint & time-travel snapshotting (`amber:checkpoint` / `amber:state.checkpoint`), TypeScript definitions (`types/amberjs.d.ts`), and 5-language documentation across English, Chinese, Spanish, French, and Hindi.

---

## 概述 (Overview)

Amber **v1.8.0** 是围绕**复杂自治多 Agent 协同网络、实时流式结构化输出与容错推倒回退机制**打造的核心里程碑版本。本版本带来了三大原生企业级核心子系统及全球化开发者生态支持：

1. **多 Agent 消息总线与 PubSub 编排网格 (`amber:bus` / `amber:events` / `amber:ai.bus`)**：
   - **分层主题与通配符路由**：支持精准匹配、单段通配符（`*`）与多段通配符（`#`），如 `agent.*.task` 与 `audit.#`。
   - **异步请求-响应 (Request-Reply RPC 模式)**：内置基于临时关联主题与相关性 ID 的异步等待机制，支持超时熔断配置。
   - **高优先级队列与死信队列 (DLQ)**：支持消息优先级排序分发，以及未路由消息的自动收集与离线自检（`getDeadLetters()`）。
   - **链路拦截中间件**：支持挂载自定义 Trace/Auth 中间件（`bus.use(...)`）。

2. **流式结构化 JSON 修复与 LLM Token 语法引擎 (`amber:grammar` / `amber:ai.grammar` / `amber:streaming`)**：
   - **流式未闭合 JSON 微秒级实时自愈 (`parsePartialJSON`)**：自动识别并实时闭合大模型输出过程中截断的字符串、未封闭数组 `[`、未封闭对象 `{`、悬挂逗号与冒号，实时返回最新有效 JS 对象。
   - **增量流式解码器 (`createStreamDecoder`)**：提供流式数据累积与实时解析快照推送机制。
   - **Server-Sent Events (SSE) 事件流转换器 (`parseSSEChunk`)**：支持从 OpenAI / Anthropic / 本地 SLM 响应中实时解析事件头与 JSON 载荷。
   - **约束生成 Token 语法构建器 (`createGrammar`)**：支持枚举选项语法（`createChoiceGrammar`）、正则表达式语法（`createRegexGrammar`）与结构校验语法（`createJSONGrammar`）。

3. **Agent 状态检查点与时间旅行回退引擎 (`amber:checkpoint` / `amber:state.checkpoint` / `amber:ai.checkpoint`)**：
   - **轻量级不可变快照 (`save`)**：毫秒级生成中间推理状态的不可变深拷贝快照，并自动关联父节点血缘树。
   - **精准时间旅行回滚 (`restore`)**：当 Agent 遭遇工具异常或网络超时时，可一键将状态精准回滚至任意历史检查点。
   - **深层结构差异对比 (`diff`)**：自动计算两个历史检查点之间的新增、修改与删除字段结构差。
   - **推测推理分支探索 (Tree-of-Thought / `fork`)**：从任意检查点派生出独立的探索分支，支持多路径投机搜索而不污染主分支。
   - **与 `amber:kv` 持久化集成 (`persist` / `restoreFromKV`)**：原生无缝批量刷盘至磁盘 WAL 事务日志。

4. **统一类型系统与全球化 5 语种技术文档**：
   - `types/amberjs.d.ts` 与 `src/types_export.rs` 完整纳入 `amber:bus`、`amber:grammar`、`amber:checkpoint` 全量 API 与类型声明。
   - 官方文档站提供 3 个全新深度技术专栏，全量覆盖 English、中文、Español、Français、हिन्दी 5 种语言导航。

---

## 模块新特性代码示例

### 1. 多 Agent 消息总线 (`amber:bus`)

```typescript
import { subscribe, publish, request, reply } from 'amber:bus';

// 1. 服务方响应 RPC 请求
subscribe('service.summarizer.run', (msg) => {
  const { doc } = msg.payload;
  reply(msg, { summary: `Summary of ${doc}` });
});

// 2. 调用方发起异步等待
async function main() {
  const res = await request('service.summarizer.run', { doc: 'Q3_Report.pdf' }, { timeoutMs: 5000 });
  console.log(res.summary);
}
```

---

### 2. 流式结构化 JSON 实时自愈 (`amber:grammar`)

```typescript
import { parsePartialJSON, createStreamDecoder } from 'amber:grammar';

// 截断的大模型输出片段
const chunk = '{"status": "analyzing", "metrics": [98.5, 99.1, ';

// 自动补全为有效对象：{ status: "analyzing", metrics: [98.5, 99.1] }
const parsed = parsePartialJSON(chunk);
console.log(parsed.metrics.length); // 2
```

---

### 3. Agent 状态检查点与回退 (`amber:checkpoint`)

```typescript
import { createCheckpointManager } from 'amber:checkpoint';

const mgr = createCheckpointManager();

// 保存检查点
mgr.save('step_1', { step: 1, memory: ['task_initialized'] });
mgr.save('step_2', { step: 2, memory: ['task_initialized', 'tool_fetched'] });

// 步骤 3 失败，时间旅行回滚到 step_2
const recovered = mgr.restore('step_2');
console.log('Recovered step:', recovered.step); // 2
```

---

## 升级与兼容性说明 (Upgrade & Compatibility)

- **完全向后兼容**：`v1.8.0` 保持与 `v1.7.0`、`v1.6.0`、`v1.5.0` 所有已发布内置模块（`amber:kv`、`amber:tools`、`amber:sandbox`、`amber:replay`、`amber:weights`、`amber:security`、`amber:wasm` 等）的完全向后兼容。
- **模块 Specifier 说明**：新增模块均可通过 `amber:bus`、`amber:grammar`、`amber:checkpoint`，或对应短名 `bus`、`grammar`、`checkpoint` 引用。同时可在 `amber:ai.bus`、`amber:ai.grammar`、`amber:ai.checkpoint` 下直接使用。

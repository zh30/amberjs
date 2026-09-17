---
title: "流式结构化输出与 Token 语法引擎 (bee:grammar)"
subtitle: "流式未闭合 JSON 实时自愈解析、SSE 事件流转换与约束生成语法"
group: "Agent 与高级特性"
id: "streaming-grammar"
---

在与大语言模型 (LLM) 和端侧轻量小模型 (SLM) 交互时，模型输出通常以 Token 流（如 Server-Sent Events / SSE）的形式逐字传输。传统的 `JSON.parse` 在遇到不完整的片段时会直接抛出 `SyntaxError`，导致应用必须完整等待整段文本生成完毕后，才能开始解析结构化数据、渲染前端 UI 或触发下游工具调用。

**Beejs v1.8.0 正式引入原生流式结构化输出与 Token 语法引擎 (`bee:grammar`)**。该模块提供流式未闭合 JSON 实时自愈补全解析、增量流式解码器、SSE 数据块快速解析以及约束生成 Token 语法校验。

---

## 1. 流式不完整 JSON 实时自愈解析 (`parsePartialJSON`)

`parsePartialJSON` 函数能够在微秒级时间内自动识别未闭合的双引号、截断的数组 `[`、未闭合的对象 `{`、悬挂逗号以及末尾字段冒号，智能将其修复为合法的 JSON 并返回当前的最新 JavaScript 对象：

```typescript
import { parsePartialJSON } from 'bee:grammar';

// 大模型流式输出过程中的不完整中间片段
const incompleteChunk = '{"status": "running", "tags": ["agent", "work';

// 自动补全闭合引号与数组闭合符：{"status": "running", "tags": ["agent", "work"]}
const parsed = parsePartialJSON(incompleteChunk);

console.log(parsed.status); // "running"
console.log(parsed.tags);   // ["agent", "work"]
```

---

## 2. 增量流式解码器 (`createStreamDecoder`)

`createStreamDecoder` 维护增量接收缓冲区，并在每次数据块到达时即时触发解析快照回调：

```typescript
import { createStreamDecoder } from 'bee:grammar';

const decoder = createStreamDecoder({
  onChunk: (snapshot, isComplete) => {
    console.log(`实时最新快照 (完成状态: ${isComplete}):`, snapshot);
  }
});

decoder.push('{"plan": ["收集需求", ');
decoder.push('"编写代码", ');
decoder.push('"执行测试"]}');
decoder.finish();
```

---

## 3. Server-Sent Events (SSE) 流解析 (`parseSSEChunk`)

原生解析来自 OpenAI、Anthropic、Ollama 或本地 SLM 的 SSE 流数据块：

```typescript
import { parseSSEChunk } from 'bee:grammar';

const rawSSE = `
event: delta
id: 1
data: {"token": "你好"}

event: delta
id: 2
data: {"token": "世界！"}
`;

const events = parseSSEChunk(rawSSE);
for (const ev of events) {
  console.log(ev.event, ev.json().token);
}
```

---

## 4. 约束生成 Token 语法 (`createGrammar`)

用于限制模型输出仅符合指定的枚举选项、正则表达式或 JSON Schema：

```typescript
import { createChoiceGrammar, createRegexGrammar } from 'bee:grammar';

// 1. 枚举选项语法
const decisionGrammar = createChoiceGrammar(['ACCEPT', 'REJECT', 'NEED_MORE_INFO']);
console.log(decisionGrammar.accept('', 'ACC')); // true
console.log(decisionGrammar.accept('ACC', 'EPT')); // true
console.log(decisionGrammar.accept('ACC', 'XYZ')); // false

// 2. 正则约束语法
const ticketGrammar = createRegexGrammar(/^BUG-\d{4}$/);
console.log(ticketGrammar.validate('BUG-1024').valid); // true
```

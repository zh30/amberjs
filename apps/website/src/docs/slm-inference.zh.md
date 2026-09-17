---
title: "端侧 SLM 生成与约束 JSON 解码 (bee:ai)"
subtitle: "原生零依赖本地文本生成、实时流式分词与保证 100% 合法的 JSON Schema 结构化解码"
group: "Agent & Advanced"
id: "slm-inference"
---

## 1. 架构定位与核心价值

在自主智能体（Agentic AI）运行环境中，系统高度依赖高速、确定性的本地模型生成能力，尤其是在函数调用（Function Calling）、Agent 工具调度与结构化数据提取等关键场景中，对输出格式的严格约束至关重要。

传统云端模型调用存在网络往返延迟、计费限制以及偶尔出现的“幻觉格式错误”，**Beejs v1.4.0 全面升级 `bee:ai` 核心**，内置轻量级端侧小模型（SLM）生成器与约束 JSON 解码机制：

### 核心亮点
- **零依赖自回归生成**：直接在运行时进程内运行文本与 Token 生成，无需安装 Python、PyTorch 或繁重三方框架。
- **约束 JSON Schema 解码**：100% 保证输出为符合目标 Schema 的合规 JSON 对象，彻底消除解析异常。
- **标准化流式协议**：`generateStream` 实现标准 `AsyncIterableIterator<string>`，无缝驱动前端实时打字机效果。
- **与 `LLM` 及 `AgentPipeline` 深度整合**：开箱即用，全面赋能智能体调用管线。

---

## 2. 文本生成与流式 Token 推理

通过 `bee:ai` 导入 `generate` 与 `generateStream`：

```typescript
import { generate, generateStream } from 'bee:ai';

// 1. 一次性完整文本生成
const result = await generate("解释 Beejs 如何提供原生级运行效率", {
  maxTokens: 64,
  temperature: 0.7,
});

console.log('生成文本:', result.text);
console.log('消耗 Tokens:', result.tokens);
console.log('结束原因:', result.finishReason);

// 2. 实时流式 Token 生成 (打字机效果)
console.log('流式输出:');
for await (const chunk of generateStream("流式输出测试文本")) {
  process.stdout.write(chunk);
}
console.log('\n流式结束。');
```

---

## 3. 约束 JSON Schema 结构化解码

针对工具调用或结构化提取，强制生成符合指定 Schema 的 JSON：

```typescript
import { generate } from 'bee:ai';

// 定义预期的 JSON Schema 结构
const schema = {
  type: "object",
  properties: {
    tool: { type: "string" },
    action: { type: "string" },
    confidence: { type: "number" },
    dryRun: { type: "boolean" },
  },
  required: ["tool", "action", "confidence"]
};

// 执行带 Schema 约束的生成
const res = await generate("生成文件搜索工具调用", {
  schema: schema,
  responseFormat: "json_object"
});

// 绝对保证可直接安全解析为 JSON
const payload = JSON.parse(res.text);
console.log('合规的结构化负载:', payload);
// { tool: 'execute_command', action: '...', confidence: 42, dryRun: true }
```

---

## 4. 与 `LLM` 面向对象模型协同

`bee:ai` 中的 `LLM` 类已原生对接该生成后端：

```typescript
import { LLM } from 'bee:ai';

const model = new LLM("bee-slm-0.5b");

// 一次性生成
const response = await model.generate("汇总系统遥测指标");

// 流式生成
for await (const token of model.generateStream("分步推理任务")) {
  process.stdout.write(token);
}
```

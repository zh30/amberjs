---
title: "Agent 工具自动合成与 OpenAPI 编译器 (bee:tools)"
subtitle: "动态 JSON Schema 校验、零模版 OpenAPI 3.x 工具合成与 AgentPipeline 直连"
group: "Agent & Advanced"
id: "tool-synthesis"
---

在自主 Agent 架构中，工具（Tool）是 Agent 连接外部物理世界的桥梁。以往手动编写参数验证、解析 HTTP 请求与处理 JSON 结构往往需要耗费大量冗余样板代码。并且，现代 Agent 普遍需要在运行时通过读取 OpenAPI / Swagger 规范动态学习并接入新 API。

**Beejs v1.7.0 原生推出 Agent 工具自动合成与 Schema 编译器（`bee:tools` / `bee:ai.tools`）**。支持直接将 JSON Schema 编译为强校验工具、一键将 OpenAPI 3.x 接口文档转化为基于原生 `fetch` 的可执行 Agent 工具、自适应解析 LLM 结构化调用，并与 `bee:ai.AgentPipeline` 深度互通。

---

## 1. Schema 工具编译器 (`compileSchemaTool`)

声明式定义工具，自动具备类型强校验、默认值注入与 OpenAI 格式标准序列化能力：

```typescript
import { compileSchemaTool } from 'bee:tools';

const searchTool = compileSchemaTool({
  name: 'web_search',
  description: '按关键词检索文档知识库',
  parameters: {
    type: 'object',
    properties: {
      query: { type: 'string', description: '搜索关键词' },
      limit: { type: 'integer', default: 5, description: '最大返回结果数' },
      safeSearch: { type: 'boolean', default: true }
    },
    required: ['query']
  },
  execute: async (args) => {
    // 若调用方未传入 limit，将自动填充为默认值 5！
    return { results: [`关于 ${args.query} 的检索文章`], count: args.limit };
  }
});

// 自动验证与默认值注入
const out = await searchTool.execute({ query: 'Rust V8' });
console.log(out); // { results: ['关于 Rust V8 的检索文章'], count: 5 }

// 导出标准的 Function Calling JSON Schema，供各类大模型使用
console.log(JSON.stringify(searchTool.toJSON(), null, 2));
```

若 Agent 传入缺少必要字段或非法类型的参数，工具将立即抛出清晰的错误阻断风险：
```text
Error: Tool 'web_search' argument validation failed: Missing required property: 'query'
```

---

## 2. OpenAPI 3.x 工具零代码合成 (`fromOpenAPI`)

读取任意 OpenAPI / Swagger 3.0 或 3.1 规范文档，毫秒级合成全套开箱即用的 Agent 工具：

```typescript
import { fromOpenAPI } from 'bee:tools';

const openApiSpec = {
  openapi: '3.0.0',
  paths: {
    '/users/{id}': {
      get: {
        operationId: 'getUser',
        summary: '获取用户信息',
        parameters: [
          { name: 'id', in: 'path', required: true, schema: { type: 'string' } }
        ]
      }
    },
    '/messages': {
      post: {
        operationId: 'sendMessage',
        summary: '发送聊天消息',
        requestBody: {
          content: {
            'application/json': {
              schema: {
                type: 'object',
                properties: { text: { type: 'string' } },
                required: ['text']
              }
            }
          }
        }
      }
    }
  }
};

// 一键自动合成，配置公共 baseUrl 与鉴权 Header
const tools = fromOpenAPI(openApiSpec, {
  baseUrl: 'https://api.myapp.com',
  headers: { 'Authorization': 'Bearer secret-key' }
});

console.log(`成功生成 ${tools.length} 个 Agent 可调用工具。`);
// 直接执行工具：内部自动替换路径变量、拼接查询参数并调用 fetch 发送请求！
const user = await tools.map.getUser.execute({ id: 'u_123' });
```

---

## 3. LLM 工具调用解析与流水线装配

大语言模型输出工具调用时常混杂 Markdown 代码块、标签或特定 JSON 包装。`parseToolCalls` 提供自适应标准化解析：

```typescript
import { parseToolCalls, registerTools } from 'bee:tools';
import ai from 'bee:ai';

const rawOutput = `
为您查询天气：
\`\`\`json
{
  "name": "get_weather",
  "arguments": { "city": "北京" }
}
\`\`\`
`;

const calls = parseToolCalls(rawOutput);
console.log(calls[0]); // { name: 'get_weather', arguments: { city: '北京' } }

// 将合成或编译好的工具批量注入 AgentPipeline 调度管道
const pipeline = new ai.AgentPipeline();
registerTools(pipeline, tools);
```

亦可通过 `ai.tools` 便捷访问：
```typescript
import ai from 'bee:ai';
const tool = ai.tools.compileSchemaTool({ ... });
```

---

## 4. API 完整参考

| 函数 / 方法 | 参数说明 | 返回类型 | 功能解析 |
| :--- | :--- | :--- | :--- |
| `compileSchemaTool(def)` | `ToolDefinition` | `AgentTool` | 编译 JSON Schema 工具并挂载强类型入参校验 |
| `fromOpenAPI(spec, opts?)`| `OpenAPISpec, { baseUrl?, headers? }` | `AgentTool[] & { map }` | 解析 OpenAPI 文档并批量合成 Agent 可执行工具 |
| `parseToolCalls(output)` | `string \| object` | `ToolCall[]` | 容错提取模型文本输出中的结构化函数调用 |
| `registerTools(pipeline, tools)` | `AgentPipeline, Tool[]` | `AgentPipeline` | 将工具集批量注册到指定的 Agent 调度管道中 |

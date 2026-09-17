---
title: "Agent Tool Auto-Synthesis & OpenAPI Compiler (bee:tools)"
subtitle: "Dynamic JSON Schema validation, zero-boilerplate OpenAPI 3.x tool synthesis, and AgentPipeline integration"
group: "Agent & Advanced"
id: "tool-synthesis"
---

Autonomous Agents interact with the real world through tools. Manually writing boilerplate validation, parsing REST endpoints, and coercing JSON arguments is tedious and error-prone. Furthermore, Agents increasingly discover APIs at runtime by reading Swagger / OpenAPI 3.x specifications.

**Beejs v1.7.0 introduces the Agent Tool Auto-Synthesis & OpenAPI Schema Compiler (`bee:tools` / `bee:ai.tools`)**. It compiles JSON Schema definitions into type-validated tools, transforms OpenAPI specifications into ready-to-execute tools backed by native `fetch`, parses structured LLM tool calls from free text or markdown, and connects directly to `bee:ai.AgentPipeline`.

---

## 1. Schema-to-Tool Compiler (`compileSchemaTool`)

Create tools with parameter validation, type coercion, and OpenAI-compatible schema serialization:

```typescript
import { compileSchemaTool } from 'bee:tools';

const searchTool = compileSchemaTool({
  name: 'web_search',
  description: 'Searches documentation for relevant articles',
  parameters: {
    type: 'object',
    properties: {
      query: { type: 'string', description: 'Search keywords' },
      limit: { type: 'integer', default: 5, description: 'Result count' },
      safeSearch: { type: 'boolean', default: true }
    },
    required: ['query']
  },
  execute: async (args) => {
    // args.limit will automatically default to 5 if omitted!
    return { results: [`Article for ${args.query}`], count: args.limit };
  }
});

// Automatic validation & default filling
const out = await searchTool.execute({ query: 'Rust V8' });
console.log(out); // { results: ['Article for Rust V8'], count: 5 }

// Export function calling JSON schema for LLMs
console.log(JSON.stringify(searchTool.toJSON(), null, 2));
```

If an agent passes missing required properties or illegal types, `searchTool.execute()` throws an immediate, informative error:
```text
Error: Tool 'web_search' argument validation failed: Missing required property: 'query'
```

---

## 2. OpenAPI 3.x Tool Auto-Synthesis (`fromOpenAPI`)

Ingest any OpenAPI or Swagger 3.0/3.1 document and synthesize native tools instantly:

```typescript
import { fromOpenAPI } from 'bee:tools';

const openApiSpec = {
  openapi: '3.0.0',
  paths: {
    '/users/{id}': {
      get: {
        operationId: 'getUser',
        summary: 'Fetch user profile',
        parameters: [
          { name: 'id', in: 'path', required: true, schema: { type: 'string' } }
        ]
      }
    },
    '/messages': {
      post: {
        operationId: 'sendMessage',
        summary: 'Post chat message',
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

// Synthesize tools with base URL and auth headers
const tools = fromOpenAPI(openApiSpec, {
  baseUrl: 'https://api.myapp.com',
  headers: { 'Authorization': 'Bearer secret-key' }
});

console.log(`Generated ${tools.length} executable tools.`);
// Execute tool directly: it performs HTTP fetch automatically!
const user = await tools.map.getUser.execute({ id: 'u_123' });
```

---

## 3. LLM Tool Call Parsing & Pipeline Wiring

LLMs output tool calls in various formats: raw JSON, markdown fences, or OpenAI blocks. `parseToolCalls` normalizes them all:

```typescript
import { parseToolCalls, registerTools } from 'bee:tools';
import ai from 'bee:ai';

// Extract tool calls from raw LLM text
const rawOutput = `
I will search for the user request:
\`\`\`json
{
  "name": "web_search",
  "arguments": { "query": "Beejs v1.7.0 release" }
}
\`\`\`
`;

const calls = parseToolCalls(rawOutput);
console.log(calls[0]); // { name: 'web_search', arguments: { query: 'Beejs v1.7.0 release' } }

// Wire tools directly into an AgentPipeline
const pipeline = new ai.AgentPipeline();
registerTools(pipeline, tools);
```

You can also access the tools module through `ai.tools`:
```typescript
import ai from 'bee:ai';
const tool = ai.tools.compileSchemaTool({ ... });
```

---

## 4. API Reference

| Function / Method | Parameters | Return Type | Description |
| :--- | :--- | :--- | :--- |
| `compileSchemaTool(def)` | `ToolDefinition` | `AgentTool` | Compiles JSON Schema tool with parameter validation |
| `fromOpenAPI(spec, opts?)`| `OpenAPISpec, { baseUrl?, headers? }` | `AgentTool[] & { map }` | Synthesizes executable tools from OpenAPI spec |
| `parseToolCalls(output)` | `string \| object` | `ToolCall[]` | Robustly extracts tool calls from LLM outputs |
| `registerTools(pipeline, tools)` | `AgentPipeline, Tool[]` | `AgentPipeline` | Registers tools into an AgentPipeline instance |

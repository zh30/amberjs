---
title: "Model Context Protocol 2.0 (bee:mcp)"
subtitle: "Production-ready native MCP standard implementation: build servers, declare tools, expose resources, and inspect with visual CLI"
group: "Agent & Advanced"
id: "mcp-protocol"
---

## 1. What is Model Context Protocol (MCP)?

**Model Context Protocol (MCP)**, open-sourced by Anthropic (specification version `2024-11-05`), has become the premier standard for large language model agents to:
1. **Call Tools**: Execute computational tasks, trigger actions, or interact with external systems;
2. **Access Resources**: Expose contextual documents, file content, or system telemetry;
3. **Template Prompts**: Provide standardized prompts and workflows to models.

Beejs v1.3.0 embeds **`bee:mcp` natively in the runtime core**. Without installing external node modules or build systems, developers can build MCP servers, run standard Stdio transports for Claude Desktop / Cursor, or connect locally in-memory via `connectLocal()`.

---

## 2. Server & Client API

Import from `bee:mcp`:

```typescript
import { McpServer, McpClient } from 'bee:mcp';
```

### `McpServer`

Register tools, resources, and prompt templates:

```typescript
const server = new McpServer({
  name: "dev-assistant",
  version: "1.3.0"
});

// 1. Tool Declaration
server.tool(
  "calculate_hash",
  "Compute cryptographic hash for input data",
  {
    type: "object",
    properties: {
      data: { type: "string", description: "Payload to hash" },
      algorithm: { type: "string", enum: ["sha256", "md5"], default: "sha256" }
    },
    required: ["data"]
  },
  async ({ data, algorithm = "sha256" }) => {
    const { crypto } = require('bee:std');
    return { hash: crypto.hash(algorithm, data) };
  }
);

// 2. Resource Declaration
server.resource(
  "system://metrics",
  "Realtime system statistics",
  () => ({
    timestamp: Date.now(),
    memoryUsage: process.memoryUsage()
  }),
  "application/json"
);

// 3. Prompt Declaration
server.prompt(
  "code_review",
  "Security and performance review prompt",
  [{ name: "code", description: "Code to review", required: true }],
  ({ code }) => ({
    prompt: `Review the following code for concurrency issues and safety:\n${code}`
  })
);
```

---

### In-Memory Testing with `connectLocal()`

Connect a client directly to the server without spawning child processes or configuring network ports:

```typescript
const client = server.connectLocal();

// Ping
console.log("Ping:", await client.ping()); // true

// List & invoke tools
const tools = await client.listTools();
const toolRes = await client.callTool("calculate_hash", { data: "Beejs", algorithm: "sha256" });
console.log("Tool result:", toolRes);

// Read resource
const res = await client.readResource("system://metrics");
const metrics = JSON.parse(res[0].text);
console.log("Resource data:", metrics);
```

---

### Production Stdio Transport

Run the server over standard input/output for Claude Desktop or Cursor:

```typescript
server.startStdio();
```

---

## 3. CLI Inspection (`bee mcp --inspect`)

Inspect MCP tool schemas with formatted tables directly from the command line:

```bash
# Inspect built-in runtime tools
bee mcp --inspect

# Inspect a user module's exposed tools
bee mcp --inspect ./my_tools.ts
```

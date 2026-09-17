---
title: "Model Context Protocol 2.0 (bee:mcp)"
subtitle: "业界领先的原生 MCP 标准实现：在运行时直接构建服务、连接工具、发布资源并支持可视化 CLI 检查"
group: "Agent 与高级特性"
id: "mcp-protocol"
---

## 1. 什么是 Model Context Protocol (MCP)？

**Model Context Protocol (MCP)** 是由 Anthropic 发起并迅速成为大模型 Agent 生态事实标准的开放协议（规范版本：`2024-11-05`）。它为 AI 模型提供了标准化的方式来：
1. **调用工具 (Tools)**：执行计算、检索外部系统或操作环境；
2. **读取资源 (Resources)**：获取动态上下文、项目元数据与结构化数据；
3. **加载提示词模板 (Prompts)**：标准化系统提示词交互工作流。

在传统架构中，运行 MCP 服务器通常需要安装专门的 Node.js 库与外部转译工具。**Beejs v1.3.0 在运行时核心内置了官方标准的 `bee:mcp` 原生引擎**，不仅支持标准的跨进程 Stdio 协议与 Claude Desktop / Cursor 直连，还首创支持进程内 `connectLocal()` 零拷贝互通与 CLI 可视化检查！

---

## 2. 核心架构与 API

引入官方模块：

```typescript
import { McpServer, McpClient } from 'bee:mcp';
```

---

### 一、创建 MCP 服务端 (`McpServer`)

使用链式语法注册工具、资源与提示词：

```typescript
const server = new McpServer({
  name: "bee-ops-assistant",
  version: "1.3.0"
});

// 1. 注册 Tool (工具)
server.tool(
  "calculate_hash",
  "为输入字符串计算加密安全散列哈希值",
  {
    type: "object",
    properties: {
      data: { type: "string", description: "待哈希文本" },
      algorithm: { type: "string", enum: ["sha256", "md5"], default: "sha256" }
    },
    required: ["data"]
  },
  async ({ data, algorithm = "sha256" }) => {
    const { crypto } = require('bee:std');
    return { hash: crypto.hash(algorithm, data) };
  }
);

// 2. 注册 Resource (动态资源)
server.resource(
  "system://metrics",
  "System Realtime Metrics",
  () => {
    return {
      cpuUsage: "12%",
      memory: process.memoryUsage(),
      timestamp: Date.now()
    };
  },
  "application/json"
);

// 3. 注册 Prompt (工作流提示词模板)
server.prompt(
  "code_audit",
  "代码安全性与性能审计模板",
  [{ name: "code", description: "待审计的源代码", required: true }],
  ({ code }) => {
    return {
      prompt: `你是一名资深系统架构师。请从内存安全、并发竞态与边界检查三个维度对以下代码进行深度审计：\n\`\`\`\n${code}\n\`\`\``
    };
  }
);
```

---

### 二、本地内存互通 (`connectLocal`)

无需创建操作系统子进程或绑定套接字，直接获取轻量级客户端实例进行本地单元测试与 Agent 编排：

```typescript
const client = server.connectLocal();

// 1. 心跳检测
const alive = await client.ping();
console.log("MCP 服务是否可用:", alive); // true

// 2. 列出所有工具
const tools = await client.listTools();
console.log("注册的工具列表:", tools.map(t => t.name));

// 3. 调用指定工具
const result = await client.callTool("calculate_hash", { data: "Hello Beejs", algorithm: "sha256" });
console.log("工具执行结果:", result);

// 4. 读取指定资源
const resContents = await client.readResource("system://metrics");
const metrics = JSON.parse(resContents[0].text);
console.log("系统实时指标:", metrics);

// 5. 渲染提示词
const promptData = await client.getPrompt("code_audit", { code: "let x = 1;" });
console.log("渲染后的系统 Prompt:", promptData.prompt);
```

---

### 三、启动 Stdio 生产服务

一行代码将当前脚本作为标准 MCP 服务器运行，直接接入 **Claude Desktop**、**Cursor** 或自治 Agent 编排器：

```typescript
// 启动标准输入输出 JSON-RPC 消息循环
server.startStdio();
```

---

## 3. CLI 命令行检查与可视化 (`bee mcp --inspect`)

Beejs 提供了开箱即用的 MCP 命令行调试套件：

```bash
# 1. 快速检查运行时内置工具
bee mcp --inspect

# 2. 检查自定义 MCP 脚本暴露的全部工具与参数模式
bee mcp --inspect ./my_tools.ts
```

输出示例：

```text
🔍 Inspecting MCP Built-in Tools

📡 Protocol Version: 2024-11-05
🛠  Discovered 3 tool(s):

┌─────────────────┬───────────────────────────────────────────────────┬─────────────────────────────────────────────────┐
│ Tool Name       │ Description                                       │ Input Schema                                    │
├─────────────────┼───────────────────────────────────────────────────┼─────────────────────────────────────────────────┤
│ execute_command │ Execute a sandboxed shell command                 │ {"type":"object","properties":{"cmd":...}}      │
│ read_file       │ Read file contents from filesystem                │ {"type":"object","properties":{"path":...}}     │
│ write_file      │ Write file contents to virtual or host filesystem │ {"type":"object","properties":{"path":...}}     │
└─────────────────┴───────────────────────────────────────────────────┴─────────────────────────────────────────────────┘
```

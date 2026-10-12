---
title: "工具、会话与 MCP"
nav: "工具与 MCP"
subtitle: "从文件导出函数，再通过 JSON-RPC 会话或 MCP stdio 服务调用它们。"
status: "stable"
id: "mcp-protocol"
---

`amber run --export-tools`（G4）、`amber session` 与 `amber mcp` 均为 **Stable**。模型不在 Amber 内部：这些命令只负责加载工具文件、调用其导出函数，所用运行时和权限代理与 `amber run` 相同。

## 工具文件

```ts
// tools.ts

/** Add two numbers. */
export function add({ a, b }: { a: number; b: number }) {
  return a + b;
}

/** Read a UTF-8 file. */
export function readText({ path }: { path: string }) {
  return require("node:fs").readFileSync(path, "utf8");
}
```

## 导出 schema

```bash
amber run --export-tools tools.ts
```

```json
{"tools":[{"name":"add","description":"Add two numbers.","inputSchema":{"type":"object"}}, ...]}
```

该命令输出 JSON 后退出，不会执行文件，也不会调用模型。契约见 [`EXPORT_TOOLS_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/EXPORT_TOOLS_CONTRACT.md)。

限制：

- 只识别以 `export function` 或 `export async function` 开头的行；`export const`、箭头函数和 `export default` 不会列出。
- 描述取自相邻的 `//` 或 `/** */` 注释，`@` 标签会被去掉。
- `inputSchema` 固定为 `{"type":"object"}`，TypeScript 类型和 `@param` 不会转换为 JSON Schema。
- 同目录下若有 `tools.json`，则完全替代源码扫描。

## JSON-RPC 会话

```bash
amber session --sandbox --allow-read ./data tools.ts
```

`amber session` 从 stdin 每行读取一个 JSON-RPC 请求，并向 stdout 每行写出一个响应。

```json
{"jsonrpc":"2.0","id":1,"method":"tools/list"}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"add","arguments":{"a":2,"b":3}}}
```

它还会响应 `initialize`、`ping`，以及返回空列表的 `prompts/list` 和 `resources/list`。工具出错返回错误码 `-32000`，未知方法返回 `-32601`。

## MCP stdio 服务

```bash
amber mcp tools.ts
amber mcp --inspect tools.ts   # 列出工具后退出
```

`amber mcp` 实现 MCP 协议版本 `2024-11-05`，支持 `initialize`、`tools/list` 与 `tools/call`。工具结果以单个 text 内容项返回，失败时设置 `isError`。

双向消息都使用 `Content-Length` 头分帧，与 LSP 的方式相同。如果你的 MCP 宿主在 stdio 上只支持按行分隔的 JSON，接入前请先确认它能处理这种分帧。

给 `session` 或 `mcp` 加上 `--isolate-per-call`，每次 `tools/call` 都会使用全新的 isolate。

## 与沙箱组合

所有权限参数在这里同样可用。这是面向 Agent 宿主的推荐用法：

```bash
amber mcp --sandbox --allow-read ./workspace --audit-log audit.jsonl tools.ts
```

## `amber:mcp` 模块

运行时中还有一个 JavaScript `McpServer` API，模块名为 `amber:mcp`。它不在 Stable 范围内，请视为 Experimental。Stable 的是上面描述的 CLI。

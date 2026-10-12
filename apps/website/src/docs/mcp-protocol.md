---
title: "Tools, sessions, and MCP"
nav: "Tools & MCP"
subtitle: "Export functions from a file, then call them through a JSON-RPC session or an MCP stdio server."
status: "stable"
id: "mcp-protocol"
---

`amber run --export-tools` (G4), `amber session`, and `amber mcp` are **Stable**. Models stay outside Amber: these commands only load a tool file and call its exports under the same runtime and permission broker as `amber run`.

## A tool file

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

## Export schemas

```bash
amber run --export-tools tools.ts
```

```json
{"tools":[{"name":"add","description":"Add two numbers.","inputSchema":{"type":"object"}}, ...]}
```

The command prints JSON and exits. It does not run the file or call a model. Contract: [`EXPORT_TOOLS_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/EXPORT_TOOLS_CONTRACT.md).

Limits:

- Only lines that start with `export function` or `export async function` are listed. `export const`, arrow functions, and `export default` are not.
- The description comes from the nearby `//` or `/** */` comment, with `@` tags dropped.
- `inputSchema` is always `{"type":"object"}`. TypeScript types and `@param` tags are not turned into JSON Schema.
- A sibling `tools.json` replaces source scanning entirely.

## JSON-RPC session

```bash
amber session --sandbox --allow-read ./data tools.ts
```

`amber session` reads one JSON-RPC request per line on stdin and writes one response per line on stdout.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/list"}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"add","arguments":{"a":2,"b":3}}}
```

It also answers `initialize`, `ping`, and empty `prompts/list` and `resources/list`. Tool errors return code `-32000`; unknown methods return `-32601`.

## MCP stdio server

```bash
amber mcp tools.ts
amber mcp --inspect tools.ts   # list the tools and exit
```

`amber mcp` speaks MCP protocol version `2024-11-05` with `initialize`, `tools/list`, and `tools/call`. Tool results come back as one text content item, with `isError` set on failure.

Messages are framed with `Content-Length` headers in both directions, the way LSP frames messages. If your MCP host only speaks newline-delimited JSON over stdio, check that it can talk to this framing before you wire it up.

Pass `--isolate-per-call` to `session` or `mcp` to give every `tools/call` a fresh isolate.

## Combine with the sandbox

Every permission flag works here too. This is the intended pattern for agent hosts:

```bash
amber mcp --sandbox --allow-read ./workspace --audit-log audit.jsonl tools.ts
```

## The `amber:mcp` module

A JavaScript `McpServer` API is also present as `amber:mcp`. It is not in the Stable scope, so treat it as Experimental. The Stable surface is the CLI described above.

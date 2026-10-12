---
title: "CLI 参考"
nav: "CLI 参考"
subtitle: "v1.17.0 中所有 amber 子命令，按成熟度分组，已对照 src/main.rs 核实。"
status: "stable"
id: "cli-usage"
---

运行 `amber --help` 或 `amber <command> --help` 查看你的构建中完整的参数列表。

## Stable 命令

| 命令 | 作用 | 契约 |
| :--- | :--- | :--- |
| `amber run <file> [args...]` | 运行 JS 或 TS 文件 | — |
| `amber eval <code>` | 求值代码片段并打印结果 | — |
| `amber repl` | 交互式 REPL | — |
| `amber test [file]` | 发现并运行 Jest 风格测试 | [测试](/docs/testing) |
| `amber bundle <entry> -o <out>` | 把本地模块图打包成一个 JS 文件 | [`BUNDLE_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/BUNDLE_CONTRACT.md) |
| `amber compile <entry> -o <bin>` | 为当前主机构建单文件可执行程序 | [`COMPILE_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/COMPILE_CONTRACT.md) |
| `amber install [--frozen-lockfile]` | 从 `package.json` 安装直接依赖 | [`INSTALL_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/INSTALL_CONTRACT.md) |
| `amber serve [file]` | 通过 HTTP/1.1 提供 fetch 处理函数 | [`SERVE_HTTP_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/SERVE_HTTP_CONTRACT.md) |
| `amber serve --https --cert --key` | 同上，走 rustls TLS | [`SERVE_HTTPS_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/SERVE_HTTPS_CONTRACT.md) |
| `amber session <file>` | 基于 stdin/stdout 的 JSON-RPC 工具会话 | [工具与 MCP](/docs/mcp-protocol) |
| `amber mcp <file>` | 为工具文件启动 MCP stdio 服务 | [工具与 MCP](/docs/mcp-protocol) |
| `amber snapshot build\|status\|clean` | 管理 V8 启动快照 | — |
| `amber version` / `amber --version` | 打印版本 | — |

## Stable 的 `amber run` 参数

| 参数 | 作用 | 契约 |
| :--- | :--- | :--- |
| `--watch`、`-w` | 被监听的文件变化时重新运行入口 | [`WATCH_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/WATCH_CONTRACT.md) |
| `--inspect`、`--inspect-brk`、`--inspect-port` | 在 `127.0.0.1` 上提供一个小型 CDP 端点 | [`INSPECT_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/INSPECT_CONTRACT.md) |
| `--export-tools` | 不执行文件，打印工具 schema JSON 后退出 | [`EXPORT_TOOLS_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/EXPORT_TOOLS_CONTRACT.md) |

`--watch` 会在上一次运行返回后重启整个进程，不是热模块替换。

## 权限与确定性参数

这些参数适用于 `run`、`test`、`serve`、`session`、`mcp` 以及其他会执行代码的命令。

| 参数 | 作用 |
| :--- | :--- |
| `--sandbox` | 默认拒绝文件、网络、环境变量和子进程访问 |
| `--allow-read <path>` / `--allow-write <path>` | 放行一个精确路径（可重复） |
| `--allow-net <host>` / `--allow-listen <host>` | 放行对某主机或 URL 的出站连接或监听 |
| `--allow-env <NAME>` / `--allow-run <command>` | 放行一个环境变量或一个命令 |
| `--deny-fs`、`--deny-net`、`--deny-env`、`--deny-run` | 不开完整沙箱，只拒绝某一类能力 |
| `--permission-policy <file>` | 从 JSON 加载放行与拒绝规则 |
| `--audit-log <file>` | 把每次权限判定以 JSONL 追加写入 |
| `--seed <u64>` | 为 `Math.random`、`crypto.getRandomValues`、`crypto.randomBytes` 设定种子 |
| `--freeze-time <spec>` | 冻结 `Date.now`、`new Date()`、`performance.now` |
| `--timeout <ms>` / `--max-memory <MB>` | 终止失控脚本 |

示例见 [Agent 沙箱](/docs/agent-sandbox)。

## Experimental 命令

这些命令存在，但不在 Stable 范围内，可能不够完善。

| 命令 | 说明 |
| :--- | :--- |
| `amber debug <file>` | 较早的调试入口，请改用 `amber run --inspect` |
| `amber init`、`amber create` | 项目脚手架 |
| `amber add`、`amber remove`、`amber prune`、`amber upgrade` | `amber install` 契约之外的依赖编辑 |
| `amber x <pkg>`（别名 `dlx`、`bunx`） | 不安装直接运行包里的可执行文件 |
| `amber test --parallel` | 以退出码 2 拒绝。V8 isolate 不跨线程共享。 |

## 存在但尚未分级

二进制里还包含以下命令。`CURRENT_SCOPE.md` 尚未对它们分级，因此本手册按 Experimental 对待：`amber fmt`、`amber lint`、`amber bench`、`amber task`、`amber types`、`amber lsp`、`amber profile`、`amber deploy`、`amber record`、`amber replay`，以及 `--import-map` 和 `--virtual-fs` 参数。详见 [Experimental](/docs/experimental)。

## 退出码

契约内的失败会打印 `error: amber serve:`、`error: amber install:` 之类的前缀并以非零码退出。参数误用（例如 `amber test --parallel`）以退出码 2 退出。

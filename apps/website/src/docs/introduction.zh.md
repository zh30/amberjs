---
title: "简介"
nav: "简介"
subtitle: "Amber 是用 Rust 和 V8 构建的 JavaScript / TypeScript 运行时。本手册对应 v1.17.0。"
status: "stable"
id: "introduction"
---

Amber 用一个 `amber` 二进制运行 JavaScript 和 TypeScript 文件。它自带测试运行器、打包器、单文件编译器、一个小型包安装器、HTTP serve 命令，以及用于运行 Agent 工具的权限沙箱。

Amber 以 MIT 协议开源，源码在 [github.com/zh30/amberjs](https://github.com/zh30/amberjs)。

## Amber 是什么

- **用 Rust 写的 V8 运行时。** CLI 通过 `src/runtime_minimal.rs` 在 V8 上执行脚本。TypeScript 和 TSX 先经 oxc 转译再运行。
- **一个二进制，多个命令。** `amber run`、`amber test`、`amber bundle`、`amber compile`、`amber install`、`amber serve` 都在同一个可执行文件里。
- **有书面契约的 Node.js 与 Web API。** 每个 Stable API 在 `docs/` 下有契约文件，在 `tests/` 下有集成测试。契约写明能做什么、边界在哪里。
- **Agent 工具的宿主。** `amber run --sandbox` 默认拒绝文件、网络、环境变量和子进程访问，除非你显式放行。`amber session` 和 `amber mcp` 通过 JSON-RPC 调用工具文件导出的函数。

## Amber 不是什么

Amber **不是** Node.js、Bun 或 Deno 的直接替代品。很多 Node 模块只实现了一部分，有些还处于 Preview。依赖某个 API 之前，请先查 [兼容性页面](/compatibility)。

Amber 目前不发布性能数字。仓库里较早的基准表格属于历史资料，本版本没有重新跑过。

## 如何阅读本手册

每项能力都带有四种标签之一：

| 标签 | 含义 |
| :--- | :--- |
| **Stable** | 有契约文档和在 CI 中运行的可执行测试，限制写在 API 旁边。 |
| **Preview** | 在默认构建中可用，但契约或边界情况仍在收紧。 |
| **Experimental** | 代码存在，但在重新验证之前不构成产品承诺。 |
| **不支持** | 目前刻意不做。Amber 会明确报错，而不是假装支持。 |

这些标签的唯一事实来源是 [`docs/CURRENT_SCOPE.md`](https://github.com/zh30/amberjs/blob/main/docs/CURRENT_SCOPE.md)。如果本站与该文件不一致，以该文件为准。

## 下一步

- [安装 Amber](/docs/installation)
- [运行第一个脚本](/docs/quick-start)
- [浏览 CLI](/docs/cli-usage)
- [查看 Node.js 兼容性](/docs/node-compat)

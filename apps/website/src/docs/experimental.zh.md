---
title: "Experimental 能力"
nav: "Experimental"
subtitle: "已随二进制发布、但尚未成为产品承诺的命令、参数与模块。"
status: "experimental"
id: "experimental"
---

`amber` 二进制包含的能力多于 Stable 范围。本页列出其余部分，不做隐藏，但其中任何一项都不构成承诺。每一项都需要重新验证，才能按[毕业规则](/docs/stability)升级。

## 已分级为 Experimental

以下内容在 `CURRENT_SCOPE.md` 中标为 Experimental：

| 能力 | 说明 |
| :--- | :--- |
| `amber debug` | 旧的调试入口，建议改用 `amber run --inspect` |
| `amber init`、`amber create` | 项目脚手架 |
| `amber add`、`amber remove`、`amber prune`、`amber upgrade` | `amber install` 契约之外的依赖编辑 |
| `amber x`（`dlx`、`bunx`） | 不安装直接运行包的可执行文件 |
| `amber test --parallel` | 直接拒绝，退出码 2 |
| N-API hello 加载器 | `process.dlopen` 能加载 C hello 插件。这不是 Node ABI 承诺，Prisma 与 sharp 不在范围内 |
| 可选 Cargo feature | `benchmarks` 与 `observability` 在 CI 中编译；`cloudnative`、`enterprise`、`multilang` 没有 CI 门禁；`tch` 已从本树移除（它引入未修复的 `zip` 0.6）；`ai` feature 为空 |
| Homebrew、GHCR、winget | 分发渠道。GHCR 镜像仅支持 linux/amd64，winget 清单尚未提交 |

## 存在但未分级

以下内容存在于 `src/main.rs` 或 `amber:*` 模块中，但范围页没有为它们分级。本手册按 Experimental 对待：

| 能力 | 宣称的功能 |
| :--- | :--- |
| `amber fmt`、`amber lint` | 格式化与检查 JS/TS 文件 |
| `amber bench` | 运行微基准 |
| `amber task` | 运行 `package.json` 脚本 |
| `amber types` | 输出 Amber 内置模块的类型定义 |
| `amber lsp` | Language Server Protocol 服务 |
| `amber profile` | 写出 `.cpuprofile` |
| `amber deploy` | 生成 Docker、独立二进制或 Kubernetes 脚手架 |
| `amber record`、`amber replay` | 录制与回放执行轨迹 |
| `--import-map` | 用 JSON import map 重映射裸模块名 |
| `--virtual-fs` | 内存文件系统覆盖层 |
| `amber:mcp` | JavaScript `McpServer` API |
| `amber:kv`、`amber:db`、`amber:vector` | 内嵌存储辅助 |
| `amber:bus`、`amber:grammar`、`amber:checkpoint`、`amber:weights`、`amber:tools` | Agent 辅助模块 |
| `amber:security`、`amber:std`、`amber:pool`、`amber:ffi` | 其他宿主模块 |

本站早期版本曾把其中多项写成已完成的功能，并常常附带性能数字。由于这些说法从未重新验证，v1.17 文档重写时删除了这些页面，旧链接会重定向到这里。

## 希望某项毕业？

请到 [github.com/zh30/amberjs/issues](https://github.com/zh30/amberjs/issues) 提交 issue 并说明使用场景。毕业需要书面契约、CI 中的可执行测试，以及成文的限制说明。

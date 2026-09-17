---
title: "Beejs v0.1 公开发布范围说明"
excerpt: "Beejs v0.1 公开版本的定位与核心能力边界。"
date: "2026-05-25"
author: "Beejs 核心团队"
readTime: "3 分钟阅读"
tag: "版本发布"
---

# Beejs v0.1 公开发布范围说明

Beejs v0.1 是本运行时的首个公开核心版本。我们的目标清晰且务实：确保默认 CLI 具备简单易用的安装体验、稳定的执行能力，并准确宣示当前版本的真实功能边界。

## v0.1 包含的核心能力

- **原生 JavaScript 执行**：基于 Rust + V8 构建的高效运行时。
- **免配置 TypeScript 体验**：传入 `.ts` 或 `.tsx` 入口文件时由 oxc 转译 TypeScript 6.0 语法（仅转译，不做 tsc 类型检查）。
- **丰富的 CLI 子命令**：内置 `run`、`eval`、`repl`、`test`、`bundle`、`serve`、项目初始化与包管理功能。
- **Node.js 与 Web API 兼容层**：集成常用的核心兼容模块。
- **自动化发布校验**：全面覆盖 代码格式（fmt）、Clippy 检查、核心库单元测试及 CLI 输出规范。

## 关于历史阶段特性的说明

仓库中包含较多历史阶段报告与试验性 feature。这些文档是宝贵的架构设计演进记录，但并不代表公开版本的默认服务承诺。

在 v0.1 阶段，所有性能指标均以最新、可复现的 Benchmark 为准。公开文档将始终聚焦于已验证的真实能力，而非历史阶段宣称的指标。

## 快速体验

```bash
curl -fsSL https://bee.zhanghe.dev/install.sh | sh
bee --version
bee eval "1 + 1"
bee run hello.js
```

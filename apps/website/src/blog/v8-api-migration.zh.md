---
title: "发布前的运行时代码梳理与重构"
excerpt: "Beejs v0.1 如何在发布前精简 CLI 输出、强化 CI 发布门禁并重构文档系统。"
date: "2026-05-25"
author: "Beejs 核心团队"
readTime: "4 分钟阅读"
tag: "工程实践"
---

# 发布前的运行时代码梳理与重构

在 v0.1 版本发布筹备过程中，我们致力于将 Beejs 从一个活跃演进的开发仓库转化为开发者可以直接安装并评估的高质量运行时。

## 纯净的 CLI 输出体验

默认的 CLI 执行路径现在直接读取 Cargo 元数据中的版本信息，并保持 `eval` 和 `run` 的输出纯净，专注于呈现用户代码的运行结果。我们从默认初始化路径中移除了冗余的内部 Web API 调试日志。

```bash
bee --version
bee eval "1 + 1"
```

## 聚焦默认构建的 CI 检查

持续集成（CI）现已全面对齐用户实际安装的默认 Release 表面：

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --lib`
- `cargo test --test beejs_core_tests`
- `cargo test --test cli_release_tests`
- macOS 与 Linux 平台构建作业

对于 Feature-gated 模块，仍可独立进行编译检查，但不再影响默认公开版本的承诺。

## 官方文档全面更新

官网与 README 现将 Beejs v0.1 明确定义为基于 Rust + V8 的 JavaScript/TypeScript 运行时，具备专注于脚本执行的 CLI 与实用兼容层。历史阶段报告将继续妥善保存在仓库中供参考。

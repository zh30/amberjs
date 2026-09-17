---
title: "Beejs v0.1 Public Scope"
date: "2026-05-25"
author: "Beejs Core Team"
readTime: "3 min read"
tag: "Release"
---

# Beejs v0.1 Public Scope

Beejs v0.1 is the first public core release target for the runtime. The goal is narrow and practical: make the default CLI installable, runnable, and honest about what the current build supports.

## What v0.1 Includes

- JavaScript execution through the Rust + V8 runtime.
- TypeScript and TSX entry files routed through oxc (TypeScript 6.0 syntax, transpile-only) before execution.
- CLI commands for `run`, `eval`, `repl`, `test`, `bundle`, `serve`, project initialization, and package operations.
- Selected Node.js and Web API compatibility modules.
- Release checks for formatting, clippy, library tests, core runtime tests, and CLI output behavior.

## What Is Not a v0.1 Promise

The repository includes many historical stage reports and feature-gated experiments. Those remain useful engineering records, but they are not the public default runtime contract.

For v0.1, performance claims must come from fresh, reproducible benchmark runs. Until then, public documentation focuses on verified capability rather than inherited stage metrics.

## Try It

```bash
curl -fsSL https://bee.zhanghe.dev/install.sh | sh
bee --version
bee eval "1 + 1"
bee run hello.js
```

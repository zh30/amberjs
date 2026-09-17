---
title: "任务调度与脚本执行器 (Task Runner)"
subtitle: "免除 Node/npm 依赖的超轻量 scripts 调度引擎，深度集成项目工作流"
group: "工程工具链"
id: "task-runner"
---

在传统 JavaScript/TypeScript 项目中，执行 `package.json` 中的构建脚本往往需要安装庞大的 Node.js 运行时与 npm。Beejs 内置了高性能原生 **Task Runner**，让你在纯净系统环境中实现瞬时脚本调度。

---

## 1. 快速使用

### 1.1 列出项目所有任务
在包含 `package.json` 的目录下直接运行 `bee task`：

```bash
$ bee task
```

终端将美观打印当前项目声明的所有 scripts 及其对应命令：

```text
📋 Available tasks in package.json:

  Task                 Command
  ────────────────────────────────────────────────────────
  build                bee bundle src/index.ts -o dist/bundle.js
  test                 bee test --coverage
  lint                 bee lint src/
  format               bee fmt src/
  serve                bee serve app.ts --port 3000
```

### 1.2 执行指定任务
通过 `bee task <name>` 或别名 `bee run <script>` 执行：

```bash
$ bee task build
# 或者与 npm / bun 体验完全一致：
$ bee run build
```

传递附加参数给底层命令（通过 `--` 分隔）：
```bash
$ bee task test -- --bail
```

---

## 2. 核心架构与环境隔离机制

### 2.1 零 npm/node 依赖
Task Runner 完全采用 Rust 原生解析 `package.json`，直接创建轻量 OS 子进程，无需在系统中安装 Node.js、npm、pnpm 或 yarn。

### 2.2 自动注入与 PATH 优先解析
当执行任务时，Beejs 会自动智能配置子进程环境变量：
1. **优先查找本地 `.bin`**：将当前项目的 `<project_root>/node_modules/.bin` 插入到系统 `PATH` 最前端；
2. **内联优先 `bee` 运行时**：将当前正在运行的 `bee` 宿主二进制路径加入 `PATH`，确保 `scripts` 中编写的 `bee fmt` 或 `bee test` 始终调用当前版本；
3. **跨平台兼容**：在 Unix（macOS / Linux）系统上自动使用 `sh -c`，在 Windows 上自动使用 `cmd.exe /C` 执行复合命令与管道符号。

---

## 3. 常见工作流配合

在典型生产项目中，推荐在 `package.json` 中配置一套全栈 Beejs 工具流：

```json
{
  "name": "my-beejs-service",
  "version": "1.0.0",
  "scripts": {
    "dev": "bee serve app.ts --watch",
    "build": "bee bundle app.ts -o dist/bundle.js --minify",
    "compile": "bee compile app.ts -o my-service",
    "check": "bee lint src/ && bee fmt src/ --check",
    "test": "bee test --coverage",
    "bench": "bee bench benches/"
  }
}
```

现在，只需一行命令即可驱动整个研发流水线：
```bash
$ bee run check
$ bee run test
$ bee run build
```

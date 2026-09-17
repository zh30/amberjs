---
title: "VS Code 官方编辑器插件"
subtitle: "深度集成 Language Server Protocol (LSP)、Chrome DevTools 远程调试、原生格式化与一键部署"
group: "生态与扩展"
id: "ide-extension"
---

## 1. 概览

为了让开发者在 Visual Studio Code 中获得首屈一指的 TypeScript / JavaScript 开发体验，Beejs 提供了官方 VS Code 扩展（`tools/vscode-extension`）。

该插件不再是一个简单的语法高亮工具，而是与 Beejs CLI 底层进程进行全双工通信的深度集成方案：
- **原生 LSP 协议直连**：通过标准输入输出（stdio）直接与 `bee lsp` 交互，提供实时的代码诊断、悬停类型提示与智能补全；
- **CDP 远程断点调试**：一键启动 `--inspect-brk` 并自动挂载 Chrome DevTools Protocol 调试会话；
- **零等待代码格式化**：直接对接 `bee fmt`（由 Rust oxc 强力驱动），实现毫秒级保存自动格式化；
- **类型声明即时导出**：通过 `bee types` 导出官方 TypeScript 声明定义，确保编辑器环境对 `bee:*` 模块完全感知；
- **可视化一键部署**：通过命令面板直接触发 `bee deploy`，快速生成 Docker 与 Kubernetes 配置。

---

## 2. 安装与环境准备

### 本地编译安装 VSIX 包

在仓库中的 `tools/vscode-extension` 目录中：

```bash
cd tools/vscode-extension
npm install
npm run package
```

编译完成后，会在目录下生成 `beejs-1.1.0.vsix`。在 VS Code 中执行：
1. 按下快捷键 `Ctrl+Shift+P`（macOS 上为 `Cmd+Shift+P`）；
2. 输入并选择 `Extensions: Install from VSIX...`；
3. 选择生成的 `beejs-1.1.0.vsix` 文件即可完成安装。

---

## 3. 核心功能与命令

### 一、语言服务器 (Beejs LSP)
插件在打开 `.js`、`.ts` 或 `.jsx`、`.tsx` 文件时自动启动后台 `bee lsp` 守护进程：
- **语法与类型诊断**：即时报告语法错误与未解析模块；
- **悬停信息 (Hover)**：显示内置 API（如 `bee:db`、`bee:vector`、`bee:std`）的 JSDoc 文档与函数签名；
- **智能补全 (Completions)**：提供模块导出与常用代码片段补全。

### 二、一键调试 (Debug with Beejs)
直接在编辑器代码行左侧打上断点，按下 `F5` 或在命令面板中执行：
- `Beejs: Debug Current Script`

插件会自动执行 `bee run --inspect-brk=<port> <file>`，并无缝将 VS Code 的调试面板挂载至该 V8 会话，支持单步跳过（Step Over）、进入函数（Step Into）以及变量堆栈实时查看。

### 三、快捷命令参考

在命令面板（`Cmd+Shift+P`）中输入 `Beejs:` 即可查看所有扩展能力：

| 命令 | 描述 | 触发动作 |
| :--- | :--- | :--- |
| `Beejs: Run Current Script` | 在集成终端中运行当前文件 | 调用 `bee run <file>` |
| `Beejs: Debug Current Script` | 启动 CDP 调试会话 | 挂载 Chrome 调试器 |
| `Beejs: Format Document` | 极速格式化当前文件 | 调用 `bee fmt <file>` |
| `Beejs: Export TypeScript Types` | 导出最新内置类型定义 | 调用 `bee types` 生成 `beejs.d.ts` |
| `Beejs: Deploy Application` | 交互式生成部署脚手架 | 调用 `bee deploy` |

---

## 4. 推荐 VS Code 配置 (`settings.json`)

将以下配置加入工作区的 `.vscode/settings.json`，即可在保存代码时获得极致的流畅格式化体验：

```json
{
  "[typescript]": {
    "editor.defaultFormatter": "beejs.beejs-tools",
    "editor.formatOnSave": true
  },
  "[javascript]": {
    "editor.defaultFormatter": "beejs.beejs-tools",
    "editor.formatOnSave": true
  },
  "beejs.executablePath": "bee",
  "beejs.enableLsp": true
}
```

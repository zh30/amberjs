---
title: "交互终端、CDP 调试与语言服务 (REPL, Debug & LSP)"
subtitle: "升级版 Rustyline 终端、Chrome DevTools 远程调试与语言服务器协议支持"
group: "工程工具链"
id: "debugging-lsp"
---

高效的开发体验离不开强大的交互式探索、可视化断点调试以及智能代码补全。Beejs 提供了三大深度集成的工具系统：
1. **交互式 REPL (`bee repl`)**：基于 Rustyline 打造，支持智能多行代码检测与跨会话历史持久化；
2. **CDP 调试器 (`bee debug` / `--inspect`)**：兼容 Chrome 开发者工具与 VS Code 的远程调试服务；
3. **语言服务器 (`bee lsp`)**：符合 LSP 3.17 标准，为现代 IDE 提供亚毫秒级诊断、格式化与文档悬停。

---

## 1. 现代化交互终端 (`bee repl`)

通过 `bee repl` 启动交互式控制台：

```bash
$ bee repl
```

### 1.1 核心特性
- **智能多行块检测**：当你在终端输入未闭合的函数 `function test() {`、对象/数组字面量或反引号模板字符串时，REPL 自动切换为多行输入提示符（`... `），待语法完整闭合后一并执行；
- **历史记录持久化**：所有执行过的命令自动持久化保存至用户主目录 `~/.beejs_history`，支持通过上下方向键自由翻看；
- **行编辑与光标自由移动**：支持退格、行首行尾跳转、单词跳转等标准行编辑体验；
- **内置辅助指令**：
  - `.help`：查看 REPL 帮助信息；
  - `.clear`：清屏并重置当前执行上下文；
  - `.exit`：安全退出 REPL 会话（亦可按 `Ctrl+D`）。

---

## 2. Chrome DevTools 协议调试 (`bee debug`)

Beejs 实现了标准 **Chrome DevTools Protocol (CDP)**，你可以直接使用 Chrome 浏览器或 VS Code 对运行中的脚本进行打断点、单步步进与变量检查。

### 2.1 启动调试会话

```bash
# 启动并在第一行自动挂起，等待调试器连接
$ bee debug app.ts

# 或者通过参数开启
$ bee run --inspect-brk app.ts
```

终端输出：
```text
Debugger listening on ws://127.0.0.1:9229/ws
Visit chrome://inspect in Google Chrome to connect.
Waiting for debugger connection before executing code...
```

### 2.2 在 Chrome 中连接调试
1. 在 Chrome 浏览器地址栏打开 `chrome://inspect`；
2. 在 **Remote Target** 列表中将自动发现正在运行的 Beejs 实例；
3. 点击 **inspect** 按钮，即可唤起专用的 DevTools 调试窗口；
4. 可以在源码面板中自由添加断点（Breakpoints）、单步步过（Step Over, `F10`）、单步进入（Step Into, `F11`）以及实时审查调用栈与局部作用域变量。

---

## 3. 语言服务器协议 (`bee lsp`)

Beejs 内置了标准的 **Language Server Protocol (LSP 3.17)** 服务，支持直接作为 VS Code、Neovim、Helix、Sublime Text 等现代编辑器的后端语言服务器。

### 3.1 启动服务

```bash
$ bee lsp
```

### 3.2 协议功能覆盖
- **`textDocument/publishDiagnostics`**：结合 OXC 静态检查，在开发者键入代码时实时推送语法错误与危险模式高亮；
- **`textDocument/formatting`**：无缝调用 `bee fmt` 提供零配置一键格式化文档；
- **`textDocument/hover`**：针对 `bee:ai`、`Tensor`、`LLM`、`AgentPipeline` 等原生内置对象，提供类型签名与 Markdown 文档悬停展示。

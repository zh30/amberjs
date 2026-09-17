---
title: "REPL, CDP Debugging & Language Server (REPL, Debug & LSP)"
subtitle: "Upgraded Rustyline REPL, Chrome DevTools Protocol remote debugging, and LSP 3.17 server"
group: "Developer Tooling"
id: "debugging-lsp"
---

High developer velocity requires rich interactive exploration, visual breakpoint debugging, and intelligent IDE completion. Beejs delivers three integrated systems:
1. **Interactive REPL (`bee repl`)**: Powered by Rustyline with smart multiline detection and persistent command history;
2. **CDP Debugger (`bee debug` / `--inspect`)**: Standard Chrome DevTools Protocol compatible endpoint;
3. **Language Server (`bee lsp`)**: LSP 3.17 compliant server providing real-time diagnostics, formatting, and hover docs.

---

## 1. Modern Interactive REPL (`bee repl`)

Launch the REPL from the command line:

```bash
$ bee repl
```

### 1.1 Features
- **Smart Multiline Block Detection**: Automatically shifts to a continuation prompt (`... `) when detecting open braces `{`, unclosed parentheses `(`, brackets `[`, or template strings `` ` ``;
- **Persistent History**: Automatically saves command history across sessions to `~/.beejs_history` with up/down arrow recall;
- **Full Line Editing**: Emacs-style keybindings, cursor movement, word skipping, and deletion;
- **Built-in Directives**:
  - `.help`: View help information;
  - `.clear`: Clear the console and reset context;
  - `.exit`: Exit the session cleanly (or press `Ctrl+D`).

---

## 2. Chrome DevTools Protocol Debugging (`bee debug`)

Beejs implements the standard **Chrome DevTools Protocol (CDP)**. You can attach Chrome DevTools or VS Code to set breakpoints, step through code, and inspect scopes.

### 2.1 Starting a Debugging Session

```bash
# Start and pause on the first statement awaiting debugger
$ bee debug app.ts

# Or with flag
$ bee run --inspect-brk app.ts
```

Console output:
```text
Debugger listening on ws://127.0.0.1:9229/ws
Visit chrome://inspect in Google Chrome to connect.
Waiting for debugger connection before executing code...
```

### 2.2 Connecting in Chrome
1. Open `chrome://inspect` in Google Chrome or Chromium;
2. Under **Remote Target**, locate your running Beejs target;
3. Click **inspect** to launch dedicated DevTools;
4. Set breakpoints, step over (`F10`), step into (`F11`), and inspect call stacks and variables.

---

## 3. Language Server Protocol (`bee lsp`)

Beejs includes a **Language Server Protocol (LSP 3.17)** daemon for editors such as VS Code, Neovim, Helix, and Sublime Text.

### 3.1 Running the Server

```bash
$ bee lsp
```

### 3.2 Implemented Capabilities
- **`textDocument/publishDiagnostics`**: Pushes AST diagnostics and syntax errors in real-time as you type;
- **`textDocument/formatting`**: Calls `bee fmt` for zero-config document formatting;
- **`textDocument/hover`**: Provides rich Markdown tooltips and type signatures for `bee:ai`, `Tensor`, `LLM`, and `AgentPipeline`.

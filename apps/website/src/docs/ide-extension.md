---
title: "Official VS Code Extension"
subtitle: "Deep integration with Language Server Protocol (LSP), Chrome DevTools Protocol debugging, formatting & deployment"
group: "Ecosystem"
id: "ide-extension"
---

## 1. Overview

To deliver a premier TypeScript / JavaScript development experience in Visual Studio Code, Beejs provides the official VS Code Extension (`tools/vscode-extension`).

Rather than basic syntax coloring, it communicates full-duplex with the Beejs binary:
- **Native LSP via stdio**: Directly communicates with `bee lsp` for live diagnostics, hover docs, and code completions.
- **CDP Remote Debugging**: Launches `--inspect-brk` with automatic attachment to the Chrome DevTools Protocol debugging engine.
- **Zero-Wait Formatting**: Direct integration with `bee fmt` (powered by Rust oxc) for sub-millisecond format-on-save.
- **Type Declaration Export**: `bee types` syncs official TypeScript definitions for complete `bee:*` intelligence.
- **One-Click Deployment**: Runs `bee deploy` directly from the Command Palette.

---

## 2. Installation & Setup

### Building and Installing VSIX

From `tools/vscode-extension`:

```bash
cd tools/vscode-extension
npm install
npm run package
```

This compiles `beejs-1.1.0.vsix`. Inside VS Code:
1. Press `Ctrl+Shift+P` (or `Cmd+Shift+P` on macOS).
2. Type and select `Extensions: Install from VSIX...`.
3. Pick the generated `beejs-1.1.0.vsix` file.

---

## 3. Key Capabilities & Commands

### 1. Language Server (Beejs LSP)
The extension automatically spawns a background `bee lsp` process on `.js`, `.ts`, `.jsx`, and `.tsx` files:
- **Diagnostics**: Real-time syntax and unresolved module warnings.
- **Hover Information**: Full signatures and JSDoc for built-ins (`bee:db`, `bee:vector`, `bee:std`).
- **Completions**: Auto-suggests module exports and idioms.

### 2. CDP Debugging (Debug with Beejs)
Place breakpoints in your editor margin and press `F5` or invoke:
- `Beejs: Debug Current Script`

The extension runs `bee run --inspect-brk=<port> <file>` and connects VS Code's debugger to the V8 session, supporting step-over, step-into, call-stack inspection, and variable watch.

### 3. Command Palette Quick Reference

| Command | Description | Action |
| :--- | :--- | :--- |
| `Beejs: Run Current Script` | Runs current file in integrated terminal | `bee run <file>` |
| `Beejs: Debug Current Script` | Spawns CDP debug session | Attaches debugger |
| `Beejs: Format Document` | Formats current document with oxc | `bee fmt <file>` |
| `Beejs: Export TypeScript Types` | Exports built-in type declarations | `bee types` |
| `Beejs: Deploy Application` | Interactive deployment generator | `bee deploy` |

---

## 4. Recommended `settings.json`

Add the following to your `.vscode/settings.json` for seamless format-on-save:

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

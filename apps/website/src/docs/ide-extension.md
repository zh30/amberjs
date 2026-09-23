---
title: "Official VS Code Extension"
subtitle: "Deep integration with Language Server Protocol (LSP), Chrome DevTools Protocol debugging, formatting & deployment"
group: "Ecosystem"
id: "ide-extension"
---

## 1. Overview

To deliver a premier TypeScript / JavaScript development experience in Visual Studio Code, Amber provides the official VS Code Extension (`tools/vscode-extension`).

Rather than basic syntax coloring, it communicates full-duplex with the Amber binary:
- **Native LSP via stdio**: Directly communicates with `amber lsp` for live diagnostics, hover docs, and code completions.
- **CDP Remote Debugging**: Launches `--inspect-brk` with automatic attachment to the Chrome DevTools Protocol debugging engine.
- **Zero-Wait Formatting**: Direct integration with `amber fmt` (powered by Rust oxc) for sub-millisecond format-on-save.
- **Type Declaration Export**: `amber types` syncs official TypeScript definitions for complete `amber:*` intelligence.
- **One-Click Deployment**: Runs `amber deploy` directly from the Command Palette.

---

## 2. Installation & Setup

### Building and Installing VSIX

From `tools/vscode-extension`:

```bash
cd tools/vscode-extension
npm install
npm run package
```

This compiles `amberjs-1.1.0.vsix`. Inside VS Code:
1. Press `Ctrl+Shift+P` (or `Cmd+Shift+P` on macOS).
2. Type and select `Extensions: Install from VSIX...`.
3. Pick the generated `amberjs-1.1.0.vsix` file.

---

## 3. Key Capabilities & Commands

### 1. Language Server (Amber LSP)
The extension automatically spawns a background `amber lsp` process on `.js`, `.ts`, `.jsx`, and `.tsx` files:
- **Diagnostics**: Real-time syntax and unresolved module warnings.
- **Hover Information**: Full signatures and JSDoc for built-ins (`amber:db`, `amber:vector`, `amber:std`).
- **Completions**: Auto-suggests module exports and idioms.

### 2. CDP Debugging (Debug with Amber)
Place breakpoints in your editor margin and press `F5` or invoke:
- `Amber: Debug Current Script`

The extension runs `amber run --inspect-brk=<port> <file>` and connects VS Code's debugger to the V8 session, supporting step-over, step-into, call-stack inspection, and variable watch.

### 3. Command Palette Quick Reference

| Command | Description | Action |
| :--- | :--- | :--- |
| `Amber: Run Current Script` | Runs current file in integrated terminal | `amber run <file>` |
| `Amber: Debug Current Script` | Spawns CDP debug session | Attaches debugger |
| `Amber: Format Document` | Formats current document with oxc | `amber fmt <file>` |
| `Amber: Export TypeScript Types` | Exports built-in type declarations | `amber types` |
| `Amber: Deploy Application` | Interactive deployment generator | `amber deploy` |

---

## 4. Recommended `settings.json`

Add the following to your `.vscode/settings.json` for seamless format-on-save:

```json
{
  "[typescript]": {
    "editor.defaultFormatter": "amberjs.amberjs-tools",
    "editor.formatOnSave": true
  },
  "[javascript]": {
    "editor.defaultFormatter": "amberjs.amberjs-tools",
    "editor.formatOnSave": true
  },
  "amberjs.executablePath": "amber",
  "amberjs.enableLsp": true
}
```

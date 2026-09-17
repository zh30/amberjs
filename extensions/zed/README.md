# Beejs Zed extension

Launches the Beejs language server (`bee lsp`) for JavaScript, TypeScript, TSX, JSX, and `.beejs` files. This is the Zed client counterpart to `tools/vscode-extension` (same `bee lsp` argv and document set).

It does **not** bundle a second language server. A `bee` binary must already be installed.

## Install as a Zed dev extension

1. Install [Rust](https://rustup.rs/) and the WASI target Zed uses to compile extensions:

   ```bash
   rustup target add wasm32-wasip2
   ```

2. Install the `bee` CLI so it is on your `PATH` (or configure an explicit path, below).

3. In Zed, open the command palette and run **zed: extensions**.
4. Click **Install Dev Extension** and choose this directory:

   ```text
   tools/zed-extension
   ```

   (from the beejs repository root: `…/beejs/tools/zed-extension`).

Zed compiles the extension to `wasm32-wasip2` on install. After that, opening a `.js` / `.ts` / `.beejs` file in a worktree should start `bee lsp`.

Publishing to `zed-industries/extensions` is not part of this tree.

## `bee` binary

Discovery order:

1. Zed setting `lsp.bee-lsp.binary.path` (explicit runtime path).
2. `Worktree::which("bee")` (and `bee.exe` on Windows) — the worktree `PATH`, not a host `std::env::var("PATH")` read inside the WASM module.

If neither finds a binary, the extension returns an error telling you to install Beejs or set the path.

Example Zed `settings.json`:

```json
{
  "lsp": {
    "bee-lsp": {
      "binary": {
        "path": "/usr/local/bin/bee"
      }
    }
  }
}
```

## Develop / test

From this directory:

```bash
cargo test
cargo build --target wasm32-wasip2
```

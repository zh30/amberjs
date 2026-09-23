# Amber Zed extension

Launches the Amber language server (`amber lsp`) for JavaScript, TypeScript, TSX, JSX, and `.amberjs` files. This is the Zed client counterpart to `tools/vscode-extension` (same `amber lsp` argv and document set).

It does **not** bundle a second language server. A `amber` binary must already be installed.

## Install as a Zed dev extension

1. Install [Rust](https://rustup.rs/) and the WASI target Zed uses to compile extensions:

   ```bash
   rustup target add wasm32-wasip2
   ```

2. Install the `amber` CLI so it is on your `PATH` (or configure an explicit path, below).

3. In Zed, open the command palette and run **zed: extensions**.
4. Click **Install Dev Extension** and choose this directory:

   ```text
   tools/zed-extension
   ```

   (from the amberjs repository root: `…/amberjs/tools/zed-extension`).

Zed compiles the extension to `wasm32-wasip2` on install. After that, opening a `.js` / `.ts` / `.amberjs` file in a worktree should start `amber lsp`.

Publishing to `zed-industries/extensions` is not part of this tree.

## `amber` binary

Discovery order:

1. Zed setting `lsp.amber-lsp.binary.path` (explicit runtime path).
2. `Worktree::which("amber")` (and `amber.exe` on Windows) — the worktree `PATH`, not a host `std::env::var("PATH")` read inside the WASM module.

If neither finds a binary, the extension returns an error telling you to install Amber or set the path.

Example Zed `settings.json`:

```json
{
  "lsp": {
    "amber-lsp": {
      "binary": {
        "path": "/usr/local/bin/amber"
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

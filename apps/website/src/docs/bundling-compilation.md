---
title: "Bundle & compile"
subtitle: "Stable amber bundle and Stable SEA amber compile"
group: "Developer Tooling"
id: "bundling-compilation"
---

Two packaging tools:

1. **`amber bundle`** — **Stable**. Pack a local JS/TS/JSON graph into one IIFE
2. **`amber compile`** — **Stable**. Copy the host `amber` binary and embed a script payload (SEA)

`amber bundle` is not webpack / rollup / esbuild parity. Full contract: [BUNDLE_CONTRACT.md](https://github.com/zh30/amberjs/blob/main/docs/BUNDLE_CONTRACT.md). `amber compile` is not pkg, nexe, or Bun compile. The SEA contract is [`docs/COMPILE_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/COMPILE_CONTRACT.md). `amber install` is a separate **Stable** subset, not an npm replacement: [`docs/INSTALL_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/INSTALL_CONTRACT.md).

---

## `amber bundle`

```bash
amber bundle src/index.ts -o dist/bundle.js
amber bundle src/index.ts -o dist/bundle.min.js --minify
amber bundle src/index.ts -o dist/bundle.js --sourcemap
```

Contract (pinned by `tests/bundle_contract_tests.rs`):

- Single file entry. Static `import` / `export from` / `require("...")` only
- Inlines `.js` / `.mjs` / `.cjs` / `.jsx` / `.ts` / `.tsx` / `.mts` / `.cts` / `.json`
- Unresolved bare specifiers stay runtime `require()` (Node builtins, missing packages)
- Unresolved `./` / `../` and non-JS/JSON assets fail with `error: amber bundle:` and do not write the outfile
- `--sourcemap` writes SourceMap v3 `<name>.map` with `sources` listed and `mappings` empty
- `--target` is a header comment; `--tree-shake` is accepted and ignored
- `node_modules` uses `package.json` `"main"` only (not `"exports"`)

Out of scope: code splitting, CSS/image/Wasm pipelines, dynamic `import()`, full Node ecosystem bundling.

---

## `amber compile` (Stable)

```bash
amber compile app.ts -o myapp
./myapp
```

Linux, macOS, and Windows hosts only. The output is a copy of **this** `amber` plus a trailer. It is not cross-compiled and not freestanding: it needs the same dynamic linker as the `amber` that built it. Other operating systems fail with `unsupported host OS` and write nothing.

On Linux and Windows the trailer is the last 32 bytes of the file. On macOS those same bytes are the end of a `__AMBER` segment inserted before `__LINKEDIT`; ad-hoc `codesign` then appends a signature, which is the end of the file.

Layout:

```text
+---------------------------------------------------------------+
|  host amber executable (unmodified copy)                      |
+---------------------------------------------------------------+
|  UTF-8 bundled script                                         |
+---------------------------------------------------------------+
|  payload length (u64 LE)  |  flags (u64 LE, must be 0)        |
+---------------------------------------------------------------+
|  magic AMBER_STANDALONE (16 bytes)                            |
+---------------------------------------------------------------+
```

`AMBER_STANDALONE` is that magic. It is not an environment variable. On boot, a valid trailer runs the payload and skips the CLI, so `--help` and `--version` are script arguments. `process.argv[0]` and `process.argv[1]` are both the executable path.

Contracted failures print a line starting with `error: amber compile:` (or `error: amber standalone:` once the binary is running) and do not leave a half-written output.

Limits, all explicit:

- dynamic `import()` and computed `require(expr)` fail the compile
- `.node` native addons are rejected, not embedded
- no virtual filesystem; workers and `fs` paths are the real disk
- macOS inserts segment `__AMBER`, then ad-hoc `codesign`; if either step fails, the compile fails
- not pkg / nexe / Bun compile parity

---

## Status

| | Status |
| :--- | :--- |
| `amber bundle` | **Stable** (limits in the contract above) |
| `amber compile` | **Stable** |
| Tests | `tests/bundle_contract_tests.rs`, `tests/compile_contract_tests.rs`, `tests/bundle_compile_tests.rs` |

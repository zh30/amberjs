---
title: "Bundle & compile"
subtitle: "Stable amber bundle (local graph); Preview SEA amber compile"
group: "Developer Tooling"
id: "bundling-compilation"
---

Two packaging tools:

1. **`amber bundle`** — **Stable**. Pack a local JS/TS/JSON graph into one IIFE
2. **`amber compile`** — **Preview**. Copy the `amber` binary and append a script payload (SEA)

`amber bundle` is not webpack / rollup / esbuild / pkg parity. Full contract: [BUNDLE_CONTRACT.md](https://github.com/zh30/amberjs/blob/main/docs/BUNDLE_CONTRACT.md).

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

## `amber compile`

```bash
amber compile app.ts -o myapp
./myapp
```

Layout of the output binary:

```text
+----------------------------------------------------------+
|  Amber runtime (copy of the host `amber` executable)       |
+----------------------------------------------------------+
|  Bundled user script payload                             |
+----------------------------------------------------------+
|  payload size (u64)  |  magic AMBER_STANDALONE (16 bytes)  |
+----------------------------------------------------------+
```

On boot, `amber` inspects its own trailer. If `AMBER_STANDALONE` is present, it runs the embedded payload and skips the normal CLI parser.

Limitations: the result is roughly the size of `amber` plus your script; native addons and a full Node module graph are out of scope.

---

## Status

| | Status |
| :--- | :--- |
| `amber bundle` | **Stable** (limits in the contract above) |
| `amber compile` | Preview |
| Tests | `tests/bundle_contract_tests.rs`, `tests/bundler_integration_tests.rs` |

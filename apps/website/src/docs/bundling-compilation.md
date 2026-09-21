---
title: "Bundle & compile"
subtitle: "Preview in v1.16.0: oxc amber bundle, SEA amber compile"
group: "Developer Tooling"
id: "bundling-compilation"
---

Two packaging tools, both **Preview**:

1. **`amber bundle`** — resolve a local module graph into one JS file
2. **`amber compile`** — copy the `amber` binary and append a script payload (SEA)

The contract is still tightening. Do not treat this as webpack / esbuild / pkg parity.

---

## `amber bundle`

```bash
amber bundle src/index.ts -o dist/bundle.js
amber bundle src/index.ts -o dist/bundle.min.js --minify
```

What it does today:

- Recursively follows static local imports
- Type-strips `.ts` / `.tsx` with oxc
- Wraps each module in an isolated registry (`__amberjs_require__`)
- Optional minify

What it does **not** claim: full `node_modules` ecosystem bundling, code splitting, or browser-app tooling.

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
| `amber bundle` | Preview |
| `amber compile` | Preview |
| Tests | `tests/bundle_compile_tests.rs` |

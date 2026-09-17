---
title: "Bundle & compile"
subtitle: "Preview in v1.16.0: oxc bee bundle, SEA bee compile"
group: "Developer Tooling"
id: "bundling-compilation"
---

Two packaging tools, both **Preview**:

1. **`bee bundle`** — resolve a local module graph into one JS file
2. **`bee compile`** — copy the `bee` binary and append a script payload (SEA)

The contract is still tightening. Do not treat this as webpack / esbuild / pkg parity.

---

## `bee bundle`

```bash
bee bundle src/index.ts -o dist/bundle.js
bee bundle src/index.ts -o dist/bundle.min.js --minify
```

What it does today:

- Recursively follows static local imports
- Type-strips `.ts` / `.tsx` with oxc
- Wraps each module in an isolated registry (`__beejs_require__`)
- Optional minify

What it does **not** claim: full `node_modules` ecosystem bundling, code splitting, or browser-app tooling.

---

## `bee compile`

```bash
bee compile app.ts -o myapp
./myapp
```

Layout of the output binary:

```text
+----------------------------------------------------------+
|  Beejs runtime (copy of the host `bee` executable)       |
+----------------------------------------------------------+
|  Bundled user script payload                             |
+----------------------------------------------------------+
|  payload size (u64)  |  magic BEE_STANDALONE (16 bytes)  |
+----------------------------------------------------------+
```

On boot, `bee` inspects its own trailer. If `BEE_STANDALONE` is present, it runs the embedded payload and skips the normal CLI parser.

Limitations: the result is roughly the size of `bee` plus your script; native addons and a full Node module graph are out of scope.

---

## Status

| | Status |
| :--- | :--- |
| `bee bundle` | Preview |
| `bee compile` | Preview |
| Tests | `tests/bundle_compile_tests.rs` |

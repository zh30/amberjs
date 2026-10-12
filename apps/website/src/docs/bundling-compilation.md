---
title: "Bundle and compile"
nav: "Bundle & compile"
subtitle: "amber bundle packs a module graph into one file. amber compile turns it into a single executable for this host."
status: "stable"
id: "bundling-compilation"
---

## amber bundle

`amber bundle` is **Stable** (G1) for the contract in [`docs/BUNDLE_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/BUNDLE_CONTRACT.md).

```bash
amber bundle src/index.ts -o dist/app.js
amber bundle src/index.ts -o dist/app.js --minify --sourcemap
```

It reads static `import` and `require` calls from a local JS, TS, or JSON graph and writes one IIFE with oxc.

### Limits

- Only static `import` and `require` are followed.
- Unresolved bare specifiers stay as runtime `require()` calls.
- A missing relative import or a non-JS/JSON asset fails with `error: amber bundle:`.
- `--sourcemap` writes a SourceMap v3 file that lists sources, but `mappings` is empty.
- `--target` only writes a header comment. `--tree-shake` is accepted and ignored.
- Only `package.json` `"main"` is used, not `"exports"`.

This is not webpack, Rollup, or esbuild.

## amber compile

`amber compile` is **Stable** (G2) for the contract in [`docs/COMPILE_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/COMPILE_CONTRACT.md).

```bash
amber compile src/cli.ts -o mytool
./mytool --help
```

It copies the running `amber` executable and appends the bundled script with an `AMBER_STANDALONE` trailer. On macOS the payload goes in a `__AMBER` Mach-O segment and the binary is ad-hoc signed.

### Limits

- Linux, macOS, and Windows hosts only. There is no cross-compilation.
- The output uses the same dynamic linker as the `amber` that built it.
- Native `.node` addons are rejected.
- Dynamic `import()` and computed `require()` fail the compile.
- There is no embedded virtual filesystem. `--help` and `--version` are passed through to your script.

This is not pkg, nexe, or `bun build --compile`.

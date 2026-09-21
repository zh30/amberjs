# `amber bundle` compatibility contract

This is the user-facing contract for Stable `amber bundle`. It is derived from `src/tooling/bundler.rs`, `src/main.rs`, and the executable tests under `tests/bundle_contract_tests.rs` / `tests/bundler_integration_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`amber bundle` is a **local module-graph packer**. It is not webpack, rollup, esbuild, or a full Node.js bundler.

## Command

```bash
amber bundle <entry> [-o|--outfile <file>] [--minify] [--sourcemap] [--target <name>] [--tree-shake] [--import-map <file>]
```

- `<entry>` is a single file (not a directory, not a multi-entry list).
- Default outfile is `<entry>` with its extension replaced by `.bundle.js` (so `app.js` → `app.bundle.js`).
- `--output` is an alias of `--outfile`.
- Exit status is non-zero on every contracted failure. Failures do not write the outfile.

## Entry points

Inlined when they resolve on disk:

| Kind | Extensions |
| :--- | :--- |
| JavaScript | `.js`, `.mjs`, `.cjs`, `.jsx` |
| TypeScript | `.ts`, `.tsx`, `.mts`, `.cts` (transpile-only via oxc; no `tsc` check) |
| JSON | `.json` (emitted as `module.exports = …`) |

Resolution from an importing file:

1. Relative specifiers (`./`, `../`) against the importer directory.
2. Extension substitution (`.ts` / `.tsx` / `.js` / `.mjs` / `.cjs` / `.jsx` / `.json`) and `index.*` inside a directory.
3. Walking parents for `node_modules/<name>`, then `package.json` `"main"` only.
4. `--import-map` remaps (WICG `imports` / prefix keys) applied before (1)–(3). Remaps that point at a local path must resolve.

## Externals

Unresolved **bare** specifiers (`fs`, `path`, `left-pad` when not found under `node_modules`) stay as runtime `require('…')`. The generated IIFE falls back to host `require` when the registry id is not a bundled module.

This is the supported way to leave Node builtins and unbundled packages outside the file.

## CJS / ESM

- Static ESM `import` / `export` / `export from` / `export *` are rewritten onto an isolated `__amberjs_require__` registry. ESM modules set `module.exports.__esModule`.
- Default import uses `__esModule` interop: `mod.default` when present, otherwise the whole `module.exports`.
- Static `require("…")` of a resolved local file is remapped to `__amberjs_require__(id)`.
- CommonJS `module.exports` assignments are left in place.
- Named ESM imports from CommonJS files are **not** statically discovered. Use `require()` (or a default / namespace import) for CJS.
- Output is one IIFE, runnable with `amber run <outfile>`. It is not a dual CJS/ESM package and not a browser ESM graph.

## Sourcemaps

`--sourcemap` writes SourceMap v3 next to the outfile by replacing the outfile extension with `.map` (`dist/bundle.js` → `dist/bundle.map`).

Pinned fields:

- `version`: `3`
- `sources`: paths of every inlined module
- `names`: `[]`
- `mappings`: `""` (inventory only; not column-accurate)

No `sourceMappingURL` comment is appended. Do not treat this as a debugger-grade map.

## Assets

JSON is the only non-JS/TS payload that is inlined. CSS, images, Wasm, workers, raw files, `data:` URLs, and copy-to-dist assets are unsupported. Importing a file with any other extension fails at bundle time.

## Stable failure diagnostics

Every contracted failure prints a line that starts with `error: amber bundle:` (stderr). Message bodies:

| Case | Body contains |
| :--- | :--- |
| Missing entry | `entry file not found: <path>` |
| Directory or non-file entry | `entry must be a file: <path>` |
| Unresolved `./` or `../` (or import-map local remap) | `cannot resolve '<spec>' from <file>` |
| Unsupported asset / extension | `unsupported module '<path>' (bundled files must be js, mjs, cjs, jsx, ts, tsx, mts, cts, or json)` |
| Named import of a missing ESM export | `missing export '<name>' from '<spec>' (imported by <file>)` |
| Re-export of a missing ESM export | `missing export '<name>' from '<spec>' (cannot re-export '<exported>' in <file>)` |
| Unreadable module / outfile | `failed to read` / `failed to write` |
| TypeScript transpile error | `TypeScript compile failed for '<path>'` |

These cases do not write the outfile.

Permission-broker denials (`--deny-fs` without `--allow-read` / `--allow-write`) also exit non-zero and include `error: amber bundle:` plus the existing `permission denied` text.

## Flags that are not a bundler feature

| Flag | Actual behavior |
| :--- | :--- |
| `--target <name>` | Written as `// Target: <name>` in the unminified header. No downlevel, no browser/node polyfill. |
| `--tree-shake` | Accepted and ignored. Unused exports stay in the graph. |
| `--minify` | oxc codegen minify plus `//` line-comment strip. Not terser/esbuild parity. |

## Explicitly out of scope

- webpack / rollup / esbuild / Vite plugin APIs or config files
- `package.json` `"exports"` / `"module"` / `"browser"` (only `"main"`)
- Full `node_modules` ecosystem bundling, hoisting, or dual-package hazard handling
- Dynamic `import()`, computed `require(expr)`, `import.meta.resolve`
- Code splitting, CSS/asset pipelines, HTML entry, service-worker graphs
- Watch mode for `amber bundle`
- Promoting `amber compile` (SEA) or `amber install`

## How this is enforced

```bash
cargo test --test bundle_contract_tests
cargo test --test bundler_integration_tests
```

CI runs `bundle_contract_tests` as an explicit step so this page cannot drift from the implementation without a red job.

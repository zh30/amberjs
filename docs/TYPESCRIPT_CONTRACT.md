# TypeScript / TSX transpile contract

This is the user-facing contract for the Stable **transpile-only** TypeScript / TSX path in Amber. It is derived from `src/typescript/oxc_backend.rs`, `src/typescript/mod.rs` (`compile_typescript`), `src/main.rs` (`read_and_compile_source`), `src/runtime_minimal.rs` (eval / CommonJS `.ts` load under G28), and `tests/typescript_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

This is **not** `tsc`. There is no project-wide type-check, no `tsconfig.json` program, and no type-error diagnostics as a product promise.

**Numbering:** This contract is **G36** after service-worker fetch intercept (**G35**). Do not renumber Node G8 / G17–G28 or Web G9–G16 / G29–G35. Bundling TS entries under [`BUNDLE_CONTRACT.md`](BUNDLE_CONTRACT.md) stays **G1**.

## Stable surface

| Path / call | Behavior |
| :--- | :--- |
| CLI entry `.ts` / `.tsx` / `.mts` / `.cts` / `.jsx` | `amber run` reads the file, calls `typescript::compile_typescript` (oxc `BACKEND_ID = oxc-0.147.2`), then executes the emitted JavaScript. |
| CommonJS `require` of `.ts` / `.tsx` / `.jsx` | Under the Stable require loader (G28), the file is oxc-transpiled then wrapped as CommonJS. Same backend as the CLI entry path. |
| Eval / inline snippets that look like TypeScript | `runtime_minimal` may route through `compile_typescript` with an `eval.ts` / `eval.tsx` filename when the source looks like TypeScript / JSX. |
| Type erasure | Type annotations, `import type` / type-only imports (with `only_remove_type_imports`), interfaces, type aliases, and other type-position syntax are erased. Value imports that look unused are kept (side-effect imports). |
| Emit target | Transform target is **ES2022** so `using` / Stage-3 decorators downlevel to something the host V8 can run. |
| `using` / Stage-3 decorators | Downleveled via oxc + a minimal `babelHelpers` prelude when the emit references `babelHelpers.*`. Decorator mode is **legacy**; `emitDecoratorMetadata` is off. |
| TSX / JSX | Classic runtime: emits `React.createElement(...)`. Not the automatic JSX runtime. Not React Refresh. |
| Source maps / thrown stacks | oxc emits a SourceMap; the CLI appends `//# sourceMappingURL=data:...` and `runtime_minimal` remaps thrown stacks to `.ts` lines when a map is active. |
| Parse / transform failure | `compile_typescript` returns `Err(String)` with oxc diagnostics (`file:line:column: message` when labels exist). The CLI fails the run; it does not execute partial emit. |

Content-hash caching (`src/typescript/cache.rs`) is an implementation detail of the same Stable path; callers still see the same emit for the same `(source, file_name)` key.

## Limits

These are real behaviors of the Stable surface above, not stand-ins for missing `tsc`:

- Backend is **oxc transpile** (`parse` → `SemanticBuilder` → `Transformer` → `Codegen`). Syntax / semantic diagnostics that oxc reports fail the compile; there is no separate Amber type checker.
- JSX is classic `React.createElement` only. Callers must provide a `React` (or compatible) binding themselves; Amber does not inject one.
- Decorator metadata (`design:type`, etc.) is not emitted.
- Async `await using` that needs a real async dispose helper may throw from the prelude (`await using requires an async dispose helper`) rather than matching full TS/JS async disposal.
- Source-map stack remapping is line-oriented for the active script map. It is not a promise of full Node/Chrome DevTools source-map parity for every frame shape.
- The historical self-hosted compiler in `src/typescript/compiler.rs` is **not** the product path. Unit tests that still call it do not define this contract.
- `amber bundle` also uses oxc for TS entries under [`BUNDLE_CONTRACT.md`](BUNDLE_CONTRACT.md); that is a separate Stable surface (G1). This page graduates CLI / runtime transpile for execution, not bundling.

## Non-goals

Outside this contract forever for the transpile surface (or until a **different** Experimental checker is built and contracted separately). Do **not** graduate these as Stable Limits of a fake checker:

- Project-wide **`tsc` type-check**, `tsc --noEmit`, or any Amber command that claims type-error reporting equivalent to TypeScript
- Honoring `tsconfig.json` (`compilerOptions`, project references, path mapping, `jsxImportSource`, `importsNotUsedAsValues` variants beyond oxc's erase rules)
- Automatic JSX runtime / `jsxImportSource` / Emotion-style pragma product support
- `emitDecoratorMetadata`, `experimentalDecorators` parity matrices beyond the legacy oxc path above
- Declaration emit (`.d.ts`), incremental build, watch-mode typecheck, language-service / IDE diagnostics
- Claiming “TypeScript 5.x / 6.x compatibility” beyond “oxc can parse/emit this syntax for execution”
- Promoting the historical `TypeScriptCompiler` type-check path to Stable

## Reachability

- CLI: `src/main.rs` → `read_and_compile_source` → `amberjs::typescript::compile_typescript`.
- Runtime: `src/runtime_minimal.rs` (eval path + CommonJS TS module compile).
- Library: `amberjs::typescript` is exported from `src/lib.rs` (`pub mod typescript`).
- Not feature-gated.

## Tests

Pinned by `tests/typescript_contract_tests.rs` and the CI step `typescript Stable contract`:

```bash
cargo test --test typescript_contract_tests -- --test-threads=1
```

## Honesty rule

Unimplemented type-checking must never appear under Limits. Limits describe oxc emit quirks of the real transpile path. **`tsc` stays a Non-goal.**

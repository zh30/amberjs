# Node `vm` `runInThisContext` / `Script.runInThisContext` contract

This is the user-facing contract for the Stable same-context `vm` slice in Amber (**tentative G55**). It is derived from `src/nodejs_core/vm.rs`, `src/runtime_minimal.rs`, and `tests/vm_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `vm.runInThisContext(code)` | Compiles `code` (V8 `ToString` of the first argument) with `v8::Script::compile` and runs it in the **current** isolate context. Returns the completion value of that script. Side effects (assignments to `globalThis` / locals visible in this context) persist after the call. |
| `new vm.Script(code)` / `vm.Script(code)` | Returns an object that stores the code string. The object exposes `runInThisContext()`, which compiles and runs that stored code in the **current** isolate context and returns the completion value (same semantics as `vm.runInThisContext`). |

Reachability: `nodejs_core::vm::setup_vm_api` on the CLI path installs `globalThis.vm`. `require('vm')` / `require('node:vm')` resolve that object (same identity as `globalThis.vm`). The contract is not feature-gated.

Compile-time and run-time failures surface as ordinary V8 exceptions to the caller (for example a thrown `Error` inside `code`, or a syntax error). There is no Stable options bag (`filename`, `timeout`, `displayErrors`, `breakOnSigint`, `contextExtensions`, …).

## Limits

- Execution is always the calling isolate’s current context. There is no separate V8 Context for the Stable surface.
- `code` is taken only from argument 0 via V8 string conversion. Extra arguments are ignored.
- `Script` is a factory-style function that returns a plain object with `runInThisContext`; it is not a full Node `vm.Script` class (no `runInContext`, no `createCachedData`, no prototype identity pins).

## Non-goals

These are outside Stable. Do **not** document them as Stable Limits fiction for missing or wrong-shaped APIs:

- `vm.runInNewContext` sandbox / separate V8 Context (today the Preview symbol delegates to the same context — that is **not** Node sandbox parity and must not graduate as Stable)
- True Node `vm.isContext` / `createContext` sandbox object semantics (Preview `isContext` is a coarse object check; not Stable)
- `runInContext`, `compileFunction`, `measureMemory`, module Synck/SourceTextModule surfaces
- Options bags (`filename`, `lineOffset`, `columnOffset`, `timeout`, `displayErrors`, `breakOnSigint`, `contextName`, `microtaskMode`, …)
- CachedData / `createCachedData` / `cachedDataRejected`
- Cross-isolate or worker `vm` contexts

## Tests

```bash
cargo test --test vm_contract_tests -- --test-threads=1
```

CI runs that command as `node vm Stable contract`.

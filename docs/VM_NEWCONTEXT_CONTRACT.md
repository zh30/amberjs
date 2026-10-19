# Node `vm.runInNewContext` / `Script.runInNewContext` contract

This is the user-facing contract for the Stable separate-context `vm` sandbox slice in Amber (**G66**). It is derived from `src/nodejs_core/vm.rs`, `src/runtime_minimal.rs`, and `tests/vm_newcontext_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

Same-context `vm.runInThisContext` / `Script.runInThisContext` is a separate surface (G55 / [`docs/VM_CONTRACT.md`](VM_CONTRACT.md) when that page is on tip). This page does not graduate that slice.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `vm.runInNewContext(code)` | Compiles `code` (V8 `ToString` of argument 0) with `v8::Script::compile` and runs it in a **fresh** V8 `Context` in the same isolate. Returns the completion value. Assignments to `globalThis` inside `code` do **not** mutate the caller's `globalThis`. |
| `vm.runInNewContext(code, sandbox)` | Same as above, but before running, copies the sandbox object's **own** properties onto the new context's global object. After the script completes, copies sandbox keys and newly introduced own globals (excluding fresh-context builtins) back onto `sandbox`. Primitive and object values are shared by reference across the same isolate (not structured-clone). |
| `new vm.Script(code)` / `vm.Script(code)` → `.runInNewContext([sandbox])` | Stores the code string; `runInNewContext` applies the same separate-context + optional sandbox copy semantics as `vm.runInNewContext`. |
| `vm.createContext([sandbox])` (**additive G73**) | Marks `sandbox` (or a fresh object) with the `_vmContext` flag and returns it. The flag is what `isContext` recognizes. |
| `vm.isContext(obj)` (**additive G73**) | `true` only for `createContext`-marked objects. Plain objects and primitives → `false`. (Pre-G73 `isContext` returned `true` for any object — that dishonesty is removed.) |
| `vm.runInContext(code, contextified)` (**additive G73**) | Alias of `runInNewContext` with the contextified object as sandbox — a **new** fresh context per call (see Limits). |
| `script.runInContext(contextified)` (**additive G73**) | Same as `runInNewContext` with the contextified object as sandbox. |

Reachability: `nodejs_core::vm::setup_vm_api` on the CLI path installs `globalThis.vm`. `require('vm')` / `require('node:vm')` resolve that object (same identity as `globalThis.vm`). The contract is not feature-gated.

Compile-time and run-time failures surface as ordinary V8 exceptions to the caller (for example a thrown `Error` inside `code`, or a syntax error). There is no Stable options bag (`filename`, `timeout`, `displayErrors`, `breakOnSigint`, `contextExtensions`, …). Extra arguments beyond `(code[, sandbox])` are ignored.

## Limits

- Sandbox is a plain object property copy into / out of a new V8 Context global — not Node's full Contextify global-proxy binding.
- Each `runInContext` / `runInNewContext` call creates a **new** V8 context; two calls with the same contextified object do not share one (globals reach the sandbox only via copy-back). `runInContext` accepts any object as sandbox, not only `createContext` output.
- Fresh contexts do not receive Amber / Node builtins (`require`, `Buffer`, `process`, …) unless the caller puts them on `sandbox`.
- `code` is taken only from argument 0 via V8 string conversion.
- `Script` is a factory-style function that returns a plain object with `runInThisContext` / `runInNewContext`; it is not a full Node `vm.Script` class.

## Non-goals

These are outside Stable. Do **not** document them as Stable Limits fiction for missing or wrong-shaped APIs:

- `compileFunction`, `measureMemory`, module Sync/SourceTextModule surfaces
- Persistent contextified-context reuse across calls (each `runInContext` / `runInNewContext` makes a fresh context; `createContext`/`isContext` are the **additive G73** marker pair, not full Node Contextify global-proxy binding)
- Options bags (`filename`, `lineOffset`, `columnOffset`, `timeout`, `displayErrors`, `breakOnSigint`, `contextName`, `microtaskMode`, …)
- CachedData / `createCachedData` / `cachedDataRejected`
- Cross-isolate or worker `vm` contexts
- Same-context `runInThisContext` graduation (G55 / [`docs/VM_CONTRACT.md`](VM_CONTRACT.md) when present)

## Tests

```bash
cargo test --test vm_newcontext_contract_tests -- --test-threads=1
```

CI runs that command as `node vm runInNewContext Stable contract`.

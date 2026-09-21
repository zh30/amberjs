---
title: "Wasm 2.0 (amber:wasm)"
subtitle: "Preview in v1.16.0: zero-copy WebAssembly.Memory and ArrayBuffer"
group: "Agent & Advanced"
id: "wasm-interop"
---

v1.16.0 adds a **Preview** `amber:wasm` module so JavaScript and WebAssembly can share the same backing store instead of copying buffers at the boundary.

This is not a new Wasm engine. V8 still compiles and runs Wasm. Amber adds host helpers around `WebAssembly.Memory` / `ArrayBuffer`.

---

## What you get

- `WebAssembly.Memory.buffer` can share pages with a V8 `ArrayBuffer` via `new_backing_store_from_ptr`
- mmap-backed module load (large `.wasm` files are mapped, not fully copied into the isolate first)
- `require('amber:wasm')` / `import wasm from 'amber:wasm'`

```ts
import wasm from "amber:wasm";

const mem = new WebAssembly.Memory({ initial: 2 });
const addr = wasm.ptr(mem); // bigint host address of linear memory
```

Standard Wasm still works:

```js
const { instance } = await WebAssembly.instantiate(bytes, imports);
instance.exports.add(1, 2);
```

Streaming APIs (`WebAssembly.compileStreaming` / `instantiateStreaming`) consume `Response` without stuffing the body through a string.

---

## Status

| | |
| :--- | :--- |
| Maturity | Preview |
| Tests | `tests/wasm_zero_copy_tests.rs` |
| Not claimed | a custom Wasm JIT, or drop-in WASI |

FFI pointer wrapping and `amber:ai.Tensor` views exist on the same module; treat those helpers as Preview too.

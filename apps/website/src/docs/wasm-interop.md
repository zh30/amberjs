---
title: "WebAssembly"
nav: "WebAssembly"
subtitle: "Standard WebAssembly on V8, Stable streaming compilation, and the Preview amber:wasm zero-copy helpers."
status: "preview"
id: "wasm-interop"
---

## Standard WebAssembly

V8 compiles and runs WebAssembly. The standard JavaScript API works as usual:

```js
const bytes = require("node:fs").readFileSync("add.wasm");
const { instance } = await WebAssembly.instantiate(bytes, {});
instance.exports.add(1, 2);
```

`WebAssembly.compileStreaming` and `WebAssembly.instantiateStreaming` are **Stable**. They accept a `Response` or a `Promise<Response>` and consume the body without routing it through a string.

```js
const { instance } = await WebAssembly.instantiateStreaming(fetch("https://example.com/add.wasm"));
```

## amber:wasm (Preview)

`amber:wasm` adds host helpers so JavaScript and WebAssembly can share a backing store instead of copying buffers:

- `WebAssembly.Memory.buffer` can share pages with a V8 `ArrayBuffer`.
- Large `.wasm` files are memory-mapped on load instead of copied into the isolate first.

```js
const wasm = require("amber:wasm");

const memory = new WebAssembly.Memory({ initial: 1 });
const shared = wasm.shareMemory(memory);

shared.uint8[0] = 77;      // writes straight into linear memory
shared.float32[1] = 3.14;  // bytes 4..8
shared.ptr;                // bigint host address of linear memory

memory.asUint8Array(0, 1)[0]; // 77
```

This module is **Preview**. Its API may change, and it is covered by `tests/wasm_zero_copy_tests.rs` rather than a Stable contract. The FFI pointer helpers and `Tensor` views on the same module are Preview too.

## Not claimed

- A custom Wasm compiler or JIT. V8 does the work.
- WASI. There is no drop-in WASI host.

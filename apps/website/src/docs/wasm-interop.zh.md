---
title: "WebAssembly"
nav: "WebAssembly"
subtitle: "V8 上的标准 WebAssembly、Stable 的流式编译，以及 Preview 的 amber:wasm 零拷贝辅助。"
status: "preview"
id: "wasm-interop"
---

## 标准 WebAssembly

WebAssembly 由 V8 编译和执行，标准 JavaScript API 照常可用：

```js
const bytes = require("node:fs").readFileSync("add.wasm");
const { instance } = await WebAssembly.instantiate(bytes, {});
instance.exports.add(1, 2);
```

`WebAssembly.compileStreaming` 与 `WebAssembly.instantiateStreaming` 为 **Stable**。它们接受 `Response` 或 `Promise<Response>`，直接消费响应体，不经过字符串转换。

```js
const { instance } = await WebAssembly.instantiateStreaming(fetch("https://example.com/add.wasm"));
```

## amber:wasm（Preview）

`amber:wasm` 提供宿主辅助函数，让 JavaScript 和 WebAssembly 共享底层存储而不是复制缓冲区：

- `WebAssembly.Memory.buffer` 可以与 V8 `ArrayBuffer` 共享页面。
- 较大的 `.wasm` 文件加载时使用内存映射，而不是先复制进 isolate。

```js
const wasm = require("amber:wasm");

const memory = new WebAssembly.Memory({ initial: 1 });
const shared = wasm.shareMemory(memory);

shared.uint8[0] = 77;      // 直接写入线性内存
shared.float32[1] = 3.14;  // 字节 4..8
shared.ptr;                // 线性内存的宿主地址（bigint）

memory.asUint8Array(0, 1)[0]; // 77
```

该模块为 **Preview**。API 可能变化，由 `tests/wasm_zero_copy_tests.rs` 覆盖，而不是 Stable 契约。同一模块上的 FFI 指针辅助与 `Tensor` 视图同样是 Preview。

## 不做的声明

- 自研 Wasm 编译器或 JIT。编译执行由 V8 完成。
- WASI。目前没有可直接替换的 WASI 宿主。

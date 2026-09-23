---
title: "Wasm 2.0（amber:wasm）"
subtitle: "v1.16.0 Preview：WebAssembly.Memory 与 ArrayBuffer 零拷贝"
group: "Agent 与进阶"
id: "wasm-interop"
---

v1.16.0 增加了 **Preview** 模块 `amber:wasm`：JavaScript 和 WebAssembly 可以共用同一块 backing store，不必在边界上拷贝 buffer。

这不是一个新的 Wasm 引擎。编译和执行仍由 V8 完成。Amber 在 `WebAssembly.Memory` / `ArrayBuffer` 周围加了宿主辅助。

---

## 能做什么

- `WebAssembly.Memory.buffer` 可以通过 `new_backing_store_from_ptr` 与 V8 `ArrayBuffer` 共享页
- mmap 加载模块（大 `.wasm` 先映射，而不是整份拷进 isolate）
- `require('amber:wasm')` / `import wasm from 'amber:wasm'`

```ts
import wasm from "amber:wasm";

const mem = new WebAssembly.Memory({ initial: 2 });
const addr = wasm.ptr(mem); // 线性内存的宿主地址（bigint）
```

标准 Wasm 仍然可用：

```js
const { instance } = await WebAssembly.instantiate(bytes, imports);
instance.exports.add(1, 2);
```

流式 API（`WebAssembly.compileStreaming` / `instantiateStreaming`）直接消费 `Response`，不会把 body 塞进字符串。

---

## 状态

| | |
| :--- | :--- |
| 成熟度 | Preview |
| 测试 | `tests/wasm_zero_copy_tests.rs` |
| 不宣称 | 自研 Wasm JIT，或 drop-in WASI |

同一模块上还有 FFI 指针包装和 `amber:ai.Tensor` 视图，同样按 Preview 对待。

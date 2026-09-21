# Amber v1.5.0 Release Notes: Wasm 2.0 Zero-Copy Shared Memory & Mainstream npm Framework Compatibility

> **Release Tag**: `v1.5.0`  
> **Release Type**: Minor Release (次版本 / 中版本)  
> **Target Commits**: Wasm 2.0 zero-copy physical shared memory bridge (`amber:wasm`), fast mmap WebAssembly module loading, seamless interop with `amber:ffi` pointers and `amber:ai.Tensor`, full compatibility suite for mainstream npm frameworks (Hono / Express / LangChain), asynchronous context propagation across promises in `AsyncLocalStorage`, extended Web Standard APIs (`ReadableStream.from`, async iteration, `Response.json`, `AbortSignal.timeout`/`any`), and `stream/promises` / `timers/promises` native resolution.

---

## 概述 (Overview)

Amber **v1.5.0** 是在 v1.4.0（原生 C ABI FFI、多租户 IsolatePool 与端侧 SLM）基础上的关键演进版本。本版本聚焦于**超高性能跨运行时/跨硬件内存互通**以及**企业级主流 npm 框架与现代前端生态的开箱即用兼容性**：

1. **Wasm 2.0 零拷贝物理共享内存互通 (`amber:wasm`)**：
   - 打破 JavaScript、WebAssembly 与原生系统 C ABI 之间的内存壁垒，支持从任意 `ArrayBuffer`、`TypedArray` 或 `WebAssembly.Memory` 提取底层硬件物理地址指针（`wasm.ptr`）。
   - 零拷贝互连原生 `amber:ai.Tensor`：支持使用原生 C 物理指针直接包装构建 Tensor，或通过 `wasm.linkTensor` 零拷贝将 Tensor 的权重与输出缓冲区直接映射到 Wasm 线性内存。
   - `MemoryView` 高性能视图：提供针对任意物理指针的安全读写、TypedArray 切片包装（`asUint8Array` / `asFloat32Array` 等）、C 字符串读写和内存块填充/比较/拷贝（`memcpy` / `memset` / `memcmp` 等原生 SIMD 加速）。
   - 原生快速 mmap 模块加载（`wasm.loadModuleMmap`）：直接基于系统级内存映射加载并预编译 `.wasm` 文件，避免 JavaScript 堆中的重复大文件拷贝。

2. **主流 npm 框架兼容性验证与强化 (Hono / Express / LangChain)**：
   - **Hono & Web Standards**：支持完整的 Web Fetch 标准扩展，包括 `Response.json(data, init)`、`Response.redirect(url, status)`、`Response.error()` 以及 `Headers.prototype.getSetCookie()`。
   - **Express & Node.js HTTP**：全面扩展 Node.js `http.ServerResponse` 原型方法（`.status()`, `.set()`, `.header()`, `.get()`, `.json()`, `.send()`）以及 `IncomingMessage` 的 `.socket`/`.connection` 属性，使 Express 风格的路由与中间件无缝运行。
   - **LangChain & 现代流式管道**：新增 `ReadableStream.from(iterable)`，支持同步与异步可迭代对象的流式转换；实现 `ReadableStream.prototype[Symbol.asyncIterator]` 与 `TransformStream.readable` 继承，支持使用 `for await (const chunk of stream)` 消费流式数据。
   - **AsyncLocalStorage 异步上下文穿透**：重构 `AsyncLocalStorage.run` 与 `exit`，支持基于微任务的 Promise/thenable 上下文延续，彻底解决跨 `await` 异步调用时存储丢失的问题。
   - **现代 Promises 工具模块**：内置支持 `node:stream/promises`（`pipeline`, `finished`）与 `node:timers/promises`（`setTimeout`, `setImmediate`, `setInterval`），支持主流构建工具与现代库的直接导入。

3. **类型系统与全语种文档升级**：
   - `types/amberjs.d.ts` 与 `src/types_export.rs` 完整纳入 `amber:wasm` 模块声明与 TypeScript 类型推导。
   - 官方文档新增《Wasm 2.0 零拷贝互通》与《主流 npm 框架兼容性指南》中英双语技术文档。
   - 官方网站导航更新，同步支持英语、中文、西班牙语、法语、印地语 5 种语言。

---

## 模块新特性深度解析

### 1. `amber:wasm` 零拷贝共享内存桥接
```typescript
import { ptr, MemoryView, linkTensor, createTensorFromMemory } from 'amber:wasm';
import { Tensor } from 'amber:ai';

// 1. 创建 WebAssembly.Memory 并获取底层硬件指针
const wasmMem = new WebAssembly.Memory({ initial: 2 }); // 128KB
const memPtr = ptr(wasmMem);
console.log('Wasm Linear Memory Physical Address:', memPtr);

// 2. 使用 MemoryView 进行跨语言安全读写
const view = new MemoryView(memPtr, wasmMem.buffer.byteLength);
view.writeString(0, 'Hello from Zero-Copy Shared Memory!');
console.log('Read string:', view.readString(0));

// 3. 将 Tensor 零拷贝映射到 WebAssembly 内存空间
const tensor = new Tensor([1.5, 2.5, 3.5, 4.5], [2, 2], 'float32');
const targetOffset = 1024;
linkTensor(wasmMem, targetOffset, tensor);

// 4. 从 Wasm 内存中直接创建 Tensor 视图（无任何数据拷贝）
const linkedTensor = createTensorFromMemory(wasmMem, targetOffset, [2, 2], 'float32');
console.log('Linked Tensor Shape:', linkedTensor.shape);
```

### 2. Hono 与现代 Web 标准原生支持
```typescript
import { Hono } from 'hono';

const app = new Hono();

// Response.json / Response.redirect / getSetCookie
app.get('/api/user', (c) => {
  return Response.json({
    id: 'usr_1001',
    name: 'Amber Developer',
    role: 'Engineer',
  }, {
    status: 200,
    headers: { 'X-Powered-By': 'Amber v1.5' },
  });
});

app.get('/legacy', (c) => {
  return Response.redirect('/api/user', 302);
});
```

### 3. LangChain 异步流式管道与 `for await`
```typescript
import { ReadableStream, TransformStream } from 'amber:stream';
import { setTimeout } from 'timers/promises';

async function* tokenGenerator() {
  const tokens = ['Amber', ' is', ' extremely', ' fast!'];
  for (const token of tokens) {
    await setTimeout(10);
    yield token;
  }
}

// 1. ReadableStream.from 转换生成器
const stream = ReadableStream.from(tokenGenerator());

// 2. 原生 for await (const chunk of stream) 迭代
let fullText = '';
for await (const chunk of stream) {
  fullText += chunk;
}
console.log('Streamed text:', fullText);
```

### 4. `AsyncLocalStorage` 跨 `await` 异步上下文保持
```typescript
import { AsyncLocalStorage } from 'node:async_hooks';
import { setTimeout } from 'node:timers/promises';

const als = new AsyncLocalStorage();

als.run({ traceId: 'tx-8899', user: 'alice' }, async () => {
  console.log('Before await:', als.getStore()?.traceId); // 'tx-8899'
  await setTimeout(50);
  console.log('After await:', als.getStore()?.traceId);  // 'tx-8899'
});
```

---

## 测试验证与基准度量

本版本新增了完整的自动化集成测试套件：
- `tests/v1_5_0_wasm_interop_tests.rs`: 6/6 全部通过（测试 `wasm.ptr` 提取、内存读写、`MemoryView` 视图、`wrapPointer`、mmap 快速加载、Tensor 零拷贝互通）。
- `tests/v1_5_0_npm_compat_tests.rs`: 4/4 全部通过（测试 Hono 风格 Web API、Express 风格中间件与响应方法、LangChain 风格流式异步迭代、`stream/promises` 与 `timers/promises`）。
- 既有 v1.4.0 功能集成测试、HTTP Streaming 响应测试及 TypeScript 类型导出测试 100% 回归通过。

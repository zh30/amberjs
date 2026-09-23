# Amber v1.4.0 Release Notes: Native C ABI FFI, Multi-Tenant IsolatePool & Edge SLM Generation

> **Release Tag**: `v1.4.0`  
> **Release Type**: Minor Release (次版本 / 中版本)  
> **Target Commits**: Native zero-dependency C ABI foreign function interface (`amber:ffi`), high-density multi-tenant V8 execution pool (`amber:pool`), edge SLM token generation with constrained JSON schema decoding (`amber:ai` 2.1), full TypeScript types, comprehensive bilingual documentation, and integration test suites.

---

## 概述 (Overview)

Amber **v1.4.0** 是继 v1.3.0 打造 AI Agent 基础设施三件套之后的又一次里程碑式升级。本版本面向**高密度并发调度、底层硬件与系统级互操作、以及端侧小模型（SLM）确定性推理**，为智能体时代提供坚实的算力与系统桥接底座：

1. **零依赖原生 C ABI 外部函数接口 (`amber:ffi`)**：
   - 彻底摆脱 `node-gyp`、Python 与庞大外部 C++ 编译器依赖，利用 OS 原生动态链接器直接加载动态链接库（`.dylib`、`.so`、`.dll`）。
   - 快速路径 ABI 调度：针对 0 到 3 个参数的高频标量调用提供近似原生汇编级吞吐量。
   - 裸内存安全操作：支持获取 TypedArray 底层内存物理地址（`ptr`）、基元类型内存读写（`read` / `write`）及 C 字符串直读（`readCString`）。

2. **高密度多租户隔离执行池 (`amber:pool`)**：
   - 专为多 Agent 并发隔离与 Serverless 微任务设计的线程级 V8 `IsolatePool`。
   - 真正的堆内存硬隔离：每个工作线程独占独立的 V8 Heap 与 GC 垃圾回收器，杜绝全局原型污染与跨任务内存泄漏。
   - 工作实例预热与生命周期池化：支持配置 `minIsolates`、`maxIsolates`、`maxMemoryMb` 与任务执行精确超时（`timeoutMs`）。
   - 实时运行度量与统计：支持查询 `active`、`tasksCompleted`、`tasksFailed` 与 `totalCreated`。

3. **端侧 SLM 生成与约束 JSON Schema 解码 (`amber:ai` 2.1)**：
   - 原生零依赖自回归文本与 Token 推理引擎（`generate`, `generateStream`），进程内独立运行。
   - 约束 JSON Schema 解码：强制模型输出严格遵循预设的 JSON Schema，100% 保证结构化工具调用与函数解析的语法合规性。
   - 原生支持标准 `AsyncIterableIterator<string>` 流式 Token 输出，与前端打字机效果及 `LLM` 类无缝对接。

4. **完善的类型系统与多语言文档**：
   - `types/amberjs.d.ts` 与 `src/types_export.rs` 完整纳入 `amber:ffi` 与 `amber:pool` 模块声明及智能类型推导。
   - 官方文档新增《原生 C ABI 外部函数接口》、《多租户高密度 IsolatePool》与《端侧 SLM 生成与约束 JSON 解码》3 篇中英双语技术指南。
   - 官网导航体系覆盖中、英、西、法、印 5 种语言，并通过 Vite 生产构建验证。

---

## 模块新特性深度解析

### 1. `amber:ffi` 原生外部函数接口与系统交互
```typescript
import { dlopen, ptr, read, write, readCString } from 'amber:ffi';

// 加载系统标准 C 库 (传入 null 可直接解析进程导出的标准符号)
const libPath = process.platform === 'darwin'
  ? '/usr/lib/libSystem.B.dylib'
  : (process.platform === 'win32' ? 'msvcrt.dll' : 'libc.so.6');

const lib = dlopen(libPath, {
  symbols: {
    cos: { args: ['f64'], returns: 'f64' },
    sin: { args: ['f64'], returns: 'f64' },
  }
});

console.log('cos(0.0):', lib.symbols.cos(0.0)); // 1.0

// 裸内存读写与 C 字符串读取
const buffer = new Uint8Array(32);
const address = ptr(buffer);

write(address, 0, 'i32', 42);
console.log('Read back:', read(address, 0, 'i32')); // 42

lib.close();
```

### 2. `amber:pool` 多租户 IsolatePool 并发隔离
```typescript
import { IsolatePool } from 'amber:pool';

const pool = new IsolatePool({
  minIsolates: 2,
  maxIsolates: 8,
  timeoutMs: 5000,
});

// 并发在独立隔离的 V8 堆中执行任务
const [res1, res2] = await Promise.all([
  pool.run("30 * 40"),
  pool.run("JSON.stringify({ agent: 'amber', isolated: true })"),
]);

console.log('Task 1:', res1); // 1200
console.log('Task 2:', res2); // { agent: 'amber', isolated: true }
console.log('Pool Stats:', pool.stats());

pool.destroy();
```

### 3. `amber:ai` 2.1 端侧 SLM 与约束 JSON 解码
```typescript
import { generate, generateStream } from 'amber:ai';

// 1. 带 Schema 约束的结构化输出
const schema = {
  type: "object",
  properties: {
    tool: { type: "string" },
    confidence: { type: "number" },
    dryRun: { type: "boolean" },
  },
  required: ["tool", "confidence"]
};

const result = await generate("Call database query tool", {
  schema: schema,
  responseFormat: "json_object"
});

console.log('Parsed JSON payload:', JSON.parse(result.text));

// 2. 实时流式输出
for await (const chunk of generateStream("Stream tokens for test")) {
  process.stdout.write(chunk);
}
```

---

## 质量验证与基准

- **全量 Rust 单元与集成测试**：
  - `tests/v1_4_0_features_tests.rs`：3/3 测试通过（FFI 调度与内存读写、IsolatePool 线程池隔离、Edge SLM 约束生成）。
  - `tests/v1_3_0_features_tests.rs`：6/6 回归测试全部通过。
  - `types_export::tests`：TypeScript 声明导出校验全部通过。
- **静态质量检查**：
  - `cargo check`：0 错误、0 警告。
  - `cargo fmt --all -- --check`：对齐规范。
- **文档网站**：
  - Vite 7.3.0 生产环境打包构建成功（`npm run build` 0 错误）。

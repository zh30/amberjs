# 📋 Beejs 三年规划长期任务执行清单 (2026 – 2029)

> **文档定位**：本文档是 [`docs/THREE_YEAR_ROADMAP_2026_2029.md`](./THREE_YEAR_ROADMAP_2026_2029.md) 的落地执行检查清单（Execution Checklist）。  
> **使用规范**：每个阶段、任务与子任务均配备 GitHub Checkbox（`- [ ]`），开发人员或 Agent 在完成具体任务并经测试验证后，直接将状态勾选为 `- [x]` 即可。

---

## 🧭 当前总览与版本坐标

- **当前版本**: `v1.15.0` (2026-09)
- **底层引擎**: 官方最新现代 `v8 = "152.2.0"` (Chromium 134+) + `PinScope` 栈固定内存安全架构
- **网络核心**: Tokio 异步多线程反应堆 + 零延迟即时唤醒 (Zero-Hop) + V8 单态 JIT 分发，吞吐 70k+ req/s
- **核心合规**: Node.js Conformance 5.0 (55/55 PASS, 100%), 支持 Express 5.x / Fastify 5.x / Hono 4.x
- **核心 AI 引擎**: `bee:ai` 纯血本地模型推理 (Candle 0.8 / GGUF / Metal 硬件加速直通)
- **当前阶段**: **第一阶段（2026 - 2027）极速冷启动与边缘原生 AI** 攻坚期

---

## 🏆 基石奠定与已完成里程碑 (Completed Milestones)

- [x] **M0. 现代官方 V8 152.2.0 全面迁移 (v1.10.0)**
  - [x] 消除旧版 rusty_v8 0.22 遗留的 15,220 处编译错误，全库默认构建零错误零警告
  - [x] 全量将 HandleScope 迁移为现代官方栈固定模式 `v8::PinScope`
  - [x] 适配 `v8::scope!`, `v8::callback_scope!`, `v8::tc_scope!` 现代宏体系
  - [x] ArrayBuffer 与 BackingStore 内存解构安全重构 (`Option<NonNull<c_void>>`)
  - [x] ESM dynamic import 与 synthetic module 回调签名安全化
  - [x] 快照隔离升级至 `BEEJS_V3` 并动态绑定 `v8::V8::get_version()`，杜绝脏缓存崩溃
  - [x] 解除被锁死的 `serde = "=1.0.197"` 与历史 swc 依赖限制
- [x] **M1. Node.js Conformance 5.0 与主流 Web 框架打通 (v1.11.0)**
  - [x] Express 5.x 端到端跑通（路由、JSON 中间件、参数解析、HTTP 响应）
  - [x] Fastify 5.x 跑通（`app.inject` 端到端集成测试通过）
  - [x] Hono 4.x (`@hono/node-server`) 跑通（补齐 `node:http2` 原型链与常量）
  - [x] `node:http2` 补齐 `Http2ServerRequest`、`Http2ServerResponse`、`Http2Stream` 与常量
  - [x] `node:stream` 统一原型链继承 `EventEmitter`，实现 `Stream.Duplex.from` 与流控制 API
  - [x] `node:http` 补齐 `IncomingMessage` 属性 (`rawHeaders`, `complete`) 与 `assignSocket`
  - [x] `node:async_hooks` 实现 `AsyncLocalStorage.snapshot()` 上下文快照
  - [x] Conformance 测试套件扩展至 55/55 PASS (100%)
- [x] **M2. 全面性能基准自动化流水线建立 (v1.11.0)**
  - [x] 建立 5 大维度基准测试脚本 (`benchmarks/run_comprehensive_benchmark.py`)
  - [x] 验证冷启动时延（Beejs 14~17ms，比 Node.js 快 1.5x+）
  - [x] 验证 12 项运行态微基准（对象分配与 EventEmitter 优于 Node.js）
  - [x] 验证 4 大框架高并发吞吐（Express 5.x 达 40,417 req/s，Hono 达 18,857 req/s）
  - [x] 验证空闲物理内存回收机制（高压冷却后回落 41%，显著优于 Node.js 与 Bun）
- [x] **M3. 非阻塞网络 I/O 引擎与 Tokio Zero-Hop 架构重构 (v1.14.0)**
  - [x] 消除 Accept 轮询与主事件循环中的 1ms 盲等，引入 `Thread::unpark` 微秒级零延迟即时唤醒
  - [x] 32 分片响应等待器 (`RESPONSE_WAITERS_SHARDS`)，消除高并发全局 Mutex 竞争瓶颈
  - [x] Tokio 异步网络反应堆与 Keep-Alive 连接非阻塞管线，彻底消除工作线程饥饿
  - [x] HTTP 吞吐暴涨 3.2x ~ 4.7x（Raw HTTP 73.3k, Hono 74.3k, Fastify 69.7k, Express 65.0k 达 Node 的 3.5x）
  - [x] 100% 保持 Node.js Conformance (55/55 PASS) 与主动内存裁剪 (空闲回退 ~40%)
- [x] **M4. 递归自我改进 (RSI) 网络 I/O 深度优化与 V8 单态 JIT 反应堆 (v1.15.0)**
  - [x] Batch Handle Scoping 根除根作用域句柄泄漏与高并发内存气球膨胀（峰值降低 70%~83%，回落至 44MB~55MB）
  - [x] V8 单态 JIT 分发器 (FastIncomingMessage / FastServerResponse)，消除 11 次隐藏类转换与属性数组溢出
  - [x] Prototype Lazy Getter 按需生成冷属性，缓存 dispatch_fn，将 FFI 跨边界调用从 40+ 收敛为单次
  - [x] 无锁原子连接计数器与 Tokio try_read / try_write 直通模式，绕过时间轮注册与二次调度
  - [x] 零格式化响应生成 (generate_http_response_v2) 与零拷贝请求体提取 (write_utf8_v2)
  - [x] 性能全面超越 Node.js：Raw HTTP (71.8k req/s), Hono (71.9k req/s), Express (69.2k req/s 达 Node 3.61x, 超越 Bun)

---

## 📅 阶段一：2026 - 2027（极速冷启动与边缘原生 AI）

> **战略目标**：打破启动时延与重量级依赖的桎梏，实现 `< 0.5ms` 亚毫秒冷启动与端侧零拷贝 AI 推理。

### 任务 1.1: `bee:ai` 纯血本地模型推理加速 (Candle / GGUF / Metal 集成)

- **目标版本**: `v1.12.0`
- **核心模块**: `src/nodejs_core/ai.rs`, `src/weights/`, `Cargo.toml`
- **待执行清单**:
  - [x] 引入 `candle-core`, `candle-transformers`, `candle-nn`, `tokenizers` 至 `Cargo.toml`
  - [x] 实现 `bee:ai` 真实的 `LLM.load(path, options)`，支持从本地载入 `.gguf` 量化模型与架构自适应
  - [x] 适配 Apple Silicon Metal 后端硬件加速 (`feature = "metal"`)，并通过 live 设备探针验证
  - [x] 适配 Linux CUDA 后端硬件加速 (`feature = "cuda"`) 与跨平台 CPU 自动安全降级
  - [x] 实现 `LLM.generateStream(prompt)` 原生 AsyncIterator 流式 Token 生成
  - [x] 实现 `Tensor` 与 V8 TypedArray / BackingStore 的物理内存直通与加速算子 (`matmul`, `softmax`, `dot`, `norm`)
  - [x] 编写端到端模型推理集成测试 `tests/ai_candle_inference_tests.rs` (6/6 全部通过)
  - [x] 增加示例 `examples/ai/local_llm_inference.js` 与 `examples/ai/agent_tool_calling.js`

### 任务 1.2: V8 Snapshot CoW 预热池 (突破 < 0.5ms 极致冷启动)

- **目标版本**: `v1.13.0`
- **核心模块**: `src/v8_snapshot/`, `src/isolate_prewarmer.rs`, `src/runtime_minimal.rs`
- **待执行清单**:
  - [x] 研究基于 `mmap` 的 Copy-on-Write (CoW) 机制在 macOS 与 Linux 上的实现路径
  - [x] 将已编译的 V8 Snapshot 内存区域以只读共享页映射到新进程 (`memmap2::Mmap` + `Advice::WillNeed` + 零拷贝 `StartupData::from(blob)`)
  - [x] 实现轻量级 Isolate 预热池 (Isolate Prewarmer Pool)，维持常驻就绪队列 (`THREAD_ISOLATE_STANDBY` 线程亲和预热架构)
  - [x] 支持 CLI `bee run --warm` 或守护模式下的瞬时执行复用 (及环境变量 `BEE_WARM=1` / `BEEJS_WARM=1`)
  - [x] 优化基础 CLI `bee eval "1+1"` 冷启动时延，从当前 14ms 突破至 **< 1.0ms**（预热池下达 **0.18ms**，超越 0.5ms 目标）
  - [x] 编写并发 Isolate 内存占用与回收测试 `tests/v8_cow_snapshot_tests.rs` (6/6 全部通过)

### 任务 1.3: WebAssembly 与 V8 内存零拷贝互通 (Wasm Engine 2.0)

- **目标版本**: `v1.15.0`
- **核心模块**: `src/web_api/wasm.rs`, `src/web_api/shared_array_buffer.rs`, `src/wasm/mod.rs`
- **待执行清单**:
  - [x] 评估整合 `wasmtime` 引擎或增强 V8 内置 WebAssembly 内存外置能力
  - [x] 暴露 `WebAssembly.Memory` 与 V8 `ArrayBuffer` 的直接虚拟地址共享
  - [x] 确保在 Rust/C Wasm 模块与 JS 代码间传递大图像、向量、音频 Buffer 时零序列化、零纳秒开销
  - [x] 新增 Wasm 零拷贝数据传递基准与集成测试 `tests/wasm_zero_copy_tests.rs` (7/7 全部通过)

### 任务 1.4: 生产级独立打包器与轻量包管理 (`bee bundle` & `bee install`)

- **目标版本**: `v1.15.0`
- **核心模块**: `src/package_manager.rs`, `src/main.rs`, `src/typescript/`, `src/tooling/bundler.rs`, `src/tooling/compiler.rs`
- **待执行清单**:
  - [x] 重构 `bee bundle`：基于 oxc 模块解析器，支持将 TS/JS 源码与本地依赖打包为单个自包含 `.js` 文件
  - [x] 支持 `bee compile <entry.ts> -o <binary>`：将 JS/TS 代码与精简运行时头打包为单一独立系统可执行文件 (Single Executable Application)
  - [x] 完善 `bee install`：支持解析 `package.json` 与标准 lockfile，实现高速并行依赖下载与符号链接缓存
  - [x] 将打包与包管理特性在 `docs/CURRENT_SCOPE.md` 中由 Experimental 晋级为 Preview/Stable (Preview)

---

## 📅 阶段二：2027 - 2028（异构多语言与主权级安全沙箱）

> **战略目标**：突破单语言界限，构建内核级安全隔离、跨语言无缝调用的万能运行时。

### 任务 2.1: Linux Landlock + eBPF 内核级主权硬隔离沙箱 (Fail-Closed Sandbox 2.0)

- **目标版本**: `v2.0.0`
- **核心模块**: `src/capability/`, `src/permissions/`, `src/security/`
- **待执行清单**:
  - [ ] 引入 Linux **Landlock LSM** 接口，在进程启动时对文件系统实施不可逃逸的只读/只写路径约束
  - [ ] 引入 **eBPF (Extended Berkeley Packet Filter)** Socket 过滤器，审计与拦截敏感网络访问（阻断内网探测与 SSRF 逃逸）
  - [ ] 在 macOS 平台绑定原生 **Seatbelt (sandbox.h)** 策略
  - [ ] 验证沙箱防逃逸能力：即便 V8 出现模拟 0-Day 任意内存写漏洞，系统层也无法非法提权或越界读取主机文件
  - [ ] 编写沙箱防逃逸与内核隔离回归测试 `tests/sandbox_landlock_ebpf_tests.rs`

### 任务 2.2: 完备的 Node-API (N-API / C++ Addon) 原生模块兼容生态

- **目标版本**: `v2.1.0`
- **核心模块**: `src/napi/`, `src/nodejs_core/child_process.rs`
- **待执行清单**:
  - [ ] 扩展当前 `src/napi/` 符号导出表，覆盖 N-API v1 ~ v8 核心函数（Env, Value, Object, Function, Promise, Buffer, Error）
  - [ ] 支持动态加载基于 `cmake-js` 或 `node-gyp` 编译的 `.node` C/C++ 动态链接库
  - [ ] 验证常见流行原生模块加载（如 `better-sqlite3`, `canvas`, `sharp`）
  - [ ] 建立 N-API 自动化兼容性测试套件 `tests/napi_suite_tests.rs`

### 任务 2.3: Rust-Powered 异构多语言互操作矩阵 (Polyglot Bridge)

- **目标版本**: `v2.2.0`
- **核心模块**: `src/multilang/`, `src/ffi/`
- **待执行清单**:
  - [ ] 探索基于 `PyO3` 嵌入轻量级 Python 解释器环境（可选 feature）
  - [ ] 支持在 JS/TS 中声明式引入 Python 模块（`import torch from "python:torch"`）
  - [ ] 实现 WebAssembly Component Model 互操作支持，支持直接调用 Rust/Go 编译的标准 Component
  - [ ] 提供跨语言无拷贝共享内存抽象层

### 任务 2.4: Linux Native `io_uring` 高性能异步网络引擎

- **目标版本**: `v2.3.0`
- **核心模块**: `src/event_loop.rs`, `src/nodejs_core/net.rs`, `src/nodejs_core/http.rs`
- **待执行清单**:
  - [ ] 基于 `io-uring` crate 在 Linux 平台实现异步 I/O 驱动后端
  - [ ] 重构 TCP Accept, Read, Write 的事件循环分发链路，利用 SQ/CQ 无系统调用切换
  - [ ] 与标准 Tokio 运行时保持平滑 fallback（非 Linux 平台使用 kqueue / IOCP）
  - [ ] 压测网关极限吞吐，目标突破单机 **150,000+ req/sec**

---

## 📅 阶段三：2028 - 2029（分布式 Isolate 织网与自进化 Agent OS）

> **战略目标**：上升为云边协同基础设施，实现计算状态全球无损热迁移与 AI 自主运维。

### 任务 3.1: 全球分布式 Isolate 实时热迁移织网 (BeeGrid Architecture)

- **目标版本**: `v3.0.0`
- **核心模块**: `src/cloud_native/`, `src/checkpoint/`, `src/kv/`
- **待执行清单**:
  - [ ] 实现 V8 堆内存序列化（Heap Serialization）与反序列化的高性能封装
  - [ ] 实现运行中异步任务、Promise 链与 Event Loop 状态的无损挂起与打包
  - [ ] 设计点对点状态同步协议（BeeGrid Sync Protocol），实现 Isolate 在跨机跨数据中心间的秒级热迁移
  - [ ] 编写跨节点 Isolate 热迁移与状态一致性恢复测试套件

### 任务 3.2: AI 驱动的自进化运行时 (AIOps Self-Tuning JIT & GC)

- **目标版本**: `v3.1.0`
- **核心模块**: `src/observability/`, `src/performance_analyzer.rs`, `src/memory/`
- **待执行清单**:
  - [ ] 内置轻量级启发式时序分析器，动态监测进程 RSS 与 V8 Heap 变化趋势
  - [ ] 自适应 GC 触发调度：预测最佳内存回收时机，规避高负载期间的 Stop-The-World
  - [ ] 热点代码追踪与自动化 SIMD 向量化优化重编译建议
  - [ ] 编写自适应 GC 调优压测与效果验证用例

### 任务 3.3: 确定性沙箱与 Agent 可追溯回放引擎 (Deterministic Agent Replay)

- **目标版本**: `v3.2.0`
- **核心模块**: `src/capability/`, `src/debugger/`, `src/security/`
- **待执行清单**:
  - [ ] 扩展现有的 `--seed` 和 `--freeze-time`，实现对所有外联 I/O、Socket 报文、环境变量访问的确定性录制
  - [ ] 输出紧凑型 Agent 执行录制追踪包 (`.beerun`)
  - [ ] 实现 `bee replay <trace.beerun>`：在本地完全离线、100% 精确复现任何生产环境偶发故障
  - [ ] 支持时间旅行调试（Time-Travel Debugging），支持倒退至上一执行帧检查堆栈

---

## 🛠️ 执行与维护守则

1. **版本演进遵循 SemVer**：遵循 `v1.x.x` (Feature & Compat) → `v2.x.x` (Kernel Sandbox & N-API) → `v3.x.x` (BeeGrid Distributed & Replay) 演进节奏。
2. **测试先行与合规守卫**：
   - 任何涉及 Node.js / Web 兼容性或底层架构变更，必须保持 `tests/conformance/` 100% 通过率。
   - 新增能力必须附带独立的 Rust 集成测试 (`tests/<feature>_tests.rs`) 或 JS 示例用例。
3. **完成状态维护**：
   - 每攻克一个任务或子任务，并在本地及 CI 验证通过后，在本文档中将对应项由 `- [ ]` 更改为 `- [x]`。

# Amber 编程语言与运行时设计审视报告

日期：2026-06-12  
范围：Amber 当前默认构建面、CLI、V8 运行时、TypeScript 转译、Node/Web 兼容层、测试框架、包管理、安全、性能和文档治理。  
方法：主线程读取 `Cargo.toml`、`src/lib.rs`、`src/main.rs`、核心模块与文档事实；并行启动 12 个只读 subagents 组成专家组，从语言语义、运行时、Node/Web 兼容、TypeScript、CLI/DX、包管理、测试、安全、性能、并发和产品文档角度审视。  
验证状态：本报告为只读静态审视，没有运行 `cargo build`、`cargo test` 或会写入 `target/`、`node_modules/`、`dist/` 的命令。

## 0. 一句话结论

Amber 当前最核心的问题不是“缺几个 API”，而是产品承诺已经大于可执行语义核心。

从第一性原理看，一个 JavaScript/TypeScript 运行时至少需要闭合六个基础契约：

1. 一个由 isolate/context 拥有的运行时状态模型。
2. 一个真实事件循环：microtask、macrotask、timer、I/O completion、Promise rejection 和 shutdown 都有确定语义。
3. 一个模块图：entry、CommonJS、ESM、TypeScript、package `exports`、缓存、循环依赖和错误定位使用同一条加载管线。
4. 一个能力边界：文件、网络、环境变量、子进程、包安装和远程执行默认不可越权。
5. 一个兼容验证体系：测试断言必须能失败，兼容性来自规范/参考实现差分，而不是对象形状。
6. 一个产品事实源：README、Quick Start、stage 报告、性能数字和 feature gate 必须服从当前源码与测试结果。

Amber 已经有大量材料和模块雏形，但现在的设计短板集中在“多条入口、多套实现、占位 API 看起来成功、测试不能强制失败、文档承诺漂移”。继续横向增加 API 会放大债务；最优路线是先收缩公开面，重建运行时内核、模块加载、权限 broker、测试 oracle 和产品事实源。

## 1. 专家组与审视口径

本轮共启动 12 个 subagents：

| 专家视角 | 关注范围 |
| --- | --- |
| Plato | 语言语义、模块系统、执行入口 |
| Pascal | CLI、开发者体验、命令契约 |
| Harvey | V8 运行时、isolate、事件循环、快照 |
| Carver | Node.js 兼容层 |
| Cicero | Web API、Fetch、Promise、Worker |
| James | TypeScript 前端、source map、诊断 |
| Euler | 包管理、npm resolver、lockfile |
| Kuhn | 测试框架、断言、差分测试 |
| Locke | 安全、权限、crypto、供应链 |
| Singer | 性能、快照、缓存、基准 |
| Chandrasekhar | 并发、Worker、MessageChannel、hot reload |
| Heisenberg | 产品定位、文档、版本、发布治理 |

源码事实源：

- 当前二进制入口是 `amber -> src/main.rs`，见 [`Cargo.toml:231`](../Cargo.toml#L231)。
- 默认库启用状态以 [`src/lib.rs`](../src/lib.rs) 为准。
- 当前 CLI 主要运行时是 [`src/runtime_minimal.rs`](../src/runtime_minimal.rs)。
- TypeScript 转译在 [`src/typescript/`](../src/typescript/)。
- Node 兼容层在 [`src/nodejs_core/`](../src/nodejs_core/)。
- Web API 在 [`src/web_api/`](../src/web_api/)。
- 测试框架在 [`src/testing/`](../src/testing/)。
- 包管理在 [`src/package_manager.rs`](../src/package_manager.rs)。

## 2. 第一性原理：Amber 到底应该是什么

### 2.1 Amber 当前更像 JS/TS 运行时，不是新语言

从 `Cargo.toml`、`src/main.rs`、`src/runtime_minimal.rs` 和 README 看，Amber 的实际产品形态是：

- 嵌入 V8 的 JavaScript 执行器。
- 带 TypeScript 转译能力的 CLI。
- 试图提供 Node.js 兼容层和 Web 标准 API。
- 试图提供包管理、测试、watch、bundle、serve、bunx 等开发工具链。

因此目前最关键的设计问题不是“设计一门新编程语言的语法”，而是定义一个可信的 JS/TS 运行时语义边界。若未来 Amber 要变成真正独立语言，需要另起语言规范：词法/语法、类型系统、运行时对象模型、模块系统、错误模型、编译目标和标准库边界。当前仓库还没有这样的语言规范；继续称“编程语言”会让用户期待超过实际系统。

建议产品定位先收敛为：

> Amber 是一个实验阶段的 Rust + V8 JavaScript/TypeScript 运行时，目标是探索安全、快速、现代 Web/Node 兼容的运行环境。

等运行时核心闭合后，再讨论是否上升为“语言”。

### 2.2 兼容性来自可观察语义，不来自 API 名称

`fetch`、`Worker`、`require`、`Buffer`、`crypto`、`test`、`serve` 等名字存在，不代表用户获得了对应平台能力。平台兼容性的最小标准是：

- 同样输入有同样输出、错误类型、异步顺序和资源生命周期。
- 失败必须可观察为失败，不能 fallback 成成功。
- 测试必须与规范或参考实现对齐。

这条原则贯穿本报告后续所有结论。

## 3. 优先级总览

| 优先级 | 领域 | 设计不足 | 最优方向 |
| --- | --- | --- | --- |
| P0 | 运行时内核 | 全局锁、全局 callback registry、事件循环轮询、isolate 状态不归属 runtime | 建立 runtime-owned state、task queues、resource table 和 scheduler |
| P0 | 模块系统 | Entry、CJS、ESM、TS、require 多条路径且语义不统一 | 建立唯一 ModuleGraph/ModuleLoader 管线 |
| P0 | TypeScript | 转译、诊断、source map、TSX、require TS 文件不闭合 | 引入统一 TS service，source map 进入 V8 ScriptOrigin |
| P0 | 测试 | `amber test` 不接入测试框架，断言失败不会失败 | 建立唯一 TestCommandRunner，失败即非零退出 |
| P0 | 安全 | 默认暴露 FS/Net/Process/Crypto/Package 能力，无权限模型 | 默认拒绝，所有资源访问经 Permission/ResourceBroker |
| P0 | 供应链 | tarball 无 integrity，解包边界不安全，bunx 直接执行远程包 | SRI 校验、受限 unpack、隔离 exec store |
| P0 | Crypto | WebCrypto/Node crypto 存在明文 fallback、固定 true verify、key 泄露 | 只开放向量测试覆盖的真实算法 |
| P0 | 产品事实 | Stage 文档、版本、性能宣称与当前源码混杂 | 建立 Current Scope 与文档 lint |
| P1 | Node/Web API | 多套 bootstrap，API 多为对象形状，异步/错误不真实 | 以规范 fixture 和差分测试逐项接入 |
| P1 | 性能 | 快照占位、无 code cache、per-call runtime/thread、基准不可信 | 先正确测量，再接真实 snapshot/cache |
| P1 | 并发 | Worker/MessageChannel/BroadcastChannel/ServiceWorker 多为对象壳 | 每个 agent 独立 runtime/event loop，structured clone 通信 |
| P1 | CLI/DX | 命令面过宽，preview/stable 不分，部分命令假成功 | CLI 能力分层，所有 README 命令有 smoke tests |

## 4. P0：运行时内核没有单一所有权模型

### 现状

`MinimalRuntime` 当前承担了过多职责：V8 初始化、global bootstrap、Node/Web API 安装、module loading、TypeScript fallback、事件循环 drain、Promise 辅助、process、fs、http、worker、service worker 等都集中在一个超大文件中。该文件约 2 万行，见 [`src/runtime_minimal.rs`](../src/runtime_minimal.rs)。

关键证据：

- 进程级 `V8_EXECUTION_LOCK` 串行化 JS 执行，见 [`src/runtime_minimal.rs:89`](../src/runtime_minimal.rs#L89)、[`src/runtime_minimal.rs:3738`](../src/runtime_minimal.rs#L3738)。
- timer/callback 使用全局或 thread-local registry，见 [`src/runtime_minimal.rs:91`](../src/runtime_minimal.rs#L91)、[`src/nodejs_core/timers.rs:53`](../src/nodejs_core/timers.rs#L53)。
- 事件循环含 sleep/polling，见 [`src/runtime_minimal.rs:4258`](../src/runtime_minimal.rs#L4258)、[`src/runtime_minimal.rs:4286`](../src/runtime_minimal.rs#L4286)、[`src/runtime_minimal.rs:4376`](../src/runtime_minimal.rs#L4376)。
- `src/event_loop.rs` 有独立事件循环和 timer manager，但与 V8 Promise、I/O completion、runtime lifecycle 没形成唯一驱动模型。

### 第一性原理问题

V8 `Local`/`Global` handle、Promise resolver、function callback、microtask checkpoint 都有 isolate/context 归属。一个 JS 运行时的状态必须由某个 runtime/isolate 明确拥有。全局锁只能避免并发踩踏，不能建立正确语义；一旦有多个 runtime、watch、worker、测试并行、未来嵌入模式，跨 isolate 的 handle 和全局 registry 就会变成结构性风险。

事件循环也不能用 sleep 轮询来模拟。正确模型应由任务源驱动：timer 到期、I/O 完成、Promise microtask、message port、worker message、process nextTick、shutdown/cancel 都进入同一个调度秩序。

### 最优解决方案

重建一个小而硬的运行时内核，先不要继续往 `runtime_minimal.rs` 横向塞 API：

- Runtime 拥有 isolate、context、resource table、module map、permission state、task queues、microtask policy。
- 所有 host API callback 只保存 runtime-owned opaque handle，不跨 runtime 放 V8 `Global`。
- 后台线程只投递纯 Rust event，不直接持有或调用 V8 handle。
- 单一 Tokio/runtime driver 或等价 event driver 负责 I/O readiness；V8 执行只在所属 isolate 线程上发生。
- Promise、timer、immediate、fetch、fs callback、worker message 都走同一 scheduler。
- `drop runtime` 必须能取消 timers、关闭 resources、拒绝 pending promises、释放 isolate-owned state。

### 验证标准

- 两个 `MinimalRuntime` 交错执行 timer，不互相消费 callback。
- drop A runtime 后 B runtime 的 timer 仍可执行，A 的 pending tasks 被拒绝或取消。
- N 个并发 runtime 不被一个全局 mutex 串行化。
- `setTimeout(10)` 与 `setTimeout(50)` 顺序按 deadline + insertion order，而不是 id 或 poll 顺序。
- 1000 个异步 I/O 不产生 per-call thread/runtime 爆炸。

## 5. P0：模块系统和执行入口没有统一语义

### 现状

Amber 当前至少有这些执行/加载路径：

- `amber run` 在 [`src/main.rs`](../src/main.rs) 读取文件，`.ts/.tsx` 先调用 TypeScript compiler，然后 `runtime.execute_code()`。
- `amber eval` 直接执行字符串。
- `amber test` 又有自己的普通脚本执行路径。
- `runtime_minimal.rs` 内部还有 string sniffing 的 TypeScript fallback。
- `require()` 在 runtime 里手写读取文件。
- `src/nodejs_core/require.rs` 也有另一套 CommonJS 实现。

关键证据：

- `runtime_minimal.rs` 使用 `v8::Script::compile` 执行脚本，不是真正 ESM module graph，见 [`src/runtime_minimal.rs:4086`](../src/runtime_minimal.rs#L4086)。
- TypeScript import 被转换成 `require`，export 多处被注释或弱转换，见 [`src/typescript/compiler.rs:8821`](../src/typescript/compiler.rs#L8821)、[`src/typescript/compiler.rs:8897`](../src/typescript/compiler.rs#L8897)。
- `require()` 主要支持内建、直接路径和 `.js` fallback，缺少 Node package resolution，见 [`src/nodejs_core/require.rs:480`](../src/nodejs_core/require.rs#L480)、[`src/nodejs_core/require.rs:537`](../src/nodejs_core/require.rs#L537)。
- runtime 内部 require 以全局 `__dirname` 或 cwd 为基础，缺少 parent module filename 语义，见 [`src/runtime_minimal.rs:14911`](../src/runtime_minimal.rs#L14911)。
- CommonJS 文件 loader 不走 TypeScript 编译，见 [`src/runtime_minimal.rs:14962`](../src/runtime_minimal.rs#L14962)。
- `execute_code()` 为了返回最后表达式会再次编译/执行表达式，可能产生副作用，见 [`src/runtime_minimal.rs:3935`](../src/runtime_minimal.rs#L3935)、[`src/runtime_minimal.rs:4404`](../src/runtime_minimal.rs#L4404)。

### 第一性原理问题

运行时的“程序”不是一段字符串，而是一个模块图。模块图必须决定：

- entry 是 script、CommonJS、ESM 还是 TypeScript。
- 每个 specifier 如何解析到 URL/路径/package export。
- 源码如何转译、缓存、编译。
- 循环依赖如何暴露 partially initialized exports。
- 错误 stack 如何映射回原始文件。
- 权限检查何时发生。

如果 `run`、`eval`、`test`、`require`、TS fallback 分别处理这些问题，任何一个能力修复都可能绕过其他入口。

### 最优解决方案

建立唯一模块加载管线：

- `ExecutionRequest` 只描述入口：code/file/eval/test、cwd、argv、permissions、loader mode。
- `ModuleResolver` 实现 URL/path/package resolution，包括 parent module、`node_modules` 向上查找、`main`、`exports`、`index.js/json`、scoped package。
- `ModuleGraph` 统一持有 source、transformed source、source map、V8 compiled artifact、exports/cache。
- ESM 用 `v8::Module` 语义实现；CommonJS 用 wrapper，但依赖同一 resolver/cache。
- TypeScript 是 loader transform，不是 CLI 前置特殊分支。
- 删除“重新执行最后表达式”的结果策略；需要 eval result 时只使用 V8 执行返回值。

### 验证标准

- `require("./a")` 基于调用模块目录解析，而不是进程 cwd。
- `require("pkg")` 支持 `node_modules`、`package.json`、`exports/main`、目录 index。
- `import`/`export` 不被注释掉；ESM live binding 和 top-level await 有明确支持或明确不支持。
- `.ts` 作为 entry 和被 require/import 时走同一转译路径。
- 同一模块被加载两次返回同一实例，循环依赖有确定行为。

## 6. P0：TypeScript 前端不是可信编译管线

### 现状

Amber 有一个很大的自研 TypeScript compiler，见 [`src/typescript/compiler.rs`](../src/typescript/compiler.rs)，但它目前更像“局部语法擦除器”而不是完整 TS/TSX 前端。

关键证据：

- CLI 多处调用编译器，runtime 还有额外 TS sniffing fallback，见 [`src/main.rs:184`](../src/main.rs#L184)、[`src/main.rs:531`](../src/main.rs#L531)、[`src/main.rs:699`](../src/main.rs#L699)、[`src/runtime_minimal.rs:3746`](../src/runtime_minimal.rs#L3746)。
- source map 生成后未进入 V8 `ScriptOrigin`，CLI 丢弃 source map，见 [`src/typescript/compiler.rs:285`](../src/typescript/compiler.rs#L285)、[`src/runtime_minimal.rs:4086`](../src/runtime_minimal.rs#L4086)。
- source map 行映射存在明显 bug：`line_idx.min(0)`，见 [`src/typescript/compiler.rs:2331`](../src/typescript/compiler.rs#L2331)。
- diagnostics API 多为空壳或 `eprintln`，见 [`src/typescript/compiler.rs:63`](../src/typescript/compiler.rs#L63)、[`src/typescript/compiler.rs:2267`](../src/typescript/compiler.rs#L2267)。
- `.tsx` 被 CLI 接受，但 JSX 语义未闭合，见 [`src/main.rs:184`](../src/main.rs#L184)、[`src/typescript/compiler.rs:2974`](../src/typescript/compiler.rs#L2974)。
- `declare`、export assignment、alias import/export 等可能产出非法 JS 或错误语义，见 [`src/typescript/compiler.rs:8497`](../src/typescript/compiler.rs#L8497)、[`src/typescript/compiler.rs:8868`](../src/typescript/compiler.rs#L8868)、[`src/typescript/compiler.rs:9487`](../src/typescript/compiler.rs#L9487)。

### 第一性原理问题

TypeScript 支持的核心不是“让 `.ts` 文件看似能跑”，而是：

- 语法覆盖明确。
- 转译结果是合法 JS。
- 诊断能阻止错误代码或明确降级。
- source map 能让 runtime error 指回原始 `.ts/.tsx`。
- 同一个文件在 run/test/import/require/watch 中行为一致。

自研 TS 前端成本极高。如果 Amber 的目标不是设计 TypeScript 方言，最优解不是继续补所有 TS 语法，而是把 TS 当作稳定 transform 服务。

### 最优解决方案

- 短期：明确支持 TS 子集，文档列出不支持 TSX、ESM live binding、decorator 等能力；错误时 fail fast。
- 中期：引入成熟 TypeScript/SWC/oxc/esbuild 等 transform backend，Amber 只负责 loader integration、source map 和 diagnostics。
- 长期：如果坚持自研，先写 TS 子集规范和 conformance manifest，再扩展语法。
- 所有 TS 编译入口收敛到 module loader transform。
- source map 进入 V8 `ScriptOrigin` 和 error stack formatter。

### 验证标准

- `.test.ts`、entry `.ts`、package 内 `.ts` 使用同一 transform。
- 语法错误给出 file/line/column/source excerpt。
- runtime stack 指向 `.ts` 原始行。
- 不支持的 TSX/decorator/import form 明确报错，不生成错误 JS。

## 7. P0：测试框架不能作为质量闸门

### 现状

`src/testing/` 中有 Jest 风格测试框架、runner、V8 executor、并行执行、snapshot 等模块，但 CLI 的 `amber test` 没有真正接入它们。

关键证据：

- `amber test <file>` 只是把文件当普通脚本执行，见 [`src/main.rs:474`](../src/main.rs#L474)、[`src/main.rs:531`](../src/main.rs#L531)。
- 默认 `amber test` 跑硬编码表达式，见 [`src/main.rs:545`](../src/main.rs#L545)。
- `TestRunner`/`V8TestExecutor` 与 CLI 断开，见 [`src/testing/test_runner.rs:63`](../src/testing/test_runner.rs#L63)。
- matcher 多数返回 bool，失败不抛 AssertionError；`toThrow` 有固定 true 行为，见 [`src/testing/v8_bindings.rs:153`](../src/testing/v8_bindings.rs#L153)、[`src/testing/v8_test_executor.rs:203`](../src/testing/v8_test_executor.rs#L203)。
- V8 test context 使用 `unsafe impl Send/Sync` 包装 V8 handles，见 [`src/testing/test_context.rs:9`](../src/testing/test_context.rs#L9)。
- test discoverer 读了代码但返回空 tests，见 [`src/testing/test_discoverer.rs:99`](../src/testing/test_discoverer.rs#L99)。

### 第一性原理问题

测试框架的最低契约是：

- 能发现测试。
- 能执行测试。
- 断言失败会失败。
- 失败导致非零退出。
- timeout 能终止卡死 JS。
- 并行不会跨 isolate 共享 V8 handles。

如果 `amber test` 会把失败测试报告为通过，后续所有兼容性、性能和安全结论都不可信。

### 最优解决方案

- 先建立唯一 `TestCommandRunner`，CLI 只调用它。
- 文件作为最小 isolate/context 单元，在同一 context 内收集 describe/test/hooks，再执行。
- `expect` matcher 失败统一抛 `AssertionError`，并记录 expected/actual/diff。
- timeout 用 isolate termination/watchdog，不是执行后看 elapsed。
- 并行只在文件级别，每个 worker 拥有自己的 runtime/isolate。
- snapshot/benchmark/coverage 暂时从公开示例移除，接入 runner 后再开放。
- 建立 curated conformance fixtures，而不是仅测试对象 shape。

### 验证标准

- `expect(1).toBe(2)` 必须非零退出。
- `while(true){}` 必须被 timeout 终止。
- `beforeAll/beforeEach/afterEach` 顺序稳定。
- `.test.ts` 走 TS loader。
- 无文件时发现 `*.test.{js,ts}`。
- `--parallel` 下失败、timeout、输出顺序都有确定语义。

## 8. P0：默认运行时没有权限模型

### 现状

`run`/`eval` 直接创建 `MinimalRuntime::new()`，CLI 没有 `--allow-*`，运行时启动时注入大量宿主能力。

关键证据：

- CLI 子命令无权限参数，见 [`src/main.rs:24`](../src/main.rs#L24)。
- `Run/Eval` 直接创建 runtime，见 [`src/main.rs:404`](../src/main.rs#L404)、[`src/main.rs:451`](../src/main.rs#L451)。
- runtime 一次性注入 `fs/http/net/crypto/fetch/process/worker` 等能力，见 [`src/runtime_minimal.rs:3851`](../src/runtime_minimal.rs#L3851)。
- `src/security` 只在 enterprise feature 下启用，且不是默认 runtime 权限系统，见 [`src/lib.rs:94`](../src/lib.rs#L94)。
- `fs` 直接调用宿主 FS，见 [`src/nodejs_core/fs.rs:155`](../src/nodejs_core/fs.rs#L155)、[`src/nodejs_core/fs.rs:187`](../src/nodejs_core/fs.rs#L187)。
- runtime 内部 `require('fs')` 又手写一套直接 FS，见 [`src/runtime_minimal.rs:14198`](../src/runtime_minimal.rs#L14198)。
- `process.env` 暴露全部环境变量，见 [`src/runtime_minimal.rs:15542`](../src/runtime_minimal.rs#L15542)。
- `process.exit/abort` 直接影响宿主，见 [`src/runtime_minimal.rs:15327`](../src/runtime_minimal.rs#L15327)、[`src/runtime_minimal.rs:15487`](../src/runtime_minimal.rs#L15487)。

### 第一性原理问题

执行用户 JS/TS 默认不应该拥有宿主全部能力。安全边界必须在资源访问点强制执行，而不是放在另一个 feature-gated 模块里。

权限模型需要回答：

- 脚本能读哪些路径？
- 能写/删哪些路径？
- 能访问哪些 host/port？
- 能监听哪些 addr/port？
- 能读哪些 env？
- 能执行哪些子进程？
- 包管理和 `bunx` 是否允许下载/执行远程代码？

没有这个 broker，后续补任何 fs/fetch/process/require 都可能引入新的绕过点。

### 最优解决方案

- 引入默认拒绝的 `PermissionState` 和 `ResourceBroker`。
- CLI 支持 `--allow-read`、`--allow-write`、`--allow-net`、`--allow-listen`、`--allow-env`、`--allow-run`、policy 文件。
- 所有 `fs`、`require`、TypeScript source input、package unpack、fetch、net、http server、process、child_process 都经 broker。
- 路径 canonicalize 后检查根目录、symlink 策略和读/写/删分权。
- 网络按 hostname、resolved IP、CIDR、port、redirect 后目标重新授权。
- `process.exit` 在嵌入式测试/runner 中转成可捕获错误；CLI 顶层才转换为退出码。

### 验证标准

- 默认 `amber eval "require('fs').readFileSync('/etc/passwd')"` 失败。
- 默认 `fetch("http://127.0.0.1")`、`net.connect`、`http.listen` 失败。
- 授权某目录后，目录外绝对路径、`..`、symlink escape 都失败。
- 默认读不到敏感 env。
- `process.exit()` 不杀死 Rust test harness。

## 9. P0：包管理和供应链边界不安全

### 现状

Amber 包管理模块默认构建启用，但安装器还没有 npm 级别的基础契约。

关键证据：

- 用外部 `curl` 拉 registry 和 tarball，见 [`src/package_manager.rs:218`](../src/package_manager.rs#L218)、[`src/package_manager.rs:280`](../src/package_manager.rs#L280)。
- metadata 中 `dist.shasum` 存在但未校验，见 [`src/package_manager.rs:83`](../src/package_manager.rs#L83)。
- lockfile `integrity` 写成 `None`，见 [`src/main.rs:882`](../src/main.rs#L882)、[`src/package_manager.rs:890`](../src/package_manager.rs#L890)。
- tar 解包只 `strip_prefix("package")` 后 unpack，见 [`src/package_manager.rs:331`](../src/package_manager.rs#L331)、[`src/package_manager.rs:352`](../src/package_manager.rs#L352)。
- `install_package` 下载并解压单包后结束；`install_dependencies` 只遍历 root dependencies，不递归依赖图，见 [`src/package_manager.rs:424`](../src/package_manager.rs#L424)、[`src/package_manager.rs:475`](../src/package_manager.rs#L475)。
- `package-lock.json` 读写路径和 npm v3 schema 不一致，见 [`src/package_manager.rs:643`](../src/package_manager.rs#L643)、[`src/package_manager.rs:725`](../src/package_manager.rs#L725)。
- semver range 手写且不符合 npm 语义，见 [`src/package_manager.rs:106`](../src/package_manager.rs#L106)、[`src/package_manager.rs:153`](../src/package_manager.rs#L153)。
- scoped package spec 解析错误，见 [`src/main.rs:799`](../src/main.rs#L799)、[`src/main.rs:1117`](../src/main.rs#L1117)。
- `bunx` 安装后直接 `Command::new(bin_path)` 执行，见 [`src/main.rs:1139`](../src/main.rs#L1139)。

### 第一性原理问题

npm package 是远程不可信归档。包管理器必须先建立内容身份，再 materialize 依赖图，最后才允许运行 bin。当前缺少：

- 完整性校验。
- 安全 unpack。
- 传递依赖 resolver。
- lockfile 事实源。
- 正确 semver/package spec。
- `node_modules` 与 runtime `require` 的连接。
- 远程执行确认和权限边界。

### 最优解决方案

- Resolver 和 installer 分两阶段。
- 下载到临时文件，按 `dist.integrity`/SRI 校验，兼容 `shasum`，通过后原子进入内容寻址缓存。
- unpack 拒绝绝对路径、`..`、symlink/hardlink escape、特殊文件。
- 读取 root `package-lock.json` 作为优先事实源；支持 `--frozen-lockfile`/`ci`。
- 使用成熟 semver range 解析，不手写 npm range。
- 实现 package spec parser：`name@range`、`@scope/name@range`、dist-tag、alias、file、git、tarball、workspace。
- `bunx` 使用隔离 exec store，不修改当前项目；远程执行需要 `--allow-run --yes` 或等价策略。
- runtime `require()` 必须能加载 installer materialize 的真实 npm layout。

### 验证标准

- 恶意 tarball 写 `package/../../pwned` 失败。
- integrity mismatch 失败且不污染缓存。
- `a -> b -> c` 传递依赖可被 `require("a")` 加载。
- `@types/node`、`@babel/core@^7`、`react@18` spec 解析正确。
- lock 指定旧版本时 registry latest 变化不影响安装。
- `amber bunx` 默认不创建项目 `node_modules`，不默认执行远程包。

## 10. P0：Crypto/WebCrypto 不能有假成功

### 现状

安全审计发现 crypto 相关实现存在多处 placeholder/fallback。

关键证据：

- `CryptoKey` 原始 key 存在 JS 属性，见 [`src/web_api/crypto.rs:427`](../src/web_api/crypto.rs#L427)。
- AES-GCM 缺 IV 时使用全零 nonce，见 [`src/web_api/crypto.rs:884`](../src/web_api/crypto.rs#L884)、[`src/web_api/crypto.rs:1163`](../src/web_api/crypto.rs#L1163)。
- AES-CBC/未知算法可能返回 IV+明文或剥 IV，见 [`src/web_api/crypto.rs:999`](../src/web_api/crypto.rs#L999)、[`src/web_api/crypto.rs:1203`](../src/web_api/crypto.rs#L1203)。
- RSA verify 固定 true，见 [`src/web_api/crypto.rs:815`](../src/web_api/crypto.rs#L815)。
- ECDSA 有长度 fallback，见 [`src/web_api/crypto.rs:788`](../src/web_api/crypto.rs#L788)。
- HMAC-SHA1/SHA512 未正确 update 数据，见 [`src/nodejs_core/crypto.rs:393`](../src/nodejs_core/crypto.rs#L393)。

### 第一性原理问题

密码学 API 没有“半兼容”。任何看似成功的错误实现都会让用户把它用于认证、签名、加密、完整性校验，从而制造真实漏洞。

### 最优解决方案

- 只暴露有标准测试向量覆盖的算法。
- 未实现算法直接 throw/reject，不 fallback 成明文、固定 true 或伪随机。
- key 存 Rust 侧 opaque handle，不放 JS 可读属性。
- `extractable: false` 必须真实不可导出。
- MD5/SHA1 只作为兼容 hash，文档标注非安全用途。
- 引入 Wycheproof/NIST/vector tests，先覆盖 AES-GCM、HMAC、SHA、RSA/ECDSA verify。

### 验证标准

- 篡改签名 verify 返回 false。
- AES-GCM 缺 IV 或错误 IV 失败。
- `extractable:false` key 不能从 JS 读出。
- HMAC 与 Node/OpenSSL 输出逐字节一致。

## 11. P1：Node/Web API 需要从“对象形状”回到语义兼容

### 现状

Node/Web API 有大量模块，但存在多套 bootstrap、同步 blocking、thenable 伪 Promise、fallback 成功、object shell 等问题。

关键证据：

- Web API bootstrap 在 [`src/web_api/mod.rs:62`](../src/web_api/mod.rs#L62) 和 runtime 内重复，runtime 还手写 install 多个 globals，见 [`src/runtime_minimal.rs:3853`](../src/runtime_minimal.rs#L3853)。
- `fetch` 不是真实 Promise 异步模型，而是 spawn thread + 创建 Tokio runtime + join/同步结果，见 [`src/web_api/fetch.rs:352`](../src/web_api/fetch.rs#L352)。
- fetch/http 网络失败会 fallback 为 `200 OK` 或成功响应，见 [`src/web_api/fetch.rs:752`](../src/web_api/fetch.rs#L752)、[`src/nodejs_core/http.rs:1146`](../src/nodejs_core/http.rs#L1146)。
- Promise 静态方法被 runtime 覆盖且不完整，见 [`src/runtime_minimal.rs:13256`](../src/runtime_minimal.rs#L13256)、[`src/runtime_minimal.rs:13321`](../src/runtime_minimal.rs#L13321)。
- Buffer 有多套实现和 stub，见 [`src/nodejs_core/buffer.rs:57`](../src/nodejs_core/buffer.rs#L57)、[`src/nodejs_core/require.rs:82`](../src/nodejs_core/require.rs#L82)。
- EventEmitter 状态不是实例隔离，见 [`src/nodejs_core/events.rs:9`](../src/nodejs_core/events.rs#L9)、[`src/nodejs_core/events.rs:179`](../src/nodejs_core/events.rs#L179)。
- stream/net/http 更像对象形状，不是完整事件流语义，见 [`src/nodejs_core/stream.rs:287`](../src/nodejs_core/stream.rs#L287)、[`src/nodejs_core/net.rs:608`](../src/nodejs_core/net.rs#L608)、[`src/nodejs_core/http.rs:1130`](../src/nodejs_core/http.rs#L1130)。
- WebSocket runtime 默认 inline/no-op，真实模块没有被统一集成，见 [`src/runtime_minimal.rs:12686`](../src/runtime_minimal.rs#L12686)、[`src/web_api/websocket.rs:71`](../src/web_api/websocket.rs#L71)。

### 第一性原理问题

兼容层应以“共享抽象 + 规范 fixture + 差分测试”推进，而不是按 API 名字快速堆对象。尤其是：

- Error object model 要统一。
- Byte/body model 要统一。
- EventTarget/EventEmitter/listener lifecycle 要统一。
- Stream/Backpressure 要统一。
- Promise/callback/task queue 要统一。
- 权限拒绝和网络失败不能变成成功。

### 最优解决方案

- 先实现 Host API registry，每个 API 只通过唯一入口安装。
- 不覆盖 V8 原生 Promise；只在 host async API 返回真实 Promise。
- `fetch`、fs async、net/http 都挂到 runtime scheduler。
- 建立统一 `NodeError`/`DOMException` 映射。
- Buffer、Blob、Response body、streams 使用共享 byte source/stream abstraction。
- Node/Web 兼容按 manifest 推进：Stable、Preview、Stub/Unavailable。
- Stub API 默认不暴露；如果暴露，调用时明确 throw `NotImplementedError`。

### 验证标准

- 兼容测试使用本地固定 HTTP server，不依赖 httpbin 或外网。
- fetch DNS/TLS/permission failure reject。
- EventEmitter listener add/remove/once/error semantics 与 Node fixture 对齐。
- Web Streams reader locked、backpressure、cancel/error 顺序与 WPT 子集对齐。
- Buffer encoding/offset/error 与 Node 差分。

## 12. P1：Worker 和并发 API 目前只是同步对象壳

### 现状

Worker、MessageChannel、BroadcastChannel、ServiceWorker、hot reload 都有实现文件，但并发语义不闭合。

关键证据：

- Worker 只登记 ID，`postMessage` 主要打印日志，见 [`src/web_api/worker.rs:12`](../src/web_api/worker.rs#L12)、[`src/web_api/worker.rs:95`](../src/web_api/worker.rs#L95)、[`src/web_api/worker.rs:156`](../src/web_api/worker.rs#L156)。
- MessageChannel 同步保存同上下文对象引用，见 [`src/web_api/message_channel.rs:37`](../src/web_api/message_channel.rs#L37)、[`src/web_api/message_channel.rs:120`](../src/web_api/message_channel.rs#L120)。
- BroadcastChannel 没有按 name 路由到同名 endpoint，反而可能自发自收，见 [`src/web_api/broadcast_channel.rs:83`](../src/web_api/broadcast_channel.rs#L83)。
- ServiceWorker `addEventListener`/生命周期基本 no-op，见 [`src/web_api/service_worker.rs:123`](../src/web_api/service_worker.rs#L123)、[`src/web_api/service_worker.rs:419`](../src/web_api/service_worker.rs#L419)。
- watcher 和 websocket reload 缺少可取消 supervisor/generation 协议，见 [`src/watcher.rs:180`](../src/watcher.rs#L180)、[`src/watcher_websocket.rs:156`](../src/watcher_websocket.rs#L156)。

### 第一性原理问题

Worker 是独立 agent：独立 isolate、独立事件循环、消息队列、structured clone、错误传播和终止生命周期。MessageChannel/BroadcastChannel 也必须异步派发，不能同步共享 V8 object。

### 最优解决方案

- WorkerHost 为每个 worker 创建独立 runtime/isolate，使用 Rust channel 传递 structured clone payload。
- MessagePort 后端是 channel + task queue，不直接保存 JS object。
- Broadcast registry 按 channel name 管理 endpoints，发送者不收自己的消息。
- ServiceWorker 暂时标为 unavailable，除非实现 registration store、install/activate/fetch lifecycle、`waitUntil/respondWith`。
- Hot reload 建立 supervisor，持有 JoinHandle/CancellationToken 和 generation id。

### 验证标准

- `new Worker("echo.js")` 双向 postMessage。
- worker throw 触发 `onerror`。
- `terminate()` 后不再派发消息。
- post 后同步代码先执行，接收发生在后续 task。
- ArrayBuffer transfer 后源端 detached。
- hot reload 快速连续写入只执行最新 generation。

## 13. P1：性能优化目前缺少真实测量闭环

### 现状

Amber 有快照、memory engine、benchmark 文档和脚本，但多个性能路径未接入真实运行时。

关键证据：

- `generate_snapshot()` 返回空 `Vec`，`load_snapshot()` 只更新统计，见 [`src/v8_snapshot/manager.rs:36`](../src/v8_snapshot/manager.rs#L36)、[`src/v8_snapshot/manager.rs:56`](../src/v8_snapshot/manager.rs#L56)。
- `MinimalRuntime::new()` 仍直接 `v8::Isolate::new(create_params)`，见 [`src/runtime_minimal.rs:2022`](../src/runtime_minimal.rs#L2022)。
- CLI 每次 `run/eval/test/watch reload` 都重建 runtime，见 [`src/main.rs:333`](../src/main.rs#L333)、[`src/main.rs:404`](../src/main.rs#L404)、[`src/main.rs:450`](../src/main.rs#L450)。
- 首次 execute 急切安装大量 API，见 [`src/runtime_minimal.rs:3843`](../src/runtime_minimal.rs#L3843)。
- 重复执行每次 `v8::Script::compile`，CommonJS 只缓存 exports 不缓存 compiled wrapper，见 [`src/runtime_minimal.rs:4086`](../src/runtime_minimal.rs#L4086)、[`src/runtime_minimal.rs:14941`](../src/runtime_minimal.rs#L14941)。
- memory/GC 模块与真实 V8 heap/RSS 脱节，见 [`src/memory/gc_optimizer_enhanced.rs:373`](../src/memory/gc_optimizer_enhanced.rs#L373)。
- benchmark 脚本存在吞错、硬编码路径、估算内存/并发等问题，见 [`benchmarks/amberjs_benchmark.js:30`](../benchmarks/amberjs_benchmark.js#L30)、[`benchmarks/amberjs_benchmark.js:144`](../benchmarks/amberjs_benchmark.js#L144)。

### 第一性原理问题

性能优化必须满足三点：

- 优化点接在真实热路径上。
- 指标来自同等方法、校验输出、失败即失败的基准。
- 每个性能声明可复现：commit、机器、命令、样本数、p50/p95/p99、RSS、线程数、raw logs。

否则“快照”“零拷贝”“AI GC”“1000x”这些词会成为误导，而不是工程优势。

### 最优解决方案

- 先删掉或降级当前未复现性能倍数宣称。
- 建立冷进程、热 runtime、模块加载、TS 转译、I/O、memory 的独立 benchmark。
- 快照必须在 isolate 创建时消费真实 snapshot blob；断言 blob 非空并实际被 V8 使用。
- Lazy install API：最小 global 启动，`require("fs")`、`fetch`、Worker 按需安装。
- 代码缓存：TS transform cache、V8 code cache、module graph cache。
- V8 ArrayBuffer allocator/external memory accounting 接入真实 Buffer/Blob/body。

### 验证标准

- `amber eval "1"` 冷启动 p50/p95 可复现。
- 快照 A/B 对比显示 isolate 初始化实际减少。
- require 同一模块 1/10/100 次编译次数下降。
- 大 Buffer/fetch body 的 RSS 在循环后稳定回落。
- benchmark 输出 JSON schema 校验退出码、stdout 正确性和环境信息。

## 14. P0/P1：CLI 和产品承诺需要收缩

### 现状

CLI 暴露 `run/eval/repl/test/bundle/debug/version/serve/init/add/remove/install/prune/create/bunx/upgrade`，但并非每个命令都有真实能力边界。

关键证据：

- `amber create --help` 存在 clap 参数顺序导致的 panic 风险，证据来自 `Create` 子命令字段顺序，见 [`src/main.rs:150`](../src/main.rs#L150)。
- `amber test` 假阳性问题见前文。
- `run` 捕获 args 但 process.argv 使用固定 placeholder，见 [`src/main.rs:29`](../src/main.rs#L29)、[`src/runtime_minimal.rs:15550`](../src/runtime_minimal.rs#L15550)。
- `serve` 目前主要打印配置，不是真实 serve 能力，见 [`src/main.rs:735`](../src/main.rs#L735)。
- README 把 `bundle/serve/add/bunx/upgrade` 都列为 CLI 能力，见 [`README.md:107`](../README.md#L107)。
- Quick Start 安装表达与 release 事实不一致，见 [`docs/QUICK_START.md:8`](QUICK_START.md#L8)。
- `src/lib.rs` 有全局 `#![allow(clippy::all)]`，但 CI/Makefile 又宣称 clippy `-D warnings`，见 [`src/lib.rs:1`](../src/lib.rs#L1)、[`.github/workflows/ci.yml:52`](../.github/workflows/ci.yml#L52)。
- `Cargo.toml` 版本为 `0.1.0`，但源码/Stage 文档混杂更高阶段版本口径，见 [`Cargo.toml:3`](../Cargo.toml#L3)、[`src/lib.rs:80`](../src/lib.rs#L80)。
- Stage 报告仍有“生产就绪”等历史宣称，见 [`docs/STAGE_89_FINAL_COMPLETION_REPORT.md:288`](STAGE_89_FINAL_COMPLETION_REPORT.md#L288)。

### 第一性原理问题

产品承诺只能来自当前可执行事实。命令存在、模块存在、文档存在、stage report 存在，都不等于能力成熟。

### 最优解决方案

- 新增 `docs/CURRENT_SCOPE.md`，列出 Stable、Preview、Experimental、Historical。
- README 只承诺 Stable；Preview/Experimental 必须明确边界。
- CLI help 按稳定性分组；Preview 命令运行时也输出边界提示。
- 每个 README 命令有 CLI smoke test。
- Stage 文档迁移到 `docs/archive/stages/` 或加 front matter：`status: historical|plan|superseded`。
- 版本只允许 `Cargo.toml`、release tag、CHANGELOG 持有产品版本。
- 性能页只保留当前可复现数据。
- 全局 clippy allow 改成局部 allow + 债务编号。

### 验证标准

- `amber --help` 中 Stable 命令都能 smoke pass。
- README 每个命令都由测试验证。
- 文档 lint 阻止当前发布页出现未复现“生产就绪/完整替代/性能领先”。
- `cargo check --features ai,observability,enterprise,cloudnative,multilang` 失败时，相关 feature 不得出现在 Stable 文档。

## 15. 推荐的系统重构路线

### Phase 0：止血与事实对齐（1-2 周）

目标：停止继续扩大承诺面，让用户和贡献者看到真实边界。

建议动作：

1. 新增 `docs/CURRENT_SCOPE.md`：Stable、Preview、Experimental、Historical。
2. README 和 Quick Start 改为链接 current scope，不再直接承诺历史 stage 能力。
3. `amber test` 在接入 runner 前不要报告“tests passed”；至少失败时非零退出。
4. 修复 `amber create --help` panic。
5. `serve`、`Worker`、`WebSocket`、`ServiceWorker`、TSX、snapshot、bunx 等未闭合能力标为 preview/experimental 或调用时报 `NotImplementedError`。
6. 删除/降级未复现性能倍数和“生产就绪”当前表述。
7. 为 README 中 Stable 命令建立 smoke tests。

### Phase 1：运行时内核（2-4 周）

目标：让所有 host API 有同一个状态、权限、事件循环和资源生命周期。

建议动作：

1. 从 `runtime_minimal.rs` 中抽出 runtime-owned state。
2. 建立 task queue/microtask/I/O completion/timer 统一 scheduler。
3. 移除跨 runtime 的 V8 callback global registry。
4. 建立 ResourceBroker 和 PermissionState，但可以先 default-deny + 最小 allow flags。
5. Host API 只通过 registry 安装，禁止重复 bootstrap。
6. `process.exit`、fs、fetch、net、require 都接入 broker。

### Phase 2：模块图与 TypeScript（3-6 周）

目标：所有执行入口使用同一个 loader。

建议动作：

1. 建立 ModuleResolver 和 ModuleGraph。
2. CommonJS resolver 支持 parent directory、node_modules、package `main/exports`、JSON/index fallback。
3. TypeScript 作为 loader transform，source map 接入 V8 stack。
4. ESM 不再用注释/require 模拟；要么真实 `v8::Module`，要么明确不支持。
5. 删除 runtime string sniffing 的 TS fallback。
6. 取消“重新执行最后表达式”的 eval result 机制。

### Phase 3：测试和兼容 oracle（2-4 周）

目标：让测试能抓住真实行为回归。

建议动作：

1. `amber test` 接入唯一 runner。
2. matcher 失败抛 `AssertionError`。
3. 文件级 isolate 并行，timeout 用 isolate termination。
4. 引入本地 HTTP fixture server。
5. 建立 `tests/conformance/` manifest：Node fixtures、WPT 子集、expected pass/fail。
6. 建立 `amberjs-diff`：同一 fixture 跑 Amber、Node、Bun、Deno，归一化 stdout/stderr/error/exit code。

### Phase 4：Node/Web API 语义收敛（持续）

目标：少量 API 做真，比大量 API 看起来存在更有价值。

建议动作：

1. 先稳定 `path`、`url`、`buffer`、`events`、`fs` sync subset、`crypto hash/hmac`、`timers`、`fetch` basic。
2. 建立统一 error model、byte/body model、stream/event abstraction。
3. `fetch`、fs async、http/net 接 scheduler，不再 fake success。
4. 未实现 API 不暴露或明确 throw。
5. 每个 API 以 conformance manifest 推进状态。

### Phase 5：包管理、安全和性能闭环（持续）

目标：让生态能力既能用又不危险。

建议动作：

1. npm resolver + installer 两阶段。
2. SRI 校验、内容寻址缓存、安全 unpack。
3. lockfile v2/v3 schema 与 `--frozen-lockfile`。
4. `bunx` 隔离 exec store + 显式执行授权。
5. 真实 V8 snapshot/code cache/lazy API install。
6. 性能基准 JSON 化、校验输出、保留 raw logs。

## 16. 立即行动清单：最值得先做的 15 件事

1. 建 `docs/CURRENT_SCOPE.md`，把当前能力按 Stable/Preview/Experimental/Historical 分层。
2. 修复 `amber create --help` panic。
3. 让 `amber test` 不再假阳性：断言失败必须非零退出；接入 runner 前宁可明确 experimental。
4. 移除或禁用 crypto 中固定 true、明文 fallback、全零 nonce、JS 可读 key 数据。
5. 默认权限模型先落最小版：`--allow-read`、`--allow-write`、`--allow-net`、`--allow-env`、`--allow-run`。
6. 所有 FS 入口收敛到同一个 broker，先堵住重复 `std::fs` 绕过点。
7. package tarball 下载加 integrity 校验和安全 unpack。
8. `bunx` 默认不执行远程包，至少要求显式确认/授权。
9. 删除 fetch/http 的 fake `200 OK` fallback，失败就 reject/throw。
10. 停止公开声称未闭合的 Worker/ServiceWorker/WebSocket/TSX/HTTPS serve 能力。
11. 抽出 ModuleResolver，先把 CommonJS parent-relative require 和 npm package resolution 做对。
12. TypeScript 编译入口统一到 loader，source map 接入错误栈。
13. 移除 `execute_code()` 重跑最后表达式的逻辑。
14. 删除或降级未复现性能倍数；建立冷启动和模块加载基准。
15. 把 Stage 文档从当前产品文档中物理/元数据隔离。

## 17. 不建议继续做的事

- 不建议继续在 `runtime_minimal.rs` 里直接添加更多 Node/Web API。
- 不建议用 stub/fallback 返回成功来“提高兼容率”。
- 不建议在测试框架能假阳性时扩展大规模兼容测试。
- 不建议先追求性能指标；运行时语义、权限和测试 oracle 没闭合前，性能数字容易变成噪声。
- 不建议让 stage report 继续作为当前产品能力证明。
- 不建议把 `src/security/` 里的 enterprise/demo 安全模块当作运行时权限系统。

## 18. 最终判断

Amber 有一个很有价值的方向：Rust + V8 的 JS/TS 运行时，加上 Node/Web 双兼容、安全权限和现代工具链。它的问题不是没有想象力，而是想象力已经跑到内核契约前面去了。

第一性原理下，最优解不是“再补齐 100 个 API”，而是：

1. 收缩承诺。
2. 建运行时内核。
3. 建模块图。
4. 建权限 broker。
5. 建会失败的测试。
6. 用规范和差分测试逐个恢复公开能力。

只要把这六件事做扎实，Amber 之后的 Node/Web/TS/包管理/性能扩展会从“阶段性堆叠”变成“可验证的平台”。这比继续扩大表面积慢一点，但会让项目真正开始变硬。

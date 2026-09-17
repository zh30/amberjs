# Amber Monorepo 迁移与架构重构任务计划书

- **项目名称**：Amber (amberjs)
- **文档编号**：PLAN-2026-MONOREPO-01
- **当前状态**：Phase 1 & Phase 2 已落地完成，工作区骨架与首批底层 Crates 已并网运行
- **目标周期**：4 个阶段（渐进式平滑演进）
- **适用分支**：`main` / `refactor/monorepo`

---

## 1. 背景与核心动机

Amber 当前采用单根目录单体 Crate 架构（单个 `Cargo.toml` 统管 20+ 万行 Rust 代码）。随着项目功能不断扩展，当前单体结构遇到了严重的工程瓶颈：

1. **测试编译爆炸与磁盘吞噬**：
   - 仓库内包含 232 个独立集成测试。在单个 Crate 模式下，每个测试均需完整静态链接整个运行时（包含 Google V8、SWC、Tokio 等），单次完整测试编译产生 120GB+ 的二进制，累计堆积可达 200GB。
   - **Monorepo 目标**：测试就近下沉至各自专有子 Crate，按需独立增量编译，彻底根除冗余链接，构建缓存体积减少 90%+。
2. **模块职责严重缠绕（God Module 风险）**：
   - `src/runtime_minimal.rs` 超过 1.8 万行，混合了 V8 启动、模块加载、HTTP 旁路、AI 算子、沙箱权限拦截等多方职责。
   - **Monorepo 目标**：通过 Cargo Workspace 的强约束有向无环图（DAG），在编译期强制实施单向依赖。
3. **多语言与生态工具协同脱节**：
   - 仓库包含 Rust 运行时内核、TypeScript 类型声明、React/Vite 官网应用、Monaco 代码演练场、VS Code 插件和 Zed 插件，依赖与版本管理长期割裂。
   - **Monorepo 目标**：建立 **Cargo Workspace + pnpm Workspace** 双工作区体系，统一代码规范、构建流与自动化发布。

---

## 2. 目标架构与目录拓扑

```text
amberjs/                                # 仓库根目录
├── Cargo.toml                          # 【根级 Cargo 虚拟工作区 (Virtual Workspace)】
├── Cargo.lock
├── pnpm-workspace.yaml                 # 【根级 pnpm 多包工作区】
├── package.json                        # 根级多语言任务协调脚本
├── Makefile                            # 根级快捷构建指令
│
├── crates/                             # 【Rust 内核子工程】
│   ├── amber_cli/                      # 最终可执行二进制 amber 入口 (main.rs, clap 命令装配)
│   ├── amber_runtime/                  # V8 Isolate 生命周期、Tokio 事件循环、V8 启动快照
│   ├── amber_node/                     # Node.js 兼容层 (fs, http, crypto, net, stream, buffer)
│   ├── amber_web/                      # WHATWG Web 标准 API (fetch, streams, websocket, crypto)
│   ├── amber_sandbox/                  # 权限代理 (Broker)、虚拟文件系统 (VFS) 与沙箱隔离
│   ├── amber_transpile/                # oxc / SWC TypeScript 快速转译与 amber bundle/compile 打包器
│   ├── amber_agent/                    # MCP (Model Context Protocol) 协议、JSON-RPC 与 AI 原生能力
│   └── amber_testing/                  # 内建 Jest 风格测试框架引擎与断言匹配器
│
├── packages/                           # 【JS/TS 官方生态包 (发布至 npm)】
│   ├── types/                          # @amberjs/types (官方 TypeScript 环境声明定义)
│   └── create-amber/                   # npm create amber@latest 项目脚手架工具
│
├── apps/                               # 【独立应用与站点】
│   └── website/                        # 官网、文档中心与 Monaco Web Playground (Cloudflare Workers)
│
├── extensions/                         # 【IDE 开发者扩展】
│   ├── vscode/                         # VS Code 官方插件 (高亮、调试适配、补全)
│   └── zed/                            # Zed 官方扩展 (Rust/Wasm)
│
├── tests/                              # 【跨 Crate 端到端与合规性测试】
│   ├── conformance/                    # Node.js 55/55 合规性测试套件
│   └── e2e/                            # CLI 端到端黑盒测试
│
├── benchmarks/                         # 性能基准测试套件
├── scripts/                            # 安装脚本、发布脚本与维护工具
└── docs/                               # 架构设计、设计规范与计划文档
```

---

## 3. 子 Crate 职责边界与依赖有向无环图 (DAG)

### 3.1 依赖关系拓扑

```text
                         ┌─────────────────┐
                         │    amber_cli    │ (最终可执行二进制 amber)
                         └────────┬────────┘
                                  │
                                  ▼
                         ┌─────────────────┐
                         │  amber_runtime  │ (V8 生命周期、快照、模块分发)
                         └──┬─────┬──────┬─┘
              ┌─────────────┘     │      └─────────────┐
              ▼                   ▼                    ▼
     ┌─────────────────┐ ┌─────────────────┐  ┌─────────────────┐
     │   amber_node    │ │    amber_web    │  │   amber_agent   │
     │(Node.js 兼容层) │ │ (WHATWG Web API)│  │ (MCP / Native AI)
     └────────┬────────┘ └────────┬────────┘  └────────┬────────┘
              │                   │                    │
              └────────────┬──────┴────────────────────┘
                           ▼
                  ┌─────────────────┐
                  │  amber_sandbox  │ (权限决策代理、VFS 隔离)
                  └────────┬────────┘
                           ▼
                  ┌─────────────────┐
                  │ amber_transpile │ (oxc AST, TS 转译, 独立打包)
                  └─────────────────┘
```

### 3.2 子 Crate 规格矩阵

| Crate 名称 | 对应当前代码位置 | 核心职责与对外接口 | 主要上游依赖 |
| :--- | :--- | :--- | :--- |
| **`amber_transpile`** | `src/typescript/`, `src/tooling/compiler.rs` | 负责 TS 语法剥离、JSX 转译、代码压缩与单文件打包。不依赖 V8。 | `oxc_allocator`, `oxc_parser` |
| **`amber_sandbox`** | `src/permissions/`, `src/vfs/`, `src/security/` | 权限策略解析（`--allow-*`/`--deny-*`）、ResourceBroker、虚拟内存文件系统。 | `amber_transpile`, `serde_json` |
| **`amber_web`** | `src/web_api/` | WHATWG Web API 实现：`fetch`, `WebSocket`, `Streams`, `Crypto`, `URL`。暴露 `register_web_apis`。 | `amber_sandbox`, `v8`, `reqwest` |
| **`amber_node`** | `src/nodejs_core/` | Node.js 标准库实现：`fs`, `http`, `crypto`, `net`, `stream`, `process` 与 CJS 加载器。 | `amber_sandbox`, `v8`, `tokio` |
| **`amber_agent`** | `src/mcp/`, `src/agent/`, `src/nodejs_core/ai.rs` | MCP 协议客户端/服务端、JSON-RPC 会话通道、零拷贝 Tensor 算子。 | `amber_sandbox`, `v8`, `tokio` |
| **`amber_runtime`** | `src/runtime_minimal.rs`, `src/v8_snapshot.rs` | V8 Isolate 管理、全局作用域初始化、Tokio 事件循环泵、Microtask Checkpoint。 | `amber_node`, `amber_web`, `amber_agent` |
| **`amber_testing`** | `src/testing/` | Jest 风格测试执行器、describe/test 收集器、快照比对、覆盖率采集。 | `amber_runtime` |
| **`amber_cli`** | `src/main.rs`, `src/cli/` | 命令行参数解析（`clap`）、REPL 交互、子命令分发、最终生成 `amber` 二进制。 | 全部 crates |

---

## 4. 四阶段分步实施路线图与任务清单

> **核心实施原则**：**随时保持编译可用（Always Compilable）**。禁止一次性重写全局，采用外围先行、核心抽离、最后并轨的三步策略。

---

### Phase 1: 建立工作区骨架与外围项目就位（低风险）

- [x] **1.1 根级多语言工作区声明**
  - [x] 创建 `pnpm-workspace.yaml`，声明 `packages/*`、`apps/*`、`extensions/*`。
  - [x] 创建根级 `package.json`，提供跨工程统合命令（`build:all`, `lint:all`, `test:all`）。
- [x] **1.2 独立应用与前端归位**
  - [x] 将当前 `website/` 目录迁移至 `apps/website/`。
  - [x] 校验 `apps/website/` 能够通过 `pnpm --filter amberjs-website build` 正确编译并完成 53 路由预渲染。
- [x] **1.3 IDE 扩展归位**
  - [x] 将 `tools/vscode-extension/` 迁移至 `extensions/vscode/`。
  - [x] 将 `tools/zed-extension/` 迁移至 `extensions/zed/`。
- [x] **1.4 TypeScript 类型声明包独立化**
  - [x] 在 `packages/types/` 下初始化 `@amberjs/types` 的 `package.json` 与 `tsconfig.json`。
  - [x] 将 `types/amberjs.d.ts` 链接为该包的类型入口，配置 npm 发布流程。

**阶段交付物**：外围应用与扩展全部归入 Monorepo 目录，根级包管理器调度正常。

---

### Phase 2: 剥离无 V8 强绑定的底层能力 Crate（解耦先行）

- [x] **2.1 抽离 `crates/amber_transpile`**
  - [x] 初始化 `crates/amber_transpile/Cargo.toml`（依赖 `oxc` 系列，不依赖 `v8`）。
  - [x] 迁移 `src/typescript/` 转译逻辑与 `src/tooling/compiler.rs` 打包逻辑。
  - [x] 编写独立单元测试，验证纯代码转译与 TS 剥离速度（204 个单测在 0.01s 内通过）。
- [x] **2.2 抽离 `crates/amber_sandbox`**
  - [x] 初始化 `crates/amber_sandbox/Cargo.toml`。
  - [x] 迁移 `src/permissions/`、`src/vfs/` 虚拟文件系统与审计日志逻辑。
  - [x] 编写基准单测，验证纯内存 VFS 的读写与权限拦截。
- [x] **2.3 根项目接入验证**
  - [x] 根 Crate 引入 `path = "crates/amber_transpile"` 与 `path = "crates/amber_sandbox"`。
  - [x] 确保 `cargo build --bin amber` 与现有测试无缝跑通。

**阶段交付物**：首批无 V8 依赖的工具 Crate 独立化，具备极速编译与独立复用能力。

---

### Phase 3: 拆解核心 V8 兼容层与功能子系统（深度解耦）

- [ ] **3.1 抽离 `crates/amber_web`**
  - [ ] 初始化 `crates/amber_web/Cargo.toml`。
  - [ ] 迁移 `src/web_api/`（Fetch, Streams, WebSocket, Web Crypto, URL, Encoding 等）。
  - [ ] 规范导出统一注册函数：`pub fn register_web_apis(scope: &mut v8::PinScope, global: v8::Local<v8::Object>) -> Result<()>`。
  - [ ] 将对应 Web 单测（约 60 个）下沉至 `crates/amber_web/tests/`。
- [ ] **3.2 抽离 `crates/amber_node`**
  - [ ] 初始化 `crates/amber_node/Cargo.toml`。
  - [ ] 迁移 `src/nodejs_core/`（`fs`, `http`, `buffer`, `net`, `crypto`, `process`, `timers` 等）。
  - [ ] 规范导出：`pub fn register_node_builtins(...)` 与 CommonJS 模块解析器。
  - [ ] 将对应 Node 单测（约 100 个）下沉至 `crates/amber_node/tests/`。
- [ ] **3.3 抽离 `crates/amber_agent`**
  - [ ] 初始化 `crates/amber_agent/Cargo.toml`。
  - [ ] 迁移 `src/mcp/`、`src/agent/`、`src/nodejs_core/ai.rs`。
  - [ ] 规范导出 `amber:ai` 与 `amber:mcp` 原生模块支持。
- [ ] **3.4 抽离 `crates/amber_testing`**
  - [ ] 初始化 `crates/amber_testing/Cargo.toml`。
  - [ ] 迁移 `src/testing/` 下的 Jest 风格执行器、断言库与快照更新逻辑。

**阶段交付物**：各核心能力拥有独立的 Crate 与单测边界，单测链接时间大幅缩短。

---

### Phase 4: 打造轻量 `amber_runtime` 与 `amber_cli`（并轨交付）

- [ ] **4.1 打造 `crates/amber_runtime`**
  - [ ] 精简重构 `src/runtime_minimal.rs`，将其核心 Isolate 抽象、事件循环泵移入该 Crate。
  - [ ] 聚合接入 `amber_web`、`amber_node`、`amber_agent` 的模块注册插件机制。
- [ ] **4.2 打造 `crates/amber_cli`**
  - [ ] 迁移 `src/main.rs` 与 `src/cli/`，使其成为纯粹的装配入口。
  - [ ] 负责 `clap` 参数解析、REPL 输入行、配置组装与最终执行分发。
  - [ ] 产出最终二进制：`[[bin]] name = "amber"`。
- [ ] **4.3 根级 `Cargo.toml` 转为纯虚拟工作区 (Virtual Workspace)**
  - [ ] 移除根级 `[package]`，启用纯 `[workspace]` 声明。
- [ ] **4.4 CI/CD 与构建脚本矩阵适配**
  - [ ] 更新 `.github/workflows/ci.yml`：测试命令适配 `cargo test --workspace`。
  - [ ] 更新 `.github/workflows/release-assets.yml`：编译目标定位至 `crates/amber_cli`。
  - [ ] 更新 `Makefile` 与 `Dockerfile` 构建路径。

**阶段交付物**：完整的现代化 Monorepo 体系落成，单次编译提速 5~10 倍。

---

## 5. 关键工程挑战与规避方案

### 5.1 V8 `Local` Handle 跨 Crate 传递规范

- **风险**：V8 的 `Local<Value>`、`PinScope`、`ContextScope` 在跨 Crate 传递时，若签名不当极易引发生命周期编译冲突。
- **规避方案**：
  所有扩展 Crate（`amber_node`, `amber_web`）均统一接收 `&mut v8::PinScope` 或 `&mut v8::HandleScope`，并返回标准 `anyhow::Result<()>`。绝不在跨 Crate 结构体中保存裸 `Local` handle。

### 5.2 避免 200GB 编译膨胀的制度约束

- **规则 1**：除真正的端到端行为（如 CLI flag 解析、完整 HTTP 链路）保留在根 `tests/` 外，模块级测试全部下沉到各 Crate 内的 `tests/`。
- **规则 2**：在 CI 中配置 `cargo-sweep`，每周定时清理超过 14 天的过期增量编译哈希。

### 5.3 向后兼容性保障

- 迁移期间，根级统一入口或别名脚本确保历史外部测试命令（如 `cargo run -- run <file>`）不中断。

---

## 6. 验收与完成指标

1. [ ] **构建通过**：根目录执行 `cargo build --release` 顺利产出 `target/release/amber`。
2. [ ] **测试通过**：`cargo test --workspace` 覆盖率不降低，全量单测在 60 秒内完成。
3. [ ] **端到端回归**：`./tests/conformance/run_conformance.sh` 依然保持 55/55 fixtures 全数通过。
4. [ ] **多端应用正常**：`pnpm --filter amberjs-website build` 正确生成文档站与 Playground。
5. [ ] **产物体积可控**：日常开发测试一周后，`target/` 目录在未深度清理情况下稳定保持在 **10GB 以内**，彻底解决 200GB 膨胀问题。

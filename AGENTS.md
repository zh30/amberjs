# Repository Guidelines

## 项目定位

Amber（crate `amberjs`，CLI `amber`）是一个用 Rust 和 V8 构建的 JavaScript/TypeScript 运行时。当前仓库同时包含核心运行时代码、历史阶段实现、性能基准、修复脚本、文档站点和大量阶段报告。做代码修改时以 `Cargo.toml`、`src/lib.rs` 和实际测试结果为准；`README.md` 与 `docs/STAGE_*` 中的部分性能或阶段描述可能是历史目标或阶段总结。对外品牌用 Amber / Amberjs，不要再写 Beejs 或 `bee` 命令。

## 关键源码与模块边界

- `src/main.rs` 是当前 Cargo 启用的 `amber` 二进制入口；`Cargo.toml` 中的 `[[bin]]` 指向这里。
- 不要再把 `bee` / `beejs` 当作默认 CLI 或 crate 名。历史 `bee:*` 模块前缀仍是运行时别名，用户文档应写 `amber:*`。
- `src/lib.rs` 是库模块启用状态的事实来源。仓库里许多目录和 `.rs` 文件存在但被注释、feature-gate 或保留为阶段产物；不要仅因文件存在就假设它参与默认构建。
- `src/runtime_minimal.rs` 是当前 CLI 主要使用的 V8 执行运行时。
- `src/nodejs_core/` 提供 Node.js 兼容层，包括 `fs`、`crypto`、`stream`、`events`、`net`、`http`、`buffer`、`path`、`os`、`url`、`dns`、`process`、`timers`、`performance`、`readline`、CommonJS `require` 等。
- `src/web_api/` 提供 Web 标准 API，包括 `fetch`、`WebSocket`、Web Crypto、URL、事件、FormData、Abort、Blob、Timers、Encoding、Performance、Streams、CompressionStream、structuredClone、Worker、ServiceWorker、BroadcastChannel、MessageChannel 等。
- `src/typescript/` 是 TypeScript 转译模块；CLI 在执行 `.ts`/`.tsx` 文件时会先调用这里。
- `src/testing/` 是 Jest 风格测试框架，包含断言、发现器、V8 测试执行器、并行执行、超时、覆盖率和性能测试支持。
- `src/package_manager.rs`、`src/watcher.rs`、`src/watcher_websocket.rs`、`src/event_loop.rs`、`src/error/`、`src/fallback/`、`src/memory/`、`src/v8_snapshot.rs` 是默认构建中需要重点留意的共享模块。
- `src/ai*`、`src/cloud_native/`、`src/enterprise/`、`src/multilang/`、`src/observability/`、`src/benchmarks/` 等多由 Cargo features 控制或处于阶段实现状态。修改这些区域时同时检查对应 feature 是否能编译。

## 目录结构

- `tests/`：Rust 集成测试。`.disabled` 与 `.bak` 文件不参与 Cargo 编译。
- `examples/`：JS/TS 示例与测试框架示例，适合做 CLI smoke test。
- `benches/`、`benchmarks/`：Criterion/自定义基准、运行脚本和历史结果。
- `docs/`：架构、计划、阶段报告、API 和用户文档。阶段报告不一定反映当前可编译模块。
- `tools/`：Rust/TypeScript 辅助工具，如 benchmark runner、debug adapter、VS Code extension。
- `scripts/` 与根目录大量 `fix_*.py`、`*_benchmark.*`：历史修复和实验脚本。不要把新的长期代码放在根目录。
- `apps/website/`：官方站点（Vite/React/Tailwind），部署到 `amberjs.com`。
- `monitoring/`、`dashboards/`、`k8s/`、`config/`、`configs/`：部署、监控和环境配置。

## 构建与运行命令

```bash
cargo build
cargo build --release
cargo run -- run examples/basics/hello_world.js
cargo run -- eval "1 + 1"
cargo run -- repl
cargo run -- test examples/testing/math.test.js
```

Makefile 中的常用目标：

```bash
make build
make test
make fmt
make lint
make clean
```

注意：`make run file=...` 可能仍按旧形式调用二进制。做 CLI 验证时优先使用 `cargo run -- run <file>` 或 `./target/release/amber run <file>`。

## 测试与质量检查

```bash
cargo test
cargo test --lib
cargo test --test path_module_tests
cargo test --test crypto_aes_gcm_tests
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

- V8 相关测试可能对 isolate 生命周期和线程敏感；需要串行化时使用 `serial_test`。
- 新增 Rust 集成测试放在 `tests/<feature>_<scenario>_tests.rs`，文件 stem 用于 `cargo test --test <stem>`。
- 新增 JS/TS 行为示例放在 `examples/`，不要新增根目录临时测试文件。
- 修改 feature-gated 模块时补充运行对应检查，例如 `cargo check --features ai`、`cargo check --features observability`、`cargo check --features enterprise`。如果 feature 当前不可编译，在结论中明确说明。
- `cargo clippy --all-targets --all-features -- -D warnings` 是 CI 期望之一，但当前仓库有大量阶段模块；遇到既有失败时先区分新增问题和历史问题。

## Website

官方站点在 `apps/website/`：

```bash
cd apps/website
npm run dev
npm run build
npm run deploy:dry-run
```

不要手动编辑 `apps/website/dist/`、`apps/website/node_modules/` 或时间戳类生成文件，除非任务明确要求处理构建产物。

## 编码约定

- Rust edition 为 2021，优先使用仓库已有的 `anyhow::Result`、`thiserror`、`serde`、`tokio`、`rusty_v8` 写法。
- 模块、函数、变量使用 `snake_case`；类型、枚举、trait 使用 `PascalCase`。
- 对外模块需要在对应 `mod.rs` 或 `src/lib.rs` 中显式声明；新增模块时确认默认构建和 feature 构建边界。
- 不要随意开启 `src/lib.rs` 中被注释的历史模块；如果需要重新启用，先做最小编译验证并处理缺失类型、重复导入和 V8 API 兼容问题。
- V8 handle、scope、context 生命周期要保持局部、明确，避免跨线程或长生命周期保存 `Local` handle。
- 对 crypto、fs、http、Web API、Node.js 兼容行为的修改应使用真实实现和可执行测试，不要引入 stub 来“通过编译”。
- 允许中文注释和提交信息；已有代码大量使用中文阶段说明。新增注释应解释原因或约束，避免重复代码字面含义。

## 文档与阶段资料

- `docs/IMPLEMENTATION_PLAN_STAGE_*` 与 `docs/STAGE_*_COMPLETION_REPORT.md` 是阶段记录，适合了解设计意图，不应覆盖当前源码事实。
- 更新用户文档时，同步核对 CLI 子命令、Cargo features、默认二进制入口和实际示例路径。
- 性能数字必须来自当前可复现命令或报告；不要从旧 README 或阶段报告直接复制为当前事实。

## 提交与 PR 约定

- 项目历史常见提交格式为 `feat(v0.3.NNN): 中文描述`、`fix(v0.3.NNN): 中文描述`、`docs(v0.3.NNN): 中文描述`。
- PR 描述应包含：变更范围、影响模块、运行过的测试/检查、未验证项、相关 `docs/` 阶段文档链接。
- 提交前检查 `git status --short`，不要回滚或清理与当前任务无关的用户改动。

## Agent skills

### Issue tracker

Issues and PRDs are tracked in GitHub Issues for `zh30/amberjs`. See `docs/agents/issue-tracker.md`.

### Triage labels

Triage uses the default five-label vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

This repo uses a single-context domain docs layout. See `docs/agents/domain.md`.

## Agent 工作守则

- 开始实现前先读 `Cargo.toml`、`src/lib.rs` 和目标模块，不要仅凭目录名判断架构。
- 只修改任务相关文件。不要整理根目录历史脚本、`.bak`、`.disabled`、`target/`、`node_modules/`、`dist/`、benchmark 结果等无关内容。
- 如果测试失败，记录具体命令和失败原因；区分本次改动引入的问题与仓库既有失败。
- 涉及依赖下载、npm registry、Cargo registry、Docker、Wrangler 或外部服务时，先确认任务确实需要网络。
- 对 CLI 行为、运行时 API、Node/Web 兼容层的修改要优先增加或更新测试，而不是只改示例。

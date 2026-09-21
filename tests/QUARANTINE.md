# Amber Test Quarantine Inventory

日期：2026-06-12  
范围：`tests/` 下的 `.disabled`、`.bak` 与 `legacy` 测试资产。  
状态：这些资产是隔离资产，不属于当前 CI 质量门禁；不得把它们当作默认 `cargo test` 覆盖面或当前兼容性证明。

## 扫描命令与当前计数

以下计数来自 `codex/amberjs-design-repair-sprint` 分支上的只读扫描：

```bash
find tests -type f -name '*.disabled' -print | wc -l
# 65

find tests -type f -name '*.bak' -print | wc -l
# 2

find tests -type d -iname '*legacy*' -print | wc -l
# 2

find tests/legacy tests/fixtures/legacy -type f -print | wc -l
# 74
```

完整路径复查命令：

```bash
find tests -type f -name '*.disabled' -print | sort
find tests -type f -name '*.bak' -print | sort
find tests -type d -iname '*legacy*' -print | sort
find tests/legacy tests/fixtures/legacy -type f -print | sort
```

## 隔离资产分类

| 类别 | 当前数量 | 示例路径 | 为什么隔离 |
| --- | ---: | --- | --- |
| `.disabled` 文件 | 65 | `tests/ai_workload_tests.rs.disabled`、`tests/stage91_phase3/package_manager_tests.rs.disabled`、`tests/websocket_tests.rs.disabled` | 后缀不是 Cargo 集成测试入口；多为历史阶段或 feature-gated 能力测试，不能代表当前默认构建质量。 |
| `.bak` 文件 | 2 | `tests/stage91_phase21_boundary_condition_tests.rs.bak`、`tests/stage91_phase21_stress_tests.rs.bak` | 备份后缀不参与 Cargo 编译；内容需要先和当前源码、活跃测试和阶段目标重新核对。 |
| `legacy` 目录 | 2 个目录，74 个文件 | `tests/legacy/README.md`、`tests/legacy/sources/test_v8.rs`、`tests/fixtures/legacy/test_data/esm/math.js` | 位于非 Cargo 自动集成测试入口目录，包含历史二进制、脚本、fixture 和旧阶段源码，默认不作为 CI gate。 |

## 恢复条件

隔离资产恢复到当前质量门禁前，必须满足以下条件：

1. 有明确 owner、issue 或修复任务，说明要恢复的行为契约。
2. 先确认目标模块在 `src/lib.rs` 与相关 Cargo feature 中属于当前构建面。
3. 将测试改造成当前约定的位置和命名，例如 `tests/<feature>_<scenario>_tests.rs`，或作为明确 fixture 放入非执行目录并由活跃测试引用。
4. 删除或替换历史假设、旧 CLI 入口、旧二进制名、阶段性性能数字和已经被禁用的 API 预期。
5. 运行对应的最小验证命令，例如 `cargo test --test <stem>`；涉及 feature-gated 模块时还要运行对应 `cargo check --features <feature>`。
6. 如果恢复的是 JS/TS CLI 行为测试，需要用当前入口验证，例如 `cargo run -- run <file>` 或由 Rust 集成测试启动当前 `amberjs` 二进制。

## 删除条件

隔离资产可以删除，但应在专门 PR 中处理，并满足至少一个条件：

1. 已被当前活跃测试覆盖，且新测试能失败地捕获同一行为回归。
2. 依赖的阶段模块、API、CLI 入口或 feature 已明确不再是 Amber 当前路线。
3. 内容只是历史备份、生成产物或无法复现的临时实验文件。
4. 与 `docs/CURRENT_SCOPE.md` 或当前 README/Quick Start 的能力边界冲突，且没有恢复计划。

删除 PR 应列出被删资产、对应替代测试或删除理由，并避免混入源码重构。

## 后续治理规则

1. 不新增新的 `.disabled` 或 `.bak` 测试文件；需要暂缓的测试应绑定 issue，并在 PR 描述中说明为何不能落入当前 gate。
2. 新增 legacy 资产必须放在明确命名的 fixture 或 archive 目录，并在本清单登记用途、owner 和退出条件。
3. 启用历史测试时，不要批量改名；每次只恢复一个行为簇，并补齐当前构建验证。
4. CI 与文档只引用活跃测试结果，不引用隔离资产作为通过证据。
5. 本清单应在新增、恢复或删除隔离资产时同步更新扫描命令结果。

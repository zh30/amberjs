---
title: "稳定性分级"
nav: "稳定性分级"
subtitle: "Amber 如何判断一项能力是 Stable、Preview、Experimental 还是不支持。"
status: "stable"
id: "stability"
---

Amber 为每项能力打上标签。标签来自 [`docs/CURRENT_SCOPE.md`](https://github.com/zh30/amberjs/blob/main/docs/CURRENT_SCOPE.md)，这是面向用户的唯一事实来源。[兼容性页面](/compatibility) 用可搜索的表格展示同一份数据。

## 四个级别

**Stable。** 能力可从 `amber` 二进制访问，在 `docs/` 下有契约文档，在 `tests/` 下有在 CI 中运行的可执行测试。契约把限制写在 API 旁边。截至 v1.17.0，共有 77 份契约，编号 G1 到 G77。

部分 Stable 条目标注为 **tentative** 或 **provisional**。它们已通过升级规则，但编号或边界仍可能调整。依赖边界行为之前请先阅读契约。

**Preview。** 能力在默认构建中可用，但契约、诊断信息或边界情况仍在收紧。`src/nodejs_core/` 和 `src/web_api/` 中 Stable 契约之外的大部分内容都是 Preview。

**Experimental。** 代码存在，但在重新验证之前不构成产品承诺。包括部分 CLI 命令、可选 Cargo feature 和 N-API hello 加载器。

**不支持。** 目前刻意不做。只要可能，Amber 会抛错或 reject 并给出明确信息，而不是返回假数据。

## 升级规则

只有同时满足以下条件，能力才会升级：

1. 可以通过 `src/main.rs` 或 `src/lib.rs` 中有文档的库 API 访问。
2. 行为描述不依赖历史阶段报告。
3. 当前的测试或冒烟命令覆盖了所描述的行为。
4. 已知限制写在能力旁边。
5. 受 feature 控制的部分有通过的 feature 检查。

规划文档里的清单进度本身不会让任何能力升级。

## 一致性记分卡

`tests/conformance/` 中是 Node 风格的测试夹具，每次 CI 构建都会运行。当前记分卡为 **56/56 PASS**；如果通过数少于 55，或有任何夹具失败或跳过，CI 就会失败。

```bash
cargo build
./tests/conformance/run_conformance.sh
cat tests/conformance/scorecard.md
```

express、fastify、hono 冒烟夹具需要这些包位于 `benchmarks/idle_memory/node_modules`。缺少时会报告 SKIP，门禁随之失败。CI 会先安装它们。

部分夹具覆盖框架冒烟路径或 `http2` 基础行为。夹具通过只是回归门禁，并不会因此让某个 API 成为 Stable。

## 性能

Amber 目前不发布稳定的性能结论。任何数字都必须包含日期、提交、确切命令、构建配置、硬件、测试框架和正确性校验。仓库中较早的基准表格属于历史资料。

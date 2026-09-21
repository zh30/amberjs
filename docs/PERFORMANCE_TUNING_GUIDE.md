# Amber 性能调优指南

本指南只覆盖 Amber v0.1 当前公开 CLI 已支持的调优和验证方式。历史阶段文档中出现的
`--optimize`、`--bytecode-cache`、`--minimal-core`、`--benchmark`、
`--performance-metrics`、`--profile` 等选项不属于当前 v0.1 CLI 合同。

## 基础原则

- 先用 `cargo build --release` 构建 release 二进制。
- 用仓库内示例和测试验证行为，再记录性能数据。
- 性能数字必须附带命令、机器信息、commit/版本和重复次数。
- 不从历史阶段报告复制性能结论作为当前事实。

## 可用命令

```bash
cargo build --release
./target/release/amber eval "1 + 1"
./target/release/amber run examples/basics/hello_world.js
./target/release/amber test examples/testing/math.test.js
```

## 脚本级测量

当前 CLI 没有专用 profiler flags。需要测量脚本时，优先在 JavaScript 内使用
`console.time()` / `console.timeEnd()` 或在外层使用系统工具：

```bash
time ./target/release/amber run examples/basics/hello_world.js
```

示例：

```javascript
console.time("work");
let total = 0;
for (let i = 0; i < 1_000_000; i++) {
  total += i;
}
console.timeEnd("work");
console.log(total);
```

## 发布前性能记录模板

```text
Version/commit:
Machine:
OS:
Command:
Iterations:
Result:
Notes:
```

## 发布闸门

性能相关文档或 README 只能写入来自当前仓库、当前命令、当前环境可复现的数据。
如果没有 fresh benchmark 证据，请只描述能力范围，不写跨 runtime 倍数结论。

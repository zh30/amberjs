---
title: "测试覆盖率与微基准套件 (Testing, Coverage & Benchmarking)"
subtitle: "免配置内置测试运行器、LCOV 覆盖率收集、语言级高精微基准与 CPU 火焰图"
group: "工程工具链"
id: "testing-benchmarking"
---

在工程研发与性能敏感系统中，代码质量测试、覆盖率统计与微基准性能剖析是不可或缺的利器。Beejs 提供了全套原生内置方案，无需安装 Jest、Vitest 或 C8。

---

## 1. 自动化测试与覆盖率 (`bee test --coverage`)

Beejs 具备原生的 Jest/Vitest 兼容测试断言库与测试发现执行器。

### 1.1 基础运行测试

```bash
# 运行所有 *.test.ts 与 *.spec.js 文件
$ bee test

# 运行指定测试文件
$ bee test tests/auth.test.ts
```

### 1.2 收集代码覆盖率 (`--coverage`)
通过 `--coverage` 标志，Beejs 将跟踪脚本行覆盖率并在控制台打印摘要报表，同时在 `coverage/` 目录导出工业级标准的 `lcov.info` 文件（支持无缝集成 Codecov、Coveralls 与 GitHub Actions）：

```bash
$ bee test --coverage
```

控制台报表输出：
```text
=============================== Coverage summary ===============================
Statements   : 96.42% ( 538/558 )
Branches     : 91.17% ( 155/170 )
Functions    : 98.24% ( 112/114 )
Lines        : 96.42% ( 538/558 )
================================================================================
📄 LCOV report saved to: coverage/lcov.info
```

---

## 2. 语言级微基准测试套件 (`bee bench`)

为了精准测量算法优化、序列化或数学计算的性能收益，Beejs 提供了专用的 `bee bench` 套件。

### 2.1 编写 Benchmark 文件

创建形如 `*.bench.ts` 或 `*.bench.js` 的文件：

```typescript
// math.bench.ts
bench("JSON.parse small payload", () => {
  JSON.parse('{"id": 1, "name": "beejs", "ok": true}');
});

bench("Array sort 1000 items", () => {
  const arr = Array.from({ length: 1000 }, (_, i) => 1000 - i);
  arr.sort((a, b) => a - b);
});
```

### 2.2 运行基准

```bash
$ bee bench
# 或指定匹配模式
$ bee bench benchmarks/
```

终端输出美观对齐的统计表格：
```text
🚀 Running benchmarks...

  Benchmark                          Iterations   Avg Time (ns)   Throughput (ops/s)
  ──────────────────────────────────────────────────────────────────────────────────
  JSON.parse small payload               10,000          412 ns        2,427,184/s
  Array sort 1000 items                   2,000       12,850 ns           77,821/s

✨ All benchmarks completed in 42.8ms.
```

---

## 3. V8 CPU 性能剖析 (`bee profile`)

想要诊断 CPU 热点函数与性能瓶颈？`bee profile` 可以对脚本执行过程进行微秒级采样，并导出符合 Chrome DevTools 标准的 `.cpuprofile` 文件。

### 3.1 采样性能剖析

```bash
# 执行并生成 app.cpuprofile
$ bee profile app.ts -o app.cpuprofile
```

### 3.2 在 Chrome 中可视化查看火焰图
1. 打开 Google Chrome 或 Chromium 浏览器；
2. 按 `F12` 打开开发者工具，进入 **Performance**（性能）或 **Memory** 面板；
3. 点击 **Load profile...** 按钮，选择生成的 `app.cpuprofile` 文件；
4. 即可交互式缩放火焰图、检查各函数的 CPU 自耗时（Self Time）与调用树（Call Tree）。

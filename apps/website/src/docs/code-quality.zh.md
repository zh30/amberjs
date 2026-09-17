---
title: "代码格式化与静态检查 (Formatter & Linter)"
subtitle: "由 Rust OXC 驱动的亚毫秒级 AST 格式化与零配置代码规范检查"
group: "工程工具链"
id: "code-quality"
---

在大型代码库中，Prettier 与 ESLint 通常因为体积庞大和启动缓慢而显著拖慢开发效率与 CI 流水线。Beejs 内置了由 **OXC**（The JavaScript Oxidation Compiler）驱动的超高速代码格式化与语法检查器。

---

## 1. 代码格式化 (`bee fmt`)

`bee fmt` 能够以比 Prettier 快数十倍的速度格式化 JavaScript、TypeScript、JSX 和 TSX 文件。

### 1.1 常用命令

```bash
# 格式化单个文件
$ bee fmt src/index.ts

# 递归格式化整个项目目录
$ bee fmt src/

# 多路径格式化
$ bee fmt src/ tests/ examples/
```

### 1.2 CI 纯比对模式 (`--check`)
在持续集成（CI）流水线中，通过 `--check` 验证代码格式是否规范，若有文件需要格式化则退出码非零，不直接修改磁盘文件：

```bash
$ bee fmt src/ --check
```

终端输出示例：
```text
🔍 Checking formatting for 24 files...
Formatted 24 files in 4.2ms (100% compliant)
```

---

## 2. 静态代码检查 (`bee lint`)

`bee lint` 基于 Rust AST 进行极速语法分析与危险模式拦截，无需繁杂的 `.eslintrc` 配置文件即可开箱即用。

### 2.1 运行静态检查

```bash
$ bee lint src/
```

当代码中存在语法错误、未捕获的死循环或危险 API 时，`bee lint` 会以美观终端高亮输出具体位置与修复建议：

```text
🚨 Lint issue in src/auth.ts:18:5
  18 |     eval(userPayload);
     |     ^^^^ no-eval: eval() is unsafe and prohibited in Beejs secure runtime.

Summary: 1 error, 0 warnings found in 18 files (checked in 3.1ms).
```

### 2.2 内置核心规则
- **`no-eval`**：禁止在生产环境调用高危 `eval()` 动态执行；
- **`no-debugger`**：禁止遗留调试断点代码；
- **`no-empty`**：警告空的条件分支或空异常捕获块；
- **`no-dupe-keys`**：检测对象字面量中重复定义的属性键名；
- **`syntax-error`**：严谨的 TypeScript / JavaScript 语法正确性诊断。

---

## 3. 性能基准对比

基于 10,000 行真实 TypeScript 代码库进行格式化与检查对比：

| 工具 | 耗时 | 内存占用 | 依赖要求 |
| :--- | :--- | :--- | :--- |
| **`bee fmt` (OXC)** | **~3.8 ms** | **< 15 MB** | **零依赖** |
| Prettier v3.2 | ~420 ms | ~120 MB | Node.js + npm |
| **`bee lint` (OXC)** | **~4.5 ms** | **< 18 MB** | **零依赖** |
| ESLint v9.0 | ~850 ms | ~180 MB | Node.js + npm |

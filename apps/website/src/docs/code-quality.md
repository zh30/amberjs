---
title: "Code Formatting & Linting (Formatter & Linter)"
subtitle: "Sub-millisecond AST formatting and zero-config diagnostics powered by Rust OXC"
group: "Developer Tooling"
id: "code-quality"
---

In large codebases, Prettier and ESLint often slow down developer loops and CI pipelines due to substantial bootstrap and memory overhead. Beejs integrates a high-performance formatter and linter driven by **OXC** (The JavaScript Oxidation Compiler) written in Rust.

---

## 1. Code Formatting (`bee fmt`)

`bee fmt` formats JavaScript, TypeScript, JSX, and TSX files orders of magnitude faster than conventional tools.

### 1.1 Basic Usage

```bash
# Format a single file
$ bee fmt src/index.ts

# Format an entire directory recursively
$ bee fmt src/

# Format multiple paths
$ bee fmt src/ tests/ examples/
```

### 1.2 CI Check Mode (`--check`)
In CI workflows, use `--check` to verify code conformance without modifying files on disk:

```bash
$ bee fmt src/ --check
```

Example output:
```text
🔍 Checking formatting for 24 files...
Formatted 24 files in 4.2ms (100% compliant)
```

---

## 2. Static Code Linting (`bee lint`)

`bee lint` analyzes AST structures and catches hazardous patterns out of the box without complex configuration files.

### 2.1 Running the Linter

```bash
$ bee lint src/
```

When violations are detected, `bee lint` prints formatted diagnostics with line numbers and recommendations:

```text
🚨 Lint issue in src/auth.ts:18:5
  18 |     eval(userPayload);
     |     ^^^^ no-eval: eval() is unsafe and prohibited in Beejs secure runtime.

Summary: 1 error, 0 warnings found in 18 files (checked in 3.1ms).
```

### 2.2 Built-in Core Rules
- **`no-eval`**: Forbids dangerous dynamic `eval()` execution;
- **`no-debugger`**: Detects leftover debugging statements;
- **`no-empty`**: Catches empty conditional or catch blocks;
- **`no-dupe-keys`**: Disallows duplicate keys in object literals;
- **`syntax-error`**: Rigorous syntax validation for TS/JS files.

---

## 3. Performance Benchmark

Benchmarked on a 10,000-line TypeScript codebase:

| Tool | Duration | Peak Memory | Prerequisites |
| :--- | :--- | :--- | :--- |
| **`bee fmt` (OXC)** | **~3.8 ms** | **< 15 MB** | **Zero dependencies** |
| Prettier v3.2 | ~420 ms | ~120 MB | Node.js + npm |
| **`bee lint` (OXC)** | **~4.5 ms** | **< 18 MB** | **Zero dependencies** |
| ESLint v9.0 | ~850 ms | ~180 MB | Node.js + npm |

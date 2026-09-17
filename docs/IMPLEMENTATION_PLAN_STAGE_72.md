# Stage 72: TypeScript 转译集成与原生支持

## 目标
修复 TypeScript 文件执行问题，实现完整的 TypeScript 原生支持。

## 问题分析

当前 `amberjs run test.ts` 失败的原因：
1. `run_script()` 在 `main.rs` 第 172 行直接将 TypeScript 代码传给 V8
2. 虽然 `transpile_ts` 配置存在，但实际没有调用转译逻辑
3. V8 无法直接执行 TypeScript 语法（如 `: number` 类型标注）

## 解决方案

### Phase 1: 修复转译流程 (核心)
**文件**: `src/main.rs`

在 `run_script()` 函数中，当检测到 TypeScript 文件时，调用 TypeScript 编译器：

```rust
match file_type {
    FileType::TypeScript => {
        // 调用 TypeScript 编译器转译
        let output = amberjs::typescript::compile_typescript(&code, &script_path.to_string_lossy())?;
        let js_code = format!("{}\n{}", setup_code, output.js_code);
        runtime.execute_code(&js_code)?;
    }
    FileType::JavaScript | ... => {
        // 现有逻辑
    }
}
```

### Phase 2: 增强编译器健壮性 (可选)
- 修复现有 TypeScript 编译器的边界情况
- 添加更多语法支持

## 成功标准
- [x] `amberjs run test.ts` 正确执行 TypeScript 代码
- [x] 类型标注被正确移除
- [x] 测试用例通过

## 测试用例
```typescript
// test.ts
const x: number = 42;
const greet = (name: string): string => `Hello, ${name}!`;
console.log("TS Test:", x);
console.log(greet("Amber"));
```

## 估计改动量
- 修改 1 个文件：`src/main.rs`
- 约 20 行代码改动

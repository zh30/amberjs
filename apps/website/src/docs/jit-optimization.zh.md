---
title: "TypeScript 与 TSX 原生支持"
subtitle: "内置基于 Rust 的 oxc 编译器，实现亚毫秒级类型擦除与下一代 JavaScript 特性支持"
group: "核心系统"
id: "jit-optimization"
---

## 1. 零配置的原生 TypeScript 执行

Beejs 坚信：**开发者不应当为运行一行 TypeScript 代码而经历繁琐的编译配置。**

在传统 Node.js 生态中，运行 TypeScript 通常需要安装 `typescript`、`ts-node`、`tsx` 或配置复杂的 `tsconfig.json` 与打包工具（Webpack/Vite）。这不仅增加了项目依赖，且每次启动都会带来明显的预编译延迟。

在 Beejs 中，你可以直接使用 `bee run` 运行任何现代前端或服务端脚本：
- **`.ts`**：标准 TypeScript 模块
- **`.tsx`**：包含 JSX 语法的 TypeScript 组件
- **`.mts` / `.cts`**：明确声明的 ESM / CommonJS TypeScript 模块
- **`.jsx`**：原生 React / JSX 模板

```bash
# 无需 tsc 构建，直接执行
bee run src/app.tsx
```

---

## 2. 为什么选用 oxc 编译器？

Beejs 内置了由 Rust 编写的高性能 **oxc**（The Oxidation Compiler）编译器作为转译中枢：

- **极致速度**：oxc 是当前业界公认最快的 JavaScript/TypeScript 解析器与转译器之一，其 AST 解析与类型擦除速度达到传统 `tsc` 的 **30~50 倍**，单个千行文件转译耗时通常低于 **1ms**；
- **纯内存处理**：转译完全在 Rust 内存中完成，直接将清洗后的合法 JavaScript 字节码送入 V8 JIT 管道，零中间磁盘临时文件产生；
- **准确语义降级**：严格对齐 TypeScript 官方规范与 TC39 最新提案。

---

## 3. 支持的下一代语言特性

除了基础的类型标注擦除，Beejs 对现代 ECMAScript 提案与 TypeScript 6.0 提供了全面的前瞻支持：

### 1. Stage 3 装饰器 (Decorators)
无需任何 experimental 标志，原生支持 TC39 标委会正式确立的 Stage 3 装饰器标准：

```typescript
// decorators.ts
function logged(value: any, context: ClassMethodDecoratorContext) {
  const methodName = String(context.name);
  return function (this: any, ...args: any[]) {
    console.log(`[LOG] Calling method ${methodName} with args:`, args);
    const result = value.apply(this, args);
    console.log(`[LOG] Method ${methodName} returned:`, result);
    return result;
  };
}

class Calculator {
  @logged
  add(a: number, b: number): number {
    return a + b;
  }
}

const calc = new Calculator();
calc.add(10, 25);
```

运行输出：
```text
[LOG] Calling method add with args: [ 10, 25 ]
[LOG] Method add returned: 35
```

### 2. 显式资源管理 (`using` / `await using`)
支持 ECMAScript Explicit Resource Management 提案。通过 `Symbol.dispose` 与 `Symbol.asyncDispose`，在离开作用域时自动确定性地关闭文件描述符、数据库连接或网络套接字，从根本上消灭资源泄漏：

```typescript
// resource.ts
class DatabaseConnection implements Disposable {
  constructor(public id: string) {
    console.log(`🔌 打开数据库连接: ${this.id}`);
  }

  query(sql: string) {
    return `Query result for "${sql}" on ${this.id}`;
  }

  [Symbol.dispose]() {
    console.log(`🔒 作用域结束，自动关闭连接: ${this.id}`);
  }
}

function processTransaction() {
  // 使用 using 关键字，函数退出时自动触发 [Symbol.dispose]
  using db = new DatabaseConnection('conn_9981');
  console.log(db.query('SELECT * FROM users'));
}

processTransaction();
console.log('事务执行完毕');
```

运行输出：
```text
🔌 打开数据库连接: conn_9981
Query result for "SELECT * FROM users" on conn_9981
🔒 作用域结束，自动关闭连接: conn_9981
事务执行完毕
```

### 3. TSX / JSX 原生降级
Beejs 原生支持 JSX 语法，将其自动转换为标准的 `React.createElement` 调用：

```tsx
// component.tsx
import React from 'react';

interface CardProps {
  title: string;
  count: number;
}

const StatCard: React.FC<CardProps> = ({ title, count }) => {
  return (
    <div className="card">
      <h3>{title}</h3>
      <span className="badge">{count * 2}</span>
    </div>
  );
};

console.log(StatCard({ title: '活跃 Worker', count: 8 }));
```

---

## 4. 类型擦除与类型检查的最佳实践

### 运行时仅做类型擦除 (Transpile-only)
与 Deno 和 Bun 一致，Beejs 在运行脚本时采用 **Transpile-only** 策略：
- **目标是极致启动速度**：在生产环境和开发频繁调试时，类型检查（Type Checking）是非常耗时且冗余的操作。因此，Beejs 仅在内存中擦除类型并降级语法，使启动时间控制在 18ms 以内；
- **类型错误宽容度**：如果代码中包含 TypeScript 类型错误，只要生成的底层 JavaScript 符合语法规范，代码依然会正常执行。

### 推荐的类型检查工作流
为了保证大型工程代码的类型绝对安全，推荐将“代码执行”与“静态检查”解耦：

1. **本地开发**：借助 VS Code、WebStorm 等 IDE 内置的 TypeScript 语言服务器（Language Server），在键入代码时实时获取错误波浪线与智能提示；
2. **本地调试与部署**：使用 `bee run` 极速运行，摆脱编译等待；
3. **CI/CD 流水线**：在代码提交或 PR 合并前，执行 `tsc --noEmit` 进行全量无产物类型审计：

```json
{
  "scripts": {
    "start": "bee run src/index.ts",
    "check": "tsc --noEmit"
  }
}
```

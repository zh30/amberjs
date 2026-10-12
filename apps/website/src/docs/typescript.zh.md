---
title: "TypeScript"
nav: "TypeScript"
subtitle: "直接运行 .ts 和 .tsx 文件。Amber 用 oxc 转译，不做类型检查。"
status: "stable"
id: "typescript"
---

TypeScript 仅转译（transpile-only）是 **Stable**（G36），契约见 [`docs/TYPESCRIPT_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/TYPESCRIPT_CONTRACT.md)。

## 哪些会被转译

- 以 `.ts`、`.tsx`、`.mts`、`.cts` 或 `.jsx` 结尾的入口文件。
- 通过 CommonJS `require` 加载、使用上述扩展名的文件。
- 看起来像 TypeScript 或 JSX 的 `amber eval` 片段。

```ts
// app.ts
type Status = "ok" | "error";

function report(status: Status): string {
  return `status=${status}`;
}

console.log(report("ok"));
```

```bash
amber run app.ts
```

## 工作方式

Amber 用 oxc 擦除类型并生成 JavaScript，再在 V8 上运行。

- `using` 声明和 Stage 3 装饰器会降级到 ES2022。
- TSX 生成经典的 `React.createElement` 调用。
- 启用 source map 时，错误堆栈指向 `.ts` 源码行。
- 解析或转换失败会让运行失败，Amber 不会猜测。

## 不做的事

这**不是** `tsc`。以下是该能力永久的非目标：

- 类型检查。有类型错误的文件仍会运行。
- `tsconfig.json` 工程、路径别名和项目引用。
- 自动 JSX runtime 和 `emitDecoratorMetadata`。

如果需要让类型错误阻断构建，请在 CI 中运行 `tsc --noEmit`。

## Amber API 的类型定义

`amber types` 会打印 Amber 内置 API 的类型定义。该命令尚未在 scope 页面中分级，请把输出当作 Experimental。

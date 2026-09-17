---
title: "官方 TypeScript 类型定义 (TypeScript Types)"
subtitle: "一键导出覆盖 bee:ai、Web 标准 API 与全局命名空间的官方类型声明"
group: "Agent 与高级特性"
id: "types-lsp"
---

为了在现代 IDE（VS Code、WebStorm、Cursor）中获得丝滑的代码补全、参数提示与类型校验，Beejs 提供了全量内置的官方 TypeScript 类型声明文件 (`types/beejs.d.ts`)。

---

## 1. 导出类型文件 (`bee types`)

Beejs 将官方类型定义内嵌于运行时二进制中，使用 `bee types` 即可零网络开销、亚毫秒级导出声明文件：

```bash
# 直接在控制台打印类型声明
$ bee types

# 导出至项目根目录，供 tsconfig.json 引用
$ bee types -o beejs.d.ts
```

终端输出：
```text
📄 TypeScript definition written to: beejs.d.ts
```

---

## 2. 在项目中配置 TypeScript

在你的 `tsconfig.json` 中，通过 `include` 或 `files` 引用导出的声明文件：

```json
{
  "compilerOptions": {
    "target": "ESNext",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "skipLibCheck": true
  },
  "include": ["src/**/*", "beejs.d.ts"]
}
```

---

## 3. 涵盖的核心类型体系

导出的类型声明涵盖 Beejs 特有的全部底层基础设施：

### 3.1 `bee:ai` 模块
包含张量运算与本地大模型流式推理定义：
```typescript
import { Tensor, LLM, AgentPipeline } from "bee:ai";

// 强类型矩阵与相似度运算
const a = new Tensor([1.0, 2.0, 3.0], [1, 3]);
const similarity: number = a.cosineSimilarity(new Tensor([2.0, 4.0, 6.0], [1, 3]));

// 本地推理引擎类型
const llm = new LLM({ model: "llama-3.2-1b" });
```

### 3.2 `bee:bench` 模块
包括微基准定义函数类型：
```typescript
import { bench } from "bee:bench";

bench("My Operation", () => {
  // benchmark code
});
```

### 3.3 全局与 Web 标准扩展
全面补全全局作用域下的 `Request`, `Response`, `fetch`, `Worker`, `WebSocket`, `BroadcastChannel`, `Performance`, 以及 `process.dlopen` 等特有接口。

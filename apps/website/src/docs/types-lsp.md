---
title: "Official TypeScript Type Definitions (TypeScript Types)"
subtitle: "Export comprehensive declarations covering amber:ai, Web APIs, and runtime globals"
group: "Agent & Advanced"
id: "types-lsp"
---

For smooth auto-completion, parameter hints, and type safety in modern IDEs (VS Code, Cursor, WebStorm), Amber includes full official TypeScript type definitions (`types/amberjs.d.ts`).

---

## 1. Exporting Definitions (`amber types`)

Definitions are embedded directly inside the compiled binary. Use `amber types` to export them instantly:

```bash
# Print definitions directly to stdout
$ amber types

# Export to your project root
$ amber types -o amberjs.d.ts
```

Console output:
```text
📄 TypeScript definition written to: amberjs.d.ts
```

---

## 2. Project Configuration

Add the exported definition file to your `tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ESNext",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "skipLibCheck": true
  },
  "include": ["src/**/*", "amberjs.d.ts"]
}
```

---

## 3. Covered Type APIs

The declaration file provides full typings for Amber infrastructure:

### 3.1 `amber:ai` Module
Typed tensor math and local LLM inference engines:
```typescript
import { Tensor, LLM, AgentPipeline } from "amber:ai";

const a = new Tensor([1.0, 2.0, 3.0], [1, 3]);
const sim: number = a.cosineSimilarity(new Tensor([2.0, 4.0, 6.0], [1, 3]));

const llm = new LLM({ model: "llama-3.2-1b" });
```

### 3.2 `amber:bench` Module
Microbenchmark runner signatures:
```typescript
import { bench } from "amber:bench";

bench("Hash Computation", () => {
  // benchmark body
});
```

### 3.3 Global & Web API Extensions
Adds complete typings for `Request`, `Response`, `fetch`, `Worker`, `WebSocket`, `BroadcastChannel`, and `process.dlopen`.

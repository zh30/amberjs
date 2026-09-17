---
title: "TypeScript & TSX Native Engine"
subtitle: "Embedded oxc Rust compiler delivering sub-millisecond type erasure and next-gen syntax features"
group: "Core Systems"
id: "jit-optimization"
---

## 1. Zero-Config Native TypeScript Execution

Beejs operates on a simple principle: **Developers should never have to configure a compiler just to run a TypeScript file.**

In traditional Node.js setups, running TypeScript requires installing `typescript`, `ts-node`, `tsx`, or writing complex `tsconfig.json` and bundler configurations. This introduces heavy dependencies and noticeable startup lags.

With Beejs, you can run any modern script file directly with `bee run`:
- **`.ts`**: Standard TypeScript modules
- **`.tsx`**: TypeScript components with JSX syntax
- **`.mts` / `.cts`**: Explicit ESM or CommonJS TypeScript modules
- **`.jsx`**: React and JSX templates

```bash
# Execute directly without tsc compilation
bee run src/app.tsx
```

---

## 2. Why the oxc Compiler?

Beejs embeds the high-performance **oxc** (The Oxidation Compiler) written in Rust:

- **Blazing Fast**: oxc is widely recognized as one of the fastest JS/TS parsers and transpilers available. It parses and strips types **30x–50x faster** than official `tsc`, transforming typical 1,000-line files in under **1 millisecond**.
- **Pure In-Memory Execution**: Transpilation happens entirely in memory. The sanitized JavaScript bytecode is handed directly to the V8 JIT pipeline without writing temporary files to disk.
- **Accurate Syntax Downleveling**: Strict adherence to TC39 stage proposals and TypeScript 6.0 semantics.

---

## 3. Next-Gen Language Features Supported

Beyond standard type stripping, Beejs includes out-of-the-box support for modern language capabilities:

### 1. Stage 3 Decorators
Native support for TC39 Stage 3 standard decorators without experimental compiler flags:

```typescript
// decorators.ts
function logged(value: any, context: ClassMethodDecoratorContext) {
  const methodName = String(context.name);
  return function (this: any, ...args: any[]) {
    console.log(`[LOG] Calling ${methodName} with args:`, args);
    const result = value.apply(this, args);
    console.log(`[LOG] ${methodName} returned:`, result);
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

Execution output:
```text
[LOG] Calling add with args: [ 10, 25 ]
[LOG] add returned: 35
```

### 2. Explicit Resource Management (`using` / `await using`)
Supports the TC39 Explicit Resource Management standard. By implementing `Symbol.dispose` and `Symbol.asyncDispose`, file handles, database connections, and sockets are deterministically cleaned up when leaving scope:

```typescript
// resource.ts
class DatabaseConnection implements Disposable {
  constructor(public id: string) {
    console.log(`🔌 Opening connection: ${this.id}`);
  }

  query(sql: string) {
    return `Query result for "${sql}" on ${this.id}`;
  }

  [Symbol.dispose]() {
    console.log(`🔒 Exiting scope, automatically closing: ${this.id}`);
  }
}

function processTransaction() {
  using db = new DatabaseConnection('conn_9981');
  console.log(db.query('SELECT * FROM users'));
}

processTransaction();
console.log('Transaction finished');
```

Execution output:
```text
🔌 Opening connection: conn_9981
Query result for "SELECT * FROM users" on conn_9981
🔒 Exiting scope, automatically closing: conn_9981
Transaction finished
```

### 3. TSX / JSX Classic Downleveling
JSX tags are transformed into standard `React.createElement` calls at sub-millisecond speeds:

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

console.log(StatCard({ title: 'Active Workers', count: 8 }));
```

---

## 4. Best Practices: Transpilation vs Type Checking

### Transpile-Only Runtime Philosophy
Like Bun and Deno, Beejs uses a **transpile-only** strategy when running code:
- **Optimized for Startup Speed**: Type checking on every script invocation is redundant and slow. Stripping types in memory keeps CLI cold starts strictly below 18ms.
- **Type Error Forgiveness**: If a file has type mismatches, it will still execute as long as the generated JavaScript syntax is valid.

### Recommended Production Workflow
Decouple runtime execution from static analysis for optimal developer velocity:

1. **Local Development**: Let your IDE (VS Code, WebStorm) handle real-time type squiggles and autocompletion via the TypeScript language server.
2. **Execution**: Run directly with `bee run` for instant feedback and zero build wait.
3. **CI Pipeline**: Run `tsc --noEmit` before merging pull requests to guarantee comprehensive static type safety:

```json
{
  "scripts": {
    "start": "bee run src/index.ts",
    "check": "tsc --noEmit"
  }
}
```

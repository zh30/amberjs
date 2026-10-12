---
title: "TypeScript"
nav: "TypeScript"
subtitle: "Run .ts and .tsx files directly. Amber transpiles with oxc and does not type-check."
status: "stable"
id: "typescript"
---

TypeScript transpile-only is **Stable** (G36). The contract is [`docs/TYPESCRIPT_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/TYPESCRIPT_CONTRACT.md).

## What runs

- Entry files ending in `.ts`, `.tsx`, `.mts`, `.cts`, or `.jsx`.
- Files loaded with CommonJS `require` that use those extensions.
- `amber eval` snippets that look like TypeScript or JSX.

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

## How it works

Amber uses oxc to strip types and emit JavaScript, then runs it on V8.

- `using` declarations and Stage 3 decorators are lowered to ES2022.
- TSX emits classic `React.createElement` calls.
- When a source map is active, stack traces point at `.ts` lines.
- A parse or transform error fails the run. Amber does not guess.

## What it does not do

This is **not** `tsc`. These are permanent non-goals for this surface:

- Type checking. A file with type errors still runs.
- `tsconfig.json` programs, path aliases, and project references.
- The automatic JSX runtime and `emitDecoratorMetadata`.

Run `tsc --noEmit` in CI if you need type errors to fail the build.

## Type definitions for Amber APIs

`amber types` prints type definitions for Amber built-ins. That command is not classified in the scope page yet, so treat its output as Experimental.

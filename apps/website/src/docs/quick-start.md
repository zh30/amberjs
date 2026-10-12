---
title: "Quick start"
nav: "Quick start"
subtitle: "Run a script, a TypeScript file, a test, and an HTTP handler in a few minutes."
status: "stable"
id: "quick-start"
---

## 1. Evaluate a snippet

```bash
amber eval "1 + 1"
```

## 2. Run a JavaScript file

```js
// hello.js
const os = require("node:os");
console.log(`Hello from Amber on ${os.platform()}`);
```

```bash
amber run hello.js
```

## 3. Run TypeScript directly

There is no build step. Amber erases types with oxc and runs the result.

```ts
// greet.ts
interface User {
  name: string;
}

const greet = (user: User): string => `Hello, ${user.name}`;
console.log(greet({ name: "Amber" }));
```

```bash
amber run greet.ts
```

Types are **not** checked. Run `tsc --noEmit` in CI if you want type errors. See [TypeScript](/docs/typescript).

## 4. Write a test

```js
// math.test.js
describe("math", () => {
  test("adds numbers", () => {
    expect(1 + 2).toBe(3);
  });
});
```

```bash
amber test math.test.js
amber test          # discover every test file under the current directory
amber test --watch  # re-run when a watched file changes
```

## 5. Serve HTTP

```js
// server.js
module.exports = {
  fetch(request) {
    return new Response("Hello from Amber");
  },
};
```

```bash
amber serve --port 3000 server.js
curl http://localhost:3000/
```

`amber serve` handles one HTTP/1.1 request per connection. It is a small handler bridge, not a general web server. See [Server mode](/docs/server-mode).

## 6. Use the REPL

```bash
amber repl
```

## Where next

- [CLI reference](/docs/cli-usage) lists every command and its maturity.
- [Node.js compatibility](/docs/node-compat) shows which modules are Stable.
- [Agent sandbox](/docs/agent-sandbox) shows how to run untrusted tool code.

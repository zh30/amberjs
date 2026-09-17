---
title: "Quick Start"
subtitle: "Run TypeScript, tests, and a tiny HTTP handler with bee"
group: "Getting Started"
id: "quick-start"
---

## 1. First script

Beejs runs `.ts` / `.tsx` without `tsc` or `ts-node`. Types are stripped by oxc, then executed on V8.

Create `hello.ts`:

```ts
const runtime = "Beejs";
console.log(`hello from ${runtime}`);
```

```bash
bee run hello.ts
# hello from Beejs
```

There is no project-wide typecheck. Use `tsc --noEmit` in CI if you want that.

One-liners and a REPL:

```bash
bee eval "console.log(crypto.randomUUID())"
bee repl
```

---

## 2. Tests

Jest-style `describe` / `test` / `expect`. Stable since v1.15.0.

```js
// math.test.js
describe("math", () => {
  test("adds numbers", () => {
    expect(2 + 3).toBe(5);
  });
});
```

```bash
bee test
bee test math.test.js
bee test --watch
```

`bee test --parallel` is rejected (exit code 2): V8 isolates are not shared across threads.

---

## 3. HTTP (Preview)

`bee serve` loads a module that exports `fetch`:

```js
// app.js
module.exports = {
  fetch() {
    return new Response("ok");
  },
};
```

```bash
bee serve app.js --host 127.0.0.1 --port 3000
```

You can also write a `node:http` server and `bee run server.ts`. TLS is rustls HTTP/1.1 when `--https --cert --key` are set.

---

## 4. Arguments

Arguments after the filename are `process.argv`:

```ts
console.log(process.argv.slice(2));
```

```bash
bee run cli.ts --name myapp --port 8080
# [ '--name', 'myapp', '--port', '8080' ]
```

---

## 5. Watch

```bash
bee run --watch hello.ts
bee run --watch --debounce 200 app.ts
```

---

## 6. Suggested layout

```text
my-bee-app/
├── package.json
├── tsconfig.json          # optional, for the editor
├── src/index.ts
└── tests/math.test.js
```

```json
{
  "name": "my-bee-app",
  "scripts": {
    "dev": "bee run --watch src/index.ts",
    "start": "bee run src/index.ts",
    "test": "bee test"
  }
}
```

Next: [CLI reference](/docs/cli-usage) · [sandbox](/docs/agent-sandbox) · [bundle / compile](/docs/bundling-compilation)

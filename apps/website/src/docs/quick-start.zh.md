---
title: "快速开始"
subtitle: "用 bee 跑 TypeScript、测试和一个很小的 HTTP 处理器"
group: "开始"
id: "quick-start"
---

## 1. 第一段脚本

Amber 不需要 `tsc` 或 `ts-node` 就能跑 `.ts` / `.tsx`。oxc 擦掉类型，再交给 V8。

新建 `hello.ts`：

```ts
const runtime = "Amber";
console.log(`hello from ${runtime}`);
```

```bash
amber run hello.ts
# hello from Amber
```

运行时不做全项目类型检查。需要的话在 CI 里跑 `tsc --noEmit`。

一行表达式和 REPL：

```bash
amber eval "console.log(crypto.randomUUID())"
amber repl
```

---

## 2. 测试

Jest 风格的 `describe` / `test` / `expect`。自 v1.15.0 起为 Stable。

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
amber test math.test.js
amber test --watch
```

`amber test --parallel` 会被拒绝（退出码 2）：V8 isolate 不能跨线程共享。

---

## 3. HTTP（Preview）

`bee serve` 加载导出 `fetch` 的模块：

```js
// app.js
module.exports = {
  fetch() {
    return new Response("ok");
  },
};
```

```bash
amber serve app.js --host 127.0.0.1 --port 3000
```

也可以写 `node:http` 服务器再用 `amber run server.ts`。`--https --cert --key` 时使用 rustls HTTP/1.1。

---

## 4. 命令行参数

文件名后面的参数在 `process.argv`：

```ts
console.log(process.argv.slice(2));
```

```bash
amber run cli.ts --name myapp --port 8080
# [ '--name', 'myapp', '--port', '8080' ]
```

---

## 5. Watch

```bash
amber run --watch hello.ts
amber run --watch --debounce 200 app.ts
```

---

## 6. 建议目录

```text
my-bee-app/
├── package.json
├── tsconfig.json          # 可选，给编辑器用
├── src/index.ts
└── tests/math.test.js
```

```json
{
  "name": "my-bee-app",
  "scripts": {
    "dev": "amber run --watch src/index.ts",
    "start": "amber run src/index.ts",
    "test": "bee test"
  }
}
```

接下来：[CLI 参考](/docs/cli-usage) · [沙箱](/docs/agent-sandbox) · [打包 / 编译](/docs/bundling-compilation)

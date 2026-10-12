---
title: "快速开始"
nav: "快速开始"
subtitle: "几分钟内运行脚本、TypeScript 文件、测试和 HTTP 处理函数。"
status: "stable"
id: "quick-start"
---

## 1. 求值一段代码

```bash
amber eval "1 + 1"
```

## 2. 运行 JavaScript 文件

```js
// hello.js
const os = require("node:os");
console.log(`Hello from Amber on ${os.platform()}`);
```

```bash
amber run hello.js
```

## 3. 直接运行 TypeScript

不需要构建步骤。Amber 用 oxc 擦除类型后直接运行。

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

类型**不会**被检查。如果需要类型错误，请在 CI 中运行 `tsc --noEmit`。详见 [TypeScript](/docs/typescript)。

## 4. 写一个测试

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
amber test          # 发现当前目录下所有测试文件
amber test --watch  # 被监听的文件变化时重新运行
```

## 5. 提供 HTTP 服务

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

`amber serve` 每个连接处理一个 HTTP/1.1 请求。它是一个小型处理函数桥接，不是通用 Web 服务器。详见 [Server 模式](/docs/server-mode)。

## 6. 使用 REPL

```bash
amber repl
```

## 接下来

- [CLI 参考](/docs/cli-usage) 列出了每个命令及其成熟度。
- [Node.js 兼容性](/docs/node-compat) 说明哪些模块是 Stable。
- [Agent 沙箱](/docs/agent-sandbox) 介绍如何运行不受信任的工具代码。

---
title: "主流 npm 框架兼容性验证 (Hono, Express, LangChain)"
subtitle: "面向全栈 Web 与 AI Agent 框架的生产级 Web 标准与 Node.js 兼容性保障"
group: "生产与架构"
id: "framework-compat"
---

## 1. 概述与设计理念

Beejs 的目标不仅是运行简单的脚本，更致力于成为现代主流 TypeScript/JavaScript 框架的无缝运行底座。

在 **Beejs v1.5.0** 中，我们对三大核心生态支柱进行了全方位对齐与强化验证：
1. **Hono**：基于 Web 标准的极速轻量级 Web 框架。
2. **Express**：经典的 Node.js 企业级服务端框架。
3. **LangChain**：主流的大模型应用开发与多 Agent 编排框架。

---

## 2. Hono 框架在 Beejs 上的运行

Hono 深度依赖 Web 标准接口（`Request`, `Response`, `Headers`, `fetch`）以及 Node.js 的 `AsyncLocalStorage` 进行请求上下文传递。

### v1.5.0 关键特性赋能
- **`Response.json(data, init)`**：原生静态工厂方法，自动注入 `application/json` 头并序列化。
- **`Response.redirect(url, status)`**：标准重定向支持。
- **`Headers.prototype.getSetCookie()`**：标准的 Set-Cookie 数组提取方法。
- **`AsyncLocalStorage` 跨异步边界上下文保持**：彻底修复跨 `async/await` Promise 解析时的上下文丢失，保障 `hono/context-storage` 正常工作。

### 示例：运行 Hono API

```typescript
import { Hono } from 'hono';
import { AsyncLocalStorage } from 'node:async_hooks';

const app = new Hono();
const als = new AsyncLocalStorage();

// 上下文传递中间件
app.use('*', async (c, next) => {
  return als.run({ traceId: 'bee-trace-001' }, async () => {
    await next();
  });
});

app.get('/api/health', (c) => {
  const store = als.getStore();
  return c.json({ status: 'healthy', traceId: store?.traceId });
});

export default app;
```

---

## 3. Express 框架在 Beejs 上的运行

Express 应用依赖于 `http.IncomingMessage`、`http.ServerResponse` 的事件流机制与方法增强。

### v1.5.0 关键特性赋能
- **`res.status(code).json(body)` / `res.send()`**：流畅的链式响应调用。
- **`res.set()`, `res.header()`, `res.get()`**：Express 标准头操作别名。
- **`req.socket` 与 `req.headers`**：完整的网络底层连接信息与标头数据。
- **`stream/promises` (`pipeline`, `finished`)**：基于 Promise 的数据流管道。
- **`timers/promises` (`setTimeout`, `setImmediate`)**：现代异步等待。

### 示例：Express 风格服务端

```typescript
import http from 'node:http';
import { setTimeout } from 'node:timers/promises';

const server = http.createServer(async (req, res) => {
  if (req.method === 'POST' && req.url === '/process') {
    await setTimeout(20);
    return res.status(200).json({ processed: true, client: req.socket?.remoteAddress });
  }

  res.status(404).send('Not Found');
});

server.listen(3000, () => {
  console.log('服务已在端口 3000 启动');
});
```

---

## 4. LangChain 在 Beejs 上的运行

LangChain 与各类大模型应用深度使用 **Web Streams**、Server-Sent Events (SSE) 流式传输以及异步迭代。

### v1.5.0 关键特性赋能
- **`ReadableStream.from(iterable)`**：直接从 Token 数组、生成器或异步生成器构造流。
- **`for await (const chunk of stream)`**：在所有 ReadableStream 上原生支持异步迭代器。
- **`TransformStream` 管道流**：通过 `stream.pipeThrough(transformer)` 实现多阶段 Token 转换。
- **`AbortSignal.timeout(ms)` 与 `AbortSignal.any()`**：自动超时控制与多信号联合取消。

### 示例：LLM Token 流式处理管道

```typescript
// 模拟来自 Beejs 本地 Edge SLM 的 Token 流
const tokenStream = ReadableStream.from([
  '思考', '中', '...', '分析', '完成', '！'
]);

// 构造 SSE 格式转换流
const sseTransform = new TransformStream({
  transform(token, controller) {
    controller.enqueue(`data: ${JSON.stringify({ token })}\n\n`);
  }
});

const outputStream = tokenStream.pipeThrough(sseTransform);

// 异步流式输出 Token
for await (const sseChunk of outputStream) {
  process.stdout.write(sseChunk);
}
```

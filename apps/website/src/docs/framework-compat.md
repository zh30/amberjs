---
title: "Mainstream npm Framework Compatibility (Hono, Express, LangChain)"
subtitle: "Production-ready Web standards and Node.js runtime compatibility for full-stack and AI frameworks"
group: "Production & Architecture"
id: "framework-compat"
---

## 1. Overview

Beejs is designed not just for scripts, but as a drop-in execution platform for mainstream TypeScript and JavaScript frameworks.

In **Beejs v1.5.0**, we completed end-to-end verification and API hardening for three ecosystem pillars:
1. **Hono**: The ultra-fast, lightweight web framework built on Web Standards.
2. **Express**: The industry standard Node.js server framework.
3. **LangChain**: The leading LLM application and agent orchestration framework.

---

## 2. Hono Framework on Beejs

Hono relies heavily on standard Web APIs (`Request`, `Response`, `Headers`, `fetch`) and Node's `AsyncLocalStorage` for request context storage.

### Key Capabilities Enabled in v1.5.0
- **`Response.json(data, init)`**: Native static factory producing JSON responses with proper headers.
- **`Response.redirect(url, status)`**: Standard redirect support.
- **`Headers.prototype.getSetCookie()`**: Standard cookie extraction array.
- **`AsyncLocalStorage` Context Storage**: Guaranteed context propagation across `async/await` promise boundaries in `hono/context-storage`.

### Example: Running Hono API

```typescript
import { Hono } from 'hono';
import { AsyncLocalStorage } from 'node:async_hooks';

const app = new Hono();
const als = new AsyncLocalStorage();

// Context middleware
app.use('*', async (c, next) => {
  return als.run({ traceId: 'bee-req-999' }, async () => {
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

## 3. Express Framework on Beejs

Express applications require full fidelity on `http.IncomingMessage`, `http.ServerResponse`, and Node stream pipelines.

### Key Capabilities Enabled in v1.5.0
- **`res.status(code).json(body)`**: Fluent response methods.
- **`res.set()`, `res.header()`, `res.get()`**: Express header shortcuts.
- **`req.socket` & `req.headers`**: Full network connection and header metadata.
- **`stream/promises` (`pipeline`, `finished`)**: Promise-based stream piping.
- **`timers/promises` (`setTimeout`, `setImmediate`)**: Modern async sleep and scheduling.

### Example: Express Microservice

```typescript
import http from 'node:http';
import { pipeline } from 'node:stream/promises';
import { setTimeout } from 'node:timers/promises';

const server = http.createServer(async (req, res) => {
  if (req.method === 'POST' && req.url === '/process') {
    // Simulate async pipeline work
    await setTimeout(20);
    return res.status(200).json({ processed: true, client: req.socket?.remoteAddress });
  }

  res.status(404).send('Not Found');
});

server.listen(3000, () => {
  console.log('Server listening on port 3000');
});
```

---

## 4. LangChain on Beejs

LangChain and LLM orchestrators make heavy use of **Web Streams**, SSE streaming, and async context tracking.

### Key Capabilities Enabled in v1.5.0
- **`ReadableStream.from(iterable)`**: Creates readable streams directly from token arrays, generators, or async generators.
- **`for await (const chunk of stream)`**: Async iterator support on all `ReadableStream` instances.
- **`TransformStream` Piping**: Full pipeline streaming via `stream.pipeThrough(transformer)`.
- **`AbortSignal.timeout(ms)` & `AbortSignal.any()`**: Automatic LLM call timeout and combined cancellation.

### Example: LLM Token Streaming Pipeline

```typescript
// Token stream from Beejs Edge SLM or external provider
const tokenStream = ReadableStream.from([
  'Thinking', ' ', 'step', ' ', 'by', ' ', 'step', '...'
]);

// Transform stream to format SSE chunks
const sseTransform = new TransformStream({
  transform(token, controller) {
    controller.enqueue(`data: ${JSON.stringify({ token })}\n\n`);
  }
});

const outputStream = tokenStream.pipeThrough(sseTransform);

// Asynchronously consume streaming tokens
for await (const sseChunk of outputStream) {
  process.stdout.write(sseChunk);
}
```

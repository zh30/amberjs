---
title: "现代 Web 服务与高并发架构"
subtitle: "支持标准 Web Fetch API (bee serve)、node:http 以及无锁多 Isolate 线程池"
group: "核心系统"
id: "server-mode"
---

Beejs 提供了双重现代 Web 服务构建范式：

1. **现代化 Web 标准服务 (`bee serve [file]`)**：基于符合 W3C / WinterCG 规范的 `Request` / `Response` 与 `fetch(req)` 导出模型；
2. **Node.js 兼容服务 (`bee run server.ts`)**：基于 `node:http` 与底层 Rust Tokio 无锁多 Worker 线程池模型。

---

## 1. 现代化 Web 应用服务 (`bee serve`)

`bee serve` 是 Beejs 官方推荐的现代轻量 Web 服务入口，体验对齐 Cloudflare Workers、Deno 与 Bun，原生支持 TypeScript 与 JSX。

### 1.1 编写首个 Web 服务脚本

只需在脚本中导出一个包含 `fetch` 处理函数的对象，或直接导出处理函数：

```typescript
// app.ts
export default {
  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);

    // 路由匹配
    if (url.pathname === "/") {
      return new Response("🚀 Welcome to Beejs Web Server!");
    }

    if (url.pathname === "/api/echo" && req.method === "POST") {
      const data = await req.json();
      return new Response(JSON.stringify({ received: data, time: Date.now() }), {
        status: 200,
        headers: { "Content-Type": "application/json" }
      });
    }

    if (url.pathname === "/api/info") {
      return new Response(JSON.stringify({
        runtime: "beejs",
        version: "v1.16.0",
        arch: process.arch,
        platform: process.platform
      }), {
        headers: { "Content-Type": "application/json", "X-Powered-By": "beejs" }
      });
    }

    return new Response("Not Found", { status: 404 });
  }
};
```

### 1.2 启动服务

```bash
# 自动探测并运行 app.ts, app.js, server.ts, index.ts 等
$ bee serve

# 或显式指定文件与端口/主机
$ bee serve app.ts --port 8080 --host 0.0.0.0
```

终端输出：

```text
🚀 Starting Beejs Web Server on http://0.0.0.0:8080
📄 Serving application: app.ts
✅ Listening on http://0.0.0.0:8080 (Ctrl+C to stop)
```

### 1.3 核心技术亮点

- **零胶水代码**：运行时直接将底层 TCP HTTP 报文解构映射为标准的 `Request` 实例，Headers 与 Body 均无缝桥接；
- **原生异步微任务支持**：支持 `async` 函数与 Promise，在事件循环中自适应执行微任务检查点；
- **完整的 Body Mixin**：`Request` 与 `Response` 均完整实现 `req.text()`, `req.json()`, `req.arrayBuffer()`；
- **资源沙箱集成**：可搭配 `--max-memory <MB>` 与 `--sandbox` 一同使用，限制 Web 应用的资源消耗。

---

## 2. 传统 Node.js 兼容服务 (`node:http`)

如果你正在迁移基于 Node.js 生态构建的微服务或 Express 风格代码，可以使用熟悉的 `node:http`：

```typescript
// server.ts - 经典 Node.js 风格服务
import http from 'node:http';

const server = http.createServer(async (req, res) => {
  const { method, url } = req;
  const parsedUrl = new URL(url || '/', `http://${req.headers.host}`);

  res.setHeader('Content-Type', 'application/json; charset=utf-8');
  res.setHeader('Access-Control-Allow-Origin', '*');

  if (method === 'GET' && parsedUrl.pathname === '/api/users') {
    res.writeHead(200);
    res.end(JSON.stringify({ code: 0, data: ['Alice', 'Bob'] }));
    return;
  }

  res.writeHead(404);
  res.end(JSON.stringify({ error: 'Not Found' }));
});

server.listen(3000, () => {
  console.log('🚀 HTTP 服务就绪: http://localhost:3000');
});
```

启动命令：

```bash
bee run server.ts
```

---

## 3. 多 Worker 线程池并发架构 (`--workers`)

### 传统单线程事件循环的局限

在传统单线程事件循环（如单进程 Node.js）中，一旦某个请求执行密集的 JSON 序列化、密码学哈希或张量推理，事件循环便会发生卡顿，导致正在排队的所有并发请求延迟剧增。

### Beejs 的无锁多 Isolate 线程池

Beejs 在底层支持**多 Worker 线程池并发模型**：

```text
                        客户端高并发请求 (TCP Traffic)
                                     │
                                     ▼
                 +─────────────────────────────────────+
                 │      主分发线程 (Rust Tokio I/O)    │
                 │   - TCP 连接监听与 SO_REUSEPORT      │
                 │   - 无锁 Channel 极速跨线程分发     │
                 +─────────────────────────────────────+
                        │           │           │
            ┌────────────┘           │           └────────────┐
            ▼                        ▼                        ▼
+─────────────────────+  +─────────────────────+  +─────────────────────+
| Worker 1 (Isolate)  |  | Worker 2 (Isolate)  |  | Worker N (Isolate)  |
| - 独立 V8 执行堆    |  | - 独立 V8 执行堆    |  | - 独立 V8 执行堆    |
| - 独立 GC 垃圾回收  |  | - 独立 GC 垃圾回收  |  | - 独立 GC 垃圾回收  |
| - 处理请求 1, 4, 7  |  | - 处理请求 2, 5, 8  |  | - 处理请求 3, 6, 9  |
+─────────────────────+  +─────────────────────+  +─────────────────────+
```

### 启用多 Worker 模式

在运行脚本时，通过 `-W` 或 `--workers` 指定工作线程数量：

```bash
# 启用 8 个并行 Worker 线程
$ bee run --workers 8 server.ts
```

或者在生产环境中设置环境变量：

```bash
export BEE_WORKERS=8
bee run server.ts
```

**核心优势**：

- **真多核并行**：各个 Worker 运行在独立且互不干扰的 V8 Isolate 中，CPU 密集型任务完全并行，不抢占主事件循环；
- **零请求创建开销**：所有 Worker 在进程启动时预热完成，请求到达时仅通过内存中的无锁队列唤醒，消除了反复创建销毁线程的系统开销。

---

## 4. 生产压测与最佳实践

可以使用 `autocannon` 或 `wrk` 对服务进行高并发压测：

```bash
# 启动 8 核心工作线程
bee run --workers 8 server.ts

# 发起压测 (100 并发连接，持续 10 秒)
npx autocannon -c 100 -d 10 http://localhost:3000/api/users
```

### 生产优化技巧

1. **轻量服务优先选择 `bee serve`**：Fetch API 模型没有传统 Event Emitter 流包装开销，在微服务与边缘计算场景拥有更高的每秒请求处理量（RPS）；
2. **合理规划 Worker 数量**：在纯 I/O 服务中，Worker 数量建议设为 `CPU核心数` 至 `CPU核心数 * 2`；在重度密集计算时，建议严格等于物理核心数；
3. **搭配安全沙箱**：生产对外暴露的不可信脚本建议添加 `--sandbox` 和 `--max-memory 512`，有效抵御内存泄漏与越权文件访问。

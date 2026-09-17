---
title: "Modern Web Serving & Concurrency"
subtitle: "Standard Web Fetch API (bee serve), node:http, and Lockless Multi-Isolate Thread Pools"
group: "Core Systems"
id: "server-mode"
---

Amber offers two high-performance paradigms for building web services:

1. **Modern Standard Web Serving (`amber serve [file]`)**: Built on W3C / WinterCG standard `Request` / `Response` and `export default { fetch(req) }` model;
2. **Node.js Compatible Serving (`amber run server.ts`)**: Built on `node:http` backed by a lockless multi-Worker thread pool in Rust Tokio.

---

## 1. Modern Web Application Serving (`bee serve`)

`bee serve` is the recommended, zero-overhead entrypoint for modern web applications, aligned with Cloudflare Workers, Deno, and Bun, featuring native TypeScript and JSX execution.

### 1.1 Writing Your First Web Service

Simply export an object with a `fetch` handler, or export the function directly:

```typescript
// app.ts
export default {
  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);

    // Route matching
    if (url.pathname === "/") {
      return new Response("🚀 Welcome to Amber Web Server!");
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

### 1.2 Starting the Server

```bash
# Automatically detects and runs app.ts, app.js, server.ts, index.ts, etc.
$ bee serve

# Or specify a custom file, port, and host
$ amber serve app.ts --port 8080 --host 0.0.0.0
```

Console output:

```text
🚀 Starting Amber Web Server on http://0.0.0.0:8080
📄 Serving application: app.ts
✅ Listening on http://0.0.0.0:8080 (Ctrl+C to stop)
```

### 1.3 Key Highlights

- **Zero Glue Overhead**: Directly bridges incoming HTTP packets to standard `Request` instances without unnecessary wrapper streams;
- **Async & Promise Support**: Supports asynchronous handlers and drains microtasks automatically in the event loop;
- **Full Body Mixin**: `Request` and `Response` fully implement `req.text()`, `req.json()`, and `req.arrayBuffer()`;
- **Sandbox Integration**: Compatible with `--max-memory <MB>` and `--sandbox` permissions to bound untrusted execution.

---

## 2. Classic Node.js Compatible Serving (`node:http`)

For legacy services or Express-style architectures, `node:http` works out of the box:

```typescript
// server.ts - Classic Node.js style
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
  console.log('🚀 HTTP Server running: http://localhost:3000');
});
```

Run command:

```bash
amber run server.ts
```

---

## 3. Multi-Worker Thread Pool Architecture (`--workers`)

### The Single-Thread Bottleneck

In conventional single-threaded runtimes, when a request triggers heavy JSON serialization, cryptography, or tensor inference, the event loop stalls and stalls all incoming requests.

### Amber Lockless Multi-Isolate Model

Amber features a built-in **multi-Worker thread pool** in Rust:

```text
                        Concurrent TCP Traffic
                                  │
                                  ▼
                 +──────────────────────────────────+
                 │      Main Dispatch Thread (Tokio)│
                 │   - TCP Listener & SO_REUSEPORT  │
                 │   - Lockless Cross-Thread Queue  │
                 +──────────────────────────────────+
                        │         │         │
            ┌───────────┘         │         └───────────┐
            ▼                     ▼                     ▼
+───────────────────+ +───────────────────+ +───────────────────+
| Worker 1 (Isolate)| | Worker 2 (Isolate)| | Worker N (Isolate)|
| - Dedicated Heap  | | - Dedicated Heap  | | - Dedicated Heap  |
| - Independent GC  | | - Independent GC  | | - Independent GC  |
+───────────────────+ +───────────────────+ +───────────────────+
```

### Launching Workers

Specify the number of worker isolates with `-W` or `--workers`:

```bash
amber run --workers 8 server.ts
```

Or via environment variable:

```bash
export BEE_WORKERS=8
amber run server.ts
```

**Benefits**:

- **True Multi-Core Parallelism**: Isolates execute independently in parallel on separate OS threads without blocking one another;
- **Zero Startup Penalty**: Workers are pre-warmed at boot time, eliminating thread creation overhead per request.

---

## 4. Production Benchmarks & Best Practices

Benchmark with tools like `autocannon` or `wrk`:

```bash
# Launch with 8 workers
amber run --workers 8 server.ts

# Benchmark 100 concurrent connections for 10 seconds
npx autocannon -c 100 -d 10 http://localhost:3000/api/users
```

### Optimization Tips

1. **Prefer `bee serve` for Microservices**: The Fetch API model avoids EventEmitter and streaming buffer wrapper overhead, yielding higher RPS;
2. **Calibrate Workers**: For I/O services, set workers to `cores` ~ `2 * cores`; for CPU/tensor-heavy workloads, match the physical core count;
3. **Enforce Resource Quotas**: For public-facing endpoints, combine with `--sandbox` and `--max-memory 512` to prevent memory leaks and unauthorized disk access.

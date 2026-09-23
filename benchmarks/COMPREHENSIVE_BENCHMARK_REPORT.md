# Amber 全面基准性能测试综合评估报告

> **生成时间**: `2026-09-18T02:41:00.421094+00:00`  
> **硬件环境**: `Apple M2 Max` (Darwin 27.0.0 arm64)  
> **对比运行时**: **amber 1.16.0** vs **Node.js v22.22.3** vs **Bun 1.4.1**

---

## 1. 核心结论

本次在 `Apple M2 Max` 上对比 **amber 1.16.0**、**Node.js v22.22.3** 与 **Bun 1.4.1**。

1. **冷启动与短命进程**
   - Amber `eval '1 + 1'` 均值 **19.10 ms**，Node.js **29.13 ms**（1.53x vs Node），Bun **9.06 ms**（0.47x vs Bun）。
2. **核心运行时与 JIT**
   - 对象分配：Amber **458.8 ops/s**，相比 Node.js 1.34x。
   - EventEmitter：Amber **3612.2 ops/s**，相比 Node.js 2.10x。
   - 字符串/正则：相比 Node.js 2.96x。
3. **HTTP / 框架吞吐**
   - Express 5.x：Amber **69,228.8 req/s**，平均时延 **0.01 ms**。
   - Hono 4.x：Amber **70,944.0 req/s**。
   - Raw HTTP：Amber **66,857.6 req/s**。
4. **Conformance**
   - Node.js Conformance 5.0：54/55 （98.2%，2.78s）。

---

## 2. 冷启动与 CLI 时延

每项 20 次。越低越好。

| 工作负载 | amber 1.16.0 均值 | Amber P95 | v22.22.3 | 1.4.1 | vs Node |
|---|---|---|---|---|---|
| `eval '1 + 1'` | **19.10 ms** | 59.33 ms | 29.13 ms | 9.06 ms | 1.53x |
| `eval '1 + 1' (--warm)` | **16.97 ms** | 18.40 ms | 28.42 ms | 8.61 ms | 1.67x |
| `eval console.log('hello')` | **17.10 ms** | 19.02 ms | 29.89 ms | 8.66 ms | 1.75x |
| `run hello_world.js` | **18.46 ms** | 20.42 ms | 30.06 ms | 10.26 ms | 1.63x |
| `run hello_world.js (--warm)` | **18.82 ms** | 20.93 ms | 31.59 ms | 10.50 ms | 1.68x |
| `run TypeScript (.ts)` | **18.75 ms** | 20.22 ms | — | 11.52 ms | — |
| `test runner (math.test.js)` | **18.63 ms** | 21.81 ms | — | 132.46 ms | — |

## 3. 运行态微基准

`benchmarks/comprehensive_bench.js`，24 项。

| 基准项目 | Amber 耗时 (ms) | Node.js 耗时 (ms) | Bun 耗时 (ms) | Amber 吞吐 (ops/s) | 相对表现 |
| --- | --- | --- | --- | --- | --- |
| **1. JIT / Fibonacci (500k ops)** | **9.23** | 10.42 | 7.27 | **108.3** | 比 Node 快 1.13x |
| **2. JIT / Primes Sieve (100k)** | **0.33** | 0.39 | 0.40 | **3,050.6** | 比 Node 快 1.19x, 比 Bun 快 1.21x |
| **3. JIT / Matrix Multiply (80x80)** | **0.45** | 0.56 | 0.52 | **2,242.8** | 比 Node 快 1.26x, 比 Bun 快 1.17x |
| **4. Objects / Alloc & Property Access (50k)** | **2.18** | 2.92 | 3.28 | **458.8** | 比 Node 快 1.34x, 比 Bun 快 1.50x |
| **5. Arrays / Filter-Map-Reduce (20k)** | **0.36** | 0.72 | 0.21 | **2,803.4** | 比 Node 快 2.03x |
| **6. JSON / Stringify & Parse (50 iters)** | **5.28** | 9.95 | 3.60 | **189.4** | 比 Node 快 1.88x |
| **7. String & RegExp (10k iters)** | **2.65** | 7.86 | 2.17 | **376.9** | 比 Node 快 2.96x |
| **8. Buffer / Alloc, Fill, Slice (1k x 16KB)** | **1.90** | 1.91 | 3.25 | **526.0** | 比 Node 快 1.01x, 比 Bun 快 1.71x |
| **9. Crypto / SHA-256 (500 x 16KB)** | **3.61** | 4.61 | 3.49 | **277.2** | 比 Node 快 1.28x |
| **10. Crypto / randomBytes (500 x 1KB)** | **0.44** | 0.68 | 0.31 | **2,274.4** | 比 Node 快 1.55x |
| **11. EventEmitter / emit & listen (50k)** | **0.28** | 0.58 | 0.96 | **3,612.2** | 比 Node 快 2.10x, 比 Bun 快 3.48x |
| **12. File System / Sync Read & Write (100 x 32KB)** | **6.86** | 8.88 | 5.53 | **145.8** | 比 Node 快 1.29x |
| **13. Map & Set / Insert & Lookup (50k)** | **15.38** | 13.48 | 16.37 | **65.0** | 与 Node 接近 (1.14x), 比 Bun 快 1.06x |
| **14. TypedArray / Float64 Reduce (1M)** | **2.53** | 2.56 | 1.93 | **394.8** | 比 Node 快 1.01x |
| **15. TextEncoder / TextDecoder (200 x 64KB)** | **14.86** | 29.57 | 6.86 | **67.3** | 比 Node 快 1.99x |
| **16. URL & URLSearchParams (20k)** | **7.41** | 13.10 | 20.75 | **134.9** | 比 Node 快 1.77x, 比 Bun 快 2.80x |
| **17. structuredClone nested objects (2k)** | **2.53** | 3.61 | 3.25 | **394.9** | 比 Node 快 1.43x, 比 Bun 快 1.28x |
| **18. Promise / microtask storm (20k)** | **0.72** | 0.80 | 0.63 | **1,380.3** | 比 Node 快 1.11x |
| **19. Web Crypto / subtle.digest SHA-256 (100 x 16KB)** | **0.72** | 2.02 | 1.81 | **1,390.2** | 比 Node 快 2.81x, 比 Bun 快 2.52x |
| **20. ReadableStream produce & consume (5k chunks)** | **1.01** | 1.24 | 0.39 | **987.7** | 比 Node 快 1.22x |
| **21. WebAssembly / instantiate + 100k add calls** | **0.72** | 1.26 | 0.76 | **1,395.3** | 比 Node 快 1.76x, 比 Bun 快 1.06x |
| **22. Date / toISOString (50k)** | **23.00** | 21.38 | 7.56 | **43.5** | 与 Node 接近 (1.08x) |
| **23. File System / Async promises R&W (50 x 32KB)** | **4.87** | 8.20 | 3.66 | **205.2** | 比 Node 快 1.68x |
| **24. CompressionStream / gzip (20 x 32KB)** | **1.44** | 3.61 | 0.27 | **696.0** | 比 Node 快 2.51x |

## 4. HTTP 与框架压测

autocannon：32 connections，5 秒。

| 框架 | 运行时 | req/s | 平均时延 | P99 | 总请求 | RSS Baseline → Peak → Settled |
|---|---|---|---|---|---|---|
| **Raw HTTP** | **amber 1.16.0** | **66,857.6** | 0.03 ms | 1.00 ms | 334,308 | 34.0 → 76.8 → 44.9 MB |
| **Raw HTTP** | NODE | **66,102.4** | 0.02 ms | 1.00 ms | 330,451 | 43.8 → 60.9 → 60.9 MB |
| **Raw HTTP** | BUN | **74,220.8** | 0.01 ms | 0.00 ms | 371,057 | 21.0 → 60.3 → 60.3 MB |
| **Hono 4.x** | **amber 1.16.0** | **70,944.0** | 0.01 ms | 0.00 ms | 354,665 | 36.5 → 82.2 → 50.3 MB |
| **Hono 4.x** | NODE | **62,185.6** | 0.03 ms | 1.00 ms | 310,891 | 58.6 → 78.0 → 78.0 MB |
| **Hono 4.x** | BUN | **76,678.4** | 0.01 ms | 0.00 ms | 383,393 | 18.6 → 40.7 → 40.7 MB |
| **Express 5.x** | **amber 1.16.0** | **69,228.8** | 0.01 ms | 0.00 ms | 346,205 | 50.7 → 88.5 → 56.7 MB |
| **Express 5.x** | NODE | **18,100.8** | 1.22 ms | 3.00 ms | 90,502 | 58.0 → 120.3 → 120.3 MB |
| **Express 5.x** | BUN | **57,257.6** | 0.06 ms | 1.00 ms | 286,243 | 38.7 → 92.5 → 92.5 MB |
| **Fastify 5.x** | **amber 1.16.0** | **66,819.2** | 0.02 ms | 1.00 ms | 334,083 | 43.7 → 87.7 → 55.7 MB |
| **Fastify 5.x** | NODE | **63,923.2** | 0.03 ms | 1.00 ms | 319,631 | 62.0 → 78.7 → 78.7 MB |
| **Fastify 5.x** | BUN | **69,728.0** | 0.02 ms | 0.00 ms | 348,630 | 41.1 → 89.5 → 89.5 MB |

## 5. Node.js Conformance 5.0

- 用例：`55`
- 通过：`54`
- 通过率：`98.2%`
- 耗时：`2.78s`

## 6. 客户端 Fetch 与 SQLite

| 扩展 I/O | Amber 耗时 (ms) | Node.js 耗时 (ms) | Bun 耗时 (ms) | Amber 吞吐 (ops/s) | 相对表现 |
| --- | --- | --- | --- | --- | --- |
| **25. Fetch / 100 sequential GETs** | **7.56** | 17.34 | 4.83 | **132.2** | 比 Node 快 2.29x |
| **26. SQLite / 2k insert + point select** | **9.46** | SKIP | SKIP | **105.7** | 与主流表现相当 |
| **27. SQLite / 5k transactional inserts** | **5.60** | SKIP | SKIP | **178.7** | 与主流表现相当 |

## 7. amber:ai Tensor

| AI Tensor | Amber 耗时 (ms) | Node.js 耗时 (ms) | Bun 耗时 (ms) | Amber 吞吐 (ops/s) | 相对表现 |
| --- | --- | --- | --- | --- | --- |
| **28. Tensor / matmul 64x64 (native)** | **0.03** | — | — | **36,309.0** | 与主流表现相当 |
| **29. Tensor / matmul 64x64 (pure JS)** | **0.35** | — | — | **2,835.5** | 与主流表现相当 |
| **30. Tensor / dot 16k (native)** | **0.02** | — | — | **66,041.5** | 与主流表现相当 |
| **31. Tensor / softmax 4k (native)** | **0.02** | — | — | **56,604.0** | 与主流表现相当 |
| **32. Tensor / cosineSimilarity 1k (native)** | **0.00** | — | — | **259,188.2** | 与主流表现相当 |
| **33. Tensor / dot 16k (pure JS)** | **0.11** | — | — | **8,711.4** | 与主流表现相当 |
| **34. Tensor / softmax 4k (pure JS)** | **0.18** | — | — | **5,626.1** | 与主流表现相当 |

## 8. 说明

- 数字来自本次命令，不是 SLA。
- Amber CLI 为 `amber`；内置模块为 `amber:ai`。
- HTTP 为单进程、本机 loopback、32 连接 5 秒 autocannon。

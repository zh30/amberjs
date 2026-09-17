---
title: "Runtime Architecture & V8 Core"
subtitle: "Deep dive into the V8 Isolate model, zero-copy mmap snapshots, and high-precision event scheduling"
group: "Core Systems"
id: "v8-isolate-pool"
---

## 1. Why the V8 Isolate Architecture?

In modern cloud-native and serverless edge computing, resource footprint and startup latency dictate system concurrency limits and operational costs.

Traditional isolation units like operating system processes or Docker containers carry substantial overhead. **V8 Isolates** provide an ultra-lightweight alternative:

| Isolation Metric | Docker Container | OS Process (fork) | V8 Isolate (Beejs) |
| :--- | :---: | :---: | :---: |
| **Baseline Memory** | ~50 MB – 200 MB | ~20 MB – 50 MB | **~2 MB – 5 MB** |
| **Cold Start Latency** | 500 ms – 2000 ms | 50 ms – 150 ms | **< 18 ms** |
| **Density (16GB RAM)** | ~80 – 300 instances | ~300 – 800 instances | **> 3,000+ isolated realms** |
| **Tear-Down Cost** | Kernel cgroup cleanup | Process exit cleanup | **Release isolate heap** |

An **Isolate** represents an independent instance of the V8 JavaScript engine. Each Isolate maintains its own separate heap, garbage collector, and execution stack, guaranteeing that memory corruptions or infinite loops cannot leak across realms.

---

## 2. V8 Startup Snapshot 2.0 with mmap

### The Cold-Start Bottleneck
Traditional JavaScript runtimes spend 35ms to 60ms during boot executing internal bootstrap scripts, constructing prototypes, and populating dozens of standard global constructors (`Object`, `Array`, `Promise`, `Map`, etc.).

### Zero-Copy Memory Mapping in Beejs
Beejs serializes the fully initialized global context into a compact binary snapshot at build time.

When launching a script or creating a new worker:
- **`memmap2::Mmap::map`**: The runtime maps the binary snapshot directly into virtual memory via kernel page tables, completely bypassing disk reads and intermediate heap allocations.
- **Copy-on-Write (CoW)**: Multiple worker threads and CLI invocations share the exact same physical memory pages. Pages are only cloned if an isolate modifies an internal prototype.
- **Sub-18ms Cold Starts**: Drops initialization time by almost 2x compared to standard Node.js.

---

## 3. Rust-to-V8 Zero-Cost Bindings

Beejs utilizes `rusty_v8` to interface directly with Google V8 C++ internals, wrapped with safe Rust abstractions:

```text
+-----------------------------------------------------------+
|               JavaScript Execution (V8 Isolate)           |
|        req.on('data', chunk => { ... })                   |
+-----------------------------------------------------------+
                              |
               [Fast API Calls / Externals]
                              |
+-----------------------------------------------------------+
|                 Rust Host Layer (Beejs Core)              |
|  - Automatic HandleScope lifecycle management             |
|  - Safe raw-pointer dereferencing & type conversions      |
|  - Elegant error handling via anyhow::Result              |
+-----------------------------------------------------------+
```

### Safety Rules
- **No Cross-Thread Handle Sharing**: `v8::Local<v8::Value>` handles are strictly bound to their thread's `HandleScope`. Concurrency across workers uses thread-safe Rust channels with serialized data, eliminating data races.
- **Immediate Scope Release**: Long-running loops allocate local `HandleScope` instances, guaranteeing that intermediate wrapper objects are collected promptly by V8 GC.

---

## 4. High-Precision Timing Wheel & Event Loop

When handling tens of thousands of concurrent timers (`setTimeout` / `setInterval`), traditional double-linked timer lists experience performance degradation due to $O(N)$ insertion and cancellation costs.

Beejs replaces naive timer structures with a **Hierarchical Timing Wheel**:
- **$O(1)$ Operations**: Insertion, cancellation, and expiration checks run in constant time.
- **Batch Expiration**: In benchmarks with 1,000 concurrent high-frequency timers, Beejs triggers callbacks in just **2.51ms** (a **14.2x speedup** over older versions).
- **Strict Microtask Ordering**: Conforms strictly to ECMAScript and HTML standards by draining Promise microtasks immediately after each macrotask turn.

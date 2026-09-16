# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.16.0] - 2026-09-16

### Added

- **Wasm Engine 2.0 zero-copy memory**: `WebAssembly.Memory` / ArrayBuffer virtual-address sharing, mmap module load, `require('bee:wasm')`.
- **Production bundler and SEA**: oxc-backed `bee bundle` and `bee compile` single-file executables.
- **Full-spectrum benchmark suite 2.0**: 24 in-process workloads plus Fetch/SQLite and `bee:ai.Tensor` phases.

### Performance

- **URL / URLSearchParams**: JIT-friendly `url_fast.js`, removed BeeURL wrapper. 20k parse **234ms → 7.28ms** (faster than Node).
- **fetch()**: HTTP/1.1 keep-alive fast path, no Tokio `block_on` on string GET. 100 sequential localhost GETs **1452ms → 6.51ms** (faster than Node).
- **ReadableStream**: JS enqueue/read hot path. 5k chunks **4.53ms → 0.92ms** (faster than Node).

---

## [1.15.0] - 2026-09-15

### Added

- **Recursive Self-Improvement (RSI) Deep Network I/O Engine & V8 Monomorphic JIT Dispatch**:
  - **Batch Handle Scoping to Eliminate Memory Ballooning**: Introduced `v8::scope!(let batch_scope, scope)` inside `pump_pending_http_requests_in_scope`, ensuring all temporary handles generated during a batch pump are dropped immediately upon batch exit instead of accumulating on the root `scope`.
  - Peak RSS during high-concurrency bursts slashed by **70% ~ 83%** (Fastify 534 MB &rarr; **87.7 MB**, Raw HTTP 435 MB &rarr; **76.5 MB**; cooldown **44.5 MB ~ 55.8 MB**, beating Node.js).
  - **V8 Monomorphic JIT Dispatch (`FastIncomingMessage` / `FastServerResponse`)**:
    - Predefined monomorphic constructors in bootstrap with fixed property layouts, eliminating 11 shape transitions per request and out-of-line `PropertyArray` re-allocations.
    - Moved cold properties (`socket`, `connection`, `rawHeaders`) to prototype lazy getters for on-demand allocation.
    - Cached `protos.dispatch_fn` in `V8HttpPrototypes` to eliminate per-request `global.get("__dispatchHttpRequest")` lookups.
    - Reduced FFI boundary transitions from 40+ calls per request down to 1 single monomorphic call.
  - **Lock-Free Atomic Connection Counter**: Replaced global mutex with `static HTTP_CONNECTION_COUNTER: AtomicU64` and `.fetch_add(1, Ordering::Relaxed)`.
  - **Direct Tokio Network Stream Handling (`try_read` / `try_write`)**: Non-blocking fast path bypasses Tokio timer wheels and task rescheduling for small HTTP responses.
  - **Zero-Formatting Response Generation (`generate_http_response_v2`)**: Replaced `write!` formatting macro and dynamic dispatch with direct static byte slices (`extend_from_slice`).
  - **Interned Header Parsing (`parse_http_request`)**: Fast-path ASCII matching of standard HTTP headers and pre-allocated header map capacity.
  - **Zero-Allocation Body Extraction (`write_utf8_v2`)**: Direct V8 buffer write into raw bytes without intermediate Rust string allocations.
  - **Breakthrough Benchmarked Throughput (Apple M2 Max)**:
    - **Raw HTTP**: **71,750.4 req/s** (surpasses Node.js at 71,404.8 req/s)
    - **Hono 4.x**: **71,852.8 req/s** (surpasses Node.js at 70,982.4 req/s)
    - **Express 5.x**: **69,228.8 req/s** (**3.61x Node.js** at 19,166.4 req/s; surpasses Bun at 63,510.4 req/s)
    - **Fastify 5.x**: **69,164.8 req/s** (ultra-low 0.01ms avg latency)
  - **Full Conformance & Test Stability**:
    - 100% Node.js Conformance Scorecard (55/55 PASS)
    - All HTTP Server Test Suites (72/72 PASS)

## [1.14.0] - 2026-09-15

### Added

- **Tokio Zero-Hop Non-Blocking Network I/O Engine**:
  - Rebuilt `node:http` and `node:https` on top of an asynchronous multi-threaded Tokio reactor, completely retiring the blocking thread-per-connection pattern.
  - 32-way sharded response waiter registry (`RESPONSE_WAITERS_SHARDS`) using fine-grained locks to eliminate global thread contention.
  - Lock-free MPSC request dispatch queues coupled with microsecond-level thread park/unpark wakers (`HTTP_DISPATCH_WAKER`), replacing 1ms blind sleeping with event-driven immediate wakeups.
  - Massive throughput improvements in real-world HTTP benchmarks:
    - **Raw HTTP**: 21,348 req/s &rarr; **73,324.8 req/s** (3.4x increase, P95 latency 0.01ms)
    - **Hono 4.x**: 20,495 req/s &rarr; **74,297.6 req/s** (3.6x increase)
    - **Fastify 5.x**: 14,874 req/s &rarr; **69,676.8 req/s** (4.7x increase)
    - **Express 5.x**: 20,381 req/s &rarr; **64,995.2 req/s** (3.2x increase)
- **V8 Copy-on-Write (CoW) Snapshot & Standby Prewarmer Pool (Task 1.2)**:
  - Kernel-level zero-copy snapshot sharing via `libc::mmap` with `MAP_PRIVATE` and `MADV_WILLNEED` page pre-faulting for ultra-fast isolate creation.
  - Thread-affine standby isolate prewarmer (`IsolatePrewarmer`) maintaining ready-to-run execution contexts in background threads.
  - New `--warm` CLI flag for instant prewarmed isolate acquisition, reducing cold startup overhead to sub-millisecond (< 0.2ms).
  - Integration test suite `tests/v8_cow_snapshot_tests.rs` verifying CoW memory safety and prewarmer isolate recycling.
- **Comprehensive Multi-Runtime Benchmark Suite**:
  - Python-based comprehensive benchmark automation (`benchmarks/run_comprehensive_benchmark.py`) comparing Beejs against Node.js v22 and Bun 1.4 across Microbenchmarks, Web frameworks, I/O workloads, and AI inference.
  - Detailed performance scorecard published in `benchmarks/COMPREHENSIVE_BENCHMARK_REPORT.md`.

## [1.12.0] - 2026-09-15

### Added

- **`bee:ai` Native Local AI Inference Engine (HuggingFace Candle & GGUF Integration)**:
  - Deep integration with HuggingFace Candle (`0.8.2`) and Tokenizers (`0.21`), providing embedded zero-dependency local model inference without requiring external Python or daemon processes.
  - Zero-copy tensor bridge: direct physical memory sharing between V8 `ArrayBuffer` (`BackingStore`) and Candle `Tensor`, eliminating serialization and IPC overhead.
  - Native quantized GGUF model loader with automatic architecture detection for Llama, Mistral, Qwen 2, Qwen 2.5, and more.
  - Hardware acceleration pipeline: direct Apple Silicon Metal support (`--features metal`), Linux CUDA acceleration (`--features cuda`), and universal multi-threaded CPU fallback with SIMD vectorization.
  - Asynchronous streaming token generation with native JS AsyncIterator protocol (`for await (const chunk of model.generateStream(prompt))`).
  - High-performance dense semantic embeddings (`candle_embeddings`) with cosine similarity and V8 bindings.
  - Added runnable AI examples: `examples/ai/local_llm_inference.js` and `examples/ai/agent_tool_calling.js`.
  - Added integration test suite `tests/ai_candle_inference_tests.rs` (6/6 PASS) verifying model loading, streaming generation, KV-cache decoding, tensor math, embeddings, and Metal device detection.

## [1.11.0] - 2026-09-14

### Added

- **Node.js Conformance 5.0 & Mainstream Framework Compatibility**:
  - Full compatibility support and smoke verification for modern server frameworks: **Express 5.x**, **Fastify 5.x**, and **Hono 4.x** (`@hono/node-server`).
  - Conformance test harness expanded to 55 fixtures with a 100% pass rate (55/55 PASS).
- **HTTP/2 Prototype & Constants**:
  - Implemented `Http2ServerRequest`, `Http2ServerResponse`, and `Http2Stream` classes and prototypes in `node:http2`.
  - Added HTTP/2 protocol constants (e.g. `constants.NGHTTP2_NO_ERROR: 0`) required by `@hono/node-server`.
- **Stream Ecosystem Modernization**:
  - Unified `Readable`, `Writable`, `Duplex`, `Transform`, and `PassThrough` under `Stream.prototype` inheriting from `EventEmitter.prototype`.
  - Implemented `Stream.Duplex.from` and `Stream.from` factory methods for iterable and async iterable adaptation.
  - Added `pause()`, `resume()`, `isPaused()`, and `setEncoding(enc)` on `Readable.prototype`.
- **AsyncLocalStorage Snapshot API**:
  - Implemented `AsyncLocalStorage.snapshot()` in `node:async_hooks` for capturing and restoring context across asynchronous boundaries.
- **Node HTTP Protocol Enhancements**:
  - Added `assignSocket(socket)` to both `ServerResponse` and `ClientRequest`.
  - Added `rawHeaders`, `complete`, and encoding control methods (`setEncoding`, `pause`, `resume`) on `IncomingMessage`.

## [1.10.0] - 2026-09-14

### Added

- **Modern V8 Engine Upgrade**: Full migration from legacy rusty_v8 to modern official `v8 = "152.2.0"` (Chromium 134+).
- **Stack-pinned Scope Architecture**: Standardized on `v8::PinScope` across all core runtime modules, Web APIs, and Node.js compat layers.
- **Modern V8 Macro Suite**: Upgraded to official `v8::scope!`, `v8::callback_scope!`, and `v8::tc_scope!` macros.
- **Snapshot Isolation & Self-Healing (`BEEJS_V3`)**: Startup snapshot versioning with dynamic V8 engine version binding to prevent binary mismatch crashes.
- **Unlocked Modern Toolchain**: Removed legacy pinned `serde = "=1.0.197"` and historical swc locks, restoring ecosystem upgrade flexibility.

### Changed

- ArrayBuffer and BackingStore memory handling safe rewrite with `Option<NonNull<c_void>>` and `detach(None)`.
- ESM dynamic import and synthetic module callbacks migrated to safe Rust signatures.
- Evaluator startup latency improved to 14.95ms (1.76x faster than Node.js).
- In-process warm isolate execution throughput exceeds 1,000,000 ops/sec (< 1µs).

## [1.9.1] - 2026-09-10

### Added

- rustls HTTP/1.1 for `bee serve --https` (requires `--cert` / `--key` PEM).
- Inspector `Runtime.evaluate` on the isolate and `--inspect-brk` pause until resume.
- Minimal N-API hello loader (`process.dlopen` → `napi_register_module_v1`).
- Homebrew formula SHA updater (`scripts/update_homebrew_formula.py`) run from Release Assets.
- In-repo winget manifest `manifests/winget/zh30.bee.yaml`.
- `cargo deny` advisories/licenses job; rustc pinned to 1.97.1.

### Changed

- Windows MSVC Release job is fail-closed and must attach `bee.exe` zip.
- `cargo-audit` no longer `continue-on-error`.
- CI feature matrix is `benchmarks` and `observability` only.
- GHCR documented as linux/amd64 only.
- `bee test --parallel` exits 2 instead of warning-and-succeeding.
- TypeScript throw stacks map to original `.ts` lines.
- VS Code extension launch uses `bee run --inspect-brk` and current GitHub Release asset names.

### Fixed

- Unix-only `libc` (`isatty`, `posix_memalign`/`madvise`) cfg-gated for the Windows default path.
- `bee serve` health JSON version uses `CARGO_PKG_VERSION`.

## [1.9.0] - 2026-09-09

### Added

- WinterTC baseline on the default runtime: `DOMException`, `URLPattern`, `navigator`, queuing strategies, `ReadableStream.from`, `bee:sockets`, `wintercg`/`wintertc` export conditions, and `import.meta.main` / `env` / `resolve`.
- Real rustls TLS for `secureTransport: "on"` and `startTls()`, including untrusted-certificate rejection.
- `v*` GitHub Release archives for linux gnu x64/arm64, macOS arm64/x64, and Windows x64, with SHA-256 checksums, CycloneDX SBOM, and cosign signatures.
- `install.sh` platform mapping for Darwin/Linux x64 and arm64, plus `install.ps1` for the Windows zip.
- In-repo Homebrew formula `Formula/bee.rb` pointing at GitHub Release assets.
- Website WinterTC docs (English and Chinese) in the docs nav.

### Changed

- PR CI fails closed on `cargo check --features` for `ai`, `benchmarks`, and `observability`.
- CI runs WinterTC as its own test step, smokes Windows, and runs library tests on macOS.
- `docker.yml` publishes `ghcr.io/zh30/beejs` on `v*` tags and `main`.
- `import.meta.resolve` uses the real ESM resolver so `"wintercg"` exports win over `"node"`.
- Unhandled promise rejections dispatch `PromiseRejectionEvent` / `onunhandledrejection`.

## [1.8.0] - 2026-09-09

### Added

- Multi-Agent message bus (`bee:bus`) with topic wildcards, request-reply RPC, middleware, and DLQ.
- Streaming structured JSON / token grammar engine (`bee:grammar`) including `parsePartialJSON` and SSE chunk parsing.
- Agent state checkpoint / time-travel snapshots (`bee:checkpoint`).

## [0.4.0] - 2026-09-04

### Added

- **Native Test Runner 2.0 (Zero-Argument Discovery & Watch Mode)**:
  - Added recursive test file discovery when `bee test` is invoked with zero arguments, scanning for `*.test.js`, `*.test.ts`, `*_test.js`, and `*_test.ts`.
  - Added intelligent noise-filtering to exclude non-test directories (`manual/`, `node_modules/`, `__snapshots__/`, `.git/`, `target/`, `dist/`).
  - Added `--watch` mode to `bee test` and `bee test <file>` using `notify` filesystem event watching with debounced test re-execution.
  - Promoted `bee test` from *Experimental* to **Stable** in `docs/CURRENT_SCOPE.md`.
- **Agent Deterministic Sandbox & Virtual Time (Deterministic Replay 1.0)**:
  - Added `--seed <u64>` CLI option backed by ChaCha8 PRNG, intercepting `Math.random()`, Web Crypto `crypto.getRandomValues()`, and Node.js `crypto.randomBytes()`.
  - Added `--freeze-time <TIMESTAMP | ISO>` CLI option for virtual deterministic clock, intercepting `Date.now()`, `new Date()`, `toISOString()`, and `performance.now()`.
  - Added `PermissionBroker::reset_state()` for complete isolation between test runs and replay executions.
  - Added integration test suite `tests/deterministic_sandbox_tests.rs`.
- **Node.js Conformance 4.0 Builtins**:
  - Implemented `child_process.execSync` and `child_process.spawnSync` with stdout/stderr capture and exit code handling.
  - Added `tests/child_process_sync_tests.rs` for sync child process execution and sandbox denial tests.
  - Added 5 new conformance fixtures: `child_process_exec_sync.js`, `child_process_exec_denied.js`, `zlib_sync.js`, `util_basics.js`, and `crypto_hmac_uuid.js`.
  - Conformance scorecard achieved **35/35 (100% Pass Rate)** in `tests/conformance/scorecard.md`.

### Changed

- **Buffer & Zlib Harmonization**:
  - Refactored `zlib.gzipSync`, `gunzipSync`, `deflateSync`, `inflateSync` to return standard `Buffer` instances and accept `Buffer`/`Uint8Array` inputs without string coercion.
- **Crypto & Timing Safe Equal**:
  - Enhanced `crypto.timingSafeEqual` to accept `Buffer` instances and safely handle 0-length slices.
- **Version Bump**:
  - Updated workspace version to `0.4.0` in `Cargo.toml`, `Cargo.lock`, `README.md`, and `docs/CURRENT_SCOPE.md`.

### Fixed

- **ArrayBuffer Pointer Safety**:
  - Fixed panic in `Buffer.from(arrayBuffer)` on 0-length ArrayBuffers where rusty_v8 backing store pointer is NULL.
- **Sandbox Permission Denial**:
  - Enforced fail-closed behavior for `child_process.execSync` and `spawnSync` when running in `--sandbox` without `--allow-run`.

---

## [0.3.0] - 2026-09-04

### Added

- **Multi-Isolate Native Concurrency (Worker Threads & Web Workers 2.0)**:
  - Implemented `WorkerHost` in `src/web_api/worker_host.rs` with dedicated OS threads and independent V8 Isolates.
  - Implemented Node.js `worker_threads` module (`Worker`, `parentPort`, `isMainThread`, `workerData`).
  - Implemented Web Worker standard API (`Worker`, `postMessage`, `onmessage`, `terminate`).
  - Integrated worker lifecycle and cross-tick message polling into main event loop.
  - Added integration test suite `tests/worker_threads_multi_isolate_tests.rs`.
- **WebAssembly 2.0 Streaming Compilation & Instantiation**:
  - Implemented `WebAssembly.compileStreaming` and `WebAssembly.instantiateStreaming` consuming Fetch `Response` / `Promise<Response>`.
  - Added integration test suite `tests/wasm_streaming_tests.rs`.
- **Enterprise Agent Sandbox Audit Trail**:
  - Implemented structured JSON Lines (`JSONL`) audit logging via `--sandbox --audit-log <path>`.
  - Added integration test suite `tests/agent_sandbox_audit_tests.rs`.
- **Node.js Conformance 3.0**:
  - Expanded conformance suite to 30/30 (100% Pass Rate).

### Fixed

- **Fetch Response Binary Integrity**:
  - Fixed `body_value_to_bytes` in `src/web_api/fetch.rs` to extract raw binary bytes from Uint8Array/ArrayBuffer without UTF-8 re-encoding.

---

## [0.2.0] - 2026-09-04

### Added

- **Instant Cold Start & V8 Snapshot Optimizations**:
  - Optimized V8 startup snapshot and isolate initialization for sub-millisecond cold starts.
- **Agent Tool Sandbox**:
  - Granular permission broker for filesystem, network, environment variables, and process execution.
  - Enforced sandbox policies for safe AI Agent tool invocations.
- **Node.js Conformance 2.0**:
  - Initial 20+ fixture conformance test harness with automated scorecard.

---

## [0.1.2] - 2026-09-02

### Added

- Initial public release of Beejs runtime with basic CLI, V8 execution engine, TypeScript support, and core Web APIs (`fetch`, `console`, `URL`).

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

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

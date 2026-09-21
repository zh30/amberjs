# Amber v1.1.0 Release Notes: Tooling Ecosystem, Agent-Native Runtime & Docs Center

> **Release Tag**: `v1.1.0`  
> **Type**: Minor Feature Release  
> **Status**: Production Stable  

---

## 🌟 Executive Summary

Amber v1.1.0 is a major leap forward for the Amber ecosystem. Following the stable foundation laid by v1.0.0, this release delivers a **complete, zero-dependency engineering tooling ecosystem**, key **Agent-native runtime evolutions**, and a completely **redesigned Documentation Center** on the official website.

Built with Rust and V8, Amber v1.1.0 turns the runtime into a fully self-contained modern development platform—eliminating the need for auxiliary Node.js/npm tooling while offering deterministic execution environments tailored for AI Agent workloads.

---

## 🚀 Key Highlights & New Capabilities

### 1. Unified Engineering Toolchain (All-In-One CLI)

- **Task Runner (`amber task` / `amber run <script>`)**:
  - Automatically parses `package.json` scripts without requiring Node.js or npm.
  - Automatically injects local `node_modules/.bin` and the executing `amber` binary directory into `PATH`.
  - Interactive table overview when run without arguments.

- **OXC-Powered Formatter & Linter (`amber fmt` & `amber lint`)**:
  - Sub-millisecond formatting for JS/TS/JSX/TSX powered by Rust-native OXC parser & codegen.
  - Built-in static code diagnostics with high-visibility source code spans and CI `--check` mode.

- **Bundler 2.0 (`amber bundle`)**:
  - Full module dependency graph resolution supporting relative imports, `node_modules`, and WICG Import Maps.
  - Scope isolation via module registry, integrated minification, and SourceMap v3 generation.

- **Single-Executable Application Compiler (`amber compile`)**:
  - Zero-installation distribution: builds standalone executable binaries in milliseconds via appended self-contained payload architecture.
  - Sub-millisecond instant execution skipping CLI parsing overhead.

- **Testing, Microbenchmark & Profiler Suite**:
  - `amber test --coverage`: Code line coverage reporting with standard `lcov.info` export.
  - `amber bench`: Language-level microbenchmark suite with warmup, monotonic timing, and throughput (ops/s) reporting.
  - `amber profile`: CPU sampling profiler exporting `.cpuprofile` for visualization in Chrome DevTools Performance panel.

- **Chrome DevTools CDP Debugger (`amber debug` / `amber run --inspect`)**:
  - Complete Chrome DevTools Protocol implementation over HTTP discovery (`/json/version`, `/json/list`) and WebSocket (`/ws`).
  - Native support for `chrome://inspect`, VS Code debugging, breakpoints, and stepping.

- **Language Server (`amber lsp`)**:
  - Standards-compliant LSP 3.17 server over stdio.
  - Real-time diagnostic push on document changes, zero-config formatting, and hover documentation for `amber:ai` symbols.

- **Official TypeScript Definitions (`amber types`)**:
  - Zero-I/O instant export of `types/amberjs.d.ts` covering `amber:ai` (Tensor, LLM, AgentPipeline), standard Web APIs, and runtime intrinsics.

### 2. Agent-Native Runtime Evolutions

- **Deterministic Sandbox & Hard Resource Quotas**:
  - `--timeout <ms>`: Independent background watchdog thread sending `terminate_execution()` to forcefully interrupt CPU-bound infinite loops.
  - `--max-memory <MB>`: Physical V8 heap memory constraints preventing runaway allocations.
  - `--seed <u64>`: Mulberry32 PRNG hijacking `Math.random()` for 100% reproducible agent trajectories.
  - `--freeze-time <iso_or_ms>`: Freezes `Date.now()` and time operations for deterministic replays.

- **Modern Web Server (`amber serve`)**:
  - High-performance HTTP server dispatching requests to standard `export default { fetch(req) }` handlers.
  - Comprehensive Request Body mixin support (`await req.json()`, `await req.text()`, `await req.arrayBuffer()`).
  - Worker thread pool integration for multi-core scaling.

- **WICG Import Maps (`--import-map <path>`)**:
  - Standards-compliant bare specifier remapping and package prefix routing across runtime and bundler.

- **Interactive REPL 2.0 (`amber repl`)**:
  - Powered by `rustyline` with history persistence (`~/.amberjs_history`), multi-line block detection, and line editing.

- **Node-API / Native Addon Foundation**:
  - `process.dlopen` dynamic library linking with automatic `.node` file resolution in `require()`.

### 3. Redesigned Documentation Center (website/src/routes/docs.tsx)

- **3-Column Modern Architecture**: Collapsible sidebar navigation, central reading experience, and right-hand On-This-Page TOC with live scroll spy.
- **Instant Search Filtering**: Real-time fuzzy keyword search across all 5 knowledge systems.
- **GFM Alert Boxes**: Colored semantic cards for `[!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]`, and `[!CAUTION]`.
- **Card-Based Pagination**: Seamless Prev/Next article browsing.
- **Bilingual Documentation**: 10 comprehensive chapters available in both Chinese (`*.zh.md`) and English (`*.md`).

---

## 📊 Verification & Quality Assurance

- **Tooling & Runtime Integration Suite**: `cargo test --test tooling_cli_tests` -> **17/17 PASS** (1.37s).
- **Core Library Conformance Suite**: `cargo test --lib` -> **385/385 PASS** (1.06s).
- **Website Production Build**: `npm run build` -> **2416 modules compiled in 1.54s with 0 errors**.
- **Formatting & Lints**: `cargo fmt --all -- --check` & `cargo clippy --bin amber --test tooling_cli_tests -- -D warnings` -> Clean.

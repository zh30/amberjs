# Current Scope

Last reviewed: 2026-09-21 (v1.16.1)

v1.16.1 notes:

These are operational facts for the `1.16.1` tag. They are not new Stable APIs and do not change the capability levels below.

- Release Assets still emit the five `amber-v*` archives (Linux/macOS tar.gz, Windows zip), checksums, SBOM, and cosign. When `CARGO_REGISTRY_TOKEN` is set, the same workflow publishes crates.io in order: `amber_transpile` → `amber_sandbox` → `amberjs`.
- `install.sh` and `install.ps1` try `amber-<tag>-<target>` first, then fall back to legacy `bee-` assets from older releases. The installed binary name is always `amber`.
- `.github/workflows/deploy-website.yml` can deploy the site to Cloudflare and upload the install scripts to R2 when `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` are set on the production environment.

Year-1 checklist vs this page (Graduation Rule):

- [`docs/THREE_YEAR_EXECUTION_CHECKLIST.md`](THREE_YEAR_EXECUTION_CHECKLIST.md) tasks 1.1–1.4 are marked done as **delivery progress**. That does not auto-promote capabilities here.
- `amber:ai`, V8 snapshot / CoW, permission-broker sandbox, `amber session` / `amber mcp`, and Wasm streaming stay **Stable** as already listed. The v1.16.0 zero-copy Memory / `amber:wasm` notes are unchanged; this review does not promote or demote them.
- `amber bundle`, `amber compile`, and `amber install` stay **Preview**. Executable tests exist (`tests/bundler_integration_tests.rs`, `tests/bundle_compile_tests.rs`, `tests/install_command_cli_tests.rs`), but the compatibility contract, diagnostics, and documented limits are not yet a Stable user promise.
- N-API hello loader stays **Experimental** (Year-2: [#101](https://github.com/zh30/amberjs/issues/101)).
- `multilang` / `cloudnative` / `enterprise` / empty `ai` stay **Experimental** and are not CI-gated (Year-2: [#100](https://github.com/zh30/amberjs/issues/100)–[#104](https://github.com/zh30/amberjs/issues/104)).

v1.16.0 notes:

- Wasm Engine 2.0: zero-copy `WebAssembly.Memory` / `ArrayBuffer` via V8 backing stores, mmap module load, `require('amber:wasm')`.
- `amber bundle` (oxc) and `amber compile` (SEA trailer `AMBER_STANDALONE`) remain **Preview**.
- URL / `fetch` / `ReadableStream` hot paths rewritten; suite 2.0 numbers are in `benchmarks/`.
- Node.js Conformance 5.0 is **55/55 PASS**.

Optimization sprint notes (2026-09-10 v1.9.1):

- Windows MSVC is a fail-closed Release target (`amber-v*-x86_64-pc-windows-msvc.zip` with `amber.exe`); Unix-only libc is cfg-gated on the default Windows path.
- `amber serve --https` uses rustls HTTP/1.1 (missing cert/key exits non-zero).
- `amber run --inspect-brk` evaluates on the isolate (`Runtime.evaluate`) and waits until resume.
- Minimal N-API hello loader (`process.dlopen` calls `napi_register_module_v1`). Experimental; not Prisma/sharp.
- TypeScript thrown stacks map to `.ts` lines; `amber test --parallel` exits 2.
- rustc pinned to 1.97.1; CI feature matrix is `benchmarks` + `observability` (empty `ai` feature is not in the matrix). `cargo-audit` is fail-closed; `cargo deny` checks advisories/licenses.
- GHCR images are linux/amd64 only. Homebrew SHA256 is rewritten from Release archives. Winget manifest URL uses the Windows zip name.

v1.9.0 notes (kept for history):

- Native Agentic AI Engine 1.0 (`amber:ai` promoted to **Stable**): Zero-copy `Tensor` (TypedArray-backed, matmul, dot, norm, softmax, cosineSimilarity), local streaming `LLM` (`load`, `generate`, `generateStream`, `embed`), and `AgentPipeline` with deterministic execution.
- Node conformance fixtures live in `tests/conformance/` (scorecard-driven, 100% PASS across 50+ fixtures).
- V8 Startup Snapshot 2.0 with zero-copy `mmap` backing: instant cold start with copy-on-write memory mapping across isolate processes.
- Native Test Runner 2.0 (`amber test` **Stable**): zero-argument discovery excluding `manual`, `node_modules`, and `__snapshots__`; built-in `--watch` mode.
- Agent Deterministic Sandbox & Virtual Time 1.0 (Deterministic Replay): `--seed <u64>` (deterministic PRNG for `Math.random()`, `crypto.getRandomValues()`, `crypto.randomBytes()`) and `--freeze-time <spec>` (virtual deterministic clock for `Date.now()`, `new Date()`, `performance.now()`).
- Node Conformance 4.0: `child_process.execSync` & `child_process.spawnSync` under permission broker; `zlib` sync methods returning standard Buffer instances; `Buffer.from(ArrayBuffer)` alignment; `string_decoder`, `perf_hooks`, and `events` expanded methods.
- Multi-Isolate Concurrency 2.0: OS-thread backed V8 isolates for `require('worker_threads')` and `globalThis.Worker`, with bi-directional `postMessage`, `parentPort`, `workerData`, and main event loop integration.
- WebAssembly 2.0: Streaming compilation and instantiation (`WebAssembly.compileStreaming`, `WebAssembly.instantiateStreaming`) consuming `Response` and `Promise<Response>` without intermediate ArrayBuffer string corruption.
- Agent tool sandbox & MCP 2.0: `amber run --sandbox` denies fs/net/env/run with fine-grained allows and structured JSONL audit trail recording (`--audit-log <path>`); `amber session` (stdin JSON-RPC) and `amber mcp` (MCP stdio server) with JSDoc schema extraction and standard error handling.
- Builtins wired: `ai` (`amber:ai`), `assert`, `string_decoder`, `zlib`, `https`, `tls`, `vm`, `worker_threads`, `perf_hooks`, `child_process`, `util`.

This page is the user-facing capability boundary for the current Amber checkout. It is intentionally narrower than many historical stage reports in this repository.

## Source Of Truth

Use these files and checks as the current fact sources:

- `Cargo.toml`: package version, enabled binary targets, Cargo features, and dependencies.
- `src/lib.rs`: the default library module surface and feature-gated modules.
- `src/main.rs`: the active `amber` CLI entrypoint.
- Executable tests and smoke commands run in the current checkout.

[`docs/THREE_YEAR_EXECUTION_CHECKLIST.md`](THREE_YEAR_EXECUTION_CHECKLIST.md) checkboxes are **delivery progress** against the three-year plan. **This page** remains the user-facing capability boundary. Checklist completion is not automatic Stable promotion.

Current facts from those sources:

- Package version is `1.16.1`.
- The active Cargo binary is `amber`, built from `src/main.rs`.
- Default Cargo features are empty: `default = []`.
- The default runtime path used by the CLI is `src/runtime_minimal.rs`.
- Modules present in the repository are not automatically public product capabilities. Many are staged, feature-gated, partially wired, or retained for historical context.

## Stability Levels

### Stable

Stable means the capability is part of the official v1.16.1 release scope, is reachable from the active `amber` binary or default library surface, and is verified by focused smoke tests, Rust integration tests, and conformance suites.

Current stable scope:

- Build Amber from source with Cargo (`v1.16.1`).
- Inspect the CLI with `amber --help`, `amber --version`, or `amber version`.
- Evaluate simple JavaScript snippets with `amber eval <code>`.
- Run JavaScript files with `amber run <file>`.
- Native Agentic AI runtime (`amber:ai`): zero-copy `Tensor` (TypedArray-backed, matmul, dot, norm, softmax, cosineSimilarity), local streaming `LLM`, and `AgentPipeline`.
- Native Test Runner (`amber test [files...]` and `amber test --watch`): automatic discovery and execution.
- Deterministic Sandbox & Virtual Time (`--seed <u64>`, `--freeze-time <spec>`).
- Multi-isolate worker threads via `require('worker_threads')` and `Worker` with bi-directional messaging.
- WebAssembly streaming compilation and instantiation via `WebAssembly.compileStreaming` / `instantiateStreaming`.
- Manage V8 startup snapshots with zero-copy `mmap` backing: `amber snapshot [build|status|clean]`.
- Run a tool file under `--sandbox` with explicit `--allow-*` / `--permission-policy` and structured JSONL audit trail (`--audit-log`).
- Agent tool execution via `amber session` (stdin JSON-RPC) and `amber mcp` (MCP stdio server).
- Use the basic REPL with `amber repl`.
- Use V8-backed execution through `src/runtime_minimal.rs` for repository examples and scripts.
- WinterTC baseline: `DOMException`, `URLPattern`, `navigator`, queuing strategies, `ReadableStream.from`, `amber:sockets` (TCP + rustls TLS), `wintercg`/`wintertc` package export conditions, and `import.meta.main` / `env` / `resolve`.

Stable does not mean Node.js, Bun, or Deno compatibility. It also does not imply a production support commitment.

### Preview

Preview means the capability is present in the default build and is useful for experiments, but its compatibility contract, diagnostics, edge cases, or test coverage are still being tightened.

Current preview scope:

- TypeScript and TSX entry files are accepted by the CLI and pass through oxc before execution. This is transpile-only: types are erased, `using` / Stage 3 decorators are downleveled to ES2022, and TSX emits classic `React.createElement`. Thrown stacks map back to `.ts` lines when oxc emits a source map. There is no project-wide `tsc` type-check.
- `amber serve --https` terminates TLS with rustls (HTTP/1.1 only). `--cert` and `--key` PEM files are required; missing material exits non-zero.
- `amber run --inspect` / `--inspect-brk` expose CDP `/json/version` and `Runtime.evaluate` on the isolate. This is not a full Chrome DevTools / V8 Inspector Protocol on the current `v8` 152.2.0 binding.
- Node.js compatibility modules under `src/nodejs_core/` are installed into the runtime, including areas such as `fs`, `crypto`, `events`, `buffer`, `path`, `os`, `url`, `dns`, `process`, `child_process` (`execSync`, `spawnSync`), `util`, `zlib`, timers, streams, HTTP, networking, readline, and CommonJS `require`. Treat these as compatibility work in progress unless a behavior is covered by current executable tests.
- Web API modules under `src/web_api/` are installed into the runtime, including areas such as fetch, WebSocket, Web Crypto, URL, events, FormData, Abort, Blob, timers, encoding, performance, streams, compression, structured clone, workers, service workers, broadcast channels, and message channels. Treat these as API-specific preview work, not blanket Web platform compatibility.
- Watch and hot reload code paths exist through `amber run --watch`, `amber test --watch`, `src/watcher.rs`, and `src/watcher_websocket.rs`.
- Agent host surface: `amber run --sandbox --export-tools`, `amber session` (stdin JSON-RPC), and `amber mcp` (MCP stdio). Models stay external. `feature=ai` is not a product LLM and may not compile.
- Production-grade Module Bundler 2.0 (`amber bundle`): oxc AST-backed recursive dependency graph resolution, TypeScript/TSX compilation, module scope isolation, CommonJS/JSON interop, and minified single `.js` output.
- Single Executable Application compiler (`amber compile`): bundles and embeds self-contained JS/TS code with the Amber runtime binary into a single, zero-dependency native executable.
- Package installer (`amber install`): package.json dependency resolution with package-lock.json integrity validation.

### Experimental

Experimental means the capability exists as code, command surface, module surface, design work, or historical implementation, but should not be presented as current product capability without fresh verification.

Current experimental scope:

- `amber debug`, `amber serve` HTTP health/fetch handler, `amber init`, `amber create`, `amber add`, `amber remove`, `amber prune`, `amber bunx`, and `amber upgrade`.
- N-API hello loader: `process.dlopen` calls `napi_register_module_v1` so a C hello addon can export `hello()`. Not a Node ABI compatibility commitment; Prisma/sharp are out of scope.
- `amber test --parallel` is rejected (exit code 2). V8 isolates are not shared across threads.
- Lightweight package-management and project setup behavior, including resolver, lifecycle, supply-chain, and package execution paths.
- V8 snapshot, benchmarking helpers, performance reporting, memory/fallback/error support modules, and ecosystem-lite helpers beyond the behaviors covered by current tests.
- Optional Cargo features: `benchmarks` and `observability` are in the CI compile matrix. `cloudnative`, `enterprise`, `multilang`, and `tch` exist in `Cargo.toml` but are not CI-gated (they may not compile). `feature = "ai"` is empty and does not enable extra modules; default `amber:ai` compiles without it. `verbose_logging` is a debug flag only.
- GHCR: `ghcr.io/zh30/amberjs` is linux/amd64 only. Homebrew `Formula/amber.rb` hashes are filled by the Release job. Winget manifest is in-repo only (not submitted to microsoft/winget-pkgs). V8 is the `v8` crate `152.2.0` (Cargo dependency alias `rusty_v8`); the 0.22 upgrade shipped in v1.10.0.

Experimental capabilities may be useful for contributors. They are not stable user promises.

### Historical

Historical means the document or code exists to preserve stage context, design intent, prior experiments, benchmark attempts, or migration notes.

Historical sources include:

- `docs/STAGE_*`
- `docs/IMPLEMENTATION_PLAN_STAGE_*`
- Stage completion reports, progress reports, and stage benchmark reports.
- Older performance comparison documents unless they include a current reproducible command, environment, commit, and validation status.
- Archived progress logs under `docs/archive/`.

Historical material may contain higher version numbers, production-readiness claims, performance multipliers, or broad compatibility statements. Those statements are not current Amber product facts unless revalidated against the current checkout and reflected in this scope page or another current-status document that links back here.

## Performance Claims

Amber does not currently publish a stable performance claim from this scope page.

Any public performance number must include:

- The date and commit or release tag.
- The exact command used to build and run the benchmark.
- Whether the binary was debug or release.
- Hardware, operating system, and relevant runtime versions.
- The benchmark harness and input files.
- Exit-code and output correctness checks, not timing alone.

Historical stage benchmark numbers are design context only. Do not cite them as current performance facts without rerunning and documenting the current command.

## Cargo Features

The default build uses no Cargo features. A feature-gated module is current only for the feature build that was actually checked.

Use focused checks such as:

```bash
cargo check --features observability
cargo check --features benchmarks
```

`enterprise`, `cloudnative`, `multilang`, `tch`, and empty `ai` are not in the v1.16.1 CI matrix.

If a feature build fails or has not been checked in the current branch, document the related capability as Experimental, not Stable.

## Graduation Rule

Move a capability upward only when all of these are true:

- It is reachable through `src/main.rs` or a documented library API in `src/lib.rs`.
- Its command or API behavior is described without relying on historical stage reports.
- Current tests or smoke commands cover the documented behavior.
- Known limitations are documented next to the capability.
- Feature-gated work has a passing feature check for the relevant feature.

Checklist completion in `docs/THREE_YEAR_EXECUTION_CHECKLIST.md` is not a sixth criterion and does not by itself move a capability to Stable.

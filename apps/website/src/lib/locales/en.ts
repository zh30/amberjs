import type { TranslationSchema } from "./types";
import { BEEJS_VERSION } from "../version";

export const en: TranslationSchema = {
  nav: {
    home: "Home",
    docs: "Manual",
    blog: "Release Notes",
    play: "Playground",
    github: "GitHub",
  },
  playground: {
    title: "Playground",
    run: "Run",
    running: "Running",
    language: "Language",
    note: "Runs in your browser (the same engine as this tab), not the bee binary. Amber cannot ship V8 inside WASM. TypeScript is checked and emitted by Monaco — the VS Code editor.",
    output: "Output",
    empty: "Run to see console output.",
    loading: "Loading editor…",
  },
  toggle: {
    label: "Language",
    en: "English",
    zh: "简体中文",
    es: "Español",
    fr: "Français",
    hi: "हिन्दी",
  },
  theme: {
    system: "System",
    light: "Light",
    dark: "Dark",
    toggle: "Switch Theme (System / Light / Dark)",
  },
  footer: {
    statusLabel: "System Status",
    statusValue: "Operational",
    stage: BEEJS_VERSION,
    contact: "Contact",
    email: "support@bee.zhanghe.dev",
    rights: "All rights reserved.",
    builtWith: "Built with Rust & V8",
    docs: "Documentation",
    blog: "Release Notes",
    githubRepo: "GitHub Repository",
    copyright: `© ${new Date().getFullYear()} Amber. Open-source under MIT.`,
  },
  home: {
    heroBadge: BEEJS_VERSION,
    heroBadgeSub: "Wasm 2.0 · bundle/compile · URL/fetch/stream RSI",
    heroBanner:
      "Amber v1.16.0: Wasm 2.0, amber bundle / compile, and URL/fetch/ReadableStream hot paths.",
    heroBannerLink: "/blog/v1.16.0",
    heroTitlePrefix: "A ",
    heroTitleAccent: "JavaScript & TypeScript runtime",
    heroTitleSuffix: " in Rust & V8",
    heroSubtitle:
      "One binary: bee. Run scripts, Jest-style tests, MCP tools, and bee:ai tensors — with an opt-in capability sandbox. Not a Node.js clone. Node Conformance 5.0 is 55/55.",
    ctaPrimary: "Explore Docs",
    ctaSecondary: "Benchmark Showdown",
    ctaNotes: "Release Notes",
    copyBtn: "Copy",
    copiedBtn: "Copied",
    latestArticle: {
      badge: "Release",
      title: "Amber v1.16.0: Wasm 2.0, packaging, and web hot paths",
      desc: "Zero-copy WebAssembly.Memory, amber bundle / compile (Preview), benchmark suite 2.0, and URL / fetch / ReadableStream RSI. Node Conformance 5.0 remains 55/55.",
      readTime: "4 min read",
      date: "2026-09-16",
      link: "/blog/v1.16.0",
      action: "Read Full Post",
    },
    benchmarksHeader: "Measured on Apple M2 Max",
    benchmarksSub:
      "Suite 2.0, 2026-09-16. Amber vs Node.js v22.22.3 vs Bun 1.4.1. Lower time is better except Express (req/s).",
    benchmarksNote:
      "Reproducible from benchmarks/ in the repo. These are one machine, one commit — not SLAs. Bun still wins several microbenches and cold start.",
    benchmarksFilterAll: "All Workloads",
    benchmarksFilterCore: "Core Execution",
    benchmarksFilterIo: "Memory & I/O",
    benchmarksFastest: "⚡ Fastest",
    benchmarksParity: "🏆 Parity",
    benchmarks: [
      {
        id: "url",
        category: "core",
        title: "URL + URLSearchParams (20k)",
        desc: "WHATWG URL parse and search-param mutations",
        beeValue: "7.28 ms",
        beeOps: "137 ops/s",
        bunValue: "15.41 ms",
        bunOps: "65 ops/s",
        nodeValue: "12.54 ms",
        nodeOps: "80 ops/s",
        multiplier: "1.72x vs Node · 2.12x vs Bun",
        isBeeWinner: true,
        beeBar: 100,
        bunBar: 47,
        nodeBar: 58,
      },
      {
        id: "fetch",
        category: "io",
        title: "fetch 100 sequential GETs",
        desc: "HTTP/1.1 keep-alive GET against a local server",
        beeValue: "6.51 ms",
        beeOps: "154 ops/s",
        bunValue: "4.25 ms",
        bunOps: "235 ops/s",
        nodeValue: "17.00 ms",
        nodeOps: "59 ops/s",
        multiplier: "2.61x vs Node · Bun still faster",
        isBeeWinner: false,
        beeBar: 65,
        bunBar: 100,
        nodeBar: 25,
      },
      {
        id: "eventemitter",
        category: "core",
        title: "EventEmitter emit (50k)",
        desc: "Synchronous listener dispatch",
        beeValue: "0.40 ms",
        beeOps: "2,508 ops/s",
        bunValue: "0.83 ms",
        bunOps: "1,205 ops/s",
        nodeValue: "0.48 ms",
        nodeOps: "2,083 ops/s",
        multiplier: "1.21x vs Node · 2.07x vs Bun",
        isBeeWinner: true,
        beeBar: 100,
        bunBar: 48,
        nodeBar: 83,
      },
      {
        id: "stream",
        category: "io",
        title: "ReadableStream 5k chunks",
        desc: "Produce and consume a byte stream",
        beeValue: "0.92 ms",
        beeOps: "1,091 ops/s",
        bunValue: "0.32 ms",
        bunOps: "3,125 ops/s",
        nodeValue: "1.16 ms",
        nodeOps: "862 ops/s",
        multiplier: "1.26x vs Node · Bun still faster",
        isBeeWinner: false,
        beeBar: 35,
        bunBar: 100,
        nodeBar: 28,
      },
      {
        id: "coldstart",
        category: "core",
        title: "CLI cold start (eval 1+1)",
        desc: "Process boot, isolate, evaluate, exit (mean of 20)",
        beeValue: "18.47 ms",
        beeOps: "mean; P95 was jittery",
        bunValue: "8.03 ms",
        bunOps: "fastest cold start",
        nodeValue: "27.53 ms",
        nodeOps: "1.49x slower than Amber",
        multiplier: "1.49x vs Node · Bun still faster",
        isBeeWinner: false,
        beeBar: 43,
        bunBar: 100,
        nodeBar: 29,
      },
      {
        id: "express",
        category: "io",
        title: "Express 5.x (32 conn, 5s)",
        desc: "autocannon throughput — higher is better",
        beeValue: "68.0k req/s",
        beeOps: "avg 0.02 ms",
        bunValue: "64.6k req/s",
        bunOps: "avg 0.02 ms",
        nodeValue: "18.9k req/s",
        nodeOps: "avg 1.17 ms",
        multiplier: "3.59x vs Node · 1.05x vs Bun",
        isBeeWinner: true,
        beeBar: 100,
        bunBar: 95,
        nodeBar: 28,
      },
    ],
    telemetryTitle: "v1.16.0 at a glance",
    telemetrySubtitle:
      "Apple M2 Max, 2026-09-16. Full tables in the repo benchmarks/ directory.",
    telemetryNote:
      "Amber vs Node v22.22.3 vs Bun 1.4.1. Conformance is tests/conformance/ (55 fixtures).",
    telemetry: [
      {
        label: "Node Conformance 5.0",
        value: "55/55",
        delta: "100% fixtures",
        note: "not “Node compatible”",
      },
      {
        label: "Express 5.x",
        value: "68k req/s",
        delta: "3.6x vs Node",
        note: "32 conn, 5s",
      },
      {
        label: "fetch 100 GETs",
        value: "6.51 ms",
        delta: "2.6x vs Node",
        note: "keep-alive HTTP/1.1",
      },
      {
        label: "CLI eval 1+1",
        value: "18.5 ms",
        delta: "1.5x vs Node",
        note: "Bun still ~8 ms",
      },
    ],
    sandboxTitle: "server.ts — Multi-Worker HTTP Architecture",
    sandboxTag: "Lock-Free Worker Pool",
    sandboxComment:
      "// Native Node.js & Web Standard Stream Response with Worker Pool",
    sandboxLog:
      "🚀 Server listening at http://localhost:3000 (8 workers active)",
    sandboxBoot: "Boot time: < 2ms · Thread pool ready",
    featuresTitle: "What you actually get",
    featuresSubtitle:
      "A V8 isolate in Rust, with an agent sandbox and a small set of honest maturity labels.",
    features: [
      {
        title: "One binary",
        desc: "run, eval, repl, test, session, and mcp in `bee`. TypeScript is oxc type-strip, not tsc.",
      },
      {
        title: "Capability sandbox",
        desc: "Opt-in --sandbox with --allow-*, JSON policy, audit JSONL, --seed, and --freeze-time for agent tools.",
      },
      {
        title: "TypeScript without tsc",
        desc: "oxc strips types from .ts / .tsx. Use tsc --noEmit in CI if you want a typecheck.",
      },
      {
        title: "Jest-style tests",
        desc: "amber test with describe / test / expect. --parallel is rejected: isolates are not shared across threads.",
      },
      {
        title: "bee:ai in-process",
        desc: 'Tensor, LLM, and AgentPipeline without a Python sidecar. Cargo feature = "ai" is empty and is not a product LLM.',
      },
      {
        title: "Node APIs, per-API",
        desc: "fs, http, fetch, Streams, URL, Web Crypto, and more. Coverage is scored in 55 conformance fixtures, not claimed as drop-in Node.",
      },
    ],
    systemsTitle: "Runtime Subsystems",
    systemsSubtitle: "Modular, high-performance architecture built in Rust.",
    systemsMeta: "Architecture Map",
    systemsLabel: "subsystem",
    systems: [
      {
        title: "V8 Runtime & Isolate Core",
        desc: "Direct V8 C++ bindings providing native execution speed, minimal heap footprint, and WASM JIT support.",
      },
      {
        title: "oxc Transpilation & Code Quality",
        desc: "Ultra-fast Rust AST engine stripping types in sub-milliseconds, powering built-in formatting (bee fmt) and linting (bee lint).",
      },
      {
        title: "Modern Web Serving & Concurrency",
        desc: "Standard Fetch API web serving (bee serve) and lockless multi-Worker thread pool concurrency model.",
      },
      {
        title: "Deterministic Agent Sandbox & Quotas",
        desc: "CPU watchdog hard interrupts, physical heap caps, Mulberry32 deterministic PRNG replay, and frozen timestamps.",
      },
      {
        title: "Packaging & Standalone SEA Compiler",
        desc: "Production Bundler 2.0 module graph packaging and zero-dependency standalone executable compiler (amber compile).",
      },
      {
        title: "Developer Experience & Debugging (CDP & LSP)",
        desc: "Featuring upgraded interactive REPL (bee repl), Chrome DevTools remote debugging, and Language Server Protocol (bee lsp).",
      },
    ],
    ctaTitle: "Install bee and run something.",
    ctaSubtitle:
      "v1.16.0 prebuilds for macOS, Linux, and Windows. One curl (or irm) away.",
    ctaButton: "Read Installation Guide",
    ctaNotesButton: "Read Release Notes",
  },
  docs: {
    title: "Runtime Manual",
    subtitle:
      "Manual for Amber v1.16.0 — runtime, CLI, sandbox, and agent APIs.",
    backToHome: "Return Home",
    searchPlaceholder: "Search docs, CLI commands & APIs...",
    onThisPage: "On this page",
    previousPage: "Previous",
    nextPage: "Next",
    groups: [
      {
        title: "Getting Started",
        items: [
          { id: "introduction", label: "Overview & Philosophy" },
          { id: "installation", label: "Installation & Setup" },
          { id: "quick-start", label: "Quick Start Guide" },
        ],
      },
      {
        title: "Core Systems",
        items: [
          { id: "v8-isolate-pool", label: "Runtime & V8 Core" },
          {
            id: "isolate-pool",
            label: "Multi-Tenant IsolatePool (bee:pool)",
            badge: "v1.4",
          },
          { id: "jit-optimization", label: "TypeScript 6.0 & TSX" },
          { id: "ai-engine", label: "Native AI Engine (bee:ai)", badge: "AI" },
          {
            id: "ai-embeddings",
            label: "Native Text Embeddings & Vectors",
            badge: "v1.3",
          },
          {
            id: "server-mode",
            label: "Modern Web Server & Fetch",
            badge: "v1.0",
          },
          { id: "memory-management", label: "SIMD & Memory Model" },
        ],
      },
      {
        title: "Developer Tooling",
        items: [
          {
            id: "task-runner",
            label: "Task Runner & Script Exec",
            badge: "NEW",
          },
          {
            id: "code-quality",
            label: "Code Formatter & Linter",
            badge: "NEW",
          },
          {
            id: "bundling-compilation",
            label: "Bundler 2.0 & SEA Compiler",
            badge: "Preview",
          },
          {
            id: "testing-benchmarking",
            label: "Testing, Coverage & Benchmarks",
            badge: "NEW",
          },
          {
            id: "debugging-lsp",
            label: "REPL, CDP Debugger & LSP",
            badge: "NEW",
          },
        ],
      },
      {
        title: "Ecosystem & Tooling",
        items: [
          {
            id: "embedded-db",
            label: "Embedded DB & Vector Engine (bee:db & bee:vector)",
            badge: "DB",
          },
          {
            id: "standard-library",
            label: "Modern Standard Library (bee:std)",
            badge: "Std",
          },
          {
            id: "package-manager-dlx",
            label: "Dynamic Package Runner (amber x / dlx)",
            badge: "CLI",
          },
          {
            id: "deployment-docker",
            label: "Deployment & Containerization (bee deploy)",
            badge: "Deploy",
          },
          {
            id: "ide-extension",
            label: "VS Code Official Extension",
            badge: "IDE",
          },
        ],
      },
      {
        title: "Agent & Advanced",
        items: [
          {
            id: "agent-sandbox",
            label: "Deterministic Sandbox & Quotas",
            badge: "Agent",
          },
          {
            id: "agent-replay",
            label: "Deterministic Agent Replay Engine (bee:replay)",
            badge: "v1.6",
          },
          {
            id: "model-weights",
            label: "GGUF & SafeTensors Loader (bee:weights)",
            badge: "v1.6",
          },
          {
            id: "capability-security",
            label: "Enterprise Capability Security (bee:security)",
            badge: "v1.6",
          },
          {
            id: "kv-store",
            label: "Persistent KV & Durable State (bee:kv)",
            badge: "v1.7",
          },
          {
            id: "tool-synthesis",
            label: "Tool Auto-Synthesis & OpenAPI (bee:tools)",
            badge: "v1.7",
          },
          {
            id: "hardened-sandbox",
            label: "Hardened Sandbox & Audit Logs (bee:sandbox)",
            badge: "v1.7",
          },
          {
            id: "agent-bus",
            label: "Agent Message Bus & PubSub (bee:bus)",
            badge: "v1.8",
          },
          {
            id: "streaming-grammar",
            label: "Streaming Partial JSON & Grammars (bee:grammar)",
            badge: "v1.8",
          },
          {
            id: "agent-checkpoint",
            label: "Agent State Checkpointing (bee:checkpoint)",
            badge: "v1.8",
          },
          {
            id: "mcp-protocol",
            label: "Model Context Protocol 2.0 (bee:mcp)",
            badge: "v1.3",
          },
          {
            id: "virtual-fs-sandbox",
            label: "Virtual Filesystem In-Memory Sandbox",
            badge: "v1.3",
          },
          {
            id: "ffi-native",
            label: "Native C ABI FFI (bee:ffi)",
            badge: "v1.4",
          },
          {
            id: "wasm-interop",
            label: "Wasm 2.0 Zero-Copy Bridge (bee:wasm)",
            badge: "v1.16",
          },
          {
            id: "slm-inference",
            label: "Edge SLM & JSON Decoding (bee:ai)",
            badge: "v1.4",
          },
          {
            id: "framework-compat",
            label: "npm Frameworks (Hono / Express / LangChain)",
            badge: "v1.5",
          },
          {
            id: "import-maps-native",
            label: "WICG Import Maps & Addons",
            badge: "v1.0",
          },
          {
            id: "types-lsp",
            label: "TypeScript Type Declarations",
            badge: "Types",
          },
        ],
      },
      {
        title: "Reference & Specs",
        items: [
          { id: "cli-usage", label: "Complete CLI Command Reference" },
          { id: "wintertc-compliance", label: "WinterTC Compliance" },
          { id: "api-reference", label: "Full API Reference" },
          { id: "modules", label: "Module Resolution & Architecture" },
        ],
      },
    ],
    sections: {
      introduction: {
        title: "Overview",
        subtitle: "Rust + V8 runtime for JavaScript and TypeScript.",
        body: [
          "Amber v1.16.0 is a Rust + V8 JavaScript/TypeScript runtime in one binary: bee. Node Conformance 5.0 is 55/55 fixtures — that is not drop-in Node compatibility.",
          "The repository also contains historical stage reports and feature-gated modules. Those documents are useful for design history, but the public release promise follows the default Cargo build.",
        ],
        cards: [
          {
            title: "Clean CLI",
            desc: "Default run and eval output avoids internal setup logs.",
          },
          {
            title: "Default Build",
            desc: "Release checks target the same feature set users install.",
          },
        ],
      },
      installation: {
        title: "Installation",
        subtitle: "Install a prebuilt archive or build from source.",
        body: [
          "Prebuilt release archives currently target macOS x86_64, macOS arm64, and Linux x86_64. Other platforms can build from source with Rust.",
        ],
        code: [
          "$ curl -fsSL https://amberjs.com/install.sh | sh",
          "$ amber --version",
        ],
      },
      "quick-start": {
        title: "Quick Start",
        subtitle: "Run your first script.",
        body: [
          "Create a JavaScript or TypeScript file and execute it with the run subcommand.",
        ],
        code: [
          'console.log("Hello from Amber");',
          "amber run hello.js",
          'bee eval "1 + 1"',
        ],
      },
      "v8-isolate-pool": {
        title: "Runtime Core",
        subtitle: "The active CLI path uses V8 through Rust.",
        body: [
          "The default binary entry is src/main.rs. Script execution is handled by src/runtime_minimal.rs, which owns the V8 isolate, context setup, and result handling.",
        ],
        list: [
          "Execute JavaScript files with amber run",
          "Evaluate snippets with bee eval",
          "Use bee repl for an interactive shell",
        ],
      },
      "jit-optimization": {
        title: "TypeScript",
        subtitle: "TS and TSX files are transpiled by oxc before execution.",
        body: [
          "When a .ts or .tsx file is passed to amber run, the CLI routes it through oxc (TypeScript 6.0 syntax, transpile-only). Types are erased. using and Stage 3 decorators downlevel to ES2022 for the current V8. TSX emits classic React.createElement. TypeScript 7.0 added no new language syntax.",
        ],
        list: [
          "Use .ts, .tsx, .mts, .cts, and .jsx entry files",
          "This is not tsc --noEmit. Invalid types can still run if the JS is valid",
          "Unused value imports stay (side effects). Only import type is erased",
          "amber run examples/basics/typescript_latest.ts",
        ],
      },
      "memory-management": {
        title: "Compatibility",
        subtitle: "Selected Node.js and Web APIs are available.",
        body: [
          "The default build includes compatibility layers for common Node.js and Web APIs. Coverage is partial and should be checked against examples or tests before relying on a specific edge case.",
        ],
        list: [
          "Node.js modules include fs, path, crypto, buffer, process, timers, and require",
          "Web APIs include fetch, URL, streams, Blob, events, timers, and Web Crypto pieces",
        ],
      },
      "server-mode": {
        title: "Serve Mode",
        subtitle: "Health-check stub, not an application server.",
        body: [
          'bee serve binds a tiny_http listener and returns a fixed {"ok":true} JSON body. It does not execute user scripts. For application HTTP, use http.createServer and amber run.',
        ],
        code: ["$ bee serve --host localhost --port 3000"],
      },
      "cli-usage": {
        title: "CLI Usage",
        subtitle: "Core commands.",
        list: [
          "amber run <file> - execute a JavaScript or TypeScript file",
          "bee eval <code> - evaluate a JavaScript snippet",
          "amber test [file] - run the built-in or file-based test runner",
          "amber bundle <entry> - write a production bundle",
          "bee serve - health stub (fixed JSON, not user scripts)",
          "amber install - install dependencies from package.json",
        ],
      },
      "api-reference": {
        title: "Full API Reference",
        subtitle:
          "Comprehensive specification of native Amber subsystems (amber:*), Node.js core, and Web APIs.",
        body: [
          "Manual for the default bee binary: CLI, Node/Web APIs (per-API, Conformance 5.0 is 55/55 fixtures), bee:ai, sandbox, and Preview packaging.",
        ],
        list: [
          "Amber Native Subsystems (bee:ai, bee:db, bee:vector, bee:bus, bee:grammar, bee:checkpoint, etc.)",
          "Node.js Core Compatibility (fs, net, http, crypto, stream, worker_threads, etc. 51 modules)",
          "Web Standard APIs (fetch, WebCrypto, Streams, WebSocket, Worker, structuredClone)",
        ],
      },
      modules: {
        title: "Modules",
        subtitle: "Default module boundaries.",
        list: [
          "src/runtime_minimal.rs - current V8 runtime",
          "src/nodejs_core/ - Node.js compatibility modules",
          "src/web_api/ - Web API modules",
          "src/testing/ - test framework",
          "src/package_manager.rs - package manager support",
        ],
      },
      "embedded-db": {
        title: "Embedded Database & Vector Engine",
        subtitle: "Native SQLite relational DB and vector similarity search.",
        body: [
          "Native SQLite 3 and Rust-based high-dimensional vector search engine with zero external dependencies.",
        ],
        list: [
          "bee:db - in-process SQLite database",
          "bee:vector - high-performance vector search engine",
        ],
      },
      "standard-library": {
        title: "Modern Standard Library",
        subtitle: "Zero-dependency official standard library.",
        body: [
          "Includes dotenv, terminal styling and tables, directory walking, UUID/JWT crypto, and assertions.",
        ],
        list: [
          "bee:std/dotenv",
          "bee:std/cli",
          "bee:std/fs",
          "bee:std/crypto",
          "bee:std/assert",
        ],
      },
      "package-manager-dlx": {
        title: "Dynamic Package Runner",
        subtitle:
          "Execute remote npm packages and CLI tools without pre-installing.",
        body: [
          "Fetches and executes CLI packages in isolated V8 environments with global cache and quota enforcement.",
        ],
        code: ['$ amber x cowsay "Hello Amber!"'],
      },
      "deployment-docker": {
        title: "Deployment & Containerization",
        subtitle:
          "One-command generation of Docker, standalone SEA binary, and Kubernetes manifests.",
        body: [
          "Generates hardened multi-stage Dockerfiles, compose stacks, and cloud-native Kubernetes manifests.",
        ],
        code: ["$ bee deploy --target docker", "$ bee deploy --target k8s"],
      },
      "ide-extension": {
        title: "VS Code Official Extension",
        subtitle:
          "Deep integration with LSP, CDP debugger, formatters, and deploy.",
        body: [
          "First-class developer experience for Visual Studio Code with native completions, debugging, and format-on-save.",
        ],
      },
    },
  },
  blog: {
    title: "Release Notes",
    subtitle: "Runtime notes, implementation updates, and release scope.",
    tagLabel: "Topic",
    back: "Return to Notes",
    operator: "Author",
    by: "By ",
    timestamp: "Date",
    readTime: "Read Time",
    readMore: "Open Note",
    notFound: "Post Not Found",
    fallbackNote: "",
  },
};

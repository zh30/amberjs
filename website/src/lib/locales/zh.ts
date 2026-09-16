import type { TranslationSchema } from "./types";
import { BEEJS_VERSION } from "../version";

export const zh: TranslationSchema = {
  nav: {
    home: "首页",
    docs: "手册",
    blog: "发布日志",
    github: "GitHub",
  },
  toggle: {
    label: "语言",
    en: "English",
    zh: "简体中文",
    es: "Español",
    fr: "Français",
    hi: "हिन्दी",
  },
  theme: {
    system: "跟随系统",
    light: "浅色模式",
    dark: "深色模式",
    toggle: "切换主题（跟随系统 / 浅色 / 深色）",
  },
  footer: {
    statusLabel: "系统状态",
    statusValue: "运行中",
    stage: BEEJS_VERSION,
    contact: "联系",
    email: "support@bee.zhanghe.dev",
    rights: "保留所有权利。",
    builtWith: "基于 Rust & V8 构建",
    docs: "文档手册",
    blog: "发布日志",
    githubRepo: "GitHub 仓库",
    copyright: `© ${new Date().getFullYear()} Beejs. 基于 MIT 协议开源。`,
  },
  home: {
    heroBadge: BEEJS_VERSION,
    heroBadgeSub: "Wasm 2.0 · bundle/compile · URL/fetch/stream 热路径",
    heroBanner:
      "Beejs v1.16.0：Wasm 2.0、bee bundle / compile，以及 URL / fetch / ReadableStream 热路径。",
    heroBannerLink: "/blog/v1.16.0",
    heroTitlePrefix: "用 Rust 和 V8 做的 ",
    heroTitleAccent: "JavaScript & TypeScript 运行时",
    heroTitleSuffix: "",
    heroSubtitle:
      "一个二进制：bee。跑脚本、Jest 风格测试、MCP 工具和 bee:ai 张量，可选能力沙箱。不是 Node.js 替代品。Node Conformance 5.0 为 55/55。",
    ctaPrimary: "查阅文档手册",
    ctaSecondary: "硬核性能实测",
    ctaNotes: "发布日志",
    copyBtn: "复制",
    copiedBtn: "已复制",
    latestArticle: {
      badge: "发版",
      title: "Beejs v1.16.0：Wasm 2.0、打包，以及 Web 热路径",
      desc: "零拷贝 WebAssembly.Memory、bee bundle / compile（Preview）、基准套件 2.0，以及 URL / fetch / ReadableStream RSI。Node Conformance 5.0 仍是 55/55。",
      readTime: "4 分钟阅读",
      date: "2026-09-16",
      link: "/blog/v1.16.0",
      action: "阅读全文",
    },
    benchmarksHeader: "Apple M2 Max 实测",
    benchmarksSub:
      "套件 2.0，2026-09-16。Beejs vs Node.js v22.22.3 vs Bun 1.4.1。除 Express（req/s）外，时间越低越好。",
    benchmarksNote:
      "可在仓库 benchmarks/ 复现。这是一台机器、一次提交，不是 SLA。Bun 在若干微基准和冷启动上仍然更快。",
    benchmarksFilterAll: "全部基准负载",
    benchmarksFilterCore: "核心计算与执行",
    benchmarksFilterIo: "内存与 I/O 吞吐",
    benchmarksFastest: "⚡ 最快",
    benchmarksParity: "🏆 顶尖",
    benchmarks: [
      {
        id: "url",
        category: "core",
        title: "URL + URLSearchParams（20k）",
        desc: "WHATWG URL 解析与 search 参数改写",
        beeValue: "7.28 ms",
        beeOps: "137 ops/s",
        bunValue: "15.41 ms",
        bunOps: "65 ops/s",
        nodeValue: "12.54 ms",
        nodeOps: "80 ops/s",
        multiplier: "比 Node 快 1.72x · 比 Bun 快 2.12x",
        isBeeWinner: true,
        beeBar: 100,
        bunBar: 47,
        nodeBar: 58,
      },
      {
        id: "fetch",
        category: "io",
        title: "fetch 连续 100 次 GET",
        desc: "对本地服务器的 HTTP/1.1 keep-alive GET",
        beeValue: "6.51 ms",
        beeOps: "154 ops/s",
        bunValue: "4.25 ms",
        bunOps: "235 ops/s",
        nodeValue: "17.00 ms",
        nodeOps: "59 ops/s",
        multiplier: "比 Node 快 2.61x · Bun 仍更快",
        isBeeWinner: false,
        beeBar: 65,
        bunBar: 100,
        nodeBar: 25,
      },
      {
        id: "eventemitter",
        category: "core",
        title: "EventEmitter emit（50k）",
        desc: "同步监听器分发",
        beeValue: "0.40 ms",
        beeOps: "2,508 ops/s",
        bunValue: "0.83 ms",
        bunOps: "1,205 ops/s",
        nodeValue: "0.48 ms",
        nodeOps: "2,083 ops/s",
        multiplier: "比 Node 快 1.21x · 比 Bun 快 2.07x",
        isBeeWinner: true,
        beeBar: 100,
        bunBar: 48,
        nodeBar: 83,
      },
      {
        id: "stream",
        category: "io",
        title: "ReadableStream 5k chunks",
        desc: "生产并消费字节流",
        beeValue: "0.92 ms",
        beeOps: "1,091 ops/s",
        bunValue: "0.32 ms",
        bunOps: "3,125 ops/s",
        nodeValue: "1.16 ms",
        nodeOps: "862 ops/s",
        multiplier: "比 Node 快 1.26x · Bun 仍更快",
        isBeeWinner: false,
        beeBar: 35,
        bunBar: 100,
        nodeBar: 28,
      },
      {
        id: "coldstart",
        category: "core",
        title: "CLI 冷启动（eval 1+1）",
        desc: "进程拉起、isolate、求值、退出（20 次均值）",
        beeValue: "18.47 ms",
        beeOps: "均值；P95 仍有抖动",
        bunValue: "8.03 ms",
        bunOps: "冷启动最快",
        nodeValue: "27.53 ms",
        nodeOps: "比 Beejs 慢 1.49x",
        multiplier: "比 Node 快 1.49x · Bun 仍更快",
        isBeeWinner: false,
        beeBar: 43,
        bunBar: 100,
        nodeBar: 29,
      },
      {
        id: "express",
        category: "io",
        title: "Express 5.x（32 连接，5 秒）",
        desc: "autocannon 吞吐 — 越高越好",
        beeValue: "68.0k req/s",
        beeOps: "平均 0.02 ms",
        bunValue: "64.6k req/s",
        bunOps: "平均 0.02 ms",
        nodeValue: "18.9k req/s",
        nodeOps: "平均 1.17 ms",
        multiplier: "比 Node 快 3.59x · 比 Bun 快 1.05x",
        isBeeWinner: true,
        beeBar: 100,
        bunBar: 95,
        nodeBar: 28,
      },
    ],
    telemetryTitle: "v1.16.0 一览",
    telemetrySubtitle: "Apple M2 Max，2026-09-16。完整表在仓库 benchmarks/。",
    telemetryNote:
      "Beejs vs Node v22.22.3 vs Bun 1.4.1。符合度是 tests/conformance/（55 个 fixtures）。",
    telemetry: [
      {
        label: "Node Conformance 5.0",
        value: "55/55",
        delta: "fixtures 全过",
        note: "不是「兼容 Node」",
      },
      {
        label: "Express 5.x",
        value: "68k req/s",
        delta: "比 Node 快 3.6x",
        note: "32 连接，5 秒",
      },
      {
        label: "fetch 100 次 GET",
        value: "6.51 ms",
        delta: "比 Node 快 2.6x",
        note: "keep-alive HTTP/1.1",
      },
      {
        label: "CLI eval 1+1",
        value: "18.5 ms",
        delta: "比 Node 快 1.5x",
        note: "Bun 仍约 8 ms",
      },
    ],
    sandboxTitle: "server.ts — 多 Worker HTTP 并发架构",
    sandboxTag: "无锁跨线程分发",
    sandboxComment:
      "// 原生支持 Node.js 与 Web 标准流式响应，内置 Worker 线程池并发",
    sandboxLog:
      "🚀 服务已启动，监听 http://localhost:3000 (8 个 Worker 线程并发就绪)",
    sandboxBoot: "启动耗时：< 2ms · 工作线程池就绪",
    featuresTitle: "实际能用的东西",
    featuresSubtitle:
      "Rust 里的 V8 isolate，加上 Agent 沙箱，以及诚实的成熟度标签。",
    features: [
      {
        title: "一个二进制",
        desc: "run、eval、repl、test、session、mcp 都在 `bee` 里。TypeScript 是 oxc 类型擦除，不是 tsc。",
      },
      {
        title: "能力沙箱",
        desc: "可选 --sandbox，配合 --allow-*、JSON 策略、审计 JSONL、--seed 和 --freeze-time。",
      },
      {
        title: "不用 tsc 的 TypeScript",
        desc: "oxc 擦掉 .ts / .tsx 的类型。需要类型检查时在 CI 跑 tsc --noEmit。",
      },
      {
        title: "Jest 风格测试",
        desc: "bee test 支持 describe / test / expect。--parallel 会被拒绝：isolate 不能跨线程共享。",
      },
      {
        title: "进程内 bee:ai",
        desc: 'Tensor、LLM、AgentPipeline，不需要 Python sidecar。Cargo feature = "ai" 是空的，不是产品级 LLM。',
      },
      {
        title: "按 API 计的 Node 面",
        desc: "fs、http、fetch、Streams、URL、Web Crypto 等。用 55 个符合度 fixtures 记分，不宣称 drop-in Node。",
      },
    ],
    systemsTitle: "运行时核心子系统",
    systemsSubtitle: "基于 Rust 构建的模块化、超高性能系统架构。",
    systemsMeta: "架构蓝图",
    systemsLabel: "子系统",
    systems: [
      {
        title: "V8 运行时与 Isolate 核心",
        desc: "深度集成 Google V8 C++ 绑定，带来原生级执行性能、轻量堆内存占用与 WASM JIT 支持。",
      },
      {
        title: "oxc 转译与代码质量引擎",
        desc: "采用超高速 Rust AST 引擎，以亚毫秒级速度擦除类型，并提供内置极速格式化 (bee fmt) 与静态检查 (bee lint)。",
      },
      {
        title: "现代 Web 服务与并发架构",
        desc: "原生支持标准 Fetch API 应用服务 (bee serve)，并具备无锁多 Worker 线程池并发模型。",
      },
      {
        title: "Agent 确定性沙箱与资源配额",
        desc: "CPU 超时 Watchdog 强行中断死循环，物理堆上限隔离，Mulberry32 随机重放与时间戳冻结。",
      },
      {
        title: "工程打包与 SEA 独立可执行编译",
        desc: "生产级 Bundler 2.0 模块打包与无需依赖的单二进制应用编译器 (bee compile)。",
      },
      {
        title: "开发者体验与调试协议 (CDP & LSP)",
        desc: "包含升级版交互终端 (bee repl)、Chrome DevTools 远程调试以及语言服务器 (bee lsp)。",
      },
    ],
    ctaTitle: "装上 bee，跑一段代码。",
    ctaSubtitle:
      "v1.16.0 预编译包覆盖 macOS、Linux、Windows。一条 curl（或 irm）。",
    ctaButton: "查看安装手册",
    ctaNotesButton: "阅读技术发布日志",
  },
  docs: {
    title: "运行时手册",
    subtitle: "Beejs v1.16.0 手册 — 运行时、CLI、沙箱与 Agent API。",
    backToHome: "返回首页",
    searchPlaceholder: "搜索文档、命令与 API...",
    onThisPage: "本页大纲",
    previousPage: "上一篇",
    nextPage: "下一篇",
    groups: [
      {
        title: "入门指南",
        items: [
          { id: "introduction", label: "概览与设计理念" },
          { id: "installation", label: "安装与环境配置" },
          { id: "quick-start", label: "快速上手指南" },
        ],
      },
      {
        title: "核心系统",
        items: [
          { id: "v8-isolate-pool", label: "运行时与 V8 架构" },
          {
            id: "isolate-pool",
            label: "多租户 IsolatePool (bee:pool)",
            badge: "v1.4",
          },
          { id: "jit-optimization", label: "TypeScript 6.0 与 TSX" },
          { id: "ai-engine", label: "原生 AI 引擎 (bee:ai)", badge: "AI" },
          {
            id: "ai-embeddings",
            label: "零依赖原生 Embedding 与语义向量",
            badge: "v1.3",
          },
          { id: "server-mode", label: "现代 Web 服务与并发", badge: "v1.0" },
          { id: "memory-management", label: "SIMD 与内存模型" },
        ],
      },
      {
        title: "工程工具链",
        items: [
          { id: "task-runner", label: "任务调度与脚本执行", badge: "NEW" },
          { id: "code-quality", label: "代码格式化与规范检查", badge: "NEW" },
          {
            id: "bundling-compilation",
            label: "打包器 2.0 与 SEA 独立二进制",
            badge: "Preview",
          },
          {
            id: "testing-benchmarking",
            label: "测试覆盖率与微基准套件",
            badge: "NEW",
          },
          {
            id: "debugging-lsp",
            label: "交互终端、CDP 调试与 LSP",
            badge: "NEW",
          },
        ],
      },
      {
        title: "生态与扩展",
        items: [
          {
            id: "embedded-db",
            label: "嵌入式数据与向量引擎 (bee:db & bee:vector)",
            badge: "DB",
          },
          {
            id: "standard-library",
            label: "现代官方标准库 (bee:std)",
            badge: "Std",
          },
          {
            id: "package-manager-dlx",
            label: "动态包即时运行 (bee x / dlx)",
            badge: "CLI",
          },
          {
            id: "deployment-docker",
            label: "全自动部署与容器编排 (bee deploy)",
            badge: "Deploy",
          },
          {
            id: "ide-extension",
            label: "VS Code 官方编辑器插件",
            badge: "IDE",
          },
        ],
      },
      {
        title: "Agent 与高级特性",
        items: [
          {
            id: "agent-sandbox",
            label: "确定性沙箱与资源硬配额",
            badge: "Agent",
          },
          {
            id: "agent-replay",
            label: "确定性 Agent 回放引擎 (bee:replay)",
            badge: "v1.6",
          },
          {
            id: "model-weights",
            label: "原生 GGUF / SafeTensors 权重加载 (bee:weights)",
            badge: "v1.6",
          },
          {
            id: "capability-security",
            label: "企业级能力安全控制 (bee:security)",
            badge: "v1.6",
          },
          {
            id: "kv-store",
            label: "持久化 KV 与可靠状态引擎 (bee:kv)",
            badge: "v1.7",
          },
          {
            id: "tool-synthesis",
            label: "工具自动合成与 OpenAPI 编译器 (bee:tools)",
            badge: "v1.7",
          },
          {
            id: "hardened-sandbox",
            label: "加固执行沙箱与合规审计日志 (bee:sandbox)",
            badge: "v1.7",
          },
          {
            id: "agent-bus",
            label: "多 Agent 消息总线与 PubSub (bee:bus)",
            badge: "v1.8",
          },
          {
            id: "streaming-grammar",
            label: "流式结构化自愈与 Token 语法 (bee:grammar)",
            badge: "v1.8",
          },
          {
            id: "agent-checkpoint",
            label: "Agent 状态检查点与回退 (bee:checkpoint)",
            badge: "v1.8",
          },
          {
            id: "mcp-protocol",
            label: "Model Context Protocol 2.0 (bee:mcp)",
            badge: "v1.3",
          },
          {
            id: "virtual-fs-sandbox",
            label: "纯内存隔离 Virtual Filesystem (VFS)",
            badge: "v1.3",
          },
          {
            id: "ffi-native",
            label: "原生 C ABI 外部接口 (bee:ffi)",
            badge: "v1.4",
          },
          {
            id: "wasm-interop",
            label: "Wasm 2.0 零拷贝互通 (bee:wasm)",
            badge: "v1.16",
          },
          {
            id: "slm-inference",
            label: "端侧 SLM 与约束 JSON 解码 (bee:ai)",
            badge: "v1.4",
          },
          {
            id: "framework-compat",
            label: "主流 npm 框架兼容 (Hono / Express / LangChain)",
            badge: "v1.5",
          },
          {
            id: "import-maps-native",
            label: "WICG 导入映射与原生插件",
            badge: "v1.0",
          },
          {
            id: "types-lsp",
            label: "官方 TypeScript 类型定义",
            badge: "Types",
          },
        ],
      },
      {
        title: "参考与规范",
        items: [
          { id: "cli-usage", label: "CLI 命令行完整参考手册" },
          { id: "wintertc-compliance", label: "WinterTC 合规" },
          { id: "api-reference", label: "完整 API 参考手册" },
          { id: "modules", label: "模块解析与工程边界" },
        ],
      },
    ],
    sections: {
      introduction: {
        title: "概览",
        subtitle: "Rust + V8 构建的 JavaScript 和 TypeScript 运行时。",
        body: [
          "Beejs v1.16.0 是 Rust + V8 的 JavaScript/TypeScript 运行时，一个二进制：bee。Node Conformance 5.0 为 55/55 fixtures —— 这不是 drop-in Node 兼容。",
          "仓库仍保留历史阶段报告和 feature-gated 模块。这些资料适合了解设计背景，但公开发布承诺以默认 Cargo 构建为准。",
        ],
        cards: [
          {
            title: "干净 CLI",
            desc: "默认 run 和 eval 输出不泄漏内部初始化日志。",
          },
          {
            title: "默认构建",
            desc: "发布检查覆盖用户实际安装的 feature 集。",
          },
        ],
      },
      installation: {
        title: "安装",
        subtitle: "使用预编译包或从源码构建。",
        body: [
          "预编译发布产物当前覆盖 macOS x86_64、macOS arm64 和 Linux x86_64。其他平台可通过 Rust 从源码构建。",
        ],
        code: [
          "$ curl -fsSL https://bee.zhanghe.dev/install.sh | sh",
          "$ bee --version",
        ],
      },
      "quick-start": {
        title: "快速开始",
        subtitle: "运行第一段脚本。",
        body: ["创建 JavaScript 或 TypeScript 文件，并通过 run 子命令执行。"],
        code: [
          'console.log("Hello from Beejs");',
          "bee run hello.js",
          'bee eval "1 + 1"',
        ],
      },
      "v8-isolate-pool": {
        title: "运行时核心",
        subtitle: "当前 CLI 路径通过 Rust 驱动 V8。",
        body: [
          "默认二进制入口是 src/main.rs。脚本执行由 src/runtime_minimal.rs 处理，负责 V8 isolate、上下文初始化和结果返回。",
        ],
        list: [
          "用 bee run 执行 JavaScript 文件",
          "用 bee eval 执行片段",
          "用 bee repl 进入交互式终端",
        ],
      },
      "jit-optimization": {
        title: "TypeScript",
        subtitle: "TS 和 TSX 文件执行前由 oxc 转译。",
        body: [
          "当 .ts 或 .tsx 文件传给 bee run 时，CLI 会走 oxc（TypeScript 6.0 语法，仅转译）。类型会被擦除。using 和 Stage 3 装饰器会降级到 ES2022，以适应当前 V8。TSX 输出 classic React.createElement。TypeScript 7.0 没有新语法。",
        ],
        list: [
          "可使用 .ts、.tsx、.mts、.cts、.jsx 入口文件",
          "这不是 tsc --noEmit。类型错误只要生成的 JS 合法仍可能执行",
          "未使用的 value import 会保留（可能有副作用），只擦除 import type",
          "bee run examples/basics/typescript_latest.ts",
        ],
      },
      "memory-management": {
        title: "兼容层",
        subtitle: "提供选定 Node.js 和 Web API。",
        body: [
          "默认构建包含常见 Node.js 与 Web API 兼容层。覆盖并非完整标准实现，依赖具体边界前应查看示例或测试。",
        ],
        list: [
          "Node.js 模块包括 fs、path、crypto、buffer、process、timers 和 require",
          "Web API 包括 fetch、URL、streams、Blob、events、timers 和部分 Web Crypto",
        ],
      },
      "server-mode": {
        title: "Serve 模式",
        subtitle: "健康检查 stub，不是应用服务器。",
        body: [
          'bee serve 用 tiny_http 绑定端口并返回固定 {"ok":true}，不执行用户脚本。应用 HTTP 请用 http.createServer 和 bee run。',
        ],
        code: ["$ bee serve --host localhost --port 3000"],
      },
      "cli-usage": {
        title: "CLI 用法",
        subtitle: "核心命令。",
        list: [
          "bee run <file> - 执行 JavaScript 或 TypeScript 文件",
          "bee eval <code> - 执行 JavaScript 片段",
          "bee test [file] - 运行内置或文件测试",
          "bee bundle <entry> - 写出生产 bundle",
          "bee serve - 健康检查 stub（固定 JSON，不跑用户脚本）",
          "bee install - 从 package.json 安装依赖",
        ],
      },
      "api-reference": {
        title: "完整 API 参考手册",
        subtitle:
          "BeeJS 原生子系统 (bee:*)、Node.js 核心及 Web 标准 API 规范。",
        body: [
          "默认 bee 二进制手册：CLI、Node/Web API（按 API 计，Conformance 5.0 为 55/55 fixtures）、bee:ai、沙箱，以及 Preview 打包。",
        ],
        list: [
          "BeeJS 原生子系统 (bee:ai, bee:db, bee:vector, bee:bus, bee:grammar, bee:checkpoint 等)",
          "Node.js 兼容层 (fs, net, http, crypto, stream, worker_threads 等 51 个核心模块)",
          "Web 标准 API (fetch, WebCrypto, Streams, WebSocket, Worker, structuredClone)",
        ],
      },
      modules: {
        title: "模块",
        subtitle: "默认模块边界。",
        list: [
          "src/runtime_minimal.rs - 当前 V8 运行时",
          "src/nodejs_core/ - Node.js 兼容模块",
          "src/web_api/ - Web API 模块",
          "src/testing/ - 测试框架",
          "src/package_manager.rs - 包管理支持",
        ],
      },
      "embedded-db": {
        title: "嵌入式数据与向量引擎",
        subtitle: "原生的 SQLite 关系数据库与高维向量相似度检索。",
        body: [
          "集成 SQLite 3 与基于 Rust 的高维向量相似度检索，提供零外部依赖的数据管理与本地 RAG 检索底座。",
        ],
        list: [
          "bee:db - 嵌入式 SQLite 数据库",
          "bee:vector - 高性能向量检索数据库",
        ],
      },
      "standard-library": {
        title: "官方现代标准库",
        subtitle: "零依赖、工程实用的现代标准库。",
        body: [
          "提供 dotenv、终端样式与表格交互、高级文件系统遍历与拷贝、UUID/JWT 密码学、深度相等断言。",
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
        title: "动态包运行器",
        subtitle: "免安装即时运行远程 npm 包与 CLI 工具。",
        body: [
          "无需事先 npm install，极速拉取并直接在 V8 隔离环境中执行，内置全局缓存与配额沙箱。",
        ],
        code: ['$ bee x cowsay "Hello Beejs!"'],
      },
      "deployment-docker": {
        title: "全自动部署与容器编排",
        subtitle: "一键生成 Docker、独立 SEA 二进制与 Kubernetes 清单。",
        body: [
          "自动化分析项目入口，输出生产级多阶段 Dockerfile、极简 compose 文件与云原生 k8s 配置。",
        ],
        code: ["$ bee deploy --target docker", "$ bee deploy --target k8s"],
      },
      "ide-extension": {
        title: "VS Code 官方编辑器插件",
        subtitle: "深度集成 LSP、CDP 调试、极速格式化与一键部署。",
        body: [
          "为 Visual Studio Code 提供官方一站式开发体验，全面支持智能代码提示、单步断点调试与保存格式化。",
        ],
      },
    },
  },
  blog: {
    title: "发布日志",
    subtitle: "运行时动态、工程实现更新与版本发布范围。",
    tagLabel: "主题",
    back: "返回发布日志",
    operator: "作者",
    by: "作者：",
    timestamp: "日期",
    readTime: "阅读时长",
    readMore: "阅读全文",
    notFound: "未找到相关文章",
    fallbackNote: "",
  },
};

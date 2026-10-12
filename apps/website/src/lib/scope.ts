/**
 * Mirror of docs/CURRENT_SCOPE.md (v1.17.0, G1–G69 on main).
 * Every Stable row must point at a contract that CURRENT_SCOPE names.
 * Do not add rows from stage reports or the execution checklist.
 */

export const SCOPE_DOC_URL =
  "https://github.com/zh30/amberjs/blob/main/docs/CURRENT_SCOPE.md";
export const CONFORMANCE_URL =
  "https://github.com/zh30/amberjs/blob/main/tests/conformance/scorecard.md";
export const REPO_URL = "https://github.com/zh30/amberjs";

export const CONFORMANCE = { pass: 56, total: 56, gate: 55 } as const;

export function contractUrl(doc: string): string {
  return `${REPO_URL}/blob/main/docs/${doc}.md`;
}

export type Area = "cli" | "runtime" | "node" | "web";

export interface L10n {
  en: string;
  zh: string;
}

/** Limits and details are written in English and Chinese; other locales read English. */
export function pick(text: L10n, lang: string): string {
  return lang === "zh" ? text.zh : text.en;
}

export interface StableItem {
  /** Graduation id from CURRENT_SCOPE, e.g. "G9". Omitted for the base release scope. */
  g?: string;
  qualifier?: "tentative" | "provisional";
  area: Area;
  name: string;
  /** Module or command surface, rendered as code. */
  api: string;
  /** Contract file stem under docs/. */
  doc?: string;
  limits: L10n;
}

export const STABLE: readonly StableItem[] = [
  // CLI & tooling
  {
    area: "cli",
    name: "amber run / eval / repl",
    api: "amber run <file> · amber eval <code> · amber repl",
    limits: {
      en: "V8 execution through runtime_minimal; repl is the basic shell.",
      zh: "经 runtime_minimal 在 V8 中执行；repl 为基础交互式环境。",
    },
  },
  {
    area: "cli",
    name: "amber test",
    api: "amber test [files...]",
    limits: {
      en: "Jest-style describe / test / expect with discovery. --parallel exits 2.",
      zh: "Jest 风格 describe / test / expect，自动发现。--parallel 以退出码 2 拒绝。",
    },
  },
  {
    g: "G1",
    area: "cli",
    name: "amber bundle",
    api: "amber bundle <entry>",
    doc: "BUNDLE_CONTRACT",
    limits: {
      en: "oxc IIFE bundle; static import/require only; not webpack/esbuild parity.",
      zh: "oxc 打包为单个 IIFE；仅静态 import/require；不对标 webpack/esbuild。",
    },
  },
  {
    g: "G2",
    area: "cli",
    name: "amber compile",
    api: "amber compile <entry> -o <bin>",
    doc: "COMPILE_CONTRACT",
    limits: {
      en: "Host single-executable on Linux/macOS/Windows; no cross-compile, no native addons.",
      zh: "在 Linux/macOS/Windows 生成本机单文件可执行；不支持交叉编译和原生插件。",
    },
  },
  {
    g: "G3",
    area: "cli",
    name: "amber install",
    api: "amber install [--frozen-lockfile]",
    doc: "INSTALL_CONTRACT",
    limits: {
      en: "Direct deps from package.json with lockfile pins and SRI checks; not npm/yarn/pnpm.",
      zh: "安装 package.json 直接依赖，校验 lockfile 与 SRI；不是 npm/yarn/pnpm 的替代品。",
    },
  },
  {
    g: "G4",
    area: "cli",
    name: "amber run --export-tools",
    api: "amber run --export-tools <file>",
    doc: "EXPORT_TOOLS_CONTRACT",
    limits: {
      en: "Prints tool schemas as JSON and exits; never executes the file or calls a model.",
      zh: "输出工具 schema JSON 后退出；不执行文件，不调用模型。",
    },
  },
  {
    g: "G5",
    area: "cli",
    name: "amber run --inspect",
    api: "--inspect · --inspect-brk · --inspect-port",
    doc: "INSPECT_CONTRACT",
    limits: {
      en: "CDP subset on 127.0.0.1 (Runtime.evaluate); no breakpoints or stepping.",
      zh: "127.0.0.1 上的 CDP 子集（Runtime.evaluate）；无断点与单步。",
    },
  },
  {
    g: "G6",
    area: "cli",
    name: "--watch",
    api: "amber run --watch · amber test --watch",
    doc: "WATCH_CONTRACT",
    limits: {
      en: "Full re-exec on change plus a localhost reload WebSocket; not in-place HMR.",
      zh: "文件变更后整体重新执行，并推送本地 WebSocket 事件；不是原地 HMR。",
    },
  },
  {
    g: "G41",
    area: "cli",
    name: "amber serve",
    api: "amber serve [--port] [script]",
    doc: "SERVE_HTTP_CONTRACT",
    limits: {
      en: "HTTP/1.1 health document or fetch-handler bridge; one request per connection.",
      zh: "HTTP/1.1 健康检查文档或 fetch 处理函数桥接；每连接一个请求。",
    },
  },
  {
    g: "G7",
    area: "cli",
    name: "amber serve --https",
    api: "amber serve --https --cert --key",
    doc: "SERVE_HTTPS_CONTRACT",
    limits: {
      en: "rustls TLS, HTTP/1.1 only; no HTTP/2, keep-alive, chunked bodies or static files.",
      zh: "rustls TLS，仅 HTTP/1.1；无 HTTP/2、keep-alive、分块传输或静态文件。",
    },
  },
  {
    g: "G36",
    area: "cli",
    name: "TypeScript / TSX",
    api: ".ts · .tsx · .mts · .cts · .jsx",
    doc: "TYPESCRIPT_CONTRACT",
    limits: {
      en: "oxc transpile-only with .ts stack remap; not tsc — no type-checking.",
      zh: "oxc 仅转译，堆栈映射回 .ts 行号；不是 tsc，不做类型检查。",
    },
  },
  {
    area: "cli",
    name: "amber session / amber mcp",
    api: "amber session <tools> · amber mcp <tools>",
    limits: {
      en: "Agent tool execution over stdin JSON-RPC and an MCP stdio server.",
      zh: "通过 stdin JSON-RPC 与 MCP stdio 服务器执行 Agent 工具。",
    },
  },
  {
    area: "cli",
    name: "amber snapshot",
    api: "amber snapshot [build|status|clean]",
    limits: {
      en: "Manage V8 startup snapshots with mmap backing.",
      zh: "管理基于 mmap 的 V8 启动快照。",
    },
  },

  // Runtime: sandbox, determinism, ai, wasm, wintertc
  {
    area: "runtime",
    name: "Capability sandbox",
    api: "--sandbox · --allow-* · --permission-policy · --audit-log",
    limits: {
      en: "Deny-by-default fs/net/env/run with exact allows and a JSONL audit trail.",
      zh: "默认拒绝 fs/net/env/run，按精确路径/主机放行，并写入 JSONL 审计日志。",
    },
  },
  {
    area: "runtime",
    name: "Deterministic execution",
    api: "--seed <u64> · --freeze-time <spec>",
    limits: {
      en: "Seeded Math.random / getRandomValues and a frozen virtual clock.",
      zh: "可复现的 Math.random / getRandomValues 与冻结的虚拟时钟。",
    },
  },
  {
    area: "runtime",
    name: "amber:ai",
    api: "Tensor · LLM · AgentPipeline",
    limits: {
      en: "In-process TypedArray tensors, local streaming LLM, pipelines. Cargo feature \"ai\" is empty.",
      zh: "进程内 TypedArray 张量、本地流式 LLM 与管线。Cargo feature \"ai\" 为空。",
    },
  },
  {
    area: "runtime",
    name: "WebAssembly streaming",
    api: "WebAssembly.compileStreaming · instantiateStreaming",
    limits: {
      en: "V8 Wasm; streaming compile from Response. amber:wasm zero-copy helpers are Preview.",
      zh: "V8 Wasm；从 Response 流式编译。amber:wasm 零拷贝辅助为 Preview。",
    },
  },
  {
    area: "runtime",
    name: "WinterTC baseline",
    api: "DOMException · URLPattern · navigator · amber:sockets · import.meta",
    limits: {
      en: "ECMA-429 minimum common API, wintertc export conditions, TCP + rustls sockets.",
      zh: "ECMA-429 最小公共 API、wintertc 导出条件、TCP + rustls 套接字。",
    },
  },

  // Node.js
  {
    g: "G28",
    qualifier: "provisional",
    area: "node",
    name: "CommonJS require",
    api: "require · module · require.resolve · createRequire",
    doc: "REQUIRE_CONTRACT",
    limits: {
      en: "node_modules walk, package.json exports/imports/main; not the Node Module class.",
      zh: "node_modules 查找、package.json exports/imports/main；不是 Node 的 Module 类。",
    },
  },
  {
    g: "G8",
    area: "node",
    name: "fs",
    api: "fs · fs/promises",
    doc: "FS_CONTRACT",
    limits: {
      en: "Byte I/O, mkdir, stat, fds, readdir dirents, promises; no glob / cp / opendir / statfs.",
      zh: "字节读写、mkdir、stat、文件描述符、readdir、promises；无 glob / cp / opendir / statfs。",
    },
  },
  {
    g: "G64",
    qualifier: "tentative",
    area: "node",
    name: "fs.watch / fs.watchFile",
    api: "fs.watch · fs.watchFile · fs.unwatchFile",
    doc: "FS_WATCH_CONTRACT",
    limits: {
      en: "Real OS watcher (notify) and stat polling; no fs.promises.watch.",
      zh: "真实系统文件监听（notify）与 stat 轮询；无 fs.promises.watch。",
    },
  },
  {
    g: "G17",
    area: "node",
    name: "path",
    api: "path · path.posix",
    doc: "PATH_CONTRACT",
    limits: {
      en: "POSIX semantics; path.win32 methods are the POSIX implementations.",
      zh: "POSIX 语义；path.win32 的方法仍为 POSIX 实现。",
    },
  },
  {
    g: "G18",
    area: "node",
    name: "events",
    api: "EventEmitter · once",
    doc: "NODE_EVENTS_CONTRACT",
    limits: {
      en: "Full listener API and static once; no events.on async iterator.",
      zh: "完整监听器 API 与静态 once；无 events.on 异步迭代器。",
    },
  },
  {
    g: "G19",
    area: "node",
    name: "buffer",
    api: "Buffer",
    doc: "BUFFER_CONTRACT",
    limits: {
      en: "from / alloc / concat / toString / slice; readUInt* and compare are outside.",
      zh: "from / alloc / concat / toString / slice；readUInt* 与 compare 不在契约内。",
    },
  },
  {
    g: "G20",
    area: "node",
    name: "os",
    api: "platform · arch · cpus · tmpdir · homedir …",
    doc: "OS_CONTRACT",
    limits: {
      en: "Host info subset; cpus()[].speed / times are outside.",
      zh: "主机信息子集；cpus()[].speed / times 不在契约内。",
    },
  },
  {
    g: "G62",
    area: "node",
    name: "os.hostname",
    api: "os.hostname()",
    doc: "OS_HOSTNAME_CONTRACT",
    limits: {
      en: "Real gethostname / GetComputerNameW; not a DNS FQDN.",
      zh: "真实 gethostname / GetComputerNameW；不是 DNS FQDN。",
    },
  },
  {
    g: "G63",
    area: "node",
    name: "os.networkInterfaces",
    api: "os.networkInterfaces()",
    doc: "OS_NETWORK_INTERFACES_CONTRACT",
    limits: {
      en: "Real OS enumerator; MAC is best-effort.",
      zh: "真实系统网卡枚举；MAC 尽力而为。",
    },
  },
  {
    g: "G21",
    area: "node",
    name: "zlib (sync)",
    api: "gzipSync · deflateSync · inflateRawSync …",
    doc: "ZLIB_CONTRACT",
    limits: {
      en: "Six sync codecs; no brotli, no zlib.constants.",
      zh: "六个同步编解码函数；无 brotli 与 zlib.constants。",
    },
  },
  {
    g: "G48 · G58",
    qualifier: "tentative",
    area: "node",
    name: "zlib (async callbacks)",
    api: "gzip · gunzip · deflate · inflate · deflateRaw · inflateRaw",
    doc: "ZLIB_ASYNC_CONTRACT",
    limits: {
      en: "Callbacks on a later tick; no streaming createGzip family.",
      zh: "在后续 tick 回调；无流式 createGzip 系列。",
    },
  },
  {
    g: "G22 · G59",
    area: "node",
    name: "util",
    api: "format · inspect · promisify · callbackify · types",
    doc: "UTIL_CONTRACT",
    limits: {
      en: "Basics plus util.types.isArrayBuffer / isRegExp; inspect ignores depth / colors.",
      zh: "基础函数及 util.types.isArrayBuffer / isRegExp；inspect 不支持 depth / colors。",
    },
  },
  {
    g: "G22 · G44 · G53 · G54",
    area: "node",
    name: "process",
    api: "nextTick · env · cwd · stdout.write · hrtime · memoryUsage",
    doc: "PROCESS_CONTRACT",
    limits: {
      en: "Basics only; hrtime is epoch-based, heap numbers are estimates; exit / stdin stay Preview.",
      zh: "仅基础能力；hrtime 基于 epoch，堆数据为估算；exit / stdin 仍为 Preview。",
    },
  },
  {
    g: "G23",
    area: "node",
    name: "crypto",
    api: "createHash · createHmac · randomBytes · randomUUID · timingSafeEqual",
    doc: "NODE_CRYPTO_CONTRACT",
    limits: {
      en: "md5 / sha* / blake3 digests; KDF, sign/verify and KeyObject stay Preview.",
      zh: "md5 / sha* / blake3 摘要；KDF、签名验签与 KeyObject 仍为 Preview。",
    },
  },
  {
    g: "G43 · G51 · G52",
    area: "node",
    name: "crypto cipheriv",
    api: "createCipheriv · createDecipheriv (AES-CBC / GCM / CTR)",
    doc: "NODE_CRYPTO_CIPHERIV_CONTRACT",
    limits: {
      en: "AES-128/192/256 CBC, GCM (auth tag, AAD) and CTR; no password createCipher.",
      zh: "AES-128/192/256 的 CBC、GCM（认证标签、AAD）与 CTR；无口令版 createCipher。",
    },
  },
  {
    g: "G24",
    area: "node",
    name: "stream",
    api: "Readable · Writable · PassThrough · pipe",
    doc: "NODE_STREAM_CONTRACT",
    limits: {
      en: "Flowing push / pipe; no backpressure. Transform / Duplex / pipeline stay Preview.",
      zh: "流动模式 push / pipe；无背压。Transform / Duplex / pipeline 仍为 Preview。",
    },
  },
  {
    g: "G25",
    area: "node",
    name: "child_process (sync)",
    api: "execSync · spawnSync",
    doc: "CHILD_PROCESS_CONTRACT",
    limits: {
      en: "Sandbox-aware; no timeout / cwd / env options.",
      zh: "受沙箱约束；无 timeout / cwd / env 选项。",
    },
  },
  {
    g: "G47 · G60",
    qualifier: "tentative",
    area: "node",
    name: "child_process (async)",
    api: "exec · execFile · spawn",
    doc: "CHILD_PROCESS_ASYNC_CONTRACT",
    limits: {
      en: "Captured output via callback on a later turn; no live stdio pipes.",
      zh: "在后续事件循环回调返回捕获的输出；无实时 stdio 管道。",
    },
  },
  {
    g: "G67",
    qualifier: "tentative",
    area: "node",
    name: "child_process.fork",
    api: "fork · child.send · process.send",
    doc: "CHILD_PROCESS_FORK_CONTRACT",
    limits: {
      en: "Real OS process with JSON IPC over localhost TCP.",
      zh: "真实系统子进程，通过本地 TCP 传递 JSON IPC。",
    },
  },
  {
    g: "G26",
    area: "node",
    name: "url",
    api: "fileURLToPath · pathToFileURL",
    doc: "NODE_URL_CONTRACT",
    limits: {
      en: "POSIX file URL helpers; legacy parse / format are outside.",
      zh: "POSIX 文件 URL 辅助函数；旧版 parse / format 不在契约内。",
    },
  },
  {
    g: "G27 · G65",
    qualifier: "provisional",
    area: "node",
    name: "dns",
    api: "lookup · resolve4 · resolve6 · reverse · getServers",
    doc: "DNS_CONTRACT",
    limits: {
      en: "Host getaddrinfo / getnameinfo, sync; no dns.promises or setServers.",
      zh: "基于系统 getaddrinfo / getnameinfo，同步；无 dns.promises 与 setServers。",
    },
  },
  {
    g: "G37",
    qualifier: "tentative",
    area: "node",
    name: "http (server)",
    api: "http.createServer · listen · req / res",
    doc: "HTTP_SERVER_CONTRACT",
    limits: {
      en: "Buffered responses; the http client, Agent, https and http2 are not supported.",
      zh: "响应完整缓冲；不支持 http 客户端、Agent、https 与 http2。",
    },
  },
  {
    g: "G45 · G61",
    qualifier: "tentative",
    area: "node",
    name: "net",
    api: "net.createServer · net.connect",
    doc: "NET_SERVER_CONTRACT",
    limits: {
      en: "Real TCP listen / accept / connect; no TLS, unix sockets or full duplex parity.",
      zh: "真实 TCP 监听 / 接受 / 连接；无 TLS、unix 套接字或完整双工语义。",
    },
  },
  {
    g: "G38",
    area: "node",
    name: "querystring",
    api: "parse · stringify · escape · unescape",
    doc: "QUERYSTRING_CONTRACT",
    limits: {
      en: "Fixed & / = separators; no options.",
      zh: "固定 & / = 分隔符；无选项。",
    },
  },
  {
    g: "G39 · G68",
    area: "node",
    name: "string_decoder",
    api: "StringDecoder",
    doc: "STRING_DECODER_CONTRACT",
    limits: {
      en: "utf8, latin1, ascii, base64, hex, utf16le with partial-unit buffering; no base64url.",
      zh: "utf8、latin1、ascii、base64、hex、utf16le，支持跨块缓冲；无 base64url。",
    },
  },
  {
    g: "G40 · G69",
    area: "node",
    name: "assert",
    api: "ok · strictEqual · deepStrictEqual · throws · match · rejects",
    doc: "ASSERT_CONTRACT",
    limits: {
      en: "Strict assertions; loose equal coercion and the deepEqual alias are not pinned.",
      zh: "严格断言；宽松 equal 强制转换与 deepEqual 别名不在契约内。",
    },
  },
  {
    g: "G42 · G50",
    area: "node",
    name: "timers",
    api: "timers · timers/promises",
    doc: "TIMERS_MODULE_CONTRACT",
    limits: {
      en: "Module timers and Promise setTimeout / setImmediate with AbortSignal.",
      zh: "模块化计时器，以及支持 AbortSignal 的 Promise 版 setTimeout / setImmediate。",
    },
  },
  {
    g: "G46",
    qualifier: "tentative",
    area: "node",
    name: "readline",
    api: "createInterface · question",
    doc: "READLINE_CONTRACT",
    limits: {
      en: "One line from host stdin; no history, completer or readline.promises.",
      zh: "从宿主 stdin 读取一行；无历史、补全与 readline.promises。",
    },
  },
  {
    g: "G49",
    area: "node",
    name: "tty",
    api: "tty.isatty",
    doc: "TTY_CONTRACT",
    limits: {
      en: "Real isatty on Unix; always false on Windows today.",
      zh: "Unix 上为真实 isatty；Windows 上目前总是 false。",
    },
  },
  {
    g: "G55 · G66",
    qualifier: "tentative",
    area: "node",
    name: "vm",
    api: "runInThisContext · runInNewContext · Script",
    doc: "VM_CONTRACT",
    limits: {
      en: "Same-context run and a fresh-context property-copy sandbox; not Contextify parity.",
      zh: "同上下文执行，以及基于属性复制的新上下文沙箱；不等同于 Contextify。",
    },
  },
  {
    g: "G56",
    qualifier: "tentative",
    area: "node",
    name: "diagnostics_channel",
    api: "channel · subscribe · tracingChannel.traceSync",
    doc: "DIAGNOSTICS_CHANNEL_CONTRACT",
    limits: {
      en: "Current-isolate pub/sub; no tracePromise.",
      zh: "当前 isolate 内发布订阅；无 tracePromise。",
    },
  },
  {
    g: "G57",
    qualifier: "tentative",
    area: "node",
    name: "async_hooks",
    api: "AsyncResource · AsyncLocalStorage",
    doc: "ASYNC_HOOKS_CONTRACT",
    limits: {
      en: "ALS run / getStore / snapshot; createHook is not supported.",
      zh: "ALS run / getStore / snapshot；不支持 createHook。",
    },
  },

  // Web platform
  {
    g: "G9",
    area: "web",
    name: "fetch",
    api: "fetch · Request · Response · Headers",
    doc: "FETCH_CONTRACT",
    limits: {
      en: "Streaming body reader, redirects, integrity, AbortSignal; no browser CORS origin.",
      zh: "流式读取响应体、重定向、integrity、AbortSignal；无浏览器 CORS 源模型。",
    },
  },
  {
    g: "G10",
    area: "web",
    name: "URL & encoding",
    api: "URL · URLSearchParams · TextEncoder · TextDecoder · structuredClone",
    doc: "URL_ENCODING_CONTRACT",
    limits: {
      en: "No IDNA; TextDecoder is UTF-8 only.",
      zh: "无 IDNA；TextDecoder 仅支持 UTF-8。",
    },
  },
  {
    g: "G11",
    area: "web",
    name: "Blob / File / FormData",
    api: "Blob · File · FormData",
    doc: "BLOB_FORMDATA_CONTRACT",
    limits: {
      en: "Multipart fetch bodies; text() / arrayBuffer() are synchronous.",
      zh: "fetch 支持 multipart 请求体；text() / arrayBuffer() 为同步返回。",
    },
  },
  {
    g: "G12",
    area: "web",
    name: "Events & channels",
    api: "EventTarget · AbortController · MessageChannel · BroadcastChannel",
    doc: "EVENTS_CHANNELS_CONTRACT",
    limits: {
      en: "No DOM tree; BroadcastChannel stays within one isolate.",
      zh: "无 DOM 树；BroadcastChannel 不跨 isolate。",
    },
  },
  {
    g: "G13",
    area: "web",
    name: "Streams, compression, timers",
    api: "ReadableStream · CompressionStream · setTimeout · performance",
    doc: "STREAMS_TIMERS_CONTRACT",
    limits: {
      en: "Queuing strategies do not apply backpressure.",
      zh: "排队策略不施加背压。",
    },
  },
  {
    g: "G14",
    area: "web",
    name: "Web Crypto",
    api: "crypto.subtle · getRandomValues · randomUUID",
    doc: "WEB_CRYPTO_CONTRACT",
    limits: {
      en: "SHA, HMAC, AES, PBKDF2, ECDH, Ed25519, ECDSA, RSA; no HKDF / RSA-PSS / X25519.",
      zh: "SHA、HMAC、AES、PBKDF2、ECDH、Ed25519、ECDSA、RSA；无 HKDF / RSA-PSS / X25519。",
    },
  },
  {
    g: "G15",
    area: "web",
    name: "WebSocket (client)",
    api: "new WebSocket('ws://…')",
    doc: "WEBSOCKET_CONTRACT",
    limits: {
      en: "Real ws:// handshake; no subprotocols or permessage-deflate; not a server.",
      zh: "真实 ws:// 握手；无子协议与 permessage-deflate；不是服务端。",
    },
  },
  {
    g: "G16",
    area: "web",
    name: "Workers",
    api: "Worker · worker_threads · navigator.serviceWorker",
    doc: "WORKERS_CONTRACT",
    limits: {
      en: "Dedicated workers with JSON messages; no SharedWorker or module workers.",
      zh: "专用 Worker，消息为 JSON；无 SharedWorker 与模块 Worker。",
    },
  },
  {
    g: "G29",
    area: "web",
    name: "atob / btoa",
    api: "atob · btoa",
    doc: "ATOB_BTOA_CONTRACT",
    limits: {
      en: "Standard Base64; throws plain Error, not DOMException.",
      zh: "标准 Base64；抛出普通 Error 而非 DOMException。",
    },
  },
  {
    g: "G30",
    qualifier: "provisional",
    area: "web",
    name: "ErrorEvent / onerror",
    api: "ErrorEvent · globalThis.onerror",
    doc: "ERROR_EVENT_CONTRACT",
    limits: {
      en: "Uncaught throws can be handled by onerror; not a document error pipeline.",
      zh: "onerror 可处理未捕获异常；不是文档级错误管线。",
    },
  },
  {
    g: "G31",
    area: "web",
    name: "Clipboard (text)",
    api: "navigator.clipboard.writeText / readText",
    doc: "CLIPBOARD_CONTRACT",
    limits: {
      en: "Process-local text store, not the OS clipboard; no ClipboardItem.",
      zh: "进程内文本存储，不是系统剪贴板；无 ClipboardItem。",
    },
  },
  {
    g: "G32",
    area: "web",
    name: "DOMParser",
    api: "new DOMParser().parseFromString()",
    doc: "DOMPARSER_CONTRACT",
    limits: {
      en: "Read-only HTML / XML parse tree with selectors; not a live DOM.",
      zh: "只读 HTML / XML 解析树与选择器；不是可变的 DOM。",
    },
  },
  {
    g: "G33",
    area: "web",
    name: "Background Sync",
    api: "registration.sync · SyncEvent",
    doc: "BACKGROUND_SYNC_CONTRACT",
    limits: {
      en: "Fires on the same CLI isolate; not Periodic Sync.",
      zh: "在同一 CLI isolate 触发；不是周期性同步。",
    },
  },
  {
    g: "G34",
    area: "web",
    name: "Cache Storage",
    api: "caches.open · cache.match / put",
    doc: "CACHE_CONTRACT",
    limits: {
      en: "In-process store; not an HTTP cache and not persisted.",
      zh: "进程内存储；不是 HTTP 缓存，也不落盘。",
    },
  },
  {
    g: "G35",
    area: "web",
    name: "Service worker fetch",
    api: "FetchEvent.respondWith",
    doc: "SW_FETCH_CONTRACT",
    limits: {
      en: "Activated worker intercepts page fetch; scope is not matched.",
      zh: "已激活的 worker 拦截页面 fetch；不匹配 scope。",
    },
  },
];

export interface ScopeItem {
  name: string;
  detail: L10n;
}

export const PREVIEW: readonly ScopeItem[] = [
  {
    name: "Rest of src/nodejs_core/",
    detail: {
      en: "Node modules and methods outside the contracts above: stream Transform / Duplex / pipeline, crypto KDF / sign / KeyObject, process.exit / stdin, perf_hooks, and more.",
      zh: "上述契约之外的 Node 模块与方法：stream Transform / Duplex / pipeline、crypto KDF / 签名 / KeyObject、process.exit / stdin、perf_hooks 等。",
    },
  },
  {
    name: "Rest of src/web_api/",
    detail: {
      en: "Web APIs outside the Stable contracts, such as ExtendableEvent. Treated per-API, not as blanket Web compatibility.",
      zh: "Stable 契约之外的 Web API，例如 ExtendableEvent。按 API 逐个评估，不代表整体 Web 兼容。",
    },
  },
  {
    name: "amber:wasm",
    detail: {
      en: "Zero-copy WebAssembly.Memory / ArrayBuffer helpers and mmap module load.",
      zh: "零拷贝 WebAssembly.Memory / ArrayBuffer 辅助与 mmap 模块加载。",
    },
  },
];

export const EXPERIMENTAL: readonly ScopeItem[] = [
  {
    name: "amber init · create · add · remove · prune · upgrade · x",
    detail: {
      en: "Package and scaffolding commands outside the Stable amber install contract.",
      zh: "Stable amber install 契约之外的包管理与脚手架命令。",
    },
  },
  {
    name: "amber debug",
    detail: {
      en: "Debug command; use the Stable --inspect CDP subset instead.",
      zh: "调试命令；建议使用 Stable 的 --inspect CDP 子集。",
    },
  },
  {
    name: "N-API hello loader",
    detail: {
      en: "process.dlopen can load a C hello addon. Not Node ABI compatibility; Prisma and sharp are out of scope.",
      zh: "process.dlopen 可加载 C 语言 hello 插件。不代表 Node ABI 兼容；Prisma 与 sharp 不在范围内。",
    },
  },
  {
    name: "Cargo features",
    detail: {
      en: "benchmarks and observability are CI-checked. cloudnative, enterprise, multilang and tch are not CI-gated; feature \"ai\" is empty.",
      zh: "benchmarks 与 observability 在 CI 中编译检查。cloudnative、enterprise、multilang、tch 不受 CI 约束；feature \"ai\" 为空。",
    },
  },
  {
    name: "Distribution channels",
    detail: {
      en: "GHCR image is linux/amd64 only. The winget manifest lives in-repo and is not submitted.",
      zh: "GHCR 镜像仅 linux/amd64。winget 清单仅在仓库内，未提交到官方源。",
    },
  },
];

/** CLI subcommands, flags and amber:* modules present in src/ but not classified by CURRENT_SCOPE. */
export const UNCLASSIFIED: readonly string[] = [
  "amber fmt",
  "amber lint",
  "amber bench",
  "amber task",
  "amber types",
  "amber lsp",
  "amber profile",
  "amber deploy",
  "amber record / replay",
  "--virtual-fs",
  "--import-map",
  "amber:mcp",
  "amber:kv",
  "amber:bus",
  "amber:grammar",
  "amber:checkpoint",
  "amber:weights",
  "amber:tools",
  "amber:security",
  "amber:std",
  "amber:db",
  "amber:vector",
  "amber:pool",
  "amber:ffi",
];

export const NON_GOALS: readonly ScopeItem[] = [
  {
    name: "SharedWorker · module workers",
    detail: {
      en: "new Worker(url, { type: 'module' }) throws.",
      zh: "new Worker(url, { type: 'module' }) 会抛出异常。",
    },
  },
  {
    name: "Push · Notification · Payment",
    detail: {
      en: "Push subscription rejects. These browser surfaces are deferred.",
      zh: "Push 订阅会被拒绝。这些浏览器能力均已延后。",
    },
  },
  {
    name: "ClipboardItem",
    detail: {
      en: "clipboard.read / write reject with an Error naming ClipboardItem.",
      zh: "clipboard.read / write 会以提及 ClipboardItem 的 Error 拒绝。",
    },
  },
  {
    name: "http client · Agent · https · http2",
    detail: {
      en: "Use the Stable Web fetch for outbound HTTP. Only the http server subset is Stable.",
      zh: "出站 HTTP 请使用 Stable 的 Web fetch。只有 http 服务端子集是 Stable。",
    },
  },
  {
    name: "Streaming zlib · brotli",
    detail: {
      en: "createGzip / createDeflate families, brotli and zlib.constants are not implemented.",
      zh: "未实现 createGzip / createDeflate 系列、brotli 与 zlib.constants。",
    },
  },
  {
    name: "Type-checking",
    detail: {
      en: "TypeScript is transpile-only. Run tsc --noEmit in CI if you need type errors.",
      zh: "TypeScript 仅转译。如需类型错误，请在 CI 中运行 tsc --noEmit。",
    },
  },
  {
    name: "fs.glob · fs.cp · fs.opendir · fs.statfs",
    detail: {
      en: "Not implemented; also no fs.promises.watch.",
      zh: "未实现；也没有 fs.promises.watch。",
    },
  },
  {
    name: "Parallel tests · cross-compile",
    detail: {
      en: "amber test --parallel exits 2. amber compile only targets the host OS.",
      zh: "amber test --parallel 以退出码 2 拒绝。amber compile 只面向当前主机系统。",
    },
  },
  {
    name: "npm workspaces · lifecycle scripts",
    detail: {
      en: "amber install does not run scripts, link .bin, or read yarn / pnpm lockfiles.",
      zh: "amber install 不执行生命周期脚本、不链接 .bin，也不读取 yarn / pnpm 锁文件。",
    },
  },
];

/** Graduation ids G1–G69. Rows above group several ids, so do not derive this from STABLE.length. */
export const GRADUATED_CONTRACTS = 69;

---
title: "Node.js 兼容性"
nav: "Node.js API"
subtitle: "哪些 Node 模块是 Stable、每份契约覆盖什么、哪些在范围之外。"
status: "stable"
id: "node-compat"
---

Amber 不是 Node.js 的直接替代品。下表中每个模块只对其契约里列出的方法是 Stable，`src/nodejs_core/` 中其余部分都是 Preview。可搜索视图见 [兼容性页面](/compatibility)。

## 模块与加载

| API | 编号 | 说明 |
| :--- | :--- | :--- |
| CommonJS `require`、`module.exports`、`require.resolve`、`createRequire` | G28（provisional） | `node_modules` 查找，`"exports"` / `"imports"` / `"main"`，JSON、TS，以及通过 `require` 加载 ESM。没有 `require.cache`；解析错误没有 `code`。 |
| `node:` 前缀与内置模块名 | G28 | 能加载某个内置模块，并不代表它的全部 API 都是 Stable。 |

## 文件系统与路径

| API | 编号 | 说明 |
| :--- | :--- | :--- |
| `fs`、`fs/promises` | G8、G77 | 字节读写、`mkdir`、`stat`、`copyFile`、`rm`、`realpath`、fd、`chmod`、`access`、`existsSync`、`readdir` dirent、系统错误。`promises.open` 返回 `FileHandle`。异步操作在下一个 tick 于 isolate 线程上执行。 |
| `fs.watch`、`fs.watchFile` | G64（tentative） | 真实的系统文件监听与 stat 轮询。没有 `fs.promises.watch`。 |
| `fs.cp`、`fs.opendir`、`fs.statfs` | G76（tentative） | 递归复制、`Dir` 快照，以及 Unix `statfs`。 |
| `path` | G17 | POSIX 语义。`path.win32` 只改变 `sep` 和 `delimiter`。 |

`fs.glob` 未实现。

## 核心工具

| API | 编号 | 说明 |
| :--- | :--- | :--- |
| `events`（`EventEmitter`） | G18 | 没有 `events.on` 异步迭代器。 |
| `buffer` / `Buffer` | G19 | `from`、`alloc`、`concat`、`byteLength`、`toString`、`slice`。`compare` 和 `readUInt*` 不在固定范围内。 |
| `util` | G22、G59 | `format`、浅层 `inspect`、`promisify`、`callbackify`、`inherits`、列出的 `util.types`。 |
| `process` | G22、G44、G53、G54、G70 | `nextTick`、`env`、`cwd`、`pid`、`platform`、`stdout.write`、`hrtime`、`memoryUsage`，以及 `stdin` 的 UTF-8 字符串分块。堆字段是估算值；`process.on` / `off` 是空操作，不属于 Stable。 |
| `os` | G20、G62、G63 | 主机信息、`hostname`、真实的 `networkInterfaces`。 |
| `querystring` | G38 | 固定 `&` 和 `=` 分隔符。 |
| `string_decoder` | G39、G68 | UTF-8，以及 latin1、ascii、base64、hex、utf16le。不支持 `base64url`。 |
| `assert` | G40、G69 | `ok`、`strictEqual`、`deepStrictEqual`、`throws`、`match`、`rejects` 等。宽松的 `equal` 语义未固定。 |
| `timers`、`timers/promises` | G42、G50 | 与全局定时器同一函数；支持 `AbortSignal` 的 Promise 版 `setTimeout` 和 `setImmediate`。 |
| `tty.isatty` | G49 | 目前在 Windows 上始终为 `false`。 |
| `vm` | G55、G66（tentative） | `runInThisContext` 与 `runInNewContext`，不是完整的 Contextify。 |
| `vm.createContext` | G73 | `createContext` / `isContext` 标记，以及 `runInContext`。不支持持久复用同一上下文。 |
| `diagnostics_channel` | G56（tentative） | `channel` 发布订阅和 `tracingChannel.traceSync`。 |
| `async_hooks` | G57（tentative） | `AsyncResource` 和 `AsyncLocalStorage`，没有 `createHook`。 |
| `readline` | G46（tentative） | 仅在宿主 stdin/stdout 上支持 `question()`。 |

## 加密与压缩

| API | 编号 | 说明 |
| :--- | :--- | :--- |
| `crypto` 哈希与随机数 | G23 | `createHash`、`createHmac`（md5、sha1、sha256、sha384、sha512、blake3）、`randomBytes`、`randomUUID`、`timingSafeEqual`。返回 `Uint8Array`。 |
| `crypto` AES 加密 | G43、G51、G52 | AES-CBC、AES-GCM、AES-CTR 的 `createCipheriv` / `createDecipheriv`。 |
| `zlib` | G21、G48、G58 | 同步与回调版的 gzip、deflate、raw deflate。 |

KDF、签名与验签、密钥对、`KeyObject`、brotli 和流式 `createGzip` 都不是 Stable。

## 进程与网络

| API | 编号 | 说明 |
| :--- | :--- | :--- |
| `child_process` 同步 | G25 | `execSync`、`spawnSync`。`pid` 为 `0`。 |
| `child_process` 异步 | G47、G60、G67 | `exec`、`execFile`、捕获输出的 `spawn`，以及使用 JSON IPC 的 `fork`。没有实时 stdio 管道。 |
| `http` 服务器 | G37（tentative） | `createServer`、`listen`、缓冲响应、keep-alive。 |
| `http` 客户端 | G72 | `request` / `get`，一次缓冲的 HTTP/1.1 响应体。没有 `https`、Agent，也不做分块流式传输。 |
| `perf_hooks` | G74 | `performance`、`PerformanceObserver`、`timerify`。 |
| `worker_threads` | G75 | 真实操作系统线程，JSON `postMessage`。没有 `transferList`，也不支持嵌套 worker。 |
| `net` 服务端与客户端 | G45、G61（tentative） | `createServer` 监听与接受连接；`net.connect` 客户端。 |
| `dns` | G27、G65 | 基于系统解析器的 `lookup`、`resolve*`、`reverse`、`getServers`。 |
| `url` | G26 | POSIX 的 `fileURLToPath` 和 `pathToFileURL`。旧版 `parse` 不在范围内。 |
| `stream` | G24、G71 | `Readable`、`Writable`、`PassThrough`、`Transform`，以及回调形式的 `pipeline`。没有 `Duplex` 或 `stream/promises`。 |

## 不支持

以下能力会失败或 reject，而不是假装可用：

- `http.Agent`、`https` 模块和 `http2`。
- 流式 `zlib.createGzip` 系列、brotli 和 `zlib.constants`。
- 除 Experimental 的 N-API hello 加载器之外的 Node 原生插件。

## 一致性

`tests/conformance/` 中的 Node 一致性记分卡在 main 上为 56/56。这个数字的含义与局限见 [稳定性分级](/docs/stability)。

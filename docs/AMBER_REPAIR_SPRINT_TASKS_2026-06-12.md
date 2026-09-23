# Amber 修复冲刺任务拆解

日期：2026-06-12  
来源：[`AMBER_LANGUAGE_DESIGN_REVIEW_2026-06-12.md`](AMBER_LANGUAGE_DESIGN_REVIEW_2026-06-12.md)  
目标：把设计审视报告拆成可执行、可验证、可并行领取的工程任务，并启动第一轮 P0/P1 修复。

## 执行原则

- 每个任务必须是可验证的竖切片，不只改一层代码。
- 行为变更必须先写失败测试，再实现。
- 每个并行 worker 拥有清晰文件边界，避免互相覆盖。
- 大型架构项先落最小闭环，再逐步替换旧路径。
- 未闭合能力要明确失败或标为实验，不返回假成功。

## 第一波已启动任务

### T1. CLI 和测试入口止血

类型：AFK  
优先级：P0  
文件边界：

- `src/main.rs`
- `tests/*cli*_tests.rs` 或新增 `tests/cli_regression_tests.rs`
- 如必须，少量触及 `src/runtime_minimal.rs` 的 argv 注入点

目标：

- 修复 `amber create --help` 相关 clap 参数顺序/默认值风险。
- 为 `amber test <file>` 假阳性建立回归测试，并让明显失败脚本至少返回非零退出或明确失败。
- 确认 `run` 子命令的 script args 不再只被 verbose 打印，至少能进入 `process.argv` 或被记录为未支持。

验收：

- `amber create --help` 不 panic。
- 一个包含失败断言/抛错的测试文件不会被报告为成功。
- 新增测试先红后绿。

### T2. 包管理供应链最小安全闭环

类型：AFK  
优先级：P0  
文件边界：

- `src/package_manager.rs`
- `tests/package_manager*_tests.rs` 或新增 `tests/package_manager_security_tests.rs`

目标：

- tarball 解包拒绝绝对路径、`..`、特殊文件、逃逸 symlink/hardlink。
- manifest struct 支持 npm 驼峰字段：`devDependencies`、`peerDependencies`、`optionalDependencies`。
- 为 integrity mismatch 设计最小可测试路径；如果完整 SRI 暂时过大，先保证 lock/cache 不把 integrity 写成可信成功。

验收：

- 恶意 tgz 不会写出目标包目录。
- `devDependencies` 等字段能被解析。
- 新增测试先红后绿。

### T3. AsyncTimerManager 事件顺序和丢任务语义

类型：AFK  
优先级：P1  
文件边界：

- `src/event_loop.rs`
- `tests/event_loop*_tests.rs` 或新增 `tests/event_loop_timer_tests.rs`

目标：

- timer 执行顺序按 deadline + insertion order。
- 队列满或调度失败不应静默成功。
- cancel race 有可观察结果。

验收：

- `setTimeout` 等价 timer 的 10ms 在 50ms 前触发。
- 同 deadline timer 按插入顺序触发。
- 超过容量或 channel 关闭时返回错误。

### T4. 当前能力事实源

类型：AFK  
优先级：P0  
文件边界：

- `docs/CURRENT_SCOPE.md`
- `README.md`
- `docs/QUICK_START.md`
- 可新增 `docs/README.md`

目标：

- 建立 Stable / Preview / Experimental / Historical 能力分层。
- README 和 Quick Start 不再把 stage 报告当作当前产品能力证明。
- 说明性能数字和 feature gate 的当前事实口径。

验收：

- 用户能从 README 找到当前真实能力边界。
- Quick Start 优先从源码构建，release 安装注明条件。
- Stage 文档被明确标为历史资料。

### T5. `execute_code` 结果语义止血

类型：AFK  
优先级：P0  
文件边界：

- `src/runtime_minimal.rs`
- `tests/*runtime*_tests.rs` 或新增 `tests/runtime_eval_semantics_tests.rs`

目标：

- 删除或绕开“重新执行最后表达式”的副作用风险。
- `eval`/`execute_code` 的返回值来自主脚本执行结果，不再次执行用户代码。

验收：

- `let i = 0; i++; i` 不会因为取结果重复递增。
- 带副作用的最后表达式只执行一次。
- 新增测试先红后绿。

### T6. TypeScript source map/诊断最小修复

类型：AFK  
优先级：P1  
文件边界：

- `src/typescript/compiler.rs`
- `src/typescript/mod.rs`
- `tests/*typescript*_tests.rs`

目标：

- 修复明显的 source map 行映射 bug。
- diagnostics 至少能聚合到返回值，而不是只 `eprintln`。
- 对未支持 TSX/复杂语法给出明确诊断，不生成看似成功的坏 JS。

验收：

- source map line mapping 有回归测试。
- 编译错误能被调用方读取。
- 未支持语法有明确错误。

## 第二波候选任务

### T7. PermissionState/ResourceBroker 最小原型

阻塞：T1/T5 之后更稳  
建议边界：新增 `src/permissions.rs`，小范围接入 `fs` 或 `process.env` 一个入口。

### T8. CommonJS resolver 最小闭环

阻塞：T5/T6 之后更稳  
建议边界：新增 resolver helper，先支持 parent-relative require、directory index、package main。

### T9. Fetch fail-open 清理

阻塞：事件循环策略未必需要，但要避免和权限 broker 冲突  
建议边界：`src/web_api/fetch.rs`，先让网络错误 reject/throw，不返回 fake 200。

已完成子项：真实 HTTP/fail-closed 路径已覆盖；Response Body mixin 现在会维护 `bodyUsed`，并禁止同一响应体重复读取或在读取后 `clone()`；`response.headers` 现在会复用 `Headers` 对象方法，并能通过 `get/has` 读取真实响应头；`new Response(body, init)` 改为 body-first 标准参数顺序，并支持 init status/statusText/headers 与同一套 Body mixin；`Headers` 已支持标准遍历入口。

### T10. Crypto 假成功禁用

阻塞：无  
建议边界：`src/web_api/crypto.rs`、`src/nodejs_core/crypto.rs`，先禁用固定 true 和明文 fallback。

### T11. 测试隔离清单

阻塞：无  
建议边界：`tests/QUARANTINE.md`，登记 `.disabled/.bak/legacy`。

### T12. 性能基准可信化

阻塞：无  
建议边界：`benchmarks/`，先让 benchmark 校验退出码和 stdout，不吞错。

## 工作流

1. 每个 worker 只改自己的文件边界。
2. worker 返回前必须列出：改动文件、测试命令、红绿验证结果、未完成项。
3. 主线程合并后运行目标测试，再决定是否进入下一波。
4. 如果某任务发现需要改共享运行时内核，先停下并缩小为单独架构任务。

## 第二波执行结果

更新时间：2026-06-12

### 已完成

- T1 file-mode test harness：`amber test <file>` 已接入 `--test-name-pattern`、`--test-skip`、`--bail`、显式 `--timeout` 和 `jest.setTimeout(ms)`；`--parallel` 在 single-file 模式明确提示串行运行；pending timer Promise 不再被当作通过；声明 `done` 参数的测试会等待 `done()`，`done(error)` 会失败，done callback 与返回 Promise 混用会 fail closed；测试超时处理改为线程 + `recv_timeout`，长阻塞闭包会在 timeout 到达时返回 `Exceeded`，不再等闭包执行完后才事后判定超时。
- T7 PermissionState/ResourceBroker 最小原型：新增默认 allow-all 的 `permissions` 模块，支持按 kind/action/resource deny/allow；支持 deny-all 后 exact allow 例外；路径资源在写入规则和检查时会做规范化；`nodejs_core::fs` 的同步文件/目录/metadata/delete/rename/rmdir、callback 风格 `readFile`/`writeFile`/`appendFile`，以及对应 `fs.promises` 入口都接入全局 broker；`require("fs")`、`require("fs/promises")` 复用全局 fs 绑定，绑定缺失时 fail closed，不再落入未接 broker 的 legacy fallback。
- T8 CommonJS resolver 最小闭环：新增 `nodejs_core::commonjs_resolver`，支持 builtin、相对/绝对文件、`.`/`..`、`.js/.json` 扩展、目录 `index`、`package.json#main`、`node_modules` 向上查找；`runtime_minimal` 的文件 require 与 `require.resolve()` 已接入；`amber run <file>` 会用真实脚本路径设置主模块 `__dirname/__filename`；模块 wrapper 内提供捕获当前模块目录的局部 `require`，避免模块修改全局 `__dirname` 后破坏相对 require。
- T3 timer 调度错误传播：`schedule_timeout`、`schedule_interval` 返回 `Result<(), TimerScheduleError>`；Node timers 生产路径改用 `try_*`，失败时清理 metadata/callback 并向 JS throw。
- T6 TypeScript 第二波：source map source-line 字段改为 relative delta；未支持 TSX/JSX 的 closing、自闭合、fragment/element 起始形态 fail-fast 并返回明确诊断；CLI `.ts/.tsx` 入口在收到 Error 级 diagnostics 时会在执行 JS 前失败，不再只打印错误后继续运行转译产物；CommonJS `require("./typed.ts")` 会复用正式 TypeScript compiler 编译模块源码，并在编译前保护 `module.exports = ...` 语句，避免 enum 等 compiler 已支持语法被运行时 heuristic 转译删坏。
- T9 fetch fail-open 清理：移除 httpbin/offline fallback response；网络、client、body 读取错误保持错误语义，HTTP 4xx/5xx 保留真实状态；修复 `Content-Type` 大小写检测，避免显式 `Content-Type` 被默认 `text/plain;charset=UTF-8` 覆盖；`http_fetch_tests` 已迁到本地一次性 HTTP fixture，不再依赖 httpbin。
- T10 crypto 假成功禁用：RSA generateKey/sign 不再生成随机 placeholder；RSA verify 保持 fail-closed；ECDSA sign key-data/parsing/signing fallback 不再生成确定性假签名；AES-CBC/未知 encrypt/decrypt 不再返回 IV+明文或 passthrough；后续竖切片已补齐真实 AES-CBC/AES-CTR 运算路径。
- nextTick/order 测试语义对齐：`nodejs_api_tests`、`process_module_tests`、`process_next_tick_tests`、`next_tick_order_test`、`next_tick_timer_order_enhanced_test` 已改为先断言主脚本 completion，再二次读取全局状态验证 nextTick/timer drain；相关全量测试通过。
- PermissionState/ResourceBroker CLI 最小沙箱：`amber run`、`amber eval`、`amber test` 支持 `--deny-fs`、`--allow-read <PATH>`、`--allow-write <PATH>`；脚本内 `fs` 读写和 CommonJS `require("./module")` 文件读取都会经过 broker；默认仍保持 allow-all 兼容模式。
- PermissionState/ResourceBroker 网络 CLI 表达：`amber run`、`amber eval`、`amber test` 支持 `--deny-net`、可重复的 `--allow-net <HOST_OR_URL>` 和可重复的 `--allow-listen <HOST_OR_URL>`；`--deny-net` 同时拒绝 `Network/Connect` 与 `Network/Listen`，`--allow-net` 只恢复 outbound connect，监听端口必须显式通过 `--allow-listen` 恢复；host allow 会匹配对应 URL host，exact URL allow 只放行完全相同 URL；默认仍保持 allow-all 兼容模式。
- PermissionState/ResourceBroker env/run CLI 表达：`amber run`、`amber eval`、`amber test` 支持 `--deny-env`、`--allow-env <NAME>`、`--deny-run`、`--allow-run <COMMAND>`；`process.env` 和 `child_process.exec/spawn/execFile` 会按 CLI 策略进入同一个 broker。
- PermissionState/ResourceBroker policy 文件策略：`amber run`、`amber eval`、`amber test` 支持 `--permission-policy <PATH>` / `--policy <PATH>` 加载 JSON 权限文件；策略覆盖 `deny_fs`、`allow_read/write`、`deny_net`、`allow_net`、`allow_listen`、`deny_env`、`allow_env`、`deny_run`、`allow_run`，其中相对文件路径按 policy 文件所在目录解析。
- `amber test` 无文件模式假绿修复：当前目录存在 `.test.js/.spec.js/.test.ts/.spec.ts/.mjs` 时会先发现并串行执行项目测试文件；只有未发现项目测试时才回退到内置 smoke tests。
- `fs.promises` thenable 路径篡改防护：`readFile`、`writeFile`、`mkdir`、`readdir`、`stat`、`unlink`、`rename` 在实际 IO 前会重新检查最终路径权限，避免脚本创建 thenable 后修改 `__path/__oldPath/__newPath` 绕过 broker。
- CommonJS package root `exports` 字符串：`package.json#exports` 为字符串时优先于 `main`，并限制目标必须以 `./` 开头，且不能包含 `.`, `..` 或 `node_modules` 路径段；`require("pkg")` 和 resolver 测试均覆盖该行为。
- PermissionState/ResourceBroker 环境变量竖切片：`process.env` 初始化时会过滤 broker deny 的 `Environment/Read/Name(key)`，不再把被拒绝的宿主环境变量复制进 JS 对象。
- CommonJS resolver `package.json` 读取接入 broker：解析 package entry 前会检查 `FileSystem/Read`，permission denied 会传播到 `require()`，不再被吞成 fallback 或继续加载 `main`。
- WebCrypto digest 字符串算法修复：`crypto.subtle.digest("SHA-512" | "SHA-384" | "SHA-1", ...)` 和 `{ name: "SHA-384" }` 不再默认 SHA-256；SHA-1/SHA-384/SHA-512 有真实实现，未知字符串算法 fail closed。
- Fetch 网络权限竖切片：`fetch()` 在每次请求当前 URL 前检查 `Network/Connect` broker，包含 redirect 后的新 URL；deny 时在发起 reqwest 请求前 fail closed。
- WebSocket/net/DNS 网络权限竖切片：`MinimalRuntime` 内联 `WebSocket` 构造器、真实 `web_api::websocket` 构造器、`net.connect/createConnection`、`dns.lookup/resolve/resolve4/resolve6/reverse` 均在触达连接或解析器前检查 `Network/Connect` broker；deny 时不再继续创建连接或执行 resolver I/O。
- Node HTTP 网络权限竖切片：`http.request(...).end()` 在连接池和 TCP 请求前检查 `Network/Connect` broker；`http.createServer().listen()` 在创建 server state、标记 `listening` 或后台 `TcpListener::bind` 前检查监听地址的 `Network/Listen` broker；deny 时抛出 `permission denied` 且不会进入 listening 状态；`--deny-net --allow-net host` 不再隐式允许 bind/listen，需显式 `--allow-listen host`；server 已监听但 dispatcher/channel 缺失时返回 `503 Service Unavailable` 并关闭连接，不再返回 fake `200 OK` / `Handler: not configured`；消息通道响应会按状态码生成 reason phrase，例如 `404 Not Found`，不再输出 `404 OK`。
- child_process 进程权限竖切片：`child_process.exec/spawn/execFile` 在返回 ChildProcess 对象前检查 `Process/Execute` broker；deny 时抛出 `permission denied`，不再返回看似执行成功的占位对象。
- `process.chdir` 进程状态权限：`process.chdir(path)` 在调用 `set_current_dir` 前检查 `Process/Execute/Path(path)`，deny 时抛出 `permission denied` 且不改变宿主 cwd。
- PackageManager 权限竖切片：cache/node_modules 创建前检查 `FileSystem/Write`；`package.json` 读写检查 `FileSystem/Read|Write`；registry metadata 和 tarball 下载前检查 `Network/Connect` 与 `Process/Execute/Name("curl")`；tarball 读取、cache/package/extract 目标写入均接入 broker。
- PackageManager lockfile 权限竖切片：`read_package_lock`、`generate_package_lock`、`update_package_lock` 的 lockfile 读写接入 `FileSystem/Read|Write`；`read_package_lock()` 优先读取项目根 `package-lock.json`，仅在根 lock 缺失时兼容旧的 `node_modules/package-lock.json`；生成 lock 时扫描 `node_modules` 和已安装包 `package.json` 前也会检查读取权限。
- PackageManager cleanup/prune 权限竖切片：`get_installed_packages` 扫描 `node_modules`、`scan_installed_package` 扫描嵌套依赖、`clean_cache` 删除/重建 cache、`prune` 扫描与删除普通/scoped package 前均接入 `FileSystem/Read|Write` broker；`amber prune --deny-fs` 会在判断 `node_modules` 是否存在前检查目录读取权限，不再把被拒绝读取的缺失目录报告成成功 noop；deny 时在实际 I/O 前 fail closed。
- CommonJS subpath exports 最小闭环：支持 `exports` object 中的字符串子路径映射，例如 `pkg/feature` 和 `@scope/pkg/feature` 解析到 `exports["./feature"]` 指向的文件；未导出 subpath 不再深层 fallback。
- CommonJS conditional exports 顺序语义：`exports` 条件对象按 package.json 字段顺序解析，只匹配 CommonJS 可用的 `require`、`node`、`default` 条件；`default` 写在前面会先命中 default，`require` 写在 `node` 前面也会保留作者顺序。
- CommonJS pattern exports 最小闭环：支持单星号 subpath pattern，例如 `exports["./features/*"] = "./dist/features/*.js"`，exact key 优先，pattern target 会复用现有包根逃逸校验；runtime `require("pkg/features/button")` 复用 resolver 路径。
- CommonJS 未导出 subpath 错误语义：package 存在 `exports` 时，未声明的 `pkg/private` 不再降级成普通 module-not-found；resolver 和 runtime 均返回 `ERR_PACKAGE_PATH_NOT_EXPORTED`，即使磁盘上存在对应私有文件也不会绕过 exports。
- CommonJS exports array fallback / null blocking：`exports` 目标支持数组 fallback，会跳过缺失目标并解析后续有效目标；`Invalid Package Target` 可继续尝试后续数组项；显式 `null` 或空数组会终止 fallback 并返回 `ERR_PACKAGE_PATH_NOT_EXPORTED`，不再回落到 `main`、私有文件、后续条件或上层 `node_modules`；`exports` 只声明子路径时，包根同样不再回落到 `main`。
- CommonJS package self-reference：包内 `require("pkg")` 与 scoped `require("@scope/pkg/feature")` 会先按最近同名 `package.json#exports` 解析自身入口/子路径，runtime 局部 `require()` 也覆盖该路径；未声明 `exports` 的包仍保持原有 `node_modules` 查找行为。
- CommonJS package imports exact 映射：包内 `require("#alias")` 会按最近 package scope 的 `package.json#imports` exact key 解析到本包内 `./` target，并复用 CommonJS 条件/target 校验；未定义 alias 返回 `ERR_PACKAGE_IMPORT_NOT_DEFINED`，裸 `#` 返回 `ERR_INVALID_MODULE_SPECIFIER`。
- CommonJS package imports pattern 映射：包内 `require("#features/button")` 支持 `package.json#imports` 单星号 pattern，例如 `"#features/*": "./src/features/*.js"`；pattern 按 Node base length / key length 选择最具体匹配，捕获段含 `.`, `..`、空段或 `node_modules` 时返回 `ERR_INVALID_MODULE_SPECIFIER`，specifier 以 `/` 结尾也会 fail closed。
- CommonJS package imports external target：`package.json#imports` 支持指向外部 package specifier，例如 `"#dep": "dep"` 与 `"#dep/*": "dep/*"`；外部 target 会从当前 package scope 走正常 CommonJS package/node_modules/exports 解析，`../`、绝对路径和 URL target 继续返回 `ERR_INVALID_PACKAGE_TARGET`。
- ESM 入口 static import 兼容层：入口代码中的 `import default from`、`import { named as alias } from`、`import * as ns from` 和 side-effect `import "mod"` 会转换成带正确绑定的 CommonJS `require()`，可从 `.mjs`/ESM 风格入口加载现有 CommonJS 模块；当目标被判定为原生 ESM 时会改走 V8 Module graph。
- ESM `.mjs/.js type:module` 最小 V8 Module graph：`.mjs` 入口静态导入相对/绝对 `.mjs` 源文本模块，或 `package.json` 声明 `type:"module"` 的 `.js` 入口/依赖时，会走 `v8::Module` 编译、实例化和求值；bare/package specifier 会复用 package/node_modules/`exports` 路径解析，并按 ESM `import`、`node`、`default` 条件顺序选择目标，目标是 `.mjs` 或 `type:"module"` `.js` 时进入 native ESM；模块依赖读取接入 `ResourceBroker`，并由 V8 保持 `export let` / imported binding 的 live binding 语义；依赖模块会缓存在 `MinimalRuntime` 实例内，跨多次 `execute_code` 复用并避免重复求值；native ESM 导入普通 CommonJS `.js/.json/.ts` 等非 ESM 文件时会通过 SyntheticModule 复用现有 `require(absolute_path)`，暴露 `default`/`namespace.default` 和 `module.exports` 自有属性的命名导出快照。
- ESM top-level await 最小闭环：`.mjs` 入口和 `type:"module"` `.js` 入口即使没有 import/export，只要包含 `await` 也会进入 V8 Module graph；同步可结算的 `await Promise.resolve(...)` 会在运行时 microtask drain 中完成；依赖模块中的 `export const value = await ...` 也能被静态 import 正确观察；入口模块遇到未结算的 pending top-level await 会 fail closed，不再把 V8 evaluation Promise 字符串化成 `[object Promise]` 假成功。
- ESM top-level await timer settle：native ESM evaluation 在 Promise pending 时会在 `timer_drain_limit_ms` 窗口内推进 nextTick、microtask 和 fired timer callbacks，因此 `await new Promise(resolve => setTimeout(resolve, ...))` 这类短生命周期 timer-backed TLA 可以完成；没有可 drain 宿主任务或超过窗口的 pending TLA 仍 fail closed。
- ESM dynamic import 最小闭环：`MinimalRuntime` 会在 isolate 上安装 V8 HostImportModuleDynamically callback；native ESM 中 `await import("./dep.mjs")` 会按 referrer 资源路径解析相对 `.mjs`，复用现有 V8 Module graph、权限读取、module cache 和同步可结算 TLA 等待，最终 resolve module namespace 对象；普通 script/CommonJS 风格入口也会给 V8 script 设置主模块 `ScriptOrigin`，因此 `import("./dep.mjs")` 可相对当前 main module path 解析并在 Promise microtask 中 resolve；库层 `Runtime::execute_file(path)` 会把当前文件路径同步给持久 `MinimalRuntime`，所以通过 library API 运行 `.js` 文件时相对 dynamic import 也不再落到默认 `/workspace/script.js`；同一模块连续 dynamic import 会复用同一 namespace 且避免重复求值；dynamic import 普通 CommonJS `.js` 文件会通过 SyntheticModule 暴露 `default` 与命名导出快照，并把 synthetic namespace 写入同一文件级 module cache，因此连续 `import("./helper.js")` 不会返回两个不同 namespace；dynamic import 内建模块会把 `path` 与 `node:path` 等 specifier 归一到同一个 synthetic module cache key，连续 `import("path")` / `import("node:path")` 会返回同一 namespace；`file://` URL specifier 会转成本地文件路径并复用同一 module cache/权限读取语义，`https:` 等非 `file://` URL 会明确 reject，不再误走 package resolver；缺失传递依赖、依赖语法错误和依赖求值抛错都会 reject dynamic import Promise，其中求值抛出的 Error 会作为原始 rejection reason 传回 JS，而不是返回 V8 默认 `Not supported` 或让整个 `execute_code` 直接失败。
- ESM 内建模块 namespace 最小竖切片：native ESM 中 `import pathDefault, * as path from "path"` / `"node:path"` 会通过 V8 SyntheticModule 暴露 `default`、`join`、`resolve`、`basename`、`dirname`、`extname`、`normalize`，复用运行时已安装的全局 `path` 对象；`import fsDefault, { readFileSync } from "fs"` 同样走 SyntheticModule，暴露 `default` 和常用同步 FS API，并复用既有 `fs` broker 权限语义；`url` / `node:url` 暴露 `default`、`URL`、`URLSearchParams`，`events` / `node:events` 暴露 `default` 和 `EventEmitter`，其中 `events` default 贴合当前 `require("events")` 的 EventEmitter 构造函数形态；`os` / `node:os` 暴露 `default` 与当前全局 `os` 对象上的平台/内存/目录方法；`stream` / `node:stream` 暴露 `default`、`Readable`、`Writable`、`Transform`、`Duplex`、`pipeline`、`passThrough`，复用当前全局 `stream` 对象；`process` / `node:process` 暴露 `default` 与常用命名属性，并与 CommonJS `require("process")` / `require("node:process")` 共同指向同一个全局 `process` 对象，避免旧空 `env` stub 造成对象身份和 API 缺失；`crypto` / `node:crypto` 暴露 `default` 与 Node/WebCrypto 常用命名属性，并与 CommonJS `require("crypto")` 共同指向同一个全局 `crypto` 对象，保留 `subtle`、`getRandomValues`、`createHash`、`randomBytes` 等既有实现语义。
- Native ESM file-backed cache invalidation：`MinimalRuntime` 的持久 ESM dependency cache 记录源码 `blake3` fingerprint；每次 native ESM 执行前会在权限允许下重新读取已缓存文件并比对 fingerprint，任一缺失、拒绝、不可读或变更都会清空整张 file-backed ESM `Global<Module>` cache，避免 V8 旧 module record 保留旧 live binding；直接依赖和传递依赖修改后会重新编译/求值，同时未修改依赖仍保持跨执行缓存。
- CommonJS -> ESM namespace 桥接：`require("./file.mjs")` 与 `require("./type-module.js")` 会走同一 V8 Module graph，同步求值后返回 module namespace 对象；结果写入独立 ESM namespace cache，多次 `require()` 在文件图未变时返回同一 namespace；cache metadata 记录整张 ESM graph 的源码 `blake3` fingerprint，入口或传递依赖变更后会重新读取/求值，同一 package scope 下 `.cjs` 仍明确保持 CommonJS 可加载；同步 `require()` 遇到 pending top-level await 的 ESM 会抛错，不再返回半初始化 namespace 或写入 namespace cache。
- CommonJS TSX/JSX 加载策略：显式 `require("./file.tsx")` 在未出现 JSX 元素语法时复用轻量 TypeScript 转译路径；`.tsx/.jsx` 中出现 JSX element/self-closing element 会在执行前返回包含 `TSX/JSX unsupported` 的诊断，避免落入 V8 语法错误或执行文件副作用。
- CommonJS TypeScript 模块加载：resolver 支持 `.ts` 扩展；runtime `require("./typed")` 会读取 `typed.ts` 并用正式 TypeScript compiler 生成 JS，再进入 CommonJS wrapper 执行；CommonJS `module.exports = ...` 语句会在编译前临时保护并在编译后还原。
- CommonJS `node:` builtin 前缀：resolver 与 runtime `require()` 支持 `node:path` 等内建模块前缀，并规范化为对应 builtin；未知 `node:` specifier 不会落入用户包解析。
- CommonJS JSON module 语义：`.json` 文件不再被当作 JS wrapper 执行；runtime 会解析 JSON、递归转换成 V8 对象/数组并写入 CommonJS module cache，避免 JSON 文件触发编译 panic 或返回空 exports。
- WebCrypto AES-GCM IV fail-closed：`crypto.subtle.encrypt/decrypt` 不再在缺失或错误长度 IV 时退到全零 nonce；当前真实后端明确要求 12-byte IV。
- WebCrypto AES-CBC 真实 encrypt/decrypt 竖切片：`crypto.subtle.importKey('raw', ..., { name: 'AES-CBC' })` 导入的 128/192/256-bit key 可用 16-byte IV 完成 OpenSSL AES-CBC + PKCS#7 padding 加解密 round-trip；缺失或短 IV 继续 fail closed。
- WebCrypto AES-CTR 真实 encrypt/decrypt 竖切片：`crypto.subtle.importKey('raw', ..., { name: 'AES-CTR' })` 导入的 128/192/256-bit key 可用 16-byte `counter` 与 `length: 1..128` 完成 OpenSSL AES-CTR 加解密 round-trip；缺失 counter、非法 length 或计数器空间不足继续 fail closed。
- WebCrypto AES key length fail-closed：`crypto.subtle.generateKey({ name:'AES-*' })` 缺失 `length` 或使用非 128/192/256-bit length 会抛错；`importKey('raw', ..., { name:'AES-*' })` 只接受 16/24/32-byte key，不再把任意 raw 长度包装成 CryptoKey。
- WebCrypto AES-KW wrap/unwrap 竖切片：`crypto.subtle.wrapKey/unwrapKey(..., { name:'AES-KW' })` 使用 OpenSSL AES key wrap 真实包装 raw key material；256-bit AES-GCM key wrap 后长度为 40 bytes，篡改 wrapped bytes 会 fail closed；wrapping/unwrapping key 需匹配 AES-KW algorithm 与 usage。
- WebCrypto wrapKey IV fail-closed：`crypto.subtle.wrapKey(..., { name: "AES-GCM" })` 与短 IV 不再退到全零 nonce；`crypto.getRandomValues()` 也修正为返回传入的 TypedArray，避免调用方拿到 `crypto` 对象而掩盖 IV 读取错误。
- WebCrypto wrapKey JWK payload 语义：`crypto.subtle.wrapKey('jwk', ...)` 不再直接加密内部 raw key bytes，而是先导出标准 JWK JSON payload（`oct`/OKP、`alg`、`key_ops`、`ext`、base64url key material）再交给 AES-GCM/AES-KW 包装；non-extractable key 会在 wrap 前 fail closed；`unwrapKey('jwk', ...)` 会解析 JWK payload 并恢复 HMAC hash metadata，unwrapped HMAC key 可继续真实 `sign/verify`。
- WebCrypto 算法名 fail-closed：`importKey`、`generateKey`、`sign`、`verify` 不再在算法对象缺失 `name` 时默认成 HMAC；畸形算法会明确抛出 `algorithm.name is required`。
- WebCrypto key usages 与 key algorithm 校验：`importKey`/`generateKey` 会拒绝算法不允许的 usage；`sign`/`verify`/`encrypt`/`decrypt` 会校验 key usage 和 `key.algorithm.name`，不再允许 AES key 走 HMAC 或 AES-CBC key 走 AES-GCM。
- WebCrypto unwrapKey 标准 IV 语义：`unwrapKey` 不再忽略 `unwrapAlgorithm.iv` 或从 wrapped blob 前缀偷取 IV；缺失、短 IV、错误 IV 和 unsupported format 均 fail closed，`wrapKey` 返回值改为标准 ciphertext/tag。
- WebCrypto JWK AES-GCM export 字段语义：`crypto.subtle.exportKey('jwk', AES-GCM-256 key)` 会导出 `kty:'oct'`、base64url/no-padding 的 `k`、`alg:'A256GCM'`、`key_ops` 与 `ext:true`，不再把 AES-GCM JWK `alg` 简化成 `A256`。
- WebCrypto JWK HMAC import 竖切片：`crypto.subtle.importKey('jwk', { kty:'oct', k, alg:'HS256' }, { name:'HMAC', hash:'SHA-256' }, ...)` 支持 base64url/no-padding `k` 解码；导入后的 CryptoKey 可完成真实 HMAC sign/verify，并可 raw export 回原始 key bytes。
- WebCrypto JWK HMAC export 字段语义：`crypto.subtle.importKey('raw', ..., { name:'HMAC', hash:'SHA-512' }, true, ['sign','verify'])` 会在 CryptoKey 中保留 hash 元数据；`sign/verify` 使用对应 HMAC-SHA 算法，`exportKey('jwk')` 导出 `kty:'oct'`、base64url/no-padding 的 `k`、`alg:'HS512'`、`key_ops` 与 `ext:true`，不再把 SHA-384/SHA-512 HMAC key 退化成 SHA-256/HS256。
- WebCrypto JWK import 字段 fail-closed：`crypto.subtle.importKey('jwk', ...)` 现在会校验 oct JWK 的 `alg/key_ops/ext`；`alg` 存在时必须匹配请求算法与 key length/hash，`key_ops` 存在时必须覆盖请求 usages，`ext:false` 不能以 `extractable:true` 导入，不再把这些约束当作可忽略装饰字段。
- WebCrypto ECDSA/HMAC 假成功收口：ECDSA verify 缺 key data 不再按签名长度返回 true；HMAC sign/verify 缺 key data 不再使用全零 key fallback。
- WebCrypto CryptoKey key material opaque 化：内部 key bytes 不再写入 JS 可见的 `__amberjs_key_data__` own property，而是存入 V8 Private property；`Object.getOwnPropertyNames(key)` 与直接属性读取均看不到 key material；`sign/verify/encrypt/decrypt/exportKey/wrapKey/unwrapKey/deriveKey/deriveBits` 仍通过 Rust 侧 helper 访问真实 key data；`deriveKey/deriveBits` 不再信任脚本伪造的 `__amberjs_key_data__` 字符串属性。
- WebCrypto RSASSA-PKCS1-v1_5 真实签验竖切片：`crypto.subtle.generateKey({ name:'RSASSA-PKCS1-v1_5', modulusLength:2048, publicExponent:Uint8Array([1,0,1]), hash:'SHA-256' }, true, ['sign','verify'])` 会生成真实 OpenSSL RSA key pair；`subtle.sign/verify` 使用 PKCS#1 v1.5 + key hash 完成 true round-trip，篡改签名返回 `false`，fake key 缺失 key data 仍 fail closed。
- WebCrypto RSA-OAEP 真实加解密竖切片：`crypto.subtle.generateKey({ name:'RSA-OAEP', modulusLength, publicExponent:Uint8Array([1,0,1]), hash:'SHA-256' }, true, ['encrypt','decrypt'])` 会生成真实 OpenSSL RSA key pair；`subtle.encrypt/decrypt` 使用 EVP RSA-OAEP + key hash/MGF1 完成 ArrayBuffer round-trip；篡改 ciphertext 或 OAEP `label` 不匹配会 fail closed；相同 non-empty `label` 可正常解密，非 BufferSource `label`、private key encrypt、public key decrypt 均会拒绝，不再把 RSA-OAEP 留在 not-implemented 分支。
- Node crypto createSign/createVerify 真实 RSA 竖切片：`crypto.generateKeyPairSync('rsa')` 和 `generateKeyPair('rsa')` 生成 OpenSSL RSA PEM；`createSign().sign(privateKey, encoding)` 和 `createVerify().verify(publicKey, signature, encoding)` 使用 OpenSSL signer/verifier，缺失或无效 PEM 会抛错，不再产生或接受 `RSA-SIG-*` mock。
- Node crypto RSA encrypt/decrypt 真实 OpenSSL 竖切片：`publicEncrypt/privateDecrypt/privateEncrypt/publicDecrypt` 使用 OpenSSL RSA PEM 解析和真实 PKCS#1/OAEP padding；占位 PEM fail closed，不再按 PEM marker 返回“前 11 字节伪填充”密文；旧正向测试已迁移到 `generateKeyPairSync('rsa')` 生成的真实 key pair。
- Node crypto createECDH 真实 OpenSSL 竖切片：`createECDH('prime256v1'|'secp256r1'|'secp384r1'|'secp521r1')` 使用 OpenSSL EC key generation 和 `Deriver` 计算 shared secret；P-256 public key 为标准 65-byte uncompressed point；无效 peer public key fail closed；`setPrivateKey` 会按曲线重新推导 public key，不再保留 XOR/旋转 placeholder。
- WebCrypto ECDH 真实 OpenSSL 竖切片：`crypto.subtle.generateKey({ name: 'ECDH', namedCurve })` 生成真实 EC private scalar 与 uncompressed public point；`deriveBits/deriveKey` 使用 OpenSSL `Deriver` 计算 shared secret；伪造或短 public key fail closed，不再使用 deterministic XOR/position formula。
- WebCrypto ECDSA P-384/P-521 真实 OpenSSL 竖切片：`crypto.subtle.generateKey({ name: 'ECDSA', namedCurve })` 生成对应曲线的 private scalar 与 uncompressed public point；`sign/verify` 按调用 hash 生成/验证 WebCrypto raw `r||s` 签名；P-384/P-521 不再复用 P-256 signing backend。
- WebCrypto Ed25519/Ed448 EdDSA 竖切片：`crypto.subtle.generateKey({ name:'Ed25519'|'Ed448' }, true, ['sign','verify'])` 生成真实 OpenSSL OKP key pair；`subtle.sign/verify` 使用 no-digest EdDSA，Ed25519 返回 64-byte signature、Ed448 返回 114-byte signature，tampered data verify 返回 `false`，不再在 usage 校验或 unsupported algorithm 分支提前失败。
- WebCrypto EdDSA raw/JWK import/export 竖切片：`exportKey('raw', publicKey)` 返回 Ed25519 32-byte / Ed448 57-byte public key，private raw export fail closed；`exportKey('jwk', publicKey|privateKey)` 输出 Node 风格 OKP JWK（`kty/crv/alg/x[/d]/key_ops/ext`，base64url no-padding）；`importKey('raw'|'jwk', ..., { name:'Ed25519'|'Ed448' })` 可导入 public/private key 并参与 no-digest `subtle.sign/verify` round-trip。
- WebCrypto EdDSA SPKI/PKCS#8 import/export 竖切片：`exportKey('spki', publicKey)` 与 `exportKey('pkcs8', privateKey)` 使用 OpenSSL DER 编解码输出 Node 兼容 key material（Ed25519 为 44/48 bytes，Ed448 为 69/73 bytes）；public key 的 `pkcs8` 与 private key 的 `spki` export fail closed；`importKey('spki'|'pkcs8', ..., { name:'Ed25519'|'Ed448' })` 可导回 CryptoKey 并完成 no-digest sign/verify。
- Node crypto EC `generateKeyPair*` 真实 OpenSSL 竖切片：`generateKeyPairSync('ec')` 与 callback 风格 `generateKeyPair('ec')` 生成真实 EC PEM key pair；`createSign/createVerify` 支持 `SHA256/SHA384/SHA512` 等 hash 名并可对 EC key 完成 ECDSA sign/verify round-trip；EC 不再返回手写 PEM placeholder。
- Node crypto KeyObject PEM 解析竖切片：`createPrivateKey/createPublicKey` 使用 OpenSSL `PKey` 解析 PEM 并按真实 key id 标记 `asymmetricKeyType`；EC SPKI public key 不再被 `BEGIN PUBLIC KEY` 误判为 RSA；无效 private key fail closed，不再被包装成伪 RSA KeyObject。
- Node crypto KeyObject DER export 竖切片：`KeyObject#export({ type: 'pkcs8', format: 'der' })` 与 public `spki/der` 使用 OpenSSL DER 导出并返回 Buffer-like 二进制对象，不再把 PEM 字符串伪装成 DER/buffer。
- Node crypto KeyObject DER import 竖切片：`createPrivateKey({ key, type: 'pkcs8', format: 'der' })` 与 `createPublicKey({ key, type: 'spki', format: 'der' })` 使用 OpenSSL DER 解析并转为内部 PEM 存储；导入后的 KeyObject 可参与真实 sign/verify round-trip。
- Node crypto KeyObject encrypted PKCS#8 PEM import 竖切片：`createPrivateKey({ key, type: 'pkcs8', format: 'pem', passphrase })` 使用 OpenSSL passphrase 解析加密私钥并转为内部 PEM 存储；导入后的 KeyObject 可参与真实 sign/verify round-trip。
- Node crypto KeyObject encrypted PKCS#8 PEM export 竖切片：`privateKey.export({ type:'pkcs8', format:'pem', cipher:'aes-256-cbc', passphrase })` 使用 OpenSSL 输出加密 PKCS#8 PEM；导出的 PEM 可用 `createPrivateKey({ passphrase })` 导回并参与真实 sign/verify。
- Node crypto `generateKeyPair*` encrypted privateKeyEncoding 竖切片：`generateKeyPairSync` 与 callback 风格 `generateKeyPair` 在 `privateKeyEncoding: { type:'pkcs8', format:'pem', cipher:'aes-256-cbc', passphrase }` 下返回加密 PKCS#8 PEM；返回私钥可用 passphrase 导入并完成真实 sign/verify round-trip。
- Node crypto `generateKeyPair*` publicKeyEncoding DER 竖切片：`generateKeyPairSync` 与 callback 风格 `generateKeyPair` 在 `publicKeyEncoding: { type:'spki', format:'der' }` 下返回 Buffer-like SPKI DER；返回值可由 `createPublicKey({ type:'spki', format:'der' })` 导回并参与真实 sign/verify round-trip，默认/PEM 路径保持 PEM 字符串兼容。
- Node crypto `generateKeyPair*` privateKeyEncoding DER 竖切片：`generateKeyPairSync` 与 callback 风格 `generateKeyPair` 在 `privateKeyEncoding: { type:'pkcs8', format:'der' }` 下返回 Buffer-like PKCS#8 DER；返回值可由 `createPrivateKey({ type:'pkcs8', format:'der' })` 导回并参与真实 sign/verify round-trip，默认/PEM 与加密 PKCS#8 PEM 路径保持兼容。
- Node crypto RSA JWK generate/import 竖切片：`generateKeyPairSync` 与 callback 风格 `generateKeyPair` 在 `publicKeyEncoding/privateKeyEncoding: { format:'jwk' }` 下返回 RSA JWK 对象；public JWK 暴露 `kty/n/e`，private JWK 暴露 `kty/n/e/d/p/q/dp/dq/qi` 且字段为 unpadded base64url；`createPublicKey/createPrivateKey({ key:jwk, format:'jwk' })` 可导回 OpenSSL key 并参与真实 sign/verify round-trip。
- Node crypto EC JWK generate/import 竖切片：`generateKeyPairSync` 与 callback 风格 `generateKeyPair` 在 EC `publicKeyEncoding/privateKeyEncoding: { format:'jwk' }` 下返回 P-256/P-384/P-521 JWK 对象；public JWK 暴露 `kty/crv/x/y`，private JWK 补充 `d`，字段为定长 unpadded base64url；`createPublicKey/createPrivateKey({ key:jwk, format:'jwk' })` 可导回 OpenSSL key 并参与真实 ECDSA sign/verify round-trip。
- Node crypto KeyObject JWK export 竖切片：RSA/EC private/public `KeyObject#export({ format:'jwk' })` 复用真实 OpenSSL PEM -> JWK 转换；导出的 JWK 可再次通过 `createPrivateKey/createPublicKey({ key:jwk, format:'jwk' })` 导入并完成真实 sign/verify round-trip。
- Node crypto KeyObject public PKCS#1 PEM export/import 竖切片：RSA public `KeyObject#export({ type:'pkcs1', format:'pem' })` 会输出 `BEGIN RSA PUBLIC KEY`；`createPublicKey({ key: pkcs1Pem, type:'pkcs1', format:'pem' })` 会按 RSA PKCS#1 解析并规范化为内部 SPKI 存储，导入后的 KeyObject 可参与真实 sign/verify round-trip；`spki/pem` 继续输出 `BEGIN PUBLIC KEY`。
- Node crypto RSA public PKCS#1 DER / generateKeyPair encoding 竖切片：public `KeyObject#export({ type:'pkcs1', format:'der' })` 会输出 RSA PKCS#1 `RSAPublicKey` DER Buffer；`createPublicKey({ key: der, type:'pkcs1', format:'der' })` 会按 PKCS#1 DER 解析并规范化为内部 SPKI PEM；`generateKeyPairSync` 与 callback 风格 `generateKeyPair` 的 `publicKeyEncoding: { type:'pkcs1', format:'pem'|'der' }` 可返回 RSA PUBLIC KEY PEM 或 PKCS#1 DER，并能导回完成真实 sign/verify round-trip。
- Node crypto `createPublicKey` private-key derivation 竖切片：`createPublicKey(privatePem)`、`createPublicKey({ key: privatePem, format:'pem' })` 与 `createPublicKey(privateKeyObject)` 会从 RSA/EC private PEM 派生 SPKI public PEM，返回 `type:'public'` 的 KeyObject 并可参与真实 sign/verify round-trip；KeyObject 输入不再把 `type:'private'` 误当作 public encoding type，也不会把缺失的 `key` 属性字符串化为 `"undefined"`。
- Node crypto EC + PKCS#1 incompatible encoding 错误语义：EC public `KeyObject#export({ type:'pkcs1', format:'pem'|'der' })` 与 `generateKeyPairSync('ec', { publicKeyEncoding:{ type:'pkcs1', format:'pem'|'der' } })` 会抛 Node 风格 `Error`，`code:'ERR_CRYPTO_INCOMPATIBLE_KEY_OPTIONS'`，message 为 `The selected key encoding pkcs1 can only be used for RSA keys.`，不再只暴露 OpenSSL RSA 转换失败细节。
- Node crypto Ed25519 最小 EdDSA 竖切片：`generateKeyPairSync('ed25519')` 与 callback 风格 `generateKeyPair('ed25519', ..., cb)` 会生成真实 OpenSSL Ed25519 SPKI/PKCS#8 PEM；新增一次性 `crypto.sign(null, data, privateKey)` 与 `crypto.verify(null, data, publicKey, signature)` 走 OpenSSL no-digest EdDSA `sign_oneshot/verify_oneshot`，正确返回 64-byte signature、true round-trip 和 tampered data false，不再把 Ed25519 拒为 unsupported key type 或缺失顶层 `crypto.sign/verify`。
- Node crypto Ed448 EdDSA 竖切片：`generateKeyPairSync('ed448')` 与 callback 风格 `generateKeyPair('ed448', ..., cb)` 会生成真实 OpenSSL Ed448 SPKI/PKCS#8 PEM；`crypto.sign(null, data, privateKey)` 与 `crypto.verify(null, data, publicKey, signature)` 复用 no-digest EdDSA 路径，正确返回 114-byte signature、true round-trip 和 tampered data false；`createPrivateKey/createPublicKey` 会把 Ed448 KeyObject 标记为 `asymmetricKeyType:'ed448'`。
- ESM `crypto` signature named exports 对齐：native ESM `import { sign, verify, createSign, createVerify } from 'crypto'` 与 `node:crypto` 现在会从 synthetic builtin module 暴露同名导出，并与 `globalThis.crypto` / `require('crypto')` 指向同一函数对象，不再出现 CommonJS 可用但 ESM named import 缺失的分裂。
- Node crypto OKP JWK EdDSA 竖切片：`generateKeyPairSync('ed25519'|'ed448', { publicKeyEncoding:{format:'jwk'}, privateKeyEncoding:{format:'jwk'} })` 返回 Node 风格 `kty:'OKP'` JWK；Ed25519 `x/d` 为 43 chars，Ed448 为 76 chars；`createPrivateKey/createPublicKey({ key:jwk, format:'jwk' })` 和 `KeyObject#export({ format:'jwk' })` 可 round-trip 并参与 no-digest sign/verify。
- Node crypto RSA-PSS signing 竖切片：`crypto.constants.RSA_PKCS1_PSS_PADDING` 已暴露；`createSign/createVerify` 支持 `{ key, padding: RSA_PKCS1_PSS_PADDING, saltLength }` options 并使用 OpenSSL RSA-PSS padding/saltLength；PSS verify 不会接受默认 PKCS#1 signature，saltLength 不匹配返回 `false`。
- Node crypto `createCipheriv/createDecipheriv` 创建期 fail-closed：AES key 长度统一按 128/192/256-bit 校验；CBC/CTR/CFB/OFB 的 IV/counter 需要 16 bytes；`createDecipheriv` 不再接受错误 key/IV 创建出可用对象，AES-CTR 短 IV 也会在创建期抛错。
- Node crypto `setAutoPadding(false)` 语义竖切片：`createCipher/createDecipher/createCipheriv/createDecipheriv` 现在保存 `_autoPadding` 状态；`setAutoPadding()` 返回 `this` 并按参数切换 OpenSSL padding；禁用 padding 后整块 AES-CBC 加密不再追加 PKCS#7 padding block，非整块 final 会抛错，解密禁用 padding 时会保留原始 padding bytes。
- Node crypto Cipher split update 状态连续性：`cipher.update()` 不再为每个完整块新建 `Crypter` 并重置 CBC IV；拆分两次 `update()` 的 AES-128-CBC 拼接输出现在匹配 Node/OpenSSL 向量，`Decipheriv` 也覆盖了拆分 ciphertext 读取。
- Node crypto Cipher/Decipher final 生命周期错误分支：`createCipher/createDecipher/createCipheriv/createDecipheriv` 现在保存 `_finalCalled`；`final()` 后再次 `final()` 或 `update()` 会抛错，不再静默生成第二个 padding block 或重新累积数据。
- Node crypto AES-CTR streaming 输出时机：`createCipheriv/createDecipheriv('aes-128-ctr', ...)` 的 `update()` 现在会按 CTR keystream 位置即时返回本次输出，`final()` 返回空尾部；不再把所有 CTR 输入攒到 `final()` 才一次性输出。
- Node crypto AES-CBC streaming 输出时机：`createCipheriv/createDecipheriv('aes-128-cbc', ...)` 的 `update()` 现在按 OpenSSL/Node 分块语义返回已完成 block；加密整块输入会即时输出对应密文块，解密会保留最后一个 block 直到 padding 安全，`final()` 只返回尚未输出的尾部。
- Node crypto AES-CFB/OFB streaming 输出时机：`createCipheriv/createDecipheriv('aes-128-cfb'|'aes-128-ofb', ...)` 现在映射到 OpenSSL CFB128/OFB 后端；`update()` 对每个分片即时返回对应输出，`final()` 返回空尾部，并覆盖加解密分片 round-trip。
- Node crypto AES-GCM auth tag / AAD 竖切片：`createCipheriv/createDecipheriv('aes-128/192/256-gcm', key, iv)` 映射到 OpenSSL GCM 后端，12-byte IV 创建期校验；`setAAD()` 会把 AAD 纳入认证，`getAuthTag()` 在 encrypt `final()` 后返回真实 tag，`setAuthTag()` 会让 decrypt `final()` 校验 tag；错误 tag 会 fail closed，不再把 GCM 当 unsupported cipher 或无认证地解密。
- Node crypto AES 裸别名兼容：`createCipheriv/createDecipheriv('aes128'|'aes192'|'aes256', ...)` 现在按 Node/OpenSSL 兼容语义映射到对应 AES-CBC 算法；别名路径继续复用创建期 key/IV 长度校验，不会绕过 fail-closed。
- Node crypto `scryptSync/scrypt` 真实 scrypt 竖切片：原 PBKDF2-HMAC-SHA256 近似实现已替换为 OpenSSL `EVP_PBE_scrypt`；同步、Promise 和 callback 路径共用真实 memory-hard KDF，新增 RFC 7914 `N=16,r=1,p=1` 64-byte 向量验证；options 解析不会再把缺失字段或 callback 函数误转为 `0`。
- Node crypto `createHmac` 真实向量竖切片：`md5`、`sha1`、`sha256`、`sha384`、`sha512` 不再只做长度/形状验证；`sha1/sha384/sha512` 会把 chained `update()` 后的 data 喂入 HMAC，`md5/sha1/sha384/sha512` 共用标准 block-size HMAC 公式并支持空 key；base64 输出有精确向量覆盖，不再让错误 digest 因长度正确而假通过；HMAC key 支持 `Uint8Array` 原始 bytes 和 string key 的 `{ encoding }` 选项，不再把 typed key 字符串化或忽略 hex/base64/base64url key 编码。
- Node crypto digest/update byte 语义竖切片：`createHash().digest('latin1'|'binary')` 与 `createHmac().digest('latin1'|'binary')` 现在按每个 digest byte 映射到同值 Latin-1 code point；`binary` 不再误回 hex，`latin1` 不再通过 UTF-8 lossy 解码产生替换字符或错误长度；无 encoding 的 `digest()` 返回二进制 `Uint8Array`，不再默认为 hex 字符串；Hash 在 `digest()` 后二次 `digest()` 或 `update()` 会抛 `Digest already called`，HMAC 在 `digest()` 后 `update()` 会抛错且二次 encoded `digest()` 返回空字符串；`createHash().update(Uint8Array)` 与 `createHmac().update(Uint8Array)` 会按原始 bytes 计算，不再把 TypedArray 字符串化；`base64url` digest 输出与 `update(str, 'base64url')` 输入按 Node 风格 URL-safe/no-padding 字节语义处理；Hash/HMAC 算法名支持常见大小写与连字符别名，并补齐 SHA-384 真实 digest/HMAC；`Hash#copy()` 可复制当前 partial state 并独立继续 update/digest，digest 后 copy 会抛 `Digest already called`；`createHash()` 会在创建期拒绝 unsupported 算法，不再延迟到 `digest()` 才失败。
- Node crypto cipher 错误 code 兼容：`createCipheriv/createDecipheriv` 的 unsupported algorithm、invalid key length、invalid IV length 会在抛出的 Error 上设置 `ERR_CRYPTO_UNKNOWN_CIPHER`、`ERR_CRYPTO_INVALID_KEYLEN`、`ERR_CRYPTO_INVALID_IV`，不再只有 message 而缺少稳定 `code`。
- `process.env` 动态权限重检：`process.env` 从初始化快照改为 accessor，每次访问都会按当前 `Environment/Read/Name(key)` broker 状态重新构造可见环境对象。
- Watcher 权限竖切片：`amber run --watch` 会在输出 watch 启动提示、创建 watcher 或配置 WebSocket 前先检查目标脚本 `FileSystem/Read` 权限；`HotReloader::watch()` 在启动前检查被 watch 根路径的 `FileSystem/Read` 权限；初始 `WalkDir` 计数不再吞扫描错误，并会对每个可 watch 文件重新检查读取权限；deny 时不会启动后台 watcher 或留下 running 状态。`WebSocketHotReloader::start()` 在 bind 监听地址前检查 `Network/Listen`，监听地址按 `ws://host:port` 进入 host/URL broker 语义，避免 outbound `--allow-net` 例外误放行 hot reload listener。
- Benchmark runner 权限竖切片：默认构建中的 `testing::perf::BenchmarkRunner::save_results()` 在创建 report 文件前检查 `FileSystem/Write`，deny 时不会写出 benchmark report。
- Debug CLI 权限竖切片：`amber debug` 现在支持与 `run/eval/test` 相同的权限参数；读取调试目标文件前检查 `FileSystem/Read`，`--deny-fs` 下不会打印被拒绝脚本内容。
- Feature-gated benchmarks 编译/Clippy 恢复：`cargo check --features benchmarks` 与 `cargo clippy --features benchmarks -- -D warnings` 已覆盖 `src/benchmarks/*`、`performance_analyzer`、`performance_regression`、`performance_reporter` 等 feature-gated 模块；修复缺失 import、`Arc::clone` 借用、无锁计数器类型、rayon 并行迭代 trait、回归检测 serde 派生和 feature 内 warning。
- CommonJS `module` 字段策略与 exports target 校验：`require()` 明确忽略非 Node 标准的 package.json `module` 字段，继续按 `exports`/`main`/`index` 解析；package `exports` 字符串 target 必须以 `./` 开头，且后续路径段不能包含 `.`, `..` 或 `node_modules`；非法字符串 target 与非法 primitive `exports` 会返回 `ERR_INVALID_PACKAGE_TARGET`，不会被加载或回退到 `main`。
- CommonJS pattern exports 精度提升：pattern trailer 不再允许空 `*` 捕获，例如 `./features/*.js` 不会匹配 `./features/.js`；多 pattern 匹配按 Node 的 base length 优先、再按 pattern length 排序，不再被更长 trailer 误导。
- CommonJS pattern capture 错误语义：pattern `*` 捕获段包含空段、`.`、`..` 或 `node_modules` 时会返回 `ERR_INVALID_MODULE_SPECIFIER`，不再把调用方 specifier 错误误报成 package target 错误。
- CommonJS exports 配置错误语义：`package.json#exports` 对象同时混用 `"."` 开头的 subpath keys 与非 `"."` condition keys 时会返回 `ERR_INVALID_PACKAGE_CONFIG`，root、subpath 与 runtime `require()` 路径都会拒绝该配置。
- CommonJS malformed `package.json` fail-closed：package scope 中存在但 JSON 解析失败的 `package.json` 会返回 `ERR_INVALID_PACKAGE_CONFIG`，resolver 与 runtime `require()` 都不会把坏配置当作缺失配置继续回退到 `main`、`index.js` 或父级 package scope。
- `amber test` file-mode lifecycle hooks 竖切片：CLI wrapper 支持全局和 `describe` 作用域的 `beforeEach/afterEach/beforeAll/afterAll`；`beforeEach` 按外到内执行，`afterEach` 按内到外执行；hook 支持 Promise 和 `done` callback，`done(error)` 会让对应 hook 失败，done callback 与 Promise 混用会 fail closed；`beforeAll` 在对应 file/suite 首个可运行测试前只运行一次，失败后会阻断同 scope 后续测试 body、但不影响外层/无关测试继续运行；`afterAll` 在对应 file/suite 最后一个可运行测试或被阻断测试后按内到外运行；失败测试后仍会运行 captured `afterEach` 和已激活 scope 的 `afterAll`，避免清理逻辑被跳过。
- `amber test` file-mode `.each` 表驱动竖切片：`test.each`、`it.each`、`describe.each` 以及 `skip/only/failing/concurrent` 相关 `.each` 入口支持数组表格和 tagged-template 表格形式；数组 rows 支持 `%s/%i/%d/%f/%j/%p/%#` 标题插值，template rows 支持 `$column` 和 `$#` 标题插值并把 row object 传给 callback；row 参数会传给 callback，若 callback arity 超过 row 参数数则把 `done` 作为最后一个参数传入；失败行会报告展开后的测试名，避免迁移 Jest table tests 时直接 method-not-found。
- `amber test` file-mode core matcher 竖切片：`expect()` 支持 `.not`、`.resolves/.rejects`、`toBeDefined`、`toBeUndefined`、`toBeNull`、`toBeNaN`、`toContain`、`toContainEqual`、`toHaveLength`、`toMatch`、`toHaveProperty`、`toBeInstanceOf`、`toMatchObject`、`toStrictEqual`、`toBeGreaterThan`、`toBeLessThan`、`toBeGreaterThanOrEqual`、`toBeLessThanOrEqual`、`toBeCloseTo`，并复用既有 `toBe/toEqual/toBeTruthy/toBeFalsy/toThrow` 的错误传播；`expect.extend()` 可注册 Jest 风格 `{ pass, message }` 自定义 matcher，支持 `.not` 与 `this.isNot` 文案；`expect.assertions()` 和 `expect.hasAssertions()` 会在单测结束时校验实际 matcher 调用次数，避免遗漏异步断言时假绿；`expect.any/anything/objectContaining/arrayContaining/stringContaining/stringMatching/closeTo` 和 `expect.not.objectContaining/arrayContaining/stringContaining/stringMatching` 可用于 `toEqual`、`toHaveProperty`、`toContain` 和 mock call 参数断言；`toBe` / `toEqual` / `toStrictEqual` 的失败文案改为失败时惰性格式化，避免通过断言对带 getter 的对象产生副作用；`toEqual` / `toStrictEqual` 会按内容比较 `Map` key/value 与 `Set` members，不再把不同 `Map` / `Set` 通过 `JSON.stringify` 假等成 `{}`；`toStrictEqual` 会区分 prototype 和 array hole；`toThrow(expected)` 支持 string message、RegExp 和 Error constructor 匹配；`rejects.toThrow(expected)` 会按 rejection reason 匹配；property path、instance 和 partial object mismatch 会给出明确失败原因；negated matcher、数值比较、throw mismatch 和 Promise state mismatch 会报告明确失败原因，避免落入 `Cannot read property` 或 method-not-found 假失败。
- `amber test` `toHaveProperty` path 兼容：string property path 现在支持 bracket notation，例如 `items[0].id`、`matrix[1][0]` 和 quoted key `records["a.b"].value`；数组路径语义保持不变，迁移 Jest/lodash 风格属性断言时不再把 `items[0]` 当作字面属性名。
- `amber test` file-mode mock 竖切片：CLI wrapper 支持 `jest.fn()`、`jest.spyOn()`、`jest.spyOn(obj, key, "get" | "set")` accessor spy、`jest.replaceProperty()`、`jest.mock()`、`jest.doMock()`、`jest.setMock()`、`jest.requireActual()`、`jest.requireMock()`、`jest.unmock()`、`jest.dontMock()`、`mock.calls`、`mock.results`、`mock.contexts`、动态 `mock.lastCall`、`mockName/getMockName`、`getMockImplementation`、`mockClear`、`mockReset`、`mockRestore`、`mockImplementation(Once)`、`withImplementation`、`mockReturnValue(Once)`、`mockReturnThis`、`mockResolvedValue(Once)`、`mockRejectedValue(Once)`、`jest.isMockFunction`、`jest.clearAllMocks/resetAllMocks/restoreAllMocks`；`mock()` / `doMock()` 当前支持显式 factory 的 CommonJS 相对/绝对模块 mock，`setMock()` 支持直接注册 module exports，factory 或手动 exports 首次 `require()` / `requireMock()` 时按 module registry 缓存，helper 模块内部相对 `require()` 也会命中 mock，`requireActual()` 可绕过 mock 读取真实模块，`unmock()` / `dontMock()` 可删除显式 mock 注册并回到真实模块；replaced property 支持 `replaceValue()` / `restore()`，getter/setter accessor spy 会被 `mockRestore()` / `restoreAllMocks()` 恢复；同时新增 `toHaveBeenCalled`、`toHaveBeenCalledTimes`、`toHaveBeenCalledWith`、`toHaveBeenNthCalledWith`、`toHaveBeenLastCalledWith`、`toHaveReturned`、`toHaveReturnedTimes`、`toHaveReturnedWith`、`toHaveNthReturnedWith`、`toHaveLastReturnedWith` 及 `.not` 语义；Jest alias `toBeCalled*`、`nthCalledWith/lastCalledWith`、`toReturn*`、`nthReturnedWith/lastReturnedWith` 复用同一语义；mock matcher 失败会报告明确 call/return mismatch；`jest` 同步暴露到 `globalThis`，required helper module 可通过 `globalThis.jest.fn()` 创建 mock。
- `amber test` wrapper TypeScript 二次转译防护：测试文件在包裹前已按扩展名完成 TS 编译，包裹后的 JS harness 会带内部 sentinel 跳过 `MinimalRuntime` 运行期 TS 启发式，避免普通 JS 测试名中的 `" in "`、template literal 或 wrapper 内部 mock metadata 触发误转译。
- `amber test` file-mode `.mjs` 防 false-green：file-mode wrapper 以同目录内部 `.amber-test.cjs` 主模块路径执行，保持相对 `require()` 基准目录，同时避免原始 `.test.mjs` 或 `type:"module"` 测试文件把 wrapper 误送入 native ESM 路径；异步 `.mjs` 测试失败会正确让 CLI 非零退出。
- `amber test` file-mode 空文件防 false-green：显式执行的 test file 若 collection 后没有注册任何 `test/it`，会以 `No tests found in test file` 非零退出，不再报告 `0 passed, 0 failed, 0 skipped` 后 `Tests passed`。
- `amber test` file-mode todo/failing 竖切片：`test.todo(name)` 与 `it.todo(name)` 注册为 skipped/pending 风格测试，不要求 callback、不执行 body，summary 计入 skipped；`test.failing(name, fn)`、`it.failing(name, fn)` 和 `failing.each` 会在测试本体或断言失败时计为通过，若测试意外通过则 fail closed 并报告 `Expected failing test to fail`，避免迁移 Jest 文件时因 `test.todo` / `test.failing` 缺失直接报 method-not-found。
- `amber test` file-mode `jest.setTimeout(ms)` 竖切片：single-file wrapper 支持 Jest 风格毫秒级超时配置，Promise/done callback 测试会使用更新后的 timeout；无显式 CLI `--timeout` 时 runtime drain 会保留有界等待窗口，避免 JS 侧自定义 timeout 尚未结算就被运行时提前判为 pending。
- `amber test` file-mode `jest.resetModules()` / `jest.isolateModules(fn)` / `jest.isolateModulesAsync(fn)` 竖切片：single-file wrapper 可清空当前 runtime 的 CommonJS `require()` cache 与 ESM namespace cache，后续 `require("./helper")` 会重新求值模块；`isolateModules(fn)` 会在 callback 期间切换到临时 module cache，并在正常返回或抛错后恢复外层 cache；`isolateModulesAsync(fn)` 会在 Promise resolve/reject 后恢复外层 cache；三者返回 `jest` 以兼容链式/断言用法。
- `amber test` file-mode `test.concurrent` 串行兼容入口：`test.concurrent` / `it.concurrent`、`concurrent.each`、`concurrent.only`、`concurrent.skip` 复用现有 file-mode queue、filter、hook、timeout 与 matcher 语义；当前不承诺真实并行调度，但迁移 Jest concurrent 用例时不会再直接 method-not-found。
- `amber test` filter regex 语义：single-file wrapper 的 `--test-name-pattern`、`--test-only`、`--test-skip` 改为按正则匹配 test name 和 suite name；无文件 discovery 模式的 `TestFilter` 也使用正则匹配，避免同一 CLI flag 在两种测试路径下语义分裂。
- `amber test` file-mode `describe.skip/only` 竖切片：suite-level skip 会继续执行 collection callback 以计数 nested tests，但测试 body 与 hooks 不会运行；suite-level only 会聚焦 suite 内所有测试；`describe.skip` 内嵌的 `test.only` 不会污染整文件 only-mode。
- `amber test` snapshot 竖切片：`expect(value).toMatchSnapshot()` 会读取同目录 `__snapshots__/<test-file>.snap` 中的 Jest 风格 `exports[\`test name 1\`] = \`...\`;` 快照并按当前 test name + 序号比对；默认模式下已有快照匹配时通过，缺失或不匹配时非零失败并报告 snapshot key；`--update-snapshots` 会创建或更新缺失/不匹配的 `.snap` 文件；snapshot 读取和写入分别经过 `FileSystem/Read|Write` broker；`expect(value).toMatchInlineSnapshot(\`...\`)` 支持静态 inline snapshot 比对并在 mismatch 时报告 expected/received；`--update-snapshots` 也会为缺失或不匹配的 inline snapshot 写回测试源文件，源码写入经过 `FileSystem/Write` broker，不再因 matcher 缺失、缺快照或写权限不足而假绿。
- testing library runner fail-closed 边界：`ParallelExecutor` 与 `EnhancedRunner` 在尚未接入真实 V8 callback 执行前，会返回明确失败和 `not implemented` 错误，不再把未执行的 `TestCase` 记为通过；skip 测试仍保留原有跳过语义。
- testing library V8 matcher false-green 收口：`V8TestExecutor` 注入的 `toBe/toEqual/toBeTruthy/toBeFalsy/toContain/toHaveLength/toBeDefined/toBeNull/toThrow` 会在 mismatch 时抛出 matcher 命名的 `Error`，成功时返回 `true`；`toBe` 现在读取 `expect(actual)` 保存的 `_actual`，`toThrow` 会真实调用函数并通过 V8 `TryCatch` 判断是否抛出，不再把 matcher 返回 `false` 或硬编码 `true` 当作测试通过。
- Assertion truthiness 语义对齐：Rust `ExtendedMatcher::Truthy/Falsy` 与 `toBeTruthy/toBeFalsy` 改用 JS/Jest truthiness；空数组、空对象和非空字符串 `"false"` / `"0"` 都会被视为 truthy，不再按容器是否为空或字符串内容特殊判 falsy。
- `MinimalRuntime` TypeScript 启发式收窄：`type`、`keyof` 和 mapped type 检测改为匹配真实 type alias/类型上下文，普通 JS 的 `` `type ${string}` `` 模板串和 `"[P in keyof T]"` 字符串不会再被运行期 TypeScript fallback 误转译；同时保留带泛型 mapped type 与 template literal type alias 的 fallback 清理能力。
- CLI preload 文件路径语义：`amber run --preload ./setup.js app.js` 对文件型 preload 改为通过 CommonJS `require(abs_preload_path)` 加载，preload 内部 `require('./helper')` 以 preload 文件所在目录解析，不再误用 main 脚本目录；preload specifier 也改用 JSON 字符串编码，避免引号路径破坏生成的 require 调用。
- CommonJS `require.main` 入口身份：默认运行时会把入口 `module` 挂到全局 `require.main`，入口脚本中 `require.main === module` 为 true；CommonJS wrapper 中的局部 `require` 会透传同一 `main`，因此 preload/helper 模块中 `require.main` 仍指向入口模块而不是 helper 自身。
- CLI preload fail-closed：`amber run --preload <file> app.js` 在 preload 读取、权限检查或执行失败时会直接非零退出，不再只打印 warning 后继续执行主入口；`--deny-fs --allow-read app.js --preload denied.js app.js` 不会运行 `app.js`。
- CLI `create` 参数顺序兼容：当前 canonical 顺序明确为 `amber create <name> [template]`，文档示例改为 `amber create my-ts-app ts`；历史 `amber create ts my-ts-app` 模板优先顺序会被归一化为 TypeScript 项目名 `my-ts-app`，不再误创建名为 `ts` 的 JavaScript 项目。
- CommonJS builtin registry 对齐：resolver 已声明的 `performance` / `node:performance` 现在可由 runtime `require()` 返回全局 `performance` 对象；`require("performance")`、`require("node:performance")` 与全局对象保持同一引用，不再被 resolver 识别后落入 `Cannot load builtin module` 错误。
- Package-manager CLI `remove` 权限闭环：`amber remove` 现在支持与脚本命令相同的 `--permission-policy` / `--policy` 等权限参数；读取 `package.json` 前检查 `FileSystem/Read`，写回前检查 `FileSystem/Write`，拒绝写入时不会修改依赖清单。
- Project/package CLI 权限入口扩展：`amber init/create/add/install/prune/bunx/upgrade` 现在同样支持 `--deny-*`、`--allow-*` 与 `--permission-policy`；项目创建会在目录/文件写入前检查 `FileSystem/Write`，包管理 registry、`curl`、`package.json`、lockfile 和 `node_modules` 访问会进入同一 broker；`upgrade` 遇到权限拒绝会 fail closed，不再把权限错误吞成普通 registry fetch failure 后继续写回。
- Bundle CLI 权限闭环：`amber bundle` 现在支持同一套权限参数；读取 entry 前检查 `FileSystem/Read`，写出 bundle 与 source map 前检查 `FileSystem/Write`，拒绝时不会创建输出文件。
- Bundle CLI 本地静态 import 竖切片：`amber bundle entry.js --outfile dist/bundle.js` 会递归内联相对静态 `import` 和本地 `export ... from` re-export 依赖，并把基础 ESM `export` 声明改写为同文件声明；default import（如 `import label from "./dep.js"`）、named import alias（如 `import { message as label }`）和 namespace import（如 `import * as mod from "./dep.js"`）会生成本地绑定；静态 ESM 扫描会在同一行内安全拆分 `; import` / `; export` 语句，并能合并多行 import/export-list 声明，不再要求每个 import/export 独占一行，且 `import ...; console.log(...)` 这类同一行后续业务代码不会被 import 处理吞掉；无分号结尾的多行本地 `export { ... }` 会在确认后续不是 `from` re-export 后闭合，不会继续吞掉后面的模块副作用代码；default export 与 named export 的内部 binding 都按模块路径唯一化，多个 default dependency 不再重复声明同一个 `__amberjs_default_export`，多个本地依赖同时导出同名 `value` 时也不会重复声明或串用同一个绑定；`export { internal as message }` 形式的 export list 会映射到 named import 的本地变量，`export { default as message } from "./dep.js"` 会映射到目标模块 default binding，`export * from "./dep.js"` 会把非 default 导出透传给 namespace import；本地 static import 或 barrel `export { ... } from` 引用缺失的 named/default export 时会在 bundle 阶段 fail closed 并拒绝写出坏产物；`--minify` 会跳过 bundle 元数据整行注释并保留语句换行边界，避免 `// module:` 注释吞掉后续可执行代码；因此产物移动到输出目录后可通过 `amber run dist/bundle.js` 独立运行，不再出现 bundle 命令成功但运行时仍从输出目录查找源依赖、丢失 barrel re-export 传递依赖、重复 default/named binding、缺失 export/re-export 延迟到运行时才炸、多行 import 残留在产物中、多行 export list 吞掉后续代码、同一行多 export 产物语法错误、同一行 import 后业务代码丢失、minify 后无输出或 import 变量未定义/namespace 为空的假成功。
- PackageManager exact install 权限前置：`install_package_exact()` 会在 registry resolve/download/extract 前检查当前 `package.json` 的 `FileSystem/Read|Write` 权限，拒绝写入时不会访问 registry、提取 package 或改写 manifest。
- PackageManager install/add/upgrade 权限前置：`install_package()` 会在 resolve/download/cache 写入前检查 `node_modules`、目标父目录和目标 package 目录写权限；`amber add` / `amber install` 在创建 PackageManager、访问 registry 或安装依赖前预检 `package-lock.json` 读写；`install_dependencies()` 在 root lockfile 存在时会用 locked version/resolved/integrity 约束安装结果，registry metadata 或 tarball 与 lockfile 冲突时 fail closed 且不解包；`amber install --frozen-lockfile` 会在创建 PackageManager、访问 registry、创建 `node_modules` 或 `.amberjs_cache` 前校验 `package.json` 依赖均存在于 root lockfile 且版本满足声明，缺失或 mismatch 时 fail closed，校验通过后也不会重写 `package-lock.json`；`amber upgrade` 在创建 PackageManager、访问 registry 或安装依赖前预检 `package.json` 写权限和 lockfile 写权限，拒绝时不创建 `node_modules` 或缓存副作用。
- ResourceBroker 路径规范化收口：文件系统权限路径会先按当前工作目录绝对化，再做 canonical/父目录 canonical 归一；因此 policy/CLI 允许的绝对未来路径与运行时相对路径检查能命中同一规则，`.amberjs_cache`、`node_modules` 等创建前授权不会在创建前后漂移成不同资源。
- URL/SearchParams live binding：默认运行时的 `new URL()` 现在提供 live `searchParams`；`url.searchParams.set/append/delete/sort()` 会同步更新 `url.search` 与 `url.href`，直接设置 `url.search` 也会更新同一个 `searchParams` 对象，不再返回与 URL 脱钩的空对象。
- Worker API fail-closed 边界：`Worker` 构造器在真实 WorkerHost、独立 isolate、事件循环和 structured-clone 消息队列实现前会同步抛错；普通脚本 URL 与 `data:` URL 均返回 `Worker script execution is not supported yet`，缺失脚本 URL 返回明确参数错误，不再制造带 `postMessage` / `terminate` / `_workerId` 的同步对象壳或输出伪生命周期日志。
- ServiceWorker registration fail-closed 边界：`navigator.serviceWorker.register()` 保留 Promise 形态，但在真实 registration store、install/activate/fetch lifecycle、`waitUntil/respondWith` 调度实现前会 reject `ServiceWorker registration is not supported yet`；不再 resolve 带 `scope`、`installing`、`active`、`waiting` 的 registration 对象壳，scope option 也不会被伪装成已注册成功。
- CacheStorage fail-closed 边界：`caches.open(name)` 在没有真实 CacheStorage backend、Request/Response 持久化和匹配策略前会 reject `Cache API is not supported yet`，不再 resolve 带 `addAll/match/put/delete/keys` 的空 Cache 壳；`caches.keys/has/delete` 改为标准 Promise 形态并返回空状态。
- PushManager subscribe fail-closed 边界：`new PushManager().subscribe()` 保留 Promise 形态，但在真实推送服务、权限授权、subscription store 和密钥生成实现前会 reject `Push subscription is not supported yet`；不再 resolve 固定 `https://push.example.com/subscribe/abc123` endpoint 的 mock `PushSubscription`。
- PushSubscription constructor fail-closed 边界：`PushSubscription` 仍作为可发现的全局类型存在，但直接 `new PushSubscription()` 会抛出 `PushSubscription construction is not supported yet`；不再返回固定 endpoint、VAPID key、`getKey/toJSON/unsubscribe` 的 mock 订阅实例。
- PushSubscription prototype fail-closed 边界：`PushSubscription.prototype.getKey.call({})` 与 `toJSON.call({})` 在没有真实 subscription internal slot 前会抛出 `PushSubscription is not supported yet`，不再返回固定 p256dh/auth key 或 `push.example.com` endpoint；`unsubscribe.call({})` 返回 Promise 并 reject 同一错误，不再 resolve `true` 假退订成功。
- CLI 源码入口读取权限闭环：`read_and_compile_source()` 在读取 JS/TS/TSX 入口文件前检查 `FileSystem/Read`，因此 `amber run --deny-fs app.js` 与 `amber test --deny-fs app.test.js` 会在读取目标文件本身时 fail closed；需要执行脚本内部 FS 权限测试时必须显式 `--allow-read` 入口文件。
- `amber test` discovery 权限闭环：无显式文件时，`TestDiscoverer` 可注入目录读取检查；CLI 在扫描当前目录和递归子目录前先检查 `FileSystem/Read`，因此 `amber test --deny-fs` 不会绕过 broker 后回退执行内置 smoke tests。
- `bunx` 进程执行权限前置：`amber bunx --deny-run <pkg>` 会在创建 `node_modules` 或 registry 下载前按目标包名检查 `Process/Execute`，避免已禁止执行时仍产生安装副作用或只在内部 `curl` 调用处才失败。
- `serve` 网络权限入口：`amber serve` 现在支持同一套 `--deny-net`、`--allow-net`、`--allow-listen` 与 policy 参数；在报告 HTTP/HTTPS server configured 前会按 `http(s)://host:port` 检查 `Network/Listen`，拒绝时不会输出成功配置提示。
- Node `fs` Stats / promises 兼容：`fs.statSync()` 与 `fs.promises.stat()` 现在返回带 `isFile()` / `isDirectory()` 方法的 Stats 对象，文件和目录会按真实 metadata 返回；`fs.promises` 同步补齐 `appendFile`、`rmdir` 入口，并为 `readFile` thenable 提供 `.catch()` 错误回调形态。
- Node `child_process.exec/execFile/spawn` 假成功收口：默认 `MinimalRuntime` 与备用 `nodejs_core::child_process` 路径会同步执行命令并把真实 `stdout/stderr/exitCode` 写入返回对象；`exec/execFile` 提供 callback 时会以同一份输出同步调用 `(error, stdout, stderr)`，成功时 `error === null`，非零退出时传入带 `code` 的 Error；`spawn(...).on('exit'|'close', listener)` 会用返回对象上的真实 exitCode 和 null signal 同步触发最小事件回调；`exec("printf ...")`、`execFile("/bin/echo", [...])` 与 `spawn("/bin/echo", [...])` 不再返回空 stdout、`null/undefined` exitCode、固定 pid 或 `"mock output"`。权限 broker 仍在执行前检查 `Process/Execute`。
- WebCrypto `getRandomValues` 兼容：现在只接受整数 TypedArray（含 BigInt typed arrays），会拒绝 `Float32Array/Float64Array`；随机填充严格限定在传入 view 的 `byteOffset..byteOffset+byteLength`，不再改写同一 ArrayBuffer 中 view 之外的字节。
- Timer `setInterval` 重复语义：interval fired 后会保留 V8 callback 并按 delay 继续调度，直到 `clearInterval` 移除 metadata、调度项和 callback；短 ref'ed interval 会在 `timer_drain_limit_ms` 窗口内保持事件循环活跃，未清理 interval 不会让一次 `execute_code` 无限阻塞。
- CLI `.tsx` 顶层 await 执行边界：`.ts/.tsx` 文件在 TypeScript 转译后若包含真实顶层 `await`、static import 或 export，会进入 V8 ESM module 执行路径；无 JSX 的 `.tsx` 顶层 await 会等待同步可结算 Promise 后再退出，不再被当作 script 抛 `await is only valid...` 语法错误。
- CLI TypeScript 运行时错误定位：`.ts/.tsx` 编译产物现在会追加原始文件 `sourceURL`，runtime 捕获 V8 异常时会提取第一条 `at ...` stack frame 并输出 `Location:`；因此 `amber run file.ts` 的运行时 throw 会显示原始 TypeScript 文件路径和源码行号，不再只返回裸异常消息。
- V8 Snapshot placeholder fail-closed：`SnapshotManager::generate_snapshot()` 在未接入真实 V8 snapshot blob 生成前会返回明确 `V8 snapshot generation is not implemented`，不再返回 `validate()` 失败的空 `Vec`；`save_snapshot_to_disk()` 和 `load_snapshot_from_disk()` 会拒绝无效 snapshot，避免把空 snapshot 持久化成可信性能产物。
- V8 Snapshot 持久化权限闭环：`save_snapshot_to_disk()`、`load_snapshot_from_disk()`、`list_persistent_snapshots()`、`delete_persistent_snapshot()` 在创建目录、写入 `.bin/.meta`、读取 metadata/blob、扫描目录和删除文件前检查全局 `FileSystem/Read|Write` broker；deny 时 fail closed 且不会写出或删除 snapshot 文件。
- Performance timeline 语义：`performance.now()` 改为基于进程 `timeOrigin` 的单调毫秒值，`performance.timeOrigin + performance.now()` 与 `Date.now()` 保持同一时间轴；`mark/measure/toJSON` 同步使用相对 timeline，`measure(name)` 无显式 mark 时从 `startTime = 0` 开始，不再混用 epoch 毫秒和相对毫秒。
- Web API Performance 事实源收敛：`web_api::init_web_api` 不再安装旧的空 timeline placeholder；`web_api::performance::setup_performance_api` 改为委托到默认运行时使用的 `nodejs_core::performance` 实现，因此备用初始化路径下 `mark/getEntriesByName/getEntriesByType/clear*/toJSON` 与 CLI 路径保持一致。
- BroadcastChannel listener 移除语义：`removeEventListener('message', listener)` 会按同一函数引用从当前 channel 的 listener 列表中过滤掉匹配项；后续 `postMessage()` 不再调用已移除 listener，不再把 `removeEventListener` 当作参数吞掉的 no-op placeholder。
- BroadcastChannel 事件类型隔离：listener 存储从单数组改为 `message` / `messageerror` 分桶；普通 `postMessage()` 只派发 `message` listener，不再误调用 `messageerror` listener。
- BroadcastChannel 同名路由语义：每个 channel 会登记到 V8 Private 全局注册表；`postMessage()` 只向同名、未关闭且不是发送者自身的 channel 派发 `message` 事件，异名 channel 隔离，`close()` 后不会继续接收后续广播。
- BroadcastChannel structured clone 投递语义：`postMessage()` 会在派发前复用当前 `structuredClone` 能力生成消息快照，接收端不再共享发送端对象引用；包含 function 等不可克隆 payload 会在派发任何 `message` 事件前抛错并 fail closed。
- MessageChannel structured clone / close 语义：`MessagePort.postMessage()` 现在会先 structured-clone payload，接收端不共享发送端对象引用，包含 function 等不可克隆 payload 会在派发前抛错；目标 port 已 close 时不会入队或派发，已 start 的 port 立即派发且不再保留已派发消息，因此重复 `start()` 不会重复投递。
- Payment Request fail-closed 边界：`web_api::init_web_api()` 路径下的 `PaymentRequest.show()` 在没有真实 payment handler/UI/用户确认模型前会 reject `Payment Request is not supported yet`；`canMakePayment()` 返回 `false`；`new PaymentResponse()` 会抛出 `PaymentResponse construction is not supported yet`，不再伪造 `basic-card` / `success` 支付响应。
- Payment Request 剩余假成功收口：`PaymentRequest.abort()` 在没有真实支付 UI/交互态时同样 reject `Payment Request is not supported yet`，不再立即 resolve；`new PaymentAddress()` 会抛出 `PaymentAddress construction is not supported yet`，不再返回全字段 `undefined` 的地址对象壳。
- Background Sync register fail-closed 边界：`registration.sync.register(tag)` 现在返回真正的 Promise，并在缺少真实 ServiceWorker registration store、网络状态调度和后台同步后端时 reject `Background Sync registration is not supported yet`；不会把 tag 写入全局静态列表。`registration.sync.getTags()` 也改为 Promise 形态，当前返回已注册 tag 快照。
- Clipboard API fail-closed 边界：`navigator.clipboard.writeText/readText/read/write` 均改为 Promise 形态，并在缺少权限模型、user activation、secure-context 与宿主剪贴板后端时 reject `Clipboard API is not supported yet`；移除进程级静态内存剪贴板状态，不再跨 runtime 保存或返回假剪贴板内容。
- Notification API fail-closed 边界：`Notification.permission` 和 `Notification.requestPermission()` 现在挂在 `Notification` 构造器上，不再泄露为 global 属性；`requestPermission()` 在没有真实用户授权后端时 resolve `"default"` 并保持权限不变，不再固定 grant；默认未授权时 `new Notification()` 抛出 `Notification permission is not granted`，不再创建通知对象壳或输出伪生命周期日志。
- Fetch Response Body mixin 消费语义：`response.text/json/arrayBuffer/blob()` 现在共享同一消费检查；首次读取后会把同一个 Response 的 `bodyUsed` 从 `false` 更新为 `true`，后续读取抛出 `Response body already consumed`。`response.clone()` 也会在 body 已消费后抛错，不再让已消费响应通过浅复制复活。
- Fetch Response Headers 语义：`fetch()` 返回的 `response.headers` 不再只是普通属性袋；现在会用全局 `Headers` 构造器生成实例，并把真实 HTTP 响应头写入其中，因此 `response.headers.get(name)` / `has(name)` 可以大小写不敏感地读取本次请求的真实响应头。
- Fetch Response 构造器语义：`new Response(body, init)` 不再把第一个参数误当 status、第二个参数误当 body；现在按 body-first 解析，支持 `init.status`、`init.statusText`、plain-object `init.headers`，并为构造出的 Response 挂载 `text/json/arrayBuffer/blob/clone` 与 `bodyUsed`。
- Fetch Headers init / Request 传递语义：`new Headers({ ... })` 现在会初始化内部 header cache；`fetch(url, { headers: new Headers(...) })` 和 `fetch(new Request(url, { headers }))` 会从 Headers 实例读取真实 entries 并发送到本地 HTTP 请求，不再把 `get/set/has/delete/append` 方法当普通属性枚举。
- Fetch HeadersInit sequence 语义：`new Headers([["Name", "value"]])` 与 `fetch(url, { headers: [["Name", "value"]] })` 现在按 header pair sequence 解析并发送真实请求头，不再把数组索引 `"0"` / `"1"` 当 header 名。
- Fetch Headers 遍历语义：`headers.keys()`、`values()`、`entries()`、`forEach(callback)` 和 `[Symbol.iterator]()` 现在从内部 header cache 暴露真实 entries；`keys/values/entries` 返回带 `next()` 的标准 iterator，`Array.from(headers.entries())`、`Array.from(headers)`、`for...of headers` 和 `forEach((value, name, headers) => ...)` 可用于框架常见 header 遍历路径。
- Fetch Headers name 规范化：Headers 内部 cache 现在按 Web 标准把 header name 归一为 lowercase；`set/append`、plain-object init、sequence init 以及自动 `content-type` 写入都会存储 lowercase 名称，因此 `keys/entries/forEach` 不再暴露原始大小写。
- FormData 遍历语义：`formData.keys()`、`values()`、`entries()` 和 `[Symbol.iterator]()` 现在返回带 `next()` 的标准 iterator，并保留重复 name 的插入顺序；`formData.forEach(callback, thisArg)` 会按 `(value, name, formData)` 调用 callback，不再是空 stub。
- FormData multipart Blob/File bytes：`fetch(url, { body: formData })` 序列化 multipart 时会从 Blob/File 的 byte store 写入真实 body bytes，保留非 UTF-8 字节；Blob 显式 filename、File 默认 `.name`、Blob/File `type` 会进入对应 part headers，普通字段不再把缺省第三参序列化成 `filename="undefined"`。
- URLSearchParams 遍历语义：`params.entries()/keys()/values()` 返回的 iterator 现在自身可迭代，`URLSearchParams` 对象本身也可通过 `[Symbol.iterator]()` / `for...of` 按 entries 遍历；`params.forEach(callback, thisArg)` 会尊重 `thisArg` 并传入 `(value, key, params)`。
- URLSearchParams 构造与编码语义：`new URLSearchParams([["k", "v"]])` 现在按 sequence pair 初始化，`new URLSearchParams(existingParams)` 会复制当前 pairs 而不是共享后续 mutation；query string 中的 `+` 会解码为空格，序列化空格时使用 `+`，保留字面 `+` 为 `%2B`。
- URLSearchParams iterable/fail-fast 语义：`new URLSearchParams(new Map([...]))` 现在会按 iterable entry 初始化，不再把 `Map` 当空 record；sequence init 中非数组 pair、长度不足或长度超过 2 的 pair 会抛 `TypeError`，不再静默忽略坏输入。
- TextEncoder.encodeInto 写入语义：默认运行时与备用 Web API 初始化路径都会把 UTF-8 字节写入传入的 `Uint8Array` 视图，并尊重 byteOffset；目标容量不足时不会拆开多字节字符，`read`/`written` 反映实际读取的 UTF-16 code units 与写入字节数，不再返回假成功。
- TextDecoder 选项语义：默认运行时与备用 Web API 初始化路径都会保存并执行 `fatal` / `ignoreBOM`；`fatal:true` 遇到 invalid UTF-8 会抛 `TypeError`，默认 `ignoreBOM:false` 会剥离初始 UTF-8 BOM，`ignoreBOM:true` 会保留 U+FEFF，不再只把选项暴露成属性但 decode 时忽略。
- Web Streams 锁语义：`ReadableStream.getReader()` 与 `WritableStream.getWriter()` 现在会把 stream 标记为 locked，锁住期间重复获取 reader/writer 会抛 `TypeError`；`releaseLock()` 会释放锁并允许重新获取，不再重置 ReadableStream 的读取索引导致队列倒带。
- Node querystring builtin 接入：默认 `MinimalRuntime` 现在会初始化全局 `querystring` 模块，`require("querystring")` 暴露 `parse/stringify/escape/unescape`；`parse` 会 percent-decode、保留重复 key 为数组，`stringify` 会对对象和数组值做 percent-encoding，不再返回固定占位结果。
- Blob.arrayBuffer 数据语义：`Blob` / `File` 构造时会保存原始 byte store；`Blob.arrayBuffer()` 与 `Blob.slice()` 从 byte store 拷贝数据生成 `ArrayBuffer` / 新 Blob，`new Uint8Array(new Blob([new Uint8Array([0,255,65])]).arrayBuffer())` 不再被 UTF-8 replacement 污染。
- Event/CustomEvent cancelable 语义：`Event` 与 `CustomEvent` 默认 `cancelable: false`，会解析 `EventInit`/`CustomEventInit` 的 `bubbles` 与 `cancelable` 字段；`preventDefault()` 只有在事件可取消时才会设置 `defaultPrevented`，`dispatchEvent()` 因此能按标准返回是否未被取消。
- DOMParser MIME 边界：`parseFromString(input, contentType)` 现在要求显式传入受支持的 MIME type（`text/html`、`text/xml`、`application/xml`、`application/xhtml+xml`、`image/svg+xml`）；缺失或非法类型会抛 `TypeError`，不再默认按 `text/html` 假成功。

### 第二波仍未完成

- PermissionState/ResourceBroker 仍需要对未启用的 inspector/debugger 历史模块、standalone benchmark tools、testing coverage/perf history 等剩余 I/O 入口做清点；默认构建中的主要 FS/net/env/run 入口目前已覆盖 CLI 与 JSON policy 的最小表达。
- ESM 已有相对/绝对 `.mjs`、`type:"module"` `.js`、bare package specifier 按 `import/node/default` 条件解析到 ESM 文件、同步可结算 top-level await、有限 timer-backed top-level await settle、pending TLA fail-closed、native ESM 相对 `.mjs` dynamic import、普通 script/CommonJS 入口相对 `.mjs` dynamic import、库层 `Runtime::execute_file` 相对 `.mjs` dynamic import、dynamic import `file://` URL specifier、非 `file://` URL 明确拒绝、dynamic import CommonJS interop namespace cache、dynamic import 内建模块 namespace cache、`path`/`fs`/`url`/`events`/`os`/`stream`/`process`/`crypto` 内建模块 namespace、native file-backed dependency cache invalidation/reload、ESM -> CommonJS default/namespace/named snapshot，以及 CommonJS -> ESM namespace bridge（`require("./file.mjs")` / `require("./type-module.js")`）的 graph fingerprint cache invalidation/reload 与 pending TLA fail-closed 竖切片；仍需要更长生命周期、跨宿主异步 I/O 的 module evaluation 设计。
- WebCrypto/Node crypto 仍需要后续清点更广的算法覆盖和真实实现边界：剩余重点包括更多 WebCrypto/Node crypto 错误对象细节（例如错误类型/参数名），以及更广 cipher 模式/别名兼容性的进一步覆盖。
- `amber test` 仍不是完整 Jest runner；本轮已修复 single-file 可见 flag、timeout、done callback、无文件发现、空文件假绿、若干假阳性、file-mode lifecycle hooks、`describe.skip/only`、`test.todo/it.todo`、`test.concurrent` 串行兼容入口、`jest.resetModules()` / `jest.isolateModules(fn)` / `jest.isolateModulesAsync(fn)`、核心/数值/object matcher、`toThrow(expected)`、`.resolves/.rejects`、regex filter、`jest.fn` / `jest.mock` / `jest.doMock` / `jest.setMock` mock，文件 snapshot 读取/`--update-snapshots`、静态 inline snapshot 比对和 inline snapshot 自动更新，库层 `ParallelExecutor` / `EnhancedRunner` 未实现执行路径的 fail-closed 竖切片，以及 `V8TestExecutor` 核心 matcher mismatch false-green 收口，但更广 matcher/mock API、watch、真实并发调度语义仍有限。

### 最新验证快照

- `cargo test --lib expect_to_be_throws_for_mismatched_values -- --nocapture`（先红后绿）
- `cargo test --lib core_matchers_throw_for_mismatches -- --nocapture`（先红后绿）
- `cargo test --lib v8_test_executor::tests -- --nocapture`
- `cargo test --test v8_test_executor_matcher_tests -- --nocapture`
- `cargo test --test cli_regression_tests test_command_supports_to_have_property_bracket_string_paths -- --nocapture`（先红后绿）
- `cargo test --test cli_regression_tests test_command_supports_object_file_matchers -- --nocapture`
- `cargo test --test cli_regression_tests test_command_reports_object_matcher_failure -- --nocapture`
- `cargo test --test cli_regression_tests test_command_supports_asymmetric_file_matchers -- --nocapture`
- `cargo test --test text_encoding_tests test_text_decoder_fatal_invalid_utf8_throws -- --nocapture`（先红后绿）
- `cargo test --test text_encoding_tests test_text_decoder_ignore_bom_controls_initial_bom -- --nocapture`（先红后绿）
- `cargo test --test text_encoding_tests -- --nocapture`
- `cargo test --test http_fetch_tests test_fetch_form_data_multipart_uses_blob_and_file_bytes -- --nocapture`（先红后绿）
- `cargo test --test http_fetch_tests form_data -- --nocapture`
- `cargo test --test blob_api_tests test_blob_array_buffer_preserves_uint8array_bytes -- --nocapture`
- `cargo test --test crypto_cipheriv_tests test_create_cipheriv_decipheriv_aes_gcm_auth_tag_and_aad_match_node_vector -- --nocapture`（先红后绿）
- `cargo test --test crypto_cipheriv_tests test_create_decipheriv_aes_gcm_rejects_wrong_auth_tag -- --nocapture`（先红后绿）
- `cargo test --test crypto_cipheriv_tests test_create_cipheriv_invalid_input_error_codes_match_node -- --nocapture`（先红后绿）
- `cargo test --test crypto_cipheriv_tests test_create_decipheriv_invalid_input_error_codes_match_node -- --nocapture`（先红后绿）
- `cargo test --test crypto_cipheriv_tests -- --nocapture`
- `cargo test --test esm_module_tests runtime_mjs_dynamic_import_rejects_non_file_url_specifier -- --nocapture`（先红后绿）
- `cargo test --test esm_module_tests runtime_mjs_entry_supports_dynamic_import_file_url_mjs -- --nocapture`
- `cargo test --test v8_snapshot_warmup_tests snapshot_persistence_uses_global_file_broker -- --nocapture`（先红后绿）
- `cargo test --test v8_snapshot_warmup_tests -- --nocapture`
- `cargo test --test testing_runner_fail_closed_tests -- --nocapture`（先红后绿）
- `cargo test --test esm_module_tests runtime_mjs_entry_supports_dynamic_import_relative_mjs -- --nocapture`（先红后绿）
- `cargo test --test esm_module_tests runtime_script_dynamic_import_resolves_relative_mjs_from_main_module_path -- --nocapture`（先红后绿）
- `cargo test --test persistent_runtime_tests test_execute_file_dynamic_import_resolves_relative_to_file -- --nocapture`（先红后绿）
- `cargo test --test cli_regression_tests eval_allow_net_host_does_not_allow_http_server_listen -- --nocapture`（先红后绿）
- `cargo test --test cli_regression_tests eval_deny_net_allows_explicit_listen_exception -- --nocapture`（先红后绿）
- `cargo test --test cli_regression_tests serve_deny_net_blocks_server_configuration -- --nocapture`
- `cargo test --test permission_state_tests -- --nocapture`
- `cargo test --lib watcher_websocket::tests::start_uses_global_network_broker_before_binding_listener -- --nocapture`（先红后绿）
- `cargo test --test esm_module_tests runtime_mjs_dynamic_import_rejects_missing_transitive_dependency -- --nocapture`
- `cargo test --test esm_module_tests runtime_mjs_dynamic_import_rejects_dependency_syntax_error -- --nocapture`
- `cargo test --test esm_module_tests runtime_mjs_dynamic_import_rejects_dependency_evaluation_error -- --nocapture`（先红后绿）
- `cargo test --test esm_module_tests runtime_mjs_dynamic_import_commonjs_dependency_uses_namespace_cache -- --nocapture`（先红后绿）
- `cargo test --test esm_module_tests runtime_mjs_dynamic_import_builtin_uses_namespace_cache -- --nocapture`（先红后绿）
- `cargo test --test esm_module_tests -- --nocapture`
- `cargo test --test crypto_createhmac_tests test_create_hmac_sha1_known_vector -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhmac_tests test_create_hmac_md5_known_vector -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhmac_tests test_hmac_empty_key_standard_algorithms -- --nocapture`（先 abort 后绿）
- `cargo test --test crypto_createhmac_tests -- --nocapture`
- `cargo test --test crypto_createhmac_tests test_hmac_latin1_and_binary_encodings_return_digest_bytes -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhash_tests test_hash_latin1_and_binary_encodings_return_digest_bytes -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhash_tests test_hash_digest_without_encoding_returns_binary_object -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhmac_tests test_hmac_digest_without_encoding_returns_binary_object -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhash_tests test_hash_rejects_digest_and_update_after_digest -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhmac_tests test_hmac_allows_empty_second_digest_but_rejects_update_after_digest -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhash_tests test_hash_update_accepts_uint8array_bytes -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhmac_tests test_hmac_update_accepts_uint8array_bytes -- --nocapture`
- `cargo test --test crypto_createhash_tests test_hash_supports_base64url_digest_and_update_input -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhmac_tests test_hmac_supports_base64url_digest_and_update_input -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhmac_tests test_hmac_accepts_uint8array_key_bytes -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhmac_tests test_hmac_string_key_respects_encoding_option -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhash_tests test_create_hash_sha384_known_vector -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhash_tests test_create_hash_accepts_common_algorithm_aliases -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhmac_tests test_create_hmac_sha384_and_alias_known_vector -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhash_tests test_hash_copy_clones_partial_state -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhash_tests test_hash_copy_after_digest_throws -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhash_tests test_hash_rejects_unsupported_algorithm_at_creation -- --nocapture`（先红后绿）
- `cargo test --test crypto_createhash_tests --test crypto_createhmac_tests -- --nocapture`
- `cargo test --test cli_regression_tests run_typescript_error_diagnostics_fail_before_execution -- --nocapture`（先红后绿）
- `cargo test --test cli_regression_tests run_accepts_double_dash_before_script_args -- --nocapture`
- `cargo test --test cli_regression_tests run_preload_file_resolves_relative_require_from_preload_dir -- --nocapture`（先红后绿）
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test crypto_cipheriv_tests test_create_decipheriv_invalid -- --nocapture`
- `cargo test --test crypto_cipheriv_tests test_create_cipheriv_aes_ctr_invalid_iv_length -- --nocapture`
- `cargo test --test crypto_scrypt_tests known_rfc7914_vector -- --nocapture`
- `cargo test --test crypto_scrypt_tests -- --nocapture`
- `cargo test --test cli_regression_tests test_command_runs_before_each_and_after_each_around_each_file_test -- --nocapture`
- `cargo test --test cli_regression_tests test_command_runs_describe_scoped_each_hooks_in_jest_order -- --nocapture`
- `cargo test --test cli_regression_tests test_command_runs_after_each_after_failed_file_test -- --nocapture`
- `cargo test --test commonjs_resolver_tests resolves_package_json_imports_pattern_for_commonjs_require -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_imports_pattern_prefers_longer_base_over_longer_trailer -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_imports_pattern_capture_parent_segment_is_invalid_module_specifier -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_imports_trailing_slash_is_invalid_module_specifier -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_resolves_package_json_imports_pattern -- --nocapture`
- `cargo test --test commonjs_resolver_tests resolves_package_json_imports_exact_external_package_target -- --nocapture`
- `cargo test --test commonjs_resolver_tests resolves_package_json_imports_pattern_external_package_target -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_imports_external_parent_target_is_invalid_package_target -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_resolves_package_json_imports_external_package_target -- --nocapture`
- `cargo test --test commonjs_resolver_tests -- --nocapture`
- `cargo test --test esm_module_tests runtime_static_default_import_loads_commonjs_module -- --nocapture`
- `cargo test --test esm_module_tests runtime_static_named_import_loads_commonjs_module -- --nocapture`
- `cargo test --test esm_module_tests runtime_static_namespace_and_side_effect_imports_load_commonjs_modules -- --nocapture`
- `cargo test --test esm_module_tests -- --nocapture`
- `cargo test --test crypto_keyobjects_tests test_create_private_key_exists -- --nocapture`
- `cargo test --lib`
- `cargo clippy --all-targets -- -D warnings`
- `cargo check --features benchmarks`
- `cargo clippy --features benchmarks -- -D warnings`
- `cargo test --test commonjs_resolver_tests package_json_exports_target_without_dot_slash_is_rejected -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_rejects_package_exports_target_without_dot_slash -- --nocapture`
- `cargo test --test commonjs_resolver_tests ignores_package_json_module_field_for_commonjs_require -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_ignores_package_json_module_field -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_exports_target_with_node_modules_segment_is_rejected -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_rejects_package_exports_target_with_node_modules_segment -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_exports_target_with_dot_segment_after_prefix_is_rejected -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_rejects_package_exports_target_with_dot_segment_after_prefix -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_exports_invalid_primitive_rejects_main_fallback -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_rejects_package_exports_invalid_primitive -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_subpath_pattern_trailer_requires_non_empty_capture -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_subpath_pattern_prefers_longer_base_over_longer_trailer -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_subpath_pattern_capture_parent_segment_is_invalid_module_specifier -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_subpath_pattern_capture_node_modules_segment_is_invalid_module_specifier -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_exports_array_null_entry_blocks_later_targets -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_reports_package_exports_array_null_entry -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_conditional_exports_empty_array_blocks_later_conditions -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_exports_mixed_dot_and_condition_keys_is_invalid_config -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_subpath_exports_mixed_dot_and_condition_keys_is_invalid_config -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_rejects_package_exports_mixed_dot_and_condition_keys -- --nocapture`
- `cargo test --test esm_module_tests runtime_static_import_esm_dependency_uses_live_binding -- --nocapture`
- `cargo test --test esm_module_tests runtime_mjs_entry_supports_top_level_await_without_imports_or_exports -- --nocapture`
- `cargo test --test esm_module_tests runtime_mjs_entry_top_level_await_settles_after_timer -- --nocapture`（先红后绿）
- `cargo test --test esm_module_tests top_level_await -- --nocapture`
- `cargo test --test esm_module_tests runtime_type_module_js_entry_supports_top_level_await_without_imports_or_exports -- --nocapture`
- `cargo test --test esm_module_tests runtime_static_import_esm_dependency_supports_top_level_await_export -- --nocapture`
- `cargo test --test esm_module_tests runtime_type_module_js_entry_uses_native_esm_graph -- --nocapture`
- `cargo test --test esm_module_tests runtime_mjs_entry_imports_type_module_js_dependency_as_esm -- --nocapture`
- `cargo test --test esm_module_tests runtime_mjs_entry_imports_package_exports_mjs_as_esm -- --nocapture`
- `cargo test --test esm_module_tests runtime_mjs_entry_uses_import_condition_for_package_exports -- --nocapture`
- `cargo test --test esm_module_tests runtime_native_esm_imports_builtin_path_namespace -- --nocapture`
- `cargo test --test esm_module_tests runtime_native_esm_imports_builtin_fs_namespace -- --nocapture`
- `cargo test --test esm_module_tests runtime_native_esm_imports_builtin_url_namespace -- --nocapture`
- `cargo test --test esm_module_tests runtime_native_esm_imports_builtin_events_namespace -- --nocapture`
- `cargo test --test esm_module_tests runtime_native_esm_imports_builtin_os_namespace -- --nocapture`
- `cargo test --test esm_module_tests runtime_native_esm_imports_builtin_stream_namespace -- --nocapture`
- `cargo test --test esm_module_tests runtime_native_esm_imports_commonjs_dependency_default_namespace -- --nocapture`
- `cargo test --test esm_module_tests runtime_native_esm_imports_commonjs_dependency_named_exports -- --nocapture`
- `cargo test --test esm_module_tests runtime_static_import_esm_dependency_uses_module_cache_between_executions -- --nocapture`
- `cargo test --test permission_state_tests esm_dependency_import_uses_global_file_read_broker -- --nocapture`
- `cargo test --test permission_state_tests esm_fs_builtin_import_uses_global_file_read_broker -- --nocapture`
- `cargo test --test commonjs_resolver_tests -- --nocapture`
- `bash -n benchmarks/run_real_comparison_fixed.sh`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --lib`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test commonjs_resolver_tests --test permission_state_tests --test http_fetch_tests --test runtime_async_tests --test cli_regression_tests --test crypto_fail_closed_tests --test crypto_rsa_tests --test crypto_ecdsa_tests --test crypto_aes_gcm_tests --test fetch_fail_closed_tests --test event_loop_timer_tests --test typescript_compiler_integration_tests --test fs_module_tests`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test package_manager_security_tests --test runtime_eval_semantics_tests --test minimal_runtime_fast_tests --test persistent_runtime_tests --test async_timer_tests`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test nodejs_api_tests test_fs`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test nodejs_api_tests --test process_module_tests --test process_next_tick_tests --test next_tick_order_test --test next_tick_timer_order_enhanced_test`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test cli_regression_tests deny_fs -- --nocapture`
- `cargo test --test package_manager_security_tests -- --nocapture`
- `cargo test --test hot_reload_tests -- --nocapture`
- `cargo test --lib watcher::tests -- --nocapture`
- `cargo test --lib watcher_websocket::tests -- --nocapture`
- `cargo test --lib testing::perf::benchmark::tests -- --nocapture`
- `cargo test --test cli_regression_tests debug_deny_fs_blocks_debug_target_file_read -- --nocapture`
- `cargo test --test test_debug_cli -- --nocapture --test-threads=1`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test cli_regression_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test permission_state_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test commonjs_resolver_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test webcrypto_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test fetch_fail_closed_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_aes_gcm_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test permission_state_tests uses_global_network_permission_broker -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test permission_state_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test commonjs_resolver_tests node_prefix -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test commonjs_resolver_tests runtime_require_loads_json_module_as_object -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test commonjs_resolver_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test permission_state_tests child_process_uses_global_process_permission_broker -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_fail_closed_tests test_create_verify_rejects_unrecognized_mock_signature -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_fail_closed_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_createverify_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test permission_state_tests process_chdir_uses_global_process_permission_broker -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test permission_state_tests process_env_rechecks_permission_after_runtime_initialization -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test permission_state_tests --test package_manager_security_tests --test commonjs_resolver_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test process_tests --test process_module_tests --test process_event_handler_tests --test module_system_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test commonjs_resolver_tests resolves_package_json_subpath_exports_pattern -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test commonjs_resolver_tests package_subpath_exports_pattern -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_wrap_key_tests test_wrap_key_without_iv_fails_closed -- --nocapture`
- `cargo test --test crypto_ecdsa_tests`
- `cargo test --test crypto_ecdh_derive_tests`
- `cargo test --test webcrypto_tests`
- `cargo test --test crypto_fail_closed_tests`
- `cargo test --test crypto_wrap_key_tests`
- `cargo test --test crypto_generatekeypairsync_tests --test crypto_generatekeypair_tests`
- `cargo test --test crypto_keyobjects_tests`
- `cargo test --test crypto_createsign_tests --test crypto_createverify_tests --test crypto_fail_closed_tests --test crypto_createecdh_tests`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_wrap_key_tests test_wrap_key_with_short_iv_fails_closed -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test webcrypto_tests test_get_random_values_returns_input_typed_array -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_wrap_key_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_aes_gcm_tests --test crypto_fail_closed_tests --test webcrypto_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test package_manager_security_tests read_package_lock_uses_global_file_read_broker -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test package_manager_security_tests generate_package_lock_uses_global_file_write_broker -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test commonjs_resolver_tests package_json_exports_blocks_unexported_subpath_with_specific_error -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test commonjs_resolver_tests runtime_require_reports_unexported_package_subpath -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test commonjs_resolver_tests resolves_relative_typescript_module_extension -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test commonjs_resolver_tests runtime_require_loads_typescript_module -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_fail_closed_tests test_subtle_import_key_missing_algorithm_name_rejects -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_fail_closed_tests test_generate_key_rejects_usage_not_allowed_for_algorithm -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_wrap_key_tests test_unwrap_key_without_iv_fails_closed -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_wrap_key_tests test_unwrap_key_with_wrong_iv_fails_closed -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_fail_closed_tests test_generated_rsa_key_pair_sign_verify_round_trip -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_createsign_tests --test crypto_createverify_tests --test crypto_generatekeypair_tests --test crypto_generatekeypairsync_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_fail_closed_tests test_public_encrypt_rejects_placeholder_public_key -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_fail_closed_tests test_private_encrypt_rejects_placeholder_private_key -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_fail_closed_tests test_public_private_rsa_encrypt_decrypt_round_trip_uses_real_key -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_fail_closed_tests test_private_public_rsa_encrypt_decrypt_round_trip_uses_real_key -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_fail_closed_tests test_rsa_encrypt_outputs_modulus_sized_ciphertext -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_publicencrypt_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_private_public_encrypt_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_fail_closed_tests test_create_ecdh_generates_uncompressed_p256_public_key -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_fail_closed_tests test_create_ecdh_compute_secret_rejects_invalid_peer_public_key -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_fail_closed_tests test_create_ecdh_set_private_key_recomputes_public_key -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_createecdh_tests -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_ecdh_derive_tests test_ecdh_generatekey_p256_public_key_is_uncompressed_point -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_ecdh_derive_tests test_ecdh_derivebits_awaited_symmetric_shared_secret -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_ecdh_derive_tests test_ecdh_derivebits_rejects_invalid_peer_public_key_material -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/amberjs-main-target cargo test --test crypto_ecdh_derive_tests -- --nocapture`
- `cargo test --test crypto_aes_cbc_tests`
- `cargo test --test crypto_aes_cbc_tests --test crypto_aes_gcm_tests --test crypto_fail_closed_tests --test crypto_importkey_tests --test crypto_exportkey_tests --test crypto_wrap_key_tests --test webcrypto_tests`
- `CARGO_TARGET_DIR=/tmp/amberjs-aesctr-target cargo test --test crypto_aes_ctr_tests --test crypto_fail_closed_tests`
- `cargo test --test crypto_fail_closed_tests`
- `cargo test --test crypto_generatekey_tests --test crypto_importkey_tests --test crypto_exportkey_tests --test crypto_aes_gcm_tests --test crypto_aes_cbc_tests --test crypto_aes_ctr_tests --test crypto_fail_closed_tests --test webcrypto_tests --test crypto_ecdh_derive_tests`
- `cargo test --test crypto_wrap_key_tests test_wrap_unwrap_aes_key_with_aes_kw_round_trip -- --nocapture`
- `cargo test --test crypto_wrap_key_tests test_unwrap_key_with_aes_kw_rejects_tampered_data -- --nocapture`
- `cargo test --test crypto_wrap_key_tests -- --nocapture`
- `cargo test --test crypto_wrap_key_tests --test crypto_generatekey_tests --test crypto_importkey_tests --test crypto_exportkey_tests --test crypto_aes_gcm_tests --test crypto_aes_cbc_tests --test crypto_aes_ctr_tests --test crypto_fail_closed_tests --test webcrypto_tests`
- `cargo test --test commonjs_resolver_tests resolves_package_json_exports_array_fallback -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_root_null_export_blocks_main_fallback -- --nocapture`
- `cargo test --test commonjs_resolver_tests resolves_package_json_subpath_exports_array_fallback -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_null_subpath_export_blocks_private_file -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_resolves_package_exports_array_fallback -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_reports_null_package_root_export -- --nocapture`
- `cargo test --test commonjs_resolver_tests -- --nocapture`
- `cargo test --test commonjs_resolver_tests package_json_subpath_exports_without_root_blocks_main_fallback -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_reports_subpath_only_exports_package_root -- --nocapture`
- `cargo test --test commonjs_resolver_tests resolves_package_self_reference_root_exports -- --nocapture`
- `cargo test --test commonjs_resolver_tests resolves_scoped_package_self_reference_subpath_exports -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_resolves_package_self_reference_subpath_exports -- --nocapture`
- `cargo test --test commonjs_resolver_tests resolves_package_json_conditional_exports_in_package_order -- --nocapture`
- `cargo test --test commonjs_resolver_tests resolves_package_json_conditional_exports_preserves_require_before_node_order -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_respects_package_conditional_export_order -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_loads_mjs_module_namespace -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_loads_js_inside_type_module_package_namespace -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_allows_cjs_inside_type_module_package -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_loads_tsx_without_jsx_using_typescript_transpile -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_rejects_tsx_element_syntax_without_executing_it -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_rejects_jsx_element_syntax_without_executing_it -- --nocapture`
- `cargo test --test commonjs_resolver_tests -- --nocapture`
- `cargo test --test cli_regression_tests eval_permission_policy_denies_fs_and_allows_relative_read_path -- --nocapture`
- `cargo test --test cli_regression_tests eval_permission_policy_denies_environment_except_allow_list -- --nocapture`
- `cargo test --test cli_regression_tests eval_deny_env_allows_explicit_environment_exception -- --nocapture`
- `cargo test --test cli_regression_tests eval_deny_run_allows_explicit_child_process_command -- --nocapture`
- `cargo test --test cli_regression_tests -- --nocapture`
- `cargo test --test permission_state_tests -- --nocapture`
- `cargo test --lib`
- `cargo test --test module_system_tests test_require_process_returns_global_process_object -- --nocapture`
- `cargo test --test esm_module_tests runtime_native_esm_imports_builtin_process_namespace -- --nocapture`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo test --test esm_module_tests -- --nocapture`
- `cargo test --test module_system_tests -- --nocapture`
- `cargo test --test process_tests -- --nocapture`
- `cargo test --test process_enhanced_tests -- --nocapture`
- `cargo test --test process_next_tick_tests -- --nocapture`
- `cargo test --lib -- --nocapture`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test esm_module_tests runtime_native_esm_imports_builtin_crypto_namespace -- --nocapture`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo test --test esm_module_tests runtime_native_esm_imports_builtin_crypto_namespace -- --nocapture`
- `cargo test --test esm_module_tests -- --nocapture`
- `cargo test --test crypto_createhash_tests -- --nocapture`
- `cargo test --test crypto_randombytes_tests -- --nocapture`
- `cargo test --test webcrypto_tests test_get_random_values_returns_input_typed_array -- --nocapture`
- `cargo test --test crypto_randomuuid_tests -- --nocapture`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test esm_module_tests runtime_static_import_esm_dependency_reloads -- --nocapture`
- `cargo test --test esm_module_tests runtime_static_import_esm_dependency_uses_module_cache_between_executions -- --nocapture`
- `cargo test --test esm_module_tests -- --nocapture`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test cli_regression_tests test_command_runs_before_all -- --nocapture`
- `cargo test --test cli_regression_tests test_command_runs_describe_scoped_all_hooks_before_following_file_tests -- --nocapture`
- `cargo test --test cli_regression_tests test_command_runs_after_all_after_failed_file_test -- --nocapture`
- `cargo test --test cli_regression_tests test_command_runs -- --nocapture`
- `cargo test --test cli_regression_tests -- --nocapture`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test cli_regression_tests test_command_test_name_pattern_uses_regex_for_file_tests -- --nocapture`
- `cargo test --test cli_regression_tests test_command_test_only_uses_regex_for_file_tests -- --nocapture`
- `cargo test --test cli_regression_tests test_command_test_skip_uses_regex_for_file_tests -- --nocapture`
- `cargo test --test enhanced_test_command_tests regexes -- --nocapture`
- `cargo test --test cli_regression_tests`
- `cargo test --test enhanced_test_command_tests`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test amberjs_core_tests test_execute_code_preserves_js_template_literal_with_type_word -- --nocapture`
- `cargo test --test amberjs_core_tests test_execute_code_preserves_mapped_type_looking_string -- --nocapture`
- `cargo test --test amberjs_core_tests test_execute_code_still_transpiles_type_aliases_with_mapped_and_template_types -- --nocapture`
- `cargo test --test amberjs_core_tests`
- `cargo test --test cli_regression_tests`
- `cargo test --test enhanced_test_command_tests`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test cli_regression_tests test_command_supports_jest_fn_mock_tracking -- --nocapture`
- `cargo test --test cli_regression_tests test_command_reports_mock_matcher_failure -- --nocapture`
- `cargo test --test cli_regression_tests test_command_sets_process_argv_for_test_file -- --nocapture`
- `cargo test --test cli_regression_tests test_command_default_timeout_fails_pending_timer_promise -- --nocapture`
- `cargo test --test cli_regression_tests test_command -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_mjs_module_namespace_reloads -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require -- --nocapture`
- `cargo test --test commonjs_resolver_tests -- --nocapture`
- `cargo test --test esm_module_tests -- --nocapture`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test cli_regression_tests test_command_supports_core_file_matchers -- --nocapture`
- `cargo test --test cli_regression_tests test_command_reports_negated_matcher_failure -- --nocapture`
- `cargo test --test cli_regression_tests test_command -- --nocapture`
- `cargo test --test cli_regression_tests -- --nocapture`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test cli_regression_tests test_command_exposes_jest_fn_to_required_helpers -- --nocapture`
- `cargo test --test cli_regression_tests test_command_reports_async_assertion_failure_in_mjs_file_mode -- --nocapture`
- `cargo test --test cli_regression_tests -- --nocapture`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test cli_regression_tests test_command_describe_skip_skips_file_suite_tests_and_hooks -- --nocapture`
- `cargo test --test cli_regression_tests test_command_describe_only_runs_only_file_suite_tests -- --nocapture`
- `cargo test --test cli_regression_tests test_command_describe_skip_suppresses_nested_only -- --nocapture`
- `cargo test --test cli_regression_tests`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test cli_regression_tests test_command_supports_to_throw_expected_file_matchers -- --nocapture`
- `cargo test --test cli_regression_tests test_command_reports_to_throw_expected_failure -- --nocapture`
- `cargo test --test cli_regression_tests`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test cli_regression_tests test_command_supports_numeric_file_matchers -- --nocapture`
- `cargo test --test cli_regression_tests test_command_reports_numeric_matcher_failure -- --nocapture`
- `cargo test --test cli_regression_tests`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test cli_regression_tests test_command_supports_resolves_and_rejects_file_matchers -- --nocapture`
- `cargo test --test cli_regression_tests test_command_reports_resolves_rejects_state_mismatch -- --nocapture`
- `cargo test --test cli_regression_tests`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test cli_regression_tests test_command_supports_object_file_matchers -- --nocapture`
- `cargo test --test cli_regression_tests test_command_reports_object_matcher_failure -- --nocapture`
- `cargo test --test cli_regression_tests`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test cli_regression_tests test_command_fails_when_file_registers_no_tests -- --nocapture`
- `cargo test --test cli_regression_tests`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test cli_regression_tests test_command_counts_todo_file_tests_as_skipped -- --nocapture`
- `cargo test --test cli_regression_tests`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test cli_regression_tests test_command_before_all_failure_blocks_remaining_suite_tests -- --nocapture`
- `cargo test --test cli_regression_tests test_command_runs_before_all_and_after_all_once_for_file_tests -- --nocapture`
- `cargo test --test cli_regression_tests test_command_runs_describe_scoped_all_hooks_before_following_file_tests -- --nocapture`
- `cargo test --test cli_regression_tests test_command_runs_after_all_after_failed_file_test -- --nocapture`
- `cargo test --test cli_regression_tests`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test commonjs_resolver_tests malformed_package_json_blocks_index_fallback -- --nocapture`
- `cargo test --test commonjs_resolver_tests runtime_require_reports_invalid_package_config_for_malformed_package_json -- --nocapture`
- `cargo test --test commonjs_resolver_tests`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --test cli_regression_tests test_command_supports_done_callback_async_file_tests -- --nocapture`
- `cargo test --test cli_regression_tests test_command_reports_done_callback_error -- --nocapture`
- `cargo test --test cli_regression_tests test_command_reports_done_callback_returned_promise_conflict -- --nocapture`
- `cargo test --test cli_regression_tests`
- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo clippy --all-targets -- -D warnings`

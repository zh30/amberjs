# Amber CLI 使用指南

本文档描述 Amber v1.16.0 默认二进制 `bee` 的当前 CLI 行为。对照 [CURRENT_SCOPE.md](CURRENT_SCOPE.md) 看 Stable / Preview / Experimental。

## 基本命令

```bash
amber --version
amber --help
amber version
```

`--verbose` 是全局参数，需要放在子命令前：

```bash
bee --verbose run examples/basics/hello_world.js
```

## 执行脚本

```bash
amber run examples/basics/hello_world.js
amber run examples/basics/typescript_demo.ts
amber run script.js -- arg1 arg2
amber run --preload ./setup.js app.js
```

执行 `.ts` 或 `.tsx` 文件时，Amber 会先调用内置 TypeScript 转译模块，再交给 V8 执行。Error 级 TypeScript diagnostics 会使命令在执行 JS 前失败；Warning/Info diagnostics 只报告，不阻断执行。抛出的栈会尽量映射回 `.ts` 行号。
`--preload`/`--require` 会在主脚本前通过 CommonJS 加载模块；文件型 preload 的相对 `require()` 以 preload 文件所在目录为基准。

### Inspector（Preview）

```bash
amber run --inspect app.js
amber run --inspect-brk --inspect-port 9229 app.ts
```

`--inspect` / `--inspect-brk` 在 `127.0.0.1:9229`（可用 `--inspect-port` 改）上提供 CDP：`GET /json/version`、`ws://127.0.0.1:9229/ws`。`--inspect-brk` 在收到 `Runtime.runIfWaitingForDebugger` 或 `Debugger.resume` 之前不执行用户脚本。`Runtime.evaluate` 在 isolate 上求值。详见 [DEBUGGER_USAGE.md](DEBUGGER_USAGE.md)。

## Eval

```bash
amber eval "1 + 1"
amber eval "console.log('hello')"
```

默认输出只包含用户代码输出或表达式结果，不打印内部初始化日志。

## 权限策略

`run`、`eval`、`test`、`bundle`、`debug`、`serve` 以及项目/包管理命令 `init`、`create`、`add`、`remove`、`install`、`prune`、`bunx`、`upgrade` 支持相同的最小权限参数：

```bash
amber eval --deny-fs "require('fs').readFileSync('secret.txt', 'utf8')"
amber run --deny-fs --allow-read config.json app.js
amber eval --deny-net --allow-net example.com "new WebSocket('wss://example.com/socket')"
amber eval --deny-env --allow-env PUBLIC_TOKEN "process.env.PUBLIC_TOKEN"
amber eval --deny-run --allow-run git "require('child_process').exec('git')"
amber eval --permission-policy bee.policy.json "process.env.PUBLIC_TOKEN"
amber bundle --deny-fs --allow-read src/index.js --allow-write dist/bundle.js src/index.js --outfile dist/bundle.js
bee debug --deny-fs --allow-read script.js script.js
bee create --deny-fs my-app js
bee add --deny-net lodash
bee install --permission-policy bee.policy.json
bee prune --deny-fs --allow-read package.json --allow-write .beejs_cache --allow-write node_modules
bee bunx --deny-run eslint
bee serve --deny-net --host 127.0.0.1 --port 3000
```

`--permission-policy` 也可以写作 `--policy`。策略文件支持 JSON，最小结构如下：

```json
{
  "permissions": {
    "deny_fs": true,
    "allow_read": ["./config.json"],
    "allow_write": ["./out"],
    "deny_net": true,
    "allow_net": ["api.example.com", "wss://api.example.com/socket"],
    "allow_listen": ["127.0.0.1", "http://127.0.0.1:3000"],
    "deny_env": true,
    "allow_env": ["PUBLIC_TOKEN"],
    "deny_run": true,
    "allow_run": ["git"]
  }
}
```

策略文件中的相对文件路径按策略文件所在目录解析。未传权限参数或策略文件时，当前默认仍是 allow-all 兼容模式。`run`、`test` 和 `bundle` 的入口源码读取也受 `FileSystem/Read` 约束；使用 `--deny-fs` 时需要对入口文件显式 `--allow-read`。`amber test` 无文件发现模式在扫描项目根目录和递归目录前同样检查 `FileSystem/Read`，避免被拒绝时回退执行内置 smoke tests。包管理命令的 registry 访问、`curl` 调用、`package.json` / lockfile 读写和 `node_modules` 扫描会进入同一套 broker；`bunx --deny-run` 会在下载或创建安装目录前按目标包名执行 `Process/Execute` 检查；`--allow-net` 只恢复 outbound `Network/Connect`，监听端口需显式 `--allow-listen` 或 policy `allow_listen`；`serve --deny-net` 会在报告 HTTP/HTTPS server configured 前按 `http(s)://host:port` 执行 `Network/Listen` 检查。

## REPL

```bash
amber repl
```

## 测试

```bash
amber test
amber test examples/testing/math.test.js
amber test examples/testing/math.test.js --test-name-pattern "adds"
amber test examples/testing/math.test.js --bail
amber test examples/testing/math.test.js --timeout 10
amber test examples/testing/math.test.js --update-snapshots
```

`--parallel` **不是**可用选项。传入时立即以退出码 **2** 失败，并说明 V8 isolate 不能跨线程共享；不会降级为串行成功。

`--update-snapshots` 会更新 file-mode 的 `expect(value).toMatchSnapshot()`，也会为缺失或不匹配的 `expect(value).toMatchInlineSnapshot()` 写回测试源文件。file snapshot 位于测试文件同目录的 `__snapshots__/<test-file>.snap`；snapshot 文件读取/写入和 inline snapshot 源文件写入都会进入文件系统权限 broker。

### 内置断言库 (Matchers)

`amber test` 内置支持以下 Jest 风格断言：

- **相等性比较**：`expect(a).toBe(b)`, `expect(a).toEqual(b)`, `expect(a).toStrictEqual(b)`
- **真值断言**：`expect(a).toBeTruthy()`, `expect(a).toBeFalsy()`
- **数值比较**：`expect(a).toBeGreaterThan(b)`, `expect(a).toBeLessThan(b)`, `expect(a).toBeGreaterThanOrEqual(b)`, `expect(a).toBeLessThanOrEqual(b)`
- **对象与属性**：`expect(obj).toHaveProperty("key")`, `expect(obj).toMatchObject({ key: val })`
- **异常捕获**：`expect(fn).toThrow()`

## Node.js & Web API 兼容说明

- **Node.js 模块**：支持 `fs` (`fs.promises`), `crypto`, `events`, `path`, `buffer`, `process`, `timers`, `http`, `net`, `os`, `url`, `querystring`, `stream`, `readline`, `child_process`, CommonJS `require`。
- **Web API 标准**：支持 `fetch` (带 Headers/Response/bodyUsed), `WebSocket`, `Web Crypto` (AES-GCM, AES-CBC, AES-CTR, ECDSA, ECDH, RSA, SHA-1/256/384/512, wrapKey/unwrapKey), `URL` / `URLSearchParams`, `Streams`, `TextEncoder` / `TextDecoder`, `Blob`, `FormData`, `AbortController`。

## Bundle

```bash
amber bundle src/index.js --outfile dist/bundle.js
amber bundle src/index.js --outfile dist/bundle.js --minify
amber bundle src/index.js --target browser --tree-shake
```

## Serve

```bash
bee serve --host localhost --port 3000
bee serve --host localhost --port 3443 --https --cert cert.pem --key key.pem
```

`--https` 使用 rustls 做 HTTP/1.1 TLS。必须同时提供存在的 `--cert` 与 `--key` PEM；缺文件或无法解析时以非 0 退出，不会打印成功监听横幅。不在本版本做 HTTP/2。无 `--https` 时行为仍是明文 HTTP。

## 项目与包管理

```bash
bee init my-app
bee create my-app js
bee create my-ts-app ts
bee add lodash
bee add lodash@4.17.21 --save-exact
bee add vitest --dev
bee install
bee prune
bee remove lodash
bee upgrade
bee bunx <package>
```

`bee create` 的当前参数顺序是 `<name> [template]`；历史文档中的 `bee create ts my-ts-app` 形式仍会被兼容为 TypeScript 模板项目。

包管理能力仍处于轻量实现阶段，遇到 npm 生态边界时应以实际命令结果为准。

## Watch

`run` 支持 watch 相关参数：

```bash
amber run app.js --watch
amber run app.js --watch --debounce 200
amber run app.js --watch --websocket-port 9999
```

## 调试

Chrome DevTools / VS Code 附加请用 `amber run --inspect` 或 `amber run --inspect-brk`（默认端口 9229），不要用 `bee debug`。见 [DEBUGGER_USAGE.md](DEBUGGER_USAGE.md)。

```bash
amber run --inspect-brk --inspect-port 9229 script.js
bee debug script.js
bee debug --deny-fs --allow-read script.js script.js
```

`bee debug` 是 Experimental：多打印诊断，不是 Inspector。`debug` 的目标文件读取会进入同一套文件系统权限 broker；使用 `--deny-fs` 时需要为目标脚本显式 `--allow-read`。

## 开发验证

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib
cargo test --test beejs_core_tests
cargo test --test cli_release_tests
cargo build --release
```

## 平台范围

v1.16.0 预编译包当前覆盖：

- macOS x86_64 (`bee-v<ver>-x86_64-apple-darwin.tar.gz`)
- macOS arm64 (`bee-v<ver>-aarch64-apple-darwin.tar.gz`)
- Linux x86_64 (`bee-v<ver>-x86_64-unknown-linux-gnu.tar.gz`)
- Linux aarch64 (`bee-v<ver>-aarch64-unknown-linux-gnu.tar.gz`)
- Windows x64 (`bee-v<ver>-x86_64-pc-windows-msvc.zip`)

Homebrew `Formula/amber.rb` SHA256 由 GitHub Release job 从上述 tar.gz 回写。容器镜像 `ghcr.io/zh30/amberjs` 仅为 linux/amd64。

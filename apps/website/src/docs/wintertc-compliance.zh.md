---
title: "WinterTC 合规"
subtitle: "Beejs 上的 ECMA-429、Runtime Keys 与 Sockets API"
group: "参考与规范"
id: "wintertc-compliance"
---

Beejs 实现 WinterTC（Ecma TC55，原 WinterCG）基线，使同一套 Web 平台代码可以在类 Node 与边缘运行时上运行。

## ECMA-429 最小公共 Web API

| API | 状态 |
|---|---|
| `DOMException` | 全局构造器，`name` / `message` / `code` 与遗留常量 |
| `globalThis.self` | 等于 `globalThis` |
| `reportError(error)` | 若设置了 `onerror` 则调用 |
| `PromiseRejectionEvent` | 未处理拒绝时派发到 `onunhandledrejection` |
| `navigator.userAgent` | `Beejs/{version}` |
| `navigator.hardwareConcurrency` / `language` / `platform` | 已提供 |
| `URLPattern` | `test()` / `exec()`，支持 `:param` |
| `ByteLengthQueuingStrategy` / `CountQueuingStrategy` | 全局 |
| `ReadableStream.from` | 支持可迭代与异步可迭代源 |

## Runtime Keys（ECMA TR-114）

`package.json` 的 `"exports"` 条件在 `node` 之前匹配 `wintercg` 与 `wintertc`。`import.meta.resolve` 使用同一解析器，因此 `"wintercg": "./winter.js"` 会优先于 `"node"`。

`import.meta.main` 标记 CLI 入口模块。`import.meta.env` 映射进程环境变量。

## Sockets API

```js
const { connect } = require('bee:sockets');
const socket = connect({ hostname: 'example.com', port: 443 }, { secureTransport: 'on' });
// socket.readable / socket.writable 是 Web Streams
// socket.startTls() 走真实 rustls 握手
```

`secureTransport: "on"` 与 `startTls()` 执行 rustls 客户端握手。对明文 TCP 端口做 TLS 会得到 TLS 错误，而不会静默保持明文。

## 测试

```bash
cargo test --test wintertc_compliance_tests -- --test-threads=1
```

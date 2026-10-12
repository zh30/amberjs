---
title: "WinterTC 基线"
nav: "WinterTC"
subtitle: "ECMA-429 最小通用 Web API、运行时键和 Sockets API。"
status: "stable"
id: "wintertc-compliance"
---

Amber 实现了 WinterTC（Ecma TC55，前身为 WinterCG）基线，使为边缘运行时编写的 Web 平台代码在 API 重叠的部分可以原样运行。该基线属于 `CURRENT_SCOPE.md` 中的 Stable 范围。

## ECMA-429 最小通用 Web API

| API | 行为 |
| :--- | :--- |
| `DOMException` | 全局构造函数，带 `name`、`message`、`code` 和旧版常量 |
| `globalThis.self` | `globalThis` 的别名 |
| `reportError(error)` | 设置了 `onerror` 时调用它 |
| `PromiseRejectionEvent` | 未处理的 rejection 通过 `onunhandledrejection` 派发 |
| `navigator.userAgent` | `Amber/<version>` |
| `navigator.hardwareConcurrency`、`language`、`platform` | 存在 |
| `URLPattern` | 支持 `:param` 分组的 `test()` 和 `exec()` |
| `ByteLengthQueuingStrategy`、`CountQueuingStrategy` | 全局构造函数 |
| `ReadableStream.from` | 可迭代与异步可迭代来源 |

## 运行时键

`package.json` 的 `"exports"` 条件中，`wintercg` 和 `wintertc` 优先于 `node`。`import.meta.resolve` 使用同一个解析器，因此发布了 `"wintercg": "./winter.js"` 的包会解析到该文件，而不是 `"node"` 入口。

CLI 入口模块的 `import.meta.main` 为 `true`。`import.meta.env` 映射进程环境变量。

## Sockets API

```js
const { connect } = require("amber:sockets");

const socket = connect(
  { hostname: "example.com", port: 443 },
  { secureTransport: "on" },
);
// socket.readable 和 socket.writable 是 Web 流。
```

`secureTransport: "on"` 和 `socket.startTls()` 会执行真实的 rustls 客户端握手。对明文 TCP 端口发起 TLS 连接会报 TLS 错误，而不是悄悄保持明文。

## 测试

```bash
cargo test --test wintertc_compliance_tests -- --test-threads=1
```

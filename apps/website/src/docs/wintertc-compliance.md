---
title: "WinterTC compliance"
subtitle: "ECMA-429, runtime keys, and Sockets API on Beejs"
group: "Reference & Specs"
id: "wintertc-compliance"
---

Beejs implements the WinterTC (Ecma TC55, formerly WinterCG) baseline so the same Web-platform code can run on Node-like and edge runtimes.

## ECMA-429 minimum common Web API

| API | Status |
|---|---|
| `DOMException` | Global constructor, `name` / `message` / `code`, legacy constants |
| `globalThis.self` | Alias of `globalThis` |
| `reportError(error)` | Calls `onerror` when set |
| `PromiseRejectionEvent` | Dispatched on unhandled rejections via `onunhandledrejection` |
| `navigator.userAgent` | `Beejs/{version}` |
| `navigator.hardwareConcurrency` / `language` / `platform` | Present |
| `URLPattern` | `test()` / `exec()` with `:param` groups |
| `ByteLengthQueuingStrategy` / `CountQueuingStrategy` | Global |
| `ReadableStream.from` | Iterable and async-iterable sources |

## Runtime keys (ECMA TR-114)

`package.json` `"exports"` conditions include `wintercg` and `wintertc` ahead of `node`. `import.meta.resolve` uses the same resolver, so a package that publishes `"wintercg": "./winter.js"` is selected instead of `"node"`.

`import.meta.main` is a boolean for the CLI entry module. `import.meta.env` maps process environment variables.

## Sockets API

```js
const { connect } = require('bee:sockets');
const socket = connect({ hostname: 'example.com', port: 443 }, { secureTransport: 'on' });
// socket.readable / socket.writable are Web Streams
// socket.startTls() performs a real rustls handshake
```

`secureTransport: "on"` and `startTls()` run a rustls client handshake. Connecting TLS to a plain TCP port fails with a TLS error; it does not silently stay in the clear.

## Tests

```bash
cargo test --test wintertc_compliance_tests -- --test-threads=1
```

---
title: "WinterTC baseline"
nav: "WinterTC"
subtitle: "ECMA-429 minimum common Web API, runtime keys, and the Sockets API."
status: "stable"
id: "wintertc-compliance"
---

Amber implements the WinterTC (Ecma TC55, formerly WinterCG) baseline so Web-platform code written for edge runtimes can run unchanged where the APIs overlap. This baseline is part of the Stable scope in `CURRENT_SCOPE.md`.

## ECMA-429 minimum common Web API

| API | Behavior |
| :--- | :--- |
| `DOMException` | Global constructor with `name`, `message`, `code`, and legacy constants |
| `globalThis.self` | Alias of `globalThis` |
| `reportError(error)` | Calls `onerror` when set |
| `PromiseRejectionEvent` | Dispatched on unhandled rejections through `onunhandledrejection` |
| `navigator.userAgent` | `Amber/<version>` |
| `navigator.hardwareConcurrency`, `language`, `platform` | Present |
| `URLPattern` | `test()` and `exec()` with `:param` groups |
| `ByteLengthQueuingStrategy`, `CountQueuingStrategy` | Global constructors |
| `ReadableStream.from` | Iterable and async-iterable sources |

## Runtime keys

`package.json` `"exports"` conditions include `wintercg` and `wintertc` ahead of `node`. `import.meta.resolve` uses the same resolver, so a package that publishes `"wintercg": "./winter.js"` resolves to that file instead of its `"node"` entry.

`import.meta.main` is `true` for the CLI entry module. `import.meta.env` maps process environment variables.

## Sockets API

```js
const { connect } = require("amber:sockets");

const socket = connect(
  { hostname: "example.com", port: 443 },
  { secureTransport: "on" },
);
// socket.readable and socket.writable are Web streams.
```

`secureTransport: "on"` and `socket.startTls()` run a real rustls client handshake. Connecting with TLS to a plain TCP port fails with a TLS error instead of silently staying in cleartext.

## Tests

```bash
cargo test --test wintertc_compliance_tests -- --test-threads=1
```

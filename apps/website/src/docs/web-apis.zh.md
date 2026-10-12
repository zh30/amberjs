---
title: "Web API"
nav: "Web API"
subtitle: "Amber 用契约固定的 Web 平台全局对象，以及刻意不做的部分。"
status: "stable"
id: "web-apis"
---

下表每一行只对其契约是 Stable，`src/web_api/` 的其余部分都是 Preview。Amber 中没有 document、DOM 树或浏览器 origin，所以不少 API 的行为更接近服务端版本，而不是浏览器版本。

## 网络

| API | 编号 | 说明 |
| :--- | :--- | :--- |
| `fetch`、`Request`、`Response`、`Headers` | G9 | 流式 body reader、重定向模式、`AbortSignal`、`integrity`。`text()` 和 `json()` 是同步的。`response.body` 不是 `ReadableStream` 实例。 |
| `WebSocket` 客户端 | G15 | `ws://` 握手，文本与二进制帧。没有子协议和 `permessage-deflate`，不是服务端。 |

## 数据与编码

| API | 编号 | 说明 |
| :--- | :--- | :--- |
| `URL`、`URLSearchParams` | G10 | 不支持 IDNA。 |
| `TextEncoder`、`TextDecoder` | G10 | 仅 UTF-8。 |
| `structuredClone` | G10 | 契约中列出的类型，包括 `ArrayBuffer` 转移。 |
| `atob`、`btoa` | G29 | 抛出 `Error`，不是 `DOMException`。 |
| `Blob`、`File`、`FormData` | G11 | `text()` 和 `arrayBuffer()` 直接返回值，而不是 Promise。 |
| `crypto.subtle`、`crypto.randomUUID`、`getRandomValues` | G14 | SHA、HMAC、AES、PBKDF2、ECDH、Ed25519、ECDSA、RSA。不支持 HKDF、RSA-PSS 和 X25519。 |
| `DOMParser` | G32 | HTML 与 XML 的只读解析树，不是活动 DOM。 |

## 事件、流与定时器

| API | 编号 | 说明 |
| :--- | :--- | :--- |
| `Event`、`EventTarget`、`CustomEvent`、`AbortController` | G12 | 没有树结构，事件传播不会跨越父节点。 |
| `MessageChannel`、`BroadcastChannel` | G12 | 同一 isolate 内投递。 |
| `ErrorEvent`、全局 `onerror` | G30（provisional） | 构造出的事件不是 `instanceof Event`。 |
| `ReadableStream`、`WritableStream`、`TransformStream` | G13 | 排队策略不提供背压。 |
| `CompressionStream`、`DecompressionStream` | G13 | gzip、deflate、deflate-raw；每次 `write` 一个 member。 |
| `setTimeout`、`setInterval`、`queueMicrotask`、`performance` | G13 | `now`、`mark`、`measure`。 |

## Worker 与 Service Worker

| API | 编号 | 说明 |
| :--- | :--- | :--- |
| 专用 `Worker`、`worker_threads` | G16 | 每个 worker 一个线程和 isolate，消息为 JSON。 |
| `navigator.serviceWorker.register` | G16 | install 与 activate 生命周期，`clients.claim()`。 |
| Service Worker `fetch` 拦截 | G35 | 已激活 worker 中的 `FetchEvent.respondWith`。 |
| `caches`（Cache Storage） | G34 | 进程内内存存储，不是 HTTP 缓存，也不持久化。 |
| Background Sync | G33 | 在同一 isolate 上触发，不是 Periodic Sync。 |
| `navigator.clipboard.writeText` / `readText` | G31 | 进程内字符串，不是系统剪贴板。 |

## WinterTC 基线

`DOMException`、`URLPattern`、`navigator`、排队策略、`ReadableStream.from`、`amber:sockets`、`wintercg` / `wintertc` 导出条件，以及 `import.meta.main` / `env` / `resolve` 属于 Stable 范围。详见 [WinterTC](/docs/wintertc-compliance)。

## 不支持

- `SharedWorker` 和模块 worker（`{ type: "module" }` 会抛错）。
- Push 订阅、`Notification` 和 Payment Request API。
- `ClipboardItem` 的 read 与 write（会 reject，错误信息中包含 `ClipboardItem`）。
- `CacheStorage.match`、`matchAll` 和 `ignore*` 选项。

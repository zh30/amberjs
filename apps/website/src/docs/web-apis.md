---
title: "Web APIs"
nav: "Web APIs"
subtitle: "The Web-platform globals Amber pins with contracts, and the ones it deliberately leaves out."
status: "stable"
id: "web-apis"
---

Each row below is Stable only for its contract. The rest of `src/web_api/` is Preview. There is no document, DOM tree, or browser origin in Amber, so several APIs behave like their server-side cousins rather than the browser versions.

## Networking

| API | Id | Notes |
| :--- | :--- | :--- |
| `fetch`, `Request`, `Response`, `Headers` | G9 | Streaming body reader, redirect modes, `AbortSignal`, `integrity`. `text()` and `json()` are synchronous. `response.body` is not a `ReadableStream` instance. |
| `WebSocket` client | G15 | `ws://` handshake, text and binary frames. No subprotocols or `permessage-deflate`. Not a server. |

## Data and encoding

| API | Id | Notes |
| :--- | :--- | :--- |
| `URL`, `URLSearchParams` | G10 | No IDNA. |
| `TextEncoder`, `TextDecoder` | G10 | UTF-8 only. |
| `structuredClone` | G10 | The types in the contract, including `ArrayBuffer` transfer. |
| `atob`, `btoa` | G29 | Throws `Error`, not `DOMException`. |
| `Blob`, `File`, `FormData` | G11 | `text()` and `arrayBuffer()` return values, not Promises. |
| `crypto.subtle`, `crypto.randomUUID`, `getRandomValues` | G14 | SHA, HMAC, AES, PBKDF2, ECDH, Ed25519, ECDSA, RSA. No HKDF, RSA-PSS, or X25519. |
| `DOMParser` | G32 | Read-only parse tree for HTML and XML. Not a live DOM. |

## Events, streams, and timers

| API | Id | Notes |
| :--- | :--- | :--- |
| `Event`, `EventTarget`, `CustomEvent`, `AbortController` | G12 | No tree, so propagation does not cross parents. |
| `MessageChannel`, `BroadcastChannel` | G12 | Same-isolate delivery. |
| `ErrorEvent`, global `onerror` | G30 (provisional) | Constructed events are not `instanceof Event`. |
| `ReadableStream`, `WritableStream`, `TransformStream` | G13 | No backpressure from queuing strategies. |
| `CompressionStream`, `DecompressionStream` | G13 | gzip, deflate, deflate-raw; one member per `write`. |
| `setTimeout`, `setInterval`, `queueMicrotask`, `performance` | G13 | `now`, `mark`, `measure`. |

## Workers and service workers

| API | Id | Notes |
| :--- | :--- | :--- |
| Dedicated `Worker`, `worker_threads` | G16 | One thread and isolate per worker. Messages are JSON. |
| `navigator.serviceWorker.register` | G16 | Install and activate lifecycle, `clients.claim()`. |
| Service-worker `fetch` interception | G35 | `FetchEvent.respondWith` from an activated worker. |
| `caches` (Cache Storage) | G34 | In-process memory store. Not an HTTP cache and not persistent. |
| Background Sync | G33 | Fires on the same isolate. Not Periodic Sync. |
| `navigator.clipboard.writeText` / `readText` | G31 | Process-local string, not the OS clipboard. |

## WinterTC baseline

`DOMException`, `URLPattern`, `navigator`, queuing strategies, `ReadableStream.from`, `amber:sockets`, `wintercg` / `wintertc` export conditions, and `import.meta.main` / `env` / `resolve` are part of the Stable scope. See [WinterTC](/docs/wintertc-compliance).

## Not supported

- `SharedWorker` and module workers (`{ type: "module" }` throws).
- Push subscriptions, `Notification`, and the Payment Request API.
- `ClipboardItem` read and write (rejects with an error naming `ClipboardItem`).
- `CacheStorage.match`, `matchAll`, and the `ignore*` options.

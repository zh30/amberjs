# Amber v1.9.0 Release Notes: WinterTC baseline, TLS sockets, and runtime release infrastructure

> **Release Tag**: `v1.9.0`  
> **Release Type**: Minor Release  
> **Date**: 2026-09-09

---

## Overview

Amber **v1.9.0** closes the WinterTC baseline on the default runtime and ships the GitHub Release / CI / install matrix a language runtime needs.

1. **WinterTC (ECMA-429 / TR-114)**: `DOMException`, `URLPattern`, `navigator`, queuing strategies, `ReadableStream.from`, `amber:sockets`, `wintercg`/`wintertc` export conditions, and `import.meta.main` / `env` / `resolve`.
2. **TLS sockets**: rustls client with Mozilla webpki roots. `secureTransport: "on"` and `startTls()` perform a real handshake. Untrusted certificates fail; they do not silently stay in the clear.
3. **Release infrastructure**: `v*` tags publish linux gnu x64/arm64, macOS arm64/x64, and Windows x64 archives, with SHA-256 checksums, CycloneDX SBOM, and cosign signatures. `install.sh` maps Darwin/Linux x64 and arm64; Windows uses `install.ps1`. Homebrew formula lives at `Formula/amber.rb`. GHCR publishes `ghcr.io/zh30/amberjs`.

---

## Runtime

```js
const { connect } = require('amber:sockets');
const socket = connect({ hostname: 'example.com', port: 443 }, { secureTransport: 'on' });
await socket.opened;
```

- `import.meta.resolve('pkg')` uses the same ESM resolver as `require` / import, so `"wintercg"` exports beat `"node"`.
- Unhandled promise rejections dispatch `PromiseRejectionEvent` on `onunhandledrejection`.

---

## Install

```bash
curl -fsSL https://amber.zhanghe.dev/install.sh | sh
brew install zh30/tap/amber
```

Windows:

```powershell
irm https://amber.zhanghe.dev/install.ps1 | iex
```

---

## Compatibility

v1.9.0 is additive. v1.8.0 `amber:bus`, `amber:grammar`, and `amber:checkpoint` APIs are unchanged.

Live GitHub Release assets, GHCR images, and Homebrew bottle hashes appear after the `v1.9.0` tag is pushed.

# Amber v1.9.1 Release Notes: infrastructure second wave

> **Release Tag**: `v1.9.1`  
> **Release Type**: Patch  
> **Date**: 2026-09-10

v1.9.1 closes the second-wave runtime supporting facilities from [IMPLEMENTATION_PLAN_v1.9.1.md](./IMPLEMENTATION_PLAN_v1.9.1.md).

## Highlights

- Windows MSVC is a required Release target (`amber-v1.9.1-x86_64-pc-windows-msvc.zip` containing `amber.exe`).
- `amber serve --https` speaks HTTP/1.1 over rustls. Missing `--cert`/`--key` exits non-zero.
- `amber run --inspect-brk` waits for DevTools, serves `GET /json/version`, and evaluates `Runtime.evaluate` on the isolate.
- `process.dlopen` calls `napi_register_module_v1` for a C hello addon. Experimental; not Prisma/sharp.
- TypeScript stacks name `.ts` lines. `amber test --parallel` exits 2.
- rustc 1.97.1, fail-closed `cargo-audit`, `cargo deny`. GHCR is linux/amd64 only.
- Homebrew formula hashes are rewritten from GitHub Release archives. V8 stays `rusty_v8` 0.22.

## Install

```bash
curl -fsSL https://amber.zhanghe.dev/install.sh | sh
brew install zh30/tap/amber
```

Windows:

```powershell
irm https://amber.zhanghe.dev/install.ps1 | iex
```

GHCR: `ghcr.io/zh30/amberjs:1.9.1` (linux/amd64 only).

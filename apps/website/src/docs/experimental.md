---
title: "Experimental surfaces"
nav: "Experimental"
subtitle: "Commands, flags, and modules that ship in the binary but are not product promises yet."
status: "experimental"
id: "experimental"
---

The `amber` binary contains more than the Stable scope. This page lists what else is there, so nothing is hidden. None of it is promised. Each item needs fresh verification before it can graduate under the [graduation rule](/docs/stability).

## Classified as Experimental

These are listed as Experimental in `CURRENT_SCOPE.md`:

| Surface | Notes |
| :--- | :--- |
| `amber debug` | Older debug entry. Prefer `amber run --inspect`. |
| `amber init`, `amber create` | Project scaffolding |
| `amber add`, `amber remove`, `amber prune`, `amber upgrade` | Package edits outside the `amber install` contract |
| `amber x` (`dlx`, `bunx`) | Run a package binary without installing |
| `amber test --parallel` | Rejected with exit code 2 |
| N-API hello loader | `process.dlopen` can load a C hello addon. Not a Node ABI commitment; Prisma and sharp are out of scope. |
| Optional Cargo features | `benchmarks` and `observability` compile in CI. `cloudnative`, `enterprise`, `multilang`, and `tch` are not CI-gated. The `ai` feature is empty. |
| Homebrew, GHCR, winget | Distribution channels. The GHCR image is linux/amd64 only; the winget manifest is not submitted. |

## Present but not classified

These exist in `src/main.rs` or as `amber:*` modules, but the scope page does not classify them. This manual treats them as Experimental:

| Surface | What it claims to do |
| :--- | :--- |
| `amber fmt`, `amber lint` | Format and lint JS and TS files |
| `amber bench` | Run microbenchmarks |
| `amber task` | Run a `package.json` script |
| `amber types` | Print type definitions for Amber built-ins |
| `amber lsp` | Language Server Protocol server |
| `amber profile` | Write a `.cpuprofile` |
| `amber deploy` | Generate Docker, standalone, or Kubernetes scaffolding |
| `amber record`, `amber replay` | Record and replay an execution trace |
| `--import-map` | Remap bare specifiers from a JSON import map |
| `--virtual-fs` | In-memory filesystem overlay |
| `amber:mcp` | JavaScript `McpServer` API |
| `amber:kv`, `amber:db`, `amber:vector` | Embedded storage helpers |
| `amber:bus`, `amber:grammar`, `amber:checkpoint`, `amber:weights`, `amber:tools` | Agent helper modules |
| `amber:security`, `amber:std`, `amber:pool`, `amber:ffi` | Assorted host modules |

Earlier versions of this site documented several of these as finished features, often with speed claims. Those pages were removed in the v1.17 docs rewrite because the claims were never re-verified. Old links redirect here.

## Want one of these to graduate?

Open an issue at [github.com/zh30/amberjs/issues](https://github.com/zh30/amberjs/issues) with your use case. Graduation needs a written contract, executable tests in CI, and documented limits.

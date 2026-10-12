---
title: "Introduction"
nav: "Introduction"
subtitle: "Amber is a JavaScript and TypeScript runtime built with Rust and V8. This manual covers v1.17.0."
status: "stable"
id: "introduction"
---

Amber runs JavaScript and TypeScript files from a single `amber` binary. It ships a test runner, a bundler, a single-file compiler, a small package installer, an HTTP serve command, and a permission sandbox for running agent tools.

Amber is open source under the MIT license. The source lives at [github.com/zh30/amberjs](https://github.com/zh30/amberjs).

## What Amber is

- **A V8 runtime written in Rust.** The CLI runs scripts on V8 through `src/runtime_minimal.rs`. TypeScript and TSX are transpiled with oxc before they run.
- **One binary, many commands.** `amber run`, `amber test`, `amber bundle`, `amber compile`, `amber install`, and `amber serve` all live in the same executable.
- **A Node.js and Web API surface with written contracts.** Each Stable API has a contract file in `docs/` and an integration test in `tests/`. The contract lists what works and where it stops.
- **A host for agent tools.** `amber run --sandbox` denies file, network, environment, and process access unless you allow it. `amber session` and `amber mcp` call exported functions from a tool file over JSON-RPC.

## What Amber is not

Amber is **not** a drop-in replacement for Node.js, Bun, or Deno. Many Node modules are partial, and several are Preview. Use the [compatibility page](/compatibility) to check an API before you depend on it.

Amber does not publish performance numbers today. Older benchmark tables in the repository are historical and were not re-run for this release.

## How to read this manual

Every capability carries one of four labels:

| Label | Meaning |
| :--- | :--- |
| **Stable** | Pinned by a contract document and an executable test that runs in CI. Limits are listed next to the API. |
| **Preview** | In the default build and usable, but the contract or edge cases are still being tightened. |
| **Experimental** | Code exists, but it is not a product promise until it is re-verified. |
| **Not supported** | Deliberately out of scope. Amber fails closed instead of pretending. |

The single source of truth for these labels is [`docs/CURRENT_SCOPE.md`](https://github.com/zh30/amberjs/blob/main/docs/CURRENT_SCOPE.md). If this site and that file disagree, the file wins.

## Next steps

- [Install Amber](/docs/installation)
- [Run your first script](/docs/quick-start)
- [Browse the CLI](/docs/cli-usage)
- [Check Node.js compatibility](/docs/node-compat)

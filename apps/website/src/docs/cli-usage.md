---
title: "CLI reference"
nav: "CLI reference"
subtitle: "Every amber subcommand in v1.17.0, grouped by maturity. Checked against src/main.rs."
status: "stable"
id: "cli-usage"
---

Run `amber --help` or `amber <command> --help` for the full flag list on your build.

## Stable commands

| Command | What it does | Contract |
| :--- | :--- | :--- |
| `amber run <file> [args...]` | Run a JS or TS file | — |
| `amber eval <code>` | Evaluate a snippet and print the result | — |
| `amber repl` | Interactive REPL | — |
| `amber test [file]` | Discover and run Jest-style tests | [Testing](/docs/testing) |
| `amber bundle <entry> -o <out>` | Pack a local module graph into one JS file | [`BUNDLE_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/BUNDLE_CONTRACT.md) |
| `amber compile <entry> -o <bin>` | Build a single executable for this host | [`COMPILE_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/COMPILE_CONTRACT.md) |
| `amber install [--frozen-lockfile]` | Install direct dependencies from `package.json` | [`INSTALL_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/INSTALL_CONTRACT.md) |
| `amber serve [file]` | Serve a fetch handler over HTTP/1.1 | [`SERVE_HTTP_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/SERVE_HTTP_CONTRACT.md) |
| `amber serve --https --cert --key` | Same, over rustls TLS | [`SERVE_HTTPS_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/SERVE_HTTPS_CONTRACT.md) |
| `amber session <file>` | JSON-RPC tool session over stdin and stdout | [MCP & sessions](/docs/mcp-protocol) |
| `amber mcp <file>` | MCP stdio server for a tool file | [MCP & sessions](/docs/mcp-protocol) |
| `amber snapshot build\|status\|clean` | Manage the V8 startup snapshot | — |
| `amber version` / `amber --version` | Print the version | — |

## Stable `amber run` flags

| Flag | What it does | Contract |
| :--- | :--- | :--- |
| `--watch`, `-w` | Re-run the entry when a watched file changes | [`WATCH_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/WATCH_CONTRACT.md) |
| `--inspect`, `--inspect-brk`, `--inspect-port` | A small CDP endpoint on `127.0.0.1` | [`INSPECT_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/INSPECT_CONTRACT.md) |
| `--export-tools` | Print tool schemas as JSON and exit, without running the file | [`EXPORT_TOOLS_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/EXPORT_TOOLS_CONTRACT.md) |

`--watch` restarts the whole process after the previous run returns. It is not hot module replacement.

## Permission and determinism flags

These flags work on `run`, `test`, `serve`, `session`, `mcp`, and the other commands that execute code.

| Flag | What it does |
| :--- | :--- |
| `--sandbox` | Deny file, network, environment, and child-process access by default |
| `--allow-read <path>` / `--allow-write <path>` | Allow one exact path (repeatable) |
| `--allow-net <host>` / `--allow-listen <host>` | Allow outbound connections or listeners for a host or URL |
| `--allow-env <NAME>` / `--allow-run <command>` | Allow one variable or one command |
| `--deny-fs`, `--deny-net`, `--deny-env`, `--deny-run` | Deny one capability without the full sandbox |
| `--permission-policy <file>` | Load allow and deny rules from JSON |
| `--audit-log <file>` | Append each permission decision as JSONL |
| `--seed <u64>` | Seed `Math.random`, `crypto.getRandomValues`, and `crypto.randomBytes` |
| `--freeze-time <spec>` | Freeze `Date.now`, `new Date()`, and `performance.now` |
| `--timeout <ms>` / `--max-memory <MB>` | Stop runaway scripts |

See [Agent sandbox](/docs/agent-sandbox) for examples.

## Experimental commands

These commands exist, but they are outside the Stable scope. Expect rough edges.

| Command | Notes |
| :--- | :--- |
| `amber debug <file>` | Older debug entry point. Use `amber run --inspect` instead. |
| `amber init`, `amber create` | Project scaffolding |
| `amber add`, `amber remove`, `amber prune`, `amber upgrade` | Package edits outside the `amber install` contract |
| `amber x <pkg>` (aliases `dlx`, `bunx`) | Run a package binary without installing |
| `amber test --parallel` | Rejected with exit code 2. V8 isolates are not shared across threads. |

## Present but not classified

The binary also includes these commands. `CURRENT_SCOPE.md` does not classify them yet, so this manual treats them as Experimental: `amber fmt`, `amber lint`, `amber bench`, `amber task`, `amber types`, `amber lsp`, `amber profile`, `amber deploy`, `amber record`, and `amber replay`, plus the `--import-map` and `--virtual-fs` flags. See [Experimental](/docs/experimental).

## Exit codes

Contracted failures print a prefix such as `error: amber serve:` or `error: amber install:` and exit non-zero. Misuse of a flag, such as `amber test --parallel`, exits 2.

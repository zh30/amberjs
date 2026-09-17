---
title: "CLI reference"
subtitle: "What bee actually ships in v1.16.0 — Stable, Preview, Experimental"
group: "Reference & Specs"
id: "cli-usage"
---

`bee --help` is the source of truth. This page groups the same commands by maturity. See [Current Scope](https://github.com/zh30/beejs/blob/main/docs/CURRENT_SCOPE.md) in the repo.

`--verbose` is global and must come **before** the subcommand: `bee --verbose run app.js`.

---

## Stable

| Command | What it does |
| :--- | :--- |
| `bee run <file> [args...]` | Run JS. `.ts` / `.tsx` go through oxc first (TS contract is Preview). |
| `bee eval <code>` | Evaluate an expression |
| `bee repl` | Interactive REPL |
| `bee test [files...] [--watch]` | Jest-style runner |
| `bee snapshot [build\|status\|clean]` | V8 startup snapshots |
| `bee session <tool>` | JSON-RPC over stdin for agent hosts |
| `bee mcp [tool]` | MCP stdio server |
| `bee --version` / `bee version` | Version |

### `bee run`

```bash
bee run app.ts
bee run app.js -- arg1 arg2
bee run --watch --debounce 200 app.ts
bee run --preload ./setup.js app.js
bee run --sandbox --permission-policy policy.json app.ts
bee run --inspect-brk app.ts
```

Useful flags:

| Flag | Purpose |
| :--- | :--- |
| `-w, --watch` | Restart on change |
| `--debounce <ms>` | Watch debounce (default 100) |
| `-r, --preload <module>` | Load before the entry |
| `--timeout <ms>` | CPU watchdog |
| `--max-memory <mb>` | V8 heap cap |
| `--seed <u64>` | Deterministic `Math.random` |
| `--freeze-time <spec>` | Freeze `Date.now` / `performance.now` |
| `--sandbox` | Deny fs / net / env / run, then `--allow-*` |
| `--permission-policy <file>` | JSON policy (alias `--policy`) |
| `--inspect` / `--inspect-brk` | CDP on `127.0.0.1:9229` (Preview) |

`bee test --parallel` is rejected (exit code 2).

---

## Preview

Present in the default binary; the contract is still tightening.

| Command | What it does |
| :--- | :--- |
| `bee serve [file]` | WinterCG `fetch` handler. `--https --cert --key` is rustls HTTP/1.1. |
| `bee bundle <entry>` | oxc module graph → one JS file |
| `bee compile <file>` | Append payload + `BEE_STANDALONE` trailer to a copy of `bee` |
| TypeScript / TSX | oxc type-strip, not `tsc` |
| `--inspect` / `--inspect-brk` | CDP `Runtime.evaluate` |

```bash
bee serve app.js --host 127.0.0.1 --port 3000
bee bundle src/index.ts -o dist/bundle.js --minify
bee compile app.ts -o myapp
```

Details: [bundle & compile](/docs/bundling-compilation).

---

## Experimental

Do **not** treat these as product promises. They exist on the CLI; behavior may be incomplete.

`debug`, `record`, `replay`, `init`, `create`, `add`, `remove`, `install`, `prune`, `x`, `upgrade`, `fmt`, `lint`, `bench`, `compile` extras, `types`, `task`, `profile`, `lsp`, `deploy`.

Chrome DevTools attach should use `bee run --inspect`, not `bee debug`.

---

## Permission flags

Default is still allow-all unless `--sandbox` or `--deny-*` is set.

```bash
bee eval --deny-fs "require('fs').readFileSync('secret.txt', 'utf8')"
bee run --deny-fs --allow-read config.json app.js
bee eval --deny-net --allow-net example.com "fetch('https://example.com')"
```

Policy file (relative paths resolve from the policy file directory):

```json
{
  "permissions": {
    "deny_fs": true,
    "allow_read": ["./config.json"],
    "allow_write": ["./out"],
    "deny_net": true,
    "allow_net": ["api.example.com"],
    "deny_env": true,
    "allow_env": ["PUBLIC_TOKEN"],
    "deny_run": true,
    "allow_run": ["git"]
  }
}
```

More: [agent sandbox](/docs/agent-sandbox).

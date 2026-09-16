<p align="center">
  <a href="https://bee.zhanghe.dev"><img src="https://bee.zhanghe.dev/logo.png" alt="Beejs" height="96"></a>
</p>

<h1 align="center">Beejs</h1>

<p align="center">
  A JavaScript and TypeScript runtime in <b>Rust</b> and <b>V8</b>.<br>
  One binary: <code>bee</code>. Built for agent tools, not as a Node.js clone.
</p>

<p align="center">
  <a href="https://bee.zhanghe.dev"><img src="https://img.shields.io/badge/docs-bee.zhanghe.dev-0f172a" alt="Docs"></a>
  <a href="https://github.com/zh30/beejs/releases/tag/v1.16.0"><img src="https://img.shields.io/badge/release-v1.16.0-22c55e" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-yellow" alt="License"></a>
  <a href="https://github.com/zh30/beejs/actions/workflows/ci.yml"><img src="https://github.com/zh30/beejs/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
</p>

<p align="center">
  <a href="https://bee.zhanghe.dev">Website</a>
  ·
  <a href="https://bee.zhanghe.dev/docs">Docs</a>
  ·
  <a href="docs/CURRENT_SCOPE.md">Current scope</a>
  ·
  <a href="https://github.com/zh30/beejs/releases">Releases</a>
  ·
  <a href="https://github.com/zh30/beejs/issues">Issues</a>
</p>

[中文文档](https://bee.zhanghe.dev/zh)

---

## What is this?

`bee` runs `.js` / `.ts` / `.tsx` on V8. TypeScript is stripped by [oxc](https://oxc.rs/) — there is no project-wide `tsc` step.

Use it when you want **one binary** that can:

- run scripts and a REPL
- run Jest-style tests
- host agent tools behind a capability sandbox (`--sandbox`, `--seed`, `--freeze-time`)
- speak MCP / JSON-RPC over stdio
- do in-process tensors via `bee:ai` (no Python sidecar)

It is **not** a drop-in Node.js replacement. APIs land incrementally and are scored in `tests/conformance/` (**55/55** on Node.js Conformance 5.0). Closest cousins: [Deno](https://github.com/denoland/deno) (V8 + Rust, permissioned) and [Bun](https://github.com/oven-sh/bun) (all-in-one CLI). Beejs keeps V8, adds an agent/MCP host, and labels every command Stable / Preview / Experimental.

The only user-facing capability map is [Current Scope](docs/CURRENT_SCOPE.md). Historical `docs/STAGE_*` numbers are not current facts.

---

## Install

Prebuilt archives: macOS (arm64, x64), Linux gnu (x64, arm64), Windows (x64 zip).

```sh
# macOS / Linux
curl -fsSL https://bee.zhanghe.dev/install.sh | sh

# pin a release
curl -fsSL https://bee.zhanghe.dev/install.sh | BEEJS_VERSION=v1.16.0 sh
```

Windows (PowerShell):

```powershell
irm https://bee.zhanghe.dev/install.ps1 | iex
```

Homebrew (formula in this repo; SHA256 is rewritten on each GitHub Release):

```sh
brew install zh30/tap/bee
```

```sh
bee --version
bee eval "1 + 1"
```

### Build from source

Needs [Rust](https://rustup.rs/) (**1.97.1**, pinned) and a C++ toolchain for V8.

```sh
git clone https://github.com/zh30/beejs.git
cd beejs
cargo build --release
./target/release/bee --version
```

---

## 60-second tour

```ts
// hello.ts
const runtime = "Beejs";
console.log(`hello from ${runtime}`);
```

```sh
bee run hello.ts
# hello from Beejs

bee eval "console.log(crypto.randomUUID())"
bee repl
```

Thrown stacks map back to `.ts` lines when a source map is present. There is still no `tsc --noEmit` in the runtime; use that in CI if you want typechecking.

Jest-style tests (Stable since v1.15.0):

```js
// math.test.js
describe("math", () => {
  test("adds numbers", () => {
    expect(2 + 3).toBe(5);
  });
});
```

```sh
bee test
bee test examples/testing/math.test.js
bee test --watch
```

`bee test --parallel` is rejected (exit code 2): V8 isolates are not shared across threads.

WinterCG-style HTTP (Preview):

```js
// app.js
module.exports = {
  fetch() {
    return new Response("ok");
  },
};
```

```sh
bee serve app.js --host 127.0.0.1 --port 3000
```

---

## Agents, sandbox, MCP

Default-deny I/O for tool processes, deterministic clocks/PRNG, stdio MCP.

```sh
bee run --sandbox --permission-policy examples/agent/echo.policy.json \
  --export-tools examples/agent/echo_tool.ts

bee session --sandbox --permission-policy examples/agent/echo.policy.json \
  examples/agent/echo_tool.ts

bee mcp --inspect examples/agent/echo_tool.ts
```

| Flag | Purpose |
| --- | --- |
| `--sandbox` | Deny fs / net / env / run, then overlay `--allow-*` |
| `--permission-policy <file>` | JSON policy (alias `--policy`) |
| `--audit-log <path>` | JSONL of allow/deny decisions |
| `--seed <u64>` | Deterministic `Math.random` / `crypto.getRandomValues` |
| `--freeze-time <spec>` | Freeze `Date.now` / `performance.now` |
| `--inspect` / `--inspect-brk` | CDP on `127.0.0.1:9229` |

---

## `bee:ai`

Stable builtins — no native addon, no Python process:

```ts
import { Tensor, LLM, AgentPipeline } from "bee:ai";

const a = Tensor.from([1, 2, 3, 4], [2, 2]);
const b = Tensor.from([5, 6, 7, 8], [2, 2]);
const c = Tensor.matmul(a, b);
```

`LLM` (`load`, `generate`, `generateStream`, `embed`) and `AgentPipeline` are in the [manual](https://bee.zhanghe.dev/docs). Cargo `feature = "ai"` is empty and is **not** a product LLM.

---

## CLI

**Stable** — everyday commands:

```text
bee run <file> [args...]     Run JS (TS/TSX via oxc)
bee eval <code>              Evaluate an expression
bee test [files...] [--watch]
bee repl
bee snapshot [build|status|clean]
bee session <tool>           JSON-RPC over stdin
bee mcp [tool]               MCP stdio server
bee --version | bee version
```

**Preview** — present, contract still tightening:

```text
bee serve [--https --cert --key]
bee bundle <entry> [-o dist/bundle.js] [--minify]
bee compile <file> [-o myapp]
bee run --inspect / --inspect-brk
TypeScript / TSX execution
```

**Experimental** — do not treat as product promises: `bee debug`, `bee init` / `create` / `add` / `remove` / `install` / `prune` / `x` / `upgrade`, N-API hello `process.dlopen`, `fmt` / `lint` / `task` / `bench` / `profile` / `deploy`.

Full flags: [CLI usage guide](docs/CLI_USAGE_GUIDE.md).

---

## Compatibility

| | Beejs 1.16.0 | Node.js | Bun | Deno |
| --- | --- | --- | --- | --- |
| Engine | V8 + Rust | V8 + C++ | JavaScriptCore + Zig | V8 + Rust |
| TypeScript | oxc, transpile-only | loaders / `tsc` | built-in | built-in |
| Secure defaults | opt-in `--sandbox` | none | none | permission flags |
| Node API | incremental Preview | native | drop-in goal | compat layer |
| Package manager | Experimental | npm | `bun` | `deno` / JSR |
| Test runner | built-in `bee test` | external | `bun test` | `deno test` |
| Native AI | `bee:ai` | — | — | — |

Node modules that exist today include `fs`, `path`, `os`, `url`, `buffer`, `events`, `stream`, `crypto`, `http`, `http2`, `net`, `child_process` (`execSync` / `spawnSync`), `zlib`, `util`, `worker_threads`. Web: `fetch`, Streams, Web Crypto, URL, `Worker`. **Coverage is per-API**, not “Node compatible.” WinterTC baseline: `DOMException`, `URLPattern`, `ReadableStream.from`, `bee:sockets`, `import.meta.main`.

On Apple M2 Max (2026-09-16), suite 2.0 URL / `fetch` / `ReadableStream` beat Node.js; Express on Beejs was ~68k req/s vs Node ~19k. Numbers live in [`benchmarks/`](benchmarks/) with command, hardware, and a correctness check. This README does not reprint them as product SLAs.

---

## Editors

- **VS Code**: [tools/vscode-extension](tools/vscode-extension) — `bee lsp` + inspector attach. Install the local `.vsix`; Marketplace is not shipped.
- **Zed**: [tools/zed-extension](tools/zed-extension) — Install Dev Extension; `bee` must be on `PATH` or set `lsp.bee-lsp.binary.path`.

```sh
bee lsp
bee run --inspect-brk app.ts
```

---

## Documentation

| | |
| --- | --- |
| [Current Scope](docs/CURRENT_SCOPE.md) | Stable / Preview / Experimental / Historical |
| [Quick start](docs/QUICK_START.md) | Source-first smoke commands |
| [CLI guide](docs/CLI_USAGE_GUIDE.md) | Flags and examples |
| [v1.16.0 notes](docs/releases/v1.16.0.md) | Wasm 2.0, bundle/compile, RSI |
| [Examples](examples/) | Scripts and tests |
| [Website](https://bee.zhanghe.dev) | Manual and blog |

---

## Contributing

```sh
cargo test --lib
cargo test --test wintertc_compliance_tests -- --test-threads=1
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

See [AGENTS.md](AGENTS.md) for module boundaries (`src/main.rs` is the `bee` entry; do not treat every directory under `src/` as a public API).

---

## License

[MIT](LICENSE)

<p align="center">
  <a href="https://bee.zhanghe.dev"><img src="https://bee.zhanghe.dev/logo.png" alt="Beejs" height="96"></a>
</p>
<h1 align="center">Beejs</h1>
<p align="center">
  A JavaScript and TypeScript runtime in <b>Rust</b> and <b>V8</b>.<br>
  One binary: <code>bee</code>.
</p>

<p align="center">
  <a href="https://bee.zhanghe.dev"><img src="https://img.shields.io/badge/docs-bee.zhanghe.dev-0f172a" alt="Docs"></a>
  <a href="https://github.com/zh30/beejs/releases/tag/v1.12.0"><img src="https://img.shields.io/badge/release-v1.12.0-22c55e" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-yellow" alt="License"></a>
  <a href="https://github.com/zh30/beejs/actions/workflows/ci.yml"><img src="https://github.com/zh30/beejs/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
</p>

<p align="center">
  <a href="https://bee.zhanghe.dev">Website</a>
  &nbsp;·&nbsp;
  <a href="https://bee.zhanghe.dev/docs">Docs</a>
  &nbsp;·&nbsp;
  <a href="docs/CURRENT_SCOPE.md">Current scope</a>
  &nbsp;·&nbsp;
  <a href="https://github.com/zh30/beejs/releases">Releases</a>
  &nbsp;·&nbsp;
  <a href="https://github.com/zh30/beejs/issues">Issues</a>
</p>

[中文文档](https://bee.zhanghe.dev/zh)

---

## What is Beejs?

Beejs ships as a **single executable** named `bee`. At its core is a V8 isolate hosted in Rust (Tokio I/O), with a built-in TypeScript pipeline, a Jest-style test runner, an agent sandbox, and a native `bee:ai` module.

It is **not** a drop-in Node.js replacement. Node and Web APIs are implemented incrementally and tracked in [Current Scope](docs/CURRENT_SCOPE.md). Use Beejs when you want:

- **One binary** for `run` / `eval` / `test` / `repl` / `mcp`
- **TypeScript without `tsc`** (oxc transpile-only)
- **Capability sandbox** for agent tools (`--sandbox`, `--seed`, `--freeze-time`)
- **In-process tensors and LLM streaming** via `bee:ai` (no Python sidecar)

Closest cousins: [Deno](https://github.com/denoland/deno) (V8 + Rust, secure-by-policy) and [Bun](https://github.com/oven-sh/bun) (all-in-one CLI). Beejs keeps V8, adds an agent/MCP host, and is explicit about what is Stable vs Preview.

---

## Install

Prebuilt archives: **macOS** (arm64, x64), **Linux gnu** (x64, arm64), **Windows** (x64 zip).

```sh
# macOS / Linux (recommended)
curl -fsSL https://bee.zhanghe.dev/install.sh | sh

# pin a release
curl -fsSL https://bee.zhanghe.dev/install.sh | BEEJS_VERSION=v1.12.0 sh
```

Windows (PowerShell):

```powershell
irm https://bee.zhanghe.dev/install.ps1 | iex
```

Homebrew (formula lives in this repo; SHA256 is rewritten on each GitHub Release):

```sh
brew install zh30/tap/bee
```

Verify:

```sh
bee --version
bee eval "1 + 1"
```

### Build from source

Requires [Rust](https://rustup.rs/) (this repo pins **1.97.1**) and a C++ toolchain for V8.

```sh
git clone https://github.com/zh30/beejs.git
cd beejs
cargo build --release
./target/release/bee --version
```

---

## Your first program

Create `hello.ts`:

```ts
const runtime = "Beejs";
console.log(`hello from ${runtime}`);
```

```sh
bee run hello.ts
# hello from Beejs
```

`.ts` / `.tsx` are type-stripped by [oxc](https://oxc.rs/) and executed on V8. There is no project-wide `tsc` check; thrown stacks map back to `.ts` lines when a source map is present.

One-liners and a REPL:

```sh
bee eval "console.log(crypto.randomUUID())"
bee repl
```

---

## Test runner

Jest-style `describe` / `test` / `expect`, auto-discovery, watch mode. Stable in v1.12.0.

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

---

## HTTP (Preview)

`bee serve` loads a module that exports `fetch` (WinterCG-style). TLS is rustls HTTP/1.1 when `--https --cert --key` are set.

```js
// app.js  — CommonJS handler used by `bee serve`
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

Default-deny I/O for tool processes, deterministic clocks/PRNG, and stdio MCP.

```sh
bee run --sandbox --permission-policy examples/agent/echo.policy.json \
  --export-tools examples/agent/echo_tool.ts

bee session --sandbox --permission-policy examples/agent/echo.policy.json \
  examples/agent/echo_tool.ts

bee mcp --inspect examples/agent/echo_tool.ts
```

Useful flags on `bee run`:

| Flag | Purpose |
| --- | --- |
| `--sandbox` | Deny fs / net / env / run, then overlay `--allow-*` |
| `--permission-policy <file>` | JSON policy (alias `--policy`) |
| `--audit-log <path>` | JSONL of allow/deny decisions |
| `--seed <u64>` | Deterministic `Math.random` / `crypto.getRandomValues` |
| `--freeze-time <spec>` | Freeze `Date.now` / `performance.now` |
| `--inspect` / `--inspect-brk` | CDP on `127.0.0.1:9229` (`Runtime.evaluate`) |

---

## `bee:ai`

Stable builtins — import without a native addon or Python process:

```ts
import { Tensor, LLM, AgentPipeline } from "bee:ai";

const a = Tensor.from([1, 2, 3, 4], [2, 2]);
const b = Tensor.from([5, 6, 7, 8], [2, 2]);
const c = Tensor.matmul(a, b);
```

`LLM` (`load`, `generate`, `generateStream`, `embed`) and `AgentPipeline` are documented in the [manual](https://bee.zhanghe.dev/docs). Cargo `feature = "ai"` is empty and is **not** a product LLM.

---

## CLI

**Stable**

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

**Preview** — present, contract still tightening: TypeScript, `bee serve --https`, `--inspect` / `--inspect-brk`.

**Experimental** — do not treat as product promises: `bee bundle`, `bee debug`, `bee serve` HTTP-only, `bee init` / `create` / `add` / `remove` / `install` / `prune` / `bunx` / `upgrade`, N-API hello `process.dlopen`.

Full flags: [CLI usage guide](docs/CLI_USAGE_GUIDE.md).

---

## Compatibility

| | Beejs 1.12.0 | Node.js | Bun | Deno |
| --- | --- | --- | --- | --- |
| Engine | V8 + Rust | V8 + C++ | JavaScriptCore + Zig | V8 + Rust |
| TypeScript | oxc, transpile-only | loaders / `tsc` | built-in | built-in |
| Secure defaults | opt-in `--sandbox` | none | none | permission flags |
| Node API | incremental Preview | native | drop-in goal | compat layer |
| Package manager | Experimental | npm | `bun` | `deno` / JSR |
| Test runner | built-in `bee test` | external | `bun test` | `deno test` |
| Native AI | `bee:ai` | — | — | — |

Node modules that exist today include `fs`, `path`, `os`, `url`, `buffer`, `events`, `stream`, `crypto`, `http`, `http2`, `net`, `child_process` (`execSync` / `spawnSync`), `zlib`, `util`, `worker_threads`. Web: `fetch`, Streams, Web Crypto, URL, `Worker`, and related APIs. **Coverage is per-API**, not “Node compatible.” The executable scorecard is `tests/conformance/` (55 fixtures). WinterTC baseline: `DOMException`, `URLPattern`, `ReadableStream.from`, `bee:sockets`, `import.meta.main`.

The only user-facing capability boundary is [Current Scope](docs/CURRENT_SCOPE.md). Historical `docs/STAGE_*` numbers and “1000x” claims are not current facts.

Performance figures belong in `benchmarks/` with commit, command, hardware, and a correctness check. This README does not reprint them.

---

## Editors

- **VS Code**: [tools/vscode-extension](tools/vscode-extension) — `bee lsp` + inspector attach. Install the local `.vsix`; Marketplace is not part of 1.9.1.
- **Zed**: [tools/zed-extension](tools/zed-extension) — Install Dev Extension; `bee` must be on `PATH` or set `lsp.bee-lsp.binary.path`.

```sh
bee lsp          # Language Server Protocol on stdin/stdout
bee run --inspect-brk app.ts
```

---

## Documentation

| | |
| --- | --- |
| [Current Scope](docs/CURRENT_SCOPE.md) | Stable / Preview / Experimental / Historical |
| [Quick start](docs/QUICK_START.md) | Source-first smoke commands |
| [CLI guide](docs/CLI_USAGE_GUIDE.md) | Flags and examples |
| [Docs index](docs/README.md) | Everything else |
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

See [Agents.md](Agents.md) for module boundaries (`src/main.rs` is the `bee` entry; do not treat every directory under `src/` as a public API).

---

## License

[MIT](LICENSE)

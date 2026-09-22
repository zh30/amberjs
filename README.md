<p align="center">
  <a href="https://amberjs.com"><img src="https://amberjs.com/logo.png" alt="Amber" height="96"></a>
</p>

<h1 align="center">Amber</h1>

<p align="center">
  A JavaScript and TypeScript runtime in <b>Rust</b> and <b>V8</b>.<br>
  One binary: <code>amber</code>. Built for agent tools, not as a Node.js clone.
</p>

<p align="center">
  <a href="https://amberjs.com"><img src="https://img.shields.io/badge/docs-amberjs.com-0f172a" alt="Docs"></a>
  <a href="https://github.com/zh30/amberjs/releases/tag/v1.16.0"><img src="https://img.shields.io/badge/release-v1.16.0-22c55e" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-yellow" alt="License"></a>
  <a href="https://github.com/zh30/amberjs/actions/workflows/ci.yml"><img src="https://github.com/zh30/amberjs/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/amberjs"><img src="https://img.shields.io/crates/v/amberjs.svg" alt="crates.io"></a>
</p>

<p align="center">
  <a href="https://amberjs.com">Website</a>
  ·
  <a href="https://amberjs.com/docs">Docs</a>
  ·
  <a href="docs/CURRENT_SCOPE.md">Current scope</a>
  ·
  <a href="https://github.com/zh30/amberjs/releases">Releases</a>
  ·
  <a href="https://github.com/zh30/amberjs/issues">Issues</a>
</p>

[中文文档](https://amberjs.com/zh)

---

## What is this?

`amber` runs `.js` / `.ts` / `.tsx` on V8. TypeScript is stripped by [oxc](https://oxc.rs/) — there is no project-wide `tsc` step.

Use it when you want **one binary** that can:

- run scripts and a REPL
- run Jest-style tests
- host agent tools behind a capability sandbox (`--sandbox`, `--seed`, `--freeze-time`)
- speak MCP / JSON-RPC over stdio
- do in-process tensors via `amber:ai` (no Python sidecar)

It is **not** a drop-in Node.js replacement. APIs land incrementally and are scored in `tests/conformance/` (**55/55** on Node.js Conformance 5.0). Closest cousins: [Deno](https://github.com/denoland/deno) (V8 + Rust, permissioned) and [Bun](https://github.com/oven-sh/bun) (all-in-one CLI). Amber keeps V8, adds an agent/MCP host, and labels every command Stable / Preview / Experimental.

The only user-facing capability map is [Current Scope](docs/CURRENT_SCOPE.md). Historical `docs/STAGE_*` numbers are not current facts.

---

## Install

Prebuilt archives: macOS (arm64, x64), Linux gnu (x64, arm64), Windows (x64 zip).

```sh
# macOS / Linux
curl -fsSL https://get.amberjs.com/install.sh | sh

# pin a release
curl -fsSL https://get.amberjs.com/install.sh | AMBER_VERSION=v1.16.0 sh
```

Windows (PowerShell):

```powershell
irm https://get.amberjs.com/install.ps1 | iex
```

Homebrew (formula in this repo; SHA256 is rewritten on each GitHub Release):

```sh
brew install zh30/tap/amber
```

crates.io (each GitHub `v*` tag publishes via Release Assets + `CARGO_REGISTRY_TOKEN`; install the `amber` binary):

```sh
cargo install amberjs
```

```sh
amber --version
amber eval "1 + 1"
```

### Build from source

Needs [Rust](https://rustup.rs/) (**1.97.1**, pinned) and a C++ toolchain for V8.

```sh
git clone https://github.com/zh30/amberjs.git
cd amberjs
cargo build --release
./target/release/amber --version
```

---

## 60-second tour

```ts
// hello.ts
const runtime = "Amber";
console.log(`hello from ${runtime}`);
```

```sh
amber run hello.ts
# hello from Amber

amber eval "console.log(crypto.randomUUID())"
amber repl
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
amber test
amber test examples/testing/math.test.js
amber test --watch
```

`amber test --parallel` is rejected (exit code 2): V8 isolates are not shared across threads.

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
amber serve app.js --host 127.0.0.1 --port 3000
```

---

## Agents, sandbox, MCP

Default-deny I/O for tool processes, deterministic clocks/PRNG, stdio MCP.

```sh
amber run --sandbox --permission-policy examples/agent/echo.policy.json \
  --export-tools examples/agent/echo_tool.ts

amber session --sandbox --permission-policy examples/agent/echo.policy.json \
  examples/agent/echo_tool.ts

amber mcp --inspect examples/agent/echo_tool.ts
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

## `amber:ai`

Stable builtins — no native addon, no Python process:

```ts
import { Tensor, LLM, AgentPipeline } from "amber:ai";

const a = Tensor.from([1, 2, 3, 4], [2, 2]);
const b = Tensor.from([5, 6, 7, 8], [2, 2]);
const c = Tensor.matmul(a, b);
```

`LLM` (`load`, `generate`, `generateStream`, `embed`) and `AgentPipeline` are in the [manual](https://amberjs.com/docs). Cargo `feature = "ai"` is empty and is **not** a product LLM.

---

## CLI

**Stable** — everyday commands:

```text
amber run <file> [args...]     Run JS (TS/TSX via oxc)
amber eval <code>              Evaluate an expression
amber test [files...] [--watch]
amber repl
amber snapshot [build|status|clean]
amber session <tool>           JSON-RPC over stdin
amber mcp [tool]               MCP stdio server
amber compile <file> [-o myapp]  Host SEA (linux/macos/windows). Contract: docs/COMPILE_CONTRACT.md
amber --version | amber version
amber bundle <entry> [-o dist/bundle.js] [--minify]
```

**Preview** — present, contract still tightening:

```text
amber serve [--https --cert --key]
amber run --inspect / --inspect-brk
TypeScript / TSX execution
```

**Experimental** — do not treat as product promises: `amber debug`, `amber init` / `create` / `add` / `remove` / `install` / `prune` / `x` / `upgrade`, N-API hello `process.dlopen`, `fmt` / `lint` / `task` / `bench` / `profile` / `deploy`.

Full flags: [CLI usage guide](docs/CLI_USAGE_GUIDE.md).

---

## Compatibility

| | Amber 1.16.0 | Node.js | Bun | Deno |
| --- | --- | --- | --- | --- |
| Engine | V8 + Rust | V8 + C++ | JavaScriptCore + Zig | V8 + Rust |
| TypeScript | oxc, transpile-only | loaders / `tsc` | built-in | built-in |
| Secure defaults | opt-in `--sandbox` | none | none | permission flags |
| Node API | incremental Preview | native | drop-in goal | compat layer |
| Package manager | Experimental | npm | `bun` | `deno` / JSR |
| Test runner | built-in `amber test` | external | `bun test` | `deno test` |
| Native AI | `amber:ai` | — | — | — |

Node modules that exist today include `fs`, `path`, `os`, `url`, `buffer`, `events`, `stream`, `crypto`, `http`, `http2`, `net`, `child_process` (`execSync` / `spawnSync`), `zlib`, `util`, `worker_threads`. Web: `fetch`, Streams, Web Crypto, URL, `Worker`. **Coverage is per-API**, not “Node compatible.” WinterTC baseline: `DOMException`, `URLPattern`, `ReadableStream.from`, `amber:sockets`, `import.meta.main`.

On Apple M2 Max (2026-09-16), suite 2.0 URL / `fetch` / `ReadableStream` beat Node.js; Express on Amber was ~68k req/s vs Node ~19k. Numbers live in [`benchmarks/`](benchmarks/) with command, hardware, and a correctness check. This README does not reprint them as product SLAs.

---

## Editors

- **VS Code**: [extensions/vscode](extensions/vscode) — `amber lsp` + inspector attach. Install the local `.vsix`; Marketplace is not shipped.
- **Zed**: [extensions/zed](extensions/zed) — Install as a Dev Extension; `amber` must be on `PATH`.

```sh
amber lsp
amber run --inspect-brk app.ts
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
| [Website](https://amberjs.com) | Manual and blog |

---

## Contributing

```sh
cargo test --lib
cargo test --test wintertc_compliance_tests -- --test-threads=1
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

See [AGENTS.md](AGENTS.md) for module boundaries (`src/main.rs` is the `amber` entry; do not treat every directory under `src/` as a public API).

---

## License

[MIT](LICENSE)

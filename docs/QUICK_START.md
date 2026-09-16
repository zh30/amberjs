# Beejs Quick Start

Beejs v1.16.0 is a Rust + V8 JavaScript/TypeScript runtime. This page only
documents behavior from the current CLI in `src/main.rs`. See
[Current Scope](CURRENT_SCOPE.md) for the Stable / Preview / Experimental map.

## Install a release

```bash
curl -fsSL https://bee.zhanghe.dev/install.sh | sh
bee --version
bee eval "1 + 1"
```

Windows (PowerShell):

```powershell
irm https://bee.zhanghe.dev/install.ps1 | iex
```

## Build from source

```bash
git clone https://github.com/zh30/beejs.git
cd beejs
cargo build --release
./target/release/bee --version
```

During development:

```bash
cargo run -- eval "1 + 1"
cargo run -- run examples/basics/hello_world.js
cargo run -- repl
```

## Run something

```bash
bee run examples/basics/hello_world.js
bee run examples/basics/typescript_demo.ts
bee repl
bee test examples/testing/math.test.js
```

`.ts` / `.tsx` are type-stripped by oxc, then executed on V8. That is Preview:
there is no project-wide `tsc` check.

## Not a Node replacement

Beejs is a core runtime for scripts, tests, agent tools, and compatibility work.
It is not documented as a complete replacement for Node.js, Bun, or Deno.

Historical Stage documents are historical materials. Performance claims belong
in `benchmarks/` with command, commit, hardware, and a correctness check.

```bash
cargo test --lib
cargo run -- run examples/basics/hello_world.js
```

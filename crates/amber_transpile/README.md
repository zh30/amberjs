# amber_transpile

TypeScript/TSX transpile engine for the [Amber](https://github.com/zh30/amberjs) runtime. It uses [oxc](https://oxc.rs/) and is transpile-only (no `tsc` typecheck).

Published as a workspace crate so `amberjs` can be released to crates.io. The CLI users want is:

```sh
cargo install amberjs
```

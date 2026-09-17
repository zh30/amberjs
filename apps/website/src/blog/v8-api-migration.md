---
title: "Runtime Cleanup Before Release"
excerpt: "How Beejs v0.1 tightened CLI output, release checks, and documentation before publishing."
date: "2026-05-25"
author: "Beejs Core Team"
readTime: "4 min read"
tag: "Engineering"
---

# Runtime Cleanup Before Release

The v0.1 release work focuses on turning the Beejs runtime from an active development repository into something users can install and evaluate without internal noise.

## Clean CLI Output

The default CLI path now reports the package version from Cargo metadata and keeps `eval` and `run` output focused on user results. Internal Web API setup messages were removed from the default runtime initialization path.

```bash
bee --version
bee eval "1 + 1"
```

## Default-Build CI

CI now checks the release surface users actually install:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --lib`
- `cargo test --test beejs_core_tests`
- `cargo test --test cli_release_tests`
- macOS and Linux build jobs

Feature-gated modules can still be checked separately, but they no longer define the default public promise.

## Documentation Reset

The website and README now describe Beejs v0.1 as a Rust + V8 JavaScript/TypeScript runtime with a focused CLI and compatibility layer. Historical stage reports remain in the repository, but release-facing pages avoid unverified performance numbers.

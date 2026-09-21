# Amber Performance Report

This file intentionally does not publish historical stage benchmark numbers as
current v0.1 facts.

Amber v0.1 is a core runtime release. Performance claims for a public release
must be generated from fresh, reproducible commands in the current repository and
recorded with the commit, machine, operating system, command, iteration count,
and raw results.

## Current Verification Commands

```bash
cargo build --release
./target/release/amber eval "1 + 1"
./target/release/amber run examples/basics/hello_world.js
./target/release/amber test examples/testing/math.test.js
```

## Benchmark Record Template

```text
Version/commit:
Machine:
OS:
Command:
Iterations:
Raw result:
Comparison target:
Notes:
```

Historical performance experiments and stage reports should stay in archive
locations and must not be presented as current release guarantees.

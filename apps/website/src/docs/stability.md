---
title: "Stability levels"
nav: "Stability levels"
subtitle: "How Amber decides what is Stable, Preview, Experimental, or not supported."
status: "stable"
id: "stability"
---

Amber labels every capability. The labels come from [`docs/CURRENT_SCOPE.md`](https://github.com/zh30/amberjs/blob/main/docs/CURRENT_SCOPE.md), which is the user-facing source of truth. The [compatibility page](/compatibility) renders the same data as a searchable table.

## The four levels

**Stable.** The capability is reachable from the `amber` binary, has a contract document in `docs/`, and has an executable test in `tests/` that runs in CI. The contract lists its limits next to the API. As of v1.17.0, 77 contracts are numbered G1 to G77.

Some Stable rows are marked **tentative** or **provisional**. They passed the graduation rule, but their id or edges may still be adjusted. Read the contract before relying on an edge case.

**Preview.** The capability is in the default build and useful, but its contract, diagnostics, or edge cases are still being tightened. Most of `src/nodejs_core/` and `src/web_api/` outside the Stable contracts are Preview.

**Experimental.** Code exists, but it is not a product promise until it is re-verified. This includes some CLI commands, optional Cargo features, and the N-API hello loader.

**Not supported.** Deliberately out of scope today. Where possible, Amber throws or rejects with a clear error instead of returning fake data.

## Graduation rule

A capability moves up only when all of these are true:

1. It is reachable through `src/main.rs` or a documented library API in `src/lib.rs`.
2. Its behavior is described without relying on historical stage reports.
3. Current tests or smoke commands cover the documented behavior.
4. Known limitations are documented next to the capability.
5. Feature-gated work has a passing feature check.

Checklist progress in the planning documents does not promote anything by itself.

## Conformance scorecard

`tests/conformance/` holds Node-style fixtures that run on every CI build. The current scorecard is **56/56 PASS**, and CI fails if fewer than 55 pass or any fixture fails or is skipped.

```bash
cargo build
./tests/conformance/run_conformance.sh
cat tests/conformance/scorecard.md
```

The express, fastify, and hono smoke fixtures need those packages under `benchmarks/idle_memory/node_modules`. Without them they report SKIP and the gate fails. CI installs them first.

Some fixtures exercise framework smoke paths or `http2` basics. A passing fixture is a regression gate. It does not by itself make an API Stable.

## Performance

Amber does not publish a stable performance claim today. Any number must include the date, commit, exact command, build profile, hardware, harness, and correctness checks. Older benchmark tables in the repository are historical.

# GitHub Actions runtime infrastructure (v1.9.1)

## Verdict

PR gates, multi-OS GitHub Releases (including Windows zip), GHCR publish, SBOM/cosign, fail-closed audit, and a compile-only feature matrix are in the workflows. This is the v1.9.1 supporting-facility surface. It is still not Node/Deno/Bun-scale (no WPT job, crates.io only when `CARGO_REGISTRY_TOKEN` is set, GHCR is linux/amd64 only).

## Live workflows

| Workflow file | Trigger | What it actually does |
|---|---|---|
| `.github/workflows/ci.yml` | `push` `main`, PR → `main`, `workflow_dispatch`, `v*` | rustc **1.97.1**, fmt, clippy `-D warnings`, `cargo test`, feature matrix `benchmarks` + `observability` (fail-closed), WinterTC named step, Node conformance fail-closed at >=55 PASS / 0 FAIL / 0 SKIP (`tests/conformance/run_conformance.sh`), `amber test examples/testing`, `cargo package --list`, ubuntu + macOS release smoke, Windows smoke, **cargo-audit fail-closed**, cargo-deny advisories/licenses |
| `.github/workflows/release-assets.yml` | `v*` tags, `workflow_dispatch` | linux gnu x64/arm64, macOS arm64/x64, Windows x64 zip (**no** `continue-on-error` on Windows); requires Unix archives **and** Windows zip with `amber.exe`; checksums, CycloneDX SBOM, cosign; Homebrew formula SHA rewrite; crates.io publish of `amber_transpile` → `amber_sandbox` → `amberjs` when `CARGO_REGISTRY_TOKEN` is set |
| `.github/workflows/docker.yml` | `main`, `v*`, `workflow_dispatch` | Build/push `ghcr.io/zh30/amberjs` **linux/amd64 only** (no fake `linux/arm64` tag) |
| `.github/workflows/performance-tests.yml` | Monday 02:00 UTC cron, `workflow_dispatch` | Release lib benchmark tests. Not a PR gate |

## Remaining honesty limits

- GHCR is **amd64-only**.
- `feature = "ai"` is empty and is not in the CI matrix; default `amber:ai` compiles without it.
- `enterprise` / `cloudnative` / `multilang` / `tch` are not CI-gated.
- crates.io publish is skipped without `CARGO_REGISTRY_TOKEN` (warning annotation). With the token, Release Assets publishes workspace crates in dependency order; already-uploaded versions are skipped.
- Live tag, brew checksum against a published Release, and `microsoft/winget-pkgs` submission are out of band.

See [IMPLEMENTATION_PLAN_v1.9.1.md](./IMPLEMENTATION_PLAN_v1.9.1.md) and [INFRASTRUCTURE_CAPABILITY_BACKLOG.md](./INFRASTRUCTURE_CAPABILITY_BACKLOG.md).

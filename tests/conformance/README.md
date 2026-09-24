# Node.js conformance scorecard

This directory is the north-star metric for Amber Node compatibility work.

## Layout

- `fixtures/` — small JS scripts asserting Node-like behavior
- `scorecard.md` — latest pass/fail summary (update when you run the suite)
- `run_conformance.sh` — runner that executes fixtures with `amber` (or `cargo run`)

## How to run

```bash
./tests/conformance/run_conformance.sh
# or after release build:
AMBER_BIN=./target/release/amber ./tests/conformance/run_conformance.sh
```

Exit code is non-zero if any fixture fails, if any fixture prints `CONFORMANCE_SKIP`, or if PASS is below **55**. A larger suite that is still 0 FAIL and 0 SKIP stays green. CI job `Format, Lint, Test, Package` (required on `main`) runs this script in the `Node conformance scorecard` step; that step has no `continue-on-error`, so a failed gate fails the job and blocks merge.

```bash
./tests/conformance/run_conformance.sh --self-test
```

## Scope policy

Start with pure-logic / sync modules (`path`, `buffer`, `events`, `assert`, `url`,
`util`, `querystring`). Agent sandbox fixtures (`fs_read_denied`,
`fs_jail_allows_prefix`, `env_denied`, `run_denied`, `fetch_allowlist`) use a
sidecar `.flags` file for `--sandbox` / `--allow-*`.

Optional `.policy.json` next to a fixture is passed as `--permission-policy`.

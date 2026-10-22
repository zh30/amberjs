# test262 conformance (ECMAScript)

North-star suite for Amber’s **language** conformance (separate from Node fixtures in `tests/conformance/`).

## Layout

| Path | Role |
|------|------|
| `PIN` | Exact `tc39/test262` git commit |
| `run_test262.py` | Discover → run → scorecard |
| `amber_host.js` | Minimal `$262` host object |
| `features.json` | Supported / always-skip feature flags |
| `scorecard.md` / `scorecard.json` | Latest run summary |
| `../../third_party/test262/` | Fetched suite (gitignored) |
| `../../scripts/fetch_test262.sh` | Clone/checkout `PIN` |

## Fetch

```bash
./scripts/fetch_test262.sh
```

Fails if the checked-out SHA ≠ `PIN`.

## Run

```bash
cargo build --release   # or: cargo build
export AMBER_BIN=./target/release/amber   # or ./target/debug/amber

# harness unit checks (no suite needed)
python3 tests/test262/run_test262.py --self-test

# subset (fast iteration)
AMBER_BIN=./target/release/amber python3 tests/test262/run_test262.py --path language/types --jobs 4

# full suite (large; expect long wall time)
AMBER_BIN=./target/release/amber python3 tests/test262/run_test262.py --jobs 8
```

Env: `AMBER_BIN`, `TEST262_ROOT`, `TEST262_TIMEOUT` (seconds per case, default 30),
`AMBER_TEST262=1` (set automatically by the runner — enables native `$262.createRealm`).


## Scorecard / gate

- **PASS** — behavior matches positive or negative metadata.
- **FAIL** — wrong result, timeout, or harness/exec error (honest; no stubs).
- **SKIP (expected)** — only when a `features:` entry is listed in `always_skip_features` (`features.json`); reason `unsupported-feature:<name>`.
- **SKIP (unexpected)** — any other SKIP (wrong reason, or feature not in `always_skip_features`). Counted as `unexpected_skip`; gate fails.

`supported` in `features.json` is informational. Features not in that list are **run** (honest PASS/FAIL), not skipped.

**Done** for this stream: full suite with **0 FAIL** and **0 unexpected SKIP**. The runner exits `1` if `FAIL>0` or `unexpected_skip>0`. Filtered runs are for iteration; they do not redefine done.

Node conformance (`./tests/conformance/run_conformance.sh`) stays ≥55/55 and is unrelated.

## CI

- `.github/workflows/test262.yml` runs a fail-closed **smoke** job (`--self-test` + `language/types*` paths) and uploads that run’s scorecard artifact (`test262-scorecard`). It does not run the full suite.
- Full suite remains local/manual (`run_test262.py` without `--path`) until green; then promote a full-suite job to a required fail-closed gate.

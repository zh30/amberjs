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
- **SKIP** — expected only when a `features:` entry is in `always_skip_features` or absent from `supported` in `features.json`.

**Done** for this stream: full suite with **0 FAIL** and **0 unexpected SKIP**. Filtered runs are for iteration; they do not redefine done.

Node conformance (`./tests/conformance/run_conformance.sh`) stays ≥55/55 and is unrelated.

## CI

- Harness PRs: self-test + small path smoke fail-closed; full suite uploads scorecard artifact (may still have FAILs while we drive to green).
- When the full suite is green, promote the full job to a required fail-closed gate.

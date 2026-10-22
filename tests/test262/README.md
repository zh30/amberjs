# test262 conformance (ECMAScript)

North-star suite for Amber’s **language** conformance (separate from Node fixtures in `tests/conformance/`).

## Layout

| Path | Role |
|------|------|
| `PIN` | Exact `tc39/test262` git commit |
| `run_test262.py` | Discover → run → scorecard |
| `amber_host.js` | Minimal `$262` host object |
| `features.json` | Supported / always-skip feature flags |
| `engine_fail_allowlist.txt` | Optional V8/Node FAIL catalog (seed draft; **OFF by default**) |
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

### Optional engine FAIL allowlist (OFF by default)

Literal **0 FAIL** is blocked by shared V8/Node (and proposal-not-in-V8) gaps. The
allowlist catalogs those honest FAILs without SKIP abuse. **Default harness and CI
behavior is unchanged** — allowlisted paths still count as FAIL until explicitly enabled.

Enable (either form):

```bash
# env
AMBER_TEST262_ENGINE_ALLOWLIST=1 AMBER_BIN=./target/release/amber \
  python3 tests/test262/run_test262.py --path language/eval-code/direct --jobs 4

# or flag
AMBER_BIN=./target/release/amber \
  python3 tests/test262/run_test262.py --engine-allowlist --path language/expressions/compound-assignment
```

When enabled:

- Matching FAILs become **`ENGINE_ALLOW`** (still listed in the scorecard).
- Exit **0** only if `FAIL=0` and `unexpected_skip=0` (`ENGINE_ALLOW` is not FAIL).
- Patterns live in `engine_fail_allowlist.txt` (exact path or directory prefix ending `/`).
- Override file with `--engine-allowlist-file`; force off with `--no-engine-allowlist`.

**Do not** move these entries into `features.json` `always_skip_features`. The seed aims to
cover the projected post-open-stack engine residual (**≈447** FAIL after
#274/#276/#279/#278/#281/#282/#295/#296 on measured main FAIL Σ **1300** @ `cb3c0005`:
language ≈216, built-ins ≈94, other ≈137) and remains a **draft until measured after
remaining open drafts land**; do **not** flip CI smoke / the default gate ON without an
explicit product decision (gate still pending Henry). Amber-owned open-stack paths are
intentionally **absent** from the seed so they stay honest FAIL until those PRs land.


## Scorecard / gate

- **PASS** — behavior matches positive or negative metadata.
- **FAIL** — wrong result, timeout, or harness/exec error (honest; no stubs).
- **ENGINE_ALLOW** — FAIL remapped only when the optional engine allowlist is ON; catalog only, not SKIP.
- **SKIP (expected)** — only when a `features:` entry is listed in `always_skip_features` (`features.json`); reason `unsupported-feature:<name>`.
- **SKIP (unexpected)** — any other SKIP (wrong reason, or feature not in `always_skip_features`). Counted as `unexpected_skip`; gate fails.

`supported` in `features.json` is informational. Features not in that list are **run** (honest PASS/FAIL), not skipped.

**Done** for this stream (default gate): full suite with **0 FAIL** and **0 unexpected SKIP**. The runner exits `1` if `FAIL>0` or `unexpected_skip>0`.
Optional allowlist mode reframes Amber-owned residual as **0 FAIL** + cataloged `ENGINE_ALLOW`,
but is **not** the default CI gate. Filtered runs are for iteration; they do not redefine done.

Node conformance (`./tests/conformance/run_conformance.sh`) stays ≥55/55 and is unrelated.

## CI

- `.github/workflows/test262.yml` runs a fail-closed **smoke** job (`--self-test` + `language/types*` paths) and uploads that run’s scorecard artifact (`test262-scorecard`). It does not run the full suite.
- Full suite remains local/manual (`run_test262.py` without `--path`) until green; then promote a full-suite job to a required fail-closed gate.
- **CI smoke stays honest while allowlist stays OFF:** the workflow must **not** set
  `AMBER_TEST262_ENGINE_ALLOWLIST` and must **not** pass `--engine-allowlist`. Smoke
  therefore still counts matching seed paths as **FAIL** (same as local default). Enabling
  the allowlist is a separate product flip (`AMBER_TEST262_ENGINE_ALLOWLIST=1` or
  `--engine-allowlist`), not a CI silent path.

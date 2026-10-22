# test262 conformance (ECMAScript)

North-star suite for Amber’s **ECMAScript / test262** conformance (separate from Node fixtures in `tests/conformance/`).

Default discovery (no `--path`) walks the full `test/` tree: **language**, **built-ins**, and other roots (`annexB`, `staging`, `intl402`, `harness`, …). Use `--path` only to filter a subset for iteration.

## Layout

| Path | Role |
|------|------|
| `PIN` | Exact `tc39/test262` git commit |
| `run_test262.py` | Discover → run → scorecard |
| `amber_host.js` | Minimal `$262` host (`detachArrayBuffer` → native `__amber_detachArrayBuffer`) |
| `features.json` | Supported / always-skip feature flags |
| `engine_fail_allowlist.txt` | Optional V8/Node FAIL catalog (seed; harness **OFF by default**) |
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
AMBER_BIN=./target/release/amber python3 tests/test262/run_test262.py --path built-ins --jobs 4
AMBER_BIN=./target/release/amber python3 tests/test262/run_test262.py --path annexB --jobs 4

# full suite (language + built-ins + other roots; allowlist OFF / honest FAIL; large)
AMBER_BIN=./target/release/amber python3 tests/test262/run_test262.py --jobs 8

# full suite — Option B evidence (allowlist ON via official env; default still OFF)
AMBER_TEST262_ENGINE_ALLOWLIST=1 AMBER_BIN=./target/release/amber \
  python3 tests/test262/run_test262.py --jobs 8
```

Env: `AMBER_BIN`, `TEST262_ROOT`, `TEST262_TIMEOUT` (seconds per case, default 30),
`AMBER_TEST262=1` (set automatically by the runner — enables native `$262.createRealm` / `$262.detachArrayBuffer` / `$262.agent`),
`AMBER_TEST262_CAN_BLOCK=0|1` (set by the runner from `CanBlockIsFalse` / `CanBlockIsTrue` — V8 `AllowAtomicsWait` / AgentCanSuspend),
`AMBER_TEST262_ENGINE_ALLOWLIST` (optional; see below).

### Engine FAIL allowlist (OFF by default; Option B env flip)

Literal **0 FAIL** is blocked by shared V8/Node/ICU/SM (and proposal-not-in-V8) gaps.
[#285](https://github.com/zh30/amberjs/pull/285) catalogs those honest FAILs without SKIP
abuse. **Harness default stays OFF** — seed paths still count as FAIL unless the official
ON switch is set.

**Option B (approved engine-gap policy — pending Henry approval):** treat proven engine
gaps as cataloged `ENGINE_ALLOW` for **full-suite / scorecard evidence only** by setting:

```bash
AMBER_TEST262_ENGINE_ALLOWLIST=1
```

(Equivalent CLI: `--engine-allowlist`. Force OFF: `--no-engine-allowlist` or unset env.)

```bash
# subset smoke with allowlist ON
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
remaining open drafts land**. **Do not** change harness default to ON, and **do not** set
the env on CI smoke, until Henry approves Option B. Amber-owned open-stack paths are
intentionally **absent** from the seed so they stay honest FAIL until those PRs land.


## Scorecard / gate

- **PASS** — behavior matches positive or negative metadata.
- **FAIL** — wrong result, timeout, or harness/exec error (honest; no stubs).
- **ENGINE_ALLOW** — FAIL remapped only when the optional engine allowlist is ON; catalog only, not SKIP.
- **SKIP (expected)** — only when a `features:` entry is listed in `always_skip_features` (`features.json`); reason `unsupported-feature:<name>`.
- **SKIP (unexpected)** — any other SKIP (wrong reason, or feature not in `always_skip_features`). Counted as `unexpected_skip`; gate fails.

`supported` in `features.json` is informational. Features not in that list are **run** (honest PASS/FAIL), not skipped.

**Done** for this stream:

- **Default / CI smoke (allowlist OFF):** full suite with **0 FAIL** and **0 unexpected SKIP**
  (literal green, or wait V8 — Option A).
- **Option B evidence (allowlist ON via env):** full suite with **0 FAIL** and
  **0 unexpected SKIP** after seeded engine gaps remap to `ENGINE_ALLOW`
  (`AMBER_TEST262_ENGINE_ALLOWLIST=1`). Pending Henry approval before treating this as
  the product gate.

The runner exits `1` if `FAIL>0` or `unexpected_skip>0`. Filtered runs are for iteration;
they do not redefine done.

Node conformance (`./tests/conformance/run_conformance.sh`) stays ≥55/55 and is unrelated.

## CI

- `.github/workflows/test262.yml` runs a fail-closed **smoke** job (`--self-test` + `language/types*` paths) and uploads that run’s scorecard artifact (`test262-scorecard`). It does not run the full suite.
- Full suite remains local/manual (`run_test262.py` without `--path`) until green; then promote a full-suite job to a required fail-closed gate.
- **CI smoke stays OFF (honest FAIL):** the workflow must **not** set
  `AMBER_TEST262_ENGINE_ALLOWLIST` and must **not** pass `--engine-allowlist`.
- **Option B full-suite evidence** (pending Henry): run locally / optionally a future
  non-required job with `AMBER_TEST262_ENGINE_ALLOWLIST=1` only — never as a silent
  default flip of the harness or smoke.

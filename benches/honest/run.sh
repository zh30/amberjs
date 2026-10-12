#!/usr/bin/env bash
# Honest Amber / Node / Bun comparison.
# Prints commit, build profile, hardware, and a median table.
# Do not publish "faster than X" claims from this script unless every compared
# runtime is present and the same harness ran successfully.
#
# Peer binaries are `node` and `bun` on PATH. Pin them with HONEST_NODE and
# HONEST_BUN. Machine-readable output: HONEST_JSON=/path/results.json
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

PROFILE="${HONEST_PROFILE:-release}"
if [[ "${HONEST_SKIP_BUILD:-0}" != "1" ]]; then
  if [[ "$PROFILE" == "release" ]]; then
    cargo build -q --release
    AMBER="${AMBER_BIN:-$ROOT/target/release/amber}"
  else
    cargo build -q
    AMBER="${AMBER_BIN:-$ROOT/target/debug/amber}"
  fi
else
  if [[ "$PROFILE" == "release" ]]; then
    AMBER="${AMBER_BIN:-$ROOT/target/release/amber}"
  else
    AMBER="${AMBER_BIN:-$ROOT/target/debug/amber}"
  fi
fi

if [[ ! -x "$AMBER" ]]; then
  echo "amber binary is not executable: $AMBER" >&2
  exit 1
fi

exec python3 "$ROOT/benches/honest/compare.py" \
  --amber "$AMBER" \
  --profile "$PROFILE" \
  --root "$ROOT" \
  "$@"

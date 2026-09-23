#!/usr/bin/env bash
# Host smoke for Stable `amber compile`. See docs/COMPILE_CONTRACT.md.
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: sea_compile_smoke.sh <amber-binary>" >&2
  exit 2
fi

AMBER=$1
if [[ ! -x "$AMBER" && ! -f "$AMBER" ]]; then
  echo "amber binary not found: $AMBER" >&2
  exit 2
fi

ROOT=$(mktemp -d)
trap 'rm -rf "$ROOT"' EXIT

cat >"$ROOT/smoke.js" <<'EOF'
const path = require("path");
if (typeof path.basename !== "function") {
  throw new Error("path.basename missing");
}
const args = process.argv.slice(2).join(",");
console.log("SEA_SMOKE_OK:" + path.basename("file.txt") + ":" + args);
EOF

OUT="$ROOT/sea-smoke"
"$AMBER" compile "$ROOT/smoke.js" -o "$OUT"
if [[ ! -f "$OUT" ]]; then
  echo "compile did not write $OUT" >&2
  exit 1
fi

GOT=$("$OUT" hello)
printf '%s\n' "$GOT"
printf '%s\n' "$GOT" | grep -F "SEA_SMOKE_OK:file.txt:hello" >/dev/null

FAIL_OUT="$ROOT/should-not-exist"
set +e
FAIL_TEXT=$("$AMBER" compile "$ROOT/missing.js" -o "$FAIL_OUT" 2>&1)
FAIL_CODE=$?
set -e
printf '%s\n' "$FAIL_TEXT"
if [[ "$FAIL_CODE" -eq 0 ]]; then
  echo "missing entry should fail" >&2
  exit 1
fi
printf '%s\n' "$FAIL_TEXT" | grep -F "error: amber compile:" >/dev/null
printf '%s\n' "$FAIL_TEXT" | grep -F "entry file not found" >/dev/null
if [[ -e "$FAIL_OUT" ]]; then
  echo "failed compile wrote $FAIL_OUT" >&2
  exit 1
fi

echo "SEA compile smoke passed on $(uname -s) $(uname -m)"

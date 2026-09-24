#!/usr/bin/env bash
# Node conformance scorecard runner for Amber.
set -u

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

if [[ -n "${AMBER_BIN:-}" ]]; then
  AMBER=("$AMBER_BIN")
elif [[ -x ./target/release/amber ]]; then
  AMBER=(./target/release/amber)
elif [[ -x ./target/debug/amber ]]; then
  AMBER=(./target/debug/amber)
else
  AMBER=(cargo run --quiet --)
fi

FIXTURE_DIR="$ROOT/tests/conformance/fixtures"
SCORECARD="$ROOT/tests/conformance/scorecard.md"
# Issue #104 floor. A larger all-PASS suite stays green; fewer fixtures or any SKIP does not.
REQUIRED_PASS=55
PASS=0
FAIL=0
SKIP=0
RESULTS=()

# 0 when CI may treat the scorecard as green.
conformance_gate_ok() {
  local pass=$1 fail=$2 skip=$3 required=$4
  [[ "$fail" -eq 0 && "$skip" -eq 0 && "$pass" -ge "$required" ]]
}

if [[ "${1:-}" == "--self-test" ]]; then
  conformance_gate_ok 55 0 0 55 || exit 1
  conformance_gate_ok 56 0 0 55 || exit 1
  if conformance_gate_ok 54 0 0 55; then exit 1; fi
  if conformance_gate_ok 0 0 0 55; then exit 1; fi
  if conformance_gate_ok 55 1 0 55; then exit 1; fi
  if conformance_gate_ok 54 0 1 55; then exit 1; fi
  if conformance_gate_ok 55 0 1 55; then exit 1; fi
  echo "conformance gate self-test ok"
  exit 0
fi

shopt -s nullglob
fixtures=("$FIXTURE_DIR"/*.js)
if [[ ${#fixtures[@]} -eq 0 ]]; then
  echo "No fixtures found in $FIXTURE_DIR" >&2
  exit 1
fi

echo "Amber Node conformance scorecard"
echo "Binary: ${AMBER[*]}"
echo "Fixtures: ${#fixtures[@]}"
echo

for fixture in "${fixtures[@]}"; do
  name="$(basename "$fixture")"
  out="$(mktemp)"
  err="$(mktemp)"
  extra=()
  policy="${fixture%.js}.policy.json"
  if [[ -f "$policy" ]]; then
    extra+=(--permission-policy "$policy")
  fi
  flags="${fixture%.js}.flags"
  if [[ -f "$flags" ]]; then
    while IFS= read -r flag || [[ -n "$flag" ]]; do
      [[ -z "$flag" || "$flag" == \#* ]] && continue
      extra+=("$flag")
    done <"$flags"
  fi
  # macOS bash 3.2 + `set -u` treats empty "${arr[@]}" as unbound.
  if [[ ${#extra[@]} -gt 0 ]]; then
    run_cmd=("${AMBER[@]}" run "${extra[@]}" "$fixture")
  else
    run_cmd=("${AMBER[@]}" run "$fixture")
  fi
  if command -v timeout >/dev/null 2>&1; then
    if timeout 30 "${run_cmd[@]}" >"$out" 2>"$err"; then
      run_ok=1
    else
      run_ok=0
    fi
  else
    if "${run_cmd[@]}" >"$out" 2>"$err"; then
      run_ok=1
    else
      run_ok=0
    fi
  fi
  if [[ "$run_ok" -eq 1 ]]; then
    if grep -q "^CONFORMANCE_SKIP$" "$out"; then
      echo "SKIP  $name"
      RESULTS+=("| $name | SKIP | optional fixture |")
      SKIP=$((SKIP + 1))
    elif grep -q "^CONFORMANCE_PASS$" "$out"; then
      echo "PASS  $name"
      RESULTS+=("| $name | PASS |")
      PASS=$((PASS + 1))
    else
      echo "FAIL  $name (missing CONFORMANCE_PASS marker)"
      RESULTS+=("| $name | FAIL | missing marker |")
      FAIL=$((FAIL + 1))
    fi
  else
    echo "FAIL  $name"
    tail -n 5 "$err" | sed 's/^/      /'
    RESULTS+=("| $name | FAIL | runtime error |")
    FAIL=$((FAIL + 1))
  fi
  rm -f "$out" "$err"
done

TOTAL=$((PASS + FAIL + SKIP))
RATE=0
if [[ $TOTAL -gt 0 ]]; then
  RATE=$((PASS * 100 / TOTAL))
fi

echo
echo "Summary: $PASS/$TOTAL passed (${RATE}%)"
if conformance_gate_ok "$PASS" "$FAIL" "$SKIP" "$REQUIRED_PASS"; then
  echo "Gate: PASS (>= ${REQUIRED_PASS} PASS, 0 FAIL, 0 SKIP)"
  GATE_LINE="**CI gate: PASS (>= ${REQUIRED_PASS} PASS, 0 FAIL, 0 SKIP)**"
else
  echo "Gate: FAIL (need >= ${REQUIRED_PASS} PASS, 0 FAIL, 0 SKIP; got ${PASS} PASS, ${FAIL} FAIL, ${SKIP} SKIP)" >&2
  GATE_LINE="**CI gate: FAIL (need >= ${REQUIRED_PASS} PASS, 0 FAIL, 0 SKIP)**"
fi

{
  echo "# Amber Node conformance scorecard"
  echo
  echo "Generated: $(date -u +%Y-%m-%dT%H:%MZ)"
  echo
  echo "| Fixture | Result | Notes |"
  echo "|---------|--------|-------|"
  for line in "${RESULTS[@]}"; do
    echo "$line"
  done
  echo
  echo "**Pass rate: ${PASS}/${TOTAL} (${RATE}%)**"
  echo
  echo "$GATE_LINE"
} >"$SCORECARD"

echo "Wrote $SCORECARD"
if [[ $FAIL -ne 0 ]]; then
  exit "$FAIL"
fi
if ! conformance_gate_ok "$PASS" "$FAIL" "$SKIP" "$REQUIRED_PASS"; then
  exit 1
fi
exit 0

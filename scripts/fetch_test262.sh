#!/usr/bin/env bash
# Fetch the pinned tc39/test262 commit into third_party/test262.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PIN_FILE="$ROOT/tests/test262/PIN"
DEST="$ROOT/third_party/test262"
REPO_URL="${TEST262_REPO_URL:-https://github.com/tc39/test262.git}"

if [[ ! -f "$PIN_FILE" ]]; then
  echo "missing pin file: $PIN_FILE" >&2
  exit 1
fi

PIN="$(tr -d '[:space:]' <"$PIN_FILE")"
if [[ -z "$PIN" || ${#PIN} -lt 7 ]]; then
  echo "invalid PIN in $PIN_FILE" >&2
  exit 1
fi

mkdir -p "$ROOT/third_party"

if [[ -d "$DEST/.git" ]]; then
  git -C "$DEST" fetch --depth 1 origin "$PIN" 2>/dev/null \
    || git -C "$DEST" fetch --depth 1 origin "+refs/heads/main:refs/remotes/origin/main"
  # Prefer exact pin; fall back to fetch by SHA if shallow missed it.
  if ! git -C "$DEST" cat-file -e "${PIN}^{commit}" 2>/dev/null; then
    git -C "$DEST" fetch --depth 1 origin "$PIN"
  fi
  git -C "$DEST" checkout --detach "$PIN"
else
  rm -rf "$DEST"
  git clone --filter=blob:none --sparse "$REPO_URL" "$DEST"
  git -C "$DEST" sparse-checkout set harness test features.txt LICENSE README.md
  if ! git -C "$DEST" cat-file -e "${PIN}^{commit}" 2>/dev/null; then
    git -C "$DEST" fetch --depth 1 origin "$PIN"
  fi
  git -C "$DEST" checkout --detach "$PIN"
fi

HEAD="$(git -C "$DEST" rev-parse HEAD)"
if [[ "$HEAD" != "$PIN" ]]; then
  echo "test262 HEAD $HEAD does not match PIN $PIN" >&2
  exit 1
fi

echo "test262 ready at $DEST"
echo "pin: $HEAD"
echo "tests: $(find "$DEST/test" -name '*.js' | wc -l)"

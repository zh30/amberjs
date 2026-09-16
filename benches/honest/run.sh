#!/usr/bin/env bash
# Honest Beejs micro-benchmarks. Prints commit, build profile, and hardware.
# Do not publish "faster than X" claims from this script unless every compared
# runtime is present and the same harness ran successfully.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

PROFILE="${HONEST_PROFILE:-release}"
if [[ "$PROFILE" == "release" ]]; then
  cargo build -q --release
  BEE="${BEE_BIN:-$ROOT/target/release/bee}"
else
  cargo build -q
  BEE="${BEE_BIN:-$ROOT/target/debug/bee}"
fi
"$BEE" --version >/dev/null 2>&1 || true

COMMIT="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
UNAME="$(uname -srm)"
CPU="$(sysctl -n machdep.cpu.brand_string 2>/dev/null || lscpu 2>/dev/null | awk -F: '/Model name/{print $2; exit}' | xargs || echo unknown)"

echo "Beejs honest benchmark"
echo "commit: $COMMIT"
echo "profile: $PROFILE"
echo "binary: $BEE"
echo "hardware: $UNAME / $CPU"
echo

time_ms() {
  python3 - "$BEE" "$@" <<'PY'
import subprocess, sys, time
bee = sys.argv[1]
args = sys.argv[2:]
start = time.perf_counter()
proc = subprocess.run([bee, *args], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
elapsed = (time.perf_counter() - start) * 1000
print(f"{elapsed:.1f}")
sys.exit(proc.returncode)
PY
}

echo -n "startup --version: "
time_ms --version
echo " ms"

echo -n "eval 1+1: "
time_ms eval "1+1"
echo " ms"

TMP="$(mktemp)"
python3 - <<'PY'
from pathlib import Path
import os
path = os.environ.get("HONEST_READ_FILE")
PY
python3 -c "import pathlib,os; pathlib.Path('$TMP').write_bytes(b'x'*1024*1024)"
READ_JS="$(mktemp)"
cat >"$READ_JS" <<EOF
const fs = require('fs');
fs.readFileSync($(python3 -c "import json; print(json.dumps('$TMP'))"));
console.log('ok');
EOF
echo -n "read 1MB: "
time_ms run "$READ_JS"
echo " ms"
rm -f "$TMP" "$READ_JS"

PORT_JS="$(mktemp)"
cat >"$PORT_JS" <<'EOF'
const http = require('http');
const port = Number(process.env.PORT || 0);
const server = http.createServer((_req, res) => { res.end('hello'); });
server.listen(18765, '127.0.0.1');
EOF
echo "HTTP hello: start bee run in background, curl once, then stop"
PORT=18765
"$BEE" run "$PORT_JS" >/dev/null 2>&1 &
HTTP_PID=$!
python3 - <<'PY'
import socket, time, sys
deadline = time.time() + 8
body = None
while time.time() < deadline:
    try:
        s = socket.create_connection(("127.0.0.1", 18765), timeout=0.2)
        s.sendall(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        body = s.recv(4096)
        s.close()
        if b"hello" in body:
            print("HTTP hello: ok")
            sys.exit(0)
    except OSError:
        time.sleep(0.05)
print("HTTP hello: skipped (server did not answer)")
sys.exit(0)
PY
kill "$HTTP_PID" 2>/dev/null || true
wait "$HTTP_PID" 2>/dev/null || true
rm -f "$PORT_JS"

IDLE_SECS="${HONEST_IDLE_SECS:-5}"
echo -n "idle ${IDLE_SECS}s bee session --sandbox (user+sys if available): "
python3 - "$BEE" "$IDLE_SECS" <<'PY'
import subprocess, sys, time, os, shutil
bee, idle_secs = sys.argv[1], float(sys.argv[2])
tool = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))), "examples", "agent", "echo_tool.ts")
# Fallback: repo-root relative from benches/honest
root = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
tool = os.path.join(root, "examples", "agent", "echo_tool.ts")
cmd = [bee, "session", "--sandbox", tool]
start = time.perf_counter()
proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(idle_secs)
proc.terminate()
try:
    proc.wait(timeout=2)
except subprocess.TimeoutExpired:
    proc.kill()
    proc.wait()
print(f"{(time.perf_counter()-start)*1000:.1f} ms wall")
PY

FETCH_ITERS="${HONEST_FETCH_ITERS:-200}"
FETCH_JS="$(mktemp)"
cat >"$FETCH_JS" <<EOF
let denied = 0;
let other = 0;
for (let i = 0; i < $FETCH_ITERS; i++) {
  try { fetch("http://127.0.0.1:9/honest-leak"); }
  catch (error) {
    const text = String(error && error.message || error);
    if (text.includes("permission")) denied += 1;
    else other += 1;
  }
}
console.log("fetch-iter=" + $FETCH_ITERS + " denied=" + denied + " other=" + other);
EOF
echo -n "sandbox fetch ${FETCH_ITERS}x: "
time_ms run --sandbox --allow-net 127.0.0.1 "$FETCH_JS"
echo " ms"
rm -f "$FETCH_JS"

echo
echo "Optional peer runtimes:"
for peer in node bun; do
  if command -v "$peer" >/dev/null 2>&1; then
    echo -n "  $peer --version: "
    "$peer" --version | head -n 1
  else
    echo "  $peer: skipped (not installed)"
  fi
done

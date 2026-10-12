#!/usr/bin/env python3
"""Side-by-side honest timings for Amber, Node, and Bun.

`benches/honest/run.sh` builds Amber and invokes this script. Every compared
runtime runs the same metric. Numbers are medians of repeated process samples
after discarded warmups (page cache, and Amber's on-disk startup snapshot if
the CLI creates one). Ratios are amber/peer: values above 1 mean Amber is
slower or uses more CPU.

This script does not read historical performance docs.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import socket
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

CLK_TCK = os.sysconf(os.sysconf_names["SC_CLK_TCK"])


def median(values: list[float]) -> float | None:
    if not values:
        return None
    return float(statistics.median(values))


def ratio(numerator: float | None, denominator: float | None) -> float | None:
    if numerator is None or denominator is None or denominator == 0:
        return None
    return numerator / denominator


def fmt_ms(value: float | None) -> str:
    if value is None:
        return "n/a"
    return f"{value:.1f} ms"


def fmt_ratio(value: float | None, numerator: float | None = None, denominator: float | None = None) -> str:
    if value is not None:
        return f"{value:.2f}x"
    if numerator is not None and denominator == 0 and numerator > 0:
        return "inf"
    return "n/a"


def command_text(argv: list[str]) -> str:
    return " ".join(argv)


def which(name: str | None) -> str | None:
    if not name:
        return None
    if os.path.isabs(name) and os.access(name, os.X_OK):
        return name
    return shutil.which(name)


def run_capture(argv: list[str], env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        argv,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
        check=False,
    )


def version_line(binary: str) -> str:
    proc = run_capture([binary, "--version"])
    text = (proc.stdout or proc.stderr or "").strip()
    if not text:
        return f"exit {proc.returncode}"
    return text.splitlines()[0].strip()


def cpu_model() -> str:
    try:
        for line in Path("/proc/cpuinfo").read_text().splitlines():
            if line.lower().startswith("model name"):
                return line.split(":", 1)[1].strip()
    except OSError:
        pass
    return platform.processor() or "unknown"


def proc_cpu_ms(pid: int) -> float | None:
    try:
        data = Path(f"/proc/{pid}/stat").read_text()
    except OSError:
        return None
    end = data.rfind(")")
    if end < 0:
        return None
    fields = data[end + 2 :].split()
    # utime, stime, cutime, cstime are fields 14-17 (indexes 11-14 after comm).
    if len(fields) < 15:
        return None
    ticks = sum(int(fields[i]) for i in (11, 12, 13, 14))
    return ticks * 1000.0 / CLK_TCK


def proc_rss_kb(pid: int) -> int | None:
    try:
        for line in Path(f"/proc/{pid}/status").read_text().splitlines():
            if line.startswith("VmRSS:"):
                return int(line.split()[1])
    except (OSError, IndexError, ValueError):
        return None
    return None


def time_process(argv: list[str], env: dict[str, str] | None = None) -> tuple[float, int]:
    start = time.perf_counter()
    proc = subprocess.run(
        argv,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        env=env,
        check=False,
    )
    elapsed_ms = (time.perf_counter() - start) * 1000.0
    return elapsed_ms, proc.returncode


def sample_process(
    argv: list[str],
    samples: int,
    warmup: int,
    env: dict[str, str] | None = None,
) -> dict:
    failures: list[dict] = []
    for _ in range(warmup):
        elapsed, code = time_process(argv, env)
        if code != 0:
            failures.append({"phase": "warmup", "ms": elapsed, "exit_code": code})
    measured: list[float] = []
    for _ in range(samples):
        elapsed, code = time_process(argv, env)
        if code != 0:
            failures.append({"phase": "sample", "ms": elapsed, "exit_code": code})
            continue
        measured.append(elapsed)
    stderr = ""
    if failures:
        proc = run_capture(argv, env)
        stderr = (proc.stderr or proc.stdout or "")[:800]
    return {
        "command": command_text(argv),
        "median_ms": median(measured),
        "min_ms": min(measured) if measured else None,
        "max_ms": max(measured) if measured else None,
        "samples_ms": [round(v, 3) for v in measured],
        "warmup": warmup,
        "requested_samples": samples,
        "failures": failures,
        "failure_excerpt": stderr,
    }


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def recv_http(sock: socket.socket) -> bytes:
    chunks: list[bytes] = []
    while True:
        try:
            chunk = sock.recv(4096)
        except socket.timeout:
            break
        if not chunk:
            break
        chunks.append(chunk)
        if b"hello" in b"".join(chunks):
            break
    return b"".join(chunks)


def http_exchange(port: int, timeout: float) -> bytes:
    with socket.create_connection(("127.0.0.1", port), timeout=timeout) as sock:
        sock.settimeout(timeout)
        sock.sendall(
            b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
        )
        return recv_http(sock)


def stop_process(proc: subprocess.Popen) -> None:
    if proc.poll() is not None:
        return
    proc.terminate()
    try:
        proc.wait(timeout=2)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=2)


def wait_for_hello(port: int, deadline: float) -> float | None:
    start = time.perf_counter()
    while time.perf_counter() < deadline:
        try:
            body = http_exchange(port, 0.2)
        except OSError:
            time.sleep(0.001)
            continue
        if b"hello" in body:
            return (time.perf_counter() - start) * 1000.0
        time.sleep(0.001)
    return None


def write_http_script(directory: Path, port: int) -> Path:
    script = directory / f"http_hello_{port}.js"
    script.write_text(
        "const http = require('http');\n"
        "const server = http.createServer((_req, res) => { res.end('hello'); });\n"
        f"server.listen({port}, '127.0.0.1');\n",
        encoding="utf-8",
    )
    return script


def sample_http_cold(binary: str, run_argv: list[str], samples: int, warmup: int) -> dict:
    measured: list[float] = []
    failures: list[str] = []

    def once() -> float | None:
        port = free_port()
        with tempfile.TemporaryDirectory(prefix="honest-http-") as tmp:
            script = write_http_script(Path(tmp), port)
            argv = [binary, *run_argv, str(script)]
            proc = subprocess.Popen(
                argv,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                start_new_session=True,
            )
            try:
                elapsed = wait_for_hello(port, time.perf_counter() + 8.0)
                if elapsed is None:
                    failures.append(
                        f"no hello from {command_text(argv)} exit={proc.poll()}"
                    )
                    return None
                return elapsed
            finally:
                stop_process(proc)

    for _ in range(warmup):
        once()
    for _ in range(samples):
        elapsed = once()
        if elapsed is not None:
            measured.append(elapsed)
    return {
        "command": f"{binary} {' '.join(run_argv)} <http-hello.js>",
        "median_ms": median(measured),
        "min_ms": min(measured) if measured else None,
        "max_ms": max(measured) if measured else None,
        "samples_ms": [round(v, 3) for v in measured],
        "warmup": warmup,
        "requested_samples": samples,
        "failures": failures[:6],
        "note": "wall time from process spawn until the first response body contains hello",
    }


def sample_http_warm_and_idle(
    binary: str,
    run_argv: list[str],
    warm_requests: int,
    idle_secs: float,
    idle_repeats: int,
) -> dict:
    """One listening server: warm request latencies, then idle CPU with no traffic."""
    warm_all: list[float] = []
    idle_cpu: list[float] = []
    idle_rss: list[int] = []
    failures: list[str] = []
    for _ in range(idle_repeats):
        port = free_port()
        with tempfile.TemporaryDirectory(prefix="honest-http-warm-") as tmp:
            script = write_http_script(Path(tmp), port)
            argv = [binary, *run_argv, str(script)]
            proc = subprocess.Popen(
                argv,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                start_new_session=True,
            )
            try:
                ready = wait_for_hello(port, time.perf_counter() + 8.0)
                if ready is None or proc.poll() is not None or proc.pid is None:
                    failures.append(
                        f"server not ready {command_text(argv)} exit={proc.poll()}"
                    )
                    continue
                latencies: list[float] = []
                for _req in range(warm_requests):
                    t0 = time.perf_counter()
                    try:
                        body = http_exchange(port, 1.0)
                    except OSError as exc:
                        failures.append(f"warm request failed: {exc}")
                        break
                    if b"hello" not in body:
                        failures.append("warm request missing hello")
                        break
                    latencies.append((time.perf_counter() - t0) * 1000.0)
                warm_all.extend(latencies)
                if proc.poll() is not None:
                    continue
                time.sleep(0.3)
                cpu_before = proc_cpu_ms(proc.pid)
                if cpu_before is None:
                    failures.append("could not read idle cpu")
                    continue
                time.sleep(idle_secs)
                if proc.poll() is not None:
                    failures.append("server exited during idle window")
                    continue
                cpu_after = proc_cpu_ms(proc.pid)
                rss = proc_rss_kb(proc.pid)
                if cpu_after is None:
                    failures.append("could not read idle cpu after window")
                    continue
                idle_cpu.append(cpu_after - cpu_before)
                if rss is not None:
                    idle_rss.append(rss)
            finally:
                stop_process(proc)
    return {
        "warm": {
            "command": f"{binary} {' '.join(run_argv)} <http-hello.js> + GET /",
            "median_ms": median(warm_all),
            "min_ms": min(warm_all) if warm_all else None,
            "max_ms": max(warm_all) if warm_all else None,
            "samples_ms": [round(v, 3) for v in warm_all],
            "requested_samples": warm_requests * idle_repeats,
            "note": "request latency after the server has already answered once",
        },
        "idle_cpu": {
            "median_ms": median(idle_cpu),
            "min_ms": min(idle_cpu) if idle_cpu else None,
            "max_ms": max(idle_cpu) if idle_cpu else None,
            "samples_ms": [round(v, 3) for v in idle_cpu],
            "window_secs": idle_secs,
            "rss_kb_median": median([float(v) for v in idle_rss]),
            "note": "user+sys CPU milliseconds over the idle window; no requests",
        },
        "failures": failures[:8],
    }


def sample_idle_session(
    argv: list[str],
    idle_secs: float,
    settle_secs: float,
    repeats: int,
) -> dict:
    measured: list[float] = []
    rss_values: list[int] = []
    walls: list[float] = []
    failures: list[str] = []
    for _ in range(repeats):
        proc = subprocess.Popen(
            argv,
            stdin=subprocess.PIPE,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
        try:
            time.sleep(settle_secs)
            if proc.poll() is not None or proc.pid is None:
                failures.append(f"exited during settle exit={proc.returncode}")
                continue
            cpu_before = proc_cpu_ms(proc.pid)
            if cpu_before is None:
                failures.append("could not read /proc stat")
                continue
            wall_start = time.perf_counter()
            time.sleep(idle_secs)
            wall = time.perf_counter() - wall_start
            if proc.poll() is not None:
                failures.append("exited during idle window")
                continue
            cpu_after = proc_cpu_ms(proc.pid)
            rss = proc_rss_kb(proc.pid)
            if cpu_after is None:
                failures.append("could not read /proc stat after window")
                continue
            measured.append(cpu_after - cpu_before)
            walls.append(wall * 1000.0)
            if rss is not None:
                rss_values.append(rss)
        finally:
            if proc.stdin is not None:
                try:
                    proc.stdin.close()
                except OSError:
                    pass
            stop_process(proc)
    return {
        "command": command_text(argv),
        "median_ms": median(measured),
        "min_ms": min(measured) if measured else None,
        "max_ms": max(measured) if measured else None,
        "samples_ms": [round(v, 3) for v in measured],
        "wall_ms_median": median(walls),
        "rss_kb_median": median([float(v) for v in rss_values]),
        "settle_secs": settle_secs,
        "window_secs": idle_secs,
        "failures": failures[:6],
        "note": "user+sys CPU milliseconds while the process is parked after settle",
    }


def peer_result(block: dict | None, key: str = "median_ms") -> float | None:
    if not block:
        return None
    value = block.get(key)
    if value is None:
        return None
    return float(value)


def metric_row(
    metric_id: str,
    label: str,
    unit: str,
    amber: dict | None,
    node: dict | None,
    bun: dict | None,
    note: str,
) -> dict:
    amber_v = peer_result(amber)
    node_v = peer_result(node)
    bun_v = peer_result(bun)
    return {
        "id": metric_id,
        "label": label,
        "unit": unit,
        "lower_is_better": True,
        "amber": amber,
        "node": node,
        "bun": bun,
        "amber_over_node": ratio(amber_v, node_v),
        "amber_over_bun": ratio(amber_v, bun_v),
        "note": note,
    }


def cell(block: dict | None, missing: str) -> str:
    if block is None:
        return missing
    value = block.get("median_ms")
    if value is None:
        return "fail"
    return fmt_ms(float(value))


def markdown_table(metrics: list[dict], node_missing: str, bun_missing: str) -> str:
    lines = [
        "| metric | amber | node | bun | amber/node | amber/bun |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for metric in metrics:
        lines.append(
            "| {label} | {amber} | {node} | {bun} | {an} | {ab} |".format(
                label=metric["label"],
                amber=cell(metric.get("amber"), "n/a"),
                node=cell(metric.get("node"), node_missing),
                bun=cell(metric.get("bun"), bun_missing),
                an=fmt_ratio(
                    metric.get("amber_over_node"),
                    peer_result(metric.get("amber")),
                    peer_result(metric.get("node")),
                ),
                ab=fmt_ratio(
                    metric.get("amber_over_bun"),
                    peer_result(metric.get("amber")),
                    peer_result(metric.get("bun")),
                ),
            )
        )
    return "\n".join(lines)


def time_verbose_once(argv: list[str]) -> dict:
    if not Path("/usr/bin/time").exists():
        return {"skipped": "GNU time not available"}
    start = time.perf_counter()
    proc = subprocess.run(
        ["/usr/bin/time", "-v", *argv],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        check=False,
    )
    elapsed = (time.perf_counter() - start) * 1000.0
    text = proc.stderr.decode("utf-8", "replace")
    interesting = {}
    for line in text.splitlines():
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        key = key.strip()
        if key in {
            "User time (seconds)",
            "System time (seconds)",
            "Maximum resident set size (kbytes)",
            "Major (requiring I/O) page faults",
            "Minor (reclaiming a frame) page faults",
            "Voluntary context switches",
            "Involuntary context switches",
            "File system inputs",
            "File system outputs",
        }:
            interesting[key] = value.strip()
    return {"wall_ms": round(elapsed, 3), "exit_code": proc.returncode, "time_v": interesting}


def startup_profile(amber: str) -> dict:
    env = os.environ.copy()
    env["AMBER_TRACE_STARTUP"] = "1"
    env["AMBER_PROFILE_STARTUP"] = "1"
    proc = run_capture([amber, "eval", "1+1"], env)
    disabled = os.environ.copy()
    disabled["AMBER_DISABLE_STARTUP_SNAPSHOT"] = "1"
    t0 = time.perf_counter()
    disabled_proc = subprocess.run(
        [amber, "eval", "1+1"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        env=disabled,
        check=False,
    )
    disabled_ms = (time.perf_counter() - t0) * 1000.0
    status = run_capture([amber, "snapshot", "status"])
    return {
        "trace_exit": proc.returncode,
        "trace_stderr": (proc.stderr or "")[:2000],
        "snapshot_disabled_eval_ms": round(disabled_ms, 3),
        "snapshot_disabled_exit": disabled_proc.returncode,
        "snapshot_status": (status.stdout or status.stderr or "")[:1500],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--amber", required=True)
    parser.add_argument("--node", default=os.environ.get("HONEST_NODE", "node"))
    parser.add_argument("--bun", default=os.environ.get("HONEST_BUN", "bun"))
    parser.add_argument("--profile", default=os.environ.get("HONEST_PROFILE", "release"))
    parser.add_argument("--root", default=str(Path(__file__).resolve().parents[2]))
    parser.add_argument("--samples", type=int, default=int(os.environ.get("HONEST_SAMPLES", "9")))
    parser.add_argument("--warmup", type=int, default=int(os.environ.get("HONEST_WARMUP", "2")))
    parser.add_argument("--http-samples", type=int, default=int(os.environ.get("HONEST_HTTP_SAMPLES", "5")))
    parser.add_argument("--warm-requests", type=int, default=int(os.environ.get("HONEST_WARM_REQUESTS", "40")))
    parser.add_argument("--idle-secs", type=float, default=float(os.environ.get("HONEST_IDLE_SECS", "3")))
    parser.add_argument("--idle-settle", type=float, default=float(os.environ.get("HONEST_IDLE_SETTLE", "1")))
    parser.add_argument("--idle-repeats", type=int, default=int(os.environ.get("HONEST_IDLE_REPEATS", "3")))
    parser.add_argument("--fetch-iters", type=int, default=int(os.environ.get("HONEST_FETCH_ITERS", "200")))
    parser.add_argument("--json", dest="json_path", default=os.environ.get("HONEST_JSON", ""))
    parser.add_argument("--skip-diagnostics", action="store_true")
    args = parser.parse_args()

    amber = which(args.amber)
    if amber is None:
        print(f"amber binary not executable: {args.amber}", file=sys.stderr)
        return 1
    node = which(args.node)
    bun = which(args.bun)
    root = Path(args.root)
    commit = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        stdout=subprocess.PIPE,
        text=True,
        check=False,
    ).stdout.strip()
    short = commit[:12] if commit else "unknown"

    node_missing = "skipped (not installed)"
    bun_missing = "skipped (not installed)"

    with tempfile.TemporaryDirectory(prefix="honest-bench-") as tmpdir:
        tmp = Path(tmpdir)
        payload = tmp / "one_mb.bin"
        payload.write_bytes(b"x" * (1024 * 1024))
        read_js = tmp / "read_1mb.js"
        read_js.write_text(
            "const fs = require('fs');\n"
            f"const buf = fs.readFileSync({json.dumps(str(payload))});\n"
            "if (buf.length !== 1048576) { throw new Error('bad length ' + buf.length); }\n",
            encoding="utf-8",
        )
        tiny_js = tmp / "tiny.js"
        tiny_js.write_text("console.log('ok');\n", encoding="utf-8")
        fetch_js = tmp / "sandbox_fetch.js"
        fetch_js.write_text(
            "let denied = 0;\nlet other = 0;\n"
            f"for (let i = 0; i < {args.fetch_iters}; i++) {{\n"
            "  try { fetch('http://127.0.0.1:9/honest-leak'); }\n"
            "  catch (error) {\n"
            "    const text = String(error && error.message || error);\n"
            "    if (text.includes('permission')) denied += 1;\n"
            "    else other += 1;\n"
            "  }\n"
            "}\n"
            f"if (denied + other !== {args.fetch_iters}) process.exit(1);\n",
            encoding="utf-8",
        )
        idle_js = tmp / "idle_timer.js"
        idle_js.write_text("setTimeout(() => {}, 1e9);\n", encoding="utf-8")
        echo_tool = root / "examples" / "agent" / "echo_tool.ts"

        process_metrics = [
            (
                "startup_version",
                "startup --version",
                "ms",
                ["--version"],
                ["--version"],
                "process wall time of --version",
            ),
            (
                "startup_tiny_eval",
                "tiny eval 0",
                "ms",
                ["eval", "0"],
                ["-e", "0"],
                "process wall time of a trivial expression",
            ),
            (
                "eval_1_plus_1",
                'eval "1+1"',
                "ms",
                ["eval", "1+1"],
                ["-e", "1+1"],
                "process wall time of eval 1+1 (peers use -e)",
            ),
            (
                "startup_script",
                "tiny script",
                "ms",
                ["run", str(tiny_js)],
                [str(tiny_js)],
                "process wall time of a one-line script",
            ),
            (
                "read_1mb",
                "read 1MB",
                "ms",
                ["run", str(read_js)],
                [str(read_js)],
                "process wall time including startup and fs.readFileSync of 1MB",
            ),
        ]

        results_by_id: dict[str, dict] = {}
        for metric_id, label, unit, amber_argv, peer_argv, note in process_metrics:
            amber_block = sample_process([amber, *amber_argv], args.samples, args.warmup)
            node_block = (
                sample_process([node, *peer_argv], args.samples, args.warmup) if node else None
            )
            bun_block = (
                sample_process([bun, *peer_argv], args.samples, args.warmup) if bun else None
            )
            results_by_id[metric_id] = metric_row(
                metric_id, label, unit, amber_block, node_block, bun_block, note
            )
            print(
                f"{label}: amber {cell(amber_block, 'n/a')} | "
                f"node {cell(node_block, node_missing)} | bun {cell(bun_block, bun_missing)}",
                flush=True,
            )

        amber_http = sample_http_cold(amber, ["run"], args.http_samples, min(1, args.warmup))
        node_http = sample_http_cold(node, [], args.http_samples, min(1, args.warmup)) if node else None
        bun_http = sample_http_cold(bun, [], args.http_samples, min(1, args.warmup)) if bun else None
        results_by_id["http_hello_cold"] = metric_row(
            "http_hello_cold",
            "HTTP hello cold",
            "ms",
            amber_http,
            node_http,
            bun_http,
            "spawn until first response body contains hello",
        )
        print(
            f"HTTP hello cold: amber {cell(amber_http, 'n/a')} | "
            f"node {cell(node_http, node_missing)} | bun {cell(bun_http, bun_missing)}",
            flush=True,
        )

        amber_live = sample_http_warm_and_idle(
            amber, ["run"], args.warm_requests, args.idle_secs, args.idle_repeats
        )
        node_live = (
            sample_http_warm_and_idle(node, [], args.warm_requests, args.idle_secs, args.idle_repeats)
            if node
            else None
        )
        bun_live = (
            sample_http_warm_and_idle(bun, [], args.warm_requests, args.idle_secs, args.idle_repeats)
            if bun
            else None
        )
        results_by_id["http_hello_warm"] = metric_row(
            "http_hello_warm",
            "HTTP hello warm",
            "ms",
            amber_live["warm"],
            None if node_live is None else node_live["warm"],
            None if bun_live is None else bun_live["warm"],
            "median request latency after the listener is up",
        )
        results_by_id["idle_http_cpu"] = metric_row(
            "idle_http_cpu",
            f"idle HTTP CPU ({args.idle_secs:.0f}s)",
            "cpu_ms",
            amber_live["idle_cpu"],
            None if node_live is None else node_live["idle_cpu"],
            None if bun_live is None else bun_live["idle_cpu"],
            "CPU time while an HTTP hello server handles no further requests",
        )
        print(
            f"HTTP hello warm: amber {cell(amber_live['warm'], 'n/a')} | "
            f"node {cell(None if node_live is None else node_live['warm'], node_missing)} | "
            f"bun {cell(None if bun_live is None else bun_live['warm'], bun_missing)}",
            flush=True,
        )
        print(
            f"idle HTTP CPU: amber {cell(amber_live['idle_cpu'], 'n/a')} | "
            f"node {cell(None if node_live is None else node_live['idle_cpu'], node_missing)} | "
            f"bun {cell(None if bun_live is None else bun_live['idle_cpu'], bun_missing)}",
            flush=True,
        )

        session_argv = [amber, "session", "--sandbox", str(echo_tool)]
        if echo_tool.is_file():
            amber_session = sample_idle_session(
                session_argv, args.idle_secs, args.idle_settle, args.idle_repeats
            )
        else:
            amber_session = {
                "median_ms": None,
                "failures": [f"missing {echo_tool}"],
                "command": command_text(session_argv),
            }
        node_park = (
            sample_idle_session(
                [node, str(idle_js)], args.idle_secs, args.idle_settle, args.idle_repeats
            )
            if node
            else None
        )
        bun_park = (
            sample_idle_session(
                [bun, str(idle_js)], args.idle_secs, args.idle_settle, args.idle_repeats
            )
            if bun
            else None
        )
        results_by_id["idle_sandbox_session"] = metric_row(
            "idle_sandbox_session",
            f"idle sandbox session CPU ({args.idle_secs:.0f}s)",
            "cpu_ms",
            amber_session,
            node_park,
            bun_park,
            "Amber is `session --sandbox`; peers park on a far setTimeout. CPU only, after settle.",
        )
        print(
            f"idle sandbox session CPU: amber {cell(amber_session, 'n/a')} | "
            f"node {cell(node_park, node_missing)} | bun {cell(bun_park, bun_missing)}",
            flush=True,
        )

        amber_timer_idle = sample_idle_session(
            [amber, "run", str(idle_js)], args.idle_secs, args.idle_settle, args.idle_repeats
        )
        results_by_id["idle_timer_cpu"] = metric_row(
            "idle_timer_cpu",
            f"idle setTimeout CPU ({args.idle_secs:.0f}s)",
            "cpu_ms",
            amber_timer_idle,
            node_park,
            bun_park,
            "Same far setTimeout script on every runtime. Peers reuse the parked-process samples.",
        )
        print(
            f"idle setTimeout CPU: amber {cell(amber_timer_idle, 'n/a')} | "
            f"node {cell(node_park, node_missing)} | bun {cell(bun_park, bun_missing)}",
            flush=True,
        )

        fetch_block = sample_process(
            [amber, "run", "--sandbox", "--allow-net", "127.0.0.1", str(fetch_js)],
            min(args.samples, 5),
            1,
        )
        fetch_only = {
            "id": "sandbox_fetch",
            "label": f"sandbox fetch {args.fetch_iters}x",
            "unit": "ms",
            "amber": fetch_block,
            "node": None,
            "bun": None,
            "amber_over_node": None,
            "amber_over_bun": None,
            "note": "Amber-only permission-denied fetch loop kept from the previous harness. Not a peer comparison.",
        }
        print(f"sandbox fetch: amber {cell(fetch_block, 'n/a')}", flush=True)

    order = [
        "startup_version",
        "startup_tiny_eval",
        "eval_1_plus_1",
        "startup_script",
        "read_1mb",
        "http_hello_cold",
        "http_hello_warm",
        "idle_http_cpu",
        "idle_sandbox_session",
        "idle_timer_cpu",
    ]
    metrics = [results_by_id[key] for key in order]
    read_delta = None
    read_med = peer_result(results_by_id["read_1mb"].get("amber"))
    script_med = peer_result(results_by_id["startup_script"].get("amber"))
    if read_med is not None and script_med is not None:
        read_delta = read_med - script_med

    diagnostics = {}
    if not args.skip_diagnostics:
        print("diagnostics: /usr/bin/time -v and startup trace", flush=True)
        diagnostics = {
            "amber_version_time_v": time_verbose_once([amber, "--version"]),
            "node_version_time_v": time_verbose_once([node, "--version"]) if node else None,
            "bun_version_time_v": time_verbose_once([bun, "--version"]) if bun else None,
            "amber_eval_time_v": time_verbose_once([amber, "eval", "1+1"]),
            "startup_profile": startup_profile(amber),
            "amber_read_minus_script_ms": None if read_delta is None else round(read_delta, 3),
        }

    binary_size = Path(amber).stat().st_size if Path(amber).exists() else None
    report = {
        "commit": commit,
        "commit_short": short,
        "profile": args.profile,
        "cargo_profile": "release" if args.profile == "release" else args.profile,
        "amber_bin": amber,
        "amber_bin_bytes": binary_size,
        "amber_version": version_line(amber),
        "node_bin": node,
        "node_version": version_line(node) if node else None,
        "bun_bin": bun,
        "bun_version": version_line(bun) if bun else None,
        "os": " ".join(platform.uname()),
        "cpu": cpu_model(),
        "logical_cpus": os.cpu_count(),
        "samples": args.samples,
        "warmup": args.warmup,
        "http_samples": args.http_samples,
        "warm_requests": args.warm_requests,
        "idle_secs": args.idle_secs,
        "idle_settle_secs": args.idle_settle,
        "idle_repeats": args.idle_repeats,
        "metrics": metrics,
        "amber_only": [fetch_only],
        "diagnostics": diagnostics,
        "ratio_meaning": "amber/peer above 1 means Amber took more time or CPU than that peer",
    }

    table = markdown_table(metrics, node_missing, bun_missing)
    print()
    print(table)
    if args.json_path:
        Path(args.json_path).write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(f"\njson: {args.json_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

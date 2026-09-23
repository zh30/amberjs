#!/usr/bin/env python3
"""Amber vs Node.js vs Bun comprehensive benchmark suite.

Phases:
1. Cold start and short-lived CLI latency
2. In-process microbenchmarks
3. HTTP / framework throughput and RSS
4. Node.js Conformance 5.0
5. Client I/O and SQLite
6. amber:ai tensor operators
"""

from __future__ import annotations

import http.client
import json
import os
import socket
import statistics
import subprocess
import threading
import time
import urllib.error
import urllib.parse
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
AMBER_BIN = ROOT / "target" / "release" / "amber"
AUTOCANNON_JS = (
    ROOT
    / "benchmarks"
    / "idle_memory"
    / "node_modules"
    / "autocannon"
    / "autocannon.js"
)
OUTPUT_JSON = ROOT / "benchmarks" / "comprehensive_results.json"
OUTPUT_MD = ROOT / "benchmarks" / "COMPREHENSIVE_BENCHMARK_REPORT.md"

RuntimeMap = dict[str, Any]


def _run(cmd: list[str], **kwargs: Any) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, check=False, text=True, **kwargs)


def get_system_info() -> dict[str, str]:
    uname = subprocess.check_output(["uname", "-srm"], text=True).strip()
    cpu = "Unknown CPU"
    for key in ("machdep.cpu.brand_string", "hw.model"):
        try:
            cpu = subprocess.check_output(["sysctl", "-n", key], text=True).strip()
            break
        except (OSError, subprocess.CalledProcessError):
            continue
    node_ver = subprocess.check_output(["node", "--version"], text=True).strip()
    bun_ver = subprocess.check_output(["bun", "--version"], text=True).strip()
    amber_ver = subprocess.check_output(
        [str(AMBER_BIN), "--version"], text=True
    ).strip()
    return {
        "os_kernel": uname,
        "cpu": cpu,
        "node_version": node_ver,
        "bun_version": bun_ver,
        "amber_version": amber_ver,
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }


def get_process_rss_mb(pid: int) -> float:
    try:
        out = subprocess.check_output(
            ["ps", "-o", "rss=", "-p", str(pid)],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
        return float(out.split()[0]) / 1024.0
    except (OSError, subprocess.CalledProcessError, ValueError, IndexError):
        return 0.0


def measure_cli_latency(cmd: list[str], iters: int = 20) -> dict[str, float]:
    timings_ms: list[float] = []
    for _ in range(iters):
        start = time.perf_counter()
        _run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        timings_ms.append((time.perf_counter() - start) * 1000.0)
    timings_sorted = sorted(timings_ms)
    p95_idx = min(int(len(timings_sorted) * 0.95), len(timings_sorted) - 1)
    return {
        "mean_ms": round(statistics.mean(timings_ms), 2),
        "min_ms": round(min(timings_ms), 2),
        "max_ms": round(max(timings_ms), 2),
        "stddev_ms": round(
            statistics.stdev(timings_ms) if len(timings_ms) > 1 else 0.0, 2
        ),
        "p95_ms": round(timings_sorted[p95_idx], 2),
    }


def run_bench_part1_startup() -> RuntimeMap:
    print("\n========================================================")
    print("▶ Phase 1: Cold Start & Short-Lived CLI Execution Latency")
    print("========================================================")
    workloads: list[tuple[str, dict[str, list[str]]]] = [
        (
            "eval '1 + 1'",
            {
                "amber": [str(AMBER_BIN), "eval", "1 + 1"],
                "node": ["node", "-e", "1 + 1"],
                "bun": ["bun", "-e", "1 + 1"],
            },
        ),
        (
            "eval '1 + 1' (--warm)",
            {
                "amber": [str(AMBER_BIN), "eval", "--warm", "1 + 1"],
                "node": ["node", "-e", "1 + 1"],
                "bun": ["bun", "-e", "1 + 1"],
            },
        ),
        (
            "eval console.log('hello')",
            {
                "amber": [str(AMBER_BIN), "eval", "console.log('hello')"],
                "node": ["node", "-e", "console.log('hello')"],
                "bun": ["bun", "-e", "console.log('hello')"],
            },
        ),
        (
            "run hello_world.js",
            {
                "amber": [str(AMBER_BIN), "run", "examples/basics/hello_world.js"],
                "node": ["node", "examples/basics/hello_world.js"],
                "bun": ["bun", "examples/basics/hello_world.js"],
            },
        ),
        (
            "run hello_world.js (--warm)",
            {
                "amber": [
                    str(AMBER_BIN),
                    "run",
                    "--warm",
                    "examples/basics/hello_world.js",
                ],
                "node": ["node", "examples/basics/hello_world.js"],
                "bun": ["bun", "examples/basics/hello_world.js"],
            },
        ),
        (
            "run TypeScript (.ts)",
            {
                "amber": [str(AMBER_BIN), "run", "examples/basics/typescript_demo.ts"],
                "bun": ["bun", "examples/basics/typescript_demo.ts"],
            },
        ),
        (
            "test runner (math.test.js)",
            {
                "amber": [str(AMBER_BIN), "test", "examples/testing/math.test.js"],
                "bun": ["bun", "test", "examples/testing/math.test.js"],
            },
        ),
    ]
    results: RuntimeMap = {}
    for name, cmds in workloads:
        print(f"  • Measuring '{name}' (20 iterations)...")
        results[name] = {}
        for rt, cmd in cmds.items():
            stats = measure_cli_latency(cmd, iters=20)
            results[name][rt] = stats
            print(
                f"    - {rt.upper().ljust(5)}: {stats['mean_ms']:>6.2f} ms "
                f"(p95: {stats['p95_ms']:>6.2f} ms, min: {stats['min_ms']:>6.2f} ms)"
            )
    return results


def parse_json_array(stdout: str) -> list[Any]:
    start = stdout.find("[")
    end = stdout.rfind("]")
    if start < 0 or end < start:
        raise ValueError("no JSON array found in benchmark stdout")
    data = json.loads(stdout[start : end + 1])
    if not isinstance(data, list):
        raise TypeError("benchmark JSON is not an array")
    return data


def run_js_json_suite(
    label: str,
    bench_file: Path,
    env: dict[str, str] | None = None,
    amber_only: bool = False,
) -> RuntimeMap:
    runs: RuntimeMap = {}
    cmds: list[tuple[str, list[str]]] = [
        ("amber", [str(AMBER_BIN), "run", str(bench_file)])
    ]
    if not amber_only:
        cmds.extend(
            [
                ("node", ["node", str(bench_file)]),
                ("bun", ["bun", str(bench_file)]),
            ]
        )
    for rt, cmd in cmds:
        print(f"  • {label} on {rt.upper()}...")
        res = _run(cmd, capture_output=True, env=env, timeout=180)
        if res.returncode != 0:
            err = (res.stderr or res.stdout)[-400:]
            print(f"    ❌ Error running {rt}: {err}")
            continue
        try:
            data = parse_json_array(res.stdout)
        except (ValueError, json.JSONDecodeError) as exc:
            print(f"    ❌ Failed to parse JSON from {rt}: {exc}")
            continue
        runs[rt] = data
        skipped = sum(1 for item in data if item.get("skipped"))
        print(f"    ✓ {rt.upper()}: {len(data)} workloads ({skipped} skipped)")
    return runs


def run_bench_part2_microbenchmarks() -> RuntimeMap:
    print("\n========================================================")
    print("▶ Phase 2: In-Process Microbenchmark Suite (24 Workloads)")
    print("========================================================")
    return run_js_json_suite(
        "microbenchmarks", ROOT / "benchmarks" / "comprehensive_bench.js"
    )


def _start_keepalive_http_server(port: int) -> threading.Event:
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind(("127.0.0.1", port))
    sock.listen(128)
    sock.settimeout(0.5)
    stop = threading.Event()
    reply = (
        b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n"
        b"Content-Length: 2\r\nConnection: keep-alive\r\n\r\nok"
    )

    def handle(conn: socket.socket) -> None:
        try:
            conn.settimeout(2)
            buf = b""
            while not stop.is_set():
                try:
                    chunk = conn.recv(4096)
                except TimeoutError:
                    continue
                if not chunk:
                    break
                buf += chunk
                while b"\r\n\r\n" in buf:
                    conn.sendall(reply)
                    buf = buf.split(b"\r\n\r\n", 1)[1]
        except OSError:
            return
        finally:
            try:
                conn.close()
            except OSError:
                return

    def accept_loop() -> None:
        while not stop.is_set():
            try:
                conn, _unused = sock.accept()
            except TimeoutError:
                continue
            except OSError:
                break
            threading.Thread(target=handle, args=(conn,), daemon=True).start()
        try:
            sock.close()
        except OSError:
            return

    threading.Thread(target=accept_loop, daemon=True).start()
    return stop


def run_bench_part5_extended_io() -> RuntimeMap:
    print("\n========================================================")
    print("▶ Phase 5: Client Fetch, SQLite, and Persistence I/O")
    print("========================================================")
    port = 19199
    stop = _start_keepalive_http_server(port)
    env = os.environ.copy()
    env["BENCH_URL"] = f"http://127.0.0.1:{port}/"
    try:
        return run_js_json_suite(
            "extended I/O", ROOT / "benchmarks" / "extended_io_bench.js", env=env
        )
    finally:
        stop.set()


def run_bench_part6_ai_tensors() -> RuntimeMap:
    print("\n========================================================")
    print("▶ Phase 6: amber:ai Tensor Operator Acceleration")
    print("========================================================")
    return run_js_json_suite(
        "AI tensors",
        ROOT / "benchmarks" / "ai_tensor_bench.js",
        amber_only=True,
    )


def wait_for_server(url: str, timeout_sec: float = 8.0) -> bool:
    parsed = urllib.parse.urlparse(url)
    host = parsed.hostname or "127.0.0.1"
    port = parsed.port or 80
    path = parsed.path or "/"
    deadline = time.time() + timeout_sec
    while time.time() < deadline:
        conn = http.client.HTTPConnection(host, port, timeout=0.5)
        try:
            conn.request("GET", path)
            resp = conn.getresponse()
            if resp.status in (200, 404):
                return True
        except (
            OSError,
            TimeoutError,
            http.client.HTTPException,
            urllib.error.URLError,
        ):
            time.sleep(0.1)
            continue
        finally:
            conn.close()
        time.sleep(0.1)
    return False


def _proxy_free_env(port: int) -> dict[str, str]:
    env = os.environ.copy()
    env["PORT"] = str(port)
    env["no_proxy"] = "127.0.0.1,localhost"
    env["NO_PROXY"] = "127.0.0.1,localhost"
    for key in (
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
    ):
        env.pop(key, None)
    return env


def run_bench_part3_http_frameworks() -> RuntimeMap:
    print("\n========================================================")
    print("▶ Phase 3: HTTP Server & Web Framework Throughput / Latency")
    print("========================================================")
    workloads = [
        ("Raw HTTP", "benchmarks/idle_memory/server_http.js", 19101),
        ("Hono 4.x", "benchmarks/idle_memory/server_hono.js", 19102),
        ("Express 5.x", "benchmarks/idle_memory/server_express.js", 19103),
        ("Fastify 5.x", "benchmarks/idle_memory/server_fastify.js", 19104),
    ]
    runtimes = [
        ("amber", [str(AMBER_BIN), "run"]),
        ("node", ["node"]),
        ("bun", ["bun"]),
    ]
    results: RuntimeMap = {}
    for fw_name, script_path, base_port in workloads:
        print(f"\n  [Framework: {fw_name}]")
        results[fw_name] = {}
        for rt_idx, (rt, cmd_prefix) in enumerate(runtimes):
            port = base_port + rt_idx * 10
            url = f"http://127.0.0.1:{port}/"
            env = _proxy_free_env(port)
            server_proc = subprocess.Popen(
                cmd_prefix + [str(ROOT / script_path)],
                env=env,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            try:
                if not wait_for_server(url, timeout_sec=8.0):
                    print(f"    ❌ {rt.upper()}: Server failed to bind within timeout")
                    results[fw_name][rt] = {"error": "Server failed to bind"}
                    continue
                time.sleep(0.5)
                baseline_rss = get_process_rss_mb(server_proc.pid)
                ac_proc = _run(
                    [
                        "node",
                        str(AUTOCANNON_JS),
                        "-c",
                        "32",
                        "-d",
                        "5",
                        "-j",
                        url,
                    ],
                    capture_output=True,
                    env=env,
                )
                peak_rss = get_process_rss_mb(server_proc.pid)
                time.sleep(2.0)
                settled_rss = get_process_rss_mb(server_proc.pid)
                if ac_proc.returncode != 0:
                    print(f"    ❌ {rt.upper()}: Autocannon failed: {ac_proc.stderr}")
                    results[fw_name][rt] = {"error": ac_proc.stderr}
                    continue
                try:
                    ac_data = json.loads(ac_proc.stdout)
                except json.JSONDecodeError as exc:
                    results[fw_name][rt] = {"error": f"invalid autocannon JSON: {exc}"}
                    continue
                req_avg = ac_data.get("requests", {}).get("average", 0)
                lat_avg = ac_data.get("latency", {}).get("average", 0)
                p99_lat = ac_data.get("latency", {}).get("p99", 0)
                total_reqs = ac_data.get("requests", {}).get("total", 0)
                results[fw_name][rt] = {
                    "requests_per_sec": round(req_avg, 1),
                    "latency_avg_ms": round(lat_avg, 2),
                    "p99_latency_ms": round(p99_lat, 2),
                    "total_requests": total_reqs,
                    "baseline_rss_mb": round(baseline_rss, 1),
                    "peak_rss_mb": round(peak_rss, 1),
                    "settled_rss_mb": round(settled_rss, 1),
                }
                print(
                    f"    ✓ {rt.upper().ljust(5)}: {req_avg:>9.1f} req/s | "
                    f"{lat_avg:>5.2f} ms avg | RSS: {baseline_rss:.1f} -> "
                    f"{peak_rss:.1f} -> {settled_rss:.1f} MB"
                )
            finally:
                server_proc.terminate()
                try:
                    server_proc.wait(timeout=2.0)
                except subprocess.TimeoutExpired:
                    server_proc.kill()
                    server_proc.wait()
    return results


def run_bench_part4_conformance() -> dict[str, float | int]:
    print("\n========================================================")
    print("▶ Phase 4: Node.js Conformance 5.0 Test Suite Validation")
    print("========================================================")
    script = ROOT / "tests" / "conformance" / "run_conformance.sh"
    start = time.perf_counter()
    proc = _run([str(script)], capture_output=True)
    dur = time.perf_counter() - start
    passed = 0
    total = 0
    for line in proc.stdout.splitlines():
        if "passed (" not in line:
            continue
        parts = line.strip().split()[1].split("/")
        try:
            passed = int(parts[0])
            total = int(parts[1])
        except (ValueError, IndexError):
            passed = 0
            total = 0
        break
    rate = (passed / total) * 100 if total else 0.0
    print(f"  • Result: {passed}/{total} passed ({rate:.1f}%) in {dur:.2f}s")
    return {
        "passed": passed,
        "total": total,
        "pass_rate_pct": round(rate, 1),
        "duration_sec": round(dur, 2),
    }


def _ms(item: dict[str, Any]) -> float:
    return float(item.get("avgMs", 0) or 0)


def format_micro_table(data: RuntimeMap, title: str) -> list[str]:
    amber_items = {item["name"]: item for item in data.get("amber", [])}
    node_items = {item["name"]: item for item in data.get("node", [])}
    bun_items = {item["name"]: item for item in data.get("bun", [])}
    names = list(amber_items) or list(node_items) or list(bun_items)
    rows = [
        f"| {title} | Amber 耗时 (ms) | Node.js 耗时 (ms) | Bun 耗时 (ms) | Amber 吞吐 (ops/s) | 相对表现 |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for name in names:
        amber = amber_items.get(name, {})
        node = node_items.get(name, {})
        bun = bun_items.get(name, {})
        if amber.get("skipped"):
            rows.append(
                f"| **{name}** | SKIP | — | — | — | {amber.get('error', 'skipped')} |"
            )
            continue
        amber_ms = _ms(amber)
        node_ms = _ms(node)
        bun_ms = _ms(bun)
        amber_ops = float(amber.get("opsSec", 0) or 0)
        tags: list[str] = []
        if node_ms > 0:
            ratio = node_ms / amber_ms if amber_ms else 0
            if amber_ms < node_ms:
                tags.append(f"比 Node 快 {ratio:.2f}x")
            elif amber_ms / node_ms < 1.15:
                tags.append(f"与 Node 接近 ({amber_ms / node_ms:.2f}x)")
            else:
                tags.append(f"比 Node 慢 {amber_ms / node_ms:.2f}x")
        if bun_ms > 0 and amber_ms and amber_ms < bun_ms:
            tags.append(f"比 Bun 快 {bun_ms / amber_ms:.2f}x")
        tag_str = ", ".join(tags) if tags else "与主流表现相当"
        node_cell = (
            f"{node_ms:.2f}" if node_ms else ("SKIP" if node.get("skipped") else "—")
        )
        bun_cell = (
            f"{bun_ms:.2f}" if bun_ms else ("SKIP" if bun.get("skipped") else "—")
        )
        rows.append(
            f"| **{name}** | **{amber_ms:.2f}** | {node_cell} | {bun_cell} | "
            f"**{amber_ops:,.1f}** | {tag_str} |"
        )
    return rows


def _lookup(
    items: dict[str, Any], pattern: str
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    amber_items = {item["name"]: item for item in items.get("amber", [])}
    node_items = {item["name"]: item for item in items.get("node", [])}
    bun_items = {item["name"]: item for item in items.get("bun", [])}
    for key, value in amber_items.items():
        if pattern in key:
            return value, node_items.get(key, {}), bun_items.get(key, {})
    return {}, {}, {}


def generate_markdown_report(
    sys_info: dict[str, str],
    p1_data: RuntimeMap,
    p2_data: RuntimeMap,
    p3_data: RuntimeMap,
    p4_data: dict[str, float | int],
    p5_data: RuntimeMap | None = None,
    p6_data: RuntimeMap | None = None,
) -> str:
    amber_eval = p1_data.get("eval '1 + 1'", {}).get("amber", {}).get("mean_ms", 0)
    node_eval = p1_data.get("eval '1 + 1'", {}).get("node", {}).get("mean_ms", 0)
    bun_eval = p1_data.get("eval '1 + 1'", {}).get("bun", {}).get("mean_ms", 0)
    eval_vs_node = (node_eval / amber_eval) if amber_eval else 1.0
    eval_vs_bun = (bun_eval / amber_eval) if amber_eval and bun_eval else 0.0
    obj_a, obj_n, _obj_u = _lookup(p2_data, "Objects / Alloc")
    ee_a, ee_n, _ee_u = _lookup(p2_data, "EventEmitter")
    str_a, str_n, _str_u = _lookup(p2_data, "String & RegExp")
    md = [
        "# Amber 全面基准性能测试综合评估报告",
        "",
        f"> **生成时间**: `{sys_info['timestamp']}`  ",
        f"> **硬件环境**: `{sys_info['cpu']}` ({sys_info['os_kernel']})  ",
        (
            f"> **对比运行时**: **{sys_info['amber_version']}** vs "
            f"**Node.js {sys_info['node_version']}** vs **Bun {sys_info['bun_version']}**"
        ),
        "",
        "---",
        "",
        "## 1. 核心结论",
        "",
        (
            f"本次在 `{sys_info['cpu']}` 上对比 **{sys_info['amber_version']}**、"
            f"**Node.js {sys_info['node_version']}** 与 **Bun {sys_info['bun_version']}**。"
        ),
        "",
        "1. **冷启动与短命进程**",
        (
            f"   - Amber `eval '1 + 1'` 均值 **{amber_eval:.2f} ms**，"
            f"Node.js **{node_eval:.2f} ms**（{eval_vs_node:.2f}x vs Node），"
            f"Bun **{bun_eval:.2f} ms**（{eval_vs_bun:.2f}x vs Bun）。"
        ),
        "2. **核心运行时与 JIT**",
    ]
    if obj_a.get("opsSec"):
        n_ratio = (
            f"，相比 Node.js {obj_n.get('avgMs', 1) / obj_a.get('avgMs', 1):.2f}x"
            if obj_n.get("avgMs")
            else ""
        )
        md.append(
            f"   - 对象分配：Amber **{obj_a.get('opsSec', 0):.1f} ops/s**{n_ratio}。"
        )
    if ee_a.get("opsSec"):
        n_ratio = (
            f"，相比 Node.js {ee_n.get('avgMs', 1) / ee_a.get('avgMs', 1):.2f}x"
            if ee_n.get("avgMs")
            else ""
        )
        md.append(
            f"   - EventEmitter：Amber **{ee_a.get('opsSec', 0):.1f} ops/s**{n_ratio}。"
        )
    if str_a.get("opsSec") and str_n.get("avgMs") and str_a.get("avgMs"):
        md.append(
            f"   - 字符串/正则：相比 Node.js "
            f"{str_n.get('avgMs', 1) / str_a.get('avgMs', 1):.2f}x。"
        )
    exp = p3_data.get("Express 5.x", {}).get("amber", {})
    hono = p3_data.get("Hono 4.x", {}).get("amber", {})
    raw = p3_data.get("Raw HTTP", {}).get("amber", {})
    md.extend(
        [
            "3. **HTTP / 框架吞吐**",
            (
                f"   - Express 5.x：Amber **{exp.get('requests_per_sec', 0):,.1f} req/s**，"
                f"平均时延 **{exp.get('latency_avg_ms', 0):.2f} ms**。"
            ),
            f"   - Hono 4.x：Amber **{hono.get('requests_per_sec', 0):,.1f} req/s**。",
            f"   - Raw HTTP：Amber **{raw.get('requests_per_sec', 0):,.1f} req/s**。",
            "4. **Conformance**",
            (
                f"   - Node.js Conformance 5.0：{p4_data.get('passed', 0)}/"
                f"{p4_data.get('total', 0)} "
                f"（{p4_data.get('pass_rate_pct', 0)}%，{p4_data.get('duration_sec', 0)}s）。"
            ),
            "",
            "---",
            "",
            "## 2. 冷启动与 CLI 时延",
            "",
            "每项 20 次。越低越好。",
            "",
            (
                f"| 工作负载 | {sys_info['amber_version']} 均值 | Amber P95 | "
                f"{sys_info['node_version']} | {sys_info['bun_version']} | vs Node |"
            ),
            "|---|---|---|---|---|---|",
        ]
    )
    for name, rts in p1_data.items():
        amber_m = rts.get("amber", {}).get("mean_ms", "N/A")
        amber_p95 = rts.get("amber", {}).get("p95_ms", "N/A")
        node_m = rts.get("node", {}).get("mean_ms", "—")
        bun_m = rts.get("bun", {}).get("mean_ms", "—")
        if isinstance(amber_m, (int, float)) and isinstance(node_m, (int, float)):
            diff = f"{node_m / amber_m:.2f}x"
        else:
            diff = "—"
        amber_str = (
            f"{amber_m:.2f} ms" if isinstance(amber_m, (int, float)) else str(amber_m)
        )
        p95_str = (
            f"{amber_p95:.2f} ms"
            if isinstance(amber_p95, (int, float))
            else str(amber_p95)
        )
        node_str = (
            f"{node_m:.2f} ms" if isinstance(node_m, (int, float)) else str(node_m)
        )
        bun_str = f"{bun_m:.2f} ms" if isinstance(bun_m, (int, float)) else str(bun_m)
        md.append(
            f"| `{name}` | **{amber_str}** | {p95_str} | {node_str} | {bun_str} | {diff} |"
        )
    md.extend(
        [
            "",
            "## 3. 运行态微基准",
            "",
            "`benchmarks/comprehensive_bench.js`，24 项。",
            "",
        ]
    )
    md.extend(format_micro_table(p2_data, "基准项目"))
    md.extend(
        [
            "",
            "## 4. HTTP 与框架压测",
            "",
            "autocannon：32 connections，5 秒。",
            "",
            "| 框架 | 运行时 | req/s | 平均时延 | P99 | 总请求 | RSS Baseline → Peak → Settled |",
            "|---|---|---|---|---|---|---|",
        ]
    )
    for fw_name, rts in p3_data.items():
        for rt, data in rts.items():
            if "error" in data:
                md.append(f"| **{fw_name}** | {rt.upper()} | 异常 | — | — | — | — |")
                continue
            label = f"**{sys_info['amber_version']}**" if rt == "amber" else rt.upper()
            mem = (
                f"{data['baseline_rss_mb']:.1f} → {data['peak_rss_mb']:.1f} → "
                f"{data['settled_rss_mb']:.1f} MB"
            )
            md.append(
                f"| **{fw_name}** | {label} | **{data['requests_per_sec']:,.1f}** | "
                f"{data['latency_avg_ms']:.2f} ms | {data['p99_latency_ms']:.2f} ms | "
                f"{data['total_requests']:,} | {mem} |"
            )
    md.extend(
        [
            "",
            "## 5. Node.js Conformance 5.0",
            "",
            f"- 用例：`{p4_data['total']}`",
            f"- 通过：`{p4_data['passed']}`",
            f"- 通过率：`{p4_data['pass_rate_pct']}%`",
            f"- 耗时：`{p4_data['duration_sec']}s`",
            "",
            "## 6. 客户端 Fetch 与 SQLite",
            "",
        ]
    )
    if p5_data:
        md.extend(format_micro_table(p5_data, "扩展 I/O"))
    else:
        md.append("_未采集_")
    md.extend(["", "## 7. amber:ai Tensor", ""])
    if p6_data:
        md.extend(format_micro_table(p6_data, "AI Tensor"))
    else:
        md.append("_未采集_")
    md.extend(
        [
            "",
            "## 8. 说明",
            "",
            "- 数字来自本次命令，不是 SLA。",
            "- Amber CLI 为 `amber`；内置模块为 `amber:ai`。",
            "- HTTP 为单进程、本机 loopback、32 连接 5 秒 autocannon。",
            "",
        ]
    )
    return "\n".join(md)


def main() -> None:
    if not AMBER_BIN.is_file():
        raise SystemExit(
            f"missing release binary: {AMBER_BIN} (run cargo build --release)"
        )
    print("=======================================================================")
    print("Amber Comprehensive Benchmark Suite")
    print("=======================================================================")
    sys_info = get_system_info()
    print(f"• Target Binary : {AMBER_BIN}")
    print(f"• Platform      : {sys_info['cpu']} | {sys_info['os_kernel']}")
    print(
        f"• Runtimes      : {sys_info['amber_version']} vs Node "
        f"{sys_info['node_version']} vs Bun {sys_info['bun_version']}"
    )
    p1 = run_bench_part1_startup()
    p2 = run_bench_part2_microbenchmarks()
    p3 = run_bench_part3_http_frameworks()
    p4 = run_bench_part4_conformance()
    p5 = run_bench_part5_extended_io()
    p6 = run_bench_part6_ai_tensors()
    full_results = {
        "system_info": sys_info,
        "phase1_startup": p1,
        "phase2_microbenchmarks": p2,
        "phase3_http_frameworks": p3,
        "phase4_conformance": p4,
        "phase5_extended_io": p5,
        "phase6_ai_tensors": p6,
    }
    OUTPUT_JSON.write_text(json.dumps(full_results, indent=2))
    print(f"\n[✓] Raw JSON results saved to: {OUTPUT_JSON}")
    OUTPUT_MD.write_text(generate_markdown_report(sys_info, p1, p2, p3, p4, p5, p6))
    print(f"[✓] Comprehensive Markdown report saved to: {OUTPUT_MD}")
    failures, rss_notes = check_regressions(p2, p3, p4)
    for note in rss_notes:
        print(f"[rss] {note}")
    if failures:
        print("\nRegression gates failed:")
        for item in failures:
            print(f"  - {item}")
        raise SystemExit(1)
    print("\nBenchmark run completed successfully!")


def check_regressions(
    p2: RuntimeMap, p3: RuntimeMap, p4: dict[str, float | int]
) -> tuple[list[str], list[str]]:
    """Lock Express/URL/EventEmitter wins and conformance. RSS is advisory."""
    failures: list[str] = []
    rss_notes: list[str] = []
    passed = int(p4.get("passed") or 0)
    total = int(p4.get("total") or 0)
    if total and passed < total:
        failures.append(f"conformance {passed}/{total} (want {total}/{total})")

    express = p3.get("Express 5.x", {})
    amber_rps = float((express.get("amber") or {}).get("requests_per_sec") or 0)
    node_rps = float((express.get("node") or {}).get("requests_per_sec") or 0)
    if node_rps and amber_rps < 2.0 * node_rps:
        failures.append(
            f"Express Amber {amber_rps:.0f} req/s < 2x Node {node_rps:.0f}"
        )

    for pattern, min_ratio in (
        ("EventEmitter", 1.2),
        ("URL & URLSearchParams", 1.2),
    ):
        amber_item, node_item, _unused = _lookup(p2, pattern)
        amber_ms = float(amber_item.get("avgMs") or 0)
        node_ms = float(node_item.get("avgMs") or 0)
        if amber_ms and node_ms and (node_ms / amber_ms) < min_ratio:
            failures.append(
                f"{pattern} Amber vs Node {node_ms / amber_ms:.2f}x < {min_ratio:.1f}x"
            )

    for framework, data in p3.items():
        if not isinstance(data, dict):
            continue
        amber = data.get("amber") or {}
        peak = float(amber.get("peak_rss_mb") or 0)
        settled = float(amber.get("settled_rss_mb") or 0)
        if peak and settled and settled > peak * 0.80:
            rss_notes.append(
                f"{framework}: settled RSS {settled:.1f} MB did not drop 20% from peak {peak:.1f} MB"
            )
    return failures, rss_notes


if __name__ == "__main__":
    main()

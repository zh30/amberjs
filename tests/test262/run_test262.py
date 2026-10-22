#!/usr/bin/env python3
"""Amber test262 harness — discover, run, scorecard.

Exit codes:
  0 — gate green (FAIL=0 and unexpected SKIP=0; for filtered runs, same).
       When --engine-allowlist / AMBER_TEST262_ENGINE_ALLOWLIST=1 is on,
       ENGINE_ALLOW does not count as FAIL.
  1 — gate red (failures or unexpected skips)
  2 — harness / environment error

Engine allowlist (optional, OFF by default):
  Catalog honest V8/Node FAILs without SKIP abuse. Default CI/smoke must not
  enable this — product gate stays FAIL-closed until explicitly flipped.
"""
from __future__ import annotations

import argparse
import concurrent.futures
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Optional

try:
    import yaml
except ImportError:
    print("PyYAML required: pip install pyyaml", file=sys.stderr)
    sys.exit(2)

ROOT = Path(__file__).resolve().parents[2]
HARNESS_DIR = Path(__file__).resolve().parent
DEFAULT_TEST262 = ROOT / "third_party" / "test262"
PIN_FILE = HARNESS_DIR / "PIN"
FEATURES_FILE = HARNESS_DIR / "features.json"
ENGINE_ALLOWLIST_FILE = HARNESS_DIR / "engine_fail_allowlist.txt"
HOST_JS = HARNESS_DIR / "amber_host.js"
SCORECARD_MD = HARNESS_DIR / "scorecard.md"
SCORECARD_JSON = HARNESS_DIR / "scorecard.json"

FRONTMATTER_RE = re.compile(r"/\*---\s*(.*?)\s*---\*/", re.DOTALL)

# Leading `;` defeats ASI when the test body ends in `})` / `.then(...)`
# without a trailing semicolon — otherwise V8 parses
# `asyncTest(...)(function wait...)` / `promiseThenResult(function wait...)`
# and false-fails (~48 built-ins cases).
ASYNC_EPILOGUE = """
;(function __test262_waitForAsyncDone() {
  if (__test262_async_done) {
    if (__test262_async_err) {
      var __e = __test262_async_err;
      __test262_async_err = null;
      throw __e;
    }
    return;
  }
  if (typeof setTimeout === 'function') {
    setTimeout(__test262_waitForAsyncDone, 0);
  } else {
    throw new Test262Error('Test262: async test did not call $DONE (no setTimeout)');
  }
})();
"""



def read_text_preserve_newlines(path: Path, *, errors: Optional[str] = None) -> str:
    """Read text without universal-newline translation.

    test262 includes Function.prototype.toString cases that use bare CR (\\r)
    as line terminators. Python's default text mode rewrites those to \\n and
    false-fails the suite; Node's harness does not.
    """
    kwargs: dict[str, Any] = {"encoding": "utf-8", "newline": ""}
    if errors is not None:
        kwargs["errors"] = errors
    with path.open(**kwargs) as fh:
        return fh.read()


def write_text_preserve_newlines(path: Path, text: str) -> None:
    """Write text without translating embedded \\r / \\n sequences."""
    with path.open("w", encoding="utf-8", newline="") as fh:
        fh.write(text)


@dataclass
class Meta:
    flags: set[str] = field(default_factory=set)
    includes: list[str] = field(default_factory=list)
    features: list[str] = field(default_factory=list)
    negative: Optional[dict[str, str]] = None
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class CaseResult:
    rel: str
    mode: str
    status: str  # PASS | FAIL | SKIP | ENGINE_ALLOW
    reason: str = ""
    ms: float = 0.0


def env_flag_true(name: str) -> bool:
    raw = (os.environ.get(name) or "").strip().lower()
    return raw in ("1", "true", "yes", "on")


def load_engine_allowlist(path: Path) -> list[str]:
    """Load exact paths and directory prefixes from engine_fail_allowlist.txt."""
    if not path.is_file():
        return []
    patterns: list[str] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        s = line.strip()
        if not s or s.startswith("#"):
            continue
        # Normalize to forward slashes; strip leading ./
        s = s.replace("\\", "/").lstrip("./")
        patterns.append(s)
    return patterns


def path_matches_allowlist(rel: str, patterns: list[str]) -> bool:
    """Match test path (relative to test/) against exact or prefix patterns."""
    rel = rel.replace("\\", "/")
    for pat in patterns:
        if pat.endswith("/"):
            if rel.startswith(pat) or rel.startswith(pat.rstrip("/") + "/"):
                return True
            # Also allow prefix without requiring trailing slash on rel dir itself
            if rel == pat.rstrip("/"):
                return True
        elif rel == pat:
            return True
    return False


def apply_engine_allowlist(
    results: list[CaseResult], patterns: list[str]
) -> list[CaseResult]:
    """Remap FAIL → ENGINE_ALLOW when path is allowlisted. PASS/SKIP unchanged."""
    if not patterns:
        return results
    out: list[CaseResult] = []
    for r in results:
        if r.status == "FAIL" and path_matches_allowlist(r.rel, patterns):
            reason = r.reason
            if not reason.startswith("engine-allowlist:"):
                reason = f"engine-allowlist:{reason}" if reason else "engine-allowlist"
            out.append(CaseResult(r.rel, r.mode, "ENGINE_ALLOW", reason, r.ms))
        else:
            out.append(r)
    return out


def parse_frontmatter(source: str) -> Meta:
    m = FRONTMATTER_RE.search(source)
    if not m:
        return Meta()
    data = yaml.safe_load(m.group(1)) or {}
    flags = set(data.get("flags") or [])
    includes = list(data.get("includes") or [])
    features = list(data.get("features") or [])
    negative = data.get("negative")
    if negative is not None and not isinstance(negative, dict):
        negative = None
    return Meta(flags=flags, includes=includes, features=features, negative=negative, raw=data)


def load_features_policy() -> tuple[set[str], set[str]]:
    if not FEATURES_FILE.is_file():
        return set(), set()
    data = json.loads(FEATURES_FILE.read_text(encoding="utf-8"))
    supported = set(data.get("supported") or [])
    always_skip = set(data.get("always_skip_features") or [])
    return supported, always_skip


def resolve_amber(explicit: Optional[str]) -> list[str]:
    if explicit:
        return [explicit]
    env = os.environ.get("AMBER_BIN")
    if env:
        return [env]
    for candidate in (ROOT / "target/release/amber", ROOT / "target/debug/amber"):
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return [str(candidate)]
    return ["cargo", "run", "--quiet", "--"]


def verify_pin(test262: Path) -> str:
    pin = PIN_FILE.read_text(encoding="utf-8").strip()
    if not (test262 / ".git").exists() and not (test262 / "harness").is_dir():
        print(f"test262 missing at {test262}; run ./scripts/fetch_test262.sh", file=sys.stderr)
        sys.exit(2)
    head = subprocess.check_output(
        ["git", "-C", str(test262), "rev-parse", "HEAD"], text=True
    ).strip()
    if head != pin:
        print(f"test262 HEAD {head} != PIN {pin}; re-run ./scripts/fetch_test262.sh", file=sys.stderr)
        sys.exit(2)
    return pin


def is_test262_fixture(path: Path) -> bool:
    """Fixture modules are imported by tests; they are not cases themselves."""
    name = path.name
    return (
        name.endswith("_FIXTURE.js")
        or name.endswith("_FIXTURE.mjs")
        or name.endswith("_FIXTURE.cjs")
        or name.endswith("_Fixture.js")
    )


def discover_tests(test262: Path, path_filter: Optional[str], limit: Optional[int]) -> list[Path]:
    base = test262 / "test"
    if path_filter:
        base = base / path_filter
    if not base.exists():
        print(f"path not found: {base}", file=sys.stderr)
        sys.exit(2)
    # Single-file --path: Path.rglob on a file yields nothing.
    if base.is_file():
        files = [base] if base.suffix == ".js" and not is_test262_fixture(base) else []
    else:
        files = sorted(
            p for p in base.rglob("*.js") if p.is_file() and not is_test262_fixture(p)
        )
    if limit is not None:
        files = files[:limit]
    return files


def modes_for(meta: Meta) -> list[str]:
    flags = meta.flags
    if "raw" in flags:
        return ["raw"]
    if "module" in flags:
        return ["module"]
    if "onlyStrict" in flags:
        return ["strict"]
    if "noStrict" in flags:
        return ["non-strict"]
    # SpiderMonkey staging shell `sm/non262-strict-shell.js` defines
    # `testLenientAndStrict`, which drives both lenient and strict via direct
    # `eval`. Prefacing the whole file with `"use strict"` (our strict mode
    # wrap) makes those direct evals inherit strictness, so the lenient arm
    # false-fails (`SameValue(false, true)`). Run once without the outer
    # strict wrapper — the shell still exercises both modes. Not a SKIP.
    if "sm/non262-strict-shell.js" in meta.includes:
        return ["non-strict"]
    return ["non-strict", "strict"]


EXPECTED_SKIP_PREFIX = "unsupported-feature:"


def skip_reason(meta: Meta, supported: set[str], always_skip: set[str]) -> Optional[str]:
    """Skip only features Amber explicitly cannot claim (`always_skip`).

    Features listed in `supported` are informational. Unknown features are
    *run* (honest PASS/FAIL) rather than silently skipped — avoids SKIP abuse
    and false skips for V8 builtins (TypedArray names, etc.).
    """
    _ = supported  # retained for scorecard / future allowlist reporting
    for feat in meta.features:
        if feat in always_skip:
            return f"{EXPECTED_SKIP_PREFIX}{feat}"
    return None


def is_expected_skip(reason: str, always_skip: set[str]) -> bool:
    """True iff SKIP reason is the intentional always_skip policy."""
    if not reason.startswith(EXPECTED_SKIP_PREFIX):
        return False
    feat = reason[len(EXPECTED_SKIP_PREFIX) :]
    return feat in always_skip


def count_unexpected_skips(results: list[CaseResult], always_skip: set[str]) -> int:
    """Count SKIP rows not explained by `always_skip_features`."""
    return sum(
        1
        for r in results
        if r.status == "SKIP" and not is_expected_skip(r.reason, always_skip)
    )


def gate_exit_code(fail_n: int, unexpected_skip: int) -> int:
    """Exit 1 when FAIL or unexpected SKIP; 0 when gate green."""
    return 0 if fail_n == 0 and unexpected_skip == 0 else 1


def read_includes(test262: Path, names: list[str]) -> str:
    parts: list[str] = []
    harness = test262 / "harness"
    # Always load assert.js + sta.js unless raw (caller handles raw).
    for name in ["assert.js", "sta.js", *names]:
        path = harness / name
        if not path.is_file():
            raise FileNotFoundError(f"missing harness include: {name}")
        parts.append(path.read_text(encoding="utf-8"))
    return "\n".join(parts)


def wrap_source(
    test262: Path,
    test_src: str,
    meta: Meta,
    mode: str,
    host_js: str,
) -> tuple[str, str]:
    """Return (source, file_suffix) for amber run."""
    # Strip frontmatter so YAML does not confuse parsers.
    body = FRONTMATTER_RE.sub("", test_src, count=1)

    # Negative parse/early errors: only the test body is parsed (no harness).
    # See test262 INTERPRETING.md — $DONOTEVALUATE must never run.
    if meta.negative and (meta.negative.get("phase") or "").lower() in (
        "parse",
        "early",
    ):
        if mode == "strict":
            body = '"use strict";\n' + body
        elif mode == "module":
            return body, ".mjs"
        return body, ".js"

    if mode == "raw":
        return body, ".js"

    includes = read_includes(test262, meta.includes)

    async_prologue = ""
    async_epilogue = ""
    if "async" in meta.flags:
        # test262 async: test calls $DONE() / $DONE(error). Keep Amber's
        # event loop alive via setTimeout until $DONE (sync check at EOF
        # races microtasks / async generators and false-fails).
        async_prologue = """
var __test262_async_done = false;
var __test262_async_err = null;
function $DONE(err) {
  __test262_async_done = true;
  if (arguments.length > 0 && err != null) {
    __test262_async_err = err;
  }
}
"""
        async_epilogue = ASYNC_EPILOGUE

    # "use strict" must be the first statement in the whole script or it is
    # ignored (Directive Prologue). Prefacing harness includes would break
    # strict-only tests (PutValue / unresolvable refs, etc.).
    parts: list[str] = []
    if mode == "strict":
        parts.append('"use strict";')
    parts.extend([host_js, includes, async_prologue, body, async_epilogue])
    # dynamic-import syntax/valid tests end with `import(...)` ExpressionStatement.
    # Amber treats a rejected completion promise as process failure; append void 0
    # so completion is undefined while still exercising the ImportCall (no SKIP).
    parts.append("void 0;")
    combined = "\n".join(parts)
    # Module cases use package.json {"type":"module"} + original .js basename
    # (see prepare_module_run_path) so relative/self-imports resolve.
    if mode == "module":
        return combined, ".js"
    return combined, ".js"


def _negative_type_matched(ntype: str, err_text: str) -> bool:
    """Match test262 negative `type` against Amber stderr (and legacy codes)."""
    if not ntype:
        return False
    if ntype in err_text:
        return True
    # Legacy Amber codes used SCREAMING_SNAKE (REFERENCE_ERROR). Current runtime
    # emits ECMAScript names; keep the alias so older binaries still score.
    legacy = {
        "ReferenceError": "REFERENCE_ERROR",
        "TypeError": "TYPE_ERROR",
        "RangeError": "RANGE_ERROR",
        "SyntaxError": "SYNTAX_ERROR",
        "EvalError": "EVAL_ERROR",
        "URIError": "URI_ERROR",
    }.get(ntype)
    return bool(legacy and legacy in err_text)


def classify_result(
    meta: Meta,
    returncode: int,
    stderr: str,
    stdout: str,
) -> tuple[str, str]:
    err_text = (stderr or "") + "\n" + (stdout or "")
    if meta.negative:
        phase = (meta.negative.get("phase") or "").lower()
        ntype = meta.negative.get("type") or ""
        # Negative tests expect a throw / early error.
        if returncode != 0:
            if _negative_type_matched(ntype, err_text):
                return "PASS", f"negative:{phase}:{ntype}"
            # Parser diagnostics may omit the exact `SyntaxError` token.
            # Amber tags syntax failures as SYNTAX_ERROR (not always "SyntaxError").
            if phase in ("parse", "early") and (not ntype or ntype == "SyntaxError"):
                markers = (
                    "SyntaxError",
                    "SYNTAX_ERROR",
                    "Unexpected token",
                    "Unexpected identifier",
                    "Parse error",
                    "parse error",
                    "Duplicate export",
                    "has already been declared",
                )
                if any(m in err_text for m in markers):
                    return "PASS", f"negative:{phase}:SyntaxError~"
            if ntype == "SyntaxError" and "SYNTAX_ERROR" in err_text:
                return "PASS", f"negative:{phase}:SYNTAX_ERROR"
            if ntype:
                return "FAIL", f"negative expected {ntype}, got: {err_text[:200]!r}"
            return "PASS", f"negative:{phase}"
        return "FAIL", "negative test completed without error"
    # Positive
    if returncode == 0:
        if "Test262Error" in err_text and "Test262Error:" in err_text:
            return "FAIL", err_text[:300]
        return "PASS", ""
    return "FAIL", err_text[:400].strip() or f"exit {returncode}"


def _strip_test262_frontmatter(source: str) -> str:
    return FRONTMATTER_RE.sub("", source, count=1)


def prepare_module_run_path(
    tmp_dir: Path,
    test_path: Path,
    source: str,
    attempt: int,
    test262: Optional[Path] = None,
) -> Path:
    """Place wrapped ESM source so relative/self-imports match test262 layout.

    Amber resolves ESM from the entry file path. Running from a random tempfile
    breaks `import * as ns from './this-file.js'` and sibling/fixture imports.
    Use original basename + {"type":"module"} + sibling copies in a per-attempt dir.

    Non-fixture sibling `.js` tests that are imported as modules often call
    `assert.*` without a harness. Prepend assert.js+sta.js (module-local) so
    those deps evaluate; fixtures are copied raw.
    """
    attempt_dir = tmp_dir / f"a{attempt}"
    if attempt_dir.exists():
        shutil.rmtree(attempt_dir)
    attempt_dir.mkdir(parents=True, exist_ok=True)
    (attempt_dir / "package.json").write_text('{"type":"module"}\n', encoding="utf-8")
    harness_preamble = ""
    if test262 is not None:
        try:
            harness_preamble = read_includes(test262, [])
        except FileNotFoundError:
            harness_preamble = ""
    # Copy siblings (fixtures and other modules) into the attempt dir.
    # Symlinks are unsafe: Amber realpath() would escape this package.json
    # {"type":"module"} boundary and load siblings as CommonJS.
    for sibling in test_path.parent.iterdir():
        if not sibling.is_file():
            continue
        if sibling.suffix not in {".js", ".mjs", ".cjs", ".json"}:
            continue
        if sibling.name == test_path.name:
            continue
        dest = attempt_dir / sibling.name
        if (
            harness_preamble
            and sibling.suffix == ".js"
            and not is_test262_fixture(sibling)
        ):
            body = _strip_test262_frontmatter(
                sibling.read_text(encoding="utf-8", errors="replace")
            )
            dest.write_text(harness_preamble + "\n" + body, encoding="utf-8")
        else:
            shutil.copy2(sibling, dest)
    out = attempt_dir / test_path.name
    write_text_preserve_newlines(out, source)
    return out


def copy_fixtures_into(dest_dir: Path, test_path: Path) -> None:
    """Copy same-dir fixtures so script-mode dynamic import() can load them."""
    dest_dir.mkdir(parents=True, exist_ok=True)
    for sibling in test_path.parent.iterdir():
        if sibling.is_file() and is_test262_fixture(sibling):
            target = dest_dir / sibling.name
            if not target.exists():
                shutil.copy2(sibling, target)


def run_one(
    amber: list[str],
    test262: Path,
    test_path: Path,
    mode: str,
    meta: Meta,
    host_js: str,
    timeout: float,
    work_dir: Path,
) -> CaseResult:
    rel = str(test_path.relative_to(test262 / "test"))
    t0 = time.perf_counter()
    try:
        source, suffix = wrap_source(
            test262, read_text_preserve_newlines(test_path), meta, mode, host_js
        )
    except Exception as exc:  # noqa: BLE001
        return CaseResult(rel, mode, "FAIL", f"wrap:{exc}", 0.0)

    # Unique path per attempt/thread — avoids parallel clobber + helps crash retries.
    tmp_dir = work_dir / f"{rel.replace('/', '__')}__{mode}"
    tmp_dir.mkdir(parents=True, exist_ok=True)

    cmd_base = [*amber, "run"]
    env = os.environ.copy()
    # Enable native __amber_createRealm for cross-realm tests.
    env.setdefault("AMBER_TEST262", "1")
    # test262 INTERPRETING.md: CanBlockIsFalse / CanBlockIsTrue select Agent [[CanBlock]].
    # Amber maps this to V8 Isolate::SetAllowAtomicsWait (AgentCanSuspend).
    if "CanBlockIsFalse" in meta.flags:
        env["AMBER_TEST262_CAN_BLOCK"] = "0"
    elif "CanBlockIsTrue" in meta.flags:
        env["AMBER_TEST262_CAN_BLOCK"] = "1"
    else:
        env.pop("AMBER_TEST262_CAN_BLOCK", None)

    status, reason = "FAIL", "no attempt"
    # CI saw exit -7 (SIGBUS) on an otherwise-green types test under --jobs 4.
    # Retry signal deaths only — never retry ordinary assertion failures.
    max_attempts = 3
    for attempt in range(max_attempts):
        if mode == "module":
            tmp = prepare_module_run_path(
                tmp_dir, test_path, source, attempt, test262=test262
            )
        else:
            # Fixtures may be ESM (export / top-level await) loaded via import().
            # Use package.json type=module for those, but run the harness wrap as
            # entry.cjs so script semantics (var/function dual) stay CommonJS.
            attempt_dir = tmp_dir / f"a{attempt}"
            attempt_dir.mkdir(parents=True, exist_ok=True)
            (attempt_dir / "package.json").write_text(
                '{"type":"module"}\n', encoding="utf-8"
            )
            copy_fixtures_into(attempt_dir, test_path)
            # entry.cjs: keep script dual-binding under type=module package.json (#272).
            # newline="" write: bare CR must survive for Function.prototype.toString LT (#284).
            tmp = attempt_dir / "entry.cjs"
            write_text_preserve_newlines(tmp, source)
        try:
            proc = subprocess.run(
                [*cmd_base, str(tmp)],
                capture_output=True,
                text=True,
                timeout=timeout,
                cwd=str(ROOT),
                env=env,
            )
            # Negative returncode => killed by signal (e.g. -7 SIGBUS).
            if proc.returncode < 0 and attempt + 1 < max_attempts:
                continue
            status, reason = classify_result(meta, proc.returncode, proc.stderr, proc.stdout)
            if status == "FAIL" and proc.returncode < 0 and attempt + 1 < max_attempts:
                continue
            break
        except subprocess.TimeoutExpired:
            status, reason = "FAIL", f"timeout>{timeout}s"
            break
        except Exception as exc:  # noqa: BLE001
            status, reason = "FAIL", f"exec:{exc}"
            break

    ms = (time.perf_counter() - t0) * 1000
    return CaseResult(rel, mode, status, reason, ms)


def write_scorecard(
    pin: str,
    amber: list[str],
    path_filter: Optional[str],
    results: list[CaseResult],
    elapsed_s: float,
    always_skip: set[str],
    *,
    engine_allowlist_enabled: bool = False,
    engine_allowlist_patterns: Optional[list[str]] = None,
) -> None:
    pass_n = sum(1 for r in results if r.status == "PASS")
    fail_n = sum(1 for r in results if r.status == "FAIL")
    skip_n = sum(1 for r in results if r.status == "SKIP")
    engine_allow_n = sum(1 for r in results if r.status == "ENGINE_ALLOW")
    total = len(results)
    unexpected_skip = count_unexpected_skips(results, always_skip)
    gate_ok = fail_n == 0 and unexpected_skip == 0 and (
        path_filter is not None or total > 0
    )
    # Full-suite gate: FAIL=0 and unexpected SKIP=0 (ENGINE_ALLOW excluded from FAIL)
    if path_filter:
        gate_note = f"filtered path={path_filter} (not full-suite gate)"
    else:
        gate_note = "full suite"
    if engine_allowlist_enabled:
        gate_note += f"; engine-allowlist ON (ENGINE_ALLOW={engine_allow_n})"
    else:
        gate_note += "; engine-allowlist OFF"
    gate_line = (
        f"**Gate: PASS** (FAIL=0, unexpected SKIP=0; {gate_note})"
        if gate_ok
        else (
            f"**Gate: FAIL** (PASS={pass_n} FAIL={fail_n} "
            f"ENGINE_ALLOW={engine_allow_n} SKIP={skip_n} "
            f"unexpected_SKIP={unexpected_skip}; {gate_note})"
        )
    )

    samples = [r for r in results if r.status == "FAIL"][:25]
    allow_samples = [r for r in results if r.status == "ENGINE_ALLOW"][:25]
    skip_samples = [r for r in results if r.status == "SKIP"][:15]

    lines = [
        "# Amber test262 scorecard",
        "",
        f"- Generated: {time.strftime('%Y-%m-%dT%H:%MZ', time.gmtime())}",
        f"- test262 pin: `{pin}`",
        f"- Amber: `{' '.join(amber)}`",
        f"- Filter: `{path_filter or '(full suite)'}`",
        f"- Elapsed: {elapsed_s:.1f}s",
        f"- Engine allowlist: `{'ON' if engine_allowlist_enabled else 'OFF'}`"
        + (
            f" ({len(engine_allowlist_patterns or [])} patterns)"
            if engine_allowlist_enabled
            else ""
        ),
        "",
        "| Status | Count |",
        "|--------|------:|",
        f"| PASS | {pass_n} |",
        f"| FAIL | {fail_n} |",
        f"| ENGINE_ALLOW | {engine_allow_n} |",
        f"| SKIP | {skip_n} |",
        f"| TOTAL | {total} |",
        "",
        gate_line,
        "",
        "## Sample failures",
        "",
        "| Test | Mode | Reason |",
        "|------|------|--------|",
    ]
    if not samples:
        lines.append("| — | — | none |")
    else:
        for r in samples:
            reason = r.reason.replace("|", "\\|").replace("\n", " ")[:120]
            lines.append(f"| `{r.rel}` | {r.mode} | {reason} |")

    lines += [
        "",
        "## Sample engine-allow (honest V8/Node catalog)",
        "",
        "| Test | Mode | Reason |",
        "|------|------|--------|",
    ]
    if not allow_samples:
        lines.append("| — | — | none |")
    else:
        for r in allow_samples:
            reason = r.reason.replace("|", "\\|").replace("\n", " ")[:120]
            lines.append(f"| `{r.rel}` | {r.mode} | {reason} |")

    lines += [
        "",
        "## Sample skips",
        "",
        "| Test | Mode | Reason |",
        "|------|------|--------|",
    ]
    if not skip_samples:
        lines.append("| — | — | none |")
    else:
        for r in skip_samples:
            lines.append(f"| `{r.rel}` | {r.mode} | {r.reason} |")

    SCORECARD_MD.write_text("\n".join(lines) + "\n", encoding="utf-8")

    payload = {
        "generated": time.strftime("%Y-%m-%dT%H:%MZ", time.gmtime()),
        "pin": pin,
        "amber": amber,
        "path_filter": path_filter,
        "elapsed_s": elapsed_s,
        "pass": pass_n,
        "fail": fail_n,
        "engine_allow": engine_allow_n,
        "skip": skip_n,
        "total": total,
        "unexpected_skip": unexpected_skip,
        "engine_allowlist_enabled": engine_allowlist_enabled,
        "engine_allowlist_patterns": len(engine_allowlist_patterns or []),
        "gate_ok": fail_n == 0 and unexpected_skip == 0,
        "failures": [
            {"test": r.rel, "mode": r.mode, "reason": r.reason[:500]}
            for r in results
            if r.status == "FAIL"
        ][:200],
        "engine_allows": [
            {"test": r.rel, "mode": r.mode, "reason": r.reason[:500]}
            for r in results
            if r.status == "ENGINE_ALLOW"
        ][:200],
        "skips": [
            {"test": r.rel, "mode": r.mode, "reason": r.reason}
            for r in results
            if r.status == "SKIP"
        ][:200],
    }
    SCORECARD_JSON.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {SCORECARD_MD}")
    print(f"Wrote {SCORECARD_JSON}")
    print(
        f"Summary: PASS={pass_n} FAIL={fail_n} ENGINE_ALLOW={engine_allow_n} "
        f"SKIP={skip_n} unexpected_SKIP={unexpected_skip} TOTAL={total}"
    )


def self_test() -> None:
    m = parse_frontmatter("/*---\nflags: [onlyStrict]\nfeatures: [BigInt]\n---*/\nok")
    assert "onlyStrict" in m.flags
    assert m.features == ["BigInt"]
    assert modes_for(m) == ["strict"]
    m2 = parse_frontmatter("/*---\nflags: [module]\n---*/")
    assert modes_for(m2) == ["module"]
    m_sm_strict = parse_frontmatter(
        "/*---\nincludes: [sm/non262-strict-shell.js]\n---*/\nok"
    )
    assert modes_for(m_sm_strict) == ["non-strict"]
    m_both = parse_frontmatter("/*---\nincludes: [assert.js]\n---*/\nok")
    assert modes_for(m_both) == ["non-strict", "strict"]
    m3 = parse_frontmatter("/*---\nnegative:\n  phase: parse\n  type: SyntaxError\n---*/")
    assert m3.negative and m3.negative["type"] == "SyntaxError"
    st, _ = classify_result(m3, 1, "SyntaxError: Unexpected token", "")
    assert st == "PASS"
    st2, _ = classify_result(Meta(), 0, "", "")
    assert st2 == "PASS"
    m4 = parse_frontmatter(
        "/*---\nnegative:\n  phase: runtime\n  type: ReferenceError\n---*/"
    )
    st3, _ = classify_result(
        m4, 1, "Error: [Amber Error] ReferenceError: x is not defined\n", ""
    )
    assert st3 == "PASS"
    st4, _ = classify_result(
        m4, 1, "Error: [Amber Error] REFERENCE_ERROR: x is not defined\n", ""
    )
    assert st4 == "PASS"
    # Runtime must surface constructor name for harness/sta.js Test262Error
    # (no .name property) — empty UNKNOWN_ERROR must not pass.
    m5 = parse_frontmatter(
        "/*---\nnegative:\n  phase: runtime\n  type: Test262Error\nflags: [module]\n---*/"
    )
    st5, reason5 = classify_result(
        m5, 1, "Error: [Amber Error] Test262Error: \n", ""
    )
    assert st5 == "PASS", reason5
    st6, _ = classify_result(m5, 1, "Error: [Amber Error] UNKNOWN_ERROR: \n\n", "")
    assert st6 == "FAIL"

    # Engine allowlist matching (prefix + exact); remapping FAIL → ENGINE_ALLOW
    pats = [
        "language/eval-code/direct/",
        "language/statements/with/unscopables-inc-dec.js",
    ]
    assert path_matches_allowlist(
        "language/eval-code/direct/var-env-lower-lex-non-strict.js", pats
    )
    assert path_matches_allowlist(
        "language/statements/with/unscopables-inc-dec.js", pats
    )
    assert not path_matches_allowlist("language/eval-code/indirect/x.js", pats)
    assert not path_matches_allowlist("language/statements/with/other.js", pats)
    remapped = apply_engine_allowlist(
        [
            CaseResult("language/eval-code/direct/a.js", "non-strict", "FAIL", "boom"),
            CaseResult(
                "language/types/boolean/S8.3_A1_T1.js", "non-strict", "FAIL", "real"
            ),
            CaseResult("language/eval-code/direct/b.js", "strict", "PASS", ""),
            CaseResult(
                "language/statements/with/unscopables-inc-dec.js", "strict", "FAIL", "gap"
            ),
        ],
        pats,
    )
    assert remapped[0].status == "ENGINE_ALLOW"
    assert remapped[0].reason.startswith("engine-allowlist:")
    assert remapped[1].status == "FAIL"
    assert remapped[2].status == "PASS"
    assert remapped[3].status == "ENGINE_ALLOW"
    untouched = apply_engine_allowlist(
        [CaseResult("language/eval-code/direct/a.js", "non-strict", "FAIL", "boom")],
        [],
    )
    assert untouched[0].status == "FAIL"
    seed = load_engine_allowlist(ENGINE_ALLOWLIST_FILE)
    assert any(p.startswith("language/eval-code/direct") for p in seed)
    assert any("compound-assignment" in p for p in seed)
    assert any(p.startswith("language/expressions/assignment") for p in seed)
    assert any(p.startswith("built-ins/Error/prototype/stack") for p in seed)
    assert any(p.startswith("intl402") for p in seed)
    assert any(p.startswith("annexB/built-ins/RegExp") for p in seed)
    assert any(p.startswith("staging/sm") for p in seed)
    assert not any("allKeyed" in p for p in seed), "keyed polyfill #287 — not in primary seed"
    assert not any("language/comments" in p for p in seed)
    assert not any(p.startswith("annexB/language/comments") for p in seed)
    always_skip = {"Temporal", "decorators"}
    supported: set[str] = set()
    # Policy skip: feature in always_skip → expected SKIP reason.
    m_skip = parse_frontmatter("/*---\nfeatures: [Temporal]\n---*/")
    reason = skip_reason(m_skip, supported, always_skip)
    assert reason == "unsupported-feature:Temporal"
    assert is_expected_skip(reason, always_skip)
    # Unknown / supported-only features are not skipped (honest PASS/FAIL).
    m_run = parse_frontmatter("/*---\nfeatures: [BigInt]\n---*/")
    assert skip_reason(m_run, {"BigInt"}, always_skip) is None
    assert skip_reason(m_run, set(), always_skip) is None

    expected = CaseResult("a.js", "strict", "SKIP", "unsupported-feature:Temporal")
    unexpected = CaseResult("b.js", "strict", "SKIP", "manual-skip-abuse")
    orphan = CaseResult("c.js", "strict", "SKIP", "unsupported-feature:not-in-policy")
    passed = CaseResult("d.js", "strict", "PASS", "")
    failed = CaseResult("e.js", "strict", "FAIL", "boom")
    assert count_unexpected_skips([expected, passed, failed], always_skip) == 0
    assert count_unexpected_skips([expected, unexpected, orphan], always_skip) == 2
    assert gate_exit_code(0, 0) == 0
    assert gate_exit_code(1, 0) == 1
    assert gate_exit_code(0, 1) == 1
    assert gate_exit_code(2, 3) == 1
    # Bare CR must survive round-trip (Function.prototype.toString LT tests).
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "cr.js"
        write_text_preserve_newlines(p, "function\rf\r(\rx\r)\r{\r;\r}")
        got = read_text_preserve_newlines(p)
        assert "\r" in got and "\n" not in got, repr(got)
        uni = p.read_text(encoding="utf-8")
        assert "\n" in uni and "\r" not in uni
    # ASI guard: async epilogue must start with `;` so bodies ending in `})`
    # / `.then(...)` do not glue onto the wait IIFE.
    assert ASYNC_EPILOGUE.lstrip().startswith(";(function __test262_waitForAsyncDone")
    m_block = parse_frontmatter("/*---\nflags: [CanBlockIsFalse]\n---*/\nok")
    assert "CanBlockIsFalse" in m_block.flags
    print("test262 harness self-test ok")


def main() -> None:
    ap = argparse.ArgumentParser(description="Run test262 against Amber")
    ap.add_argument("--path", help="subdir under test/ (e.g. language/types)")
    ap.add_argument("--limit", type=int, help="max test files")
    ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    ap.add_argument("--timeout", type=float, default=float(os.environ.get("TEST262_TIMEOUT", "30")))
    ap.add_argument("--amber", help="path to amber binary")
    ap.add_argument("--test262", type=Path, default=None)
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--keep-tmp", action="store_true")
    ap.add_argument(
        "--engine-allowlist",
        action="store_true",
        default=None,
        help=(
            "Remap allowlisted FAILs to ENGINE_ALLOW (not counted as FAIL). "
            "Also enabled by AMBER_TEST262_ENGINE_ALLOWLIST=1. Default OFF."
        ),
    )
    ap.add_argument(
        "--no-engine-allowlist",
        action="store_true",
        help="Force engine allowlist OFF even if AMBER_TEST262_ENGINE_ALLOWLIST=1",
    )
    ap.add_argument(
        "--engine-allowlist-file",
        type=Path,
        default=None,
        help=f"Allowlist path (default: {ENGINE_ALLOWLIST_FILE.name})",
    )
    args = ap.parse_args()

    if args.self_test:
        self_test()
        return

    if args.no_engine_allowlist:
        engine_allowlist_enabled = False
    elif args.engine_allowlist:
        engine_allowlist_enabled = True
    else:
        engine_allowlist_enabled = env_flag_true("AMBER_TEST262_ENGINE_ALLOWLIST")

    allowlist_path = args.engine_allowlist_file or ENGINE_ALLOWLIST_FILE
    allowlist_patterns: list[str] = []
    if engine_allowlist_enabled:
        allowlist_patterns = load_engine_allowlist(allowlist_path)
        if not allowlist_patterns:
            print(
                f"engine allowlist enabled but no patterns in {allowlist_path}",
                file=sys.stderr,
            )
            sys.exit(2)

    test262 = args.test262 or Path(os.environ.get("TEST262_ROOT", str(DEFAULT_TEST262)))
    pin = verify_pin(test262)
    amber = resolve_amber(args.amber)
    host_js = HOST_JS.read_text(encoding="utf-8")
    supported, always_skip = load_features_policy()

    tests = discover_tests(test262, args.path, args.limit)
    if not tests:
        print("no tests discovered", file=sys.stderr)
        sys.exit(2)

    print(f"Amber test262")
    print(f"  pin: {pin}")
    print(f"  amber: {' '.join(amber)}")
    print(f"  files: {len(tests)}")
    print(f"  jobs: {args.jobs}")
    print(
        f"  engine-allowlist: "
        f"{'ON (' + str(len(allowlist_patterns)) + ' patterns)' if engine_allowlist_enabled else 'OFF'}"
    )
    print()

    work = Path(tempfile.mkdtemp(prefix="amber-test262-"))
    results: list[CaseResult] = []
    t0 = time.perf_counter()

    jobs: list[tuple[Path, str, Meta]] = []
    for tp in tests:
        meta = parse_frontmatter(tp.read_text(encoding="utf-8", errors="replace"))
        reason = skip_reason(meta, supported, always_skip)
        if reason:
            rel = str(tp.relative_to(test262 / "test"))
            for mode in modes_for(meta):
                results.append(CaseResult(rel, mode, "SKIP", reason, 0.0))
            continue
        for mode in modes_for(meta):
            jobs.append((tp, mode, meta))

    def _run(item: tuple[Path, str, Meta]) -> CaseResult:
        tp, mode, meta = item
        return run_one(amber, test262, tp, mode, meta, host_js, args.timeout, work)

    if args.jobs <= 1:
        for item in jobs:
            r = _run(item)
            results.append(r)
            print(
                f"{r.status:4} {r.rel} [{r.mode}]"
                + (f" — {r.reason[:80]}" if r.status != "PASS" else "")
            )
    else:
        done = 0
        with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as ex:
            futs = {ex.submit(_run, item): item for item in jobs}
            for fut in concurrent.futures.as_completed(futs):
                r = fut.result()
                results.append(r)
                done += 1
                if done % 50 == 0 or r.status == "FAIL":
                    print(f"{r.status:4} {r.rel} [{r.mode}] ({done}/{len(jobs)})")

    elapsed = time.perf_counter() - t0
    results.sort(key=lambda r: (r.rel, r.mode))
    if engine_allowlist_enabled:
        results = apply_engine_allowlist(results, allowlist_patterns)
    write_scorecard(
        pin,
        amber,
        args.path,
        results,
        elapsed,
        always_skip,
        engine_allowlist_enabled=engine_allowlist_enabled,
        engine_allowlist_patterns=allowlist_patterns if engine_allowlist_enabled else None,
    )

    if not args.keep_tmp:
        shutil.rmtree(work, ignore_errors=True)

    fail_n = sum(1 for r in results if r.status == "FAIL")
    unexpected_skip = count_unexpected_skips(results, always_skip)
    if unexpected_skip:
        print(f"unexpected SKIP={unexpected_skip} (not explained by always_skip_features)")
    sys.exit(gate_exit_code(fail_n, unexpected_skip))


if __name__ == "__main__":
    main()

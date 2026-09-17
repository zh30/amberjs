#!/usr/bin/env python3
"""Rewrite Formula/amber.rb version and sha256 values from release archives."""

from __future__ import annotations

import argparse
import hashlib
import os
import re
import sys
from pathlib import Path


ASSETS = (
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-linux-gnu",
)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    value = digest.hexdigest()
    if set(value) <= {"0"}:
        raise SystemExit(f"refusing all-zero sha256 for {path}")
    return value


def find_archive(release_dir: Path, version: str, target: str) -> Path:
    name = f"amber-v{version}-{target}.tar.gz"
    matches = list(release_dir.rglob(name))
    if not matches:
        raise SystemExit(f"missing archive {name} under {release_dir}")
    return matches[0]


def update_formula(formula: Path, version: str, hashes: dict[str, str]) -> str:
    text = formula.read_text()
    text = re.sub(r'version "[^"]+"', f'version "{version}"', text, count=1)
    for target, digest in hashes.items():
        if set(digest) <= {"0"}:
            raise SystemExit(f"refusing all-zero sha256 for {target}")
        pattern = rf'(amber-v#\{{version\}}-{re.escape(target)}\.tar\.gz"\n\s+sha256 )"[^"]+"'
        replacement = rf'\1"{digest}"'
        updated, count = re.subn(pattern, replacement, text)
        if count != 1:
            raise SystemExit(f"failed to patch sha256 for {target} (matches={count})")
        text = updated
    return text


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--formula", required=True)
    parser.add_argument("--release-dir", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()

    version = args.version.lstrip("v")
    release_dir = Path(args.release_dir)
    formula = Path(args.formula)
    hashes = {}
    for target in ASSETS:
        archive = find_archive(release_dir, version, target)
        hashes[target] = sha256_file(archive)

    text = update_formula(formula, version, hashes)
    if args.write:
        formula.write_text(text)
    else:
        sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

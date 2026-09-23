#!/usr/bin/env python3
"""
scripts/generate_release_notes.py

Generates a formal, comprehensive Release Document for Amber releases in GitHub Actions.

Features:
1. Priority 1: Reads formal release document from `docs/releases/<tag>.md`.
2. Priority 2: Extracts corresponding version section from `CHANGELOG.md`.
3. Priority 3: Dynamically parses `git log` and `git diff --stat` between previous tag and current tag,
   categorizing commits by type (feat, fix, docs, test, perf, chore).
4. Computes SHA256 checksums for all binary tarballs in `--release-dir`, generates `checksums.txt`,
   and embeds a formatted Markdown verification table.
5. Emits the final combined document to `--output` (default: `RELEASE_NOTES.md`).
"""

import argparse
import hashlib
import os
import re
import subprocess
import sys
from pathlib import Path


def run_cmd(cmd, cwd=None):
    try:
        res = subprocess.run(
            cmd,
            cwd=cwd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=True,
        )
        return res.stdout.strip()
    except subprocess.CalledProcessError as e:
        return ""


def get_git_tag():
    tag = run_cmd(["git", "describe", "--tags", "--exact-match", "HEAD"])
    if tag:
        return tag
    tag = run_cmd(["git", "describe", "--tags", "--abbrev=0"])
    return tag if tag else "HEAD"


def get_previous_tag(tag):
    prev = run_cmd(["git", "describe", "--tags", "--abbrev=0", f"{tag}^"])
    return prev if prev else None


def format_bytes(size_bytes):
    if size_bytes < 1024:
        return f"{size_bytes} B"
    elif size_bytes < 1024 * 1024:
        return f"{size_bytes / 1024:.1f} KB"
    else:
        return f"{size_bytes / (1024 * 1024):.1f} MB"


def detect_target_platform(filename):
    # Triple matches must come before generic substrings like "linux".
    if "aarch64-unknown-linux-gnu" in filename:
        return "Linux (aarch64)"
    if "x86_64-unknown-linux-gnu" in filename:
        return "Linux (x86_64)"
    if "aarch64-apple-darwin" in filename or "darwin-arm64" in filename:
        return "macOS Apple Silicon (ARM64)"
    if "x86_64-apple-darwin" in filename or "darwin-x64" in filename:
        return "macOS Intel (x86_64)"
    if "x86_64-pc-windows-msvc" in filename or "windows" in filename:
        return "Windows (x86_64)"
    if "linux" in filename:
        return "Linux"
    return "Unknown Platform"


def compute_sha256(filepath):
    h = hashlib.sha256()
    with open(filepath, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


def find_release_doc(tag, repo_root):
    clean_version = tag.lstrip("v")
    candidates = [
        repo_root / "docs" / "releases" / f"{tag}.md",
        repo_root / "docs" / "releases" / f"v{clean_version}.md",
        repo_root / "docs" / "releases" / f"{clean_version}.md",
        repo_root / "docs" / f"RELEASE_NOTES_{tag}.md",
        repo_root / "docs" / f"RELEASE_NOTES_v{clean_version}.md",
        repo_root / "docs" / f"RELEASE_NOTES_{clean_version}.md",
    ]
    for c in candidates:
        if c.exists():
            return c
    return None


def extract_changelog_section(tag, repo_root):
    changelog_path = repo_root / "CHANGELOG.md"
    if not changelog_path.exists():
        return ""

    content = changelog_path.read_text(encoding="utf-8")
    clean_version = tag.lstrip("v")
    pattern = rf"##\s*\[{re.escape(clean_version)}\](?:[^\n]*)\n(.*?)(?=\n##\s*\[|\Z)"
    match = re.search(pattern, content, re.DOTALL)
    if match:
        section = match.group(1).strip()
        return f"# Amber {tag} Release Notes\n\n{section}"
    return ""


def generate_git_changelog(tag, prev_tag, repo_root):
    revision_range = f"{prev_tag}..{tag}" if prev_tag else tag
    log_output = run_cmd(["git", "log", revision_range, "--pretty=format:%h|||%s|||%an"])

    features = []
    fixes = []
    docs = []
    tests = []
    others = []

    for line in log_output.splitlines():
        if not line.strip():
            continue
        parts = line.split("|||")
        if len(parts) < 3:
            continue
        commit_hash, subject, author = parts[0], parts[1], parts[2]
        entry = f"- `{commit_hash}` {subject} (@{author})"

        lower = subject.lower()
        if lower.startswith("feat"):
            features.append(entry)
        elif lower.startswith("fix"):
            fixes.append(entry)
        elif lower.startswith("docs"):
            docs.append(entry)
        elif lower.startswith("test"):
            tests.append(entry)
        else:
            others.append(entry)

    diff_stat = run_cmd(["git", "diff", "--stat", revision_range])

    sections = [f"# Amber {tag} Release Notes\n"]
    if features:
        sections.append("### 🚀 Features\n" + "\n".join(features) + "\n")
    if fixes:
        sections.append("### 🐛 Bug Fixes\n" + "\n".join(fixes) + "\n")
    if docs:
        sections.append("### 📚 Documentation\n" + "\n".join(docs) + "\n")
    if tests:
        sections.append("### 🧪 Testing\n" + "\n".join(tests) + "\n")
    if others:
        sections.append("### 🔧 Other Changes\n" + "\n".join(others) + "\n")

    if diff_stat:
        sections.append("### 📊 File Changes Summary\n```text\n" + diff_stat + "\n```\n")

    return "\n".join(sections)


def process_release_assets(release_dir):
    if not release_dir or not release_dir.exists():
        return [], ""

    asset_files = sorted(
        [
            p
            for p in release_dir.glob("**/*")
            if p.is_file()
            and (p.name.endswith(".tar.gz") or p.name.endswith(".zip"))
            and not p.name.endswith(".sha256")
        ]
    )

    if not asset_files:
        return [], ""

    checksum_lines = []
    table_rows = []

    for path in asset_files:
        sha256 = compute_sha256(path)
        size_str = format_bytes(path.stat().st_size)
        platform = detect_target_platform(path.name)
        checksum_lines.append(f"{sha256}  {path.name}")
        table_rows.append(f"| `{path.name}` | {platform} | {size_str} | `{sha256}` |")

    # Write checksums.txt into release_dir
    checksum_file = release_dir / "checksums.txt"
    checksum_file.write_text("\n".join(checksum_lines) + "\n", encoding="utf-8")

    markdown = [
        "\n---\n",
        "## 📦 预编译二进制资产与 SHA-256 校验和\n",
        "| 资产文件名 | 目标系统 / 架构 | 文件大小 | SHA-256 校验和 |",
        "|---|---|---|---|",
    ]
    markdown.extend(table_rows)
    markdown.append("\n### 校验方法\n")
    markdown.append("```bash")
    markdown.append("# 下载对应架构压缩包与 checksums.txt 后执行：")
    markdown.append("shasum -a 256 -c checksums.txt")
    markdown.append("# 或 Linux 环境下：")
    markdown.append("sha256sum -c checksums.txt")
    markdown.append("```\n")

    return asset_files, "\n".join(markdown)


def main():
    parser = argparse.ArgumentParser(description="Generate formal Amber release notes.")
    parser.add_argument("--tag", help="Release tag name (e.g. v0.4.0)", default=None)
    parser.add_argument(
        "--release-dir",
        help="Directory containing release archives",
        default="release",
    )
    parser.add_argument(
        "--output",
        help="Path to output markdown file",
        default="RELEASE_NOTES.md",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent
    tag = args.tag if args.tag else get_git_tag()
    release_dir = Path(args.release_dir)
    if not release_dir.is_absolute():
        release_dir = repo_root / release_dir

    print(f"[*] Generating release notes for tag: {tag}")
    print(f"[*] Repository root: {repo_root}")
    print(f"[*] Release directory: {release_dir}")

    # 1. Search for dedicated doc
    doc_path = find_release_doc(tag, repo_root)
    body = ""
    if doc_path:
        print(f"[+] Found dedicated release document: {doc_path}")
        body = doc_path.read_text(encoding="utf-8")
    else:
        # 2. Extract from CHANGELOG.md
        body = extract_changelog_section(tag, repo_root)
        if body:
            print("[+] Extracted release section from CHANGELOG.md")
        else:
            # 3. Dynamic generation from git
            print("[*] Generating dynamic changelog from git log...")
            prev_tag = get_previous_tag(tag)
            body = generate_git_changelog(tag, prev_tag, repo_root)

    # 4. Scan assets & append checksums table
    _, assets_table = process_release_assets(release_dir)
    if assets_table:
        body = body.rstrip() + "\n\n" + assets_table

    output_path = Path(args.output)
    if not output_path.is_absolute():
        output_path = repo_root / output_path

    output_path.write_text(body, encoding="utf-8")
    print(f"[✓] Release notes successfully written to: {output_path} ({len(body)} bytes)")


if __name__ == "__main__":
    main()

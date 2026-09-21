#!/usr/bin/env python3
"""
修复所有剩余的编译错误
"""

import re
from pathlib import Path

def fix_all_errors(file_path):
    """修复单个文件的所有错误"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content
        changes = []

        # 修复 1: std::sync::atomic 中的 Arc/Mutex/RwLock
        pattern = r'use std::sync::atomic::\{([^}]+)\};'
        matches = re.findall(pattern, content)

        for match in matches:
            items = [item.strip() for item in match.split(',')]
            atomic_items = []
            sync_items = []

            for item in items:
                if item in ['Arc', 'Mutex', 'RwLock']:
                    sync_items.append(item)
                else:
                    atomic_items.append(item)

            if sync_items and atomic_items:
                old_import = f"use std::sync::atomic::{{{match}}};"
                new_import = f"use std::sync::{{{', '.join(sync_items)}}};\nuse std::sync::atomic::{{{', '.join(atomic_items)}}};"

                if old_import in content:
                    content = content.replace(old_import, new_import)
                    changes.append(f"  Split atomic import: {old_import[:50]}...")

        # 修复 2: TokioInstant 和 TokioDuration 类型错误
        content = re.sub(r'use std::time::\{Duration, TokioInstant\};', 'use std::time::{Duration, Instant};', content)
        content = re.sub(r'use tokio::time::\{TokioDuration, TokioInstant\};', 'use tokio::time::{Duration, Instant};', content)

        if 'TokioInstant' in content or 'TokioDuration' in content:
            changes.append("  Replaced TokioInstant/TokioDuration with standard types")

        # 修复 3: 重复的 Mutex 导入（std::sync::Mutex 和 tokio::sync::Mutex）
        lines = content.split('\n')
        for i, line in enumerate(lines):
            if 'use std::sync::{Arc, Mutex}' in line and any('use tokio::sync::Mutex;' in other_line for other_line in lines):
                # 重命名 tokio 的 Mutex
                for j, other_line in enumerate(lines):
                    if 'use tokio::sync::Mutex;' in other_line:
                        lines[j] = other_line.replace('use tokio::sync::Mutex;', 'use tokio::sync::Mutex as TokioMutex;')
                        changes.append("  Renamed tokio::sync::Mutex to TokioMutex")
            elif 'use std::sync::Mutex;' in line and any('use tokio::sync::Mutex;' in other_line for other_line in lines):
                # 重命名 std 的 Mutex
                lines[i] = 'use std::sync::Mutex as StdMutex;'
                changes.append("  Renamed std::sync::Mutex to StdMutex")

        content = '\n'.join(lines)

        if changes:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            return True, changes
        return False, []

    except Exception as e:
        print(f"Error processing {file_path}: {e}", file=sys.stderr)
        return False, [f"ERROR: {e}"]

def main():
    """主函数"""
    project_root = Path("/Users/henry/code/amberjs")

    # 修复所有 Rust 文件
    rust_files = list(project_root.rglob("*.rs"))

    total_files = 0
    fixed_files = 0
    total_changes = 0

    print("=" * 80)
    print("修复所有剩余的编译错误")
    print("=" * 80)

    for file_path in rust_files:
        # 跳过备份文件
        if file_path.name.endswith('.bak'):
            continue

        total_files += 1
        fixed, changes = fix_all_errors(file_path)

        if fixed:
            fixed_files += 1
            total_changes += len(changes)
            rel_path = file_path.relative_to(project_root)
            print(f"\n✅ Fixed: {rel_path}")
            for change in changes[:2]:  # 只显示前2个变化
                print(change)
            if len(changes) > 2:
                print(f"  ... and {len(changes) - 2} more changes")

    print("\n" + "=" * 80)
    print(f"修复完成！")
    print(f"总文件数: {total_files}")
    print(f"修复文件数: {fixed_files}")
    print(f"总修改数: {total_changes}")
    print("=" * 80)

if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""
Amber 剩余导入语法错误修复脚本
专门处理 std::sync::atomic 相关的所有错误
"""

import os
import re
import sys
from pathlib import Path

def fix_file_imports(file_path):
    """修复单个文件的导入语法错误"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()

        original_lines = lines.copy()
        changes = []
        modified = False

        for i, line in enumerate(lines):
            original_line = line

            # 模式 1: use std::sync::atomic::Arc, , Mutex, ;
            # 更宽松的匹配
            if re.search(r'use std::sync::atomic::[A-Za-z_][A-Za-z0-9_,\s]*;', line):
                # 提取所有项目
                match = re.search(r'use std::sync::atomic::([A-Za-z_][A-Za-z0-9_,\s]*);', line)
                if match:
                    items_str = match.group(1)
                    # 分割并清理
                    items = [item.strip().strip(',').strip() for item in items_str.split(',')]
                    items = [item for item in items if item and not item.isspace()]
                    if items:
                        new_line = f"use std::sync::atomic::{{{', '.join(items)}}};\n"
                        lines[i] = new_line
                        changes.append(f"  Line {i+1}: {original_line.strip()} -> {new_line.strip()}")
                        modified = True

            # 模式 2: use std::sync::Ordering; (应该是 atomic::Ordering)
            elif 'use std::sync::Ordering;' in line:
                new_line = line.replace('use std::sync::Ordering;', 'use std::sync::atomic::Ordering;')
                if new_line != line:
                    lines[i] = new_line
                    changes.append(f"  Line {i+1}: {original_line.strip()} -> {new_line.strip()}")
                    modified = True

            # 模式 3: use std::sync::{..., Ordering}; (在花括号内的 Ordering)
            elif 'std::sync::{' in line and 'Ordering' in line and 'atomic' not in line:
                # 替换 Ordering 为 atomic::Ordering
                new_line = re.sub(r'Ordering(?!\w)', 'atomic::Ordering', line)
                if new_line != line:
                    lines[i] = new_line
                    changes.append(f"  Line {i+1}: {original_line.strip()} -> {new_line.strip()}")
                    modified = True

            # 模式 4: 修复空导入 use std::sync::atomic:: ;
            elif re.match(r'\s*use std::sync::atomic::\s*;\s*$', line):
                lines[i] = '\n'
                changes.append(f"  Line {i+1}: Removed empty import: {original_line.strip()}")
                modified = True

        if modified:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.writelines(lines)
            return True, changes
        return False, []

    except Exception as e:
        print(f"Error processing {file_path}: {e}", file=sys.stderr)
        return False, [f"ERROR: {e}"]

def main():
    """主函数"""
    project_root = Path("/Users/henry/code/amberjs")
    rust_files = list(project_root.rglob("*.rs"))

    total_files = 0
    fixed_files = 0
    total_changes = 0

    print("=" * 80)
    print("Amber 剩余导入语法错误修复脚本")
    print("=" * 80)

    # 手动修复一些关键文件
    key_files = [
        "src/benchmarks/concurrent.rs",
        "src/runtime_lite.rs",
        "src/process_pool.rs",
        "src/concurrent_execution.rs",
        "src/shared_object_cache.rs",
        "src/network/memory_mapper.rs",
        "src/zero_copy.rs",
        "src/isolate_prewarmer.rs",
        "src/io/dma_engine.rs",
        "src/io/memory_mapper.rs",
        "src/memory/zero_copy.rs",
        "src/memory/gc_optimizer.rs",
    ]

    for key_file in key_files:
        file_path = project_root / key_file
        if file_path.exists():
            total_files += 1
            fixed, changes = fix_file_imports(file_path)
            if fixed:
                fixed_files += 1
                total_changes += len(changes)
                print(f"\n✅ Fixed: {key_file}")
                for change in changes:
                    print(change)

    # 处理所有 Rust 文件
    for file_path in rust_files:
        # 跳过备份文件
        if file_path.name.endswith('.bak'):
            continue

        # 跳过已经处理的关键文件
        if str(file_path.relative_to(project_root)) in key_files:
            continue

        total_files += 1
        fixed, changes = fix_file_imports(file_path)

        if fixed:
            fixed_files += 1
            total_changes += len(changes)
            rel_path = file_path.relative_to(project_root)
            print(f"\n✅ Fixed: {rel_path}")
            for change in changes[:3]:  # 只显示前3个变化
                print(change)
            if len(changes) > 3:
                print(f"  ... and {len(changes) - 3} more changes")

    print("\n" + "=" * 80)
    print(f"修复完成！")
    print(f"总文件数: {total_files}")
    print(f"修复文件数: {fixed_files}")
    print(f"总修改数: {total_changes}")
    print("=" * 80)

if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""
修复重复导入错误 (E0252)
"""

import re
from pathlib import Path

def fix_duplicate_imports(file_path):
    """修复单个文件的重复导入错误"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()

        original_lines = lines.copy()
        changes = []
        modified = False

        # 修复特定的重复导入模式
        for i, line in enumerate(lines):
            if line.strip().startswith('use '):
                original_line = line

                # 模式1: Duration 重复
                if 'use std::time::{Duration, Instant, SystemTime};' in line:
                    # 检查是否还有其他的 Duration 导入
                    if any('use std::time::{Duration, Instant};' in other_line for other_line in lines):
                        # 删除这个导入，让简单的版本保留
                        lines[i] = ''
                        changes.append(f"  Line {i+1}: Removed: {original_line.strip()}")
                        modified = True

                # 模式2: Instant 重复
                elif 'use std::time::Instant;' in line:
                    # 检查是否还有其他的 Instant 导入
                    if any('use std::time::{Duration, Instant};' in other_line for other_line in lines):
                        # 删除这个导入
                        lines[i] = ''
                        changes.append(f"  Line {i+1}: Removed: {original_line.strip()}")
                        modified = True

                # 模式3: HashMap 重复
                elif 'use std::collections::HashMap;' in line:
                    # 检查是否还有其他的 HashMap 导入
                    if any('use std::collections::{HashMap, HashSet};' in other_line for other_line in lines):
                        lines[i] = ''
                        changes.append(f"  Line {i+1}: Removed: {original_line.strip()}")
                        modified = True

                # 模式4: Mutex 重复 (std::sync::Mutex vs tokio::sync::Mutex)
                elif 'use std::sync::Mutex;' in line:
                    # 检查是否还有 tokio::sync::Mutex
                    if any('use tokio::sync::Mutex;' in other_line for other_line in lines):
                        # 重命名为 std_mutex 以避免冲突
                        lines[i] = 'use std::sync::Mutex as StdMutex;\n'
                        changes.append(f"  Line {i+1}: Renamed to avoid conflict: {original_line.strip()} -> use std::sync::Mutex as StdMutex;")
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

    # 手动修复关键文件
    key_files = [
        "src/concurrent_execution.rs",
        "src/jit/optimization.rs",
        "src/jit/vectorization_optimizer.rs",
        "src/testing/mod.rs",
    ]

    total_files = 0
    fixed_files = 0
    total_changes = 0

    print("=" * 80)
    print("修复重复导入错误 (E0252)")
    print("=" * 80)

    for key_file in key_files:
        file_path = project_root / key_file
        if file_path.exists():
            total_files += 1
            fixed, changes = fix_duplicate_imports(file_path)
            if fixed:
                fixed_files += 1
                total_changes += len(changes)
                print(f"\n✅ Fixed: {key_file}")
                for change in changes:
                    print(change)

    print("\n" + "=" * 80)
    print(f"修复完成！")
    print(f"总文件数: {total_files}")
    print(f"修复文件数: {fixed_files}")
    print(f"总修改数: {total_changes}")
    print("=" * 80)

if __name__ == "__main__":
    main()

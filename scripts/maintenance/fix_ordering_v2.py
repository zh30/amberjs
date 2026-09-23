#!/usr/bin/env python3
"""
修复 Ordering 重复导入错误 - V2版本
更精确的修复逻辑
"""

import re
import os
from pathlib import Path

def fix_file(file_path):
    """修复单个文件"""
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    modified = False
    i = 0
    new_lines = []

    while i < len(lines):
        line = lines[i].rstrip()
        next_line = lines[i + 1].rstrip() if i + 1 < len(lines) else ""

        # 检查模式：当前行包含 atomic::Ordering，下一行是单独的 Ordering 导入
        if (re.search(r'use std::sync::\{[^}]*atomic::Ordering[^}]*\};', line) and
            re.match(r'use std::sync::atomic::Ordering;', next_line)):
            # 删除下一行
            print(f"  删除重复: {next_line}")
            modified = True
            new_lines.append(line + '\n')
            i += 2
            continue

        # 检查：atomic 中包含非原子类型（Arc, Mutex, RwLock）
        if re.search(r'use std::sync::atomic::\{[^}]*(Arc|Mutex|RwLock)[^}]*\};', line):
            # 提取所有类型
            match = re.search(r'use std::sync::atomic::\{([^}]+)\};', line)
            if match:
                all_types = [t.strip() for t in match.group(1).split(',')]

                # 分离原子类型和非原子类型
                atomic_types = [t for t in all_types if 'atomic::' in t or t in ['AtomicUsize', 'AtomicBool', 'Ordering']]
                non_atomic_types = [t for t in all_types if t not in atomic_types]

                # 重新构建导入
                new_imports = []
                if non_atomic_types:
                    new_imports.append('std::sync::{' + ', '.join(non_atomic_types))
                if atomic_types:
                    # 处理 atomic:: 前缀
                    cleaned_atomic = [t.replace('atomic::', '') for t in atomic_types]
                    new_imports.append('atomic::{' + ', '.join(cleaned_atomic) + '}')

                # 组合新行
                if len(new_imports) == 1 and not non_atomic_types:
                    # 只有 atomic 类型
                    new_line = f"use std::sync::{atomic_types[0]};"
                elif len(new_imports) == 1 and not atomic_types:
                    # 只有非原子类型
                    new_line = f"use std::sync::{non_atomic_types[0]};"
                else:
                    # 混合类型
                    combined = '}, '.join(new_imports) + '};'
                    new_line = combined.replace('}, },', '}},')

                print(f"  修复: {line}")
                print(f"     -> {new_line}")
                line = new_line
                modified = True

        new_lines.append(line + '\n')
        i += 1

    if modified:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.writelines(new_lines)
        return True

    return False

def main():
    src_dir = Path('/Users/henry/code/amberjs/src')
    rust_files = list(src_dir.rglob('*.rs'))

    print("🔧 修复 Ordering 重复导入错误...")

    modified_files = 0
    for file_path in rust_files:
        if fix_file(file_path):
            print(f"✅ {file_path.relative_to(src_dir)}")
            modified_files += 1

    print(f"\n🎉 完成! 修改了 {modified_files} 个文件")

if __name__ == '__main__':
    main()

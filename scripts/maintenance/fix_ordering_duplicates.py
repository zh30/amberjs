#!/usr/bin/env python3
"""
修复 Ordering 重复导入错误
修复模式：
use std::sync::{Arc, Mutex, atomic::Ordering};  // 已包含 Ordering
use std::sync::atomic::Ordering;                // 重复导入

修复为：
use std::sync::{Arc, Mutex, atomic::Ordering};  // 保留一个
"""

import re
import os
from pathlib import Path

def fix_ordering_duplicates(file_path):
    """修复单个文件中的 Ordering 重复导入"""
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    original_content = content

    # 查找并修复 Ordering 重复导入
    # 模式1: atomic::Ordering 在行内，后面又单独导入
    # use std::sync::{..., atomic::Ordering};
    # use std::sync::atomic::Ordering;

    lines = content.split('\n')
    modified_lines = []
    i = 0
    changes = 0

    while i < len(lines):
        line = lines[i]

        # 检查是否是包含 atomic::Ordering 的导入行
        if re.match(r'\s*use std::sync::\{[^}]*atomic::Ordering[^}]*\};', line):
            # 检查下一行是否也是 Ordering 导入
            if i + 1 < len(lines):
                next_line = lines[i + 1]
                if re.match(r'\s*use std::sync::atomic::Ordering;', next_line):
                    # 跳过下一行（删除重复导入）
                    print(f"  删除重复导入: {next_line.strip()}")
                    changes += 1
                    i += 2  # 跳过两行
                    continue

        # 检查是否是包含 Mutex/RwLock 在 atomic 中的导入
        if re.match(r'\s*use std::sync::atomic::\{[^}]*\}.*;', line):
            # 检查是否包含 Mutex 或 RwLock（它们不应该在 atomic 中）
            if 'Mutex' in line or 'RwLock' in line or 'Arc' in line:
                # 修复：将 atomic::{...} 改为 {...}
                fixed_line = re.sub(r'use std::sync::atomic::\{', 'use std::sync::{', line)
                # 重新排序，把 atomic 类型放前面
                fixed_line = re.sub(r'(Arc,?\s*|Mutex,?\s*|RwLock,?\s*)', '', fixed_line)
                # 添加 atomic 类型
                atomic_types = []
                if 'atomic::' in line:
                    atomic_match = re.search(r'atomic::([^,\}]+)', line)
                    if atomic_match:
                        atomic_types.append(atomic_match.group(1))

                if atomic_types:
                    # 在 { 后面插入 atomic::types,
                    fixed_line = re.sub(r'use std::sync::\{', f'use std::sync::{{atomic::{", ".join(atomic_types)}, ', fixed_line)

                if fixed_line != line:
                    print(f"  修复路径错误: {line.strip()} -> {fixed_line.strip()}")
                    line = fixed_line
                    changes += 1

        modified_lines.append(line)
        i += 1

    if changes > 0:
        content = '\n'.join(modified_lines)
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)
        return changes

    return 0

def main():
    """主函数：查找并修复所有 Ordering 重复导入错误"""
    src_dir = Path('/Users/henry/code/amberjs/src')

    print("🔧 开始修复 Ordering 重复导入错误...")

    total_changes = 0
    files_modified = 0

    # 获取所有 Rust 文件
    rust_files = list(src_dir.rglob('*.rs'))

    print(f"📁 扫描 {len(rust_files)} 个 Rust 文件...")

    for file_path in rust_files:
        changes = fix_ordering_duplicates(file_path)
        if changes > 0:
            print(f"✅ {file_path.relative_to(src_dir)}: {changes} 处修改")
            files_modified += 1
            total_changes += changes

    print(f"\n🎉 修复完成!")
    print(f"   修改文件数: {files_modified}")
    print(f"   总修改数: {total_changes}")

if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""
修复重复导入错误 (E0252)
- 修复 HashMap 重复导入
- 修复 RwLock/Mutex 重复导入 (std::sync vs tokio::sync)
"""

import re
import os
from pathlib import Path

def fix_duplicate_imports(file_path):
    """修复文件中的重复导入错误"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content
        lines = content.split('\n')
        modified = False

        # 收集所有导入
        std_collections_imports = []
        std_sync_imports = []
        tokio_sync_imports = []
        other_imports = []

        for line in lines:
            if 'use std::collections::' in line:
                std_collections_imports.append(line.strip())
            elif 'use std::sync::' in line and 'atomic' not in line:
                std_sync_imports.append(line.strip())
            elif 'use tokio::sync::' in line:
                tokio_sync_imports.append(line.strip())
            else:
                other_imports.append(line)

        # 合并 HashMap 导入
        hashmap_imports = [line for line in std_collections_imports if 'HashMap' in line]
        if len(hashmap_imports) > 1:
            # 提取所有相关类型
            types = set()
            for line in hashmap_imports:
                if '::{' in line:
                    # 提取大括号内的类型
                    match = re.search(r'::\{([^}]+)\}', line)
                    if match:
                        types_in_line = [t.strip() for t in match.group(1).split(',')]
                        types.update(types_in_line)
                elif '::' in line and 'HashMap' in line:
                    # 提取具体类型名
                    match = re.search(r'::(\w+)', line)
                    if match:
                        types.add(match.group(1))

            # 创建合并后的导入
            if types:
                merged_import = f"use std::collections::{{{', '.join(sorted(types))}}};"
                # 在其他导入中查找合适位置插入
                insert_index = len(other_imports)
                for i, line in enumerate(other_imports):
                    if 'use std::' in line:
                        insert_index = i + 1

                # 移除旧的 HashMap 导入
                new_std_collections_imports = [line for line in std_collections_imports if 'HashMap' not in line]

                # 添加合并后的导入
                new_std_collections_imports.append(merged_import)

                # 重建文件内容
                other_imports[insert_index:insert_index] = new_std_collections_imports
                content = '\n'.join(other_imports)
                modified = True
                print(f"  修复 HashMap 重复导入: {file_path}")

        # 合并 RwLock/Mutex 导入
        std_sync_types = set()
        tokio_sync_types = set()

        for line in std_sync_imports:
            if '::{' in line:
                match = re.search(r'::\{([^}]+)\}', line)
                if match:
                    types_in_line = [t.strip() for t in match.group(1).split(',')]
                    std_sync_types.update(types_in_line)
            elif '::' in line:
                match = re.search(r'::(\w+)', line)
                if match:
                    std_sync_types.add(match.group(1))

        for line in tokio_sync_imports:
            if '::{' in line:
                match = re.search(r'::\{([^}]+)\}', line)
                if match:
                    types_in_line = [t.strip() for t in match.group(1).split(',')]
                    tokio_sync_types.update(types_in_line)
            elif '::' in line:
                match = re.search(r'::(\w+)', line)
                if match:
                    tokio_sync_types.add(match.group(1))

        # 检查冲突并重命名
        conflicts = std_sync_types & tokio_sync_types
        if conflicts and len(tokio_sync_imports) > 0:
            # 重命名 tokio 类型
            renamed_tokio_imports = []
            for line in tokio_sync_imports:
                new_line = line
                if 'RwLock' in conflicts:
                    new_line = new_line.replace('RwLock', 'TokioRwLock')
                if 'Mutex' in conflicts:
                    new_line = new_line.replace('Mutex', 'TokioMutex')
                if 'Arc' in conflicts:
                    new_line = new_line.replace('Arc', 'TokioArc')
                if 'mpsc' not in new_line:  # 保留 mpsc
                    renamed_tokio_imports.append(new_line)

            # 移除旧的 tokio 导入
            other_imports = [line for line in other_imports if 'tokio::sync::' not in line]

            # 添加重命名后的导入
            if renamed_tokio_imports:
                # 找到合适位置插入
                insert_index = len(other_imports)
                for i, line in enumerate(other_imports):
                    if 'use tokio::' in line:
                        insert_index = i + 1
                other_imports[insert_index:insert_index] = renamed_tokio_imports

            content = '\n'.join(other_imports)
            modified = True
            print(f"  修复 RwLock/Mutex 冲突: {file_path} (重命名: {conflicts})")

        # 写回文件
        if modified and content != original_content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            return True

    except Exception as e:
        print(f"  错误处理文件 {file_path}: {e}")

    return False

def main():
    """主函数：扫描并修复所有 Rust 源文件"""
    src_dir = Path('/Users/henry/code/amberjs/src')
    fixed_count = 0
    total_files = 0

    print("=== 修复重复导入错误 (E0252) ===\n")

    # 扫描所有 .rs 文件
    for rust_file in src_dir.rglob('*.rs'):
        total_files += 1
        if fix_duplicate_imports(rust_file):
            fixed_count += 1

    print(f"\n=== 修复完成 ===")
    print(f"处理文件数: {total_files}")
    print(f"修复文件数: {fixed_count}")

if __name__ == '__main__':
    main()

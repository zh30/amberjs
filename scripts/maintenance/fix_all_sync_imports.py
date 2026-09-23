#!/usr/bin/env python3
"""
彻底清理所有 std::sync 导入问题
"""

import re
from pathlib import Path

def fix_all_sync_imports(file_path):
    """彻底修复单个文件的 std::sync 导入"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content

        # 查找所有 std::sync 导入行
        lines = content.split('\n')
        new_lines = []
        sync_imports = []

        for line in lines:
            if 'use std::sync::' in line and line.strip().endswith(';'):
                sync_imports.append(line.strip())
            else:
                new_lines.append(line)

        # 如果有多个 std::sync 导入，合并它们
        if len(sync_imports) > 1:
            all_types = set()
            for sync_import in sync_imports:
                match = re.search(r'use std::sync::\{([^}]+)\};', sync_import)
                if match:
                    types = [t.strip() for t in match.group(1).split(',')]
                    all_types.update(types)

            # 生成合并后的导入
            if all_types:
                types_str = ', '.join(sorted(all_types))
                merged_import = f"use std::sync::{{{types_str}}};"

                # 找到合适的位置插入合并后的导入（通常在最后一个 use 语句后）
                insert_index = -1
                for i, line in enumerate(new_lines):
                    if line.strip().startswith('use ') and line.strip().endswith(';'):
                        insert_index = i + 1

                if insert_index > 0:
                    new_lines.insert(insert_index, merged_import)
                else:
                    new_lines.insert(0, merged_import)

        # 检查是否有变化
        new_content = '\n'.join(new_lines)
        if new_content != original_content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(new_content)
            print(f"Fixed: {file_path}")
            return True
        return False

    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        return False

def main():
    """主函数"""
    src_dir = Path('/Users/henry/code/amberjs/src')

    # 查找所有 Rust 文件
    rust_files = list(src_dir.rglob('*.rs'))

    fixed_count = 0
    for file_path in rust_files:
        if fix_all_sync_imports(file_path):
            fixed_count += 1

    print(f"\nFixed {fixed_count} files")

if __name__ == '__main__':
    main()

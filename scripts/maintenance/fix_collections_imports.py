#!/usr/bin/env python3
"""
修复错误的 collections 导入错误
移除错误的 `use std::collections::{..., collections}` 模式
"""

import re
from pathlib import Path

def fix_collections_imports(file_path):
    """修复文件中的 collections 导入错误"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content

        # 修复错误的 collections 导入
        # 匹配模式: use std::collections::{..., collections};
        pattern = r'use std::collections::\{([^}]*),\s*collections\};'
        content = re.sub(pattern, r'use std::collections::{\1};', content)

        # 匹配模式: use std::collections::{collections, ...};
        pattern = r'use std::collections::\{collections,\s*([^}]*)\};'
        content = re.sub(pattern, r'use std::collections::{\1};', content)

        # 匹配模式: use std::collections::{collections};
        pattern = r'use std::collections::\{collections\};'
        content = re.sub(pattern, r'// use std::collections::collections; // Removed - invalid import', content)

        # 写回文件
        if content != original_content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"  修复 collections 导入: {file_path}")
            return True

    except Exception as e:
        print(f"  错误处理文件 {file_path}: {e}")

    return False

def main():
    """主函数：扫描并修复所有 Rust 源文件"""
    src_dir = Path('/Users/henry/code/amberjs/src')
    fixed_count = 0
    total_files = 0

    print("=== 修复错误的 collections 导入 ===\n")

    # 扫描所有 .rs 文件
    for rust_file in src_dir.rglob('*.rs'):
        total_files += 1
        if fix_collections_imports(rust_file):
            fixed_count += 1

    print(f"\n=== 修复完成 ===")
    print(f"处理文件数: {total_files}")
    print(f"修复文件数: {fixed_count}")

if __name__ == '__main__':
    main()

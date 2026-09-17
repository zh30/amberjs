#!/usr/bin/env python3
"""
修复缺少的 derive 宏导入错误
- 添加 thiserror::Error 的导入
- 添加 serde::{Serialize, Deserialize} 的导入
"""

import re
from pathlib import Path

def fix_missing_derive_imports(file_path):
    """修复文件中的缺少的 derive 导入"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content
        lines = content.split('\n')
        modified = False

        # 检查是否使用了 derive 宏但没有导入
        has_error_derive = '#[derive(Error' in content
        has_serialize_derive = '#[derive(Serialize' in content or '#[derive(Deserialize' in content
        has_debug_derive = '#[derive(Debug' in content

        # 检查是否已经有相关导入
        has_thiserror = 'use thiserror::' in content or 'thiserror::Error' in content
        has_serde = 'use serde::' in content
        has_anyhow = 'use anyhow::' in content

        new_imports = []

        # 如果使用了 Error derive 但没有导入，添加导入
        if has_error_derive and not has_thiserror:
            new_imports.append('use thiserror::Error;')
            modified = True

        # 如果使用了 Serialize/Deserialize derive 但没有导入，添加导入
        if has_serialize_derive and not has_serde:
            new_imports.append('use serde::{Serialize, Deserialize};')
            modified = True

        # 如果使用了 anyhow 但没有导入，添加导入
        if 'anyhow::' in content and not has_anyhow:
            new_imports.append('use anyhow::{Result, Error};')

        # 在适当位置插入导入（通常在其他 use 语句之后）
        if new_imports:
            insert_index = 0
            for i, line in enumerate(lines):
                if line.strip().startswith('use '):
                    insert_index = i + 1

            lines[insert_index:insert_index] = new_imports
            content = '\n'.join(lines)
            modified = True
            print(f"  添加 derive 导入: {file_path}")

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

    print("=== 修复缺少的 derive 宏导入 ===\n")
    print("添加:")
    print("  - thiserror::Error (for #[derive(Error)])")
    print("  - serde::{Serialize, Deserialize} (for #[derive(Serialize/Deserialize)])")
    print()

    # 扫描所有 .rs 文件
    for rust_file in src_dir.rglob('*.rs'):
        total_files += 1
        if fix_missing_derive_imports(rust_file):
            fixed_count += 1

    print(f"\n=== 修复完成 ===")
    print(f"处理文件数: {total_files}")
    print(f"修复文件数: {fixed_count}")

if __name__ == '__main__':
    main()

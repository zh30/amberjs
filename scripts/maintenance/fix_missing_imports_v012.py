#!/usr/bin/env python3
"""
修复缺失导入的自动化脚本 (v0.1.2)
修复 AtomicUsize、HashMap、Instant 等基础类型的导入缺失问题

使用方法:
    python fix_missing_imports_v012.py
"""

import re
import os
import sys
from pathlib import Path

def fix_file_imports(file_path):
    """修复单个文件的导入"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content
        lines = content.split('\n')

        # 需要添加的导入
        imports_to_add = {
            'AtomicUsize': 'std::sync::atomic::AtomicUsize',
            'AtomicBool': 'std::sync::atomic::AtomicBool',
            'Ordering': 'std::sync::atomic::Ordering',
            'Instant': 'std::time::Instant',
            'Duration': 'std::time::Duration',
        }

        # 检查每个缺失的类型
        for type_name, import_path in imports_to_add.items():
            # 跳过注释行
            non_comment_lines = [line for line in lines if not line.strip().startswith('//')]

            # 检查文件是否已经导入了这个类型
            has_import = False
            for line in non_comment_lines:
                if f'use {import_path}' in line or f'use {import_path}::' in line:
                    has_import = True
                    break

            if not has_import:
                # 检查文件是否使用了这个类型
                type_pattern = rf'\b{type_name}\b'
                if re.search(type_pattern, content):
                    # 找到合适的位置插入导入（在其他 use 语句附近）
                    insert_index = None
                    for i, line in enumerate(lines):
                        if line.strip().startswith('use std::'):
                            insert_index = i + 1

                    if insert_index:
                        # 格式化导入语句
                        import_statement = f'use {import_path};'
                        lines.insert(insert_index, import_statement)
                        print(f"  ✓ 添加导入: {import_statement} 到 {file_path}")

        # 检查是否需要添加 std::sync::atomic 的批量导入
        has_atomic_import = any('use std::sync::atomic::' in line for line in lines)
        has_atomic_usage = 'AtomicUsize' in content or 'AtomicBool' in content or 'Ordering' in content

        if has_atomic_usage and not has_atomic_import:
            # 找到合适位置插入
            for i, line in enumerate(lines):
                if line.strip().startswith('use std::'):
                    lines.insert(i, 'use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};')
                    print(f"  ✓ 添加批量原子类型导入到 {file_path}")
                    break

        # 重新组合内容
        new_content = '\n'.join(lines)

        # 只有在有变更时才写入文件
        if new_content != original_content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(new_content)
            return True

    except Exception as e:
        print(f"  ✗ 错误处理文件 {file_path}: {e}", file=sys.stderr)

    return False

def main():
    """主函数"""
    print("🔧 开始修复缺失的导入 (v0.1.2)")
    print("=" * 60)

    src_dir = Path('/Users/henry/code/amberjs/src')
    if not src_dir.exists():
        print(f"❌ 源目录不存在: {src_dir}")
        return 1

    # 获取所有 Rust 文件
    rust_files = list(src_dir.rglob('*.rs'))
    print(f"📁 找到 {len(rust_files)} 个 Rust 文件")

    fixed_count = 0
    for file_path in rust_files:
        if fix_file_imports(file_path):
            fixed_count += 1

    print("=" * 60)
    print(f"✅ 修复完成! 共修复 {fixed_count} 个文件")
    print()
    print("下一步: 运行 'cargo check' 验证修复效果")

    return 0

if __name__ == '__main__':
    sys.exit(main())

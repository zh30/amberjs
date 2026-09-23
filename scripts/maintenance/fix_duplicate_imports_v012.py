#!/usr/bin/env python3
"""
清理重复导入的脚本 (v0.1.2)
修复由自动化脚本产生的重复导入问题
"""

import re
import sys
from pathlib import Path

def clean_file_imports(file_path):
    """清理单个文件的重复导入"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content
        lines = content.split('\n')

        # 合并重复的 use 语句
        use_statements = {}
        other_lines = []

        for line in lines:
            if line.strip().startswith('use std::time::'):
                # 提取类型名
                match = re.match(r'use std::time::(\w+);', line.strip())
                if match:
                    type_name = match.group(1)
                    if type_name not in use_statements:
                        use_statements[type_name] = line.strip()
            elif line.strip().startswith('use std::sync::atomic::'):
                # 处理原子类型
                match = re.match(r'use std::sync::atomic::(\{[^}]+\}|\w+);', line.strip())
                if match:
                    items = match.group(1)
                    if items.startswith('{'):
                        # 批量导入，如 {AtomicUsize, AtomicBool}
                        items_list = [item.strip() for item in items[1:-1].split(',')]
                        for item in items_list:
                            if item not in use_statements:
                                use_statements[item] = f'use std::sync::atomic::{item};'
                    else:
                        # 单个导入
                        if items not in use_statements:
                            use_statements[items] = line.strip()
            else:
                other_lines.append(line)

        # 重新组合
        new_lines = []
        for line in other_lines:
            new_lines.append(line)

        # 添加清理后的 use 语句
        if use_statements:
            # 在最后一个 use 语句后插入
            insert_pos = 0
            for i, line in enumerate(new_lines):
                if line.strip().startswith('use '):
                    insert_pos = i + 1

            # 按类型分组
            time_imports = []
            atomic_imports = []

            for type_name, import_line in sorted(use_statements.items()):
                if 'std::time::' in import_line:
                    time_imports.append(import_line)
                elif 'std::sync::atomic::' in import_line:
                    atomic_imports.append(import_line)

            # 插入时间相关导入
            if time_imports:
                # 尝试合并为一行
                if len(time_imports) > 1:
                    types = [line.split('::')[2].rstrip(';') for line in time_imports]
                    merged = f'use std::time::{{{", ".join(types)}}};'
                    new_lines.insert(insert_pos, merged)
                else:
                    new_lines.insert(insert_pos, time_imports[0])
                insert_pos += 1

            # 插入原子类型导入
            if atomic_imports:
                if len(atomic_imports) > 1:
                    types = [line.split('::')[3].rstrip(';') for line in atomic_imports]
                    merged = f'use std::sync::atomic::{{{", ".join(types)}}};'
                    new_lines.insert(insert_pos, merged)
                else:
                    new_lines.insert(insert_pos, atomic_imports[0])

        new_content = '\n'.join(new_lines)

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
    print("🧹 开始清理重复导入 (v0.1.2)")
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
        if clean_file_imports(file_path):
            fixed_count += 1
            print(f"  ✓ 清理重复导入: {file_path}")

    print("=" * 60)
    print(f"✅ 清理完成! 共清理 {fixed_count} 个文件")
    print()
    print("下一步: 运行 'cargo check' 验证修复效果")

    return 0

if __name__ == '__main__':
    sys.exit(main())

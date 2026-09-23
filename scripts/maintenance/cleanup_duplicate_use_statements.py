#!/usr/bin/env python3
"""
清理重复的 use 语句
"""

import os
import re
from pathlib import Path

def cleanup_use_statements(file_path):
    """清理单个文件的重复 use 语句"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content
        lines = content.split('\n')

        # 找到所有 use 语句
        use_lines = []
        other_lines = []

        for i, line in enumerate(lines):
            line_stripped = line.strip()
            if line_stripped.startswith('use ') or line_stripped.startswith('pub use '):
                use_lines.append((i, line_stripped))
            else:
                other_lines.append((i, line))

        if not use_lines:
            return False

        # 检查是否有重复
        seen_imports = set()
        duplicate_imports = set()
        unique_imports = []

        for line_num, import_line in use_lines:
            if import_line in seen_imports:
                duplicate_imports.add(import_line)
            else:
                seen_imports.add(import_line)
                unique_imports.append((line_num, import_line))

        if not duplicate_imports:
            return False

        # 移除重复的导入
        new_lines = []
        for line in lines:
            line_stripped = line.strip()
            if line_stripped in duplicate_imports:
                # 跳过重复的导入
                continue
            new_lines.append(line)

        # 重新组合
        content = '\n'.join(new_lines)

        # 清理多余的空行
        content = re.sub(r'\n\s*\n\s*\n', '\n\n', content)

        if content != original_content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            return True

        return False

    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        return False

def main():
    """主函数"""
    src_dir = Path('/Users/henry/code/amberjs/src')

    if not src_dir.exists():
        print(f"Source directory not found: {src_dir}")
        return

    fixed_files = []
    total_files = 0

    # 遍历所有 Rust 文件
    for rust_file in src_dir.rglob('*.rs'):
        total_files += 1
        if cleanup_use_statements(rust_file):
            fixed_files.append(rust_file.relative_to(src_dir))

    print(f"\n✅ 重复导入清理完成!")
    print(f"📊 统计信息:")
    print(f"   - 处理文件总数: {total_files}")
    print(f"   - 修复文件数: {len(fixed_files)}")

    if fixed_files:
        print(f"\n📝 修复的文件:")
        for file_path in sorted(fixed_files)[:20]:
            print(f"   - {file_path}")
        if len(fixed_files) > 20:
            print(f"   ... 还有 {len(fixed_files) - 20} 个文件")

if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""
清理重复的 Rust 导入
合并重复的导入语句，确保每个类型只导入一次
"""

import os
import re
from pathlib import Path
from collections import defaultdict

def fix_duplicate_imports(file_path):
    """修复单个文件的重复导入"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content
        lines = content.split('\n')

        # 收集所有导入
        use_imports = []
        other_lines = []

        for i, line in enumerate(lines):
            line_stripped = line.strip()
            if line_stripped.startswith('use ') or line_stripped.startswith('pub use '):
                use_imports.append((i, line_stripped))
            else:
                other_lines.append((i, line))

        if not use_imports:
            return False

        # 按模块分组导入
        imports_by_module = defaultdict(set)

        for line_num, import_line in use_imports:
            # 解析导入语句
            if '::' in import_line:
                # 提取模块路径和导入的内容
                match = re.match(r'(pub )?use ([^;]+);', import_line)
                if match:
                    use_part = match.group(2).strip()

                    # 分离模块路径和类型
                    if '{' in use_part:
                        module_path = use_part.split('{')[0].strip()
                        types_part = use_part.split('{')[1].split('}')[0].strip()

                        # 按逗号分割类型
                        types = [t.strip() for t in types_part.split(',') if t.strip()]

                        for type_name in types:
                            imports_by_module[module_path].add(type_name)
                    else:
                        # 简单导入如 use std::sync::Arc;
                        imports_by_module[module_path].add('')

        # 重新构建导入语句
        new_imports = []
        for module_path in sorted(imports_by_module.keys()):
            types = sorted(imports_by_module[module_path])
            types = [t for t in types if t]  # 过滤空字符串

            if types:
                # 有多个类型，用大括号
                if len(types) > 1:
                    types_str = '{' + ', '.join(types) + '}'
                else:
                    types_str = types[0]

                import_line = f'use {module_path}::{types_str};'
            else:
                # 简单导入
                import_line = f'use {module_path};'

            new_imports.append(import_line)

        # 重建所有行
        new_lines = []
        last_was_use = False
        use_section_complete = False

        for line in lines:
            line_stripped = line.strip()

            # 如果是导入行，跳过
            if line_stripped.startswith('use ') or line_stripped.startswith('pub use '):
                if not last_was_use:
                    # 开始新的导入部分
                    if new_imports:
                        new_lines.extend([''] + new_imports + [''])
                    use_section_complete = True
                last_was_use = True
            else:
                # 普通行
                if last_was_use and not use_section_complete:
                    # 导入部分结束
                    if new_imports:
                        new_lines.extend([''] + new_imports + [''])
                    use_section_complete = True

                new_lines.append(line)
                last_was_use = False

        # 如果文件只有导入没有其他内容
        if last_was_use and not use_section_complete and new_imports:
            new_lines.extend([''] + new_imports)

        # 重新组合
        content = '\n'.join(new_lines)

        # 清理多余的空行（不超过2个连续空行）
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
        if fix_duplicate_imports(rust_file):
            fixed_files.append(rust_file.relative_to(src_dir))

    print(f"\n✅ 重复导入清理完成!")
    print(f"📊 统计信息:")
    print(f"   - 处理文件总数: {total_files}")
    print(f"   - 修复文件数: {len(fixed_files)}")
    print(f"   - 修复率: {len(fixed_files)/total_files*100:.1f}%")

    if fixed_files:
        print(f"\n📝 修复的文件:")
        for file_path in sorted(fixed_files)[:20]:
            print(f"   - {file_path}")
        if len(fixed_files) > 20:
            print(f"   ... 还有 {len(fixed_files) - 20} 个文件")

if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""
Stage 61: 安全清理简单的未使用导入
只处理最明显、最安全的未使用导入删除
"""

import re
from pathlib import Path

def fix_simple_unused_imports(file_path):
    """修复单个文件中的简单未使用导入"""
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    original_lines = lines.copy()
    changes = []

    # 查找并删除简单的未使用导入
    new_lines = []
    for line in lines:
        original_line = line

        # 1. 删除单独的 Context 导入 (最安全的)
        if re.match(r'\s*use anyhow::\{[^}]*Context[^}]*\};', line):
            if 'Context' not in ''.join(lines):  # 检查整个文件是否使用了 Context
                line = ''
                changes.append("Removed unused 'Context' import from anyhow")

        # 2. 删除单独的 Command 导入
        elif re.match(r'\s*use clap::\{[^}]*Command[^}]*\};', line):
            line = ''
            changes.append("Removed unused 'Command' import from clap")

        # 3. 删除单独的 warn 导入
        elif re.match(r'\s*use tracing::\{[^}]*warn[^}]*\};', line):
            line = ''
            changes.append("Removed unused 'warn' import from tracing")

        # 4. 删除单独的 error 导入
        elif re.match(r'\s*use tracing::\{[^}]*error[^}]*\};', line):
            line = ''
            changes.append("Removed unused 'error' import from tracing")

        # 5. 删除单独的 instrument 导入
        elif re.match(r'\s*use tracing::\{[^}]*instrument[^}]*\};', line):
            line = ''
            changes.append("Removed unused 'instrument' import from tracing")

        new_lines.append(line)

    # 如果有修改，写回文件
    if new_lines != original_lines:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.writelines(new_lines)
        return changes

    return []

def main():
    """主函数：扫描并修复简单的未使用导入"""
    src_dir = Path('/Users/henry/code/amberjs/src')
    total_files = 0
    fixed_files = 0
    total_changes = 0

    print("🔧 Stage 61: 安全清理简单的未使用导入")
    print("=" * 60)

    # 递归查找所有 .rs 文件
    for rs_file in src_dir.rglob('*.rs'):
        total_files += 1
        changes = fix_simple_unused_imports(rs_file)

        if changes:
            fixed_files += 1
            print(f"\n📝 {rs_file.relative_to(src_dir)}:")
            for change in changes:
                print(f"  ✅ {change}")
                total_changes += 1

    print("\n" + "=" * 60)
    print(f"✅ 完成！扫描了 {total_files} 个文件")
    print(f"📊 修复了 {fixed_files} 个文件")
    print(f"🎯 总计 {total_changes} 处修改")

if __name__ == '__main__':
    main()

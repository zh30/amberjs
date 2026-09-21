#!/usr/bin/env python3
"""
Stage 73 Phase 2: 清理编译警告
目标: 从 338 个警告减少到 < 50 个
"""

import os
import re
import sys
from pathlib import Path
from typing import List, Tuple

def fix_unused_imports(file_path: str) -> int:
    """修复未使用的导入"""
    changes = 0
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content

        # 修复模式1: 单个未使用的导入
        patterns = [
            (r'use\s+([a-zA-Z0-9_:<>,&\s]+);\s*//\s*unused\s*$', r'// \1 - removed'),
            (r'^\s*use\s+[^;]+;\s*$', lambda m: f"// {m.group(0).strip()} - unused (auto-removed)")
        ]

        for pattern, replacement in patterns:
            if callable(replacement):
                content = re.sub(pattern, replacement, content, flags=re.MULTILINE)
            else:
                content = re.sub(pattern, replacement, content, flags=re.MULTILINE)

        if content != original_content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            changes += 1

    except Exception as e:
        print(f"Error processing {file_path}: {e}")

    return changes

def fix_unused_variables(file_path: str) -> int:
    """修复未使用的变量"""
    changes = 0
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()

        modified = False
        for i, line in enumerate(lines):
            # 修复 let mut var -> let var
            if 'let mut ' in line and 'help: remove this `mut`' in line:
                lines[i] = line.replace('let mut ', 'let ')
                modified = True
                changes += 1

            # 修复未使用参数: name -> _name
            elif re.search(r'\(\s*([^,)]+)\s*\)', line) and 'unused variable' in line:
                var_match = re.search(r'unused variable: `([^`]+)`', line)
                if var_match:
                    var_name = var_match.group(1)
                    old_pattern = rf'\b{var_name}\b'
                    lines[i] = re.sub(old_pattern, f'_{var_name}', line)
                    modified = True
                    changes += 1

        if modified:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.writelines(lines)

    except Exception as e:
        print(f"Error processing {file_path}: {e}")

    return changes

def fix_cfg_conditions(file_path: str) -> int:
    """修复 unexpected cfg conditions"""
    changes = 0
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content

        # 注释掉 verbose_logging 条件
        content = re.sub(
            r'if cfg!\(feature = "verbose_logging"\) \{',
            '// if cfg!(feature = "verbose_logging") { // disabled - no such feature',
            content
        )

        if content != original_content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            changes += 1

    except Exception as e:
        print(f"Error processing {file_path}: {e}")

    return changes

def fix_unused_assignments(file_path: str) -> int:
    """修复未使用的赋值"""
    changes = 0
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()

        modified = False
        for i, line in enumerate(lines):
            # 修复 value assigned to `var` is never read -> var is never read
            if 'value assigned to `' in line and 'is never read' in line:
                var_match = re.search(r'value assigned to `([^`]+)`', line)
                if var_match:
                    var_name = var_match.group(1)
                    old_line = lines[i]
                    new_line = old_line.replace('let mut ', 'let ')
                    new_line = new_line.replace('let ', f'let _{var_name} = ')
                    lines[i] = new_line
                    modified = True
                    changes += 1

        if modified:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.writelines(lines)

    except Exception as e:
        print(f"Error processing {file_path}: {e}")

    return changes

def find_rust_files(root_dir: str) -> List[str]:
    """查找所有 Rust 文件"""
    rust_files = []
    for root, dirs, files in os.walk(root_dir):
        # 跳过 target 目录
        if 'target' in root:
            continue
        for file in files:
            if file.endswith('.rs'):
                rust_files.append(os.path.join(root, file))
    return rust_files

def main():
    """主函数"""
    print("🚀 Stage 73 Phase 2: 开始清理编译警告")
    print("=" * 60)

    # 获取所有 Rust 文件
    rust_files = find_rust_files('/Users/henry/code/amberjs/src')
    print(f"📁 找到 {len(rust_files)} 个 Rust 文件")

    total_changes = 0
    changes_by_type = {
        'unused_imports': 0,
        'unused_variables': 0,
        'cfg_conditions': 0,
        'unused_assignments': 0
    }

    # 批量修复
    for i, file_path in enumerate(rust_files):
        if i % 20 == 0:
            print(f"📝 处理进度: {i}/{len(rust_files)}")

        # 修复不同类型的警告
        changes = fix_unused_imports(file_path)
        changes_by_type['unused_imports'] += changes
        total_changes += changes

        changes = fix_unused_variables(file_path)
        changes_by_type['unused_variables'] += changes
        total_changes += changes

        changes = fix_cfg_conditions(file_path)
        changes_by_type['cfg_conditions'] += changes
        total_changes += changes

        changes = fix_unused_assignments(file_path)
        changes_by_type['unused_assignments'] += changes
        total_changes += changes

    print("\n" + "=" * 60)
    print("✅ 清理完成!")
    print(f"📊 总修改: {total_changes} 处")
    print(f"   - 未使用导入: {changes_by_type['unused_imports']}")
    print(f"   - 未使用变量: {changes_by_type['unused_variables']}")
    print(f"   - CFG 条件: {changes_by_type['cfg_conditions']}")
    print(f"   - 未使用赋值: {changes_by_type['unused_assignments']}")
    print("\n🔍 请运行以下命令验证:")
    print("   cargo clippy --all-targets --all-features -- -D warnings")

if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""
Stage 73 Phase 2: 精确清理编译警告
只处理明确的警告，避免破坏代码结构
"""

import os
import re
import sys
from pathlib import Path

def fix_unused_variables_precise(file_path: str) -> int:
    """精确修复未使用的变量（仅 let mut 情况）"""
    changes = 0
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()

        modified = False
        for i, line in enumerate(lines):
            # 只修复明确的 let mut 问题
            original_line = line
            # 修复 let mut var -> let var (仅当明确标记时)
            if 'let mut ' in line and ('help: remove this `mut`' in line or '_var_name' in line):
                line = line.replace('let mut ', 'let ')
                modified = True
                changes += 1

            # 修复未使用的赋值
            if 'value assigned to `' in line and 'is never read' in line:
                var_match = re.search(r'value assigned to `([^`]+)`', line)
                if var_match:
                    var_name = var_match.group(1)
                    # 将变量名改为 _var_name
                    next_line = lines[i + 1] if i + 1 < len(lines) else ''
                    if var_name in next_line:
                        lines[i + 1] = next_line.replace(var_name, f'_{var_name}')
                        modified = True
                        changes += 1

        if modified:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.writelines(lines)

    except Exception as e:
        print(f"Error processing {file_path}: {e}")

    return changes

def comment_out_cfg_conditions(file_path: str) -> int:
    """注释掉 cfg 条件"""
    changes = 0
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content

        # 只注释 verbose_logging 相关条件
        if 'cfg!(feature = "verbose_logging")' in content:
            content = content.replace(
                'if cfg!(feature = "verbose_logging") {',
                '// if cfg!(feature = "verbose_logging") { // disabled - no such feature'
            )
            changes += 1

        if content != original_content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)

    except Exception as e:
        print(f"Error processing {file_path}: {e}")

    return changes

def main():
    """主函数"""
    print("🎯 Stage 73 Phase 2: 精确清理编译警告")
    print("=" * 60)

    # 只处理有问题的文件
    problem_files = [
        '/Users/henry/code/amberjs/src/testing/v8_bindings.rs',
        '/Users/henry/code/amberjs/src/runtime_lite.rs',
        '/Users/henry/code/amberjs/src/v8_context_pool.rs',
        '/Users/henry/code/amberjs/src/cli/file_watcher.rs',
        '/Users/henry/code/amberjs/src/cli/repl.rs',
        '/Users/henry/code/amberjs/src/observability/prometheus_exporter.rs',
        '/Users/henry/code/amberjs/src/network/zero_copy/sender.rs',
    ]

    total_changes = 0

    for file_path in problem_files:
        if os.path.exists(file_path):
            print(f"📝 处理: {os.path.basename(file_path)}")
            changes = fix_unused_variables_precise(file_path)
            changes += comment_out_cfg_conditions(file_path)
            total_changes += changes
        else:
            print(f"⚠️  文件不存在: {file_path}")

    print("\n" + "=" * 60)
    print("✅ 精确清理完成!")
    print(f"📊 总修改: {total_changes} 处")
    print("\n🔍 验证编译:")
    print("   cargo build --release")

if __name__ == '__main__':
    main()

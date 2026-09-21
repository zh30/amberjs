#!/usr/bin/env python3
"""
Stage 61: 清理编译警告脚本 (保守版本)
只修复未使用的变量，不删除任何导入
这样更安全，不会破坏代码
"""

import re
from pathlib import Path

def fix_unused_variables(file_path):
    """修复单个文件中的未使用变量"""
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    original_content = content
    changes = []

    # 修复未使用的变量: let variable_name =  -> let _variable_name =
    # 但要排除已经在使用的变量

    # 查找所有 let 语句
    let_pattern = r'\blet (\w+) ='
    lets = re.findall(let_pattern, content)

    # 检查每个变量是否被使用
    for var_name in lets:
        # 检查变量是否以 _ 开头（已经是未使用标记）
        if var_name.startswith('_'):
            continue

        # 检查变量在后面的代码中是否被使用
        # 简单的检查：看变量名是否在后面的代码中出现（但不是作为变量声明）
        var_usage_pattern = r'\b' + re.escape(var_name) + r'\b'
        usages = re.findall(var_usage_pattern, content)

        # 如果只使用了一次（就是声明的那次），那么是未使用的
        if len(usages) <= 1:
            # 添加下划线前缀
            new_content = re.sub(
                r'\blet ' + re.escape(var_name) + r' =',
                r'let _' + var_name + r' =',
                content,
                count=1  # 只替换第一次出现
            )
            if new_content != content:
                content = new_content
                changes.append(f"Renamed unused variable '{var_name}' to '_{var_name}'")

    # 写入文件
    if content != original_content:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)
        return changes

    return []

def main():
    """主函数：扫描所有 Rust 文件并修复未使用的变量"""
    src_dir = Path('/Users/henry/code/amberjs/src')
    total_files = 0
    fixed_files = 0
    total_changes = 0

    print("🔧 Stage 61: 清理编译警告 (保守版本)")
    print("只修复未使用的变量，不删除导入")
    print("=" * 60)

    # 递归查找所有 .rs 文件
    for rs_file in src_dir.rglob('*.rs'):
        total_files += 1
        changes = fix_unused_variables(rs_file)

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
    print("\n下一步: 运行 'cargo check --lib' 验证修复结果")

if __name__ == '__main__':
    main()

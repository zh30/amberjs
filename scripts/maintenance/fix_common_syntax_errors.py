#!/usr/bin/env python3
"""
修复 Amber 项目中的常见语法错误
"""

import os
import re
import sys

def fix_collect_errors(content):
    """修复 collect::<Vec<_>() 错误，应该为 collect::<Vec<_>>()"""
    # 查找并修复 collect::<Vec<_>() 模式
    pattern = r'collect::<Vec<_>\(\)'
    fixed = re.sub(pattern, 'collect::<Vec<_>>()', content)
    return fixed

def fix_format_string_errors(content):
    """修复格式化字符串参数不匹配的问题"""
    # 修复 format!("{}://{}, protocol", host_part) - 参数数量不匹配
    # 应该为 format!("{}://{} protocol", host_part)
    pattern1 = r'format!\(\s*"\s*\{\}://\{\},\s*protocol"\s*,\s*([^)]+)\s*\)'
    replacement1 = r'format!("{}://{} protocol", \1)'
    content = re.sub(pattern1, replacement1, content)

    return content

def fix_bracket_mismatches(content):
    """修复括号不匹配问题"""
    # 修复连续的右括号
    content = re.sub(r'\(\s*\)\s*\)\s*\)', '()', content)

    return content

def process_file(file_path):
    """处理单个文件"""
    print(f"处理文件: {file_path}")

    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    original_content = content

    # 应用修复
    content = fix_collect_errors(content)
    content = fix_format_string_errors(content)
    content = fix_bracket_mismatches(content)

    if content != original_content:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"  ✓ 已修复: {file_path}")
        return True
    else:
        print(f"  - 无需修复: {file_path}")
        return False

def main():
    """主函数"""
    print("开始修复 Amber 编译错误...")

    # 需要修复的文件列表（从编译错误中提取）
    files_to_fix = [
        'src/ai/code_generator.rs',
        'src/benchmarks/startup.rs',
        'src/benchmarks/execution.rs',
        'src/benchmarks/memory.rs',
        'src/benchmarks/concurrent.rs',
        'src/web_api/url.rs',
        'src/observability/dashboard/manager.rs',
        'src/concurrent_execution.rs',
    ]

    fixed_count = 0
    for file_path in files_to_fix:
        if os.path.exists(file_path):
            if process_file(file_path):
                fixed_count += 1
        else:
            print(f"警告: 文件不存在: {file_path}")

    print(f"\n修复完成！共修复了 {fixed_count} 个文件")

if __name__ == '__main__':
    main()
